use std::fmt;
use std::sync::Arc;

use azure_core::credentials::{Secret, TokenCredential};
use azure_identity::{DefaultAzureCredential, TokenCredentialOptions};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{AzureError, AzureErrorResponse, Result};
use crate::types::{AzureCloud, AZURE_OPENAI_API_VERSION};

const MAX_RESPONSE_BODY_PREVIEW_CHARS: usize = 256;
const TEST_CONNECTION_PROMPT: &str = "Reply with the single word ok.";

#[derive(Clone, PartialEq, Eq)]
pub enum AzureOpenAiAuth {
    DefaultAzureCredential,
    ApiKey(Secret),
}

impl fmt::Debug for AzureOpenAiAuth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DefaultAzureCredential => f.write_str("DefaultAzureCredential"),
            Self::ApiKey(_) => f.write_str("ApiKey(<redacted>)"),
        }
    }
}

impl AzureOpenAiAuth {
    fn uses_api_key(&self) -> bool {
        matches!(self, Self::ApiKey(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AzureOpenAiClientOptions {
    pub endpoint: String,
    pub deployment_name: String,
    pub cloud: AzureCloud,
    pub auth: AzureOpenAiAuth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AzureOpenAiEndpoint {
    deployment_name: String,
    normalized_endpoint: Url,
    chat_completions_url: Url,
}

impl AzureOpenAiEndpoint {
    pub fn new(endpoint: &str, cloud: AzureCloud, deployment_name: &str) -> Result<Self> {
        let endpoint = endpoint.trim();
        if endpoint.is_empty() {
            return Err(AzureError::OpenAiConfiguration(
                "Endpoint is required for Azure OpenAI.".to_string(),
            ));
        }

        let deployment_name = deployment_name.trim();
        if deployment_name.is_empty() {
            return Err(AzureError::OpenAiConfiguration(
                "Deployment name is required for Azure OpenAI chat completions.".to_string(),
            ));
        }

        let mut parsed =
            Url::parse(endpoint).map_err(|error| AzureError::InvalidOpenAiEndpoint {
                endpoint: endpoint.to_string(),
                reason: format!("failed to parse URL: {error}"),
            })?;

        if parsed.scheme() != "https" {
            return Err(AzureError::InvalidOpenAiEndpoint {
                endpoint: endpoint.to_string(),
                reason: "endpoint must use https".to_string(),
            });
        }

        if parsed.host_str().is_none() {
            return Err(AzureError::InvalidOpenAiEndpoint {
                endpoint: endpoint.to_string(),
                reason: "endpoint must include a hostname".to_string(),
            });
        }

        let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
        let expected_suffix = cloud.openai_host_suffix();
        let expected_suffix_with_dot = format!(".{expected_suffix}");
        if !(host == expected_suffix || host.ends_with(&expected_suffix_with_dot)) {
            return Err(AzureError::InvalidOpenAiEndpoint {
                endpoint: endpoint.to_string(),
                reason: format!(
                    "host '{host}' does not match the selected Azure cloud suffix '{expected_suffix}'"
                ),
            });
        }

        let resource_label = host.strip_suffix(expected_suffix).unwrap_or_default();
        if resource_label.trim_end_matches('.').is_empty() {
            return Err(AzureError::InvalidOpenAiEndpoint {
                endpoint: endpoint.to_string(),
                reason: format!(
                    "endpoint must include an Azure OpenAI resource name before '{expected_suffix}'"
                ),
            });
        }

        let path = parsed.path();
        if !(path.is_empty() || path == "/") {
            return Err(AzureError::InvalidOpenAiEndpoint {
                endpoint: endpoint.to_string(),
                reason:
                    "endpoint must not include a path; use the Azure OpenAI resource root URL only"
                        .to_string(),
            });
        }

        if parsed.query().is_some() {
            return Err(AzureError::InvalidOpenAiEndpoint {
                endpoint: endpoint.to_string(),
                reason: "endpoint must not include a query string".to_string(),
            });
        }

        if parsed.fragment().is_some() {
            return Err(AzureError::InvalidOpenAiEndpoint {
                endpoint: endpoint.to_string(),
                reason: "endpoint must not include a URL fragment".to_string(),
            });
        }

        parsed.set_path("/");
        parsed.set_query(None);
        parsed.set_fragment(None);

        let mut chat_completions_url = parsed.clone();
        {
            let mut segments = chat_completions_url.path_segments_mut().map_err(|_| {
                AzureError::InvalidOpenAiEndpoint {
                    endpoint: endpoint.to_string(),
                    reason: "endpoint cannot be used as a base URL".to_string(),
                }
            })?;
            segments.clear();
            segments.push("openai");
            segments.push("deployments");
            segments.push(deployment_name);
            segments.push("chat");
            segments.push("completions");
        }
        chat_completions_url
            .query_pairs_mut()
            .append_pair("api-version", AZURE_OPENAI_API_VERSION);

        Ok(Self {
            deployment_name: deployment_name.to_string(),
            normalized_endpoint: parsed,
            chat_completions_url,
        })
    }

    pub fn deployment_name(&self) -> &str {
        &self.deployment_name
    }

    pub fn normalized_endpoint(&self) -> &Url {
        &self.normalized_endpoint
    }

    pub fn chat_completions_url(&self) -> &Url {
        &self.chat_completions_url
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AzureOpenAiChatRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AzureOpenAiChatMessage {
    pub role: AzureOpenAiChatRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum AzureOpenAiResponseFormat {
    Text,
    JsonObject,
    JsonSchema(AzureOpenAiResponseFormatJsonSchema),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct AzureOpenAiResponseFormatJsonSchema {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub schema: Value,
    pub strict: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AzureOpenAiChatCompletionsRequest {
    pub messages: Vec<AzureOpenAiChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<AzureOpenAiResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AzureOpenAiTokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AzureOpenAiChatCompletion {
    pub id: String,
    pub model: String,
    pub content: String,
    pub usage: Option<AzureOpenAiTokenUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AzureOpenAiConnectionTestResult {
    pub normalized_endpoint: String,
    pub chat_completions_url: String,
    pub model: String,
}

pub struct AzureOpenAiClient {
    endpoint: AzureOpenAiEndpoint,
    cloud: AzureCloud,
    auth: AzureOpenAiAuth,
    credential: Option<Arc<dyn TokenCredential>>,
    http: Client,
}

impl fmt::Debug for AzureOpenAiClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AzureOpenAiClient")
            .field("endpoint", &self.endpoint)
            .field("cloud", &self.cloud)
            .field("auth", &self.auth)
            .finish_non_exhaustive()
    }
}

impl AzureOpenAiClient {
    pub fn new(options: AzureOpenAiClientOptions) -> Result<Self> {
        let endpoint =
            AzureOpenAiEndpoint::new(&options.endpoint, options.cloud, &options.deployment_name)?;
        let credential = match options.auth {
            AzureOpenAiAuth::DefaultAzureCredential => {
                Some(build_default_azure_credential(options.cloud)?)
            }
            AzureOpenAiAuth::ApiKey(_) => None,
        };

        Ok(Self {
            endpoint,
            cloud: options.cloud,
            auth: options.auth,
            credential,
            http: Client::new(),
        })
    }

    pub fn with_credential(
        options: AzureOpenAiClientOptions,
        credential: Arc<dyn TokenCredential>,
    ) -> Result<Self> {
        let endpoint =
            AzureOpenAiEndpoint::new(&options.endpoint, options.cloud, &options.deployment_name)?;

        Ok(Self {
            endpoint,
            cloud: options.cloud,
            auth: options.auth,
            credential: Some(credential),
            http: Client::new(),
        })
    }

    pub fn endpoint(&self) -> &AzureOpenAiEndpoint {
        &self.endpoint
    }

    pub async fn chat_completions(
        &self,
        request: AzureOpenAiChatCompletionsRequest,
    ) -> Result<AzureOpenAiChatCompletion> {
        if request.messages.is_empty() {
            return Err(AzureError::OpenAiConfiguration(
                "At least one chat message is required for Azure OpenAI chat completions."
                    .to_string(),
            ));
        }

        let wire_request = build_wire_request(request);
        let response = self.send_chat_request(&wire_request).await?;
        let status = response.status().as_u16();
        if !response.status().is_success() {
            let body = read_response_text(response).await?;
            return Err(map_openai_response_error(
                status,
                body,
                &self.endpoint,
                &self.auth,
            ));
        }

        let body = response.bytes().await.map_err(|error| {
            AzureError::Serialization(format!(
                "Failed to read Azure OpenAI success response body ({status}): {error}"
            ))
        })?;
        let parsed = serde_json::from_slice::<WireChatCompletionResponse>(&body).map_err(|error| {
            let preview = response_body_preview(&String::from_utf8_lossy(&body));
            AzureError::Serialization(format!(
                "Failed to deserialize Azure OpenAI success response ({status}): {error}; body: {preview}"
            ))
        })?;

        let content = parsed
            .choices
            .into_iter()
            .find_map(|choice| choice.message.content)
            .map(|content| content.trim().to_string())
            .filter(|content| !content.is_empty())
            .ok_or_else(|| {
                AzureError::Serialization(
                    "Azure OpenAI returned no assistant content in the chat completion response."
                        .to_string(),
                )
            })?;

        Ok(AzureOpenAiChatCompletion {
            id: parsed.id,
            model: parsed.model,
            content,
            usage: parsed.usage.map(|usage| AzureOpenAiTokenUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
            }),
        })
    }

    pub async fn test_connection(&self) -> Result<AzureOpenAiConnectionTestResult> {
        let result = self
            .chat_completions(AzureOpenAiChatCompletionsRequest {
                messages: vec![
                    AzureOpenAiChatMessage {
                        role: AzureOpenAiChatRole::System,
                        content: "You are a connectivity probe for Telescope AI Insights."
                            .to_string(),
                    },
                    AzureOpenAiChatMessage {
                        role: AzureOpenAiChatRole::User,
                        content: TEST_CONNECTION_PROMPT.to_string(),
                    },
                ],
                response_format: Some(AzureOpenAiResponseFormat::Text),
                temperature: Some(0.0),
                max_tokens: Some(8),
            })
            .await?;

        Ok(AzureOpenAiConnectionTestResult {
            normalized_endpoint: self.endpoint.normalized_endpoint().to_string(),
            chat_completions_url: self.endpoint.chat_completions_url().to_string(),
            model: result.model,
        })
    }

    async fn send_chat_request(
        &self,
        request: &WireChatCompletionRequest,
    ) -> Result<reqwest::Response> {
        let mut builder = self
            .http
            .post(self.endpoint.chat_completions_url().clone())
            .json(request);

        match &self.auth {
            AzureOpenAiAuth::DefaultAzureCredential => {
                let credential = self.credential.as_ref().ok_or_else(|| {
                    AzureError::OpenAiCredential(
                        "No credential source was configured for Azure OpenAI bearer auth."
                            .to_string(),
                    )
                })?;
                let token = credential
                    .get_token(&[self.cloud.openai_token_scope()])
                    .await
                    .map_err(|error| AzureError::OpenAiCredential(error.to_string()))?;
                builder = builder.bearer_auth(token.token.secret());
            }
            AzureOpenAiAuth::ApiKey(api_key) => {
                builder = builder.header("api-key", api_key.secret());
            }
        }

        builder.send().await.map_err(map_openai_network_error)
    }
}

fn build_default_azure_credential(cloud: AzureCloud) -> Result<Arc<dyn TokenCredential>> {
    let mut options = TokenCredentialOptions::default();
    options.set_authority_host(cloud.auth_endpoint().to_string());

    DefaultAzureCredential::with_options(options)
        .map(|credential| credential as Arc<dyn TokenCredential>)
        .map_err(|error| AzureError::OpenAiCredential(error.to_string()))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WireChatCompletionRequest {
    messages: Vec<AzureOpenAiChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

fn build_wire_request(request: AzureOpenAiChatCompletionsRequest) -> WireChatCompletionRequest {
    WireChatCompletionRequest {
        messages: request.messages,
        response_format: request.response_format.map(response_format_json),
        temperature: request.temperature,
        max_tokens: request.max_tokens,
    }
}

fn response_format_json(response_format: AzureOpenAiResponseFormat) -> Value {
    match response_format {
        AzureOpenAiResponseFormat::Text => json!({ "type": "text" }),
        AzureOpenAiResponseFormat::JsonObject => json!({ "type": "json_object" }),
        AzureOpenAiResponseFormat::JsonSchema(schema) => {
            let mut json_schema = serde_json::Map::new();
            json_schema.insert("name".to_string(), json!(schema.name));
            if let Some(ref description) = schema.description {
                json_schema.insert("description".to_string(), json!(description));
            }
            json_schema.insert("schema".to_string(), schema.schema);
            json_schema.insert("strict".to_string(), json!(schema.strict));
            json!({
                "type": "json_schema",
                "json_schema": json_schema
            })
        }
    }
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
            "Failed to read Azure OpenAI response body ({status}): {error}"
        ))
    })
}

fn map_openai_network_error(error: reqwest::Error) -> AzureError {
    if error.is_timeout() {
        AzureError::OpenAiTimeout(error.to_string())
    } else {
        AzureError::OpenAiNetwork(error.to_string())
    }
}

fn map_openai_response_error(
    status: u16,
    body: String,
    endpoint: &AzureOpenAiEndpoint,
    auth: &AzureOpenAiAuth,
) -> AzureError {
    match serde_json::from_str::<AzureErrorResponse>(&body) {
        Ok(error_response) => classify_openai_response_error(
            status,
            Some(error_response.error.code),
            error_response.error.message,
            endpoint,
            auth,
        ),
        Err(parse_error) => classify_openai_response_error(
            status,
            None,
            format!(
                "Unparseable Azure OpenAI error response: {parse_error}; body: {}",
                response_body_preview(&body)
            ),
            endpoint,
            auth,
        ),
    }
}

fn classify_openai_response_error(
    status: u16,
    code: Option<String>,
    message: String,
    endpoint: &AzureOpenAiEndpoint,
    auth: &AzureOpenAiAuth,
) -> AzureError {
    let endpoint_url = endpoint.normalized_endpoint().to_string();
    let code_lc = code
        .as_deref()
        .unwrap_or("unexpectedresponse")
        .to_ascii_lowercase();
    let message_lc = message.to_ascii_lowercase();

    if status == 404
        || code_lc.contains("deployment")
        || message_lc.contains("deployment")
        || message_lc.contains("resource not found")
    {
        return AzureError::OpenAiConfiguration(format!(
            "Azure OpenAI deployment '{}' could not be used at {}: {}",
            endpoint.deployment_name(),
            endpoint.normalized_endpoint(),
            message,
        ));
    }

    let invalid_api_key = code_lc.contains("invalidapikey")
        || code_lc.contains("invalidsubscriptionkey")
        || message_lc.contains("api key")
        || message_lc.contains("subscription key")
        || message_lc.contains("access key")
        || message_lc.contains("invalid key")
        || message_lc.contains("incorrect key")
        || message_lc.contains("invalid subscription key");

    if auth.uses_api_key() && (status == 401 || invalid_api_key) {
        return AzureError::OpenAiInvalidApiKey {
            endpoint: endpoint_url,
            message,
        };
    }

    if status == 401
        || code_lc.contains("unauthorized")
        || code_lc.contains("authentication")
        || message_lc.contains("authentication")
        || message_lc.contains("unauthorized")
    {
        return AzureError::OpenAiAuthenticationFailed {
            endpoint: endpoint_url,
            message,
        };
    }

    if status == 403
        || code_lc.contains("forbidden")
        || code_lc.contains("permission")
        || code_lc.contains("accessdenied")
        || message_lc.contains("forbidden")
        || message_lc.contains("permission")
        || message_lc.contains("not authorized")
    {
        return AzureError::OpenAiPermissionDenied {
            endpoint: endpoint_url,
            message,
        };
    }

    if status == 408 || status == 504 {
        return AzureError::OpenAiTimeout(format!(
            "Azure OpenAI returned HTTP {status}: {message}"
        ));
    }

    if status == 429 {
        return AzureError::OpenAiApi {
            status,
            code: code.unwrap_or_else(|| "TooManyRequests".to_string()),
            message,
        };
    }

    AzureError::OpenAiApi {
        status,
        code: code.unwrap_or_else(|| "UnexpectedResponse".to_string()),
        message,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireChatCompletionResponse {
    id: String,
    model: String,
    choices: Vec<WireChatCompletionChoice>,
    usage: Option<WireChatCompletionUsage>,
}

#[derive(Debug, Deserialize)]
struct WireChatCompletionChoice {
    message: WireChatCompletionMessage,
}

#[derive(Debug, Deserialize)]
struct WireChatCompletionMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WireChatCompletionUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AzureAiProviderErrorClass;
    use azure_core::credentials::AccessToken;
    use typespec_client_core::date::OffsetDateTime;

    #[derive(Debug)]
    struct FakeCredential;

    #[async_trait::async_trait]
    impl TokenCredential for FakeCredential {
        async fn get_token(&self, _scopes: &[&str]) -> azure_core::Result<AccessToken> {
            Ok(AccessToken::new("fake-token", OffsetDateTime::now_utc()))
        }

        async fn clear_cache(&self) -> azure_core::Result<()> {
            Ok(())
        }
    }

    fn api_key_options(endpoint: &str) -> AzureOpenAiClientOptions {
        AzureOpenAiClientOptions {
            endpoint: endpoint.to_string(),
            deployment_name: "gpt-4o-mini".to_string(),
            cloud: AzureCloud::Commercial,
            auth: AzureOpenAiAuth::ApiKey(Secret::new("top-secret")),
        }
    }

    #[test]
    fn endpoint_normalization_builds_chat_completions_url() {
        let endpoint = AzureOpenAiEndpoint::new(
            "https://demo.openai.azure.com/",
            AzureCloud::Commercial,
            "gpt-4o-mini",
        )
        .unwrap();

        assert_eq!(
            endpoint.normalized_endpoint().as_str(),
            "https://demo.openai.azure.com/"
        );
        assert_eq!(
            endpoint.chat_completions_url().as_str(),
            "https://demo.openai.azure.com/openai/deployments/gpt-4o-mini/chat/completions?api-version=2024-10-21"
        );
    }

    #[test]
    fn endpoint_normalization_rejects_openai_path_input() {
        let error = AzureOpenAiEndpoint::new(
            "https://demo.openai.azure.com/openai/v1/",
            AzureCloud::Commercial,
            "gpt-4o-mini",
        )
        .unwrap_err();

        assert!(matches!(error, AzureError::InvalidOpenAiEndpoint { .. }));
    }

    #[test]
    fn endpoint_normalization_rejects_query_input() {
        let error = AzureOpenAiEndpoint::new(
            "https://demo.openai.azure.com/?api-version=2024-10-21",
            AzureCloud::Commercial,
            "gpt-4o-mini",
        )
        .unwrap_err();

        assert!(matches!(error, AzureError::InvalidOpenAiEndpoint { .. }));
    }

    #[test]
    fn endpoint_normalization_rejects_non_https() {
        let error = AzureOpenAiEndpoint::new(
            "http://demo.openai.azure.com",
            AzureCloud::Commercial,
            "gpt-4o-mini",
        )
        .unwrap_err();

        assert!(matches!(error, AzureError::InvalidOpenAiEndpoint { .. }));
    }

    #[test]
    fn endpoint_normalization_rejects_cloud_mismatch() {
        let error = AzureOpenAiEndpoint::new(
            "https://demo.openai.azure.us",
            AzureCloud::Commercial,
            "gpt-4o-mini",
        )
        .unwrap_err();

        assert!(matches!(error, AzureError::InvalidOpenAiEndpoint { .. }));
    }

    #[test]
    fn endpoint_normalization_rejects_unrelated_paths() {
        let error = AzureOpenAiEndpoint::new(
            "https://demo.openai.azure.com/custom/path",
            AzureCloud::Commercial,
            "gpt-4o-mini",
        )
        .unwrap_err();

        assert!(matches!(error, AzureError::InvalidOpenAiEndpoint { .. }));
    }

    #[test]
    fn auth_debug_redacts_api_key() {
        let rendered = format!("{:?}", AzureOpenAiAuth::ApiKey(Secret::new("secret-value")));

        assert!(!rendered.contains("secret-value"));
        assert!(rendered.contains("<redacted>"));
    }

    #[test]
    fn build_wire_request_serializes_json_schema_response_format() {
        let request = build_wire_request(AzureOpenAiChatCompletionsRequest {
            messages: vec![AzureOpenAiChatMessage {
                role: AzureOpenAiChatRole::User,
                content: "hello".to_string(),
            }],
            response_format: Some(AzureOpenAiResponseFormat::JsonSchema(
                AzureOpenAiResponseFormatJsonSchema {
                    name: "insights".to_string(),
                    description: Some("Structured response".to_string()),
                    schema: json!({ "type": "object" }),
                    strict: true,
                },
            )),
            temperature: Some(0.2),
            max_tokens: Some(128),
        });

        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["responseFormat"]["type"], "json_schema");
        assert_eq!(value["responseFormat"]["json_schema"]["name"], "insights");
    }

    #[test]
    fn map_openai_response_error_maps_permission_denied() {
        let endpoint = AzureOpenAiEndpoint::new(
            "https://demo.openai.azure.com",
            AzureCloud::Commercial,
            "gpt-4o-mini",
        )
        .unwrap();

        let error = map_openai_response_error(
            403,
            r#"{"error":{"code":"Forbidden","message":"Caller lacks RBAC."}}"#.to_string(),
            &endpoint,
            &AzureOpenAiAuth::DefaultAzureCredential,
        );

        assert!(matches!(error, AzureError::OpenAiPermissionDenied { .. }));
    }

    #[test]
    fn map_openai_response_error_maps_service_401_to_provider_authentication_failure() {
        let endpoint = AzureOpenAiEndpoint::new(
            "https://demo.openai.azure.com",
            AzureCloud::Commercial,
            "gpt-4o-mini",
        )
        .unwrap();

        let error = map_openai_response_error(
            401,
            r#"{"error":{"code":"Unauthorized","message":"Bearer token was rejected."}}"#
                .to_string(),
            &endpoint,
            &AzureOpenAiAuth::DefaultAzureCredential,
        );

        assert!(matches!(
            error,
            AzureError::OpenAiAuthenticationFailed { .. }
        ));
    }

    #[test]
    fn map_openai_response_error_maps_api_key_401_to_invalid_api_key() {
        let endpoint = AzureOpenAiEndpoint::new(
            "https://demo.openai.azure.com",
            AzureCloud::Commercial,
            "gpt-4o-mini",
        )
        .unwrap();

        let error = map_openai_response_error(
            401,
            r#"{"error":{"code":"Unauthorized","message":"Access denied due to invalid subscription key."}}"#.to_string(),
            &endpoint,
            &AzureOpenAiAuth::ApiKey(Secret::new("top-secret")),
        );

        assert!(matches!(error, AzureError::OpenAiInvalidApiKey { .. }));
    }

    #[test]
    fn map_openai_response_error_maps_deployment_failures_to_configuration() {
        let endpoint = AzureOpenAiEndpoint::new(
            "https://demo.openai.azure.com",
            AzureCloud::Commercial,
            "missing-deployment",
        )
        .unwrap();

        let error = map_openai_response_error(
            404,
            r#"{"error":{"code":"DeploymentNotFound","message":"The API deployment for this resource does not exist."}}"#.to_string(),
            &endpoint,
            &AzureOpenAiAuth::DefaultAzureCredential,
        );

        assert!(matches!(error, AzureError::OpenAiConfiguration(_)));
    }

    #[test]
    fn map_openai_response_error_preserves_unparseable_non_auth_error_bodies() {
        let endpoint = AzureOpenAiEndpoint::new(
            "https://demo.openai.azure.com",
            AzureCloud::Commercial,
            "gpt-4o-mini",
        )
        .unwrap();

        let error = map_openai_response_error(
            502,
            "<html>gateway failure</html>".to_string(),
            &endpoint,
            &AzureOpenAiAuth::DefaultAzureCredential,
        );

        match error {
            AzureError::OpenAiApi {
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
    fn map_openai_response_error_classifies_unparseable_api_key_401() {
        let endpoint = AzureOpenAiEndpoint::new(
            "https://demo.openai.azure.com",
            AzureCloud::Commercial,
            "gpt-4o-mini",
        )
        .unwrap();

        let error = map_openai_response_error(
            401,
            "<html>unauthorized</html>".to_string(),
            &endpoint,
            &AzureOpenAiAuth::ApiKey(Secret::new("top-secret")),
        );

        assert!(matches!(error, AzureError::OpenAiInvalidApiKey { .. }));
    }

    #[test]
    fn default_credential_options_follow_selected_cloud_authority() {
        let mut options = TokenCredentialOptions::default();
        options.set_authority_host(AzureCloud::UsGovernment.auth_endpoint().to_string());

        assert_eq!(
            options
                .authority_host()
                .unwrap()
                .as_str()
                .trim_end_matches('/'),
            AzureCloud::UsGovernment.auth_endpoint()
        );
    }

    #[test]
    fn client_construction_supports_bearer_auth_seam() {
        let client = AzureOpenAiClient::with_credential(
            AzureOpenAiClientOptions {
                endpoint: "https://demo.openai.azure.com".to_string(),
                deployment_name: "gpt-4o-mini".to_string(),
                cloud: AzureCloud::Commercial,
                auth: AzureOpenAiAuth::DefaultAzureCredential,
            },
            Arc::new(FakeCredential),
        )
        .unwrap();

        assert_eq!(client.endpoint().deployment_name(), "gpt-4o-mini");
    }

    #[test]
    fn client_construction_supports_api_key_mode() {
        let client =
            AzureOpenAiClient::new(api_key_options("https://demo.openai.azure.com")).unwrap();

        assert_eq!(
            client.endpoint().chat_completions_url().as_str(),
            "https://demo.openai.azure.com/openai/deployments/gpt-4o-mini/chat/completions?api-version=2024-10-21"
        );
    }

    #[test]
    fn json_schema_response_format_omits_null_description() {
        let request = build_wire_request(AzureOpenAiChatCompletionsRequest {
            messages: vec![AzureOpenAiChatMessage {
                role: AzureOpenAiChatRole::User,
                content: "test".to_string(),
            }],
            response_format: Some(AzureOpenAiResponseFormat::JsonSchema(
                AzureOpenAiResponseFormatJsonSchema {
                    name: "insights".to_string(),
                    description: None,
                    schema: json!({ "type": "object" }),
                    strict: true,
                },
            )),
            temperature: None,
            max_tokens: None,
        });

        let value = serde_json::to_value(&request).unwrap();
        let json_schema = &value["responseFormat"]["json_schema"];
        assert_eq!(json_schema["name"], "insights");
        assert_eq!(json_schema["strict"], true);
        assert!(
            json_schema.get("description").is_none(),
            "description should not appear in serialized output when None"
        );
    }

    #[test]
    fn json_schema_response_format_includes_description_when_set() {
        let request = build_wire_request(AzureOpenAiChatCompletionsRequest {
            messages: vec![AzureOpenAiChatMessage {
                role: AzureOpenAiChatRole::User,
                content: "test".to_string(),
            }],
            response_format: Some(AzureOpenAiResponseFormat::JsonSchema(
                AzureOpenAiResponseFormatJsonSchema {
                    name: "insights".to_string(),
                    description: Some("structured".to_string()),
                    schema: json!({ "type": "object" }),
                    strict: true,
                },
            )),
            temperature: None,
            max_tokens: None,
        });

        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(
            value["responseFormat"]["json_schema"]["description"],
            "structured"
        );
    }

    #[test]
    fn map_openai_response_error_classifies_http_408_as_timeout() {
        let endpoint = AzureOpenAiEndpoint::new(
            "https://demo.openai.azure.com",
            AzureCloud::Commercial,
            "gpt-4o-mini",
        )
        .unwrap();

        let error = map_openai_response_error(
            408,
            r#"{"error":{"code":"RequestTimeout","message":"The request timed out."}}"#.to_string(),
            &endpoint,
            &AzureOpenAiAuth::DefaultAzureCredential,
        );

        assert!(
            matches!(error, AzureError::OpenAiTimeout(_)),
            "HTTP 408 should map to OpenAiTimeout, got: {error:?}"
        );
        assert_eq!(
            error.ai_provider_error_class(),
            AzureAiProviderErrorClass::Timeout,
        );
    }

    #[test]
    fn map_openai_response_error_classifies_http_504_as_timeout() {
        let endpoint = AzureOpenAiEndpoint::new(
            "https://demo.openai.azure.com",
            AzureCloud::Commercial,
            "gpt-4o-mini",
        )
        .unwrap();

        let error = map_openai_response_error(
            504,
            r#"{"error":{"code":"GatewayTimeout","message":"upstream timed out"}}"#.to_string(),
            &endpoint,
            &AzureOpenAiAuth::DefaultAzureCredential,
        );

        assert!(
            matches!(error, AzureError::OpenAiTimeout(_)),
            "HTTP 504 should map to OpenAiTimeout, got: {error:?}"
        );
    }

    #[test]
    fn map_openai_response_error_classifies_http_429_as_rate_limited() {
        let endpoint = AzureOpenAiEndpoint::new(
            "https://demo.openai.azure.com",
            AzureCloud::Commercial,
            "gpt-4o-mini",
        )
        .unwrap();

        let error = map_openai_response_error(
            429,
            r#"{"error":{"code":"TooManyRequests","message":"Rate limit reached."}}"#.to_string(),
            &endpoint,
            &AzureOpenAiAuth::DefaultAzureCredential,
        );

        match error {
            AzureError::OpenAiApi { status, code, .. } => {
                assert_eq!(status, 429);
                assert_eq!(code, "TooManyRequests");
            }
            other => panic!("expected OpenAiApi for 429, got: {other:?}"),
        }
    }
}
