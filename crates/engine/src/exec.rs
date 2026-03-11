//! Container exec functionality.
//!
//! Provides non-interactive command execution in running containers.
//! Full interactive TTY support (xterm.js) is deferred to M4.

use k8s_openapi::api::core::v1::Pod;
use kube::{api::AttachParams, Api, Client};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

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

/// Execute a command in a container and return the output.
///
/// This is non-interactive — for one-shot commands like `ls`, `cat`, etc.
/// The command is executed via the Kubernetes exec sub-resource using WebSockets.
pub async fn exec_command(client: &Client, req: &ExecRequest) -> crate::Result<ExecResult> {
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
    if let Some(mut stdout) = attached.stdout() {
        stdout
            .read_to_end(&mut stdout_buf)
            .await
            .map_err(|e| crate::EngineError::Other(format!("Failed to read stdout: {e}")))?;
    }

    let mut stderr_buf = Vec::new();
    if let Some(mut stderr) = attached.stderr() {
        stderr
            .read_to_end(&mut stderr_buf)
            .await
            .map_err(|e| crate::EngineError::Other(format!("Failed to read stderr: {e}")))?;
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
}
