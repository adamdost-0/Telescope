//! Port forwarding for Kubernetes pods.

use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortForwardRequest {
    pub namespace: String,
    pub pod: String,
    pub local_port: u16,
    pub remote_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortForwardStatus {
    pub active: bool,
    pub local_port: u16,
    pub remote_port: u16,
    pub pod: String,
    pub namespace: String,
    pub error: Option<String>,
}

/// Start port forwarding from `local_port` to `remote_port` on a pod.
/// Pass `local_port = 0` to auto-assign a free port.
/// Returns the actual local port that was bound.
pub async fn start_port_forward(client: &Client, req: &PortForwardRequest) -> crate::Result<u16> {
    // Verify the pod exists before binding the local port
    let pods: Api<Pod> = Api::namespaced(client.clone(), &req.namespace);
    pods.get(&req.pod).await?;

    // Bind local TCP listener
    let listener = TcpListener::bind(format!("127.0.0.1:{}", req.local_port))
        .await
        .map_err(|e| {
            crate::EngineError::Other(format!("Failed to bind port {}: {}", req.local_port, e))
        })?;

    let actual_port = listener
        .local_addr()
        .map_err(|e| crate::EngineError::Other(e.to_string()))?
        .port();

    let pod_name = req.pod.clone();
    let remote_port = req.remote_port;

    // Spawn the forwarding loop — accepts connections and pipes them to the pod
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((tcp_stream, _addr)) => {
                    let pods_clone = pods.clone();
                    let pod_clone = pod_name.clone();
                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_connection(&pods_clone, &pod_clone, remote_port, tcp_stream)
                                .await
                        {
                            tracing::error!("Port forward connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Port forward accept error: {}", e);
                    break;
                }
            }
        }
    });

    Ok(actual_port)
}

/// Handle a single forwarded TCP connection by piping it to the pod via
/// the Kubernetes API server websocket tunnel.
async fn handle_connection(
    pods: &Api<Pod>,
    pod_name: &str,
    port: u16,
    mut tcp_stream: tokio::net::TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pf = pods.portforward(pod_name, &[port]).await?;
    let mut upstream = pf
        .take_stream(port)
        .ok_or("Failed to get port forward stream")?;

    tokio::io::copy_bidirectional(&mut tcp_stream, &mut upstream).await?;

    // Drive the port-forwarder to completion so the websocket is closed cleanly.
    pf.join().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_forward_request_serializes() {
        let req = PortForwardRequest {
            namespace: "default".to_string(),
            pod: "my-pod".to_string(),
            local_port: 8080,
            remote_port: 80,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"local_port\":8080"));
        assert!(json.contains("\"remote_port\":80"));
    }

    #[test]
    fn port_forward_status_serializes() {
        let status = PortForwardStatus {
            active: true,
            local_port: 9090,
            remote_port: 80,
            pod: "my-pod".to_string(),
            namespace: "default".to_string(),
            error: None,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"active\":true"));
        let deser: PortForwardStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.local_port, 9090);
    }
}
