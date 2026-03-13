use std::sync::{Arc, Mutex};

use telescope_core::{ConnectionState, ResourceStore};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

/// All watched GVK strings. Used for cache invalidation on connect,
/// disconnect, namespace switch, and startup cleanup.
pub const ALL_WATCHED_GVKS: &[&str] = &[
    "v1/Pod",
    "v1/Event",
    "v1/Node",
    "admissionregistration.k8s.io/v1/ValidatingWebhookConfiguration",
    "admissionregistration.k8s.io/v1/MutatingWebhookConfiguration",
    "apps/v1/Deployment",
    "apps/v1/StatefulSet",
    "apps/v1/DaemonSet",
    "apps/v1/ReplicaSet",
    "v1/Service",
    "v1/ConfigMap",
    "batch/v1/Job",
    "batch/v1/CronJob",
    "networking.k8s.io/v1/Ingress",
    "networking.k8s.io/v1/NetworkPolicy",
    "discovery.k8s.io/v1/EndpointSlice",
    "v1/PersistentVolumeClaim",
    "v1/ResourceQuota",
    "v1/LimitRange",
    "rbac.authorization.k8s.io/v1/Role",
    "rbac.authorization.k8s.io/v1/ClusterRole",
    "rbac.authorization.k8s.io/v1/RoleBinding",
    "rbac.authorization.k8s.io/v1/ClusterRoleBinding",
    "v1/ServiceAccount",
    "autoscaling/v2/HorizontalPodAutoscaler",
    "policy/v1/PodDisruptionBudget",
    "scheduling.k8s.io/v1/PriorityClass",
    "storage.k8s.io/v1/StorageClass",
    "v1/PersistentVolume",
];

/// Shared application state for the hub server.
///
/// `ResourceStore` is in `std::sync::Mutex` because `rusqlite::Connection`
/// is `Send` but not `Sync`. The same `Arc<Mutex<ResourceStore>>` is shared
/// with the `ResourceWatcher` background task.
pub struct HubState {
    pub store: Arc<Mutex<ResourceStore>>,
    pub connection_state: Arc<RwLock<ConnectionState>>,
    pub watch_handle: tokio::sync::Mutex<Option<JoinHandle<()>>>,
    pub active_context: RwLock<Option<String>>,
    pub active_namespace: RwLock<String>,
    pub audit_log_path: String,
}

impl HubState {
    pub fn new(db_path: &str, audit_path: &str) -> Self {
        let store = ResourceStore::open(db_path).expect("Failed to open store");

        // Clear stale data from previous runs.
        for gvk in ALL_WATCHED_GVKS {
            let _ = store.delete_all_by_gvk(gvk);
        }

        Self {
            store: Arc::new(Mutex::new(store)),
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            watch_handle: tokio::sync::Mutex::new(None),
            active_context: RwLock::new(None),
            active_namespace: RwLock::new("default".to_string()),
            audit_log_path: audit_path.to_string(),
        }
    }
}

/// Clear all watched resource data from the store.
pub fn clear_all_resources(store: &ResourceStore) {
    for gvk in ALL_WATCHED_GVKS {
        let _ = store.delete_all_by_gvk(gvk);
    }
}
