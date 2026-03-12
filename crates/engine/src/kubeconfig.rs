//! Kubeconfig parsing and context management.

use kube::config::Kubeconfig;
use serde::{Deserialize, Serialize};

/// A simplified cluster context for the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterContext {
    /// Context name from kubeconfig.
    pub name: String,
    /// Cluster server URL.
    pub cluster_server: Option<String>,
    /// Default namespace (if set).
    pub namespace: Option<String>,
    /// Whether this is the currently active context.
    pub is_active: bool,
    /// Authentication method: "exec", "token", "certificate", or "unknown".
    pub auth_type: String,
}

/// Load kubeconfig from default location (~/.kube/config) and list contexts.
pub fn list_contexts() -> crate::Result<Vec<ClusterContext>> {
    let kubeconfig = Kubeconfig::read()?;
    let active = kubeconfig.current_context.as_deref().unwrap_or("");

    let contexts: Vec<ClusterContext> = kubeconfig
        .contexts
        .iter()
        .map(|named_ctx| {
            let ctx = &named_ctx.context;
            let cluster_server = ctx.as_ref().and_then(|c| {
                let cluster_name = &c.cluster;
                kubeconfig
                    .clusters
                    .iter()
                    .find(|nc| nc.name == *cluster_name)
                    .and_then(|nc| nc.cluster.as_ref())
                    .and_then(|cl| cl.server.clone())
            });

            let auth_type = ctx
                .as_ref()
                .and_then(|c| c.user.as_deref())
                .and_then(|user_name| kubeconfig.auth_infos.iter().find(|a| a.name == user_name))
                .map(|auth| {
                    match &auth.auth_info {
                        Some(info) if info.exec.is_some() => "exec",
                        Some(info) if info.token.is_some() => "token",
                        Some(info)
                            if info.client_certificate.is_some()
                                || info.client_certificate_data.is_some() =>
                        {
                            "certificate"
                        }
                        _ => "unknown",
                    }
                    .to_string()
                })
                .unwrap_or_else(|| "unknown".to_string());

            ClusterContext {
                name: named_ctx.name.clone(),
                cluster_server,
                namespace: ctx.as_ref().and_then(|c| c.namespace.clone()),
                is_active: named_ctx.name == active,
                auth_type,
            }
        })
        .collect();

    Ok(contexts)
}

/// Get the currently active context name from kubeconfig.
pub fn active_context() -> crate::Result<String> {
    let kubeconfig = Kubeconfig::read()?;
    kubeconfig
        .current_context
        .ok_or(crate::EngineError::NoActiveContext)
}
