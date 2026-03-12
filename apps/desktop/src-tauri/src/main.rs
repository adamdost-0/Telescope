#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};

use futures::{AsyncBufReadExt, TryStreamExt};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{Mutex as TokioMutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{error, info};

use telescope_core::{ConnectionState, ResourceEntry, ResourceStore};
use telescope_engine::ClusterContext;

/// Application state managed by Tauri.
///
/// `ResourceStore` stays in `std::sync::Mutex` because `rusqlite::Connection`
/// is `Send` but not `Sync`. The same `Arc<Mutex<ResourceStore>>` is shared
/// with the `ResourceWatcher` background task.
struct AppState {
    #[allow(dead_code)]
    db_path: String,
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
    let gvks = vec![
        "v1/Pod",
        "apps/v1/Deployment",
        "apps/v1/StatefulSet",
        "apps/v1/DaemonSet",
        "batch/v1/Job",
        "batch/v1/CronJob",
        "v1/Service",
        "v1/ConfigMap",
        "v1/Secret",
        "v1/Event",
        "v1/Node",
    ];
    let counts: Vec<(String, u64)> = gvks
        .iter()
        .map(|gvk| {
            let count = store.count(gvk, None).unwrap_or(0);
            (gvk.to_string(), count)
        })
        .collect();
    Ok(counts)
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
        let _ = store.delete_all_by_gvk("v1/Pod");
        let _ = store.delete_all_by_gvk("v1/Event");
        let _ = store.delete_all_by_gvk("v1/Node");
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

    info!(
        "Watch started for context={}, namespace={}",
        context_name, namespace
    );
    Ok(())
}

/// Disconnect from the current cluster.
#[tauri::command]
async fn disconnect(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    info!("Disconnecting");

    abort_watch(&state).await;

    // Clear stored data.
    {
        let store = state
            .store
            .lock()
            .map_err(|e| format!("Store lock failed: {}", e))?;
        let _ = store.delete_all_by_gvk("v1/Pod");
        let _ = store.delete_all_by_gvk("v1/Event");
        let _ = store.delete_all_by_gvk("v1/Node");
    }

    // Reset state.
    {
        let mut ctx = state.active_context.write().await;
        *ctx = None;
    }

    set_connection_state(&app, &state.connection_state, ConnectionState::Disconnected).await;

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

        // Clear old pod and event data.
        {
            let store = state
                .store
                .lock()
                .map_err(|e| format!("Store lock failed: {}", e))?;
            let _ = store.delete_all_by_gvk("v1/Pod");
            let _ = store.delete_all_by_gvk("v1/Event");
            let _ = store.delete_all_by_gvk("v1/Node");
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
    gvk: String,
    namespace: String,
    name: String,
    replicas: i32,
) -> Result<String, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    telescope_engine::actions::scale_resource(&client, &gvk, &namespace, &name, replicas)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a namespaced Kubernetes resource by GVK, namespace, and name.
#[tauri::command]
async fn delete_resource(gvk: String, namespace: String, name: String) -> Result<String, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    let result = telescope_engine::actions::delete_resource(&client, &gvk, &namespace, &name)
        .await
        .map_err(|e| e.to_string())?;
    if result.success {
        Ok(result.message)
    } else {
        Err(result.message)
    }
}

/// Apply a resource from JSON/YAML content using server-side apply.
#[tauri::command]
async fn apply_resource(
    json_content: String,
    dry_run: bool,
) -> Result<telescope_engine::actions::ApplyResult, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    telescope_engine::actions::apply_resource(&client, &json_content, dry_run)
        .await
        .map_err(|e| e.to_string())
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

    // Spawn the events watcher (secondary — no state machine transitions).
    let events_watcher = watcher.clone();
    let ns_events = ns.clone();
    let events_task = tokio::spawn(async move {
        if let Err(e) = events_watcher.watch_events(&ns_events).await {
            error!("Events watch error: {}", e);
        }
    });

    // Spawn the nodes watcher (cluster-scoped — no namespace needed).
    let nodes_watcher = watcher.clone();
    let nodes_task = tokio::spawn(async move {
        if let Err(e) = nodes_watcher.watch_nodes().await {
            error!("Node watch error: {}", e);
        }
    });

    // Spawn the main watch task (pods + state machine).
    let task = tokio::spawn(async move {
        if let Err(e) = watcher.watch_pods(&ns).await {
            error!("Watch error: {}", e);
        }
        // When the pod watch ends, abort the state forwarder and other watchers.
        state_forwarder.abort();
        events_task.abort();
        nodes_task.abort();
    });

    let mut handle = state.watch_handle.lock().await;
    *handle = Some(task);
}

/// Restart a Deployment rollout.
#[tauri::command]
async fn rollout_restart(namespace: String, name: String) -> Result<String, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    telescope_engine::actions::rollout_restart(&client, &namespace, &name)
        .await
        .map_err(|e| e.to_string())
}

/// Get rollout status for a Deployment.
#[tauri::command]
async fn rollout_status(
    namespace: String,
    name: String,
) -> Result<telescope_engine::actions::RolloutStatus, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    telescope_engine::actions::rollout_status(&client, &namespace, &name)
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
    namespace: String,
    pod: String,
    container: Option<String>,
    command: Vec<String>,
) -> Result<telescope_engine::exec::ExecResult, String> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| e.to_string())?;
    let req = telescope_engine::exec::ExecRequest {
        namespace,
        pod,
        container,
        command,
    };
    telescope_engine::exec::exec_command(&client, &req)
        .await
        .map_err(|e| e.to_string())
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

fn main() {
    tracing_subscriber::fmt::init();
    eprintln!("[telescope] Starting Telescope desktop app");

    let data_dir = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(|h| std::path::PathBuf::from(h).join(".telescope"))
        .unwrap_or_else(|_| std::env::temp_dir().join("telescope"));
    let db_path = data_dir.join("resources.db");
    eprintln!("[telescope] DB path: {:?}", db_path);
    std::fs::create_dir_all(db_path.parent().unwrap()).expect("Failed to create data directory");

    let db_path_str = db_path.to_string_lossy().to_string();
    let store = ResourceStore::open(&db_path_str).expect("Failed to initialize resource store");

    // Clear stale data from previous runs.
    let _ = store.delete_all_by_gvk("v1/Pod");
    let _ = store.delete_all_by_gvk("v1/Event");
    let _ = store.delete_all_by_gvk("v1/Node");

    let app_state = AppState {
        db_path: db_path_str,
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
            count_resources,
            get_resource,
            list_namespaces,
            list_helm_releases,
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
        ])
        .setup(|_app| {
            eprintln!("[telescope] Tauri setup complete, window should be loading frontend");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
