pub mod aks;
pub mod client;
pub mod error;
pub mod resolve;
pub mod types;

pub use aks::{get_cluster, list_node_pools, AksClusterDetail, AksNodePool, PowerState};
pub use client::ArmClient;
pub use error::{AzureError, Result};
pub use resolve::{resolve_aks_identity, resolve_aks_identity_from_preferences};
pub use types::{AksResourceId, AzureCloud, AKS_API_VERSION};
