pub mod aks;
pub mod client;
pub mod error;
pub mod openai;
pub mod resolve;
pub mod types;

pub use aks::{
    create_node_pool, delete_node_pool, get_cluster, get_pool_upgrade_profile, get_upgrade_profile,
    list_maintenance_configs, list_node_pools, scale_node_pool, start_cluster, stop_cluster,
    update_autoscaler, upgrade_cluster, upgrade_pool_node_image, upgrade_pool_version,
    AksClusterDetail, AksNodePool, AvailableUpgrade, CreateNodePoolRequest, MaintenanceConfig,
    PoolUpgradeProfile, PowerState, UpgradeProfile,
};
pub use client::ArmClient;
pub use error::{AzureAiProviderErrorClass, AzureError, Result};
pub use openai::{
    AzureOpenAiAuth, AzureOpenAiChatCompletion, AzureOpenAiChatCompletionsRequest,
    AzureOpenAiChatMessage, AzureOpenAiChatRole, AzureOpenAiClient, AzureOpenAiClientOptions,
    AzureOpenAiConnectionTestResult, AzureOpenAiEndpoint, AzureOpenAiResponseFormat,
    AzureOpenAiResponseFormatJsonSchema, AzureOpenAiTokenUsage,
};
pub use resolve::{
    extract_fqdn, inspect_aks_identity_preferences, read_aks_identity_preferences,
    resolve_aks_identity, resolve_aks_identity_from_preferences, save_aks_identity_preferences,
    unresolved_aks_identity_message, AksIdentityPreferenceStatus, AksIdentityPreferences,
};
pub use types::{AksResourceId, AzureCloud, AKS_API_VERSION, AZURE_OPENAI_API_VERSION};
