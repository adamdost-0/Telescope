use axum::extract::ws::WebSocketUpgrade;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod auth;
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

    if let Some(oidc) = auth::OidcConfig::from_env() {
        tracing::info!(
            issuer = %oidc.issuer_url,
            client_id = %oidc.client_id,
            "OIDC authentication enabled"
        );
    } else {
        tracing::info!("OIDC authentication disabled — all requests are anonymous");
    }

    // API routes protected by auth middleware.
    let api_routes = Router::new()
        .route("/contexts", get(routes::list_contexts))
        .route("/connect", post(routes::connect))
        .route("/disconnect", post(routes::disconnect))
        .route("/connection-state", get(routes::connection_state))
        .route("/resources", get(routes::get_resources))
        .route("/pods", get(routes::get_pods))
        .route("/events", get(routes::get_events))
        .route("/namespaces", get(routes::list_namespaces))
        .route("/pods/{namespace}/{name}/logs", get(routes::get_pod_logs))
        .route("/cluster-info", get(routes::cluster_info))
        .route("/search", get(routes::search))
        .route("/helm/releases", get(routes::helm_releases))
        .route("/metrics/pods", get(routes::pod_metrics))
        .route("/crds", get(routes::list_crds))
        .route("/audit", get(routes::get_audit_log))
        .layer(axum::middleware::from_fn(auth::auth_middleware))
        .with_state(Arc::clone(&hub_state));

    let app = Router::new()
        // Health check (unauthenticated)
        .route("/healthz", get(|| async { "ok" }))
        // Auth routes (unauthenticated)
        .route("/auth/login", get(auth::login))
        .route("/auth/callback", get(auth::auth_callback))
        .route("/auth/logout", post(auth::logout))
        // /auth/me needs the auth middleware to populate AuthUser
        .route(
            "/auth/me",
            get(auth::me).layer(axum::middleware::from_fn(auth::auth_middleware)),
        )
        // WebSocket
        .route("/ws", get(ws_handler))
        // API routes
        .nest("/api/v1", api_routes)
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
