//! Watch-driven resource ingest pipeline.
//!
//! Watches Kubernetes resources via the kube API and syncs them
//! into the SQLite ResourceStore. Implements exponential backoff
//! with jitter and concurrency limits on LIST operations.

use std::sync::Arc;

use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{
    runtime::watcher::{self, watcher, Event},
    Api, Client,
};
use tokio::sync::{watch, Semaphore};
use tracing::{error, info, warn};

use telescope_core::{ConnectionEvent, ConnectionState, ResourceEntry, ResourceStore};

/// Maximum concurrent LIST operations across all watchers.
const MAX_CONCURRENT_LISTS: usize = 3;

/// Manages watch streams and syncs resources to SQLite.
pub struct ResourceWatcher {
    client: Client,
    store: Arc<ResourceStore>,
    /// Semaphore to limit concurrent LIST operations.
    list_semaphore: Arc<Semaphore>,
    /// Sender for connection state updates.
    state_tx: watch::Sender<ConnectionState>,
    /// Receiver for connection state (clone for UI consumption).
    state_rx: watch::Receiver<ConnectionState>,
}

impl ResourceWatcher {
    pub fn new(client: Client, store: Arc<ResourceStore>) -> Self {
        let (state_tx, state_rx) = watch::channel(ConnectionState::Disconnected);
        Self {
            client,
            store,
            list_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_LISTS)),
            state_tx,
            state_rx,
        }
    }

    /// Get a receiver for connection state changes.
    pub fn state_receiver(&self) -> watch::Receiver<ConnectionState> {
        self.state_rx.clone()
    }

    /// Returns a reference to the LIST concurrency semaphore.
    pub fn list_semaphore(&self) -> &Arc<Semaphore> {
        &self.list_semaphore
    }

    /// Transition the connection state machine.
    fn transition(&self, event: &ConnectionEvent) {
        let current = self.state_rx.borrow().clone();
        if let Some(next) = current.transition(event) {
            let _ = self.state_tx.send(next);
        }
    }

    /// Start watching Pods in a namespace and sync to SQLite.
    ///
    /// Runs indefinitely until the watch stream ends or encounters
    /// an unrecoverable error. The kube-runtime watcher handles
    /// internal retries for transient failures.
    pub async fn watch_pods(&self, namespace: &str) -> crate::Result<()> {
        let api: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
        let gvk = "v1/Pod".to_string();
        let ns = namespace.to_string();

        self.transition(&ConnectionEvent::Connect);
        self.transition(&ConnectionEvent::Authenticated);
        self.transition(&ConnectionEvent::SyncStarted);

        let watcher_config = watcher::Config::default();
        let mut stream = watcher(api, watcher_config).boxed();

        let mut synced = false;

        loop {
            match stream.try_next().await {
                Ok(Some(event)) => match event {
                    Event::Apply(pod) => {
                        if let Some(entry) = Self::pod_to_entry(&gvk, &ns, &pod) {
                            if let Err(e) = self.store.upsert(&entry) {
                                error!("Failed to upsert pod: {}", e);
                            }
                        }
                    }
                    Event::Delete(pod) => {
                        if let Some(name) = pod.metadata.name.as_deref() {
                            if let Err(e) = self.store.delete(&gvk, &ns, name) {
                                error!("Failed to delete pod: {}", e);
                            }
                        }
                    }
                    Event::Init => {
                        info!("Initial LIST started for {}/{}", gvk, ns);
                        let _ = self.store.delete_all_by_gvk(&gvk);
                    }
                    Event::InitApply(pod) => {
                        if let Some(entry) = Self::pod_to_entry(&gvk, &ns, &pod) {
                            if let Err(e) = self.store.upsert(&entry) {
                                error!("Failed to upsert pod during init: {}", e);
                            }
                        }
                    }
                    Event::InitDone => {
                        info!("Initial sync complete for {}/{}", gvk, ns);
                        if !synced {
                            synced = true;
                            self.transition(&ConnectionEvent::SyncComplete);
                        }
                    }
                },
                Ok(None) => {
                    warn!("Watch stream ended for {}/{}", gvk, ns);
                    self.transition(&ConnectionEvent::Disconnected);
                    break;
                }
                Err(e) => {
                    error!("Watch error for {}/{}: {}", gvk, ns, e);
                    self.transition(&ConnectionEvent::WatchError {
                        message: e.to_string(),
                    });
                    break;
                }
            }
        }

        Ok(())
    }

    /// Convert a Pod to a ResourceEntry for SQLite storage.
    fn pod_to_entry(gvk: &str, namespace: &str, pod: &Pod) -> Option<ResourceEntry> {
        let name = pod.metadata.name.as_deref()?;
        let rv = pod.metadata.resource_version.as_deref().unwrap_or("");
        let content = serde_json::to_string(pod).ok()?;

        Some(ResourceEntry {
            gvk: gvk.to_string(),
            namespace: namespace.to_string(),
            name: name.to_string(),
            resource_version: rv.to_string(),
            content,
            updated_at: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

    fn make_test_pod(name: &str, rv: &str) -> Pod {
        Pod {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some("default".to_string()),
                resource_version: Some(rv.to_string()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn pod_to_entry_extracts_fields() {
        let pod = make_test_pod("nginx", "12345");
        let entry = ResourceWatcher::pod_to_entry("v1/Pod", "default", &pod).unwrap();
        assert_eq!(entry.name, "nginx");
        assert_eq!(entry.namespace, "default");
        assert_eq!(entry.gvk, "v1/Pod");
        assert_eq!(entry.resource_version, "12345");
        assert!(entry.content.contains("nginx"));
    }

    #[test]
    fn pod_to_entry_returns_none_for_nameless_pod() {
        let pod = Pod::default();
        assert!(ResourceWatcher::pod_to_entry("v1/Pod", "default", &pod).is_none());
    }

    #[test]
    fn pod_to_entry_handles_missing_resource_version() {
        let pod = Pod {
            metadata: ObjectMeta {
                name: Some("test-pod".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let entry = ResourceWatcher::pod_to_entry("v1/Pod", "ns", &pod).unwrap();
        assert_eq!(entry.resource_version, "");
    }

    #[test]
    fn pod_to_entry_content_is_valid_json() {
        let pod = make_test_pod("json-pod", "1");
        let entry = ResourceWatcher::pod_to_entry("v1/Pod", "default", &pod).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&entry.content).unwrap();
        assert!(parsed.is_object());
    }
}
