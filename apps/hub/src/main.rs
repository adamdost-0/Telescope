use axum::extract::ws::WebSocketUpgrade;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod routes;
mod state;
mod ws;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,telescope_engine=debug".into()),
        )
        .init();

    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".into());
    let db_path =
        std::env::var("DB_PATH").unwrap_or_else(|_| "/tmp/telescope-hub/resources.db".into());
    let audit_path =
        std::env::var("AUDIT_PATH").unwrap_or_else(|_| "/tmp/telescope-hub/audit.log".into());

    // Ensure parent directories exist.
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    if let Some(parent) = std::path::Path::new(&audit_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Touch the audit file and restrict permissions.
        if std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&audit_path)
            .is_ok()
        {
            std::fs::set_permissions(&audit_path, std::fs::Permissions::from_mode(0o600)).ok();
        }
    }

    let hub_state = Arc::new(state::HubState::new(&db_path, &audit_path));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o600)).ok();
    }

    let app = Router::new()
        // Health check
        .route("/healthz", get(|| async { "ok" }))
        // WebSocket
        .route("/ws", get(ws_handler))
        // Context / connection lifecycle
        .route("/api/v1/contexts", get(routes::list_contexts))
        .route("/api/v1/connect", post(routes::connect))
        .route("/api/v1/disconnect", post(routes::disconnect))
        .route("/api/v1/connection-state", get(routes::connection_state))
        // Resource queries
        .route("/api/v1/resources", get(routes::get_resources))
        .route("/api/v1/pods", get(routes::get_pods))
        .route("/api/v1/events", get(routes::get_events))
        .route("/api/v1/namespaces", get(routes::list_namespaces))
        .route(
            "/api/v1/pods/{namespace}/{name}/logs",
            get(routes::get_pod_logs),
        )
        .route("/api/v1/cluster-info", get(routes::cluster_info))
        .route("/api/v1/search", get(routes::search))
        // Helm
        .route("/api/v1/helm/releases", get(routes::helm_releases))
        // Metrics & CRDs
        .route("/api/v1/metrics/pods", get(routes::pod_metrics))
        .route("/api/v1/crds", get(routes::list_crds))
        // Middleware
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(hub_state);

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Telescope Hub listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(ws::handle_ws)
}
