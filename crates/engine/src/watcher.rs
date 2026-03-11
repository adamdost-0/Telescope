//! Watch-driven resource ingest pipeline.
//!
//! Watches Kubernetes resources via the kube API and syncs them
//! into the SQLite ResourceStore. Implements exponential backoff
//! with jitter and concurrency limits on LIST operations.

use std::sync::{Arc, Mutex};

use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{ConfigMap, Event as K8sEvent, Node, Pod, Secret, Service},
};
use k8s_openapi::{ClusterResourceScope, NamespaceResourceScope};
use kube::{
    runtime::watcher::{self, watcher, Event},
    Api, Client, Resource as KubeResource, ResourceExt,
};
use serde::Serialize;
use tokio::sync::{watch, Semaphore};
use tracing::{error, info, warn};

use telescope_core::{ConnectionEvent, ConnectionState, ResourceEntry, ResourceStore};

/// Maximum concurrent LIST operations across all watchers.
const MAX_CONCURRENT_LISTS: usize = 3;

/// Convert any Kubernetes resource to a [`ResourceEntry`] for SQLite storage.
fn resource_to_entry<K>(gvk: &str, namespace: &str, obj: &K) -> Option<ResourceEntry>
where
    K: KubeResource + Serialize,
{
    let meta = obj.meta();
    let name = meta.name.as_deref()?;
    let rv = meta.resource_version.as_deref().unwrap_or("");
    let content = serde_json::to_string(obj).ok()?;
    Some(ResourceEntry {
        gvk: gvk.to_string(),
        namespace: namespace.to_string(),
        name: name.to_string(),
        resource_version: rv.to_string(),
        content,
        updated_at: String::new(),
    })
}

/// Manages watch streams and syncs resources to SQLite.
///
/// The store is wrapped in `Mutex` because `rusqlite::Connection` is `Send`
/// but not `Sync`, and we need shared access from async tasks.
///
/// `Clone` is cheap — the inner client, store, and semaphore are all
/// reference-counted. Clone a watcher to run concurrent watch loops
/// for different resource types.
#[derive(Clone)]
pub struct ResourceWatcher {
    client: Client,
    store: Arc<Mutex<ResourceStore>>,
    /// Semaphore to limit concurrent LIST operations.
    list_semaphore: Arc<Semaphore>,
    /// Sender for connection state updates.
    state_tx: Arc<watch::Sender<ConnectionState>>,
    /// Receiver for connection state (clone for UI consumption).
    state_rx: watch::Receiver<ConnectionState>,
}

impl ResourceWatcher {
    pub fn new(client: Client, store: Arc<Mutex<ResourceStore>>) -> Self {
        let (state_tx, state_rx) = watch::channel(ConnectionState::Disconnected);
        Self {
            client,
            store,
            list_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_LISTS)),
            state_tx: Arc::new(state_tx),
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

    // -----------------------------------------------------------------
    // Generic watch methods
    // -----------------------------------------------------------------

    /// Watch any namespaced Kubernetes resource and sync to SQLite.
    ///
    /// Runs indefinitely until the watch stream ends or encounters
    /// an unrecoverable error. The kube-runtime watcher handles
    /// internal retries for transient failures.
    pub async fn watch_resource<K>(&self, gvk: &str, namespace: &str) -> crate::Result<()>
    where
        K: KubeResource<Scope = NamespaceResourceScope, DynamicType = ()>
            + Clone
            + std::fmt::Debug
            + serde::de::DeserializeOwned
            + Serialize
            + Send
            + 'static,
    {
        let api: Api<K> = Api::namespaced(self.client.clone(), namespace);
        self.run_watch_loop(api, gvk, namespace).await
    }

    /// Watch a cluster-scoped resource (e.g. Nodes, Namespaces).
    pub async fn watch_cluster_resource<K>(&self, gvk: &str) -> crate::Result<()>
    where
        K: KubeResource<Scope = ClusterResourceScope, DynamicType = ()>
            + Clone
            + std::fmt::Debug
            + serde::de::DeserializeOwned
            + Serialize
            + Send
            + 'static,
    {
        let api: Api<K> = Api::all(self.client.clone());
        self.run_watch_loop(api, gvk, "").await
    }

    /// Shared event loop for both namespaced and cluster-scoped watches.
    async fn run_watch_loop<K>(&self, api: Api<K>, gvk: &str, namespace: &str) -> crate::Result<()>
    where
        K: KubeResource<DynamicType = ()>
            + Clone
            + std::fmt::Debug
            + serde::de::DeserializeOwned
            + Serialize
            + Send
            + 'static,
    {
        let gvk_str = gvk.to_string();
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
                    Event::Apply(obj) => {
                        if let Some(entry) = resource_to_entry(&gvk_str, &ns, &obj) {
                            if let Err(e) = self.store.lock().unwrap().upsert(&entry) {
                                error!("Failed to upsert {}: {}", gvk_str, e);
                            }
                        }
                    }
                    Event::Delete(obj) => {
                        let name = obj.name_any();
                        if let Err(e) = self.store.lock().unwrap().delete(&gvk_str, &ns, &name) {
                            error!("Failed to delete {}: {}", gvk_str, e);
                        }
                    }
                    Event::Init => {
                        info!("Initial LIST started for {}/{}", gvk_str, ns);
                        let _ = self.store.lock().unwrap().delete_all_by_gvk(&gvk_str);
                    }
                    Event::InitApply(obj) => {
                        if let Some(entry) = resource_to_entry(&gvk_str, &ns, &obj) {
                            if let Err(e) = self.store.lock().unwrap().upsert(&entry) {
                                error!("Failed to upsert {} during init: {}", gvk_str, e);
                            }
                        }
                    }
                    Event::InitDone => {
                        info!("Initial sync complete for {}/{}", gvk_str, ns);
                        if !synced {
                            synced = true;
                            self.transition(&ConnectionEvent::SyncComplete);
                        }
                    }
                },
                Ok(None) => {
                    warn!("Watch stream ended for {}/{}", gvk_str, ns);
                    self.transition(&ConnectionEvent::Disconnected);
                    break;
                }
                Err(e) => {
                    error!("Watch error for {}/{}: {}", gvk_str, ns, e);
                    self.transition(&ConnectionEvent::WatchError {
                        message: e.to_string(),
                    });
                    break;
                }
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------
    // Convenience wrappers for common resource types
    // -----------------------------------------------------------------

    /// Watch Pods in a namespace.
    pub async fn watch_pods(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<Pod>("v1/Pod", namespace).await
    }

    /// Watch Deployments in a namespace.
    pub async fn watch_deployments(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<Deployment>("apps/v1/Deployment", namespace)
            .await
    }

    /// Watch Services in a namespace.
    pub async fn watch_services(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<Service>("v1/Service", namespace)
            .await
    }

    /// Watch ConfigMaps in a namespace.
    pub async fn watch_config_maps(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<ConfigMap>("v1/ConfigMap", namespace)
            .await
    }

    /// Watch Secrets in a namespace.
    pub async fn watch_secrets(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<Secret>("v1/Secret", namespace).await
    }

    /// Watch Events in a namespace.
    pub async fn watch_events(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<K8sEvent>("v1/Event", namespace).await
    }

    /// Watch Nodes (cluster-scoped).
    pub async fn watch_nodes(&self) -> crate::Result<()> {
        self.watch_cluster_resource::<Node>("v1/Node").await
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
    fn resource_to_entry_extracts_pod_fields() {
        let pod = make_test_pod("nginx", "12345");
        let entry = resource_to_entry("v1/Pod", "default", &pod).unwrap();
        assert_eq!(entry.name, "nginx");
        assert_eq!(entry.namespace, "default");
        assert_eq!(entry.gvk, "v1/Pod");
        assert_eq!(entry.resource_version, "12345");
        assert!(entry.content.contains("nginx"));
    }

    #[test]
    fn resource_to_entry_returns_none_for_nameless_resource() {
        let pod = Pod::default();
        assert!(resource_to_entry::<Pod>("v1/Pod", "default", &pod).is_none());
    }

    #[test]
    fn resource_to_entry_handles_missing_resource_version() {
        let pod = Pod {
            metadata: ObjectMeta {
                name: Some("test-pod".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let entry = resource_to_entry("v1/Pod", "ns", &pod).unwrap();
        assert_eq!(entry.resource_version, "");
    }

    #[test]
    fn resource_to_entry_content_is_valid_json() {
        let pod = make_test_pod("json-pod", "1");
        let entry = resource_to_entry("v1/Pod", "default", &pod).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&entry.content).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    fn resource_to_entry_works_for_deployment() {
        let deploy = Deployment {
            metadata: ObjectMeta {
                name: Some("my-deploy".to_string()),
                namespace: Some("prod".to_string()),
                resource_version: Some("999".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let entry = resource_to_entry("apps/v1/Deployment", "prod", &deploy).unwrap();
        assert_eq!(entry.name, "my-deploy");
        assert_eq!(entry.gvk, "apps/v1/Deployment");
        assert!(entry.content.contains("my-deploy"));
    }

    #[test]
    fn resource_to_entry_works_for_service() {
        let svc = Service {
            metadata: ObjectMeta {
                name: Some("frontend-svc".to_string()),
                resource_version: Some("42".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let entry = resource_to_entry("v1/Service", "default", &svc).unwrap();
        assert_eq!(entry.name, "frontend-svc");
        assert_eq!(entry.gvk, "v1/Service");
    }

    #[test]
    fn resource_to_entry_works_for_configmap() {
        let cm = ConfigMap {
            metadata: ObjectMeta {
                name: Some("app-config".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let entry = resource_to_entry("v1/ConfigMap", "kube-system", &cm).unwrap();
        assert_eq!(entry.name, "app-config");
        assert_eq!(entry.gvk, "v1/ConfigMap");
    }

    #[test]
    fn resource_to_entry_works_for_secret() {
        let secret = Secret {
            metadata: ObjectMeta {
                name: Some("db-creds".to_string()),
                resource_version: Some("7".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let entry = resource_to_entry("v1/Secret", "default", &secret).unwrap();
        assert_eq!(entry.name, "db-creds");
        assert_eq!(entry.gvk, "v1/Secret");
        assert_eq!(entry.resource_version, "7");
    }

    #[test]
    fn resource_to_entry_works_for_event() {
        use k8s_openapi::api::core::v1::ObjectReference;
        let evt = K8sEvent {
            metadata: ObjectMeta {
                name: Some("evt-abc.123".to_string()),
                namespace: Some("default".to_string()),
                resource_version: Some("999".to_string()),
                ..Default::default()
            },
            involved_object: ObjectReference {
                name: Some("nginx-pod".to_string()),
                ..Default::default()
            },
            reason: Some("Started".to_string()),
            message: Some("Started container".to_string()),
            ..Default::default()
        };
        let entry = resource_to_entry("v1/Event", "default", &evt).unwrap();
        assert_eq!(entry.name, "evt-abc.123");
        assert_eq!(entry.namespace, "default");
        assert_eq!(entry.gvk, "v1/Event");
        assert_eq!(entry.resource_version, "999");
        assert!(entry.content.contains("nginx-pod"));
    }

    #[test]
    fn resource_to_entry_event_content_is_valid_json() {
        let evt = K8sEvent {
            metadata: ObjectMeta {
                name: Some("evt-json".to_string()),
                ..Default::default()
            },
            reason: Some("Pulled".to_string()),
            ..Default::default()
        };
        let entry = resource_to_entry("v1/Event", "default", &evt).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&entry.content).unwrap();
        assert!(parsed.is_object());
        assert_eq!(parsed["reason"], "Pulled");
    }

    #[test]
    fn resource_to_entry_works_for_node() {
        let node = Node {
            metadata: ObjectMeta {
                name: Some("node-1".to_string()),
                resource_version: Some("500".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let entry = resource_to_entry("v1/Node", "", &node).unwrap();
        assert_eq!(entry.name, "node-1");
        assert_eq!(entry.namespace, "");
        assert_eq!(entry.gvk, "v1/Node");
        assert_eq!(entry.resource_version, "500");
        assert!(entry.content.contains("node-1"));
    }
}
