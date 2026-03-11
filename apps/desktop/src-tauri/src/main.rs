#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use tauri::State;
use telescope_core::{ConnectionState, ResourceEntry, ResourceStore};
use telescope_engine::ClusterContext;

/// Application state managed by Tauri.
///
/// Both fields are wrapped in `Mutex` because Tauri requires managed state
/// to be `Send + Sync`, and `ResourceStore` contains a `rusqlite::Connection`
/// which is not `Sync`.
struct AppState {
    store: Mutex<ResourceStore>,
    connection_state: Mutex<ConnectionState>,
}

/// List available Kubernetes contexts from kubeconfig.
#[tauri::command]
fn list_contexts() -> Result<Vec<ClusterContext>, String> {
    telescope_engine::kubeconfig::list_contexts().map_err(|e| e.to_string())
}

/// Get the currently active kubeconfig context.
#[tauri::command]
fn active_context() -> Result<String, String> {
    telescope_engine::kubeconfig::active_context().map_err(|e| e.to_string())
}

/// Get the current connection state.
#[tauri::command]
fn get_connection_state(state: State<'_, AppState>) -> ConnectionState {
    state.connection_state.lock().unwrap().clone()
}

/// List pods in a namespace from the SQLite store.
#[tauri::command]
fn get_pods(
    state: State<'_, AppState>,
    namespace: Option<String>,
) -> Result<Vec<ResourceEntry>, String> {
    let store = state.store.lock().unwrap();
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
    let store = state.store.lock().unwrap();
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
    let store = state.store.lock().unwrap();
    store
        .get(&gvk, &namespace, &name)
        .map_err(|e| e.to_string())
}

fn main() {
    let store = ResourceStore::open(":memory:").expect("Failed to initialize resource store");

    let app_state = AppState {
        store: Mutex::new(store),
        connection_state: Mutex::new(ConnectionState::Disconnected),
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
