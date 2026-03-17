use serde::{Deserialize, Serialize};

/// Azure cloud environment
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AzureCloud {
    #[default]
    Commercial,
    UsGovernment,
    UsGovSecret,
    UsGovTopSecret,
}

impl AzureCloud {
    pub fn detect_from_url(server_url: &str) -> Self {
        if server_url.contains(".azmk8s.io") {
            Self::Commercial
        } else if server_url.contains(".azmk8s.us")
            || server_url.contains(".cx.aks.containerservice.azure.us")
        {
            Self::UsGovernment
        } else if server_url.contains(".scloud") {
            Self::UsGovSecret
        } else if server_url.contains(".eaglex.ic.gov") {
            Self::UsGovTopSecret
        } else {
            Self::Commercial
        }
    }

    pub fn arm_endpoint(&self) -> &str {
        match self {
            Self::Commercial => "https://management.azure.com",
            Self::UsGovernment => "https://management.usgovcloudapi.net",
            Self::UsGovSecret => "https://management.azure.microsoft.scloud",
            Self::UsGovTopSecret => "https://management.azure.microsoft.eaglex.ic.gov",
        }
    }

    pub fn auth_endpoint(&self) -> &str {
        match self {
            Self::Commercial => "https://login.microsoftonline.com",
            Self::UsGovernment => "https://login.microsoftonline.us",
            Self::UsGovSecret => "https://login.microsoftonline.us",
            Self::UsGovTopSecret => "https://login.microsoftonline.us",
        }
    }

    pub fn token_scope(&self) -> &str {
        match self {
            Self::Commercial => "https://management.azure.com/.default",
            Self::UsGovernment => "https://management.usgovcloudapi.net/.default",
            Self::UsGovSecret => "https://management.azure.microsoft.scloud/.default",
            Self::UsGovTopSecret => "https://management.azure.microsoft.eaglex.ic.gov/.default",
        }
    }
}

/// AKS cluster resource identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AksResourceId {
    pub subscription_id: String,
    pub resource_group: String,
    pub cluster_name: String,
}

impl AksResourceId {
    pub fn arm_path(&self) -> String {
        format!(
            "/subscriptions/{}/resourceGroups/{}/providers/Microsoft.ContainerService/managedClusters/{}",
            self.subscription_id, self.resource_group, self.cluster_name
        )
    }

    pub fn agent_pool_path(&self, pool_name: &str) -> String {
        format!("{}/agentPools/{}", self.arm_path(), pool_name)
    }

    pub fn upgrade_profile_path(&self) -> String {
        format!("{}/upgradeProfiles/default", self.arm_path())
    }

    pub fn maintenance_config_path(&self) -> String {
        format!("{}/maintenanceConfigurations", self.arm_path())
    }
}

pub const AKS_API_VERSION: &str = "2024-09-01";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_arm_endpoints() {
        assert_eq!(
            AzureCloud::Commercial.arm_endpoint(),
            "https://management.azure.com"
        );
        assert_eq!(
            AzureCloud::UsGovernment.arm_endpoint(),
            "https://management.usgovcloudapi.net"
        );
        assert_eq!(
            AzureCloud::UsGovSecret.arm_endpoint(),
            "https://management.azure.microsoft.scloud"
        );
        assert_eq!(
            AzureCloud::UsGovTopSecret.arm_endpoint(),
            "https://management.azure.microsoft.eaglex.ic.gov"
        );
    }

    #[test]
    fn cloud_auth_endpoints() {
        assert_eq!(
            AzureCloud::Commercial.auth_endpoint(),
            "https://login.microsoftonline.com"
        );
        assert_eq!(
            AzureCloud::UsGovernment.auth_endpoint(),
            "https://login.microsoftonline.us"
        );
    }

    #[test]
    fn cloud_token_scopes() {
        assert_eq!(
            AzureCloud::Commercial.token_scope(),
            "https://management.azure.com/.default"
        );
        assert_eq!(
            AzureCloud::UsGovernment.token_scope(),
            "https://management.usgovcloudapi.net/.default"
        );
    }

    #[test]
    fn default_cloud_is_commercial() {
        assert_eq!(AzureCloud::default(), AzureCloud::Commercial);
    }

    #[test]
    fn detect_cloud_from_aks_url() {
        assert_eq!(
            AzureCloud::detect_from_url("https://myaks.hcp.eastus.azmk8s.io:443"),
            AzureCloud::Commercial
        );
        assert_eq!(
            AzureCloud::detect_from_url("https://myaks.hcp.usgovvirginia.azmk8s.us:443"),
            AzureCloud::UsGovernment
        );
        assert_eq!(
            AzureCloud::detect_from_url("https://cluster.cx.aks.containerservice.azure.us"),
            AzureCloud::UsGovernment
        );
        assert_eq!(
            AzureCloud::detect_from_url("https://example.invalid"),
            AzureCloud::Commercial
        );
    }

    #[test]
    fn aks_resource_id_arm_path() {
        let id = AksResourceId {
            subscription_id: "sub-123".to_string(),
            resource_group: "rg-prod".to_string(),
            cluster_name: "aks-cluster".to_string(),
        };
        assert_eq!(
            id.arm_path(),
            "/subscriptions/sub-123/resourceGroups/rg-prod/providers/Microsoft.ContainerService/managedClusters/aks-cluster"
        );
    }

    #[test]
    fn aks_resource_id_agent_pool_path() {
        let id = AksResourceId {
            subscription_id: "sub-123".to_string(),
            resource_group: "rg-prod".to_string(),
            cluster_name: "aks-cluster".to_string(),
        };
        assert!(id
            .agent_pool_path("nodepool1")
            .ends_with("/agentPools/nodepool1"));
    }

    #[test]
    fn aks_resource_id_upgrade_profile_path() {
        let id = AksResourceId {
            subscription_id: "sub-123".to_string(),
            resource_group: "rg-prod".to_string(),
            cluster_name: "aks-cluster".to_string(),
        };
        assert!(id
            .upgrade_profile_path()
            .ends_with("/upgradeProfiles/default"));
    }

    #[test]
    fn aks_resource_id_maintenance_config_path() {
        let id = AksResourceId {
            subscription_id: "sub-123".to_string(),
            resource_group: "rg-prod".to_string(),
            cluster_name: "aks-cluster".to_string(),
        };
        assert!(id
            .maintenance_config_path()
            .ends_with("/maintenanceConfigurations"));
    }

    #[test]
    fn cloud_serialization_roundtrip() {
        let cloud = AzureCloud::UsGovernment;
        let json = serde_json::to_string(&cloud).unwrap();
        let deserialized: AzureCloud = serde_json::from_str(&json).unwrap();
        assert_eq!(cloud, deserialized);
    }

    #[test]
    fn aks_resource_id_serialization_roundtrip() {
        let id = AksResourceId {
            subscription_id: "sub-123".to_string(),
            resource_group: "rg-prod".to_string(),
            cluster_name: "aks-cluster".to_string(),
        };
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: AksResourceId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.subscription_id, "sub-123");
        assert_eq!(deserialized.resource_group, "rg-prod");
        assert_eq!(deserialized.cluster_name, "aks-cluster");
    }
}
