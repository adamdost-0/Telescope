//! Namespace listing.

use k8s_openapi::api::core::v1::Namespace;
use kube::{Api, Client};

/// List all namespace names from the cluster.
pub async fn list_namespaces(client: &Client) -> crate::Result<Vec<String>> {
    let ns_api: Api<Namespace> = Api::all(client.clone());
    let ns_list = ns_api.list(&Default::default()).await?;
    let names: Vec<String> = ns_list
        .items
        .iter()
        .filter_map(|ns| ns.metadata.name.clone())
        .collect();
    Ok(names)
}
