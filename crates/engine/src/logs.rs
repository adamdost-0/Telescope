//! Pod log streaming.

use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, LogParams},
    Client,
};
use serde::{Deserialize, Serialize};

/// Parameters for a log stream request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRequest {
    pub namespace: String,
    pub pod: String,
    pub container: Option<String>,
    pub previous: bool,
    pub tail_lines: Option<i64>,
    pub follow: bool,
}

/// A chunk of log output emitted to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogChunk {
    pub lines: String,
    pub is_complete: bool,
}

/// Build [`LogParams`] from a [`LogRequest`].
fn build_log_params(req: &LogRequest) -> LogParams {
    LogParams {
        container: req.container.clone(),
        follow: req.follow,
        previous: req.previous,
        tail_lines: req.tail_lines,
        ..Default::default()
    }
}

/// Fetch pod logs (non-streaming, returns all at once).
pub async fn get_pod_logs(client: &Client, req: &LogRequest) -> crate::Result<String> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), &req.namespace);
    let params = build_log_params(req);
    let logs = pods.logs(&req.pod, &params).await?;
    Ok(logs)
}

/// Start a streaming log follow. Returns an `AsyncBufRead` reader.
///
/// Callers should read lines from the returned reader and forward them to the
/// UI (e.g. via Tauri events).
pub async fn stream_pod_logs(
    client: &Client,
    req: &LogRequest,
) -> crate::Result<impl futures::AsyncBufRead> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), &req.namespace);
    let mut params = build_log_params(req);
    params.follow = true;
    let reader = pods.log_stream(&req.pod, &params).await?;
    Ok(reader)
}

/// List containers in a pod (regular + init).
pub async fn list_containers(
    client: &Client,
    namespace: &str,
    pod: &str,
) -> crate::Result<Vec<String>> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);
    let pod_obj = pods.get(pod).await?;

    let mut containers = Vec::new();
    if let Some(spec) = &pod_obj.spec {
        for c in &spec.containers {
            containers.push(c.name.clone());
        }
        if let Some(init) = &spec.init_containers {
            for c in init {
                containers.push(format!("init:{}", c.name));
            }
        }
    }
    Ok(containers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_log_params_defaults() {
        let req = LogRequest {
            namespace: "default".into(),
            pod: "my-pod".into(),
            container: None,
            previous: false,
            tail_lines: None,
            follow: false,
        };
        let params = build_log_params(&req);
        assert!(!params.follow);
        assert!(!params.previous);
        assert!(params.container.is_none());
        assert!(params.tail_lines.is_none());
    }

    #[test]
    fn build_log_params_with_options() {
        let req = LogRequest {
            namespace: "kube-system".into(),
            pod: "coredns-abc".into(),
            container: Some("coredns".into()),
            previous: true,
            tail_lines: Some(500),
            follow: true,
        };
        let params = build_log_params(&req);
        assert!(params.follow);
        assert!(params.previous);
        assert_eq!(params.container.as_deref(), Some("coredns"));
        assert_eq!(params.tail_lines, Some(500));
    }

    #[test]
    fn log_request_serde_roundtrip() {
        let req = LogRequest {
            namespace: "ns".into(),
            pod: "pod".into(),
            container: Some("main".into()),
            previous: false,
            tail_lines: Some(100),
            follow: true,
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: LogRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.namespace, "ns");
        assert_eq!(parsed.container.as_deref(), Some("main"));
        assert_eq!(parsed.tail_lines, Some(100));
        assert!(parsed.follow);
    }

    #[test]
    fn log_chunk_serde() {
        let chunk = LogChunk {
            lines: "hello world\n".into(),
            is_complete: false,
        };
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("hello world"));
        let parsed: LogChunk = serde_json::from_str(&json).unwrap();
        assert!(!parsed.is_complete);
    }

    #[test]
    fn list_containers_extracts_names() {
        // Verify container name extraction logic with a manually built Pod spec.
        use k8s_openapi::api::core::v1::{Container, PodSpec};

        let spec = PodSpec {
            containers: vec![
                Container {
                    name: "app".into(),
                    ..Default::default()
                },
                Container {
                    name: "sidecar".into(),
                    ..Default::default()
                },
            ],
            init_containers: Some(vec![Container {
                name: "setup".into(),
                ..Default::default()
            }]),
            ..Default::default()
        };

        // Extract names using the same logic as list_containers.
        let mut names = Vec::new();
        for c in &spec.containers {
            names.push(c.name.clone());
        }
        if let Some(init) = &spec.init_containers {
            for c in init {
                names.push(format!("init:{}", c.name));
            }
        }
        assert_eq!(names, vec!["app", "sidecar", "init:setup"]);
    }
}
