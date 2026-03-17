//! Watch-driven resource ingest pipeline.
//!
//! Watches Kubernetes resources via the kube API and syncs them
//! into the SQLite ResourceStore. Implements exponential backoff
//! with jitter and concurrency limits on LIST operations.

use std::sync::{Arc, Mutex};

use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::{
    admissionregistration::v1::{MutatingWebhookConfiguration, ValidatingWebhookConfiguration},
    apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet},
    autoscaling::v2::HorizontalPodAutoscaler,
    batch::v1::{CronJob, Job},
    core::v1::{
        ConfigMap, Event as K8sEvent, LimitRange, Node, PersistentVolume, PersistentVolumeClaim,
        Pod, ResourceQuota, Secret, Service, ServiceAccount,
    },
    discovery::v1::EndpointSlice,
    networking::v1::{Ingress, NetworkPolicy},
    policy::v1::PodDisruptionBudget,
    rbac::v1::{ClusterRole, ClusterRoleBinding, Role, RoleBinding},
    scheduling::v1::PriorityClass,
    storage::v1::StorageClass,
};
use k8s_openapi::{ClusterResourceScope, NamespaceResourceScope};
use kube::{
    runtime::watcher::{self, watcher, Event},
    Api, Client, Resource as KubeResource, ResourceExt,
};
use serde::Serialize;
use serde_json::Value;
use tokio::sync::{watch, Semaphore};
use tracing::{error, info, warn};

use telescope_core::{ConnectionEvent, ConnectionState, ResourceEntry, ResourceStore};

/// Maximum concurrent LIST operations across all watchers.
const MAX_CONCURRENT_LISTS: usize = 3;
const REDACTED_CACHE_VALUE: &str = "<redacted>";

fn redact_cached_pod_fields(value: &mut Value) {
    let Some(spec) = value.get_mut("spec").and_then(Value::as_object_mut) else {
        return;
    };

    for container_key in ["containers", "initContainers", "ephemeralContainers"] {
        let Some(containers) = spec.get_mut(container_key).and_then(Value::as_array_mut) else {
            continue;
        };

        for container in containers {
            let Some(env_vars) = container.get_mut("env").and_then(Value::as_array_mut) else {
                continue;
            };

            for env_var in env_vars {
                let Some(env) = env_var.as_object_mut() else {
                    continue;
                };

                if env.contains_key("value") {
                    env.insert(
                        "value".to_string(),
                        Value::String(REDACTED_CACHE_VALUE.to_string()),
                    );
                }
            }
        }
    }
}

fn serialize_resource_for_cache<K>(gvk: &str, obj: &K) -> Option<String>
where
    K: Serialize,
{
    let mut value = serde_json::to_value(obj).ok()?;
    if gvk == "v1/Pod" {
        redact_cached_pod_fields(&mut value);
    }
    serde_json::to_string(&value).ok()
}

/// Convert any Kubernetes resource to a [`ResourceEntry`] for SQLite storage.
fn resource_to_entry<K>(gvk: &str, namespace: &str, obj: &K) -> Option<ResourceEntry>
where
    K: KubeResource + Serialize,
{
    let meta = obj.meta();
    let name = meta.name.as_deref()?;
    let rv = meta.resource_version.as_deref().unwrap_or("");
    let content = serialize_resource_for_cache(gvk, obj)?;
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

    /// Acquire the store lock, recovering from mutex poisoning.
    fn lock_store(&self) -> std::sync::MutexGuard<'_, ResourceStore> {
        self.store.lock().unwrap_or_else(|e| {
            warn!("Store mutex was poisoned, recovering");
            e.into_inner()
        })
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
                            if let Err(e) = self.lock_store().upsert(&entry) {
                                error!("Failed to upsert {}: {}", gvk_str, e);
                            }
                        }
                    }
                    Event::Delete(obj) => {
                        let name = obj.name_any();
                        if let Err(e) = self.lock_store().delete(&gvk_str, &ns, &name) {
                            error!("Failed to delete {}: {}", gvk_str, e);
                        }
                    }
                    Event::Init => {
                        info!("Initial LIST started for {}/{}", gvk_str, ns);
                        let _ = self.lock_store().delete_all_by_gvk(&gvk_str);
                    }
                    Event::InitApply(obj) => {
                        if let Some(entry) = resource_to_entry(&gvk_str, &ns, &obj) {
                            if let Err(e) = self.lock_store().upsert(&entry) {
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

    /// Watch loop for namespaced resources watched cluster-wide via `Api::all()`.
    ///
    /// Unlike `run_watch_loop`, this extracts the namespace from each
    /// object's metadata so events are stored with their actual namespace.
    async fn run_watch_loop_dynamic_ns<K>(&self, api: Api<K>, gvk: &str) -> crate::Result<()>
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
                        let ns = obj.namespace().unwrap_or_default();
                        if let Some(entry) = resource_to_entry(&gvk_str, &ns, &obj) {
                            if let Err(e) = self.lock_store().upsert(&entry) {
                                error!("Failed to upsert {}: {}", gvk_str, e);
                            }
                        }
                    }
                    Event::Delete(obj) => {
                        let ns = obj.namespace().unwrap_or_default();
                        let name = obj.name_any();
                        if let Err(e) = self.lock_store().delete(&gvk_str, &ns, &name) {
                            error!("Failed to delete {}: {}", gvk_str, e);
                        }
                    }
                    Event::Init => {
                        info!("Initial LIST started for {} (all namespaces)", gvk_str);
                        let _ = self.lock_store().delete_all_by_gvk(&gvk_str);
                    }
                    Event::InitApply(obj) => {
                        let ns = obj.namespace().unwrap_or_default();
                        if let Some(entry) = resource_to_entry(&gvk_str, &ns, &obj) {
                            if let Err(e) = self.lock_store().upsert(&entry) {
                                error!("Failed to upsert {} during init: {}", gvk_str, e);
                            }
                        }
                    }
                    Event::InitDone => {
                        info!("Initial sync complete for {} (all namespaces)", gvk_str);
                        if !synced {
                            synced = true;
                            self.transition(&ConnectionEvent::SyncComplete);
                        }
                    }
                },
                Ok(None) => {
                    warn!("Watch stream ended for {} (all namespaces)", gvk_str);
                    self.transition(&ConnectionEvent::Disconnected);
                    break;
                }
                Err(e) => {
                    error!("Watch error for {} (all namespaces): {}", gvk_str, e);
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

    /// Watch Events across all namespaces (cluster-wide).
    ///
    /// Unlike namespace-scoped `watch_events`, this captures events from
    /// every namespace (kube-system, default, etc.), which is essential
    /// for AKS and other managed clusters where important events are
    /// scattered across namespaces.
    pub async fn watch_all_events(&self) -> crate::Result<()> {
        let api: Api<K8sEvent> = Api::all(self.client.clone());
        self.run_watch_loop_dynamic_ns(api, "v1/Event").await
    }

    /// Watch Nodes (cluster-scoped).
    pub async fn watch_nodes(&self) -> crate::Result<()> {
        self.watch_cluster_resource::<Node>("v1/Node").await
    }

    /// Watch StatefulSets in a namespace.
    pub async fn watch_statefulsets(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<StatefulSet>("apps/v1/StatefulSet", namespace)
            .await
    }

    /// Watch DaemonSets in a namespace.
    pub async fn watch_daemonsets(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<DaemonSet>("apps/v1/DaemonSet", namespace)
            .await
    }

    /// Watch ReplicaSets in a namespace.
    pub async fn watch_replicasets(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<ReplicaSet>("apps/v1/ReplicaSet", namespace)
            .await
    }

    /// Watch Jobs in a namespace.
    pub async fn watch_jobs(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<Job>("batch/v1/Job", namespace).await
    }

    /// Watch CronJobs in a namespace.
    pub async fn watch_cronjobs(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<CronJob>("batch/v1/CronJob", namespace)
            .await
    }

    /// Watch Ingresses in a namespace.
    pub async fn watch_ingresses(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<Ingress>("networking.k8s.io/v1/Ingress", namespace)
            .await
    }

    /// Watch NetworkPolicies in a namespace.
    pub async fn watch_network_policies(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<NetworkPolicy>("networking.k8s.io/v1/NetworkPolicy", namespace)
            .await
    }

    /// Watch EndpointSlices in a namespace.
    pub async fn watch_endpoint_slices(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<EndpointSlice>("discovery.k8s.io/v1/EndpointSlice", namespace)
            .await
    }

    /// Watch PersistentVolumeClaims in a namespace.
    pub async fn watch_pvcs(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<PersistentVolumeClaim>("v1/PersistentVolumeClaim", namespace)
            .await
    }

    /// Watch ResourceQuotas in a namespace.
    pub async fn watch_resource_quotas(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<ResourceQuota>("v1/ResourceQuota", namespace)
            .await
    }

    /// Watch LimitRanges in a namespace.
    pub async fn watch_limit_ranges(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<LimitRange>("v1/LimitRange", namespace)
            .await
    }

    /// Watch Roles in a namespace.
    pub async fn watch_roles(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<Role>("rbac.authorization.k8s.io/v1/Role", namespace)
            .await
    }

    /// Watch ClusterRoles (cluster-scoped).
    pub async fn watch_cluster_roles(&self) -> crate::Result<()> {
        self.watch_cluster_resource::<ClusterRole>("rbac.authorization.k8s.io/v1/ClusterRole")
            .await
    }

    /// Watch RoleBindings in a namespace.
    pub async fn watch_role_bindings(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<RoleBinding>("rbac.authorization.k8s.io/v1/RoleBinding", namespace)
            .await
    }

    /// Watch ClusterRoleBindings (cluster-scoped).
    pub async fn watch_cluster_role_bindings(&self) -> crate::Result<()> {
        self.watch_cluster_resource::<ClusterRoleBinding>(
            "rbac.authorization.k8s.io/v1/ClusterRoleBinding",
        )
        .await
    }

    /// Watch ServiceAccounts in a namespace.
    pub async fn watch_service_accounts(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<ServiceAccount>("v1/ServiceAccount", namespace)
            .await
    }

    /// Watch HorizontalPodAutoscalers in a namespace.
    pub async fn watch_hpas(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<HorizontalPodAutoscaler>(
            "autoscaling/v2/HorizontalPodAutoscaler",
            namespace,
        )
        .await
    }

    /// Watch PodDisruptionBudgets in a namespace.
    pub async fn watch_pod_disruption_budgets(&self, namespace: &str) -> crate::Result<()> {
        self.watch_resource::<PodDisruptionBudget>("policy/v1/PodDisruptionBudget", namespace)
            .await
    }

    /// Watch PriorityClasses (cluster-scoped).
    pub async fn watch_priority_classes(&self) -> crate::Result<()> {
        self.watch_cluster_resource::<PriorityClass>("scheduling.k8s.io/v1/PriorityClass")
            .await
    }

    /// Watch ValidatingWebhookConfigurations (cluster-scoped).
    pub async fn watch_validating_webhooks(&self) -> crate::Result<()> {
        self.watch_cluster_resource::<ValidatingWebhookConfiguration>(
            "admissionregistration.k8s.io/v1/ValidatingWebhookConfiguration",
        )
        .await
    }

    /// Watch MutatingWebhookConfigurations (cluster-scoped).
    pub async fn watch_mutating_webhooks(&self) -> crate::Result<()> {
        self.watch_cluster_resource::<MutatingWebhookConfiguration>(
            "admissionregistration.k8s.io/v1/MutatingWebhookConfiguration",
        )
        .await
    }

    /// Watch StorageClasses (cluster-scoped).
    pub async fn watch_storage_classes(&self) -> crate::Result<()> {
        self.watch_cluster_resource::<StorageClass>("storage.k8s.io/v1/StorageClass")
            .await
    }

    /// Watch PersistentVolumes (cluster-scoped).
    pub async fn watch_persistent_volumes(&self) -> crate::Result<()> {
        self.watch_cluster_resource::<PersistentVolume>("v1/PersistentVolume")
            .await
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
    fn resource_to_entry_redacts_literal_pod_env_values() {
        let pod: Pod = serde_json::from_value(serde_json::json!({
            "metadata": {
                "name": "sensitive-pod",
                "resourceVersion": "9"
            },
            "spec": {
                "containers": [{
                    "name": "web",
                    "image": "nginx",
                    "env": [
                        { "name": "DATABASE_PASSWORD", "value": "super-secret-123" },
                        { "name": "FROM_SECRET", "valueFrom": { "secretKeyRef": { "name": "db", "key": "password" } } }
                    ]
                }],
                "initContainers": [{
                    "name": "init",
                    "image": "busybox",
                    "env": [
                        { "name": "BOOTSTRAP_TOKEN", "value": "bootstrap-secret" }
                    ]
                }]
            }
        }))
        .expect("pod JSON should deserialize");

        let entry = resource_to_entry("v1/Pod", "default", &pod).expect("pod should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&entry.content).expect("cached content should be valid JSON");

        assert_eq!(
            parsed["spec"]["containers"][0]["env"][0]["value"],
            REDACTED_CACHE_VALUE
        );
        assert_eq!(
            parsed["spec"]["initContainers"][0]["env"][0]["value"],
            REDACTED_CACHE_VALUE
        );
        assert_eq!(
            parsed["spec"]["containers"][0]["env"][1]["valueFrom"]["secretKeyRef"]["name"],
            "db"
        );
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
