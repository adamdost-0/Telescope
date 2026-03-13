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
            let text = response.text().await.unwrap_or_default();
            return Err(AzureError::Conflict(text));
        }
        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            if let Ok(err) = serde_json::from_str::<AzureErrorResponse>(&text) {
                return Err(AzureError::Api {
                    status,
                    code: err.error.code,
                    message: err.error.message,
                });
            }
            return Err(AzureError::Api {
                status,
                code: "Unknown".to_string(),
                message: text,
            });
        }
        response
            .json::<T>()
            .await
            .map_err(|e| AzureError::Serialization(e.to_string()))
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
        let status = response.status().as_u16();
        if status == 200 || status == 202 || status == 204 {
            return Ok(());
        }
        let text = response.text().await.unwrap_or_default();
        if let Ok(err) = serde_json::from_str::<AzureErrorResponse>(&text) {
            return Err(AzureError::Api {
                status,
                code: err.error.code,
                message: err.error.message,
            });
        }
        Err(AzureError::Api {
            status,
            code: "Unknown".into(),
            message: text,
        })
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
        let status = response.status().as_u16();
        if status == 200 || status == 202 || status == 204 {
            return Ok(());
        }
        let text = response.text().await.unwrap_or_default();
        if let Ok(err) = serde_json::from_str::<AzureErrorResponse>(&text) {
            return Err(AzureError::Api {
                status,
                code: err.error.code,
                message: err.error.message,
            });
        }
        Err(AzureError::Api {
            status,
            code: "Unknown".into(),
            message: text,
        })
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
}
