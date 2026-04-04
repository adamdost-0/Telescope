//! Kubernetes client construction and management.

use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::Client;
use serde::{Deserialize, Serialize};

/// Cluster version and authentication metadata returned to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    /// Kubernetes server version, e.g. "1.30".
    pub server_version: String,
    /// Platform string reported by the API server (e.g. "linux/amd64").
    pub platform: String,
    /// API server URL from kubeconfig.
    pub server_url: String,
    /// Authentication method: "exec", "token", "certificate", or "unknown".
    pub auth_type: String,
    /// Name of the exec credential plugin (e.g. "kubelogin"), if auth_type is "exec".
    pub exec_plugin: Option<String>,
    /// True when the server URL matches an AKS managed-cluster endpoint.
    pub is_aks: bool,
    /// Human-readable auth hint for the UI (e.g. "Authenticated via Azure Entra ID").
    pub auth_hint: Option<String>,
    /// Azure subscription ID (populated for AKS clusters when resolved).
    pub subscription_id: Option<String>,
    /// Azure resource group (populated for AKS clusters when resolved).
    pub resource_group: Option<String>,
    /// Full ARM resource ID (populated for AKS clusters when resolved).
    pub azure_resource_id: Option<String>,
}

/// Retrieve cluster version info from the API server.
pub async fn get_cluster_info(client: &Client, context_name: &str) -> crate::Result<ClusterInfo> {
    let version = client.apiserver_version().await?;

    // Read kubeconfig to extract server URL & auth metadata for this context.
    let kubeconfig = Kubeconfig::read()?;
    let (server_url, auth_type, exec_plugin) = extract_auth_meta(&kubeconfig, context_name);

    let is_aks = server_url.as_ref().map(|u| is_aks_url(u)).unwrap_or(false);

    let auth_hint = if is_aks {
        let detail = match exec_plugin.as_deref() {
            Some("kubelogin") => "Authenticated via Azure Entra ID (kubelogin)",
            Some(p) => {
                return Ok(build_info(
                    version,
                    server_url,
                    auth_type,
                    Some(p.to_string()),
                    is_aks,
                    Some(format!("Authenticated via exec plugin ({})", p)),
                ))
            }
            None if auth_type == "exec" => "Authenticated via exec credential plugin",
            _ => "Authenticated via Azure Entra ID",
        };
        Some(detail.to_string())
    } else {
        match auth_type.as_str() {
            "exec" => {
                let msg = match exec_plugin.as_deref() {
                    Some(p) => format!("Auth: exec ({})", p),
                    None => "Auth: exec credential plugin".to_string(),
                };
                Some(msg)
            }
            "token" => Some("Auth: bearer token".to_string()),
            "certificate" => Some("Auth: client certificate".to_string()),
            _ => None,
        }
    };

    Ok(build_info(
        version,
        server_url,
        auth_type,
        exec_plugin,
        is_aks,
        auth_hint,
    ))
}

fn build_info(
    version: k8s_openapi::apimachinery::pkg::version::Info,
    server_url: Option<String>,
    auth_type: String,
    exec_plugin: Option<String>,
    is_aks: bool,
    auth_hint: Option<String>,
) -> ClusterInfo {
    ClusterInfo {
        server_version: format!("{}.{}", version.major, version.minor),
        platform: version.platform,
        server_url: server_url.unwrap_or_default(),
        auth_type,
        exec_plugin,
        is_aks,
        auth_hint,
        subscription_id: None,
        resource_group: None,
        azure_resource_id: None,
    }
}

/// Extract server URL, auth type, and exec plugin name from kubeconfig for a
/// given context.
fn extract_auth_meta(
    kubeconfig: &Kubeconfig,
    context_name: &str,
) -> (Option<String>, String, Option<String>) {
    let named_ctx = kubeconfig.contexts.iter().find(|c| c.name == context_name);
    let ctx = named_ctx.and_then(|c| c.context.as_ref());

    let server_url = ctx.and_then(|c| {
        kubeconfig
            .clusters
            .iter()
            .find(|nc| nc.name == c.cluster)
            .and_then(|nc| nc.cluster.as_ref())
            .and_then(|cl| cl.server.clone())
    });

    let auth_info = ctx
        .and_then(|c| c.user.as_deref())
        .and_then(|user_name| kubeconfig.auth_infos.iter().find(|a| a.name == user_name))
        .and_then(|a| a.auth_info.as_ref());

    let (auth_type, exec_plugin) = match auth_info {
        Some(info) if info.exec.is_some() => {
            let plugin_name = info.exec.as_ref().and_then(|e| {
                e.command.as_ref().map(|cmd| {
                    // Extract binary name from path
                    std::path::Path::new(cmd)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(cmd)
                        .to_string()
                })
            });
            ("exec".to_string(), plugin_name)
        }
        Some(info) if info.token.is_some() => ("token".to_string(), None),
        Some(info)
            if info.client_certificate.is_some() || info.client_certificate_data.is_some() =>
        {
            ("certificate".to_string(), None)
        }
        _ => ("unknown".to_string(), None),
    };

    (server_url, auth_type, exec_plugin)
}

/// Returns true when the URL matches an AKS managed cluster endpoint.
fn is_aks_url(url: &str) -> bool {
    url.contains(".azmk8s.io")
        || url.contains(".azmk8s.us")
        || url.contains(".cx.aks.containerservice.azure.us")
}

async fn build_client_from_kubeconfig(
    kubeconfig: Kubeconfig,
    options: &KubeConfigOptions,
    context_label: &str,
) -> crate::Result<kube::Config> {
    kube::Config::from_custom_kubeconfig(kubeconfig, options)
        .await
        .map_err(|error| {
            crate::EngineError::Other(format!(
                "Failed to build config for context '{context_label}': {error}"
            ))
        })
}

/// Create a Kubernetes client from the default kubeconfig.
/// Uses the currently active context.
pub async fn create_client() -> crate::Result<Client> {
    let kubeconfig = crate::kubeconfig::load_kubeconfig_for_context(None)?;
    let options = KubeConfigOptions::default();
    let config = build_client_from_kubeconfig(kubeconfig, &options, "current context").await?;
    let client = Client::try_from(config)?;
    Ok(client)
}

/// Create a client for a specific kubeconfig context.
pub async fn create_client_for_context(context_name: &str) -> crate::Result<Client> {
    let kubeconfig = crate::kubeconfig::load_kubeconfig_for_context(Some(context_name))?;
    let options = KubeConfigOptions {
        context: Some(context_name.to_string()),
        ..Default::default()
    };
    let config = build_client_from_kubeconfig(kubeconfig, &options, context_name).await?;
    let client = Client::try_from(config)?;
    Ok(client)
}

/// Create a client for a specific kubeconfig context that impersonates the
/// given user. The hub's service account must hold the `impersonate` RBAC
/// verb for Users and Groups.
///
/// When `user_email` is empty or `"anonymous@local"`, no impersonation
/// headers are injected and the hub's own identity is used.
pub async fn create_client_for_context_as_user(
    context_name: &str,
    user_email: &str,
    groups: &[String],
) -> crate::Result<Client> {
    let kubeconfig = crate::kubeconfig::load_kubeconfig_for_context(Some(context_name))?;
    let options = KubeConfigOptions {
        context: Some(context_name.to_string()),
        ..Default::default()
    };
    let mut config = build_client_from_kubeconfig(kubeconfig, &options, context_name).await?;

    if !user_email.is_empty() && user_email != "anonymous@local" {
        config.auth_info.impersonate = Some(user_email.to_string());
        if !groups.is_empty() {
            config.auth_info.impersonate_groups = Some(groups.to_vec());
        }
    }

    let client = Client::try_from(config)?;
    Ok(client)
}
