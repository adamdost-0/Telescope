use serde::{Deserialize, Serialize};

use crate::{AksResourceId, ArmClient, Result};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PowerState {
    pub code: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AksNodePool {
    pub name: String,
    pub vm_size: Option<String>,
    pub count: Option<i32>,
    pub os_type: Option<String>,
    pub os_disk_size_gb: Option<i32>,
    pub mode: Option<String>,
    pub orchestrator_version: Option<String>,
    pub enable_auto_scaling: Option<bool>,
    pub min_count: Option<i32>,
    pub max_count: Option<i32>,
    pub availability_zones: Option<Vec<String>>,
    pub node_labels: Option<serde_json::Value>,
    pub node_taints: Option<Vec<String>>,
    pub provisioning_state: Option<String>,
    pub power_state: Option<PowerState>,
    pub max_pods: Option<i32>,
    pub node_image_version: Option<String>,
    pub vnet_subnet_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AksClusterSku {
    pub name: Option<String>,
    pub tier: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AksNetworkProfile {
    pub network_plugin: Option<String>,
    pub network_policy: Option<String>,
    pub service_cidr: Option<String>,
    pub pod_cidr: Option<String>,
    pub dns_service_ip: Option<String>,
    pub outbound_type: Option<String>,
    pub load_balancer_sku: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AksApiServerAccessProfile {
    pub authorized_ip_ranges: Option<Vec<String>>,
    pub enable_private_cluster: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AksIdentityProfile {
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub principal_id: Option<String>,
    pub tenant_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AksAutoUpgradeProfile {
    pub upgrade_channel: Option<String>,
    pub node_os_upgrade_channel: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AksOidcIssuerProfile {
    pub enabled: Option<bool>,
    pub issuer_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AksWorkloadIdentityProfile {
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AksSecurityProfile {
    pub workload_identity: Option<AksWorkloadIdentityProfile>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AksClusterDetail {
    pub kubernetes_version: Option<String>,
    pub provisioning_state: Option<String>,
    pub power_state: Option<PowerState>,
    pub fqdn: Option<String>,
    pub dns_prefix: Option<String>,
    pub sku: Option<AksClusterSku>,
    pub network_profile: Option<AksNetworkProfile>,
    pub api_server_access_profile: Option<AksApiServerAccessProfile>,
    pub identity: Option<AksIdentityProfile>,
    pub addon_profiles: Option<serde_json::Map<String, serde_json::Value>>,
    pub auto_upgrade_profile: Option<AksAutoUpgradeProfile>,
    pub oidc_issuer_profile: Option<AksOidcIssuerProfile>,
    pub security_profile: Option<AksSecurityProfile>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ManagedClusterEnvelope {
    pub properties: AksClusterDetail,
    pub sku: Option<AksClusterSku>,
    pub identity: Option<AksIdentityProfile>,
}

pub async fn get_cluster(
    client: &ArmClient,
    resource_id: &AksResourceId,
) -> Result<AksClusterDetail> {
    let response: ManagedClusterEnvelope = client.get(&resource_id.arm_path()).await?;
    let mut detail = response.properties;
    if detail.sku.is_none() {
        detail.sku = response.sku;
    }
    if detail.identity.is_none() {
        detail.identity = response.identity;
    }
    Ok(detail)
}

pub async fn list_node_pools(
    client: &ArmClient,
    resource_id: &AksResourceId,
) -> Result<Vec<AksNodePool>> {
    let path = format!("{}/agentPools", resource_id.arm_path());
    let response: serde_json::Value = client.get(&path).await?;
    let pools = response
        .get("value")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    let mut result = Vec::with_capacity(pools.len());
    for pool_val in pools {
        let name = pool_val
            .get("name")
            .and_then(|name| name.as_str())
            .unwrap_or_default()
            .to_string();
        let properties = pool_val.get("properties").cloned().unwrap_or_default();
        let mut pool = serde_json::from_value::<AksNodePool>(properties).unwrap_or_default();
        pool.name = name;
        result.push(pool);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_node_pool_properties_and_name() {
        let response = serde_json::json!({
            "value": [
                {
                    "name": "systempool",
                    "properties": {
                        "vmSize": "Standard_D4s_v5",
                        "count": 3,
                        "mode": "System",
                        "enableAutoScaling": true,
                        "minCount": 1,
                        "maxCount": 5,
                        "availabilityZones": ["1", "2"],
                        "provisioningState": "Succeeded",
                        "powerState": {
                            "code": "Running"
                        }
                    }
                }
            ]
        });

        let pools = response
            .get("value")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        let mut parsed = Vec::new();

        for pool_val in pools {
            let name = pool_val
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            let properties = pool_val.get("properties").cloned().unwrap_or_default();
            let mut pool = serde_json::from_value::<AksNodePool>(properties).unwrap_or_default();
            pool.name = name;
            parsed.push(pool);
        }

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "systempool");
        assert_eq!(parsed[0].vm_size.as_deref(), Some("Standard_D4s_v5"));
        assert_eq!(parsed[0].count, Some(3));
        assert_eq!(parsed[0].mode.as_deref(), Some("System"));
        assert_eq!(parsed[0].provisioning_state.as_deref(), Some("Succeeded"));
        assert_eq!(
            parsed[0]
                .power_state
                .as_ref()
                .and_then(|state| state.code.as_deref()),
            Some("Running")
        );
    }
}
