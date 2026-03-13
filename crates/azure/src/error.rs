use thiserror::Error;

#[derive(Error, Debug)]
pub enum AzureError {
    #[error("Azure authentication failed: {0}")]
    Auth(String),

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
        assert_eq!(
            err.to_string(),
            "Azure authentication failed: token expired"
        );
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
