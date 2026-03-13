//! Namespace listing.

use k8s_openapi::api::core::v1::Namespace;
use kube::{
    api::{DeleteParams, ObjectMeta, PostParams},
    Api, Client,
};

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

/// Create a namespace with the given name.
pub async fn create_namespace(client: &Client, name: &str) -> crate::Result<String> {
    let ns_api: Api<Namespace> = Api::all(client.clone());
    let namespace = Namespace {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let created = ns_api.create(&PostParams::default(), &namespace).await?;

    Ok(created.metadata.name.unwrap_or_else(|| name.to_string()))
}

/// Delete a namespace by name.
pub async fn delete_namespace(client: &Client, name: &str) -> crate::Result<String> {
    let ns_api: Api<Namespace> = Api::all(client.clone());
    ns_api.delete(name, &DeleteParams::default()).await?;
    Ok(name.to_string())
}
