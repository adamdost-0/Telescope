//! AKS resource identity resolution.
//!
//! Bridges K8s kubeconfig context with the Azure management plane by resolving
//! the subscription ID, resource group, and cluster name for an AKS cluster.

use crate::types::AksResourceId;
use serde::Deserialize;
use telescope_core::store::ResourceStore;
use tracing::{debug, warn};

/// Preference keys used to store/retrieve Azure identity overrides.
pub const PREF_AZURE_SUBSCRIPTION: &str = "azure_subscription";
pub const PREF_AZURE_RESOURCE_GROUP: &str = "azure_resource_group";
pub const PREF_AZURE_CLUSTER_NAME: &str = "azure_cluster_name";

/// Status of saved AKS identity preferences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AksIdentityPreferenceStatus {
    Missing,
    Incomplete { missing_fields: Vec<&'static str> },
    Complete(AksResourceId),
}

/// Resolve the AKS resource identity for a given API server URL.
///
/// Resolution precedence:
/// 1. **Saved preferences** — manual overrides stored in `ResourceStore`
/// 2. **Azure CLI** — `az aks list` filtered by FQDN
/// 3. **Kubeconfig hints** — FQDN region extraction (partial)
///
/// Returns `None` for non-AKS clusters or when resolution fails entirely.
pub async fn resolve_aks_identity(
    server_url: &str,
    preferred_id: Option<AksResourceId>,
) -> Option<AksResourceId> {
    if !server_url.contains(".azmk8s.io") && !server_url.contains(".azmk8s.us") {
        return None;
    }

    // 1. Check saved preferences
    if let Some(id) = preferred_id {
        debug!("AKS identity resolved from saved preferences");
        return Some(id);
    }

    // 2. Try Azure CLI
    if let Some(id) = resolve_from_az_cli(server_url).await {
        debug!("AKS identity resolved via az CLI");
        return Some(id);
    }

    // 3. Kubeconfig / FQDN hints (partial — region only, no subscription/RG)
    debug!("AKS identity could not be fully resolved; az CLI may not be installed");
    None
}

/// Read saved AKS identity overrides from the resource store.
pub fn resolve_aks_identity_from_preferences(
    store: Option<&ResourceStore>,
) -> Option<AksResourceId> {
    match inspect_aks_identity_preferences(store) {
        AksIdentityPreferenceStatus::Complete(id) => Some(id),
        AksIdentityPreferenceStatus::Missing | AksIdentityPreferenceStatus::Incomplete { .. } => {
            None
        }
    }
}

/// Inspect saved AKS identity preference completeness.
pub fn inspect_aks_identity_preferences(
    store: Option<&ResourceStore>,
) -> AksIdentityPreferenceStatus {
    let Some(store) = store else {
        return AksIdentityPreferenceStatus::Missing;
    };

    let subscription = read_preference(store, PREF_AZURE_SUBSCRIPTION);
    let resource_group = read_preference(store, PREF_AZURE_RESOURCE_GROUP);
    let cluster_name = read_preference(store, PREF_AZURE_CLUSTER_NAME);

    let has_any = subscription.is_some() || resource_group.is_some() || cluster_name.is_some();
    let mut missing_fields = Vec::new();
    if subscription.is_none() {
        missing_fields.push("subscription");
    }
    if resource_group.is_none() {
        missing_fields.push("resource group");
    }
    if cluster_name.is_none() {
        missing_fields.push("cluster name");
    }

    if missing_fields.is_empty() {
        AksIdentityPreferenceStatus::Complete(AksResourceId {
            subscription_id: subscription.expect("subscription present when no fields missing"),
            resource_group: resource_group.expect("resource group present when no fields missing"),
            cluster_name: cluster_name.expect("cluster name present when no fields missing"),
        })
    } else if has_any {
        AksIdentityPreferenceStatus::Incomplete { missing_fields }
    } else {
        AksIdentityPreferenceStatus::Missing
    }
}

/// Create an actionable error when an AKS cluster cannot be mapped to an ARM resource.
pub fn unresolved_aks_identity_message(
    server_url: &str,
    preference_status: &AksIdentityPreferenceStatus,
) -> String {
    let cluster_hint = extract_fqdn(server_url).unwrap_or_else(|| server_url.to_string());
    let base = format!(
        "Connected cluster looks like AKS, but Telescope could not resolve its Azure resource ID for {cluster_hint}."
    );
    match preference_status {
        AksIdentityPreferenceStatus::Incomplete { missing_fields } => format!(
            "{base} Saved AKS identity settings are incomplete; add {} in Settings, or sign in with Azure CLI so `az aks list` can resolve the cluster.",
            format_field_list(missing_fields)
        ),
        AksIdentityPreferenceStatus::Missing | AksIdentityPreferenceStatus::Complete(_) => format!(
            "{base} Save Azure subscription, resource group, and cluster name in Settings, or sign in with Azure CLI so `az aks list` can resolve the cluster."
        ),
    }
}

/// Attempt resolution from user-saved preferences in the resource store.
fn read_preference(store: &ResourceStore, key: &str) -> Option<String> {
    store
        .get_preference(key)
        .ok()
        .flatten()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn format_field_list(fields: &[&'static str]) -> String {
    match fields {
        [] => String::new(),
        [single] => single.to_string(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let head = fields[..fields.len() - 1].join(", ");
            format!("{head}, and {}", fields[fields.len() - 1])
        }
    }
}

/// Row shape returned by `az aks list`.
#[derive(Deserialize)]
struct AzAksEntry {
    id: Option<String>,
    #[serde(alias = "resourceGroup")]
    resource_group: Option<String>,
    name: Option<String>,
    #[allow(dead_code)]
    fqdn: Option<String>,
}

/// Attempt resolution via the Azure CLI (`az aks list`).
///
/// Runs `az aks list -o json` and filters by the FQDN extracted from
/// `server_url`. Gracefully returns `None` when `az` is not installed.
async fn resolve_from_az_cli(server_url: &str) -> Option<AksResourceId> {
    let fqdn = extract_fqdn(server_url)?;

    let output = match tokio::process::Command::new("az")
        .args([
            "aks",
            "list",
            "--query",
            &format!("[?fqdn=='{fqdn}']"),
            "-o",
            "json",
        ])
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) => {
            debug!("az CLI not available: {e}");
            return None;
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("az aks list failed: {stderr}");
        return None;
    }

    let entries: Vec<AzAksEntry> = match serde_json::from_slice(&output.stdout) {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to parse az aks list output: {e}");
            return None;
        }
    };

    let entry = entries.into_iter().next()?;

    // The `id` field is the full ARM resource ID; extract subscription from it.
    let arm_id = entry.id?;
    let subscription_id = extract_subscription_from_arm_id(&arm_id)?;

    Some(AksResourceId {
        subscription_id,
        resource_group: entry.resource_group?,
        cluster_name: entry.name?,
    })
}

/// Extract the FQDN from a server URL.
/// `https://myaks-dns-abc123.hcp.eastus.azmk8s.io:443` → `myaks-dns-abc123.hcp.eastus.azmk8s.io`
fn extract_fqdn(server_url: &str) -> Option<String> {
    let url = server_url
        .strip_prefix("https://")
        .or_else(|| server_url.strip_prefix("http://"))?;
    let host = url.split(':').next()?;
    Some(host.to_string())
}

/// Extract subscription ID from an ARM resource ID.
/// `/subscriptions/SUB_ID/resourceGroups/...` → `SUB_ID`
fn extract_subscription_from_arm_id(arm_id: &str) -> Option<String> {
    let parts: Vec<&str> = arm_id.split('/').collect();
    for (i, part) in parts.iter().enumerate() {
        if part.eq_ignore_ascii_case("subscriptions") {
            return parts.get(i + 1).map(|s| s.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_fqdn_from_aks_url() {
        let url = "https://myaks-dns-abc123.hcp.eastus.azmk8s.io:443";
        assert_eq!(
            extract_fqdn(url).unwrap(),
            "myaks-dns-abc123.hcp.eastus.azmk8s.io"
        );
    }

    #[test]
    fn extract_fqdn_no_port() {
        let url = "https://myaks.hcp.westus2.azmk8s.io";
        assert_eq!(extract_fqdn(url).unwrap(), "myaks.hcp.westus2.azmk8s.io");
    }

    #[test]
    fn extract_fqdn_non_url_returns_none() {
        assert!(extract_fqdn("not-a-url").is_none());
    }

    #[test]
    fn extract_subscription_from_valid_arm_id() {
        let arm_id = "/subscriptions/abc-123/resourceGroups/my-rg/providers/Microsoft.ContainerService/managedClusters/my-aks";
        assert_eq!(extract_subscription_from_arm_id(arm_id).unwrap(), "abc-123");
    }

    #[test]
    fn extract_subscription_invalid_returns_none() {
        assert!(extract_subscription_from_arm_id("/something/else").is_none());
    }

    #[test]
    fn non_aks_url_returns_none() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(resolve_aks_identity("https://k8s.example.com:6443", None));
        assert!(result.is_none());
    }

    #[test]
    fn unresolved_identity_message_mentions_missing_fields_for_partial_preferences() {
        let message = unresolved_aks_identity_message(
            "https://demo.hcp.eastus.azmk8s.io:443",
            &AksIdentityPreferenceStatus::Incomplete {
                missing_fields: vec!["resource group", "cluster name"],
            },
        );

        assert!(message.contains("demo.hcp.eastus.azmk8s.io"));
        assert!(message.contains("resource group and cluster name"));
        assert!(message.contains("`az aks list`"));
    }

    #[test]
    fn unresolved_identity_message_mentions_settings_for_missing_preferences() {
        let message = unresolved_aks_identity_message(
            "https://demo.hcp.eastus.azmk8s.io:443",
            &AksIdentityPreferenceStatus::Missing,
        );

        assert!(message.contains("Save Azure subscription, resource group, and cluster name"));
    }
}
