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

fn parse_api_error(status: u16, body: String) -> AzureError {
    match serde_json::from_str::<AzureErrorResponse>(&body) {
        Ok(err) => AzureError::Api {
            status,
            code: err.error.code,
            message: err.error.message,
        },
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

impl ArmClient {
    pub fn new(cloud: AzureCloud) -> Result<Self> {
        let credential =
            DefaultAzureCredential::new().map_err(|e| AzureError::Auth(e.to_string()))?;
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
            .map_err(|e| AzureError::Auth(e.to_string()))?;
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

    async fn handle_response<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T> {
        let status = response.status().as_u16();
        if status == 404 {
            return Err(AzureError::NotFound);
        }
        if status == 409 {
            let text = read_response_text(response).await?;
            let conflict = match parse_api_error(status, text) {
                AzureError::Api { code, message, .. } => {
                    AzureError::Conflict(format!("[{code}] {message}"))
                }
                other => AzureError::Conflict(other.to_string()),
            };
            return Err(conflict);
        }
        if !response.status().is_success() {
            let text = read_response_text(response).await?;
            return Err(parse_api_error(status, text));
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
            .map_err(|e| AzureError::Network(e.to_string()))?;
        self.handle_response(response).await
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
            .map_err(|e| AzureError::Network(e.to_string()))?;
        self.handle_response(response).await
    }

    pub async fn post(&self, path: &str, body: Option<&impl Serialize>) -> Result<()> {
        let token = self.get_token().await?;
        let url = self.url(path);
        tracing::debug!("ARM POST {}", url);
        let mut req = self.http.post(&url).bearer_auth(&token);
        if let Some(b) = body {
            req = req.json(b);
        }
        let response = req
            .send()
            .await
            .map_err(|e| AzureError::Network(e.to_string()))?;
        if response.status().is_success() {
            return Ok(());
        }
        let status = response.status().as_u16();
        let text = read_response_text(response).await?;
        Err(parse_api_error(status, text))
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
            .map_err(|e| AzureError::Network(e.to_string()))?;
        if response.status().is_success() {
            return Ok(());
        }
        let status = response.status().as_u16();
        let text = read_response_text(response).await?;
        Err(parse_api_error(status, text))
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
    fn parse_api_error_reports_unparseable_bodies() {
        let err = parse_api_error(502, "<html>gateway failure</html>".to_string());

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
}
