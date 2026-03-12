//! OIDC authentication scaffolding.
//!
//! When `OIDC_ENABLED=true` the middleware requires a `Bearer` JWT in the
//! `Authorization` header and extracts user claims.  When disabled (the
//! default) every request passes through as an anonymous user.
//!
//! Full Azure Entra ID integration (discovery, token validation, PKCE) is
//! out of scope for this scaffold — the login/callback routes return
//! `501 Not Implemented` until a real OIDC client is registered.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tracing::warn;

// ---------------------------------------------------------------------------
// OIDC configuration (from environment)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct OidcConfig {
    pub enabled: bool,
    /// e.g. `https://login.microsoftonline.com/{tenant}/v2.0`
    pub issuer_url: String,
    pub client_id: String,
    /// e.g. `http://localhost:3001/auth/callback`
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

impl OidcConfig {
    /// Build config from environment variables. Returns `None` when OIDC is
    /// not enabled or required variables are missing.
    pub fn from_env() -> Option<Self> {
        let enabled = std::env::var("OIDC_ENABLED").unwrap_or_default() == "true";
        if !enabled {
            return None;
        }
        Some(Self {
            enabled,
            issuer_url: std::env::var("OIDC_ISSUER_URL").ok()?,
            client_id: std::env::var("OIDC_CLIENT_ID").ok()?,
            redirect_uri: std::env::var("OIDC_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:3001/auth/callback".into()),
            scopes: vec!["openid".into(), "profile".into(), "email".into()],
        })
    }
}

// ---------------------------------------------------------------------------
// Authenticated user identity
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub email: String,
    pub name: String,
    pub groups: Vec<String>,
}

// ---------------------------------------------------------------------------
// Auth middleware
// ---------------------------------------------------------------------------

pub async fn auth_middleware(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let oidc_enabled = std::env::var("OIDC_ENABLED").unwrap_or_default() == "true";

    if !oidc_enabled {
        req.extensions_mut().insert(AuthUser {
            email: "anonymous@local".into(),
            name: "Anonymous".into(),
            groups: vec![],
        });
        return Ok(next.run(req).await);
    }

    // Extract Bearer token from Authorization header.
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..];

    // TODO(m7-100): Validate JWT signature, expiry, and audience once a real
    // OIDC provider is configured.  For now we only decode the payload claims.
    match decode_jwt_claims(token) {
        Some(user) => {
            req.extensions_mut().insert(user);
            Ok(next.run(req).await)
        }
        None => {
            warn!("Failed to decode JWT claims from Authorization header");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

// ---------------------------------------------------------------------------
// JWT helpers
// ---------------------------------------------------------------------------

/// Decode the payload of a JWT **without** signature verification.
///
/// This is intentionally minimal — real validation will be added when the
/// OIDC provider integration lands.
fn decode_jwt_claims(token: &str) -> Option<AuthUser> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }

    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&payload).ok()?;

    Some(AuthUser {
        email: claims["email"]
            .as_str()
            .or(claims["preferred_username"].as_str())
            .unwrap_or("unknown")
            .to_string(),
        name: claims["name"].as_str().unwrap_or("Unknown").to_string(),
        groups: claims["groups"]
            .as_array()
            .map(|g| {
                g.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

// ---------------------------------------------------------------------------
// Auth routes
// ---------------------------------------------------------------------------

/// `GET /auth/login` — redirect to OIDC provider (placeholder).
pub async fn login() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        "OIDC login not yet configured. Set OIDC_ENABLED=true with issuer/client config.",
    )
}

/// `GET /auth/callback` — OIDC callback (placeholder).
pub async fn auth_callback() -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, "OIDC callback placeholder")
}

/// `POST /auth/logout`
pub async fn logout() -> impl IntoResponse {
    (StatusCode::OK, "Logged out")
}

/// `GET /auth/me` — return current user identity.
pub async fn me(axum::Extension(user): axum::Extension<AuthUser>) -> Json<AuthUser> {
    Json(user)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_valid_jwt_claims() {
        // Build a minimal JWT with base64url-encoded payload.
        let payload = serde_json::json!({
            "email": "alice@contoso.com",
            "name": "Alice",
            "groups": ["admins", "devs"]
        });
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&payload).unwrap());
        let token = format!("header.{}.signature", encoded);

        let user = decode_jwt_claims(&token).expect("should decode");
        assert_eq!(user.email, "alice@contoso.com");
        assert_eq!(user.name, "Alice");
        assert_eq!(user.groups, vec!["admins", "devs"]);
    }

    #[test]
    fn decode_jwt_preferred_username_fallback() {
        let payload = serde_json::json!({
            "preferred_username": "bob@contoso.com",
            "name": "Bob"
        });
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&payload).unwrap());
        let token = format!("header.{}.signature", encoded);

        let user = decode_jwt_claims(&token).expect("should decode");
        assert_eq!(user.email, "bob@contoso.com");
    }

    #[test]
    fn decode_jwt_invalid_token() {
        assert!(decode_jwt_claims("not-a-jwt").is_none());
        assert!(decode_jwt_claims("a.b").is_none());
        assert!(decode_jwt_claims("a.!!!.c").is_none());
    }

    #[test]
    fn oidc_config_disabled_by_default() {
        // Ensure OIDC_ENABLED is not set (it shouldn't be in test env).
        std::env::remove_var("OIDC_ENABLED");
        assert!(OidcConfig::from_env().is_none());
    }
}
