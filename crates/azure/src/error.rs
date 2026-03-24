use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AzureAiProviderErrorClass {
    Configuration,
    Credential,
    Authorization,
    Endpoint,
    Timeout,
    Network,
    Unknown,
}

#[derive(Error, Debug)]
pub enum AzureError {
    #[error("Azure authentication failed: {0}. Reconnect your Azure account and retry.")]
    Auth(String),

    #[error("Azure token expired: {0}. Reconnect your Azure account and retry.")]
    TokenExpired(String),

    #[error(
        "Azure subscription '{subscription_id}' was not found. Verify the subscription ID and that your account can access it."
    )]
    SubscriptionNotFound { subscription_id: String },

    #[error(
        "Azure resource group '{resource_group}' was not found{subscription_context}. Verify the AKS identity settings and subscription scope."
    )]
    ResourceGroupNotFound {
        resource_group: String,
        subscription_context: String,
    },

    #[error(
        "AKS cluster '{cluster_name}' was not found{resource_group_context}{subscription_context}. Verify cluster name/resource group and Azure cloud selection."
    )]
    ClusterNotFound {
        cluster_name: String,
        resource_group_context: String,
        subscription_context: String,
    },

    #[error(
        "Azure permission denied for {scope}: {message}. Ensure your identity has at least Reader access to this scope."
    )]
    PermissionDenied { scope: String, message: String },

    #[error("Azure API error ({status}): [{code}] {message}")]
    Api {
        status: u16,
        code: String,
        message: String,
    },

    #[error("Resource not found")]
    NotFound,

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Azure ARM request timed out: {0}. Check connectivity and retry.")]
    Timeout(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Azure OpenAI configuration error: {0}")]
    OpenAiConfiguration(String),

    #[error("Azure OpenAI endpoint '{endpoint}' is invalid: {reason}")]
    InvalidOpenAiEndpoint { endpoint: String, reason: String },

    #[error(
        "Azure OpenAI credential acquisition failed: {0}. Reauthenticate your Azure account or switch to API key mode."
    )]
    OpenAiCredential(String),

    #[error(
        "Azure OpenAI rejected authentication at {endpoint}: {message}. Verify the selected auth mode, cloud profile, and endpoint. If you are using API key mode, verify the API key. If you are using Azure login, reauthenticate only if your Azure session is stale."
    )]
    OpenAiAuthenticationFailed { endpoint: String, message: String },

    #[error(
        "Azure OpenAI API key authentication failed at {endpoint}: {message}. Verify that the API key belongs to this Azure OpenAI resource and retry."
    )]
    OpenAiInvalidApiKey { endpoint: String, message: String },

    #[error(
        "Azure OpenAI authorization failed for chat completions at {endpoint}: {message}. If you are using Azure login, ensure your identity has Cognitive Services OpenAI User or Contributor access. If you are using API key mode, verify that this resource and deployment allow the request."
    )]
    OpenAiPermissionDenied { endpoint: String, message: String },

    #[error("Azure OpenAI API error ({status}): [{code}] {message}")]
    OpenAiApi {
        status: u16,
        code: String,
        message: String,
    },

    #[error("Azure OpenAI request timed out: {0}. Check connectivity and retry.")]
    OpenAiTimeout(String),

    #[error("Azure OpenAI network error: {0}")]
    OpenAiNetwork(String),
}

pub type Result<T> = std::result::Result<T, AzureError>;

/// Azure REST API error response shape
#[derive(serde::Deserialize)]
pub(crate) struct AzureErrorResponse {
    pub error: AzureErrorBody,
}

#[derive(serde::Deserialize)]
pub(crate) struct AzureErrorBody {
    pub code: String,
    pub message: String,
}

impl AzureError {
    pub fn ai_provider_error_class(&self) -> AzureAiProviderErrorClass {
        match self {
            AzureError::OpenAiConfiguration(_) => AzureAiProviderErrorClass::Configuration,
            AzureError::InvalidOpenAiEndpoint { .. } => AzureAiProviderErrorClass::Endpoint,
            AzureError::OpenAiCredential(_)
            | AzureError::OpenAiAuthenticationFailed { .. }
            | AzureError::OpenAiInvalidApiKey { .. }
            | AzureError::TokenExpired(_) => {
                AzureAiProviderErrorClass::Credential
            }
            AzureError::OpenAiPermissionDenied { .. } => AzureAiProviderErrorClass::Authorization,
            AzureError::OpenAiTimeout(_) | AzureError::Timeout(_) => {
                AzureAiProviderErrorClass::Timeout
            }
            AzureError::OpenAiNetwork(_) | AzureError::Network(_) => {
                AzureAiProviderErrorClass::Network
            }
            AzureError::OpenAiApi { .. } => AzureAiProviderErrorClass::Unknown,
            _ => AzureAiProviderErrorClass::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_auth() {
        let err = AzureError::Auth("token expired".to_string());
        assert!(err
            .to_string()
            .contains("Azure authentication failed: token expired"));
    }

    #[test]
    fn error_display_api() {
        let err = AzureError::Api {
            status: 400,
            code: "BadRequest".to_string(),
            message: "Invalid parameter".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Azure API error (400): [BadRequest] Invalid parameter"
        );
    }

    #[test]
    fn error_display_not_found() {
        let err = AzureError::NotFound;
        assert_eq!(err.to_string(), "Resource not found");
    }

    #[test]
    fn error_display_token_expired() {
        let err = AzureError::TokenExpired("ExpiredAuthenticationToken".to_string());
        assert!(err.to_string().contains("Azure token expired"));
    }

    #[test]
    fn error_response_deserialization() {
        let json =
            r#"{"error":{"code":"ResourceNotFound","message":"The resource was not found"}}"#;
        let resp: AzureErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.error.code, "ResourceNotFound");
        assert_eq!(resp.error.message, "The resource was not found");
    }

    #[test]
    fn error_response_deserialization_complex() {
        let json = r#"{"error":{"code":"AuthorizationFailed","message":"The client does not have authorization to perform action."}}"#;
        let resp: AzureErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.error.code, "AuthorizationFailed");
    }

    #[test]
    fn ai_provider_error_classifies_endpoint_error() {
        let err = AzureError::InvalidOpenAiEndpoint {
            endpoint: "https://example.invalid".to_string(),
            reason: "host does not match the selected Azure cloud".to_string(),
        };

        assert_eq!(
            err.ai_provider_error_class(),
            AzureAiProviderErrorClass::Endpoint
        );
    }

    #[test]
    fn ai_provider_error_classifies_openai_credential_error() {
        let err = AzureError::OpenAiCredential("az login required".to_string());

        assert_eq!(
            err.ai_provider_error_class(),
            AzureAiProviderErrorClass::Credential
        );
    }

    #[test]
    fn ai_provider_error_classifies_openai_authentication_failure() {
        let err = AzureError::OpenAiAuthenticationFailed {
            endpoint: "https://example.openai.azure.com".to_string(),
            message: "Unauthorized".to_string(),
        };

        assert_eq!(
            err.ai_provider_error_class(),
            AzureAiProviderErrorClass::Credential
        );
    }

    #[test]
    fn ai_provider_error_classifies_invalid_api_key() {
        let err = AzureError::OpenAiInvalidApiKey {
            endpoint: "https://example.openai.azure.com".to_string(),
            message: "Access denied due to invalid subscription key".to_string(),
        };

        assert_eq!(
            err.ai_provider_error_class(),
            AzureAiProviderErrorClass::Credential
        );
    }

    #[test]
    fn ai_provider_error_classifies_openai_timeout_error() {
        let err = AzureError::OpenAiTimeout("request timed out".to_string());

        assert_eq!(
            err.ai_provider_error_class(),
            AzureAiProviderErrorClass::Timeout
        );
    }

    #[test]
    fn ai_provider_error_classifies_openai_permission_denied() {
        let err = AzureError::OpenAiPermissionDenied {
            endpoint: "https://example.openai.azure.com".to_string(),
            message: "forbidden".to_string(),
        };

        assert_eq!(
            err.ai_provider_error_class(),
            AzureAiProviderErrorClass::Authorization
        );
    }
}
