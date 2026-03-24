//! AI Insights request, response, and settings contracts.

use serde::{Deserialize, Serialize};

/// Stable persisted preference keys for non-secret AI Insights settings.
pub struct AiInsightsSettingsKeys;

impl AiInsightsSettingsKeys {
    pub const AUTH_MODE: &str = "ai_insights_auth_mode";
    pub const CLOUD_PROFILE: &str = "ai_insights_cloud_profile";
    pub const ENDPOINT: &str = "ai_insights_endpoint";
    pub const DEPLOYMENT_NAME: &str = "ai_insights_deployment_name";
    pub const MODEL_NAME: &str = "ai_insights_model_name";
    pub const ALL: [&'static str; 5] = [
        Self::AUTH_MODE,
        Self::CLOUD_PROFILE,
        Self::ENDPOINT,
        Self::DEPLOYMENT_NAME,
        Self::MODEL_NAME,
    ];
}

/// User-selected authentication mode for AI Insights.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AiInsightsAuthMode {
    #[default]
    AzureLogin,
    ApiKey,
}

/// Selected Azure cloud profile for Azure OpenAI requests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AiInsightsCloudProfile {
    #[default]
    Commercial,
    UsGovernment,
    UsGovernmentSecret,
    UsGovernmentTopSecret,
}

/// Non-secret settings required to configure AI Insights.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsSettings {
    pub auth_mode: AiInsightsAuthMode,
    pub cloud_profile: AiInsightsCloudProfile,
    pub endpoint: String,
    pub deployment_name: String,
    #[serde(default)]
    pub model_name: Option<String>,
}

/// Scope visible to the model contract so namespace-limited requests stay explicit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum AiInsightsScope {
    Cluster,
    Namespace { namespace: String },
}

/// Engine-owned request contract for generating AI Insights.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsRequest {
    pub scope: AiInsightsScope,
    pub settings: AiInsightsSettings,
}

/// Stable redaction policy version for AI Insights context shaping.
pub const AI_INSIGHTS_REDACTION_POLICY_VERSION: &str = "v1";

/// Maximum workload summaries included in the shaped AI Insights context.
pub const AI_INSIGHTS_WORKLOAD_CAP: usize = 12;
/// Maximum pod summaries included in the shaped AI Insights context.
pub const AI_INSIGHTS_POD_CAP: usize = 12;
/// Maximum warning-event summaries included in the shaped AI Insights context.
pub const AI_INSIGHTS_EVENT_CAP: usize = 10;
/// Maximum node summaries included in the shaped AI Insights context.
pub const AI_INSIGHTS_NODE_CAP: usize = 8;
/// Maximum Helm release summaries included in the shaped AI Insights context.
pub const AI_INSIGHTS_HELM_RELEASE_CAP: usize = 10;

/// Allowlist-only context contract sent to later AI orchestration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsContext {
    pub scope: AiInsightsScope,
    pub redaction_policy_version: String,
    pub connection: AiInsightsConnectionSummary,
    pub workloads: AiInsightsCollection<AiInsightsWorkloadSummary>,
    pub pods: AiInsightsCollection<AiInsightsPodSummary>,
    pub events: AiInsightsCollection<AiInsightsEventSummary>,
    pub nodes: AiInsightsCollection<AiInsightsNodeSummary>,
    pub helm_releases: AiInsightsCollection<AiInsightsHelmReleaseSummary>,
    pub aks: Option<AiInsightsAksSummary>,
}

/// Curated connection summary included in the shaped AI Insights context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsConnectionSummary {
    pub status: AiInsightsConnectionStatus,
    pub message: Option<String>,
    pub resources_synced: Option<u32>,
    pub resources_total: Option<u32>,
}

/// Connection status values surfaced to AI Insights orchestration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AiInsightsConnectionStatus {
    Disconnected,
    Connecting,
    Syncing,
    Ready,
    Degraded,
    Error,
    Backoff,
}

/// Deterministically capped collection of curated summaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsCollection<T> {
    pub total_count: u32,
    pub items: Vec<T>,
}

/// Curated workload summary included in the shaped AI Insights context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsWorkloadSummary {
    pub kind: String,
    pub namespace: String,
    pub name: String,
    pub desired_replicas: Option<u32>,
    pub ready_replicas: Option<u32>,
    pub available_replicas: Option<u32>,
    pub updated_replicas: Option<u32>,
    pub unavailable_replicas: Option<u32>,
    pub issue: Option<String>,
}

/// Curated pod summary included in the shaped AI Insights context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsPodSummary {
    pub namespace: String,
    pub name: String,
    pub phase: Option<String>,
    pub ready_containers: u32,
    pub total_containers: u32,
    pub restart_count: u32,
    pub node_name: Option<String>,
    pub issue: Option<String>,
}

/// Curated warning-event summary included in the shaped AI Insights context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsEventSummary {
    pub namespace: String,
    pub regarding_kind: String,
    pub regarding_name: String,
    pub reason: String,
    pub message: String,
    pub count: u32,
    pub last_seen: Option<String>,
}

/// Curated node summary included in the shaped AI Insights context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsNodeSummary {
    pub name: String,
    pub ready: bool,
    pub unschedulable: bool,
    pub pressures: Vec<String>,
    pub kubelet_version: Option<String>,
}

/// Curated Helm release summary included in the shaped AI Insights context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsHelmReleaseSummary {
    pub namespace: String,
    pub name: String,
    pub chart: String,
    pub app_version: String,
    pub revision: i32,
    pub status: String,
}

/// Narrow AKS posture summary included when desktop ARM data is available.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsAksSummary {
    pub kubernetes_version: Option<String>,
    pub provisioning_state: Option<String>,
    pub power_state: Option<String>,
    pub dns_prefix: Option<String>,
    pub private_cluster: Option<bool>,
    pub local_accounts_disabled: Option<bool>,
    pub network_plugin: Option<String>,
    pub outbound_type: Option<String>,
}

/// Strict advisory-only response contract rendered by the UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsResponse {
    pub summary: String,
    pub risks: Vec<AiInsightsRisk>,
    pub observations: Vec<AiInsightsObservation>,
    pub recommendations: Vec<AiInsightsRecommendation>,
    pub references: Vec<AiInsightsReference>,
}

/// Risk item included in the structured AI Insights response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsRisk {
    pub title: String,
    pub detail: String,
    pub impact: AiInsightsRiskImpact,
}

/// Supported impact levels for AI Insights risks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AiInsightsRiskImpact {
    Low,
    Medium,
    High,
}

/// Observation item included in the structured AI Insights response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsObservation {
    pub area: String,
    pub detail: String,
}

/// Recommendation item included in the structured AI Insights response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsRecommendation {
    pub action: String,
    pub rationale: String,
    pub confidence: f64,
}

/// Resource reference included in the structured AI Insights response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsReference {
    pub kind: String,
    pub name: String,
    pub namespace: Option<String>,
}

/// Dev-only diagnostics metadata for Settings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsDevDiagnostics {
    pub prompt_version: Option<String>,
    pub redaction_policy_version: Option<String>,
    pub cloud_profile: AiInsightsCloudProfile,
    pub auth_mode: AiInsightsAuthMode,
    pub context_size: Option<AiInsightsContextSize>,
    pub schema_validation_failure: Option<AiInsightsSchemaValidationFailure>,
    pub provider_error_classification: Option<AiInsightsProviderErrorClass>,
}

/// Context-size metadata surfaced only in dev mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsContextSize {
    pub serialized_bytes: u64,
    pub resource_count: u32,
}

/// Schema-validation metadata surfaced only in dev mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiInsightsSchemaValidationFailure {
    pub path: Option<String>,
    pub message: String,
}

/// Classified provider failure metadata surfaced only in dev mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AiInsightsProviderErrorClass {
    Configuration,
    Credential,
    Authorization,
    Endpoint,
    Timeout,
    Network,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_serializes_namespace_scope_explicitly() {
        let request = AiInsightsRequest {
            scope: AiInsightsScope::Namespace {
                namespace: "payments".to_string(),
            },
            settings: AiInsightsSettings {
                auth_mode: AiInsightsAuthMode::ApiKey,
                cloud_profile: AiInsightsCloudProfile::UsGovernment,
                endpoint: "https://example.openai.azure.com".to_string(),
                deployment_name: "gpt-4o-mini".to_string(),
                model_name: Some("gpt-4o-mini".to_string()),
            },
        };

        let value = serde_json::to_value(&request).unwrap();

        assert_eq!(
            value,
            json!({
                "scope": {
                    "kind": "namespace",
                    "namespace": "payments"
                },
                "settings": {
                    "authMode": "apiKey",
                    "cloudProfile": "usGovernment",
                    "endpoint": "https://example.openai.azure.com",
                    "deploymentName": "gpt-4o-mini",
                    "modelName": "gpt-4o-mini"
                }
            })
        );
    }

    #[test]
    fn response_serializes_with_prd_shape() {
        let response = AiInsightsResponse {
            summary: "Cluster health is degraded by a single workload.".to_string(),
            risks: vec![AiInsightsRisk {
                title: "Crash looping deployment".to_string(),
                detail: "payments-api restarted 9 times in the last hour.".to_string(),
                impact: AiInsightsRiskImpact::High,
            }],
            observations: vec![AiInsightsObservation {
                area: "Workloads".to_string(),
                detail: "Replica availability is healthy outside the payments namespace."
                    .to_string(),
            }],
            recommendations: vec![AiInsightsRecommendation {
                action: "Inspect recent events for payments-api".to_string(),
                rationale: "Repeated restarts usually correlate with probe or config failures."
                    .to_string(),
                confidence: 0.92,
            }],
            references: vec![AiInsightsReference {
                kind: "Deployment".to_string(),
                name: "payments-api".to_string(),
                namespace: Some("payments".to_string()),
            }],
        };

        let value = serde_json::to_value(&response).unwrap();

        assert_eq!(
            value,
            json!({
                "summary": "Cluster health is degraded by a single workload.",
                "risks": [
                    {
                        "title": "Crash looping deployment",
                        "detail": "payments-api restarted 9 times in the last hour.",
                        "impact": "high"
                    }
                ],
                "observations": [
                    {
                        "area": "Workloads",
                        "detail": "Replica availability is healthy outside the payments namespace."
                    }
                ],
                "recommendations": [
                    {
                        "action": "Inspect recent events for payments-api",
                        "rationale": "Repeated restarts usually correlate with probe or config failures.",
                        "confidence": 0.92
                    }
                ],
                "references": [
                    {
                        "kind": "Deployment",
                        "name": "payments-api",
                        "namespace": "payments"
                    }
                ]
            })
        );
    }

    #[test]
    fn response_rejects_unknown_fields() {
        let invalid = json!({
            "summary": "ok",
            "risks": [],
            "observations": [],
            "recommendations": [],
            "references": [],
            "chat": "not allowed"
        });

        let result = serde_json::from_value::<AiInsightsResponse>(invalid);

        assert!(result.is_err());
    }

    #[test]
    fn diagnostics_roundtrip_uses_explicit_metadata_fields() {
        let diagnostics = AiInsightsDevDiagnostics {
            prompt_version: Some("2026-03-24".to_string()),
            redaction_policy_version: Some("v1".to_string()),
            cloud_profile: AiInsightsCloudProfile::Commercial,
            auth_mode: AiInsightsAuthMode::AzureLogin,
            context_size: Some(AiInsightsContextSize {
                serialized_bytes: 2048,
                resource_count: 17,
            }),
            schema_validation_failure: Some(AiInsightsSchemaValidationFailure {
                path: Some("recommendations[0].confidence".to_string()),
                message: "confidence must be between 0 and 1".to_string(),
            }),
            provider_error_classification: Some(AiInsightsProviderErrorClass::Authorization),
        };

        let json = serde_json::to_string(&diagnostics).unwrap();
        let roundtrip: AiInsightsDevDiagnostics = serde_json::from_str(&json).unwrap();

        assert_eq!(roundtrip, diagnostics);
    }

    #[test]
    fn context_serializes_with_explicit_scope_and_curated_categories() {
        let context = AiInsightsContext {
            scope: AiInsightsScope::Namespace {
                namespace: "payments".to_string(),
            },
            redaction_policy_version: AI_INSIGHTS_REDACTION_POLICY_VERSION.to_string(),
            connection: AiInsightsConnectionSummary {
                status: AiInsightsConnectionStatus::Ready,
                message: None,
                resources_synced: Some(29),
                resources_total: Some(29),
            },
            workloads: AiInsightsCollection {
                total_count: 1,
                items: vec![AiInsightsWorkloadSummary {
                    kind: "Deployment".to_string(),
                    namespace: "payments".to_string(),
                    name: "payments-api".to_string(),
                    desired_replicas: Some(3),
                    ready_replicas: Some(2),
                    available_replicas: Some(2),
                    updated_replicas: Some(3),
                    unavailable_replicas: Some(1),
                    issue: Some("1 replica is unavailable".to_string()),
                }],
            },
            pods: AiInsightsCollection {
                total_count: 0,
                items: Vec::new(),
            },
            events: AiInsightsCollection {
                total_count: 0,
                items: Vec::new(),
            },
            nodes: AiInsightsCollection {
                total_count: 0,
                items: Vec::new(),
            },
            helm_releases: AiInsightsCollection {
                total_count: 0,
                items: Vec::new(),
            },
            aks: None,
        };

        let value = serde_json::to_value(&context).unwrap();

        assert_eq!(value["scope"]["kind"], "namespace");
        assert_eq!(value["scope"]["namespace"], "payments");
        assert_eq!(value["redactionPolicyVersion"], "v1");
        assert!(value.get("workloads").is_some());
        assert!(value.get("pods").is_some());
        assert!(value.get("events").is_some());
        assert!(value.get("nodes").is_some());
        assert!(value.get("helmReleases").is_some());
        assert!(value.get("aks").is_some());
    }

    #[test]
    fn settings_keys_are_stable_and_complete() {
        assert_eq!(
            AiInsightsSettingsKeys::ALL,
            [
                "ai_insights_auth_mode",
                "ai_insights_cloud_profile",
                "ai_insights_endpoint",
                "ai_insights_deployment_name",
                "ai_insights_model_name",
            ]
        );
    }
}
