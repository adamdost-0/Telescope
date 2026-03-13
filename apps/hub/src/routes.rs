use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use telescope_core::{ConnectionState, ResourceEntry};
use telescope_engine::actions::{
    apply_resource as engine_apply_resource, delete_resource as engine_delete_resource,
    rollout_restart as engine_rollout_restart, rollout_status as engine_rollout_status,
    scale_resource as engine_scale_resource, ApplyResult, DeleteResult, RolloutStatus,
};
use telescope_engine::audit::AuditEntry;
use telescope_engine::exec::{
    exec_command as engine_exec_command, ExecRequest as EngineExecRequest, ExecResult,
};
use telescope_engine::helm::{
    get_release_history as engine_get_release_history,
    get_release_values as engine_get_release_values,
    redact_sensitive_values as engine_redact_sensitive_values,
    rollback_release as engine_rollback_release, HelmRelease,
};
use telescope_engine::kubeconfig::active_context as engine_active_context;
use telescope_engine::metrics::{
    get_node_metrics as engine_get_node_metrics,
    is_metrics_available as engine_is_metrics_available, NodeMetricsData, PodMetrics,
};
use telescope_engine::portforward::{
    start_port_forward as engine_start_port_forward, PortForwardRequest as EnginePortForwardRequest,
};

use crate::auth::AuthUser;
use crate::state::{clear_all_resources, HubState, ALL_WATCHED_GVKS};

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

#[derive(Deserialize)]
pub struct AuditQuery {
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct DeleteRequest {
    pub gvk: String,
    pub namespace: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct ApplyRequest {
    #[serde(alias = "json_content")]
    pub manifest: String,
    #[serde(default, alias = "dryRun")]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct ScaleRequest {
    pub gvk: String,
    pub namespace: String,
    pub name: String,
    pub replicas: i32,
}

#[derive(Deserialize)]
pub struct RolloutStatusQuery {
    pub gvk: String,
    pub namespace: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct ExecCommandRequest {
    pub namespace: String,
    pub pod: String,
    pub container: Option<String>,
    pub command: Vec<String>,
}

#[derive(Deserialize)]
pub struct PortForwardRequest {
    pub namespace: String,
    pub pod: String,
    #[serde(alias = "localPort")]
    pub local_port: u16,
    #[serde(alias = "remotePort")]
    pub remote_port: u16,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct PodLogStreamQuery {
    pub container: Option<String>,
    #[serde(alias = "tailLines")]
    pub tail_lines: Option<i64>,
}

#[derive(Deserialize)]
pub struct HelmValuesQuery {
    pub reveal: Option<bool>,
}

#[derive(Deserialize)]
pub struct RollbackRequest {
    pub revision: i32,
}

#[derive(Deserialize)]
pub struct NamespaceRequest {
    pub namespace: String,
}

#[derive(Deserialize)]
pub struct PreferenceValueRequest {
    pub value: String,
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Serialize)]
pub struct PortForwardResponse {
    pub namespace: String,
    pub pod: String,
    pub local_port: u16,
    pub remote_port: u16,
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
    Extension(user): Extension<AuthUser>,
    Json(body): Json<ConnectRequest>,
) -> ApiResult<serde_json::Value> {
    info!(actor = %user.email, "Connecting to context: {}", body.context_name);

    // Check cluster access.
    if !crate::auth::user_can_access_cluster(&user, &body.context_name) {
        return Err(api_err(
            axum::http::StatusCode::FORBIDDEN,
            "Access denied for this cluster",
        ));
    }

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

    // Build a kube client for the requested context, impersonating the
    // authenticated user when OIDC is enabled.
    let client = telescope_engine::client::create_client_for_context_as_user(
        &body.context_name,
        &user.email,
        &user.groups,
    )
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
            actor: user.email.clone(),
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

pub async fn disconnect(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
) -> ApiResult<serde_json::Value> {
    let ctx_name = state
        .active_context
        .read()
        .await
        .clone()
        .unwrap_or_default();
    info!(actor = %user.email, "Disconnecting");

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
            actor: user.email.clone(),
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

pub async fn list_namespaces(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
) -> ApiResult<Vec<String>> {
    let client = active_client_for_user(&state, &user).await?;
    telescope_engine::namespace::list_namespaces(&client)
        .await
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ---------------------------------------------------------------------------
// GET /api/v1/secrets?namespace=...
// ---------------------------------------------------------------------------

pub async fn list_secrets(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Query(params): Query<NamespaceQuery>,
) -> ApiResult<Vec<ResourceEntry>> {
    let namespace = match params.namespace.filter(|ns| !ns.is_empty()) {
        Some(namespace) => namespace,
        None => state.active_namespace.read().await.clone(),
    };
    let client = active_client_for_user(&state, &user).await?;
    let result = telescope_engine::secrets::list_secrets(&client, &namespace).await;
    let ctx_name = state
        .active_context
        .read()
        .await
        .clone()
        .unwrap_or_default();

    telescope_engine::audit::log_audit(
        &state.audit_log_path,
        &AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            actor: user.email.clone(),
            context: ctx_name,
            namespace: namespace.clone(),
            action: "list_secrets".into(),
            resource_type: "v1/Secret".into(),
            resource_name: String::new(),
            result: if result.is_ok() {
                "success".into()
            } else {
                "failure".into()
            },
            detail: result.as_ref().err().map(|e| e.to_string()),
        },
    );

    result
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ---------------------------------------------------------------------------
// GET /api/v1/secrets/:namespace/:name
// ---------------------------------------------------------------------------

pub async fn get_secret(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Path((namespace, name)): Path<(String, String)>,
) -> ApiResult<Option<ResourceEntry>> {
    let client = active_client_for_user(&state, &user).await?;
    let result = telescope_engine::secrets::get_secret(&client, &namespace, &name).await;
    let ctx_name = state
        .active_context
        .read()
        .await
        .clone()
        .unwrap_or_default();

    telescope_engine::audit::log_audit(
        &state.audit_log_path,
        &AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            actor: user.email.clone(),
            context: ctx_name,
            namespace: namespace.clone(),
            action: "get_secret".into(),
            resource_type: "v1/Secret".into(),
            resource_name: name.clone(),
            result: if result.is_ok() {
                "success".into()
            } else {
                "failure".into()
            },
            detail: result.as_ref().err().map(|e| e.to_string()),
        },
    );

    result
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ---------------------------------------------------------------------------
// GET /api/v1/pods/:namespace/:name/logs?container=...&tail=...&previous=...
// ---------------------------------------------------------------------------

pub async fn get_pod_logs(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<PodLogQuery>,
) -> ApiResult<serde_json::Value> {
    let client = active_client_for_user(&state, &user).await?;

    let req = telescope_engine::logs::LogRequest {
        namespace: namespace.clone(),
        pod: name.clone(),
        container: params.container,
        previous: params.previous.unwrap_or(false),
        tail_lines: params.tail.or(Some(1000)),
        follow: false,
    };

    let ctx_name = state
        .active_context
        .read()
        .await
        .clone()
        .unwrap_or_default();

    let result = telescope_engine::logs::get_pod_logs(&client, &req).await;

    telescope_engine::audit::log_audit(
        &state.audit_log_path,
        &AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            actor: user.email.clone(),
            context: ctx_name,
            namespace,
            action: "get_logs".into(),
            resource_type: "v1/Pod".into(),
            resource_name: name,
            result: if result.is_ok() {
                "success".into()
            } else {
                "failure".into()
            },
            detail: result.as_ref().err().map(|e| e.to_string()),
        },
    );

    let logs = result
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
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Query(params): Query<NamespaceQuery>,
) -> ApiResult<Vec<telescope_engine::helm::HelmRelease>> {
    let client = active_client_for_user(&state, &user).await?;
    telescope_engine::helm::list_releases(&client, params.namespace.as_deref())
        .await
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ---------------------------------------------------------------------------
// GET /api/v1/metrics/pods?namespace=...
// ---------------------------------------------------------------------------

pub async fn pod_metrics(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Query(params): Query<NamespaceQuery>,
) -> ApiResult<Vec<PodMetrics>> {
    let client = active_client_for_user(&state, &user).await?;
    telescope_engine::metrics::get_pod_metrics(&client, params.namespace.as_deref())
        .await
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ---------------------------------------------------------------------------
// GET /api/v1/crds
// ---------------------------------------------------------------------------

pub async fn list_crds(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
) -> ApiResult<Vec<telescope_engine::crd::CrdInfo>> {
    let client = active_client_for_user(&state, &user).await?;
    telescope_engine::crd::list_crds(&client)
        .await
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ---------------------------------------------------------------------------
// GET /api/v1/audit?limit=100
// ---------------------------------------------------------------------------

pub async fn get_audit_log(
    State(state): State<Arc<HubState>>,
    Query(params): Query<AuditQuery>,
) -> Json<Vec<serde_json::Value>> {
    let limit = params.limit.unwrap_or(100);
    let content = std::fs::read_to_string(&state.audit_log_path).unwrap_or_default();
    let entries: Vec<serde_json::Value> = content
        .lines()
        .rev()
        .take(limit)
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    Json(entries)
}

// ---------------------------------------------------------------------------
// Resource actions
// ---------------------------------------------------------------------------

pub async fn delete_resource(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Json(body): Json<DeleteRequest>,
) -> ApiResult<DeleteResult> {
    let client = active_client_for_user(&state, &user).await?;
    let result = engine_delete_resource(&client, &body.gvk, &body.namespace, &body.name).await;

    audit_action(
        &state,
        &user,
        body.namespace.clone(),
        "delete",
        body.gvk.clone(),
        body.name.clone(),
        result
            .as_ref()
            .is_ok_and(|delete_result| delete_result.success),
        None,
    )
    .await;

    result
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub async fn apply_resource(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Json(body): Json<ApplyRequest>,
) -> ApiResult<ApplyResult> {
    let client = active_client_for_user(&state, &user).await?;
    let result = engine_apply_resource(&client, &body.manifest, body.dry_run).await;

    audit_action(
        &state,
        &user,
        String::new(),
        "apply",
        "resource".to_string(),
        String::new(),
        result
            .as_ref()
            .is_ok_and(|apply_result| apply_result.success),
        Some(format!("dry_run={}", body.dry_run)),
    )
    .await;

    result
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub async fn scale_resource(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Json(body): Json<ScaleRequest>,
) -> ApiResult<MessageResponse> {
    let client = active_client_for_user(&state, &user).await?;
    let result = engine_scale_resource(
        &client,
        &body.gvk,
        &body.namespace,
        &body.name,
        body.replicas,
    )
    .await;

    audit_action(
        &state,
        &user,
        body.namespace.clone(),
        "scale",
        body.gvk.clone(),
        body.name.clone(),
        result.is_ok(),
        Some(format!("replicas={}", body.replicas)),
    )
    .await;

    result
        .map(|message| Json(MessageResponse { message }))
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub async fn rollout_restart(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Json(body): Json<DeleteRequest>,
) -> ApiResult<MessageResponse> {
    let client = active_client_for_user(&state, &user).await?;
    let result = engine_rollout_restart(&client, &body.gvk, &body.namespace, &body.name).await;

    audit_action(
        &state,
        &user,
        body.namespace.clone(),
        "rollout_restart",
        body.gvk.clone(),
        body.name.clone(),
        result.is_ok(),
        None,
    )
    .await;

    result
        .map(|message| Json(MessageResponse { message }))
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub async fn rollout_status(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Query(params): Query<RolloutStatusQuery>,
) -> ApiResult<RolloutStatus> {
    let client = active_client_for_user(&state, &user).await?;
    engine_rollout_status(&client, &params.gvk, &params.namespace, &params.name)
        .await
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ---------------------------------------------------------------------------
// Ops routes
// ---------------------------------------------------------------------------

pub async fn exec_command(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Json(body): Json<ExecCommandRequest>,
) -> ApiResult<ExecResult> {
    let client = active_client_for_user(&state, &user).await?;
    let request = EngineExecRequest {
        namespace: body.namespace.clone(),
        pod: body.pod.clone(),
        container: body.container,
        command: body.command,
    };
    let command_detail = request.command.join(" ");
    let result = engine_exec_command(&client, &request).await;

    audit_action(
        &state,
        &user,
        request.namespace.clone(),
        "exec",
        "Pod".to_string(),
        request.pod.clone(),
        result.is_ok(),
        Some(command_detail),
    )
    .await;

    result
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub async fn start_port_forward(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Json(body): Json<PortForwardRequest>,
) -> ApiResult<PortForwardResponse> {
    let client = active_client_for_user(&state, &user).await?;
    let request = EnginePortForwardRequest {
        namespace: body.namespace.clone(),
        pod: body.pod.clone(),
        local_port: body.local_port,
        remote_port: body.remote_port,
    };
    let result = engine_start_port_forward(&client, &request).await;

    audit_action(
        &state,
        &user,
        request.namespace.clone(),
        "port_forward",
        "Pod".to_string(),
        request.pod.clone(),
        result.is_ok(),
        Some(format!(
            "local_port={},remote_port={}",
            request.local_port, request.remote_port
        )),
    )
    .await;

    result
        .map(|local_port| {
            Json(PortForwardResponse {
                namespace: request.namespace,
                pod: request.pod,
                local_port,
                remote_port: request.remote_port,
            })
        })
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub async fn list_containers(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Path((namespace, pod)): Path<(String, String)>,
) -> ApiResult<Vec<String>> {
    let client = active_client_for_user(&state, &user).await?;
    telescope_engine::logs::list_containers(&client, &namespace, &pod)
        .await
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub async fn stream_pod_logs(
    Extension(_user): Extension<AuthUser>,
    Path((_namespace, _pod)): Path<(String, String)>,
    Query(_params): Query<PodLogStreamQuery>,
) -> ApiResult<serde_json::Value> {
    // TODO: Hub log streaming should use SSE or WebSocket forwarding so browser mode
    // can match the Tauri `log-chunk` event flow. For now, use the snapshot logs route.
    Err(api_err(
        axum::http::StatusCode::NOT_IMPLEMENTED,
        "Hub log streaming is not implemented yet; use the snapshot logs endpoint",
    ))
}

// ---------------------------------------------------------------------------
// Helm routes
// ---------------------------------------------------------------------------

pub async fn get_helm_release_history(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Path((namespace, name)): Path<(String, String)>,
) -> ApiResult<Vec<HelmRelease>> {
    let client = active_client_for_user(&state, &user).await?;
    engine_get_release_history(&client, &namespace, &name)
        .await
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub async fn get_helm_release_values(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Path((namespace, name)): Path<(String, String)>,
    Query(params): Query<HelmValuesQuery>,
) -> ApiResult<String> {
    let client = active_client_for_user(&state, &user).await?;
    let mut values = engine_get_release_values(&client, &namespace, &name)
        .await
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !params.reveal.unwrap_or(false) && !values.trim_start().starts_with('#') {
        let mut json = serde_yaml::from_str::<serde_json::Value>(&values).map_err(|e| {
            api_err(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse Helm values YAML: {e}"),
            )
        })?;
        engine_redact_sensitive_values(&mut json);
        values = serde_yaml::to_string(&json).map_err(|e| {
            api_err(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize redacted Helm values YAML: {e}"),
            )
        })?;
    }

    Ok(Json(values))
}

pub async fn rollback_helm_release(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Path((namespace, name)): Path<(String, String)>,
    Json(body): Json<RollbackRequest>,
) -> ApiResult<MessageResponse> {
    let result = engine_rollback_release(&namespace, &name, body.revision).await;

    audit_action(
        &state,
        &user,
        namespace.clone(),
        "helm_rollback",
        "HelmRelease".to_string(),
        name.clone(),
        result.is_ok(),
        Some(format!("revision={}", body.revision)),
    )
    .await;

    result
        .map(|message| Json(MessageResponse { message }))
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ---------------------------------------------------------------------------
// Misc routes
// ---------------------------------------------------------------------------

pub async fn active_context(State(state): State<Arc<HubState>>) -> ApiResult<Option<String>> {
    if let Some(context) = state.active_context.read().await.clone() {
        return Ok(Json(Some(context)));
    }

    match engine_active_context() {
        Ok(context) => Ok(Json(Some(context))),
        Err(telescope_engine::EngineError::NoActiveContext) => Ok(Json(None)),
        Err(e) => Err(api_err(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )),
    }
}

pub async fn get_namespace(State(state): State<Arc<HubState>>) -> Json<String> {
    Json(state.active_namespace.read().await.clone())
}

pub async fn set_namespace(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Json(body): Json<NamespaceRequest>,
) -> ApiResult<serde_json::Value> {
    info!(actor = %user.email, namespace = %body.namespace, "Switching namespace");

    let client = if state.active_context.read().await.is_some() {
        Some(active_client_for_user(&state, &user).await?)
    } else {
        None
    };

    {
        let mut namespace = state.active_namespace.write().await;
        *namespace = body.namespace.clone();
    }

    if let Some(client) = client {
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

        set_connection_state(&state.connection_state, ConnectionState::Connecting).await;
        spawn_watch_task(&state, client, &body.namespace).await;
    }

    audit_action(
        &state,
        &user,
        body.namespace.clone(),
        "set_namespace",
        "namespace".to_string(),
        body.namespace.clone(),
        true,
        None,
    )
    .await;

    Ok(Json(serde_json::json!({ "namespace": body.namespace })))
}

pub async fn get_resource_counts(
    State(state): State<Arc<HubState>>,
) -> ApiResult<Vec<(String, u64)>> {
    let store = state.store.lock().map_err(|e| {
        api_err(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Lock failed: {e}"),
        )
    })?;
    let counts = ALL_WATCHED_GVKS
        .iter()
        .map(|gvk| (gvk.to_string(), store.count(gvk, None).unwrap_or(0)))
        .collect();
    Ok(Json(counts))
}

pub async fn node_metrics(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
) -> ApiResult<Vec<NodeMetricsData>> {
    let client = active_client_for_user(&state, &user).await?;
    engine_get_node_metrics(&client)
        .await
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub async fn metrics_available(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
) -> ApiResult<bool> {
    let client = active_client_for_user(&state, &user).await?;
    Ok(Json(engine_is_metrics_available(&client).await))
}

pub async fn get_preference(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Path(key): Path<String>,
) -> ApiResult<Option<String>> {
    let store = state.store.lock().map_err(|e| {
        api_err(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Store lock failed: {e}"),
        )
    })?;
    let scoped_key = preference_storage_key(&user, &key);
    store
        .get_preference(&scoped_key)
        .map(Json)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub async fn set_preference(
    State(state): State<Arc<HubState>>,
    Extension(user): Extension<AuthUser>,
    Path(key): Path<String>,
    Json(body): Json<PreferenceValueRequest>,
) -> ApiResult<serde_json::Value> {
    let store = state.store.lock().map_err(|e| {
        api_err(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Store lock failed: {e}"),
        )
    })?;
    let scoped_key = preference_storage_key(&user, &key);
    store
        .set_preference(&scoped_key, &body.value)
        .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "status": "ok" })))
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

async fn active_client_for_user(
    state: &HubState,
    user: &AuthUser,
) -> Result<kube::Client, (axum::http::StatusCode, Json<ErrorResponse>)> {
    let context_name = state
        .active_context
        .read()
        .await
        .clone()
        .ok_or_else(|| api_err(axum::http::StatusCode::BAD_REQUEST, "Not connected"))?;

    telescope_engine::client::create_client_for_context_as_user(
        &context_name,
        &user.email,
        &user.groups,
    )
    .await
    .map_err(|e| api_err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[allow(clippy::too_many_arguments)]
async fn audit_action(
    state: &HubState,
    user: &AuthUser,
    namespace: String,
    action: &str,
    resource_type: String,
    resource_name: String,
    success: bool,
    detail: Option<String>,
) {
    telescope_engine::audit::log_audit(
        &state.audit_log_path,
        &AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            actor: user.email.clone(),
            context: state
                .active_context
                .read()
                .await
                .clone()
                .unwrap_or_default(),
            namespace,
            action: action.to_string(),
            resource_type,
            resource_name,
            result: if success {
                "success".into()
            } else {
                "failure".into()
            },
            detail,
        },
    );
}

fn preference_storage_key(user: &AuthUser, key: &str) -> String {
    format!("{}:{key}", user.email)
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
