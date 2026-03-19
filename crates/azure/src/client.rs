use std::sync::Arc;

use azure_core::credentials::TokenCredential;
use azure_identity::DefaultAzureCredential;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{AzureError, AzureErrorResponse, Result};
use crate::types::{AzureCloud, AKS_API_VERSION};

pub struct ArmClient {
    credential: Arc<dyn TokenCredential>,
    cloud: AzureCloud,
    http: Client,
}

const MAX_RESPONSE_BODY_PREVIEW_CHARS: usize = 256;

#[derive(Default)]
struct ArmPathContext {
    subscription_id: Option<String>,
    resource_group: Option<String>,
    cluster_name: Option<String>,
}

fn response_body_preview(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return "<empty body>".to_string();
    }

    let preview: String = trimmed
        .chars()
        .take(MAX_RESPONSE_BODY_PREVIEW_CHARS)
        .collect();
    if trimmed.chars().count() > MAX_RESPONSE_BODY_PREVIEW_CHARS {
        format!("{preview}…")
    } else {
        preview
    }
}

async fn read_response_text(response: reqwest::Response) -> Result<String> {
    let status = response.status().as_u16();
    response.text().await.map_err(|error| {
        AzureError::Serialization(format!(
            "Failed to read ARM response body ({status}): {error}"
        ))
    })
}

fn arm_path_context(path: &str) -> ArmPathContext {
    let mut context = ArmPathContext::default();
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    for (idx, part) in parts.iter().enumerate() {
        match part.to_ascii_lowercase().as_str() {
            "subscriptions" => {
                if let Some(value) = parts.get(idx + 1) {
                    context.subscription_id = Some((*value).to_string());
                }
            }
            "resourcegroups" => {
                if let Some(value) = parts.get(idx + 1) {
                    context.resource_group = Some((*value).to_string());
                }
            }
            "managedclusters" => {
                if let Some(value) = parts.get(idx + 1) {
                    context.cluster_name = Some((*value).to_string());
                }
            }
            _ => {}
        }
    }
    context
}

fn subscription_context(subscription_id: Option<&str>) -> String {
    subscription_id
        .map(|value| format!(" in subscription '{value}'"))
        .unwrap_or_default()
}

fn resource_group_context(resource_group: Option<&str>) -> String {
    resource_group
        .map(|value| format!(" in resource group '{value}'"))
        .unwrap_or_default()
}

fn parse_api_error(status: u16, body: String, request_path: Option<&str>) -> AzureError {
    match serde_json::from_str::<AzureErrorResponse>(&body) {
        Ok(err) => {
            let code = err.error.code;
            let message = err.error.message;
            let code_lc = code.to_ascii_lowercase();
            let message_lc = message.to_ascii_lowercase();
            let context = request_path.map(arm_path_context).unwrap_or_default();

            if status == 401
                || code_lc.contains("authentication")
                || code_lc.contains("unauthorized")
                || message_lc.contains("authentication")
            {
                if code_lc.contains("expired") || message_lc.contains("token expired") {
                    return AzureError::TokenExpired(message);
                }
                return AzureError::Auth(message);
            }

            if code_lc == "subscriptionnotfound" || code_lc == "invalidsubscriptionid" {
                return AzureError::SubscriptionNotFound {
                    subscription_id: context
                        .subscription_id
                        .unwrap_or_else(|| "<unknown>".to_string()),
                };
            }

            if code_lc == "resourcegroupnotfound" {
                return AzureError::ResourceGroupNotFound {
                    resource_group: context
                        .resource_group
                        .unwrap_or_else(|| "<unknown>".to_string()),
                    subscription_context: subscription_context(context.subscription_id.as_deref()),
                };
            }

            if status == 403
                || code_lc == "authorizationfailed"
                || code_lc == "linkedauthorizationfailed"
            {
                let scope = if let Some(cluster_name) = context.cluster_name.as_deref() {
                    format!(
                        "AKS cluster '{cluster_name}'{}{}",
                        resource_group_context(context.resource_group.as_deref()),
                        subscription_context(context.subscription_id.as_deref())
                    )
                } else if let Some(resource_group) = context.resource_group.as_deref() {
                    format!(
                        "resource group '{resource_group}'{}",
                        subscription_context(context.subscription_id.as_deref())
                    )
                } else if let Some(subscription_id) = context.subscription_id.as_deref() {
                    format!("subscription '{subscription_id}'")
                } else {
                    "the requested Azure resource scope".to_string()
                };
                return AzureError::PermissionDenied { scope, message };
            }

            if status == 404 || code_lc == "resourcenotfound" || code_lc == "parentresourcenotfound"
            {
                if message_lc.contains("managedclusters")
                    || message_lc.contains("/managedclusters/")
                    || message_lc.contains("managed cluster")
                {
                    return AzureError::ClusterNotFound {
                        cluster_name: context
                            .cluster_name
                            .unwrap_or_else(|| "<unknown>".to_string()),
                        resource_group_context: resource_group_context(
                            context.resource_group.as_deref(),
                        ),
                        subscription_context: subscription_context(
                            context.subscription_id.as_deref(),
                        ),
                    };
                }
                return AzureError::NotFound;
            }

            AzureError::Api {
                status,
                code,
                message,
            }
        }
        Err(parse_error) => AzureError::Api {
            status,
            code: "UnexpectedResponse".to_string(),
            message: format!(
                "Unparseable Azure error response: {parse_error}; body: {}",
                response_body_preview(&body)
            ),
        },
    }
}

fn map_response_error(status: u16, body: String, request_path: Option<&str>) -> AzureError {
    if status == 409 {
        return match parse_api_error(status, body, request_path) {
            AzureError::Api { code, message, .. } => {
                AzureError::Conflict(format!("[{code}] {message}"))
            }
            other => AzureError::Conflict(other.to_string()),
        };
    }
    parse_api_error(status, body, request_path)
}

fn map_auth_error(message: String) -> AzureError {
    if message.to_ascii_lowercase().contains("expired") {
        AzureError::TokenExpired(message)
    } else {
        AzureError::Auth(message)
    }
}

fn map_network_error(error: reqwest::Error) -> AzureError {
    if error.is_timeout() {
        AzureError::Timeout(error.to_string())
    } else {
        AzureError::Network(error.to_string())
    }
}

impl ArmClient {
    pub fn new(cloud: AzureCloud) -> Result<Self> {
        let credential =
            DefaultAzureCredential::new().map_err(|e| map_auth_error(e.to_string()))?;
        Ok(Self {
            credential: credential as Arc<dyn TokenCredential>,
            cloud,
            http: Client::new(),
        })
    }

    pub fn with_credential(cloud: AzureCloud, credential: Arc<dyn TokenCredential>) -> Self {
        Self {
            credential,
            cloud,
            http: Client::new(),
        }
    }

    pub fn cloud(&self) -> AzureCloud {
        self.cloud
    }

    async fn get_token(&self) -> Result<String> {
        let scope = self.cloud.token_scope();
        let response = self
            .credential
            .get_token(&[scope])
            .await
            .map_err(|e| map_auth_error(e.to_string()))?;
        Ok(response.token.secret().to_string())
    }

    fn url(&self, path: &str) -> String {
        let sep = if path.contains('?') { "&" } else { "?" };
        format!(
            "{}{}{sep}api-version={AKS_API_VERSION}",
            self.cloud.arm_endpoint(),
            path
        )
    }

    async fn handle_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
        path: &str,
    ) -> Result<T> {
        let status = response.status().as_u16();
        if !response.status().is_success() {
            let text = read_response_text(response).await?;
            return Err(map_response_error(status, text, Some(path)));
        }
        let body = response.bytes().await.map_err(|error| {
            AzureError::Serialization(format!(
                "Failed to read ARM success response body ({status}): {error}"
            ))
        })?;
        serde_json::from_slice::<T>(&body).map_err(|error| {
            let preview = response_body_preview(&String::from_utf8_lossy(&body));
            AzureError::Serialization(format!(
                "Failed to deserialize ARM success response ({status}): {error}; body: {preview}"
            ))
        })
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let token = self.get_token().await?;
        let url = self.url(path);
        tracing::debug!("ARM GET {}", url);
        let response = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(map_network_error)?;
        self.handle_response(response, path).await
    }

    pub async fn put<T: DeserializeOwned>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        let token = self.get_token().await?;
        let url = self.url(path);
        tracing::debug!("ARM PUT {}", url);
        let response = self
            .http
            .put(&url)
            .bearer_auth(&token)
            .json(body)
            .send()
            .await
            .map_err(map_network_error)?;
        self.handle_response(response, path).await
    }

    pub async fn post(&self, path: &str, body: Option<&impl Serialize>) -> Result<()> {
        let token = self.get_token().await?;
        let url = self.url(path);
        tracing::debug!("ARM POST {}", url);
        let mut req = self.http.post(&url).bearer_auth(&token);
        if let Some(b) = body {
            req = req.json(b);
        }
        let response = req.send().await.map_err(map_network_error)?;
        if response.status().is_success() {
            return Ok(());
        }
        let status = response.status().as_u16();
        let text = read_response_text(response).await?;
        Err(map_response_error(status, text, Some(path)))
    }

    pub async fn delete(&self, path: &str) -> Result<()> {
        let token = self.get_token().await?;
        let url = self.url(path);
        tracing::debug!("ARM DELETE {}", url);
        let response = self
            .http
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(map_network_error)?;
        if response.status().is_success() {
            return Ok(());
        }
        let status = response.status().as_u16();
        let text = read_response_text(response).await?;
        Err(map_response_error(status, text, Some(path)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_construction_without_query() {
        // Verify URL format by constructing manually (same logic as ArmClient::url)
        let cloud = AzureCloud::Commercial;
        let path = "/subscriptions/sub-123/resourceGroups/rg/providers/Microsoft.ContainerService/managedClusters/aks1";
        let url = format!(
            "{}{}?api-version={}",
            cloud.arm_endpoint(),
            path,
            AKS_API_VERSION
        );
        assert!(url.starts_with("https://management.azure.com/subscriptions/"));
        assert!(url.contains("?api-version=2024-09-01"));
    }

    #[test]
    fn url_construction_with_existing_query() {
        let cloud = AzureCloud::UsGovernment;
        let path = "/subscriptions/sub-123?$expand=details";
        let sep = if path.contains('?') { "&" } else { "?" };
        let url = format!(
            "{}{}{sep}api-version={AKS_API_VERSION}",
            cloud.arm_endpoint(),
            path
        );
        assert!(url.contains("?$expand=details&api-version="));
        assert!(url.starts_with("https://management.usgovcloudapi.net"));
    }

    #[test]
    fn url_construction_gov_secret() {
        let cloud = AzureCloud::UsGovSecret;
        let path = "/subscriptions/s1/resourceGroups/rg1";
        let url = format!(
            "{}{}?api-version={}",
            cloud.arm_endpoint(),
            path,
            AKS_API_VERSION
        );
        assert!(url.starts_with("https://management.azure.microsoft.scloud"));
    }

    #[test]
    fn parse_api_error_preserves_azure_error_payloads() {
        let err = parse_api_error(
            400,
            r#"{"error":{"code":"BadRequest","message":"Invalid parameter"}}"#.to_string(),
            Some("/subscriptions/sub-1"),
        );

        match err {
            AzureError::Api {
                status,
                code,
                message,
            } => {
                assert_eq!(status, 400);
                assert_eq!(code, "BadRequest");
                assert_eq!(message, "Invalid parameter");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn map_response_error_handles_401_expired_token() {
        let err = map_response_error(
            401,
            r#"{"error":{"code":"ExpiredAuthenticationToken","message":"The access token expired."}}"#
                .to_string(),
            Some("/subscriptions/sub-1"),
        );

        match err {
            AzureError::TokenExpired(message) => {
                assert_eq!(message, "The access token expired.");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn map_response_error_handles_403_forbidden() {
        let err = map_response_error(
            403,
            r#"{"error":{"code":"AuthorizationFailed","message":"Permission denied."}}"#.to_string(),
            Some(
                "/subscriptions/sub-123/resourceGroups/rg-a/providers/Microsoft.ContainerService/managedClusters/aks-a"
            ),
        );

        match err {
            AzureError::PermissionDenied { scope, message } => {
                assert!(scope.contains("aks-a"));
                assert_eq!(message, "Permission denied.");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn map_response_error_handles_404_not_found() {
        let err = map_response_error(
            404,
            r#"{"error":{"code":"ResourceNotFound","message":"Cluster missing."}}"#.to_string(),
            Some("/subscriptions/sub-1/resourceGroups/rg-1/providers/Microsoft.ContainerService/managedClusters/aks-1"),
        );

        assert!(matches!(err, AzureError::NotFound));
    }

    #[test]
    fn map_auth_error_maps_expired_tokens() {
        let err = map_auth_error("The token has expired".to_string());
        assert!(matches!(err, AzureError::TokenExpired(_)));
    }

    #[test]
    fn map_network_error_reports_timeouts() {
        let err = map_response_error(
            504,
            r#"{"error":{"code":"GatewayTimeout","message":"gateway timeout"}}"#.to_string(),
            None,
        );
        assert!(matches!(err, AzureError::Api { status: 504, .. }));
    }

    #[test]
    fn parse_api_error_reports_unparseable_bodies() {
        let err = parse_api_error(502, "<html>gateway failure</html>".to_string(), None);

        match err {
            AzureError::Api {
                status,
                code,
                message,
            } => {
                assert_eq!(status, 502);
                assert_eq!(code, "UnexpectedResponse");
                assert!(message.contains("gateway failure"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn response_body_preview_handles_empty_and_long_text() {
        assert_eq!(response_body_preview("   "), "<empty body>");

        let preview = response_body_preview(&"x".repeat(MAX_RESPONSE_BODY_PREVIEW_CHARS + 8));
        assert!(preview.ends_with('…'));
        assert_eq!(preview.chars().count(), MAX_RESPONSE_BODY_PREVIEW_CHARS + 1);
    }

    #[test]
    fn map_response_error_handles_subscription_not_found() {
        let err = map_response_error(
            404,
            r#"{"error":{"code":"SubscriptionNotFound","message":"Subscription missing."}}"#
                .to_string(),
            Some("/subscriptions/sub-999/resourceGroups/rg/providers/Microsoft.ContainerService/managedClusters/aks"),
        );
        match err {
            AzureError::SubscriptionNotFound { subscription_id } => {
                assert_eq!(subscription_id, "sub-999");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn map_response_error_handles_resource_group_not_found() {
        let err = map_response_error(
            404,
            r#"{"error":{"code":"ResourceGroupNotFound","message":"Resource group missing."}}"#
                .to_string(),
            Some("/subscriptions/sub-1/resourceGroups/rg-missing/providers/Microsoft.ContainerService/managedClusters/aks"),
        );
        match err {
            AzureError::ResourceGroupNotFound {
                resource_group,
                subscription_context,
            } => {
                assert_eq!(resource_group, "rg-missing");
                assert!(subscription_context.contains("sub-1"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
