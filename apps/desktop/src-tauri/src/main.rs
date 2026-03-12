#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};

use futures::{AsyncBufReadExt, TryStreamExt};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{Mutex as TokioMutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{error, info};

use telescope_core::{ConnectionState, ResourceEntry, ResourceStore};
use telescope_engine::audit::AuditEntry;
use telescope_engine::ClusterContext;

/// All watched GVK strings. Used for cache invalidation on connect,
/// disconnect, namespace switch, and startup cleanup.
const ALL_WATCHED_GVKS: &[&str] = &[
    "v1/Pod",
    "v1/Event",
    "v1/Node",
    "apps/v1/Deployment",
    "apps/v1/StatefulSet",
    "apps/v1/DaemonSet",
    "apps/v1/ReplicaSet",
    "v1/Service",
    "v1/ConfigMap",
    "batch/v1/Job",
    "batch/v1/CronJob",
    "networking.k8s.io/v1/Ingress",
    "v1/PersistentVolumeClaim",
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
    store: Arc<Mutex<ResourceStore>>,
    connection_state: Arc<RwLock<ConnectionState>>,
    watch_handle: TokioMutex<Option<JoinHandle<()>>>,
    active_context: RwLock<Option<String>>,
    active_namespace: RwLock<String>,
}

// ---------------------------------------------------------------------------
// Sync commands (read-only)
// ---------------------------------------------------------------------------

/// Get cluster version and auth info for the connected context.
#[tauri::command]
async fn get_cluster_info(
    state: State<'_, AppState>,
) -> Result<telescope_engine::ClusterInfo, String> {
    let ctx = state.active_context.read().await.clone();
    let context_name = ctx.ok_or_else(|| "Not connected".to_string())?;
    let client = telescope_engine::client::create_client_for_context(&context_name)
        .await
        .map_err(|e| e.to_string())?;
    telescope_engine::client::get_cluster_info(&client, &context_name)
        .await
        .map_err(|e| e.to_string())
}

/// List available Kubernetes contexts from kubeconfig.
#[tauri::command]
fn list_contexts() -> Result<Vec<ClusterContext>, String> {
    eprintln!("[telescope] list_contexts called");
    let result = telescope_engine::kubeconfig::list_contexts().map_err(|e| e.to_string());
    eprintln!(
        "[telescope] list_contexts result: {:?}",
        result.as_ref().map(|v| v.len())
    );
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
    let store = state
        .store
        .lock()
        .map_err(|e| format!("Store lock failed: {}", e))?;
    let events = store
        .list("v1/Event", namespace.as_deref())
        .map_err(|e| e.to_string())?;

    if let Some(obj_name) = involved_object {
        let needle = format!("\"name\":\"{}\"", obj_name);
        Ok(events
            .into_iter()
            .filter(|entry| entry.content.contains(&needle))
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

/// List Helm releases by parsing Helm release Secrets from Kubernetes.
#[tauri::command]
async fn list_helm_releases(
    namespace: Option<String>,
) -> Result<Vec<telescope_engine::helm::HelmRelease>, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    telescope_engine::helm::list_releases(&client, namespace.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// Get all revisions of a specific Helm release, sorted by revision number.
#[tauri::command]
async fn get_helm_release_history(
    namespace: String,
    name: String,
) -> Result<Vec<telescope_engine::helm::HelmRelease>, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
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
    namespace: String,
    name: String,
    reveal: Option<bool>,
) -> Result<String, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    let mut values = telescope_engine::helm::get_release_values(&client, &namespace, &name)
        .await
        .map_err(|e| e.to_string())?;
    if !reveal.unwrap_or(false) {
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&values) {
            telescope_engine::helm::redact_sensitive_values(&mut json);
            values = serde_json::to_string_pretty(&json).unwrap_or(values);
        }
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
    let outcome = telescope_engine::helm::rollback_release(&namespace, &name, revision).await;
    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    telescope_engine::audit::log_audit(
        &state.audit_log_path,
        &AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            context: state
                .active_context
                .read()
                .await
                .clone()
                .unwrap_or_default(),
            namespace: namespace.clone(),
            action: "helm_rollback".into(),
            resource_type: "HelmRelease".into(),
            resource_name: name.clone(),
            result: result_str.into(),
            detail: Some(format!("revision={}", revision)),
        },
    );
    outcome.map_err(|e| e.to_string())
}

/// List available namespaces from the connected cluster.
#[tauri::command]
async fn list_namespaces(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let ctx = state.active_context.read().await.clone();
    if ctx.is_none() {
        return Ok(vec!["default".to_string()]);
    }
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    telescope_engine::namespace::list_namespaces(&client)
        .await
        .map_err(|e| e.to_string())
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
    eprintln!("[telescope] connect_to_context called: {}", context_name);
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
    let namespace = state.active_namespace.read().await.clone();

    // Spawn the watcher background task.
    spawn_watch_task(&app, &state, client, &namespace).await;

    telescope_engine::audit::log_audit(
        &state.audit_log_path,
        &AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            context: context_name.clone(),
            namespace: namespace.clone(),
            action: "connect".into(),
            resource_type: "context".into(),
            resource_name: context_name.clone(),
            result: "success".into(),
            detail: None,
        },
    );

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

    set_connection_state(&app, &state.connection_state, ConnectionState::Disconnected).await;

    telescope_engine::audit::log_audit(
        &state.audit_log_path,
        &AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            context: ctx_name.clone(),
            namespace: String::new(),
            action: "disconnect".into(),
            resource_type: "context".into(),
            resource_name: ctx_name,
            result: "success".into(),
            detail: None,
        },
    );

    Ok(())
}

/// Change the watched namespace and restart the watch.
#[tauri::command]
async fn set_namespace(
    app: AppHandle,
    state: State<'_, AppState>,
    namespace: String,
) -> Result<(), String> {
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

        set_connection_state(&app, &state.connection_state, ConnectionState::Connecting).await;

        let client = telescope_engine::client::create_client_for_context(&ctx)
            .await
            .map_err(|e| e.to_string())?;

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
    namespace: String,
    pod: String,
    container: Option<String>,
    previous: Option<bool>,
    tail_lines: Option<i64>,
) -> Result<String, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;

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
async fn list_containers(namespace: String, pod: String) -> Result<Vec<String>, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;

    telescope_engine::logs::list_containers(&client, &namespace, &pod)
        .await
        .map_err(|e| e.to_string())
}

/// Start streaming logs for a pod. Emits `log-chunk` events to the frontend.
#[tauri::command]
async fn start_log_stream(
    app: AppHandle,
    namespace: String,
    pod: String,
    container: Option<String>,
    tail_lines: Option<i64>,
) -> Result<(), String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;

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

/// Scale a Deployment or StatefulSet to the specified replica count.
#[tauri::command]
async fn scale_resource(
    state: State<'_, AppState>,
    gvk: String,
    namespace: String,
    name: String,
    replicas: i32,
) -> Result<String, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    let outcome =
        telescope_engine::actions::scale_resource(&client, &gvk, &namespace, &name, replicas).await;
    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    telescope_engine::audit::log_audit(
        &state.audit_log_path,
        &AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            context: state
                .active_context
                .read()
                .await
                .clone()
                .unwrap_or_default(),
            namespace: namespace.clone(),
            action: "scale".into(),
            resource_type: gvk.clone(),
            resource_name: name.clone(),
            result: result_str.into(),
            detail: Some(format!("replicas={}", replicas)),
        },
    );
    outcome.map_err(|e| e.to_string())
}

/// Delete a namespaced Kubernetes resource by GVK, namespace, and name.
#[tauri::command]
async fn delete_resource(
    state: State<'_, AppState>,
    gvk: String,
    namespace: String,
    name: String,
) -> Result<String, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    let outcome =
        telescope_engine::actions::delete_resource(&client, &gvk, &namespace, &name).await;
    let result_str = match &outcome {
        Ok(r) if r.success => "success",
        _ => "failure",
    };
    telescope_engine::audit::log_audit(
        &state.audit_log_path,
        &AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            context: state
                .active_context
                .read()
                .await
                .clone()
                .unwrap_or_default(),
            namespace: namespace.clone(),
            action: "delete".into(),
            resource_type: gvk.clone(),
            resource_name: name.clone(),
            result: result_str.into(),
            detail: None,
        },
    );
    let result = outcome.map_err(|e| e.to_string())?;
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
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    let outcome = telescope_engine::actions::apply_resource(&client, &json_content, dry_run).await;
    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    telescope_engine::audit::log_audit(
        &state.audit_log_path,
        &AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            context: state
                .active_context
                .read()
                .await
                .clone()
                .unwrap_or_default(),
            namespace: String::new(),
            action: "apply".into(),
            resource_type: "resource".into(),
            resource_name: String::new(),
            result: result_str.into(),
            detail: Some(format!("dry_run={}", dry_run)),
        },
    );
    outcome.map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Abort the current watch task if one is running.
async fn abort_watch(state: &State<'_, AppState>) {
    let mut handle = state.watch_handle.lock().await;
    if let Some(h) = handle.take() {
        h.abort();
        info!("Previous watch task aborted");
    }
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
    spawn_ns_watch!(watcher, ns, aux_tasks, watch_pvcs, "PVC");

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
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    let outcome =
        telescope_engine::actions::rollout_restart(&client, &gvk, &namespace, &name).await;
    let result_str = if outcome.is_ok() {
        "success"
    } else {
        "failure"
    };
    telescope_engine::audit::log_audit(
        &state.audit_log_path,
        &AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            context: state
                .active_context
                .read()
                .await
                .clone()
                .unwrap_or_default(),
            namespace: namespace.clone(),
            action: "rollout_restart".into(),
            resource_type: gvk.clone(),
            resource_name: name.clone(),
            result: result_str.into(),
            detail: None,
        },
    );
    outcome.map_err(|e| e.to_string())
}

/// Get rollout status for a Deployment or StatefulSet.
#[tauri::command]
async fn rollout_status(
    gvk: String,
    namespace: String,
    name: String,
) -> Result<telescope_engine::actions::RolloutStatus, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    telescope_engine::actions::rollout_status(&client, &gvk, &namespace, &name)
        .await
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
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
    let cmd_detail = command.join(" ");
    let audit_ns = namespace.clone();
    let audit_pod = pod.clone();
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
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
    telescope_engine::audit::log_audit(
        &state.audit_log_path,
        &AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            context: state
                .active_context
                .read()
                .await
                .clone()
                .unwrap_or_default(),
            namespace: audit_ns,
            action: "exec".into(),
            resource_type: "Pod".into(),
            resource_name: audit_pod,
            result: result_str.into(),
            detail: Some(cmd_detail),
        },
    );
    outcome.map_err(|e| e.to_string())
}

/// Start port forwarding from a local port to a pod port.
#[tauri::command]
async fn start_port_forward(
    namespace: String,
    pod: String,
    local_port: u16,
    remote_port: u16,
) -> Result<u16, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
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
    namespace: Option<String>,
) -> Result<Vec<telescope_engine::metrics::PodMetrics>, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    telescope_engine::metrics::get_pod_metrics(&client, namespace.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn check_metrics_available() -> Result<bool, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    Ok(telescope_engine::metrics::is_metrics_available(&client).await)
}

#[tauri::command]
async fn get_node_metrics() -> Result<Vec<telescope_engine::metrics::NodeMetricsData>, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    telescope_engine::metrics::get_node_metrics(&client)
        .await
        .map_err(|e| e.to_string())
}

// Entry point
// ---------------------------------------------------------------------------

// ── CRD discovery commands ────────────────────────────────────────────────

/// List all Custom Resource Definitions installed on the cluster.
#[tauri::command]
async fn list_crds() -> Result<Vec<telescope_engine::crd::CrdInfo>, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
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

fn main() {
    tracing_subscriber::fmt::init();
    eprintln!("[telescope] Starting Telescope desktop app");

    let data_dir = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(|h| std::path::PathBuf::from(h).join(".telescope"))
        .unwrap_or_else(|_| std::env::temp_dir().join("telescope"));
    let db_path = data_dir.join("resources.db");
    eprintln!("[telescope] DB path: {:?}", db_path);
    // safe: path always has a parent after join()
    std::fs::create_dir_all(db_path.parent().unwrap()).expect("Failed to create data directory");

    let db_path_str = db_path.to_string_lossy().to_string();
    let store = ResourceStore::open(&db_path_str).expect("Failed to initialize resource store");

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

    let app_state = AppState {
        db_path: db_path_str,
        audit_log_path: audit_path,
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
            get_pods,
            get_resources,
            get_events,
            get_resource_counts,
            search_resources,
            count_resources,
            get_resource,
            list_namespaces,
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
            rollout_restart,
            rollout_status,
            exec_command,
            get_pod_metrics,
            check_metrics_available,
            get_node_metrics,
            list_crds,
            get_preference,
            set_preference,
        ])
        .setup(|_app| {
            eprintln!("[telescope] Tauri setup complete, window should be loading frontend");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
