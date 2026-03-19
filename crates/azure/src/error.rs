use thiserror::Error;

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
}
