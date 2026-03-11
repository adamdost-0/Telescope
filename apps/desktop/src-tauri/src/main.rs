#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};

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

/// List available Kubernetes contexts from kubeconfig.
#[tauri::command]
fn list_contexts() -> Result<Vec<ClusterContext>, String> {
    eprintln!("[telescope] list_contexts called");
    let result = telescope_engine::kubeconfig::list_contexts().map_err(|e| e.to_string());
    eprintln!("[telescope] list_contexts result: {:?}", result.as_ref().map(|v| v.len()));
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

        // Clear old pod data.
        {
            let store = state
                .store
                .lock()
                .map_err(|e| format!("Store lock failed: {}", e))?;
            let _ = store.delete_all_by_gvk("v1/Pod");
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

/// Spawn a background task that watches pods and forwards state changes.
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

    let watcher = telescope_engine::ResourceWatcher::new(client, store);
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

    // Spawn the main watch task.
    let task = tokio::spawn(async move {
        if let Err(e) = watcher.watch_pods(&ns).await {
            error!("Watch error: {}", e);
        }
        // When the watch ends, abort the state forwarder.
        state_forwarder.abort();
    });

    let mut handle = state.watch_handle.lock().await;
    *handle = Some(task);
}

// ---------------------------------------------------------------------------
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
            get_pods,
            count_resources,
            get_resource,
            list_namespaces,
            connect_to_context,
            disconnect,
            set_namespace,
            get_namespace,
        ])
        .setup(|_app| {
            eprintln!("[telescope] Tauri setup complete, window should be loading frontend");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
