//! Container exec functionality.
//!
//! Provides non-interactive command execution in running containers.
//! Full interactive TTY support (xterm.js) is deferred to M4.

use std::time::Duration;

use k8s_openapi::api::core::v1::Pod;
use kube::{api::AttachParams, Api, Client};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

/// Maximum output size per stream (stdout/stderr): 10 MB.
const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024;

/// Overall timeout for a single exec command: 5 minutes.
const EXEC_TIMEOUT: Duration = Duration::from_secs(300);

/// Request payload for executing a command in a container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecRequest {
    pub namespace: String,
    pub pod: String,
    pub container: Option<String>,
    pub command: Vec<String>,
}

/// Result of a non-interactive exec command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

/// Read from an async reader into a `Vec<u8>`, stopping at `max` bytes.
/// Returns `Ok(true)` if the limit was hit, `Ok(false)` if EOF was reached first.
async fn bounded_read<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut R,
    buf: &mut Vec<u8>,
    max: usize,
) -> std::io::Result<bool> {
    let mut chunk = vec![0u8; 8192];
    loop {
        let n = reader.read(&mut chunk).await?;
        if n == 0 {
            return Ok(false);
        }
        if buf.len() + n > max {
            // Append only what fits, then signal truncation
            let remaining = max.saturating_sub(buf.len());
            buf.extend_from_slice(&chunk[..remaining]);
            return Ok(true);
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

/// Execute a command in a container and return the output.
///
/// This is non-interactive — for one-shot commands like `ls`, `cat`, etc.
/// The command is executed via the Kubernetes exec sub-resource using WebSockets.
///
/// Safety limits:
/// - Output capped at [`MAX_OUTPUT_BYTES`] (10 MB) per stream.
/// - Entire operation times out after [`EXEC_TIMEOUT`] (5 minutes).
pub async fn exec_command(client: &Client, req: &ExecRequest) -> crate::Result<ExecResult> {
    match tokio::time::timeout(EXEC_TIMEOUT, exec_command_inner(client, req)).await {
        Ok(result) => result,
        Err(_) => Err(crate::EngineError::Other(format!(
            "Command timed out after {} minutes",
            EXEC_TIMEOUT.as_secs() / 60
        ))),
    }
}

async fn exec_command_inner(client: &Client, req: &ExecRequest) -> crate::Result<ExecResult> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), &req.namespace);

    let mut params = AttachParams {
        stdout: true,
        stderr: true,
        stdin: false,
        tty: false,
        ..Default::default()
    };
    if let Some(container) = &req.container {
        params.container = Some(container.clone());
    }

    let mut attached = pods.exec(&req.pod, &req.command, &params).await?;

    let mut stdout_buf = Vec::new();
    let mut stdout_truncated = false;
    if let Some(mut stdout) = attached.stdout() {
        stdout_truncated = bounded_read(&mut stdout, &mut stdout_buf, MAX_OUTPUT_BYTES)
            .await
            .map_err(|e| crate::EngineError::Other(format!("Failed to read stdout: {e}")))?;
    }

    let mut stderr_buf = Vec::new();
    let mut stderr_truncated = false;
    if let Some(mut stderr) = attached.stderr() {
        stderr_truncated = bounded_read(&mut stderr, &mut stderr_buf, MAX_OUTPUT_BYTES)
            .await
            .map_err(|e| crate::EngineError::Other(format!("Failed to read stderr: {e}")))?;
    }

    if stdout_truncated || stderr_truncated {
        return Err(crate::EngineError::Other(
            "Output exceeded 10MB limit".to_string(),
        ));
    }

    let status = match attached.take_status() {
        Some(status_future) => status_future.await,
        None => None,
    };
    let success = status
        .map(|s| s.status.as_deref() == Some("Success"))
        .unwrap_or(true);

    Ok(ExecResult {
        stdout: String::from_utf8_lossy(&stdout_buf).to_string(),
        stderr: String::from_utf8_lossy(&stderr_buf).to_string(),
        success,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_request_serialization() {
        let req = ExecRequest {
            namespace: "default".into(),
            pod: "my-pod".into(),
            container: Some("main".into()),
            command: vec!["ls".into(), "-la".into()],
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: ExecRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.namespace, "default");
        assert_eq!(deserialized.pod, "my-pod");
        assert_eq!(deserialized.container, Some("main".into()));
        assert_eq!(deserialized.command, vec!["ls", "-la"]);
    }

    #[test]
    fn exec_result_serialization() {
        let result = ExecResult {
            stdout: "hello\n".into(),
            stderr: String::new(),
            success: true,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ExecResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.stdout, "hello\n");
        assert!(deserialized.stderr.is_empty());
        assert!(deserialized.success);
    }

    #[test]
    fn max_output_bytes_is_10mb() {
        assert_eq!(MAX_OUTPUT_BYTES, 10 * 1024 * 1024);
    }

    #[test]
    fn exec_timeout_is_five_minutes() {
        assert_eq!(EXEC_TIMEOUT, Duration::from_secs(300));
    }

    #[tokio::test]
    async fn bounded_read_stops_at_limit() {
        let data = vec![0xABu8; 100];
        let mut cursor = std::io::Cursor::new(data);
        let mut buf = Vec::new();
        let truncated = bounded_read(&mut cursor, &mut buf, 50).await.unwrap();
        assert!(truncated);
        assert_eq!(buf.len(), 50);
    }

    #[tokio::test]
    async fn bounded_read_eof_before_limit() {
        let data = vec![0xCDu8; 30];
        let mut cursor = std::io::Cursor::new(data);
        let mut buf = Vec::new();
        let truncated = bounded_read(&mut cursor, &mut buf, 1024).await.unwrap();
        assert!(!truncated);
        assert_eq!(buf.len(), 30);
    }
}
