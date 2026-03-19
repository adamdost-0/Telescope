#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use futures::{AsyncBufReadExt, TryStreamExt};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};
use telescope_azure::{
    AksNodePool, ArmClient, AzureCloud, CreateNodePoolRequest, MaintenanceConfig,
    PoolUpgradeProfile, UpgradeProfile,
};
use tokio::sync::{Mutex as TokioMutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

use telescope_core::{ConnectionState, ResourceEntry, ResourceStore};
use telescope_engine::{audit::AuditEntry, validation, ClusterContext};

/// All watched GVK strings. Used for cache invalidation on connect,
/// disconnect, namespace switch, and startup cleanup.
const ALL_WATCHED_GVKS: &[&str] = &[
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

/// Clear all watched resource data from the store.
fn clear_all_resources(store: &ResourceStore) {
    for gvk in ALL_WATCHED_GVKS {
        let _ = store.delete_all_by_gvk(gvk);
    }
}

/// Application state managed by Tauri.
///
/// `ResourceStore` stays in `std::sync::Mutex` because `rusqlite::Connection`
/// is `Send` but not `Sync`. The same `Arc<Mutex<ResourceStore>>` is shared
/// with the `ResourceWatcher` background task.
struct AppState {
    #[allow(dead_code)]
    db_path: String,
    audit_log_path: String,
    audit_actor: String,
    active_connection: RwLock<Option<ActiveConnection>>,
    store: Arc<Mutex<ResourceStore>>,
    connection_state: Arc<RwLock<ConnectionState>>,
    watch_handle: TokioMutex<Option<JoinHandle<()>>>,
    active_context: RwLock<Option<String>>,
    active_namespace: RwLock<String>,
}

#[derive(Clone)]
struct ActiveConnection {
    context_name: String,
    client: kube::Client,
}

async fn write_audit_entry(
    state: &AppState,
    context: Option<String>,
    namespace: impl Into<String>,
    action: impl Into<String>,
    resource_type: impl Into<String>,
    resource_name: impl Into<String>,
    result: impl Into<String>,
    detail: Option<String>,
) -> Result<(), String> {
    let context = match context {
        Some(context) => context,
        None => state
            .active_context
            .read()
            .await
            .clone()
            .unwrap_or_default(),
    };

    let entry = AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        actor: state.audit_actor.clone(),
        context,
        namespace: namespace.into(),
        action: action.into(),
        resource_type: resource_type.into(),
        resource_name: resource_name.into(),
        result: result.into(),
        detail,
    };

    telescope_engine::audit::log_audit(&state.audit_log_path, &entry)
        .map_err(|error| format!("Failed to write audit log: {error}"))
}

fn finish_audited_command<T, E>(
    outcome: std::result::Result<T, E>,
    audit_result: Result<(), String>,
) -> Result<T, String>
where
    E: std::fmt::Display,
{
    match (outcome, audit_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(audit_error)) => Err(format!(
            "Operation completed but failed to write audit log: {audit_error}"
        )),
        (Err(error), Ok(())) => Err(error.to_string()),
        (Err(error), Err(audit_error)) => {
            Err(format!("{error}; audit log write failed: {audit_error}"))
        }
    }
}

// ---------------------------------------------------------------------------
// Sync commands (read-only)
// ---------------------------------------------------------------------------

/// Get cluster version and auth info for the connected context.
#[tauri::command]
async fn get_cluster_info(
    state: State<'_, AppState>,
) -> Result<telescope_engine::ClusterInfo, String> {
    let connection = active_connection(&state).await?;
    let mut info =
        telescope_engine::client::get_cluster_info(&connection.client, &connection.context_name)
            .await
            .map_err(|e| e.to_string())?;

    // Attempt AKS identity resolution when connected to an AKS cluster.
    if info.is_aks {
        let preferred_id = {
            let store_guard = state.store.lock().map_err(|e| e.to_string())?;
            telescope_azure::resolve_aks_identity_from_preferences(Some(&store_guard))
        };

        if let Some(id) =
            telescope_azure::resolve_aks_identity(&info.server_url, preferred_id).await
        {
            info.azure_resource_id = Some(id.arm_path());
            info.subscription_id = Some(id.subscription_id);
            info.resource_group = Some(id.resource_group);
        }
    }

    Ok(info)
}

/// Resolved AKS identity information returned to the frontend.
#[derive(serde::Serialize)]
struct AksIdentityInfo {
    subscription_id: String,
    resource_group: String,
    cluster_name: String,
    arm_resource_id: String,
}

/// Resolve AKS identity (subscription, RG, cluster name) for the active context.
#[tauri::command]
async fn resolve_aks_identity(
    state: State<'_, AppState>,
) -> Result<Option<AksIdentityInfo>, String> {
    match resolve_active_aks_resource_id(&state).await? {
        Some(id) => Ok(Some(AksIdentityInfo {
            arm_resource_id: id.arm_path(),
            subscription_id: id.subscription_id,
            resource_group: id.resource_group,
            cluster_name: id.cluster_name,
        })),
        None => Ok(None),
    }
}

/// List AKS node pools from the Azure ARM API for the active cluster.
#[tauri::command]
async fn list_aks_node_pools(state: State<'_, AppState>) -> Result<Vec<AksNodePool>, String> {
    let (client, resource_id) = active_aks_arm_client(&state).await?;
    telescope_azure::list_node_pools(&client, &resource_id)
        .await
        .map_err(|e| format!("Failed to list AKS node pools via Azure ARM: {e}"))
}

#[tauri::command]
async fn start_aks_cluster(state: State<'_, AppState>) -> Result<(), String> {
    let (client, resource_id) = active_aks_arm_client(&state).await?;
    telescope_azure::start_cluster(&client, &resource_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_aks_cluster(state: State<'_, AppState>) -> Result<(), String> {
    let (client, resource_id) = active_aks_arm_client(&state).await?;
    telescope_azure::stop_cluster(&client, &resource_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_aks_upgrade_profile(state: State<'_, AppState>) -> Result<UpgradeProfile, String> {
    let (client, resource_id) = active_aks_arm_client(&state).await?;
    telescope_azure::get_upgrade_profile(&client, &resource_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn upgrade_aks_cluster(
    state: State<'_, AppState>,
    target_version: String,
) -> Result<(), String> {
    let target_version = validation::validate_kubernetes_version(&target_version, "targetVersion")
        .map_err(|error| error.to_string())?;
    let (client, resource_id) = active_aks_arm_client(&state).await?;
    telescope_azure::upgrade_cluster(&client, &resource_id, &target_version)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_pool_upgrade_profile(
    state: State<'_, AppState>,
    pool: String,
) -> Result<PoolUpgradeProfile, String> {
    let pool = validation::validate_aks_node_pool_name(&pool).map_err(|error| error.to_string())?;
    let (client, resource_id) = active_aks_arm_client(&state).await?;
    telescope_azure::get_pool_upgrade_profile(&client, &resource_id, &pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn upgrade_pool_version(
    state: State<'_, AppState>,
    pool: String,
    version: String,
) -> Result<(), String> {
    let pool = validation::validate_aks_node_pool_name(&pool).map_err(|error| error.to_string())?;
    let version = validation::validate_kubernetes_version(&version, "version")
        .map_err(|error| error.to_string())?;
    let (client, resource_id) = active_aks_arm_client(&state).await?;
    telescope_azure::upgrade_pool_version(&client, &resource_id, &pool, &version)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn upgrade_pool_node_image(state: State<'_, AppState>, pool: String) -> Result<(), String> {
    let pool = validation::validate_aks_node_pool_name(&pool).map_err(|error| error.to_string())?;
    let (client, resource_id) = active_aks_arm_client(&state).await?;
    telescope_azure::upgrade_pool_node_image(&client, &resource_id, &pool)
        .await
        .map_err(|e| e.to_string())
}

/// Frontend-facing config struct for creating a new AKS node pool.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateNodePoolConfig {
    pub name: String,
    pub vm_size: String,
    pub count: i32,
    pub os_type: Option<String>,
    pub mode: Option<String>,
    pub orchestrator_version: Option<String>,
    pub enable_auto_scaling: Option<bool>,
    pub min_count: Option<i32>,
    pub max_count: Option<i32>,
    pub availability_zones: Option<Vec<String>>,
    pub max_pods: Option<i32>,
    pub node_labels: Option<std::collections::HashMap<String, String>>,
    pub node_taints: Option<Vec<String>>,
}

/// Scale an AKS node pool to a target count via the Azure ARM API.
#[tauri::command]
async fn scale_aks_node_pool(
    state: State<'_, AppState>,
    pool_name: String,
    count: i32,
) -> Result<AksNodePool, String> {
    let pool_name =
        validation::validate_aks_node_pool_name(&pool_name).map_err(|error| error.to_string())?;
    let count = validate_i32_param(count, "count", 0, validation::MAX_NODE_POOL_COUNT)?;
    let (client, resource_id) =
        active_aks_arm_client_for(&state, "Scaling node pools requires an AKS cluster").await?;
    let outcome = telescope_azure::scale_node_pool(&client, &resource_id, &pool_name, count).await;

    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        String::new(),
        "scale_aks_node_pool",
        "AksNodePool",
        pool_name.clone(),
        result_str,
        Some(format!("count={count}")),
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

/// Update autoscaler settings on an AKS node pool.
#[tauri::command]
async fn update_aks_autoscaler(
    state: State<'_, AppState>,
    pool_name: String,
    enabled: bool,
    min: Option<i32>,
    max: Option<i32>,
) -> Result<AksNodePool, String> {
    let pool_name =
        validation::validate_aks_node_pool_name(&pool_name).map_err(|error| error.to_string())?;
    let (min, max) = validation::validate_autoscaler_bounds(enabled, min, max)
        .map_err(|error| error.to_string())?;
    let (client, resource_id) =
        active_aks_arm_client_for(&state, "Updating autoscaler requires an AKS cluster").await?;
    let outcome =
        telescope_azure::update_autoscaler(&client, &resource_id, &pool_name, enabled, min, max)
            .await;

    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        String::new(),
        "update_aks_autoscaler",
        "AksNodePool",
        pool_name.clone(),
        result_str,
        Some(format!("enabled={enabled} min={min:?} max={max:?}")),
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

/// Create a new AKS node pool via the Azure ARM API.
#[tauri::command]
async fn create_aks_node_pool(
    state: State<'_, AppState>,
    config: CreateNodePoolConfig,
) -> Result<AksNodePool, String> {
    let request = validate_create_node_pool_config(config)?;
    let (client, resource_id) =
        active_aks_arm_client_for(&state, "Creating node pools requires an AKS cluster").await?;
    let outcome = telescope_azure::create_node_pool(&client, &resource_id, &request).await;

    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        String::new(),
        "create_aks_node_pool",
        "AksNodePool",
        request.name.clone(),
        result_str,
        Some(format!(
            "vmSize={} count={}",
            request.vm_size, request.count
        )),
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

/// Delete an AKS node pool via the Azure ARM API.
#[tauri::command]
async fn delete_aks_node_pool(state: State<'_, AppState>, pool_name: String) -> Result<(), String> {
    let pool_name =
        validation::validate_aks_node_pool_name(&pool_name).map_err(|error| error.to_string())?;
    let (client, resource_id) =
        active_aks_arm_client_for(&state, "Deleting node pools requires an AKS cluster").await?;
    let outcome = telescope_azure::delete_node_pool(&client, &resource_id, &pool_name).await;

    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        String::new(),
        "delete_aks_node_pool",
        "AksNodePool",
        pool_name.clone(),
        result_str,
        None,
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

/// List available Kubernetes contexts from kubeconfig.
#[tauri::command]
fn list_contexts() -> Result<Vec<ClusterContext>, String> {
    debug!("list_contexts called");
    let result = telescope_engine::kubeconfig::list_contexts().map_err(|e| e.to_string());
    debug!(context_count = ?result.as_ref().map(|v| v.len()), "list_contexts completed");
    result
}

/// Get the currently active kubeconfig context.
#[tauri::command]
async fn active_context(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let ctx = state.active_context.read().await.clone();
    if ctx.is_some() {
        return Ok(ctx);
    }
    // Fall back to kubeconfig active context
    telescope_engine::kubeconfig::active_context()
        .map(Some)
        .map_err(|e| e.to_string())
}

/// Get the current connection state.
#[tauri::command]
async fn get_connection_state(state: State<'_, AppState>) -> Result<ConnectionState, String> {
    Ok(state.connection_state.read().await.clone())
}

/// List pods in a namespace from the SQLite store.
#[tauri::command]
fn get_pods(
    state: State<'_, AppState>,
    namespace: Option<String>,
) -> Result<Vec<ResourceEntry>, String> {
    let store = state
        .store
        .lock()
        .map_err(|e| format!("Store lock failed: {}", e))?;
    store
        .list("v1/Pod", namespace.as_deref())
        .map_err(|e| e.to_string())
}

/// Get resources by GVK and optional namespace.
#[tauri::command]
fn get_resources(
    state: State<'_, AppState>,
    gvk: String,
    namespace: Option<String>,
) -> Result<Vec<ResourceEntry>, String> {
    let store = state
        .store
        .lock()
        .map_err(|e| format!("Store lock failed: {}", e))?;
    store
        .list(&gvk, namespace.as_deref())
        .map_err(|e| e.to_string())
}

/// List events, optionally filtered by involved object name.
#[tauri::command]
fn get_events(
    state: State<'_, AppState>,
    namespace: Option<String>,
    involved_object: Option<String>,
) -> Result<Vec<ResourceEntry>, String> {
    let namespace = namespace
        .as_deref()
        .map(validate_namespace_param)
        .transpose()?;
    let involved_object = involved_object
        .as_deref()
        .map(|value| validate_identifier_param(value, "involvedObject"))
        .transpose()?;
    let store = state
        .store
        .lock()
        .map_err(|e| format!("Store lock failed: {}", e))?;
    let events = store
        .list("v1/Event", namespace.as_deref())
        .map_err(|e| e.to_string())?;

    if let Some(obj_name) = involved_object {
        Ok(events
            .into_iter()
            .filter(|entry| validation::event_matches_involved_object_name(entry, &obj_name))
            .collect())
    } else {
        Ok(events)
    }
}

/// Return counts for all major resource types.
#[tauri::command]
fn get_resource_counts(state: State<'_, AppState>) -> Result<Vec<(String, u64)>, String> {
    let store = state
        .store
        .lock()
        .map_err(|e| format!("Lock failed: {}", e))?;
    // Security: v1/Secret is excluded — secrets are fetched on-demand, never cached.
    let counts: Vec<(String, u64)> = ALL_WATCHED_GVKS
        .iter()
        .map(|gvk| {
            let count = store.count(gvk, None).unwrap_or(0);
            (gvk.to_string(), count)
        })
        .collect();
    Ok(counts)
}

/// Search across all cached resource types by name or GVK substring match.
#[tauri::command]
fn search_resources(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<ResourceEntry>, String> {
    let store = state
        .store
        .lock()
        .map_err(|e| format!("Store lock failed: {}", e))?;
    let mut results = Vec::new();
    let query_lower = query.to_lowercase();
    for gvk in ALL_WATCHED_GVKS {
        if let Ok(entries) = store.list(gvk, None) {
            for entry in entries {
                if entry.name.to_lowercase().contains(&query_lower)
                    || entry.gvk.to_lowercase().contains(&query_lower)
                {
                    results.push(entry);
                    if results.len() >= 20 {
                        return Ok(results);
                    }
                }
            }
        }
    }
    Ok(results)
}

/// Count resources by GVK and optional namespace.
#[tauri::command]
fn count_resources(
    state: State<'_, AppState>,
    gvk: String,
    namespace: Option<String>,
) -> Result<u64, String> {
    let store = state
        .store
        .lock()
        .map_err(|e| format!("Store lock failed: {}", e))?;
    store
        .count(&gvk, namespace.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_dynamic_resources(
    state: State<'_, AppState>,
    group: String,
    version: String,
    plural: String,
    namespace: Option<String>,
) -> Result<Vec<ResourceEntry>, String> {
    let client = active_client(&state).await?;
    let kind = telescope_engine::dynamic::resolve_dynamic_kind(&client, &group, &version, &plural)
        .await
        .map_err(|e| e.to_string())?;
    telescope_engine::dynamic::list_dynamic_resources(
        &client,
        &group,
        &version,
        &kind,
        &plural,
        namespace.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_dynamic_resource(
    state: State<'_, AppState>,
    group: String,
    version: String,
    plural: String,
    namespace: Option<String>,
    name: String,
) -> Result<Option<ResourceEntry>, String> {
    let client = active_client(&state).await?;
    let kind = telescope_engine::dynamic::resolve_dynamic_kind(&client, &group, &version, &plural)
        .await
        .map_err(|e| e.to_string())?;
    telescope_engine::dynamic::get_dynamic_resource(
        &client,
        &group,
        &version,
        &kind,
        &plural,
        namespace.as_deref(),
        &name,
    )
    .await
    .map_err(|e| e.to_string())
}

/// Get a specific resource by GVK, namespace, and name.
#[tauri::command]
fn get_resource(
    state: State<'_, AppState>,
    gvk: String,
    namespace: String,
    name: String,
) -> Result<Option<ResourceEntry>, String> {
    let store = state
        .store
        .lock()
        .map_err(|e| format!("Store lock failed: {}", e))?;
    store
        .get(&gvk, &namespace, &name)
        .map_err(|e| e.to_string())
}

/// List secrets in a namespace via a direct uncached Kubernetes API read.
#[tauri::command]
async fn get_secrets(
    state: State<'_, AppState>,
    namespace: String,
) -> Result<Vec<ResourceEntry>, String> {
    let client = active_client(&state).await?;
    telescope_engine::secrets::list_secrets(&client, &namespace)
        .await
        .map_err(|e| e.to_string())
}

/// Get one secret via a direct uncached Kubernetes API read.
#[tauri::command]
async fn get_secret(
    state: State<'_, AppState>,
    namespace: String,
    name: String,
) -> Result<Option<ResourceEntry>, String> {
    let client = active_client(&state).await?;
    telescope_engine::secrets::get_secret(&client, &namespace, &name)
        .await
        .map_err(|e| e.to_string())
}

/// List Helm releases by parsing Helm release Secrets from Kubernetes.
#[tauri::command]
async fn list_helm_releases(
    state: State<'_, AppState>,
    namespace: Option<String>,
) -> Result<Vec<telescope_engine::helm::HelmRelease>, String> {
    let client = active_client(&state).await?;
    telescope_engine::helm::list_releases(&client, namespace.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// Get all revisions of a specific Helm release, sorted by revision number.
#[tauri::command]
async fn get_helm_release_history(
    state: State<'_, AppState>,
    namespace: String,
    name: String,
) -> Result<Vec<telescope_engine::helm::HelmRelease>, String> {
    let client = active_client(&state).await?;
    telescope_engine::helm::get_release_history(&client, &namespace, &name)
        .await
        .map_err(|e| e.to_string())
}

/// Get the user-supplied values for the latest revision of a Helm release.
///
/// By default, sensitive keys (passwords, tokens, secrets, etc.) are redacted.
/// Pass `reveal: true` to return unredacted values.
#[tauri::command]
async fn get_helm_release_values(
    state: State<'_, AppState>,
    namespace: String,
    name: String,
    reveal: Option<bool>,
) -> Result<String, String> {
    let client = active_client(&state).await?;
    let mut values = telescope_engine::helm::get_release_values(&client, &namespace, &name)
        .await
        .map_err(|e| e.to_string())?;
    if !reveal.unwrap_or(false) && !values.trim_start().starts_with('#') {
        let mut json = serde_yaml::from_str::<serde_json::Value>(&values)
            .map_err(|e| format!("Failed to parse Helm values YAML: {e}"))?;
        telescope_engine::helm::redact_sensitive_values(&mut json);
        values = serde_yaml::to_string(&json)
            .map_err(|e| format!("Failed to serialize redacted Helm values YAML: {e}"))?;
    }
    Ok(values)
}

/// Roll back a Helm release to a specific revision using the helm CLI.
#[tauri::command]
async fn helm_rollback(
    state: State<'_, AppState>,
    namespace: String,
    name: String,
    revision: i32,
) -> Result<String, String> {
    let namespace = validate_namespace_param(&namespace)?;
    let name = validate_k8s_name_param(&name, "name")?;
    let revision = validate_i32_param(revision, "revision", 1, i32::MAX)?;
    let outcome = telescope_engine::helm::rollback_release(&namespace, &name, revision).await;
    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        namespace.clone(),
        "helm_rollback",
        "HelmRelease",
        name.clone(),
        result_str,
        Some(format!("revision={revision}")),
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

/// List available namespaces from the connected cluster.
#[tauri::command]
async fn list_namespaces(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let client = active_client(&state).await?;
    telescope_engine::namespace::list_namespaces(&client)
        .await
        .map_err(|e| e.to_string())
}

/// Create a namespace in the connected cluster.
#[tauri::command]
async fn create_namespace(state: State<'_, AppState>, name: String) -> Result<String, String> {
    let name = validate_namespace_param(&name)?;
    let client = active_client(&state).await?;
    let outcome = telescope_engine::namespace::create_namespace(&client, &name).await;
    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        name.clone(),
        "create_namespace",
        "v1/Namespace",
        name,
        result_str,
        None,
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

/// Delete a namespace in the connected cluster.
#[tauri::command]
async fn delete_namespace(state: State<'_, AppState>, name: String) -> Result<String, String> {
    let name = validate_namespace_param(&name)?;
    let client = active_client(&state).await?;
    let outcome = telescope_engine::namespace::delete_namespace(&client, &name).await;
    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        name.clone(),
        "delete_namespace",
        "v1/Namespace",
        name,
        result_str,
        None,
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

// ---------------------------------------------------------------------------
// Async commands (connection lifecycle)
// ---------------------------------------------------------------------------

/// Connect to a specific Kubernetes context and start watching pods.
#[tauri::command]
async fn connect_to_context(
    app: AppHandle,
    state: State<'_, AppState>,
    context_name: String,
) -> Result<(), String> {
    info!("Connecting to context: {}", context_name);

    // Abort any existing watch task.
    abort_watch(&state).await;

    // Clear previous data.
    {
        let store = state
            .store
            .lock()
            .map_err(|e| format!("Store lock failed: {}", e))?;
        clear_all_resources(&store);
    }
    {
        let mut ctx = state.active_context.write().await;
        *ctx = None;
    }
    {
        let mut active_connection = state.active_connection.write().await;
        *active_connection = None;
    }

    // Update connection state to Connecting.
    set_connection_state(&app, &state.connection_state, ConnectionState::Connecting).await;

    // Build a kube client for the requested context.
    let client = telescope_engine::client::create_client_for_context(&context_name)
        .await
        .map_err(|e| {
            let msg = format!("Failed to connect to context '{}': {}", context_name, e);
            error!("{}", msg);
            msg
        })?;

    // Update active context and read namespace.
    {
        let mut ctx = state.active_context.write().await;
        *ctx = Some(context_name.clone());
    }
    {
        let mut active_connection = state.active_connection.write().await;
        *active_connection = Some(ActiveConnection {
            context_name: context_name.clone(),
            client: client.clone(),
        });
    }
    let namespace = state.active_namespace.read().await.clone();

    // Spawn the watcher background task.
    spawn_watch_task(&app, &state, client, &namespace).await;

    write_audit_entry(
        &state,
        Some(context_name.clone()),
        namespace.clone(),
        "connect",
        "context",
        context_name.clone(),
        "success",
        None,
    )
    .await?;

    info!(
        "Watch started for context={}, namespace={}",
        context_name, namespace
    );
    Ok(())
}

/// Disconnect from the current cluster.
#[tauri::command]
async fn disconnect(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let ctx_name = state
        .active_context
        .read()
        .await
        .clone()
        .unwrap_or_default();
    info!("Disconnecting");

    abort_watch(&state).await;

    // Clear stored data.
    {
        let store = state
            .store
            .lock()
            .map_err(|e| format!("Store lock failed: {}", e))?;
        clear_all_resources(&store);
    }

    // Reset state.
    {
        let mut ctx = state.active_context.write().await;
        *ctx = None;
    }
    {
        let mut active_connection = state.active_connection.write().await;
        *active_connection = None;
    }

    set_connection_state(&app, &state.connection_state, ConnectionState::Disconnected).await;

    write_audit_entry(
        &state,
        Some(ctx_name.clone()),
        String::new(),
        "disconnect",
        "context",
        ctx_name,
        "success",
        None,
    )
    .await?;

    Ok(())
}

/// Change the watched namespace and restart the watch.
#[tauri::command]
async fn set_namespace(
    app: AppHandle,
    state: State<'_, AppState>,
    namespace: String,
) -> Result<(), String> {
    let namespace = validate_namespace_param(&namespace)?;
    info!("Switching namespace to: {}", namespace);

    {
        let mut ns = state.active_namespace.write().await;
        *ns = namespace.clone();
    }

    // If we have an active context, reconnect with the new namespace.
    let context_name = state.active_context.read().await.clone();
    if let Some(ctx) = context_name {
        abort_watch(&state).await;

        // Clear old resource data.
        {
            let store = state
                .store
                .lock()
                .map_err(|e| format!("Store lock failed: {}", e))?;
            clear_all_resources(&store);
        }
        {
            let mut active_connection = state.active_connection.write().await;
            *active_connection = None;
        }

        set_connection_state(&app, &state.connection_state, ConnectionState::Connecting).await;

        let client = telescope_engine::client::create_client_for_context(&ctx)
            .await
            .map_err(|e| e.to_string())?;
        {
            let mut active_connection = state.active_connection.write().await;
            *active_connection = Some(ActiveConnection {
                context_name: ctx.clone(),
                client: client.clone(),
            });
        }

        spawn_watch_task(&app, &state, client, &namespace).await;
    }

    Ok(())
}

/// Get the currently active namespace.
#[tauri::command]
async fn get_namespace(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.active_namespace.read().await.clone())
}

// ---------------------------------------------------------------------------
// Log commands
// ---------------------------------------------------------------------------

/// Fetch pod logs (non-streaming snapshot).
#[tauri::command]
async fn get_pod_logs(
    state: State<'_, AppState>,
    namespace: String,
    pod: String,
    container: Option<String>,
    previous: Option<bool>,
    tail_lines: Option<i64>,
) -> Result<String, String> {
    let namespace = validate_namespace_param(&namespace)?;
    let pod = validate_k8s_name_param(&pod, "pod")?;
    let container = validate_optional_k8s_name_param(container, "container")?;
    let client = active_client(&state).await?;

    let req = telescope_engine::logs::LogRequest {
        namespace,
        pod,
        container,
        previous: previous.unwrap_or(false),
        tail_lines: tail_lines.or(Some(1000)),
        follow: false,
    };

    telescope_engine::logs::get_pod_logs(&client, &req)
        .await
        .map_err(|e| e.to_string())
}

/// List containers in a pod.
#[tauri::command]
async fn list_containers(
    state: State<'_, AppState>,
    namespace: String,
    pod: String,
) -> Result<Vec<String>, String> {
    let namespace = validate_namespace_param(&namespace)?;
    let pod = validate_k8s_name_param(&pod, "pod")?;
    let client = active_client(&state).await?;

    telescope_engine::logs::list_containers(&client, &namespace, &pod)
        .await
        .map_err(|e| e.to_string())
}

/// Start streaming logs for a pod. Emits `log-chunk` events to the frontend.
#[tauri::command]
async fn start_log_stream(
    app: AppHandle,
    state: State<'_, AppState>,
    namespace: String,
    pod: String,
    container: Option<String>,
    tail_lines: Option<i64>,
) -> Result<(), String> {
    let namespace = validate_namespace_param(&namespace)?;
    let pod = validate_k8s_name_param(&pod, "pod")?;
    let container = validate_optional_k8s_name_param(container, "container")?;
    let client = active_client(&state).await?;

    let req = telescope_engine::logs::LogRequest {
        namespace,
        pod: pod.clone(),
        container,
        previous: false,
        tail_lines: tail_lines.or(Some(100)),
        follow: true,
    };

    let reader = telescope_engine::logs::stream_pod_logs(&client, &req)
        .await
        .map_err(|e| e.to_string())?;

    let app_handle = app.clone();
    let pod_name = pod.clone();

    tokio::spawn(async move {
        futures::pin_mut!(reader);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.try_next().await {
            let chunk = telescope_engine::logs::LogChunk {
                lines: line,
                is_complete: false,
            };
            app_handle.emit("log-chunk", &chunk).ok();
        }
        // Signal stream complete
        let done = telescope_engine::logs::LogChunk {
            lines: String::new(),
            is_complete: true,
        };
        app_handle.emit("log-chunk", &done).ok();
        info!("Log stream ended for pod {}", pod_name);
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// Resource actions
// ---------------------------------------------------------------------------

#[tauri::command]
async fn apply_dynamic_resource(
    state: State<'_, AppState>,
    group: String,
    version: String,
    kind: String,
    plural: String,
    namespace: Option<String>,
    manifest: String,
    dry_run: bool,
) -> Result<telescope_engine::actions::ApplyResult, String> {
    let group = validate_identifier_param(&group, "group")?;
    let version = validate_identifier_param(&version, "version")?;
    let kind = validate_identifier_param(&kind, "kind")?;
    let plural = validate_identifier_param(&plural, "plural")?;
    let namespace = namespace
        .as_deref()
        .map(validate_namespace_param)
        .transpose()?;
    telescope_engine::actions::validate_apply_resource_content(&manifest)
        .map_err(|error| error.to_string())?;
    let client = active_client(&state).await?;
    let outcome = telescope_engine::dynamic::apply_dynamic_resource(
        &client,
        &group,
        &version,
        &kind,
        &plural,
        namespace.as_deref(),
        &manifest,
        dry_run,
    )
    .await;
    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        namespace.clone().unwrap_or_default(),
        "apply_dynamic",
        format!("{group}/{version}/{kind}"),
        String::new(),
        result_str,
        Some(format!("dry_run={dry_run}")),
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

#[tauri::command]
async fn delete_dynamic_resource(
    state: State<'_, AppState>,
    group: String,
    version: String,
    kind: String,
    plural: String,
    namespace: Option<String>,
    name: String,
) -> Result<String, String> {
    let group = validate_identifier_param(&group, "group")?;
    let version = validate_identifier_param(&version, "version")?;
    let kind = validate_identifier_param(&kind, "kind")?;
    let plural = validate_identifier_param(&plural, "plural")?;
    let namespace = namespace
        .as_deref()
        .map(validate_namespace_param)
        .transpose()?;
    let name = validate_identifier_param(&name, "name")?;
    let client = active_client(&state).await?;
    let outcome = telescope_engine::dynamic::delete_dynamic_resource(
        &client,
        &group,
        &version,
        &kind,
        &plural,
        namespace.as_deref().unwrap_or_default(),
        &name,
    )
    .await;
    let result_str = match &outcome {
        Ok(r) if r.success => "success",
        _ => "failure",
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        namespace.clone().unwrap_or_default(),
        "delete_dynamic",
        format!("{group}/{version}/{kind}"),
        name.clone(),
        result_str,
        None,
    )
    .await;
    let result = finish_audited_command(outcome, audit_result)?;
    if result.success {
        Ok(result.message)
    } else {
        Err(result.message)
    }
}

/// Scale a Deployment or StatefulSet to the specified replica count.
#[tauri::command]
async fn scale_resource(
    state: State<'_, AppState>,
    gvk: String,
    namespace: String,
    name: String,
    replicas: i32,
) -> Result<String, String> {
    let namespace = validate_namespace_param(&namespace)?;
    let name = validate_k8s_name_param(&name, "name")?;
    let replicas = validate_i32_param(replicas, "replicas", 0, validation::MAX_REPLICAS)?;
    let client = active_client(&state).await?;
    let outcome =
        telescope_engine::actions::scale_resource(&client, &gvk, &namespace, &name, replicas).await;
    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        namespace.clone(),
        "scale",
        gvk.clone(),
        name.clone(),
        result_str,
        Some(format!("replicas={replicas}")),
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

/// Delete a namespaced Kubernetes resource by GVK, namespace, and name.
#[tauri::command]
async fn delete_resource(
    state: State<'_, AppState>,
    gvk: String,
    namespace: String,
    name: String,
) -> Result<String, String> {
    let namespace = validate_namespace_param(&namespace)?;
    let name = validate_k8s_name_param(&name, "name")?;
    let client = active_client(&state).await?;
    let outcome =
        telescope_engine::actions::delete_resource(&client, &gvk, &namespace, &name).await;
    let result_str = match &outcome {
        Ok(r) if r.success => "success",
        _ => "failure",
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        namespace.clone(),
        "delete",
        gvk.clone(),
        name.clone(),
        result_str,
        None,
    )
    .await;
    let result = finish_audited_command(outcome, audit_result)?;
    if result.success {
        Ok(result.message)
    } else {
        Err(result.message)
    }
}

/// Apply a resource from JSON/YAML content using server-side apply.
#[tauri::command]
async fn apply_resource(
    state: State<'_, AppState>,
    json_content: String,
    dry_run: bool,
) -> Result<telescope_engine::actions::ApplyResult, String> {
    telescope_engine::actions::validate_apply_resource_content(&json_content)
        .map_err(|error| error.to_string())?;
    let client = active_client(&state).await?;
    let outcome = telescope_engine::actions::apply_resource(&client, &json_content, dry_run).await;
    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        String::new(),
        "apply",
        "resource",
        String::new(),
        result_str,
        Some(format!("dry_run={dry_run}")),
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn validate_k8s_name_param(value: &str, field: &str) -> Result<String, String> {
    validation::validate_k8s_name_field(value, field).map_err(|error| error.to_string())
}

fn validate_namespace_param(value: &str) -> Result<String, String> {
    validation::validate_namespace(value).map_err(|error| error.to_string())
}

fn validate_identifier_param(value: &str, field: &str) -> Result<String, String> {
    validation::validate_identifier(value, field).map_err(|error| error.to_string())
}

fn validate_i32_param(value: i32, field: &str, min: i32, max: i32) -> Result<i32, String> {
    validation::validate_i32_range(value, field, min, max).map_err(|error| error.to_string())
}

fn validate_i64_param(value: i64, field: &str, min: i64, max: i64) -> Result<i64, String> {
    validation::validate_i64_range(value, field, min, max).map_err(|error| error.to_string())
}

fn validate_optional_allowed_value_param(
    value: Option<String>,
    field: &str,
    allowed: &[&str],
) -> Result<Option<String>, String> {
    match normalize_optional_string(value) {
        Some(value) => validation::validate_allowed_value(&value, field, allowed)
            .map(Some)
            .map_err(|error| error.to_string()),
        None => Ok(None),
    }
}

fn validate_optional_k8s_name_param(
    value: Option<String>,
    field: &str,
) -> Result<Option<String>, String> {
    match normalize_optional_string(value) {
        Some(value) => validate_k8s_name_param(&value, field).map(Some),
        None => Ok(None),
    }
}

fn validate_optional_kubernetes_version_param(
    value: Option<String>,
    field: &str,
) -> Result<Option<String>, String> {
    match normalize_optional_string(value) {
        Some(value) => validation::validate_kubernetes_version(&value, field)
            .map(Some)
            .map_err(|error| error.to_string()),
        None => Ok(None),
    }
}

fn validate_create_node_pool_config(
    config: CreateNodePoolConfig,
) -> Result<CreateNodePoolRequest, String> {
    let name =
        validation::validate_aks_node_pool_name(&config.name).map_err(|error| error.to_string())?;
    let vm_size =
        validation::validate_aks_vm_size(&config.vm_size).map_err(|error| error.to_string())?;
    let count = validate_i32_param(config.count, "count", 1, validation::MAX_NODE_POOL_COUNT)?;
    let os_type =
        validate_optional_allowed_value_param(config.os_type, "osType", &["Linux", "Windows"])?;
    let mode = validate_optional_allowed_value_param(config.mode, "mode", &["User", "System"])?;
    let orchestrator_version = validate_optional_kubernetes_version_param(
        config.orchestrator_version,
        "orchestratorVersion",
    )?;

    let enable_auto_scaling = config.enable_auto_scaling.unwrap_or(false);
    let (min_count, max_count) = validation::validate_autoscaler_bounds(
        enable_auto_scaling,
        config.min_count,
        config.max_count,
    )
    .map_err(|error| error.to_string())?;

    if let (Some(min_count), Some(max_count)) = (min_count, max_count) {
        if count < min_count || count > max_count {
            return Err(format!(
                "count must be between {min_count} and {max_count} when autoscaling is enabled"
            ));
        }
    }

    let availability_zones = match config.availability_zones {
        Some(zones) => {
            let validated = validation::validate_aks_availability_zones(&zones)
                .map_err(|error| error.to_string())?;
            if validated.is_empty() {
                None
            } else {
                Some(validated)
            }
        }
        None => None,
    };

    let max_pods = match config.max_pods {
        Some(max_pods) => Some(validate_i32_param(max_pods, "maxPods", 10, 250)?),
        None => None,
    };

    let node_labels = match config.node_labels {
        Some(labels) => {
            let mut validated = HashMap::with_capacity(labels.len());
            for (key, value) in labels {
                let key = validate_identifier_param(&key, "node label key")?;
                let value = validate_identifier_param(&value, "node label value")?;
                validated.insert(key, value);
            }

            if validated.is_empty() {
                None
            } else {
                Some(validated)
            }
        }
        None => None,
    };

    let node_taints = match config.node_taints {
        Some(taints) => {
            let mut validated = Vec::with_capacity(taints.len());
            for taint in taints {
                validated.push(validate_identifier_param(&taint, "node taint")?);
            }

            if validated.is_empty() {
                None
            } else {
                Some(validated)
            }
        }
        None => None,
    };

    Ok(CreateNodePoolRequest {
        name,
        vm_size,
        count,
        os_type,
        mode,
        orchestrator_version,
        enable_auto_scaling: Some(enable_auto_scaling),
        min_count,
        max_count,
        availability_zones,
        max_pods,
        node_labels,
        node_taints,
    })
}

/// Abort the current watch task if one is running.
async fn abort_watch(state: &State<'_, AppState>) {
    let mut handle = state.watch_handle.lock().await;
    if let Some(h) = handle.take() {
        h.abort();
        info!("Previous watch task aborted");
    }
}

fn parse_azure_cloud(value: &str) -> Option<AzureCloud> {
    match value.trim().to_ascii_lowercase().as_str() {
        "commercial" => Some(AzureCloud::Commercial),
        "usgovernment" | "us-government" | "us_government" => Some(AzureCloud::UsGovernment),
        "usgovsecret" | "us-gov-secret" | "us_gov_secret" => Some(AzureCloud::UsGovSecret),
        "usgovtopsecret" | "us-gov-top-secret" | "us_gov_top_secret" => {
            Some(AzureCloud::UsGovTopSecret)
        }
        _ => None,
    }
}

fn azure_cloud_name(cloud: AzureCloud) -> &'static str {
    match cloud {
        AzureCloud::Commercial => "Commercial",
        AzureCloud::UsGovernment => "UsGovernment",
        AzureCloud::UsGovSecret => "UsGovSecret",
        AzureCloud::UsGovTopSecret => "UsGovTopSecret",
    }
}

async fn detect_active_azure_cloud(state: &State<'_, AppState>) -> Result<AzureCloud, String> {
    let context_name = if let Some(context_name) = state.active_context.read().await.clone() {
        context_name
    } else {
        telescope_engine::kubeconfig::active_context().map_err(|e| e.to_string())?
    };

    let server_url = telescope_engine::kubeconfig::list_contexts()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|ctx| ctx.name == context_name)
        .and_then(|ctx| ctx.cluster_server);

    Ok(server_url
        .as_deref()
        .map(AzureCloud::detect_from_url)
        .unwrap_or_default())
}

async fn resolve_active_aks_resource_id(
    state: &State<'_, AppState>,
) -> Result<Option<telescope_azure::AksResourceId>, String> {
    let connection = active_connection(state).await?;
    let info =
        telescope_engine::client::get_cluster_info(&connection.client, &connection.context_name)
            .await
            .map_err(|e| e.to_string())?;

    if !info.is_aks {
        return Ok(None);
    }

    let preferred_id = {
        let store_guard = state.store.lock().map_err(|e| e.to_string())?;
        telescope_azure::resolve_aks_identity_from_preferences(Some(&store_guard))
    };

    Ok(telescope_azure::resolve_aks_identity(&info.server_url, preferred_id).await)
}

async fn configured_azure_cloud(state: &State<'_, AppState>) -> Result<AzureCloud, String> {
    let preference = {
        let store = state.store.lock().map_err(|e| e.to_string())?;
        store
            .get_preference("azure_cloud")
            .map_err(|e| e.to_string())?
    };

    if let Some(value) = preference {
        let trimmed = value.trim();
        if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("auto") {
            return parse_azure_cloud(trimmed)
                .ok_or_else(|| format!("Invalid azure_cloud preference: {}", trimmed));
        }
    }

    detect_active_azure_cloud(state).await
}

async fn active_aks_arm_client(
    state: &State<'_, AppState>,
) -> Result<(ArmClient, telescope_azure::AksResourceId), String> {
    active_aks_arm_client_for(state, "This action requires an AKS cluster").await
}

async fn active_aks_arm_client_for(
    state: &State<'_, AppState>,
    missing_cluster_message: &str,
) -> Result<(ArmClient, telescope_azure::AksResourceId), String> {
    let connection = active_connection(state).await?;
    let info =
        telescope_engine::client::get_cluster_info(&connection.client, &connection.context_name)
            .await
            .map_err(|e| e.to_string())?;

    if !info.is_aks {
        return Err(missing_cluster_message.to_string());
    }

    let preference_status = {
        let store_guard = state.store.lock().map_err(|e| e.to_string())?;
        telescope_azure::inspect_aks_identity_preferences(Some(&store_guard))
    };
    let preferred_id = match &preference_status {
        telescope_azure::AksIdentityPreferenceStatus::Complete(id) => Some(id.clone()),
        telescope_azure::AksIdentityPreferenceStatus::Missing
        | telescope_azure::AksIdentityPreferenceStatus::Incomplete { .. } => None,
    };
    let resource_id = telescope_azure::resolve_aks_identity(&info.server_url, preferred_id)
        .await
        .ok_or_else(|| {
            telescope_azure::unresolved_aks_identity_message(&info.server_url, &preference_status)
        })?;
    let cloud = configured_azure_cloud(state).await?;
    let client = ArmClient::new(cloud).map_err(|e| e.to_string())?;
    Ok((client, resource_id))
}

async fn active_client(state: &State<'_, AppState>) -> Result<kube::Client, String> {
    Ok(active_connection(state).await?.client)
}

async fn active_connection(state: &State<'_, AppState>) -> Result<ActiveConnection, String> {
    let lifecycle_state = state.connection_state.read().await.clone();
    match lifecycle_state {
        ConnectionState::Ready
        | ConnectionState::Syncing { .. }
        | ConnectionState::Degraded { .. } => {}
        ConnectionState::Disconnected => return Err("Not connected".to_string()),
        ConnectionState::Connecting => {
            return Err(
                "Connection is still initializing; try again once sync completes".to_string(),
            )
        }
        ConnectionState::Error { message } => {
            return Err(format!("Connection is unavailable: {message}"))
        }
        ConnectionState::Backoff { .. } => {
            return Err("Connection is retrying after a watch error; try again shortly".to_string())
        }
    }

    state
        .active_connection
        .read()
        .await
        .clone()
        .ok_or_else(|| "Active connection is unavailable".to_string())
}

/// Update connection state and emit an event to the frontend.
async fn set_connection_state(
    app: &AppHandle,
    conn_state: &Arc<RwLock<ConnectionState>>,
    new_state: ConnectionState,
) {
    {
        let mut s = conn_state.write().await;
        *s = new_state.clone();
    }
    app.emit("connection-state-changed", &new_state).ok();
}

/// Spawn background tasks that watch pods and events, forwarding state changes.
async fn spawn_watch_task(
    app: &AppHandle,
    state: &State<'_, AppState>,
    client: kube::Client,
    namespace: &str,
) {
    let store = Arc::clone(&state.store);
    let conn_state = Arc::clone(&state.connection_state);
    let app_handle = app.clone();
    let ns = namespace.to_string();

    let watcher = telescope_engine::ResourceWatcher::new(client, Arc::clone(&store));
    watcher.register_watches(29); // 28 aux watchers + 1 pod watcher
    let mut state_rx = watcher.state_receiver();

    // Spawn a task to forward state changes from the watcher to the UI.
    let conn_state_fwd = Arc::clone(&conn_state);
    let app_fwd = app_handle.clone();
    let state_forwarder = tokio::spawn(async move {
        while state_rx.changed().await.is_ok() {
            let new_state = state_rx.borrow().clone();
            {
                let mut s = conn_state_fwd.write().await;
                *s = new_state.clone();
            }
            app_fwd.emit("connection-state-changed", &new_state).ok();
        }
    });

    // Collect auxiliary task handles so the main task can abort them on exit.
    let mut aux_tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    // -- Cluster-wide / cluster-scoped watchers --

    let w = watcher.clone();
    aux_tasks.push(tokio::spawn(async move {
        if let Err(e) = w.watch_all_events().await {
            error!("Events watch error: {}", e);
        }
    }));

    let w = watcher.clone();
    aux_tasks.push(tokio::spawn(async move {
        if let Err(e) = w.watch_nodes().await {
            error!("Node watch error: {}", e);
        }
    }));

    let w = watcher.clone();
    aux_tasks.push(tokio::spawn(async move {
        if let Err(e) = w.watch_cluster_roles().await {
            error!("ClusterRole watch error: {}", e);
        }
    }));

    let w = watcher.clone();
    aux_tasks.push(tokio::spawn(async move {
        if let Err(e) = w.watch_cluster_role_bindings().await {
            error!("ClusterRoleBinding watch error: {}", e);
        }
    }));

    let w = watcher.clone();
    aux_tasks.push(tokio::spawn(async move {
        if let Err(e) = w.watch_priority_classes().await {
            error!("PriorityClass watch error: {}", e);
        }
    }));

    let w = watcher.clone();
    aux_tasks.push(tokio::spawn(async move {
        if let Err(e) = w.watch_validating_webhooks().await {
            error!("ValidatingWebhookConfiguration watch error: {}", e);
        }
    }));

    let w = watcher.clone();
    aux_tasks.push(tokio::spawn(async move {
        if let Err(e) = w.watch_mutating_webhooks().await {
            error!("MutatingWebhookConfiguration watch error: {}", e);
        }
    }));

    let w = watcher.clone();
    aux_tasks.push(tokio::spawn(async move {
        if let Err(e) = w.watch_storage_classes().await {
            error!("StorageClass watch error: {}", e);
        }
    }));

    let w = watcher.clone();
    aux_tasks.push(tokio::spawn(async move {
        if let Err(e) = w.watch_persistent_volumes().await {
            error!("PersistentVolume watch error: {}", e);
        }
    }));

    // -- Namespaced resource watchers --
    // Uses a macro to reduce boilerplate for each resource type.
    macro_rules! spawn_ns_watch {
        ($watcher:expr, $ns:expr, $tasks:ident, $method:ident, $label:expr) => {{
            let w = $watcher.clone();
            let ns_clone = $ns.clone();
            $tasks.push(tokio::spawn(async move {
                if let Err(e) = w.$method(&ns_clone).await {
                    error!("{} watch error: {}", $label, e);
                }
            }));
        }};
    }

    spawn_ns_watch!(watcher, ns, aux_tasks, watch_deployments, "Deployment");
    spawn_ns_watch!(watcher, ns, aux_tasks, watch_statefulsets, "StatefulSet");
    spawn_ns_watch!(watcher, ns, aux_tasks, watch_daemonsets, "DaemonSet");
    spawn_ns_watch!(watcher, ns, aux_tasks, watch_replicasets, "ReplicaSet");
    spawn_ns_watch!(watcher, ns, aux_tasks, watch_services, "Service");
    spawn_ns_watch!(watcher, ns, aux_tasks, watch_config_maps, "ConfigMap");
    spawn_ns_watch!(watcher, ns, aux_tasks, watch_jobs, "Job");
    spawn_ns_watch!(watcher, ns, aux_tasks, watch_cronjobs, "CronJob");
    spawn_ns_watch!(watcher, ns, aux_tasks, watch_ingresses, "Ingress");
    spawn_ns_watch!(
        watcher,
        ns,
        aux_tasks,
        watch_network_policies,
        "NetworkPolicy"
    );
    spawn_ns_watch!(
        watcher,
        ns,
        aux_tasks,
        watch_endpoint_slices,
        "EndpointSlice"
    );
    spawn_ns_watch!(watcher, ns, aux_tasks, watch_pvcs, "PVC");
    spawn_ns_watch!(
        watcher,
        ns,
        aux_tasks,
        watch_resource_quotas,
        "ResourceQuota"
    );
    spawn_ns_watch!(watcher, ns, aux_tasks, watch_limit_ranges, "LimitRange");
    spawn_ns_watch!(watcher, ns, aux_tasks, watch_roles, "Role");
    spawn_ns_watch!(watcher, ns, aux_tasks, watch_role_bindings, "RoleBinding");
    spawn_ns_watch!(
        watcher,
        ns,
        aux_tasks,
        watch_service_accounts,
        "ServiceAccount"
    );
    spawn_ns_watch!(watcher, ns, aux_tasks, watch_hpas, "HPA");
    spawn_ns_watch!(watcher, ns, aux_tasks, watch_pod_disruption_budgets, "PDB");

    // Spawn the main watch task (pods + lifecycle coordinator).
    let task = tokio::spawn(async move {
        if let Err(e) = watcher.watch_pods(&ns).await {
            error!("Pod watch error: {}", e);
        }
        // When the pod watch ends, abort the state forwarder and all auxiliary watchers.
        state_forwarder.abort();
        for t in aux_tasks {
            t.abort();
        }
    });

    let mut handle = state.watch_handle.lock().await;
    *handle = Some(task);
}

/// Restart a Deployment, StatefulSet, or DaemonSet rollout.
#[tauri::command]
async fn rollout_restart(
    state: State<'_, AppState>,
    gvk: String,
    namespace: String,
    name: String,
) -> Result<String, String> {
    let namespace = validate_namespace_param(&namespace)?;
    let name = validate_k8s_name_param(&name, "name")?;
    let client = active_client(&state).await?;
    let outcome =
        telescope_engine::actions::rollout_restart(&client, &gvk, &namespace, &name).await;
    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        namespace.clone(),
        "rollout_restart",
        gvk.clone(),
        name.clone(),
        result_str,
        None,
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

/// Get rollout status for a Deployment or StatefulSet.
#[tauri::command]
async fn rollout_status(
    state: State<'_, AppState>,
    gvk: String,
    namespace: String,
    name: String,
) -> Result<telescope_engine::actions::RolloutStatus, String> {
    let client = active_client(&state).await?;
    telescope_engine::actions::rollout_status(&client, &gvk, &namespace, &name)
        .await
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Node operations
// ---------------------------------------------------------------------------

/// Cordon a node (mark as unschedulable).
#[tauri::command]
async fn cordon_node(state: State<'_, AppState>, name: String) -> Result<String, String> {
    let name = validate_k8s_name_param(&name, "node name")?;
    let client = active_client(&state).await?;
    let outcome = telescope_engine::node_ops::cordon_node(&client, &name).await;
    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        String::new(),
        "cordon",
        "Node",
        name.clone(),
        result_str,
        None,
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

/// Uncordon a node (mark as schedulable).
#[tauri::command]
async fn uncordon_node(state: State<'_, AppState>, name: String) -> Result<String, String> {
    let name = validate_k8s_name_param(&name, "node name")?;
    let client = active_client(&state).await?;
    let outcome = telescope_engine::node_ops::uncordon_node(&client, &name).await;
    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        String::new(),
        "uncordon",
        "Node",
        name.clone(),
        result_str,
        None,
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

/// Drain a node: cordon then evict eligible pods.
#[tauri::command]
async fn drain_node(
    state: State<'_, AppState>,
    name: String,
    grace_period: Option<i64>,
    ignore_daemonsets: Option<bool>,
    force: Option<bool>,
) -> Result<telescope_engine::node_ops::DrainResult, String> {
    let name = validate_k8s_name_param(&name, "node name")?;
    let grace_period = validate_i64_param(
        grace_period.unwrap_or(30),
        "gracePeriod",
        0,
        validation::MAX_DRAIN_GRACE_PERIOD_SECONDS,
    )?;
    let client = active_client(&state).await?;
    let options = telescope_engine::node_ops::DrainOptions {
        grace_period,
        ignore_daemonsets: ignore_daemonsets.unwrap_or(true),
        force: force.unwrap_or(false),
    };
    let outcome = telescope_engine::node_ops::drain_node(&client, &name, &options).await;
    let result_str = match &outcome {
        Ok(r) if r.success => "success",
        _ => "failure",
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        String::new(),
        "drain",
        "Node",
        name.clone(),
        result_str,
        Some(format!(
            "grace_period={}, ignore_daemonsets={}, force={}",
            options.grace_period, options.ignore_daemonsets, options.force
        )),
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

/// Add a taint to a node.
#[tauri::command]
async fn add_node_taint(
    state: State<'_, AppState>,
    name: String,
    key: String,
    value: String,
    effect: String,
) -> Result<String, String> {
    let name = validate_k8s_name_param(&name, "node name")?;
    let key = validate_identifier_param(&key, "key")?;
    let value = validate_identifier_param(&value, "value")?;
    let effect = validation::validate_taint_effect(&effect).map_err(|error| error.to_string())?;
    let client = active_client(&state).await?;
    let outcome =
        telescope_engine::node_ops::add_taint(&client, &name, &key, &value, &effect).await;
    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        String::new(),
        "add_taint",
        "Node",
        name.clone(),
        result_str,
        Some(format!("{key}={value}:{effect}")),
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

/// Remove a taint from a node by key.
#[tauri::command]
async fn remove_node_taint(
    state: State<'_, AppState>,
    name: String,
    key: String,
) -> Result<String, String> {
    let name = validate_k8s_name_param(&name, "node name")?;
    let key = validate_identifier_param(&key, "key")?;
    let client = active_client(&state).await?;
    let outcome = telescope_engine::node_ops::remove_taint(&client, &name, &key).await;
    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        String::new(),
        "remove_taint",
        "Node",
        name.clone(),
        result_str,
        Some(format!("key={key}")),
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

/// Fetch maintenance configurations for an AKS cluster.
#[tauri::command]
async fn list_aks_maintenance_configs(
    state: State<'_, AppState>,
) -> Result<Vec<MaintenanceConfig>, String> {
    let resource_id = resolve_active_aks_resource_id(&state)
        .await?
        .ok_or_else(|| "Maintenance config requires an AKS cluster".to_string())?;
    let cloud = detect_active_azure_cloud(&state).await?;
    let arm_client = ArmClient::new(cloud).map_err(|e| e.to_string())?;
    telescope_azure::list_maintenance_configs(&arm_client, &resource_id)
        .await
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Container exec
// ---------------------------------------------------------------------------

/// Execute a command in a running container (non-interactive).
#[tauri::command]
async fn exec_command(
    state: State<'_, AppState>,
    namespace: String,
    pod: String,
    container: Option<String>,
    command: Vec<String>,
) -> Result<telescope_engine::exec::ExecResult, String> {
    let namespace = validate_namespace_param(&namespace)?;
    let pod = validate_k8s_name_param(&pod, "pod")?;
    let container = validate_optional_k8s_name_param(container, "container")?;
    let command = validation::validate_exec_command(&command).map_err(|error| error.to_string())?;
    let cmd_detail = command.join(" ");
    let audit_ns = namespace.clone();
    let audit_pod = pod.clone();
    let client = active_client(&state).await?;
    let req = telescope_engine::exec::ExecRequest {
        namespace,
        pod,
        container,
        command,
    };
    let outcome = telescope_engine::exec::exec_command(&client, &req).await;
    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    let audit_result = write_audit_entry(
        &state,
        None,
        audit_ns,
        "exec",
        "Pod",
        audit_pod,
        result_str,
        Some(cmd_detail),
    )
    .await;
    finish_audited_command(outcome, audit_result)
}

/// Start port forwarding from a local port to a pod port.
#[tauri::command]
async fn start_port_forward(
    state: State<'_, AppState>,
    namespace: String,
    pod: String,
    local_port: u16,
    remote_port: u16,
) -> Result<u16, String> {
    let namespace = validate_namespace_param(&namespace)?;
    let pod = validate_k8s_name_param(&pod, "pod")?;
    let client = active_client(&state).await?;
    let req = telescope_engine::portforward::PortForwardRequest {
        namespace,
        pod,
        local_port,
        remote_port,
    };
    telescope_engine::portforward::start_port_forward(&client, &req)
        .await
        .map_err(|e| e.to_string())
}

// Metrics commands
// ---------------------------------------------------------------------------

#[tauri::command]
async fn get_pod_metrics(
    state: State<'_, AppState>,
    namespace: Option<String>,
) -> Result<Vec<telescope_engine::metrics::PodMetrics>, String> {
    let client = active_client(&state).await?;
    telescope_engine::metrics::get_pod_metrics(&client, namespace.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn check_metrics_available(state: State<'_, AppState>) -> Result<bool, String> {
    let client = active_client(&state).await?;
    Ok(telescope_engine::metrics::is_metrics_available(&client).await)
}

#[tauri::command]
async fn get_node_metrics(
    state: State<'_, AppState>,
) -> Result<Vec<telescope_engine::metrics::NodeMetricsData>, String> {
    let client = active_client(&state).await?;
    telescope_engine::metrics::get_node_metrics(&client)
        .await
        .map_err(|e| e.to_string())
}

// Entry point
// ---------------------------------------------------------------------------

// ── CRD discovery commands ────────────────────────────────────────────────

/// List all Custom Resource Definitions installed on the cluster.
#[tauri::command]
async fn list_crds(
    state: State<'_, AppState>,
) -> Result<Vec<telescope_engine::crd::CrdInfo>, String> {
    let client = active_client(&state).await?;
    telescope_engine::crd::list_crds(&client)
        .await
        .map_err(|e| e.to_string())
}

// ── User preference commands ──────────────────────────────────────────────

#[tauri::command]
fn get_preference(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    store.get_preference(&key).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_preference(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    store
        .set_preference(&key, &value)
        .map_err(|e| e.to_string())
}

/// Fetch comprehensive AKS cluster details from the Azure ARM API.
#[tauri::command]
async fn get_aks_cluster_detail(
    state: State<'_, AppState>,
) -> Result<Option<telescope_azure::aks::AksClusterDetail>, String> {
    let resource_id = match resolve_active_aks_resource_id(&state).await? {
        Some(resource_id) => resource_id,
        None => return Ok(None),
    };
    let cloud = configured_azure_cloud(&state).await?;
    let arm_client = telescope_azure::ArmClient::new(cloud).map_err(|e| e.to_string())?;

    match telescope_azure::aks::get_cluster(&arm_client, &resource_id).await {
        Ok(detail) => Ok(Some(detail)),
        Err(e) => {
            tracing::warn!("Failed to fetch AKS cluster detail: {e}");
            Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn get_azure_cloud(state: State<'_, AppState>) -> Result<String, String> {
    Ok(azure_cloud_name(configured_azure_cloud(&state).await?).to_string())
}

#[tauri::command]
async fn set_azure_cloud(state: State<'_, AppState>, cloud: String) -> Result<(), String> {
    let normalized = if cloud.trim().is_empty() || cloud.eq_ignore_ascii_case("auto") {
        "auto".to_string()
    } else {
        let cloud = parse_azure_cloud(&cloud)
            .ok_or_else(|| format!("Unsupported Azure cloud: {}", cloud))?;
        azure_cloud_name(cloud).to_string()
    };

    let store = state.store.lock().map_err(|e| e.to_string())?;
    store
        .set_preference("azure_cloud", &normalized)
        .map_err(|e| e.to_string())
}

fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = run() {
        error!("Fatal startup error: {}", e);
        eprintln!("Telescope failed to start: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Telescope desktop app");

    let data_dir = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(|h| std::path::PathBuf::from(h).join(".telescope"))
        .unwrap_or_else(|_| std::env::temp_dir().join("telescope"));
    let db_path = data_dir.join("resources.db");
    debug!(db_path = ?db_path, "Resolved desktop database path");
    // safe: path always has a parent after join()
    std::fs::create_dir_all(db_path.parent().unwrap())?;

    let db_path_str = db_path.to_string_lossy().to_string();
    let store = ResourceStore::open(&db_path_str)?;

    // Set restrictive file permissions on the database (Unix only).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o600)).ok();
    }

    let audit_path = data_dir.join("audit.log").to_string_lossy().to_string();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&audit_path)
            .is_ok()
        {
            std::fs::set_permissions(&audit_path, std::fs::Permissions::from_mode(0o600)).ok();
        }
    }

    // Clear stale data from previous runs.
    clear_all_resources(&store);

    let audit_actor = telescope_engine::audit::resolve_actor_identity();

    let app_state = AppState {
        db_path: db_path_str,
        audit_log_path: audit_path,
        audit_actor,
        active_connection: RwLock::new(None),
        store: Arc::new(Mutex::new(store)),
        connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
        watch_handle: TokioMutex::new(None),
        active_context: RwLock::new(None),
        active_namespace: RwLock::new("default".to_string()),
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            list_contexts,
            active_context,
            get_connection_state,
            get_cluster_info,
            resolve_aks_identity,
            list_aks_node_pools,
            start_aks_cluster,
            stop_aks_cluster,
            get_aks_upgrade_profile,
            upgrade_aks_cluster,
            get_pool_upgrade_profile,
            upgrade_pool_version,
            upgrade_pool_node_image,
            scale_aks_node_pool,
            update_aks_autoscaler,
            create_aks_node_pool,
            delete_aks_node_pool,
            get_pods,
            get_resources,
            get_events,
            get_resource_counts,
            search_resources,
            count_resources,
            get_resource,
            list_dynamic_resources,
            get_dynamic_resource,
            get_secrets,
            get_secret,
            list_namespaces,
            create_namespace,
            delete_namespace,
            list_helm_releases,
            get_helm_release_history,
            get_helm_release_values,
            helm_rollback,
            connect_to_context,
            disconnect,
            set_namespace,
            get_namespace,
            get_pod_logs,
            list_containers,
            start_log_stream,
            start_port_forward,
            scale_resource,
            delete_resource,
            apply_resource,
            apply_dynamic_resource,
            delete_dynamic_resource,
            rollout_restart,
            rollout_status,
            cordon_node,
            uncordon_node,
            drain_node,
            add_node_taint,
            remove_node_taint,
            exec_command,
            get_pod_metrics,
            check_metrics_available,
            get_node_metrics,
            list_crds,
            get_preference,
            set_preference,
            get_azure_cloud,
            set_azure_cloud,
            get_aks_cluster_detail,
            list_aks_maintenance_configs,
        ])
        .setup(|_app| {
            info!("Tauri setup complete, loading frontend");
            Ok(())
        })
        .build(tauri::generate_context!())?
        .run(|app, event| {
            if let RunEvent::Exit = event {
                let clear_result: Result<(), String> = {
                    let state = app.state::<AppState>();
                    let result = match state.store.lock() {
                        Ok(store) => {
                            clear_all_resources(&store);
                            Ok(())
                        }
                        Err(error) => Err(error.to_string()),
                    };
                    result
                };
                if let Err(error) = clear_result {
                    error!("Failed to clear cached resources on exit: {}", error);
                }
            }
        });

    Ok(())
}
