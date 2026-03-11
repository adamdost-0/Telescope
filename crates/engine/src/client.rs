//! Kubernetes client construction and management.

use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::Client;

/// Create a Kubernetes client from the default kubeconfig.
/// Uses the currently active context.
pub async fn create_client() -> crate::Result<Client> {
    let client = Client::try_default().await?;
    Ok(client)
}

/// Create a client for a specific kubeconfig context.
pub async fn create_client_for_context(context_name: &str) -> crate::Result<Client> {
    let kubeconfig = Kubeconfig::read()?;
    let options = KubeConfigOptions {
        context: Some(context_name.to_string()),
        ..Default::default()
    };
    let config = kube::Config::from_custom_kubeconfig(kubeconfig, &options)
        .await
        .map_err(|e| {
            crate::EngineError::Other(format!(
                "Failed to build config for context '{}': {}",
                context_name, e
            ))
        })?;
    let client = Client::try_from(config)?;
    Ok(client)
}
