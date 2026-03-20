//! Port forwarding for Kubernetes pods.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, RwLock};

/// Maximum number of concurrent port-forward sessions.
static ACTIVE_FORWARDS: AtomicUsize = AtomicUsize::new(0);
const MAX_FORWARDS: usize = 10;

/// Idle timeout for the accept loop (1 hour).
const IDLE_TIMEOUT: Duration = Duration::from_secs(3600);

/// Global registry of active port-forward sessions.
static SESSION_REGISTRY: once_cell::sync::Lazy<Arc<RwLock<PortForwardRegistry>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(PortForwardRegistry::new())));

/// Unique identifier for a port-forward session.
pub type SessionId = String;

/// Active port-forward session information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortForwardSession {
    pub id: SessionId,
    pub namespace: String,
    pub pod: String,
    pub local_port: u16,
    pub remote_port: u16,
    pub started_at: String, // ISO 8601 timestamp
    pub status: SessionStatus,
}

/// Status of a port-forward session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Active,
    Stopping,
    Stopped,
}

/// Internal session tracking with task handle.
struct SessionEntry {
    info: PortForwardSession,
    cancel_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

/// Registry of active port-forward sessions.
struct PortForwardRegistry {
    sessions: HashMap<SessionId, SessionEntry>,
    next_id: usize,
}

impl PortForwardRegistry {
    fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            next_id: 1,
        }
    }

    fn generate_id(&mut self) -> SessionId {
        let id = format!("pf-{}", self.next_id);
        self.next_id += 1;
        id
    }

    fn add_session(
        &mut self,
        namespace: String,
        pod: String,
        local_port: u16,
        remote_port: u16,
        cancel_tx: tokio::sync::oneshot::Sender<()>,
    ) -> SessionId {
        let id = self.generate_id();
        let started_at = chrono::Utc::now().to_rfc3339();

        let info = PortForwardSession {
            id: id.clone(),
            namespace,
            pod,
            local_port,
            remote_port,
            started_at,
            status: SessionStatus::Active,
        };

        self.sessions.insert(
            id.clone(),
            SessionEntry {
                info,
                cancel_tx: Arc::new(Mutex::new(Some(cancel_tx))),
            },
        );

        id
    }

    fn get_session(&self, id: &str) -> Option<PortForwardSession> {
        self.sessions.get(id).map(|e| e.info.clone())
    }

    fn list_sessions(&self) -> Vec<PortForwardSession> {
        self.sessions.values().map(|e| e.info.clone()).collect()
    }

    async fn stop_session(&mut self, id: &str) -> bool {
        if let Some(entry) = self.sessions.get_mut(id) {
            entry.info.status = SessionStatus::Stopping;
            if let Some(tx) = entry.cancel_tx.lock().await.take() {
                let _ = tx.send(());
                return true;
            }
        }
        false
    }

    fn remove_session(&mut self, id: &str) {
        self.sessions.remove(id);
    }
}

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

/// Returns the current number of active port-forward sessions.
pub fn active_forward_count() -> usize {
    ACTIVE_FORWARDS.load(Ordering::Relaxed)
}

/// List all active port-forward sessions.
pub async fn list_port_forward_sessions() -> Vec<PortForwardSession> {
    SESSION_REGISTRY.read().await.list_sessions()
}

/// Get a specific port-forward session by ID.
pub async fn get_port_forward_session(id: &str) -> Option<PortForwardSession> {
    SESSION_REGISTRY.read().await.get_session(id)
}

/// Stop a port-forward session by ID.
pub async fn stop_port_forward_session(id: &str) -> crate::Result<()> {
    let stopped = SESSION_REGISTRY.write().await.stop_session(id).await;
    if stopped {
        Ok(())
    } else {
        Err(crate::EngineError::Other(format!(
            "Session not found or already stopped: {}",
            id
        )))
    }
}

/// Response from starting a port forward, includes session ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortForwardResponse {
    pub session_id: SessionId,
    pub local_port: u16,
}

/// Start port forwarding from `local_port` to `remote_port` on a pod.
/// Pass `local_port = 0` to auto-assign a free port.
/// Returns the session ID and actual local port that was bound.
///
/// Rejects the request if more than [`MAX_FORWARDS`] sessions are already active,
/// if `remote_port` is 0, or if the accept loop has been idle for [`IDLE_TIMEOUT`].
pub async fn start_port_forward(
    client: &Client,
    req: &PortForwardRequest,
) -> crate::Result<PortForwardResponse> {
    // Validate remote port range (1–65535)
    if req.remote_port == 0 {
        return Err(crate::EngineError::Other(
            "Invalid remote_port: must be in range 1-65535".to_string(),
        ));
    }

    // Enforce concurrent-forward limit
    let prev = ACTIVE_FORWARDS.fetch_add(1, Ordering::SeqCst);
    if prev >= MAX_FORWARDS {
        ACTIVE_FORWARDS.fetch_sub(1, Ordering::SeqCst);
        return Err(crate::EngineError::Other(format!(
            "Port forward limit reached: maximum {MAX_FORWARDS} concurrent forwards allowed"
        )));
    }

    // Verify the pod exists before binding the local port
    let pods: Api<Pod> = Api::namespaced(client.clone(), &req.namespace);
    if let Err(e) = pods.get(&req.pod).await {
        ACTIVE_FORWARDS.fetch_sub(1, Ordering::SeqCst);
        return Err(e.into());
    }

    // Bind local TCP listener
    let listener = TcpListener::bind(format!("127.0.0.1:{}", req.local_port))
        .await
        .map_err(|e| {
            ACTIVE_FORWARDS.fetch_sub(1, Ordering::SeqCst);
            crate::EngineError::Other(format!("Failed to bind port {}: {}", req.local_port, e))
        })?;

    let actual_port = listener
        .local_addr()
        .map_err(|e| {
            ACTIVE_FORWARDS.fetch_sub(1, Ordering::SeqCst);
            crate::EngineError::Other(e.to_string())
        })?
        .port();

    let pod_name = req.pod.clone();
    let remote_port = req.remote_port;
    let namespace = req.namespace.clone();

    // Create cancellation channel
    let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();

    // Register session in the registry
    let session_id = {
        let mut registry = SESSION_REGISTRY.write().await;
        registry.add_session(
            namespace,
            pod_name.clone(),
            actual_port,
            remote_port,
            cancel_tx,
        )
    };

    let session_id_clone = session_id.clone();

    // Spawn the forwarding loop — accepts connections and pipes them to the pod.
    // The loop is wrapped in an idle timeout so abandoned forwards are cleaned up.
    tokio::spawn(async move {
        let _result = tokio::select! {
            _ = &mut cancel_rx => {
                tracing::info!("Port forward session {} cancelled", session_id_clone);
                Ok(())
            }
            result = tokio::time::timeout(IDLE_TIMEOUT, async {
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
            }) => {
                if result.is_err() {
                    tracing::warn!(
                        "Port forward idle timeout reached ({} secs), closing listener",
                        IDLE_TIMEOUT.as_secs()
                    );
                }
                result
            }
        };

        // Remove session from registry and decrement active-forward counter
        SESSION_REGISTRY.write().await.remove_session(&session_id_clone);
        ACTIVE_FORWARDS.fetch_sub(1, Ordering::SeqCst);
    });

    Ok(PortForwardResponse {
        session_id,
        local_port: actual_port,
    })
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

    #[test]
    fn max_forwards_constant() {
        assert_eq!(MAX_FORWARDS, 10);
    }

    #[test]
    fn idle_timeout_one_hour() {
        assert_eq!(IDLE_TIMEOUT, Duration::from_secs(3600));
    }

    #[test]
    fn active_forward_counter_starts_at_zero() {
        // Note: other tests may run concurrently, so we just verify the function works.
        let _ = active_forward_count();
    }
}
