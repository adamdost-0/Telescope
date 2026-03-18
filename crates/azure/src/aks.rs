use std::{collections::HashMap, time::Duration};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::sleep;

use crate::{AksResourceId, ArmClient, AzureError, Result};

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

/// Kubelet identity details from `properties.identityProfile.kubeletidentity`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AksKubeletIdentity {
    pub client_id: Option<String>,
    pub object_id: Option<String>,
    pub resource_id: Option<String>,
}

/// Map of identity profile entries (e.g. `kubeletidentity`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AksIdentityProfileMap {
    pub kubeletidentity: Option<AksKubeletIdentity>,
}

/// Maintenance configuration for an AKS cluster.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MaintenanceConfig {
    pub name: String,
    pub not_allowed_time: Vec<MaintenanceTimeSpan>,
    pub time_in_week: Vec<MaintenanceTimeInWeek>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MaintenanceTimeSpan {
    pub start: Option<String>,
    pub end: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MaintenanceTimeInWeek {
    pub day: Option<String>,
    pub hour_slots: Option<Vec<i32>>,
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
    pub identity_profile: Option<AksIdentityProfileMap>,
    pub auto_upgrade_profile: Option<AksAutoUpgradeProfile>,
    pub oidc_issuer_profile: Option<AksOidcIssuerProfile>,
    pub security_profile: Option<AksSecurityProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableUpgrade {
    pub kubernetes_version: String,
    pub is_preview: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UpgradeProfile {
    pub current_version: String,
    pub upgrades: Vec<AvailableUpgrade>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PoolUpgradeProfile {
    pub current_version: String,
    pub upgrades: Vec<AvailableUpgrade>,
    pub latest_node_image_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManagedClusterEnvelope {
    pub properties: AksClusterDetail,
    pub sku: Option<AksClusterSku>,
    pub identity: Option<AksIdentityProfile>,
}

#[derive(Debug, Deserialize)]
struct ArmListResponse<T> {
    value: Vec<T>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NamedPropertiesResponse<T> {
    name: String,
    properties: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MaintenanceConfigPropertiesResponse {
    #[serde(default)]
    not_allowed_time: Vec<MaintenanceTimeSpan>,
    #[serde(default)]
    time_in_week: Vec<MaintenanceTimeInWeek>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClusterUpgradeProfileEnvelope {
    properties: ClusterUpgradeProfileProperties,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClusterUpgradeProfileProperties {
    control_plane_profile: ClusterUpgradePayload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClusterUpgradePayload {
    kubernetes_version: String,
    #[serde(default)]
    upgrades: Vec<AvailableUpgrade>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PoolUpgradeProfileEnvelope {
    properties: PoolUpgradeProfileProperties,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PoolUpgradeProfileProperties {
    kubernetes_version: Option<String>,
    orchestrator_version: Option<String>,
    #[serde(default)]
    upgrades: Vec<AvailableUpgrade>,
    latest_node_image_version: Option<String>,
}

const ARM_POLL_INTERVAL: Duration = Duration::from_secs(15);
const ARM_MAX_POLLS: usize = 80;

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

pub async fn start_cluster(client: &ArmClient, resource_id: &AksResourceId) -> Result<()> {
    let path = format!("{}/start", resource_id.arm_path());
    client.post(&path, None::<&Value>).await?;
    wait_for_cluster_state(client, resource_id, Some("Running"), None).await
}

pub async fn stop_cluster(client: &ArmClient, resource_id: &AksResourceId) -> Result<()> {
    let path = format!("{}/stop", resource_id.arm_path());
    client.post(&path, None::<&Value>).await?;
    wait_for_cluster_state(client, resource_id, Some("Stopped"), None).await
}

pub async fn list_node_pools(
    client: &ArmClient,
    resource_id: &AksResourceId,
) -> Result<Vec<AksNodePool>> {
    let path = format!("{}/agentPools", resource_id.arm_path());
    let response: ArmListResponse<NamedPropertiesResponse<AksNodePool>> = client.get(&path).await?;
    Ok(response.value.into_iter().map(into_node_pool).collect())
}

pub async fn get_upgrade_profile(
    client: &ArmClient,
    resource_id: &AksResourceId,
) -> Result<UpgradeProfile> {
    let response: ClusterUpgradeProfileEnvelope =
        client.get(&resource_id.upgrade_profile_path()).await?;
    Ok(into_cluster_upgrade_profile(response))
}

pub async fn upgrade_cluster(
    client: &ArmClient,
    resource_id: &AksResourceId,
    target_version: &str,
) -> Result<()> {
    let path = resource_id.arm_path();
    let mut cluster: Value = client.get(&path).await?;
    let properties = require_properties_map(&mut cluster, "cluster upgrade request")?;
    properties.insert(
        "kubernetesVersion".to_string(),
        serde_json::json!(target_version),
    );
    let _: Value = client.put(&path, &cluster).await?;
    wait_for_cluster_state(client, resource_id, None, Some(target_version)).await
}

/// Fetch maintenance configurations for an AKS cluster.
pub async fn list_maintenance_configs(
    client: &ArmClient,
    resource_id: &AksResourceId,
) -> Result<Vec<MaintenanceConfig>> {
    let path = resource_id.maintenance_config_path();
    let response: ArmListResponse<NamedPropertiesResponse<MaintenanceConfigPropertiesResponse>> =
        client.get(&path).await?;

    Ok(response
        .value
        .into_iter()
        .map(|item| MaintenanceConfig {
            name: item.name,
            not_allowed_time: item.properties.not_allowed_time,
            time_in_week: item.properties.time_in_week,
        })
        .collect())
}

pub async fn get_pool_upgrade_profile(
    client: &ArmClient,
    resource_id: &AksResourceId,
    pool_name: &str,
) -> Result<PoolUpgradeProfile> {
    let path = format!(
        "{}/upgradeProfiles/default",
        resource_id.agent_pool_path(pool_name)
    );
    let response: PoolUpgradeProfileEnvelope = client.get(&path).await?;
    into_pool_upgrade_profile(response)
}

// ── Node-pool mutation types ─────────────────────────────────────────────

/// Configuration for creating a new AKS node pool via ARM API.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CreateNodePoolRequest {
    pub name: String,
    pub vm_size: String,
    pub count: i32,
    pub os_type: Option<String>,
    pub mode: Option<String>,
    pub orchestrator_version: Option<String>,
    pub enable_auto_scaling: Option<bool>,
    pub min_count: Option<i32>,
    pub max_count: Option<i32>,
    pub availability_zones: Option<Vec<String>>,
    pub max_pods: Option<i32>,
    pub node_labels: Option<HashMap<String, String>>,
    pub node_taints: Option<Vec<String>>,
}

// ── Node-pool mutation operations ────────────────────────────────────────

fn into_node_pool(response: NamedPropertiesResponse<AksNodePool>) -> AksNodePool {
    let mut pool = response.properties;
    pool.name = response.name;
    pool
}

fn require_properties_map<'a>(
    value: &'a mut Value,
    context: &str,
) -> Result<&'a mut serde_json::Map<String, Value>> {
    value
        .get_mut("properties")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            AzureError::Serialization(format!(
                "Azure ARM response for {context} did not include a properties object"
            ))
        })
}

/// Scale a node pool to a specific node count.
pub async fn scale_node_pool(
    client: &ArmClient,
    resource_id: &AksResourceId,
    pool_name: &str,
    count: i32,
) -> Result<AksNodePool> {
    let path = resource_id.agent_pool_path(pool_name);
    let mut current: serde_json::Value = client.get(&path).await?;
    let properties = require_properties_map(&mut current, "node pool scale request")?;
    properties.insert("count".to_string(), serde_json::json!(count));
    let _: Value = client.put(&path, &current).await?;
    wait_for_pool_state(client, resource_id, pool_name, None, None).await?;
    let result: NamedPropertiesResponse<AksNodePool> = client.get(&path).await?;
    Ok(into_node_pool(result))
}

/// Update the autoscaler settings on a node pool.
pub async fn update_autoscaler(
    client: &ArmClient,
    resource_id: &AksResourceId,
    pool_name: &str,
    enabled: bool,
    min: Option<i32>,
    max: Option<i32>,
) -> Result<AksNodePool> {
    let path = resource_id.agent_pool_path(pool_name);
    let mut current: serde_json::Value = client.get(&path).await?;
    let properties = require_properties_map(&mut current, "node pool autoscaler update")?;
    properties.insert("enableAutoScaling".to_string(), serde_json::json!(enabled));
    if let Some(min_v) = min {
        properties.insert("minCount".to_string(), serde_json::json!(min_v));
    }
    if let Some(max_v) = max {
        properties.insert("maxCount".to_string(), serde_json::json!(max_v));
    }
    let _: Value = client.put(&path, &current).await?;
    wait_for_pool_state(client, resource_id, pool_name, None, None).await?;
    let result: NamedPropertiesResponse<AksNodePool> = client.get(&path).await?;
    Ok(into_node_pool(result))
}

/// Create a new node pool on an AKS cluster.
pub async fn create_node_pool(
    client: &ArmClient,
    resource_id: &AksResourceId,
    config: &CreateNodePoolRequest,
) -> Result<AksNodePool> {
    let path = resource_id.agent_pool_path(&config.name);
    let body = serde_json::json!({
        "properties": {
            "count": config.count,
            "vmSize": config.vm_size,
            "osType": config.os_type.as_deref().unwrap_or("Linux"),
            "mode": config.mode.as_deref().unwrap_or("User"),
            "orchestratorVersion": config.orchestrator_version,
            "enableAutoScaling": config.enable_auto_scaling.unwrap_or(false),
            "minCount": config.min_count,
            "maxCount": config.max_count,
            "availabilityZones": config.availability_zones,
            "maxPods": config.max_pods,
            "nodeLabels": config.node_labels,
            "nodeTaints": config.node_taints,
        }
    });
    let _: Value = client.put(&path, &body).await?;
    wait_for_pool_state(client, resource_id, &config.name, None, None).await?;
    let result: NamedPropertiesResponse<AksNodePool> = client.get(&path).await?;
    Ok(into_node_pool(result))
}

/// Delete a node pool from an AKS cluster.
pub async fn delete_node_pool(
    client: &ArmClient,
    resource_id: &AksResourceId,
    pool_name: &str,
) -> Result<()> {
    let path = resource_id.agent_pool_path(pool_name);
    client.delete(&path).await?;
    for _ in 0..ARM_MAX_POLLS {
        match client.get::<Value>(&path).await {
            Err(_) => return Ok(()),
            Ok(val) => {
                let state = val
                    .pointer("/properties/provisioningState")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if state != "deleting" {
                    return Ok(());
                }
                sleep(ARM_POLL_INTERVAL).await;
            }
        }
    }
    Err(AzureError::Api {
        status: 408,
        code: "Timeout".to_string(),
        message: format!("Timed out waiting for node pool {} deletion", pool_name),
    })
}

pub async fn upgrade_pool_version(
    client: &ArmClient,
    resource_id: &AksResourceId,
    pool_name: &str,
    version: &str,
) -> Result<()> {
    let path = resource_id.agent_pool_path(pool_name);
    let mut pool: Value = client.get(&path).await?;
    let properties = require_properties_map(&mut pool, "node pool upgrade request")?;
    properties.insert(
        "orchestratorVersion".to_string(),
        serde_json::json!(version),
    );
    let _: Value = client.put(&path, &pool).await?;
    wait_for_pool_state(client, resource_id, pool_name, Some(version), None).await
}

pub async fn upgrade_pool_node_image(
    client: &ArmClient,
    resource_id: &AksResourceId,
    pool_name: &str,
) -> Result<()> {
    let desired_node_image = get_pool_upgrade_profile(client, resource_id, pool_name)
        .await?
        .latest_node_image_version;
    let path = format!(
        "{}/upgradeNodeImageVersion",
        resource_id.agent_pool_path(pool_name)
    );
    client.post(&path, None::<&Value>).await?;
    wait_for_pool_state(
        client,
        resource_id,
        pool_name,
        None,
        desired_node_image.as_deref(),
    )
    .await
}

fn into_cluster_upgrade_profile(response: ClusterUpgradeProfileEnvelope) -> UpgradeProfile {
    let ClusterUpgradePayload {
        kubernetes_version,
        upgrades,
    } = response.properties.control_plane_profile;

    UpgradeProfile {
        current_version: kubernetes_version,
        upgrades,
    }
}

fn into_pool_upgrade_profile(response: PoolUpgradeProfileEnvelope) -> Result<PoolUpgradeProfile> {
    let PoolUpgradeProfileProperties {
        kubernetes_version,
        orchestrator_version,
        upgrades,
        latest_node_image_version,
    } = response.properties;
    let current_version = kubernetes_version.or(orchestrator_version).ok_or_else(|| {
        AzureError::Serialization(
            "Azure ARM node pool upgrade profile was missing kubernetesVersion/orchestratorVersion"
                .to_string(),
        )
    })?;

    Ok(PoolUpgradeProfile {
        current_version,
        upgrades,
        latest_node_image_version,
    })
}

fn normalize_state(state: Option<&str>) -> String {
    state.unwrap_or_default().to_ascii_lowercase()
}

async fn wait_for_cluster_state(
    client: &ArmClient,
    resource_id: &AksResourceId,
    desired_power_state: Option<&str>,
    desired_version: Option<&str>,
) -> Result<()> {
    for _ in 0..ARM_MAX_POLLS {
        let cluster = get_cluster(client, resource_id).await?;
        let provisioning_state = normalize_state(cluster.provisioning_state.as_deref());
        let power_state = normalize_state(
            cluster
                .power_state
                .as_ref()
                .and_then(|power_state| power_state.code.as_deref()),
        );
        let current_version = cluster.kubernetes_version.as_deref().unwrap_or_default();
        let power_matches = desired_power_state
            .map(|state| power_state == state.to_ascii_lowercase())
            .unwrap_or(true);
        let version_matches = desired_version
            .map(|version| current_version == version)
            .unwrap_or(true);
        if provisioning_state == "succeeded" && power_matches && version_matches {
            return Ok(());
        }
        sleep(ARM_POLL_INTERVAL).await;
    }

    Err(AzureError::Api {
        status: 408,
        code: "Timeout".to_string(),
        message: format!(
            "Timed out waiting for cluster {} to reach the requested state",
            resource_id.cluster_name
        ),
    })
}

async fn wait_for_pool_state(
    client: &ArmClient,
    resource_id: &AksResourceId,
    pool_name: &str,
    desired_version: Option<&str>,
    desired_node_image: Option<&str>,
) -> Result<()> {
    let path = resource_id.agent_pool_path(pool_name);
    for _ in 0..ARM_MAX_POLLS {
        let response: NamedPropertiesResponse<AksNodePool> = client.get(&path).await?;
        let pool = into_node_pool(response);
        let provisioning_state = normalize_state(pool.provisioning_state.as_deref());
        let version_matches = desired_version
            .map(|version| pool.orchestrator_version.as_deref() == Some(version))
            .unwrap_or(true);
        let node_image_matches = desired_node_image
            .map(|version| pool.node_image_version.as_deref() == Some(version))
            .unwrap_or(true);
        if provisioning_state == "succeeded" && version_matches && node_image_matches {
            return Ok(());
        }
        sleep(ARM_POLL_INTERVAL).await;
    }

    Err(AzureError::Api {
        status: 408,
        code: "Timeout".to_string(),
        message: format!(
            "Timed out waiting for node pool {} on cluster {} to reach the requested state",
            pool_name, resource_id.cluster_name
        ),
    })
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

        let parsed =
            serde_json::from_value::<ArmListResponse<NamedPropertiesResponse<AksNodePool>>>(
                response,
            )
            .unwrap()
            .value
            .into_iter()
            .map(into_node_pool)
            .collect::<Vec<_>>();

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

    #[test]
    fn parses_cluster_upgrade_profile_response() {
        let response = serde_json::json!({
            "properties": {
                "controlPlaneProfile": {
                    "kubernetesVersion": "1.29.4",
                    "upgrades": [
                        { "kubernetesVersion": "1.29.7", "isPreview": false },
                        { "kubernetesVersion": "1.30.2", "isPreview": true }
                    ]
                }
            }
        });

        let profile = into_cluster_upgrade_profile(
            serde_json::from_value::<ClusterUpgradeProfileEnvelope>(response).unwrap(),
        );
        assert_eq!(profile.current_version, "1.29.4");
        assert_eq!(profile.upgrades.len(), 2);
        assert_eq!(profile.upgrades[0].kubernetes_version, "1.29.7");
        assert!(!profile.upgrades[0].is_preview);
        assert!(profile.upgrades[1].is_preview);
    }

    #[test]
    fn parses_pool_upgrade_profile_response() {
        let response = serde_json::json!({
            "properties": {
                "kubernetesVersion": "1.29.4",
                "latestNodeImageVersion": "AKSUbuntu-2204gen2containerd-2024.10.12",
                "upgrades": [
                    { "kubernetesVersion": "1.29.7", "isPreview": false }
                ]
            }
        });

        let profile = into_pool_upgrade_profile(
            serde_json::from_value::<PoolUpgradeProfileEnvelope>(response).unwrap(),
        )
        .unwrap();
        assert_eq!(profile.current_version, "1.29.4");
        assert_eq!(
            profile.latest_node_image_version.as_deref(),
            Some("AKSUbuntu-2204gen2containerd-2024.10.12")
        );
        assert_eq!(profile.upgrades[0].kubernetes_version, "1.29.7");
    }

    #[test]
    fn node_pool_list_requires_value_array() {
        let result = serde_json::from_value::<ArmListResponse<NamedPropertiesResponse<AksNodePool>>>(
            serde_json::json!({}),
        );

        assert!(result.is_err());
    }

    #[test]
    fn managed_cluster_response_requires_properties() {
        let result = serde_json::from_value::<ManagedClusterEnvelope>(serde_json::json!({}));

        assert!(result.is_err());
    }

    #[test]
    fn pool_upgrade_profile_requires_current_version() {
        let response = serde_json::json!({
            "properties": {
                "latestNodeImageVersion": "image"
            }
        });

        let err = into_pool_upgrade_profile(
            serde_json::from_value::<PoolUpgradeProfileEnvelope>(response).unwrap(),
        )
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("missing kubernetesVersion/orchestratorVersion"));
    }
}
