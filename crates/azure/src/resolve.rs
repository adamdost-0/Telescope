//! AKS resource identity resolution.
//!
//! Bridges K8s kubeconfig context with the Azure management plane by resolving
//! the subscription ID, resource group, and cluster name for an AKS cluster.

use crate::types::AksResourceId;
use serde::Deserialize;
use tracing::{debug, warn};

/// Preference keys used to store/retrieve Azure identity overrides.
pub const PREF_AZURE_SUBSCRIPTION: &str = "azure_subscription";
pub const PREF_AZURE_RESOURCE_GROUP: &str = "azure_resource_group";
pub const PREF_AZURE_CLUSTER_NAME: &str = "azure_cluster_name";

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
    store: Option<&telescope_core::store::ResourceStore>,
) -> Option<AksResourceId> {
    resolve_from_preferences(store)
}

/// Attempt resolution from user-saved preferences in the resource store.
fn resolve_from_preferences(
    store: Option<&telescope_core::store::ResourceStore>,
) -> Option<AksResourceId> {
    let store = store?;
    let sub = store.get_preference(PREF_AZURE_SUBSCRIPTION).ok()??;
    let rg = store.get_preference(PREF_AZURE_RESOURCE_GROUP).ok()??;
    let name = store.get_preference(PREF_AZURE_CLUSTER_NAME).ok()??;

    if sub.is_empty() || rg.is_empty() || name.is_empty() {
        return None;
    }

    Some(AksResourceId {
        subscription_id: sub,
        resource_group: rg,
        cluster_name: name,
    })
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
}
