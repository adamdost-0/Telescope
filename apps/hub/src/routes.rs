use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use tracing::{error, info};

use telescope_core::{ConnectionState, ResourceEntry};
use telescope_engine::audit::AuditEntry;

use crate::state::{clear_all_resources, HubState};

// ---------------------------------------------------------------------------
// Query / request types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ConnectRequest {
    #[serde(alias = "contextName")]
    pub context_name: String,
}

#[derive(Deserialize)]
pub struct ResourceQuery {
    pub gvk: String,
    pub namespace: Option<String>,
}

#[derive(Deserialize)]
pub struct NamespaceQuery {
    pub namespace: Option<String>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Deserialize)]
pub struct PodLogQuery {
    pub container: Option<String>,
    pub tail: Option<i64>,
    pub previous: Option<bool>,
}

#[derive(Deserialize)]
pub struct EventQuery {
    pub namespace: Option<String>,
    pub involved_object: Option<String>,
}

// ---------------------------------------------------------------------------
// Response helpers
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

type ApiResult<T> = Result<Json<T>, (axum::http::StatusCode, Json<ErrorResponse>)>;

fn api_err(
    status: axum::http::StatusCode,
    msg: impl Into<String>,
) -> (axum::http::StatusCode, Json<ErrorResponse>) {
    (status, Json(ErrorResponse { error: msg.into() }))
}

// ---------------------------------------------------------------------------
// GET /api/v1/contexts
// ---------------------------------------------------------------------------

pub async fn list_contexts() -> ApiResult<Vec<telescope_engine::ClusterContext>> {
    match telescope_engine::kubeconfig::list_contexts() {
        Ok(contexts) => Ok(Json(contexts)),
        Err(e) => {
            error!("list_contexts error: {}", e);
            Ok(Json(vec![]))
        }
    }
}

// ---------------------------------------------------------------------------
// POST /api/v1/connect
// ---------------------------------------------------------------------------

pub async fn connect(
    State(state): State<Arc<HubState>>,
    Json(body): Json<ConnectRequest>,
) -> ApiResult<serde_json::Value> {
    info!("Connecting to context: {}", body.context_name);

    // Abort any existing watch task.
    abort_watch(&state).await;

    // Clear previous data.
    {
        let store = state.store.lock().map_err(|e| {
            api_err(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Store lock failed: {e}"),
            )
        })?;
        clear_all_resources(&store);
    }

    // Update connection state to Connecting.
    set_connection_state(&state.connection_state, ConnectionState::Connecting).await;

    // Build a kube client for the requested context.
    let client = telescope_engine::client::create_client_for_context(&body.context_name)
        .await
        .map_err(|e| {
            let msg = format!("Failed to connect: {}", e);
            error!("{}", msg);
            api_err(axum::http::StatusCode::BAD_REQUEST, msg)
        })?;

    // Update active context and read namespace.
    {
        let mut ctx = state.active_context.write().await;
        *ctx = Some(body.context_name.clone());
    }
    let namespace = state.active_namespace.read().await.clone();

    // Spawn the watcher background task.
    spawn_watch_task(&state, client, &namespace).await;

    telescope_engine::audit::log_audit(
        &state.audit_log_path,
        &AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            context: body.context_name.clone(),
            namespace: namespace.clone(),
            action: "connect".into(),
            resource_type: "context".into(),
            resource_name: body.context_name.clone(),
            result: "success".into(),
            detail: None,
        },
    );

    info!(
        "Watch started for context={}, namespace={}",
        body.context_name, namespace
    );
    Ok(Json(serde_json::json!({
        "status": "connected",
        "context": body.context_name,
        "namespace": namespace
    })))
}

// ---------------------------------------------------------------------------
// POST /api/v1/disconnect
// ---------------------------------------------------------------------------

pub async fn disconnect(State(state): State<Arc<HubState>>) -> ApiResult<serde_json::Value> {
    let ctx_name = state
        .active_context
        .read()
        .await
        .clone()
        .unwrap_or_default();
    info!("Disconnecting");

    abort_watch(&state).await;

    {
        let store = state.store.lock().map_err(|e| {
            api_err(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Store lock failed: {e}"),
            )
        })?;
        clear_all_resources(&store);
    }

    {
        let mut ctx = state.active_context.write().await;
        *ctx = None;
    }

    set_connection_state(&state.connection_state, ConnectionState::Disconnected).await;

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

    Ok(Json(serde_json::json!({ "status": "disconnected" })))
}

// ---------------------------------------------------------------------------
// GET /api/v1/connection-state
// ---------------------------------------------------------------------------

pub async fn connection_state(State(state): State<Arc<HubState>>) -> Json<ConnectionState> {
    Json(state.connection_state.read().await.clone())
}

// ---------------------------------------------------------------------------
// GET /api/v1/resources?gvk=...&namespace=...
// ---------------------------------------------------------------------------

pub async fn get_resources(
    State(state): State<Arc<HubState>>,
    Query(params): Query<ResourceQuery>,
) -> ApiResult<Vec<ResourceEntry>> {
    let store = state.store.lock().map_err(|e| {
        api_err(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Store lock failed: {e}"),
        )
    })?;
    match store.list(&params.gvk, params.namespace.as_deref()) {
        Ok(entries) => Ok(Json(entries)),
        Err(e) => {
            error!("get_resources error: {}", e);
            Ok(Json(vec![]))
        }
    }
}

// ---------------------------------------------------------------------------
// GET /api/v1/pods?namespace=...
// ---------------------------------------------------------------------------

pub async fn get_pods(
    State(state): State<Arc<HubState>>,
    Query(params): Query<NamespaceQuery>,
) -> ApiResult<Vec<ResourceEntry>> {
    let store = state.store.lock().map_err(|e| {
        api_err(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Store lock failed: {e}"),
        )
    })?;
    match store.list("v1/Pod", params.namespace.as_deref()) {
        Ok(entries) => Ok(Json(entries)),
        Err(e) => {
            error!("get_pods error: {}", e);
            Ok(Json(vec![]))
        }
    }
}

// ---------------------------------------------------------------------------
// GET /api/v1/events?namespace=...&involved_object=...
// ---------------------------------------------------------------------------

pub async fn get_events(
    State(state): State<Arc<HubState>>,
    Query(params): Query<EventQuery>,
) -> ApiResult<Vec<ResourceEntry>> {
    let store = state.store.lock().map_err(|e| {
        api_err(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Store lock failed: {e}"),
        )
    })?;
    let events = store
        .list("v1/Event", params.namespace.as_deref())
        .unwrap_or_default();

    if let Some(obj_name) = params.involved_object {
        let needle = format!("\"name\":\"{}\"", obj_name);
        Ok(Json(
            events
                .into_iter()
                .filter(|entry| entry.content.contains(&needle))
                .collect(),
        ))
    } else {
        Ok(Json(events))
    }
}

// ---------------------------------------------------------------------------
// GET /api/v1/namespaces
// ---------------------------------------------------------------------------

pub async fn list_namespaces(State(state): State<Arc<HubState>>) -> ApiResult<Vec<String>> {
    let ctx = state.active_context.read().await.clone();
    if ctx.is_none() {
        return Ok(Json(vec!["default".to_string()]));
    }
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    telescope_engine::namespace::list_namespaces(&client)
        .await
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ---------------------------------------------------------------------------
// GET /api/v1/pods/:namespace/:name/logs?container=...&tail=...&previous=...
// ---------------------------------------------------------------------------

pub async fn get_pod_logs(
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<PodLogQuery>,
) -> ApiResult<serde_json::Value> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let req = telescope_engine::logs::LogRequest {
        namespace,
        pod: name,
        container: params.container,
        previous: params.previous.unwrap_or(false),
        tail_lines: params.tail.or(Some(1000)),
        follow: false,
    };

    let logs = telescope_engine::logs::get_pod_logs(&client, &req)
        .await
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({ "logs": logs })))
}

// ---------------------------------------------------------------------------
// GET /api/v1/cluster-info
// ---------------------------------------------------------------------------

pub async fn cluster_info(
    State(state): State<Arc<HubState>>,
) -> ApiResult<telescope_engine::ClusterInfo> {
    let ctx = state.active_context.read().await.clone();
    let context_name =
        ctx.ok_or_else(|| api_err(axum::http::StatusCode::BAD_REQUEST, "Not connected"))?;
    let client = telescope_engine::client::create_client_for_context(&context_name)
        .await
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    telescope_engine::client::get_cluster_info(&client, &context_name)
        .await
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ---------------------------------------------------------------------------
// GET /api/v1/search?q=...
// ---------------------------------------------------------------------------

pub async fn search(
    State(state): State<Arc<HubState>>,
    Query(params): Query<SearchQuery>,
) -> ApiResult<Vec<ResourceEntry>> {
    let store = state.store.lock().map_err(|e| {
        api_err(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Store lock failed: {e}"),
        )
    })?;
    let mut results = Vec::new();
    let query_lower = params.q.to_lowercase();
    for gvk in crate::state::ALL_WATCHED_GVKS {
        if let Ok(entries) = store.list(gvk, None) {
            for entry in entries {
                if entry.name.to_lowercase().contains(&query_lower)
                    || entry.gvk.to_lowercase().contains(&query_lower)
                {
                    results.push(entry);
                    if results.len() >= 20 {
                        return Ok(Json(results));
                    }
                }
            }
        }
    }
    Ok(Json(results))
}

// ---------------------------------------------------------------------------
// GET /api/v1/helm/releases?namespace=...
// ---------------------------------------------------------------------------

pub async fn helm_releases(
    Query(params): Query<NamespaceQuery>,
) -> ApiResult<Vec<telescope_engine::helm::HelmRelease>> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    telescope_engine::helm::list_releases(&client, params.namespace.as_deref())
        .await
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ---------------------------------------------------------------------------
// GET /api/v1/metrics/pods?namespace=...
// ---------------------------------------------------------------------------

pub async fn pod_metrics(
    Query(params): Query<NamespaceQuery>,
) -> ApiResult<Vec<telescope_engine::metrics::PodMetrics>> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    telescope_engine::metrics::get_pod_metrics(&client, params.namespace.as_deref())
        .await
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ---------------------------------------------------------------------------
// GET /api/v1/crds
// ---------------------------------------------------------------------------

pub async fn list_crds() -> ApiResult<Vec<telescope_engine::crd::CrdInfo>> {
    let client = telescope_engine::client::create_client()
        .await
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    telescope_engine::crd::list_crds(&client)
        .await
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Abort the current watch task if one is running.
async fn abort_watch(state: &HubState) {
    let mut handle = state.watch_handle.lock().await;
    if let Some(h) = handle.take() {
        h.abort();
        info!("Previous watch task aborted");
    }
}

/// Update the shared connection state.
async fn set_connection_state(
    conn_state: &Arc<tokio::sync::RwLock<ConnectionState>>,
    new_state: ConnectionState,
) {
    let mut s = conn_state.write().await;
    *s = new_state;
}

/// Spawn background tasks that watch resources, forwarding state changes.
async fn spawn_watch_task(state: &HubState, client: kube::Client, namespace: &str) {
    let store = Arc::clone(&state.store);
    let conn_state = Arc::clone(&state.connection_state);
    let ns = namespace.to_string();

    let watcher = telescope_engine::ResourceWatcher::new(client, Arc::clone(&store));
    let mut state_rx = watcher.state_receiver();

    // Forward state changes from the watcher to the shared connection state.
    let conn_state_fwd = Arc::clone(&conn_state);
    let state_forwarder = tokio::spawn(async move {
        while state_rx.changed().await.is_ok() {
            let new_state = state_rx.borrow().clone();
            let mut s = conn_state_fwd.write().await;
            *s = new_state;
        }
    });

    let mut aux_tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    // Cluster-wide watchers
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

    // Namespaced resource watchers
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

    // Main watch task (pods + lifecycle coordinator).
    let task = tokio::spawn(async move {
        if let Err(e) = watcher.watch_pods(&ns).await {
            error!("Pod watch error: {}", e);
        }
        state_forwarder.abort();
        for t in aux_tasks {
            t.abort();
        }
    });

    let mut handle = state.watch_handle.lock().await;
    *handle = Some(task);
}
