//! Allowlist-only AI Insights context builder.

use std::cmp::Ordering;
use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;
use telescope_core::{ConnectionState, ResourceEntry, ResourceStore};
use tracing::warn;

use crate::helm::HelmRelease;
use crate::insights::{
    AiInsightsAksSummary, AiInsightsCollection, AiInsightsConnectionStatus,
    AiInsightsConnectionSummary, AiInsightsContext, AiInsightsEventSummary,
    AiInsightsHelmReleaseSummary, AiInsightsNodeSummary, AiInsightsPodSummary, AiInsightsScope,
    AiInsightsWorkloadSummary, AI_INSIGHTS_EVENT_CAP, AI_INSIGHTS_HELM_RELEASE_CAP,
    AI_INSIGHTS_NODE_CAP, AI_INSIGHTS_POD_CAP, AI_INSIGHTS_REDACTION_POLICY_VERSION,
    AI_INSIGHTS_WORKLOAD_CAP,
};

const REDACTED_SENSITIVE_VALUE: &str = "<redacted>";
const WORKLOAD_GVKS: &[(&str, &str)] = &[
    ("apps/v1/Deployment", "Deployment"),
    ("apps/v1/StatefulSet", "StatefulSet"),
    ("apps/v1/DaemonSet", "DaemonSet"),
];
const WARNING_EVENT_TYPE: &str = "warning";

/// Inputs required to shape an allowlist-only AI Insights context.
pub struct AiInsightsContextInput<'a> {
    pub scope: &'a AiInsightsScope,
    pub connection_state: &'a ConnectionState,
    pub store: &'a ResourceStore,
    pub helm_releases: &'a [HelmRelease],
    pub aks_summary: Option<&'a AiInsightsAksSummary>,
}

/// Build a deterministic, allowlist-only AI Insights context.
pub fn build_ai_insights_context(
    input: &AiInsightsContextInput<'_>,
) -> crate::Result<AiInsightsContext> {
    let namespace = scope_namespace(input.scope);

    Ok(AiInsightsContext {
        scope: input.scope.clone(),
        redaction_policy_version: AI_INSIGHTS_REDACTION_POLICY_VERSION.to_string(),
        connection: summarize_connection_state(input.connection_state),
        workloads: build_workload_collection(input.store, namespace)?,
        pods: build_pod_collection(input.store, namespace)?,
        events: build_event_collection(input.store, namespace)?,
        nodes: if namespace.is_some() {
            empty_collection()
        } else {
            build_node_collection(input.store)?
        },
        helm_releases: build_helm_release_collection(input.helm_releases, namespace),
        aks: if namespace.is_some() {
            None
        } else {
            input
                .aks_summary
                .map(sanitize_aks_summary)
                .filter(aks_summary_has_signal)
        },
    })
}

/// Serialize a shaped AI Insights context using stable field ordering.
pub fn serialize_ai_insights_context(context: &AiInsightsContext) -> crate::Result<String> {
    serde_json::to_string(context).map_err(|error| {
        crate::EngineError::Other(format!("Failed to serialize AI Insights context: {error}"))
    })
}

fn build_workload_collection(
    store: &ResourceStore,
    namespace: Option<&str>,
) -> crate::Result<AiInsightsCollection<AiInsightsWorkloadSummary>> {
    let mut items = Vec::new();

    for (gvk, kind) in WORKLOAD_GVKS {
        for entry in list_entries(store, gvk, namespace)? {
            let Some(value) = parse_entry_json(&entry) else {
                continue;
            };
            let (
                desired_replicas,
                ready_replicas,
                available_replicas,
                updated_replicas,
                unavailable_replicas,
            ) = workload_replica_counts(kind, &value);
            let issue =
                workload_issue(&value, desired_replicas, ready_replicas, available_replicas);

            items.push(AiInsightsWorkloadSummary {
                kind: (*kind).to_string(),
                namespace: sanitize_string(&entry.namespace),
                name: sanitize_string(&entry.name),
                desired_replicas,
                ready_replicas,
                available_replicas,
                updated_replicas,
                unavailable_replicas,
                issue,
            });
        }
    }

    items.sort_by(compare_workloads);
    let total_count = items.len() as u32;
    items.truncate(AI_INSIGHTS_WORKLOAD_CAP);

    Ok(AiInsightsCollection { total_count, items })
}

fn build_pod_collection(
    store: &ResourceStore,
    namespace: Option<&str>,
) -> crate::Result<AiInsightsCollection<AiInsightsPodSummary>> {
    let mut items = Vec::new();

    for entry in list_entries(store, "v1/Pod", namespace)? {
        let Some(value) = parse_entry_json(&entry) else {
            continue;
        };

        let container_statuses = value_at(&value, &["status", "containerStatuses"])
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let ready_containers = container_statuses
            .iter()
            .filter(|status| {
                value_at(status, &["ready"])
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            })
            .count() as u32;
        let restart_count = container_statuses
            .iter()
            .filter_map(|status| value_at(status, &["restartCount"]).and_then(Value::as_u64))
            .sum::<u64>() as u32;
        let total_containers = if container_statuses.is_empty() {
            value_at(&value, &["spec", "containers"])
                .and_then(Value::as_array)
                .map(|containers| containers.len() as u32)
                .unwrap_or(0)
        } else {
            container_statuses.len() as u32
        };

        items.push(AiInsightsPodSummary {
            namespace: sanitize_string(&entry.namespace),
            name: sanitize_string(&entry.name),
            phase: sanitize_optional_string(read_string(&value, &["status", "phase"])),
            ready_containers,
            total_containers,
            restart_count,
            node_name: sanitize_optional_string(read_string(&value, &["spec", "nodeName"])),
            issue: pod_issue(&value, ready_containers, total_containers, restart_count),
        });
    }

    items.sort_by(compare_pods);
    let total_count = items.len() as u32;
    items.truncate(AI_INSIGHTS_POD_CAP);

    Ok(AiInsightsCollection { total_count, items })
}

fn build_event_collection(
    store: &ResourceStore,
    namespace: Option<&str>,
) -> crate::Result<AiInsightsCollection<AiInsightsEventSummary>> {
    let mut items = Vec::new();

    for entry in list_entries(store, "v1/Event", namespace)? {
        let Some(value) = parse_entry_json(&entry) else {
            continue;
        };

        let event_type = read_string(&value, &["type"]).unwrap_or_default();
        if !event_type.eq_ignore_ascii_case(WARNING_EVENT_TYPE) {
            continue;
        }

        items.push(AiInsightsEventSummary {
            namespace: sanitize_string(&entry.namespace),
            regarding_kind: sanitize_string(
                read_string(&value, &["involvedObject", "kind"])
                    .as_deref()
                    .unwrap_or("Unknown"),
            ),
            regarding_name: sanitize_string(
                read_string(&value, &["involvedObject", "name"])
                    .as_deref()
                    .unwrap_or(&entry.name),
            ),
            reason: sanitize_string(
                read_string(&value, &["reason"])
                    .as_deref()
                    .unwrap_or("Unknown"),
            ),
            message: sanitize_string(read_string(&value, &["message"]).as_deref().unwrap_or("")),
            count: event_count(&value),
            last_seen: sanitize_optional_string(event_last_seen(&value)),
        });
    }

    items.sort_by(compare_events);
    let total_count = items.len() as u32;
    items.truncate(AI_INSIGHTS_EVENT_CAP);

    Ok(AiInsightsCollection { total_count, items })
}

fn build_node_collection(
    store: &ResourceStore,
) -> crate::Result<AiInsightsCollection<AiInsightsNodeSummary>> {
    let mut items = Vec::new();

    for entry in list_entries(store, "v1/Node", None)? {
        let Some(value) = parse_entry_json(&entry) else {
            continue;
        };

        let pressures = node_pressures(&value);
        items.push(AiInsightsNodeSummary {
            name: sanitize_string(&entry.name),
            ready: node_is_ready(&value),
            unschedulable: value_at(&value, &["spec", "unschedulable"])
                .and_then(Value::as_bool)
                .unwrap_or(false),
            pressures,
            kubelet_version: sanitize_optional_string(read_string(
                &value,
                &["status", "nodeInfo", "kubeletVersion"],
            )),
        });
    }

    items.sort_by(compare_nodes);
    let total_count = items.len() as u32;
    items.truncate(AI_INSIGHTS_NODE_CAP);

    Ok(AiInsightsCollection { total_count, items })
}

fn build_helm_release_collection(
    helm_releases: &[HelmRelease],
    namespace: Option<&str>,
) -> AiInsightsCollection<AiInsightsHelmReleaseSummary> {
    let mut items = helm_releases
        .iter()
        .filter(|release| {
            namespace.is_none_or(|scope_namespace| release.namespace == scope_namespace)
        })
        .map(|release| AiInsightsHelmReleaseSummary {
            namespace: sanitize_string(&release.namespace),
            name: sanitize_string(&release.name),
            chart: sanitize_string(&release.chart),
            app_version: sanitize_string(&release.app_version),
            revision: release.revision,
            status: sanitize_string(&release.status),
        })
        .collect::<Vec<_>>();

    items.sort_by(compare_helm_releases);
    let total_count = items.len() as u32;
    items.truncate(AI_INSIGHTS_HELM_RELEASE_CAP);

    AiInsightsCollection { total_count, items }
}

fn summarize_connection_state(state: &ConnectionState) -> AiInsightsConnectionSummary {
    match state {
        ConnectionState::Disconnected => AiInsightsConnectionSummary {
            status: AiInsightsConnectionStatus::Disconnected,
            message: None,
            resources_synced: None,
            resources_total: None,
        },
        ConnectionState::Connecting => AiInsightsConnectionSummary {
            status: AiInsightsConnectionStatus::Connecting,
            message: None,
            resources_synced: None,
            resources_total: None,
        },
        ConnectionState::Syncing {
            resources_synced,
            resources_total,
        } => AiInsightsConnectionSummary {
            status: AiInsightsConnectionStatus::Syncing,
            message: None,
            resources_synced: Some(*resources_synced),
            resources_total: *resources_total,
        },
        ConnectionState::Ready => AiInsightsConnectionSummary {
            status: AiInsightsConnectionStatus::Ready,
            message: None,
            resources_synced: None,
            resources_total: None,
        },
        ConnectionState::Degraded { message } => AiInsightsConnectionSummary {
            status: AiInsightsConnectionStatus::Degraded,
            message: Some(sanitize_string(message)),
            resources_synced: None,
            resources_total: None,
        },
        ConnectionState::Error { message } => AiInsightsConnectionSummary {
            status: AiInsightsConnectionStatus::Error,
            message: Some(sanitize_string(message)),
            resources_synced: None,
            resources_total: None,
        },
        ConnectionState::Backoff { attempt, wait } => AiInsightsConnectionSummary {
            status: AiInsightsConnectionStatus::Backoff,
            message: Some(sanitize_string(&format!(
                "Retry attempt {} in {}s",
                attempt,
                wait.as_secs()
            ))),
            resources_synced: None,
            resources_total: None,
        },
    }
}

fn sanitize_aks_summary(summary: &AiInsightsAksSummary) -> AiInsightsAksSummary {
    AiInsightsAksSummary {
        kubernetes_version: sanitize_optional_string(summary.kubernetes_version.clone()),
        provisioning_state: sanitize_optional_string(summary.provisioning_state.clone()),
        power_state: sanitize_optional_string(summary.power_state.clone()),
        dns_prefix: sanitize_optional_string(summary.dns_prefix.clone()),
        private_cluster: summary.private_cluster,
        local_accounts_disabled: summary.local_accounts_disabled,
        network_plugin: sanitize_optional_string(summary.network_plugin.clone()),
        outbound_type: sanitize_optional_string(summary.outbound_type.clone()),
    }
}

fn aks_summary_has_signal(summary: &AiInsightsAksSummary) -> bool {
    summary.kubernetes_version.is_some()
        || summary.provisioning_state.is_some()
        || summary.power_state.is_some()
        || summary.dns_prefix.is_some()
        || summary.private_cluster.is_some()
        || summary.local_accounts_disabled.is_some()
        || summary.network_plugin.is_some()
        || summary.outbound_type.is_some()
}

fn scope_namespace(scope: &AiInsightsScope) -> Option<&str> {
    match scope {
        AiInsightsScope::Cluster => None,
        AiInsightsScope::Namespace { namespace } => Some(namespace.as_str()),
    }
}

fn empty_collection<T>() -> AiInsightsCollection<T> {
    AiInsightsCollection {
        total_count: 0,
        items: Vec::new(),
    }
}

fn list_entries(
    store: &ResourceStore,
    gvk: &str,
    namespace: Option<&str>,
) -> crate::Result<Vec<ResourceEntry>> {
    store
        .list(gvk, namespace)
        .map_err(|error| crate::EngineError::Store(error.to_string()))
}

fn parse_entry_json(entry: &ResourceEntry) -> Option<Value> {
    serde_json::from_str(&entry.content)
        .map_err(|error| {
            warn!(
                gvk = %entry.gvk,
                namespace = %entry.namespace,
                name = %entry.name,
                error = %error,
                "Failed to parse cached resource for AI Insights context"
            );
            error
        })
        .ok()
}

fn workload_replica_counts(
    kind: &str,
    value: &Value,
) -> (
    Option<u32>,
    Option<u32>,
    Option<u32>,
    Option<u32>,
    Option<u32>,
) {
    match kind {
        "DaemonSet" => (
            read_u32(value, &["status", "desiredNumberScheduled"]),
            read_u32(value, &["status", "numberReady"]),
            read_u32(value, &["status", "numberAvailable"]),
            read_u32(value, &["status", "updatedNumberScheduled"]),
            read_u32(value, &["status", "numberUnavailable"]),
        ),
        _ => (
            read_u32(value, &["spec", "replicas"]),
            read_u32(value, &["status", "readyReplicas"]),
            read_u32(value, &["status", "availableReplicas"]),
            read_u32(value, &["status", "updatedReplicas"]),
            read_u32(value, &["status", "unavailableReplicas"]),
        ),
    }
}

fn workload_issue(
    value: &Value,
    desired_replicas: Option<u32>,
    ready_replicas: Option<u32>,
    available_replicas: Option<u32>,
) -> Option<String> {
    first_non_ready_condition(value_at(value, &["status", "conditions"]).and_then(Value::as_array))
        .or_else(|| {
            let desired = desired_replicas.unwrap_or(0);
            let available = available_replicas.or(ready_replicas).unwrap_or(0);
            (desired > available).then(|| {
                sanitize_string(&format!(
                    "{} of {} replicas are available",
                    available, desired
                ))
            })
        })
}

fn pod_issue(
    value: &Value,
    ready_containers: u32,
    total_containers: u32,
    restart_count: u32,
) -> Option<String> {
    if let Some(container_statuses) =
        value_at(value, &["status", "containerStatuses"]).and_then(Value::as_array)
    {
        for status in container_statuses {
            if let Some(waiting) = value_at(status, &["state", "waiting"]) {
                if let Some(issue) = state_reason_message(waiting) {
                    return Some(issue);
                }
            }
            if let Some(terminated) = value_at(status, &["state", "terminated"]) {
                if let Some(issue) = state_reason_message(terminated) {
                    return Some(issue);
                }
            }
        }
    }

    first_non_ready_condition(value_at(value, &["status", "conditions"]).and_then(Value::as_array))
        .or_else(|| {
            let phase = read_string(value, &["status", "phase"]);
            phase
                .as_deref()
                .filter(|phase| !matches!(*phase, "Running" | "Succeeded"))
                .map(|phase| sanitize_string(&format!("Pod phase is {phase}")))
        })
        .or_else(|| {
            (total_containers > 0 && ready_containers < total_containers).then(|| {
                sanitize_string(&format!(
                    "{} of {} containers are ready",
                    ready_containers, total_containers
                ))
            })
        })
        .or_else(|| {
            (restart_count > 0)
                .then(|| sanitize_string(&format!("Pod restarted {} times", restart_count)))
        })
}

fn event_count(value: &Value) -> u32 {
    read_u32(value, &["count"])
        .or_else(|| read_u32(value, &["series", "count"]))
        .unwrap_or(1)
}

fn event_last_seen(value: &Value) -> Option<String> {
    read_string(value, &["lastTimestamp"])
        .or_else(|| read_string(value, &["eventTime"]))
        .or_else(|| read_string(value, &["series", "lastObservedTime"]))
        .or_else(|| read_string(value, &["metadata", "creationTimestamp"]))
}

fn node_is_ready(value: &Value) -> bool {
    value_at(value, &["status", "conditions"])
        .and_then(Value::as_array)
        .and_then(|conditions| {
            conditions.iter().find_map(|condition| {
                (read_string(condition, &["type"]).as_deref() == Some("Ready"))
                    .then(|| read_string(condition, &["status"]))
                    .flatten()
            })
        })
        .is_some_and(|status| status == "True")
}

fn node_pressures(value: &Value) -> Vec<String> {
    let condition_priority = [
        "MemoryPressure",
        "DiskPressure",
        "PIDPressure",
        "NetworkUnavailable",
    ];

    condition_priority
        .iter()
        .filter_map(|condition_type| {
            let conditions =
                value_at(value, &["status", "conditions"]).and_then(Value::as_array)?;
            conditions.iter().find_map(|condition| {
                let current_type = read_string(condition, &["type"])?;
                let current_status = read_string(condition, &["status"])?;
                if current_type == *condition_type && current_status != "False" {
                    Some(sanitize_string(condition_type))
                } else {
                    None
                }
            })
        })
        .collect()
}

fn first_non_ready_condition(conditions: Option<&Vec<Value>>) -> Option<String> {
    let mut issues = conditions
        .into_iter()
        .flat_map(|conditions| conditions.iter())
        .filter_map(|condition| {
            let condition_type = read_string(condition, &["type"])?;
            let status = read_string(condition, &["status"])?;
            if status == "True" {
                return None;
            }

            let mut detail = read_string(condition, &["message"])
                .or_else(|| read_string(condition, &["reason"]))
                .unwrap_or_else(|| format!("Condition {condition_type} is {status}"));
            if detail.is_empty() {
                detail = format!("Condition {condition_type} is {status}");
            }

            Some((
                condition_priority(&condition_type),
                sanitize_string(&detail),
            ))
        })
        .collect::<Vec<_>>();

    issues.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    issues.into_iter().next().map(|(_, issue)| issue)
}

fn state_reason_message(state: &Value) -> Option<String> {
    read_string(state, &["message"])
        .or_else(|| read_string(state, &["reason"]))
        .map(|message| sanitize_string(&message))
}

fn condition_priority(condition_type: &str) -> u8 {
    match condition_type {
        "Ready" | "Available" => 0,
        "ReplicaFailure" | "Progressing" => 1,
        _ => 2,
    }
}

fn read_string(value: &Value, path: &[&str]) -> Option<String> {
    value_at(value, path)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn read_u32(value: &Value, path: &[&str]) -> Option<u32> {
    value_at(value, path)
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

fn sanitize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let sanitized = sanitize_string(&value);
        (!sanitized.is_empty()).then_some(sanitized)
    })
}

fn sanitize_string(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if contains_sensitive_content(trimmed) {
        REDACTED_SENSITIVE_VALUE.to_string()
    } else {
        trimmed.to_string()
    }
}

fn contains_sensitive_content(value: &str) -> bool {
    looks_like_jwt(value)
        || looks_like_opaque_token(value)
        || looks_like_kubeconfig(value)
        || looks_like_connection_string(value)
        || looks_like_service_account_credential(value)
}

fn looks_like_jwt(value: &str) -> bool {
    static JWT_RE: OnceLock<Regex> = OnceLock::new();
    JWT_RE
        .get_or_init(|| {
            Regex::new(r"(?i)\beyj[a-z0-9_-]+\.[a-z0-9._-]+\.[a-z0-9._-]+\b")
                .expect("JWT regex should compile")
        })
        .is_match(value)
}

fn looks_like_opaque_token(value: &str) -> bool {
    static BEARER_TOKEN_RE: OnceLock<Regex> = OnceLock::new();

    let trimmed = value.trim();
    if BEARER_TOKEN_RE
        .get_or_init(|| {
            Regex::new(r"(?i)bearer\s+[a-z0-9._\-/+=]{24,}")
                .expect("Bearer token regex should compile")
        })
        .is_match(trimmed)
    {
        return true;
    }

    if trimmed.len() < 32 || trimmed.contains(char::is_whitespace) {
        return false;
    }

    let has_lower = trimmed
        .chars()
        .any(|character| character.is_ascii_lowercase());
    let has_upper = trimmed
        .chars()
        .any(|character| character.is_ascii_uppercase());
    let has_digit = trimmed.chars().any(|character| character.is_ascii_digit());
    let has_separator = trimmed.contains('-') || trimmed.contains('_') || trimmed.contains('=');
    let only_token_chars = trimmed.chars().all(|character| {
        character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '/' | '+' | '=')
    });

    only_token_chars && has_lower && has_digit && (has_upper || has_separator)
}

fn looks_like_kubeconfig(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let markers = [
        "current-context",
        "clusters:",
        "contexts:",
        "users:",
        "certificate-authority-data",
        "client-certificate-data",
        "client-key-data",
        "kind: config",
        "\"clusters\"",
        "\"contexts\"",
        "\"users\"",
    ];

    markers
        .iter()
        .filter(|marker| lower.contains(**marker))
        .count()
        >= 3
}

fn looks_like_connection_string(value: &str) -> bool {
    static URI_WITH_AUTH_RE: OnceLock<Regex> = OnceLock::new();

    let lower = value.to_ascii_lowercase();
    let has_semicolon_connection_parts = (lower.contains("accountkey=")
        || lower.contains("sharedaccesskey=")
        || lower.contains("sharedaccesssignature=")
        || lower.contains("defaultendpointsprotocol=")
        || lower.contains("endpointsuffix=")
        || lower.contains("user id=")
        || lower.contains("uid=")
        || lower.contains("password="))
        && value.contains(';');

    has_semicolon_connection_parts
        || URI_WITH_AUTH_RE
            .get_or_init(|| {
                Regex::new(r"(?i)\b[a-z][a-z0-9+.-]*://[^\s/:]+:[^\s@]+@[^\s]+")
                    .expect("Connection-string regex should compile")
            })
            .is_match(value)
}

fn looks_like_service_account_credential(value: &str) -> bool {
    static SERVICE_ACCOUNT_PRINCIPAL_RE: OnceLock<Regex> = OnceLock::new();

    let lower = value.to_ascii_lowercase();
    lower.contains("/var/run/secrets/kubernetes.io/serviceaccount")
        || lower.contains("kubernetes.io/service-account-token")
        || SERVICE_ACCOUNT_PRINCIPAL_RE
            .get_or_init(|| {
                Regex::new(r"(?i)system:serviceaccount:[a-z0-9.-]+:[a-z0-9.-]+")
                    .expect("Service-account regex should compile")
            })
            .is_match(value)
}

fn compare_workloads(
    left: &AiInsightsWorkloadSummary,
    right: &AiInsightsWorkloadSummary,
) -> Ordering {
    workload_severity(left)
        .cmp(&workload_severity(right))
        .then_with(|| unavailable_count(right).cmp(&unavailable_count(left)))
        .then_with(|| left.namespace.cmp(&right.namespace))
        .then_with(|| left.kind.cmp(&right.kind))
        .then_with(|| left.name.cmp(&right.name))
}

fn compare_pods(left: &AiInsightsPodSummary, right: &AiInsightsPodSummary) -> Ordering {
    pod_severity(left)
        .cmp(&pod_severity(right))
        .then_with(|| right.restart_count.cmp(&left.restart_count))
        .then_with(|| left.namespace.cmp(&right.namespace))
        .then_with(|| left.name.cmp(&right.name))
}

fn compare_events(left: &AiInsightsEventSummary, right: &AiInsightsEventSummary) -> Ordering {
    right
        .count
        .cmp(&left.count)
        .then_with(|| right.last_seen.cmp(&left.last_seen))
        .then_with(|| left.namespace.cmp(&right.namespace))
        .then_with(|| left.regarding_kind.cmp(&right.regarding_kind))
        .then_with(|| left.regarding_name.cmp(&right.regarding_name))
        .then_with(|| left.reason.cmp(&right.reason))
        .then_with(|| left.message.cmp(&right.message))
}

fn compare_nodes(left: &AiInsightsNodeSummary, right: &AiInsightsNodeSummary) -> Ordering {
    node_severity(left)
        .cmp(&node_severity(right))
        .then_with(|| right.pressures.len().cmp(&left.pressures.len()))
        .then_with(|| right.unschedulable.cmp(&left.unschedulable))
        .then_with(|| left.name.cmp(&right.name))
}

fn compare_helm_releases(
    left: &AiInsightsHelmReleaseSummary,
    right: &AiInsightsHelmReleaseSummary,
) -> Ordering {
    helm_severity(left)
        .cmp(&helm_severity(right))
        .then_with(|| left.namespace.cmp(&right.namespace))
        .then_with(|| left.name.cmp(&right.name))
        .then_with(|| right.revision.cmp(&left.revision))
}

fn workload_severity(summary: &AiInsightsWorkloadSummary) -> u8 {
    if summary.issue.is_some() {
        0
    } else if unavailable_count(summary) > 0 {
        1
    } else {
        2
    }
}

fn pod_severity(summary: &AiInsightsPodSummary) -> u8 {
    if summary.issue.is_some() {
        0
    } else if summary.restart_count > 0 {
        1
    } else if summary.total_containers > 0 && summary.ready_containers < summary.total_containers {
        2
    } else {
        3
    }
}

fn node_severity(summary: &AiInsightsNodeSummary) -> u8 {
    if !summary.ready {
        0
    } else if !summary.pressures.is_empty() {
        1
    } else if summary.unschedulable {
        2
    } else {
        3
    }
}

fn helm_severity(summary: &AiInsightsHelmReleaseSummary) -> u8 {
    let status = summary.status.to_ascii_lowercase();
    if status.contains("failed") || status.contains("pending") {
        0
    } else if status.contains("uninstall") || status.contains("superseded") {
        1
    } else {
        2
    }
}

fn unavailable_count(summary: &AiInsightsWorkloadSummary) -> u32 {
    summary.unavailable_replicas.unwrap_or_else(|| {
        let desired = summary.desired_replicas.unwrap_or(0);
        let available = summary
            .available_replicas
            .or(summary.ready_replicas)
            .unwrap_or(0);
        desired.saturating_sub(available)
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;

    use super::*;
    use telescope_core::{now_rfc3339, ResourceEntry};

    fn insert_entry(store: &ResourceStore, gvk: &str, namespace: &str, name: &str, content: Value) {
        store
            .upsert(&ResourceEntry {
                gvk: gvk.to_string(),
                namespace: namespace.to_string(),
                name: name.to_string(),
                resource_version: "1".to_string(),
                content: serde_json::to_string(&content).expect("entry JSON should serialize"),
                updated_at: now_rfc3339(),
            })
            .expect("entry should insert");
    }

    fn empty_input<'a>(
        scope: &'a AiInsightsScope,
        store: &'a ResourceStore,
        helm_releases: &'a [HelmRelease],
        aks_summary: Option<&'a AiInsightsAksSummary>,
    ) -> AiInsightsContextInput<'a> {
        AiInsightsContextInput {
            scope,
            connection_state: &ConnectionState::Ready,
            store,
            helm_releases,
            aks_summary,
        }
    }

    #[test]
    fn insights_context_enforces_caps_and_stable_ordering() {
        let store = ResourceStore::open(":memory:").expect("store should open");

        insert_entry(
            &store,
            "apps/v1/Deployment",
            "payments",
            "zz-problem",
            json!({
                "spec": { "replicas": 4 },
                "status": {
                    "readyReplicas": 1,
                    "availableReplicas": 1,
                    "updatedReplicas": 4,
                    "unavailableReplicas": 3,
                    "conditions": [{ "type": "Available", "status": "False", "message": "Minimum replicas unavailable" }]
                }
            }),
        );

        for index in 0..13 {
            insert_entry(
                &store,
                "apps/v1/Deployment",
                "payments",
                &format!("svc-{index:02}"),
                json!({
                    "spec": { "replicas": 2 },
                    "status": {
                        "readyReplicas": 2,
                        "availableReplicas": 2,
                        "updatedReplicas": 2,
                        "unavailableReplicas": 0,
                        "conditions": [{ "type": "Available", "status": "True" }]
                    }
                }),
            );
        }

        let scope = AiInsightsScope::Cluster;
        let first = build_ai_insights_context(&empty_input(&scope, &store, &[], None)).unwrap();
        let second = build_ai_insights_context(&empty_input(&scope, &store, &[], None)).unwrap();

        assert_eq!(first.workloads.total_count, 14);
        assert_eq!(first.workloads.items.len(), AI_INSIGHTS_WORKLOAD_CAP);
        assert_eq!(first.workloads.items.first().unwrap().name, "zz-problem");
        assert_eq!(first.workloads.items[1].name, "svc-00");
        assert_eq!(
            serialize_ai_insights_context(&first).unwrap(),
            serialize_ai_insights_context(&second).unwrap()
        );
    }

    #[test]
    fn insights_context_is_allowlist_only_and_omits_raw_resource_bodies() {
        let store = ResourceStore::open(":memory:").expect("store should open");

        insert_entry(
            &store,
            "v1/Pod",
            "payments",
            "payments-api-0",
            json!({
                "metadata": {
                    "annotations": { "debug-note": "top-secret" },
                    "managedFields": [{ "manager": "kubectl" }]
                },
                "spec": {
                    "nodeName": "node-a",
                    "containers": [{
                        "name": "api",
                        "env": [{ "name": "DATABASE_PASSWORD", "value": "super-secret-123" }]
                    }]
                },
                "status": {
                    "phase": "Running",
                    "containerStatuses": [{ "ready": true, "restartCount": 0 }]
                }
            }),
        );
        insert_entry(
            &store,
            "v1/Secret",
            "payments",
            "db-creds",
            json!({
                "data": { "password": "super-secret-456" },
                "stringData": { "token": "plain-secret" }
            }),
        );

        let scope = AiInsightsScope::Cluster;
        let context = build_ai_insights_context(&empty_input(&scope, &store, &[], None)).unwrap();
        let serialized = serialize_ai_insights_context(&context).unwrap();

        assert!(serialized.contains("payments-api-0"));
        assert!(!serialized.contains("super-secret-123"));
        assert!(!serialized.contains("super-secret-456"));
        assert!(!serialized.contains("plain-secret"));
        assert!(!serialized.contains("annotations"));
        assert!(!serialized.contains("managedFields"));
        assert!(!serialized.contains("env"));
        assert!(!serialized.contains("stringData"));
    }

    #[test]
    fn insights_context_redacts_tokens_kubeconfigs_connection_strings_and_service_accounts() {
        let store = ResourceStore::open(":memory:").expect("store should open");

        insert_entry(
            &store,
            "v1/Event",
            "payments",
            "warning-token",
            json!({
                "type": "Warning",
                "reason": "BackOff",
                "message": "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.signature failed",
                "count": 5,
                "lastTimestamp": "2026-03-24T10:00:00Z",
                "involvedObject": { "kind": "Pod", "name": "payments-api-0" }
            }),
        );
        insert_entry(
            &store,
            "v1/Event",
            "payments",
            "warning-kubeconfig",
            json!({
                "type": "Warning",
                "reason": "ConfigError",
                "message": "apiVersion: v1\nkind: Config\nclusters:\n- name: prod\ncontexts:\n- name: prod\nusers:\n- name: prod\ncurrent-context: prod",
                "count": 3,
                "lastTimestamp": "2026-03-24T09:00:00Z",
                "involvedObject": { "kind": "Pod", "name": "payments-api-1" }
            }),
        );

        let helm_releases = vec![HelmRelease {
            name: "billing".to_string(),
            namespace: "payments".to_string(),
            chart: "postgres://admin:password@db.internal:5432/app".to_string(),
            app_version: "1.0.0".to_string(),
            revision: 7,
            status: "failed".to_string(),
            updated: "2026-03-24T08:00:00Z".to_string(),
        }];
        let aks_summary = AiInsightsAksSummary {
            kubernetes_version: Some("1.30.4".to_string()),
            provisioning_state: Some("Succeeded".to_string()),
            power_state: Some("Running".to_string()),
            dns_prefix: Some("system:serviceaccount:payments:builder".to_string()),
            private_cluster: Some(true),
            local_accounts_disabled: Some(true),
            network_plugin: Some("azure".to_string()),
            outbound_type: Some("loadBalancer".to_string()),
        };
        let scope = AiInsightsScope::Cluster;

        let context = build_ai_insights_context(&empty_input(
            &scope,
            &store,
            &helm_releases,
            Some(&aks_summary),
        ))
        .unwrap();

        assert_eq!(context.events.items[0].message, REDACTED_SENSITIVE_VALUE);
        assert_eq!(context.events.items[1].message, REDACTED_SENSITIVE_VALUE);
        assert_eq!(
            context.helm_releases.items[0].chart,
            REDACTED_SENSITIVE_VALUE
        );
        assert_eq!(
            context.aks.unwrap().dns_prefix,
            Some(REDACTED_SENSITIVE_VALUE.to_string())
        );
    }

    #[test]
    fn insights_context_makes_namespace_scope_explicit_and_limits_cluster_only_sections() {
        let store = ResourceStore::open(":memory:").expect("store should open");

        insert_entry(
            &store,
            "apps/v1/Deployment",
            "payments",
            "payments-api",
            json!({
                "spec": { "replicas": 2 },
                "status": { "readyReplicas": 2, "availableReplicas": 2, "updatedReplicas": 2 }
            }),
        );
        insert_entry(
            &store,
            "apps/v1/Deployment",
            "default",
            "ignored-api",
            json!({
                "spec": { "replicas": 2 },
                "status": { "readyReplicas": 2, "availableReplicas": 2, "updatedReplicas": 2 }
            }),
        );
        insert_entry(
            &store,
            "v1/Node",
            "",
            "cluster-node-1",
            json!({
                "spec": { "unschedulable": false },
                "status": {
                    "nodeInfo": { "kubeletVersion": "1.30.4" },
                    "conditions": [{ "type": "Ready", "status": "True" }]
                }
            }),
        );

        let helm_releases = vec![
            HelmRelease {
                name: "payments-release".to_string(),
                namespace: "payments".to_string(),
                chart: "payments-1.0.0".to_string(),
                app_version: "1.0.0".to_string(),
                revision: 3,
                status: "deployed".to_string(),
                updated: "2026-03-24T08:00:00Z".to_string(),
            },
            HelmRelease {
                name: "default-release".to_string(),
                namespace: "default".to_string(),
                chart: "default-1.0.0".to_string(),
                app_version: "1.0.0".to_string(),
                revision: 2,
                status: "deployed".to_string(),
                updated: "2026-03-24T07:00:00Z".to_string(),
            },
        ];
        let aks_summary = AiInsightsAksSummary {
            kubernetes_version: Some("1.30.4".to_string()),
            provisioning_state: Some("Succeeded".to_string()),
            power_state: Some("Running".to_string()),
            dns_prefix: Some("aks-prod".to_string()),
            private_cluster: Some(false),
            local_accounts_disabled: Some(true),
            network_plugin: Some("azure".to_string()),
            outbound_type: Some("loadBalancer".to_string()),
        };
        let scope = AiInsightsScope::Namespace {
            namespace: "payments".to_string(),
        };
        let backoff = ConnectionState::Backoff {
            attempt: 2,
            wait: Duration::from_secs(4),
        };
        let input = AiInsightsContextInput {
            scope: &scope,
            connection_state: &backoff,
            store: &store,
            helm_releases: &helm_releases,
            aks_summary: Some(&aks_summary),
        };

        let context = build_ai_insights_context(&input).unwrap();

        assert_eq!(context.scope, scope);
        assert_eq!(
            context.connection.status,
            AiInsightsConnectionStatus::Backoff
        );
        assert_eq!(context.workloads.total_count, 1);
        assert_eq!(context.workloads.items[0].name, "payments-api");
        assert_eq!(context.helm_releases.total_count, 1);
        assert_eq!(context.helm_releases.items[0].name, "payments-release");
        assert_eq!(context.nodes.total_count, 0);
        assert!(context.nodes.items.is_empty());
        assert!(context.aks.is_none());
    }

    #[test]
    fn insights_context_namespace_scope_excludes_cross_namespace_pods_and_events() {
        let store = ResourceStore::open(":memory:").expect("store should open");

        // Pod in target namespace
        insert_entry(
            &store,
            "v1/Pod",
            "payments",
            "payments-api",
            json!({
                "spec": { "containers": [{ "name": "api" }] },
                "status": { "phase": "Running", "containerStatuses": [{ "ready": true, "restartCount": 0 }] }
            }),
        );
        // Pod in different namespace - should be excluded
        insert_entry(
            &store,
            "v1/Pod",
            "monitoring",
            "prometheus-0",
            json!({
                "spec": { "containers": [{ "name": "prom" }] },
                "status": { "phase": "Running", "containerStatuses": [{ "ready": true, "restartCount": 0 }] }
            }),
        );
        // Warning event in target namespace
        insert_entry(
            &store,
            "v1/Event",
            "payments",
            "warning-payments",
            json!({
                "type": "Warning",
                "reason": "BackOff",
                "message": "Back-off restarting failed container",
                "count": 3,
                "lastTimestamp": "2026-03-24T10:00:00Z",
                "involvedObject": { "kind": "Pod", "name": "payments-api" }
            }),
        );
        // Warning event in different namespace - should be excluded
        insert_entry(
            &store,
            "v1/Event",
            "monitoring",
            "warning-monitoring",
            json!({
                "type": "Warning",
                "reason": "Unhealthy",
                "message": "Readiness probe failed",
                "count": 1,
                "lastTimestamp": "2026-03-24T09:00:00Z",
                "involvedObject": { "kind": "Pod", "name": "prometheus-0" }
            }),
        );

        let scope = AiInsightsScope::Namespace {
            namespace: "payments".to_string(),
        };
        let context = build_ai_insights_context(&empty_input(&scope, &store, &[], None)).unwrap();

        assert_eq!(context.pods.total_count, 1);
        assert_eq!(context.pods.items[0].name, "payments-api");
        assert_eq!(context.events.total_count, 1);
        assert_eq!(context.events.items[0].regarding_name, "payments-api");
    }

    #[test]
    fn insights_context_enforces_pod_cap() {
        let store = ResourceStore::open(":memory:").expect("store should open");

        for i in 0..(AI_INSIGHTS_POD_CAP + 5) {
            insert_entry(
                &store,
                "v1/Pod",
                "default",
                &format!("pod-{i:02}"),
                json!({
                    "spec": { "containers": [{ "name": "main" }] },
                    "status": { "phase": "Running", "containerStatuses": [{ "ready": true, "restartCount": 0 }] }
                }),
            );
        }

        let scope = AiInsightsScope::Cluster;
        let context = build_ai_insights_context(&empty_input(&scope, &store, &[], None)).unwrap();

        assert_eq!(context.pods.total_count as usize, AI_INSIGHTS_POD_CAP + 5);
        assert_eq!(context.pods.items.len(), AI_INSIGHTS_POD_CAP);
    }

    #[test]
    fn insights_context_enforces_event_cap() {
        let store = ResourceStore::open(":memory:").expect("store should open");

        for i in 0..(AI_INSIGHTS_EVENT_CAP + 5) {
            insert_entry(
                &store,
                "v1/Event",
                "default",
                &format!("event-{i:02}"),
                json!({
                    "type": "Warning",
                    "reason": "BackOff",
                    "message": format!("event message {i}"),
                    "count": i + 1,
                    "lastTimestamp": format!("2026-03-24T{:02}:00:00Z", i % 24),
                    "involvedObject": { "kind": "Pod", "name": format!("pod-{i:02}") }
                }),
            );
        }

        let scope = AiInsightsScope::Cluster;
        let context = build_ai_insights_context(&empty_input(&scope, &store, &[], None)).unwrap();

        assert_eq!(
            context.events.total_count as usize,
            AI_INSIGHTS_EVENT_CAP + 5
        );
        assert_eq!(context.events.items.len(), AI_INSIGHTS_EVENT_CAP);
    }

    #[test]
    fn insights_context_enforces_node_cap() {
        let store = ResourceStore::open(":memory:").expect("store should open");

        for i in 0..(AI_INSIGHTS_NODE_CAP + 3) {
            insert_entry(
                &store,
                "v1/Node",
                "",
                &format!("node-{i:02}"),
                json!({
                    "spec": { "unschedulable": false },
                    "status": {
                        "nodeInfo": { "kubeletVersion": "1.30.4" },
                        "conditions": [{ "type": "Ready", "status": "True" }]
                    }
                }),
            );
        }

        let scope = AiInsightsScope::Cluster;
        let context = build_ai_insights_context(&empty_input(&scope, &store, &[], None)).unwrap();

        assert_eq!(context.nodes.total_count as usize, AI_INSIGHTS_NODE_CAP + 3);
        assert_eq!(context.nodes.items.len(), AI_INSIGHTS_NODE_CAP);
    }

    #[test]
    fn insights_context_enforces_helm_release_cap() {
        let mut releases = Vec::new();
        for i in 0..(AI_INSIGHTS_HELM_RELEASE_CAP + 4) {
            releases.push(HelmRelease {
                name: format!("release-{i:02}"),
                namespace: "default".to_string(),
                chart: format!("chart-{i}"),
                app_version: "1.0.0".to_string(),
                revision: (i + 1) as i32,
                status: "deployed".to_string(),
                updated: format!("2026-03-24T{:02}:00:00Z", i % 24),
            });
        }

        let store = ResourceStore::open(":memory:").expect("store should open");
        let scope = AiInsightsScope::Cluster;
        let context =
            build_ai_insights_context(&empty_input(&scope, &store, &releases, None)).unwrap();

        assert_eq!(
            context.helm_releases.total_count as usize,
            AI_INSIGHTS_HELM_RELEASE_CAP + 4
        );
        assert_eq!(
            context.helm_releases.items.len(),
            AI_INSIGHTS_HELM_RELEASE_CAP
        );
    }
}
