//! Watch-driven resource ingest pipeline.
//!
//! Watches Kubernetes resources via the kube API and syncs them
//! into the SQLite ResourceStore. Implements exponential backoff
//! with jitter and concurrency limits on LIST operations.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
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
const LAST_APPLIED_CONFIGURATION_ANNOTATION: &str =
    "kubectl.kubernetes.io/last-applied-configuration";
const SENSITIVE_CACHE_KEYS: &[&str] = &[
    "accesskey",
    "apikey",
    "auth",
    "cabundle",
    "clientsecret",
    "connectionstring",
    "credentials",
    "dockercfg",
    "dockerconfigjson",
    "kubeconfig",
    "password",
    "passwd",
    "privatekey",
    "secret",
    "secretkey",
    "token",
];

/// Kubernetes API structural field names that contain sensitive-sounding
/// substrings (e.g. "secret", "key", "token") but are reference metadata,
/// not user-supplied secret values. These are excluded from redaction.
const SAFE_STRUCTURAL_KEYS: &[&str] = &[
    "secretkeyref",
    "secretref",
    "configmapkeyref",
    "configmapref",
    "serviceaccounttokenprojection",
    "tokenreviewstatus",
    "valuefrom",
];

fn redacted_cache_value() -> Value {
    Value::String(REDACTED_CACHE_VALUE.to_string())
}

fn normalize_cache_key(key: &str) -> String {
    key.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn is_sensitive_cache_key(key: &str) -> bool {
    let normalized = normalize_cache_key(key);
    if SAFE_STRUCTURAL_KEYS.iter().any(|safe| *safe == normalized) {
        return false;
    }
    SENSITIVE_CACHE_KEYS
        .iter()
        .any(|needle| normalized.contains(needle))
}

fn redact_annotation_values(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if let Some(annotations) = map.get_mut("annotations").and_then(Value::as_object_mut) {
                for (key, annotation_value) in annotations.iter_mut() {
                    if key == LAST_APPLIED_CONFIGURATION_ANNOTATION
                        || annotation_value.is_string()
                        || annotation_value.is_object()
                        || annotation_value.is_array()
                    {
                        *annotation_value = redacted_cache_value();
                    }
                }
            }

            for child in map.values_mut() {
                redact_annotation_values(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_annotation_values(item);
            }
        }
        _ => {}
    }
}

fn redact_container_fields(container: &mut Value) {
    let Some(container) = container.as_object_mut() else {
        return;
    };

    for field in ["command", "args"] {
        let Some(values) = container.get_mut(field).and_then(Value::as_array_mut) else {
            continue;
        };

        for value in values {
            *value = redacted_cache_value();
        }
    }

    let Some(env_vars) = container.get_mut("env").and_then(Value::as_array_mut) else {
        return;
    };

    for env_var in env_vars {
        let Some(env) = env_var.as_object_mut() else {
            continue;
        };

        if env.contains_key("value") {
            env.insert("value".to_string(), redacted_cache_value());
        }
    }
}

fn redact_cached_pod_fields(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for container_key in ["containers", "initContainers", "ephemeralContainers"] {
                let Some(containers) = map.get_mut(container_key).and_then(Value::as_array_mut)
                else {
                    continue;
                };

                for container in containers {
                    redact_container_fields(container);
                }
            }

            for child in map.values_mut() {
                redact_cached_pod_fields(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_cached_pod_fields(item);
            }
        }
        _ => {}
    }
}

fn redact_sensitive_cache_branches(value: &mut Value, force_redact: bool) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                let should_redact = force_redact || is_sensitive_cache_key(key);
                if should_redact {
                    if child.is_string() {
                        *child = redacted_cache_value();
                    } else {
                        redact_sensitive_cache_branches(child, true);
                    }
                } else {
                    redact_sensitive_cache_branches(child, false);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_sensitive_cache_branches(item, force_redact);
            }
        }
        _ => {
            if force_redact && value.is_string() {
                *value = redacted_cache_value();
            }
        }
    }
}

fn redact_map_field_entries(value: &mut Value, field_names: &[&str]) {
    let Some(object) = value.as_object_mut() else {
        return;
    };

    for field_name in field_names {
        let Some(entries) = object.get_mut(*field_name).and_then(Value::as_object_mut) else {
            continue;
        };

        for entry in entries.values_mut() {
            *entry = redacted_cache_value();
        }
    }
}

fn redact_webhook_client_config(value: &mut Value) {
    let Some(webhooks) = value.get_mut("webhooks").and_then(Value::as_array_mut) else {
        return;
    };

    for webhook in webhooks {
        let Some(client_config) = webhook
            .get_mut("clientConfig")
            .and_then(Value::as_object_mut)
        else {
            continue;
        };

        for field in ["url", "caBundle"] {
            if client_config.contains_key(field) {
                client_config.insert(field.to_string(), redacted_cache_value());
            }
        }

        if let Some(service) = client_config
            .get_mut("service")
            .and_then(Value::as_object_mut)
        {
            if service.contains_key("path") {
                service.insert("path".to_string(), redacted_cache_value());
            }
        }
    }
}

fn sanitize_cached_resource(gvk: &str, value: &mut Value) {
    redact_annotation_values(value);
    redact_cached_pod_fields(value);
    redact_sensitive_cache_branches(value, false);

    match gvk {
        "v1/Secret" => redact_map_field_entries(value, &["data", "stringData", "binaryData"]),
        "v1/ConfigMap" => redact_map_field_entries(value, &["data", "binaryData"]),
        "admissionregistration.k8s.io/v1/MutatingWebhookConfiguration"
        | "admissionregistration.k8s.io/v1/ValidatingWebhookConfiguration" => {
            redact_webhook_client_config(value);
        }
        _ => {}
    }
}

fn serialize_resource_for_cache<K>(gvk: &str, obj: &K) -> Option<String>
where
    K: Serialize,
{
    let mut value = serde_json::to_value(obj)
        .map_err(|e| {
            warn!(error = %e, gvk, "failed to serialize resource to value");
            e
        })
        .ok()?;
    sanitize_cached_resource(gvk, &mut value);
    serde_json::to_string(&value)
        .map_err(|e| {
            warn!(error = %e, gvk, "failed to serialize resource to string");
            e
        })
        .ok()
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
        updated_at: telescope_core::now_rfc3339(),
    })
}

/// Tracks aggregate sync progress across multiple concurrent watch tasks.
///
/// When [`ResourceWatcher::register_watches`] is called with the total
/// expected watch count, individual watch loops report via
/// [`ResourceWatcher::mark_watch_synced`] and [`mark_watch_failed`].
/// `SyncComplete` is emitted only after **all** watches have reported.
struct SyncTracker {
    expected: AtomicU32,
    synced: AtomicU32,
    failed: AtomicU32,
    /// Ensures the completion event is emitted exactly once.
    emitted: AtomicBool,
}

impl SyncTracker {
    fn new() -> Self {
        Self {
            expected: AtomicU32::new(0),
            synced: AtomicU32::new(0),
            failed: AtomicU32::new(0),
            emitted: AtomicBool::new(false),
        }
    }

    fn reset(&self, expected: u32) {
        self.expected.store(expected, Ordering::SeqCst);
        self.synced.store(0, Ordering::SeqCst);
        self.failed.store(0, Ordering::SeqCst);
        self.emitted.store(false, Ordering::SeqCst);
    }
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
    /// Aggregate sync tracker for multi-watch readiness.
    sync_tracker: Arc<SyncTracker>,
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
            sync_tracker: Arc::new(SyncTracker::new()),
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

    /// Register the total number of watch tasks that will be spawned.
    ///
    /// Must be called **before** any watch task starts. Drives the
    /// connection state to `Syncing`. When all registered watches
    /// report (via `InitDone` or error), the aggregate state transitions
    /// to `Ready` (all synced) or `Degraded` (some failed).
    pub fn register_watches(&self, count: u32) {
        self.sync_tracker.reset(count);
        self.transition(&ConnectionEvent::Connect);
        self.transition(&ConnectionEvent::Authenticated);
        if count > 0 {
            self.transition(&ConnectionEvent::SyncProgress {
                synced: 0,
                total: Some(count),
            });
        } else {
            self.transition(&ConnectionEvent::SyncComplete);
        }
    }

    /// Record that one watch completed its initial LIST successfully.
    fn mark_watch_synced(&self, gvk: &str) {
        let new_synced = self.sync_tracker.synced.fetch_add(1, Ordering::SeqCst) + 1;
        let expected = self.sync_tracker.expected.load(Ordering::SeqCst);

        info!(
            gvk,
            synced = new_synced,
            expected,
            "Watch completed initial sync"
        );

        if expected > 0 {
            self.transition(&ConnectionEvent::SyncProgress {
                synced: new_synced,
                total: Some(expected),
            });
            self.check_sync_complete();
        } else {
            // Legacy mode: no registration, immediate transition.
            self.transition(&ConnectionEvent::SyncComplete);
        }
    }

    /// Record that one watch failed before completing its initial LIST.
    fn mark_watch_failed(&self, gvk: &str, error: &str) {
        let new_failed = self.sync_tracker.failed.fetch_add(1, Ordering::SeqCst) + 1;
        let expected = self.sync_tracker.expected.load(Ordering::SeqCst);

        warn!(
            gvk,
            failed = new_failed,
            expected,
            error,
            "Watch failed during initial sync"
        );

        if expected > 0 {
            self.check_sync_complete();
        } else {
            self.transition(&ConnectionEvent::WatchError {
                message: error.to_string(),
            });
        }
    }

    /// If all registered watches have reported, emit the final state event.
    fn check_sync_complete(&self) {
        let synced = self.sync_tracker.synced.load(Ordering::SeqCst);
        let failed = self.sync_tracker.failed.load(Ordering::SeqCst);
        let expected = self.sync_tracker.expected.load(Ordering::SeqCst);

        if expected == 0 || synced + failed < expected {
            return;
        }

        // Ensure exactly one caller emits the completion event.
        if self
            .sync_tracker
            .emitted
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }

        if failed > 0 {
            self.transition(&ConnectionEvent::WatchError {
                message: format!(
                    "{} of {} watches failed during initial sync",
                    failed, expected
                ),
            });
        } else {
            self.transition(&ConnectionEvent::SyncComplete);
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
                            self.mark_watch_synced(&gvk_str);
                        }
                    }
                },
                Ok(None) => {
                    warn!("Watch stream ended for {}/{}", gvk_str, ns);
                    if !synced {
                        self.mark_watch_failed(&gvk_str, "watch stream ended before initial sync");
                    } else {
                        self.transition(&ConnectionEvent::Disconnected);
                    }
                    break;
                }
                Err(e) => {
                    error!("Watch error for {}/{}: {}", gvk_str, ns, e);
                    if !synced {
                        self.mark_watch_failed(&gvk_str, &e.to_string());
                    } else {
                        self.transition(&ConnectionEvent::WatchError {
                            message: e.to_string(),
                        });
                    }
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
                            self.mark_watch_synced(&gvk_str);
                        }
                    }
                },
                Ok(None) => {
                    warn!("Watch stream ended for {} (all namespaces)", gvk_str);
                    if !synced {
                        self.mark_watch_failed(&gvk_str, "watch stream ended before initial sync");
                    } else {
                        self.transition(&ConnectionEvent::Disconnected);
                    }
                    break;
                }
                Err(e) => {
                    error!("Watch error for {} (all namespaces): {}", gvk_str, e);
                    if !synced {
                        self.mark_watch_failed(&gvk_str, &e.to_string());
                    } else {
                        self.transition(&ConnectionEvent::WatchError {
                            message: e.to_string(),
                        });
                    }
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
    fn resource_to_entry_redacts_pod_commands_args_and_annotations() {
        let deploy: Deployment = serde_json::from_value(serde_json::json!({
            "metadata": {
                "name": "annotated-deploy",
                "resourceVersion": "10",
                "labels": {
                    "app": "annotated-deploy"
                },
                "annotations": {
                    "example.com/runbook": "https://internal.example.com/runbook"
                }
            },
            "spec": {
                "selector": {
                    "matchLabels": {
                        "app": "annotated-deploy"
                    }
                },
                "template": {
                    "metadata": {
                        "labels": {
                            "app": "annotated-deploy"
                        },
                        "annotations": {
                            "kubectl.kubernetes.io/restartedAt": "2026-04-04T10:00:00Z"
                        }
                    },
                    "spec": {
                        "containers": [{
                            "name": "web",
                            "image": "nginx",
                            "command": ["sh", "-c"],
                            "args": ["echo token=$API_TOKEN"],
                            "env": [{ "name": "API_TOKEN", "value": "sensitive-token" }]
                        }]
                    }
                }
            }
        }))
        .expect("deployment JSON should deserialize");

        let entry = resource_to_entry("apps/v1/Deployment", "default", &deploy)
            .expect("deployment should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&entry.content).expect("cached content should be valid JSON");

        assert_eq!(
            parsed["metadata"]["annotations"]["example.com/runbook"],
            REDACTED_CACHE_VALUE
        );
        assert_eq!(
            parsed["spec"]["template"]["metadata"]["annotations"]
                ["kubectl.kubernetes.io/restartedAt"],
            REDACTED_CACHE_VALUE
        );
        assert_eq!(
            parsed["spec"]["template"]["spec"]["containers"][0]["command"][0],
            REDACTED_CACHE_VALUE
        );
        assert_eq!(
            parsed["spec"]["template"]["spec"]["containers"][0]["args"][0],
            REDACTED_CACHE_VALUE
        );
        assert_eq!(
            parsed["spec"]["template"]["spec"]["containers"][0]["env"][0]["value"],
            REDACTED_CACHE_VALUE
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
        let cm: ConfigMap = serde_json::from_value(serde_json::json!({
            "metadata": {
                "name": "app-config",
                "annotations": {
                    "example.com/owner": "platform-team"
                }
            },
            "data": {
                "DATABASE_URL": "postgres://db.internal.example.com/app",
                "FEATURE_FLAG": "enabled"
            },
            "binaryData": {
                "ca.crt": "ZmFrZS1jZXJ0"
            }
        }))
        .expect("configmap JSON should deserialize");
        let entry = resource_to_entry("v1/ConfigMap", "kube-system", &cm).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&entry.content).unwrap();
        assert_eq!(entry.name, "app-config");
        assert_eq!(entry.gvk, "v1/ConfigMap");
        assert_eq!(parsed["data"]["DATABASE_URL"], REDACTED_CACHE_VALUE);
        assert_eq!(parsed["data"]["FEATURE_FLAG"], REDACTED_CACHE_VALUE);
        assert_eq!(parsed["binaryData"]["ca.crt"], REDACTED_CACHE_VALUE);
        assert_eq!(
            parsed["metadata"]["annotations"]["example.com/owner"],
            REDACTED_CACHE_VALUE
        );
    }

    #[test]
    fn resource_to_entry_works_for_secret() {
        let secret: Secret = serde_json::from_value(serde_json::json!({
            "metadata": {
                "name": "db-creds",
                "resourceVersion": "7",
                "annotations": {
                    "kubectl.kubernetes.io/last-applied-configuration": "{\"data\":{\"password\":\"raw\"}}"
                }
            },
            "data": {
                "password": "c3VwZXItc2VjcmV0"
            },
            "stringData": {
                "token": "plain-text-token"
            }
        }))
        .expect("secret JSON should deserialize");
        let entry = resource_to_entry("v1/Secret", "default", &secret).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&entry.content).unwrap();
        assert_eq!(entry.name, "db-creds");
        assert_eq!(entry.gvk, "v1/Secret");
        assert_eq!(entry.resource_version, "7");
        assert_eq!(parsed["data"]["password"], REDACTED_CACHE_VALUE);
        assert_eq!(parsed["stringData"]["token"], REDACTED_CACHE_VALUE);
        assert_eq!(
            parsed["metadata"]["annotations"]["kubectl.kubernetes.io/last-applied-configuration"],
            REDACTED_CACHE_VALUE
        );
    }

    #[test]
    fn resource_to_entry_redacts_webhook_client_config_payloads() {
        let webhook: ValidatingWebhookConfiguration = serde_json::from_value(serde_json::json!({
            "metadata": {
                "name": "validate-demo"
            },
            "webhooks": [{
                "name": "validate.demo.example.com",
                "admissionReviewVersions": ["v1"],
                "sideEffects": "None",
                "clientConfig": {
                    "url": "https://internal.example.com/validate",
                    "caBundle": "ZmFrZS1jYS1idW5kbGU=",
                    "service": {
                        "namespace": "default",
                        "name": "webhook-service",
                        "path": "/validate"
                    }
                }
            }]
        }))
        .expect("webhook JSON should deserialize");

        let entry = resource_to_entry(
            "admissionregistration.k8s.io/v1/ValidatingWebhookConfiguration",
            "",
            &webhook,
        )
        .expect("webhook should serialize");
        let parsed: serde_json::Value = serde_json::from_str(&entry.content).unwrap();

        assert_eq!(
            parsed["webhooks"][0]["clientConfig"]["url"],
            REDACTED_CACHE_VALUE
        );
        assert_eq!(
            parsed["webhooks"][0]["clientConfig"]["caBundle"],
            REDACTED_CACHE_VALUE
        );
        assert_eq!(
            parsed["webhooks"][0]["clientConfig"]["service"]["path"],
            REDACTED_CACHE_VALUE
        );
        assert_eq!(
            parsed["webhooks"][0]["clientConfig"]["service"]["name"],
            "webhook-service"
        );
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

    /// Helper: create a ResourceWatcher backed by an in-memory store.
    fn make_test_watcher() -> ResourceWatcher {
        let store = ResourceStore::open(":memory:").expect("in-memory store");
        let (state_tx, state_rx) = watch::channel(ConnectionState::Disconnected);
        ResourceWatcher {
            client: {
                // Build a dummy client — never used in these tests.
                let config = kube::Config {
                    cluster_url: "https://localhost:6443".parse().unwrap(),
                    default_namespace: "default".into(),
                    root_cert: None,
                    connect_timeout: None,
                    read_timeout: None,
                    write_timeout: None,
                    accept_invalid_certs: true,
                    auth_info: Default::default(),
                    proxy_url: None,
                    tls_server_name: None,
                    headers: vec![],
                    disable_compression: false,
                };
                Client::try_from(config).unwrap()
            },
            store: Arc::new(Mutex::new(store)),
            list_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_LISTS)),
            sync_tracker: Arc::new(SyncTracker::new()),
            state_tx: Arc::new(state_tx),
            state_rx,
        }
    }

    #[tokio::test]
    async fn register_watches_transitions_to_syncing() {
        let w = make_test_watcher();
        w.register_watches(3);
        let state = w.state_rx.borrow().clone();
        assert_eq!(
            state,
            ConnectionState::Syncing {
                resources_synced: 0,
                resources_total: Some(3)
            }
        );
    }

    #[tokio::test]
    async fn register_watches_zero_transitions_to_ready() {
        let w = make_test_watcher();
        w.register_watches(0);
        let state = w.state_rx.borrow().clone();
        assert_eq!(state, ConnectionState::Ready);
    }

    #[tokio::test]
    async fn mark_synced_emits_progress_not_ready_until_all() {
        let w = make_test_watcher();
        w.register_watches(3);

        w.mark_watch_synced("v1/Pod");
        let state = w.state_rx.borrow().clone();
        assert_eq!(
            state,
            ConnectionState::Syncing {
                resources_synced: 1,
                resources_total: Some(3)
            }
        );

        w.mark_watch_synced("apps/v1/Deployment");
        let state = w.state_rx.borrow().clone();
        assert_eq!(
            state,
            ConnectionState::Syncing {
                resources_synced: 2,
                resources_total: Some(3)
            }
        );

        w.mark_watch_synced("v1/Service");
        let state = w.state_rx.borrow().clone();
        assert_eq!(state, ConnectionState::Ready);
    }

    #[tokio::test]
    async fn mixed_sync_and_failure_emits_degraded() {
        let w = make_test_watcher();
        w.register_watches(3);

        w.mark_watch_synced("v1/Pod");
        w.mark_watch_synced("apps/v1/Deployment");
        w.mark_watch_failed("v1/Service", "403 forbidden");

        let state = w.state_rx.borrow().clone();
        assert!(matches!(state, ConnectionState::Degraded { .. }));
    }

    #[tokio::test]
    async fn all_watches_fail_emits_degraded() {
        let w = make_test_watcher();
        w.register_watches(2);

        w.mark_watch_failed("v1/Pod", "timeout");
        w.mark_watch_failed("v1/Service", "unauthorized");

        let state = w.state_rx.borrow().clone();
        assert!(matches!(state, ConnectionState::Degraded { .. }));
    }

    #[tokio::test]
    async fn legacy_mode_no_registration_immediate_ready() {
        let w = make_test_watcher();
        // Drive state to Syncing manually (legacy: first watch does this).
        w.transition(&ConnectionEvent::Connect);
        w.transition(&ConnectionEvent::Authenticated);

        // Without register_watches, mark_watch_synced emits SyncComplete directly.
        w.mark_watch_synced("v1/Pod");
        let state = w.state_rx.borrow().clone();
        assert_eq!(state, ConnectionState::Ready);
    }

    #[test]
    fn sync_tracker_reset_clears_state() {
        let tracker = SyncTracker::new();
        tracker.synced.fetch_add(5, Ordering::SeqCst);
        tracker.failed.fetch_add(2, Ordering::SeqCst);
        tracker.emitted.store(true, Ordering::SeqCst);

        tracker.reset(10);
        assert_eq!(tracker.expected.load(Ordering::SeqCst), 10);
        assert_eq!(tracker.synced.load(Ordering::SeqCst), 0);
        assert_eq!(tracker.failed.load(Ordering::SeqCst), 0);
        assert!(!tracker.emitted.load(Ordering::SeqCst));
    }
}
