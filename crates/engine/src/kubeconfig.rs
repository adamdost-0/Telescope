//! Kubeconfig parsing and context management.

use std::path::PathBuf;

use kube::config::Kubeconfig;
use serde::{Deserialize, Serialize};
use telescope_core::resolve_trusted_binary;

#[cfg(target_os = "windows")]
const TRUSTED_EXEC_HELPER_PATHS: &[&str] = &[
    r"C:\Program Files\Azure\CLI2\wbin\az.cmd",
    r"C:\Program Files (x86)\Microsoft SDKs\Azure\CLI2\wbin\az.cmd",
    r"C:\Program Files\Microsoft SDKs\Azure\CLI2\wbin\az.cmd",
    r"C:\ProgramData\chocolatey\bin\az.cmd",
    r"C:\Program Files\kubelogin\kubelogin.exe",
    r"C:\ProgramData\chocolatey\bin\kubelogin.exe",
    r"C:\Program Files\Amazon\AWSCLIV2\aws.exe",
    r"C:\ProgramData\chocolatey\bin\aws.exe",
    r"C:\Program Files\Amazon\EKS\aws-iam-authenticator.exe",
    r"C:\ProgramData\chocolatey\bin\aws-iam-authenticator.exe",
    r"C:\Program Files\Google\Cloud SDK\google-cloud-sdk\bin\gke-gcloud-auth-plugin.exe",
    r"C:\Program Files\Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd",
];

#[cfg(not(target_os = "windows"))]
const TRUSTED_EXEC_HELPER_PATHS: &[&str] = &[
    "/usr/local/bin/az",
    "/opt/homebrew/bin/az",
    "/usr/bin/az",
    "/snap/bin/az",
    "/usr/local/bin/kubelogin",
    "/opt/homebrew/bin/kubelogin",
    "/usr/bin/kubelogin",
    "/snap/bin/kubelogin",
    "/usr/local/bin/aws",
    "/opt/homebrew/bin/aws",
    "/usr/bin/aws",
    "/snap/bin/aws",
    "/usr/local/bin/aws-iam-authenticator",
    "/opt/homebrew/bin/aws-iam-authenticator",
    "/usr/bin/aws-iam-authenticator",
    "/snap/bin/aws-iam-authenticator",
    "/usr/local/bin/gke-gcloud-auth-plugin",
    "/opt/homebrew/bin/gke-gcloud-auth-plugin",
    "/usr/bin/gke-gcloud-auth-plugin",
    "/snap/bin/gke-gcloud-auth-plugin",
    "/usr/local/bin/gcloud",
    "/opt/homebrew/bin/gcloud",
    "/usr/bin/gcloud",
    "/snap/bin/gcloud",
];

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

pub(crate) fn load_kubeconfig_for_context(context_name: Option<&str>) -> crate::Result<Kubeconfig> {
    let mut kubeconfig = Kubeconfig::read()?;
    pin_exec_helper_paths(&mut kubeconfig, context_name)?;
    Ok(kubeconfig)
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

fn pin_exec_helper_paths(
    kubeconfig: &mut Kubeconfig,
    context_name: Option<&str>,
) -> crate::Result<()> {
    pin_exec_helper_paths_with(
        kubeconfig,
        context_name,
        TRUSTED_EXEC_HELPER_PATHS.iter().map(PathBuf::from),
    )
}

fn pin_exec_helper_paths_with<I>(
    kubeconfig: &mut Kubeconfig,
    context_name: Option<&str>,
    trusted_paths: I,
) -> crate::Result<()>
where
    I: IntoIterator<Item = PathBuf>,
{
    let Some(target_context) = context_name
        .map(str::to_string)
        .or_else(|| kubeconfig.current_context.clone())
    else {
        return Ok(());
    };

    let user_name = kubeconfig
        .contexts
        .iter()
        .find(|named_ctx| named_ctx.name == target_context)
        .ok_or_else(|| {
            crate::EngineError::Other(format!(
                "Context '{target_context}' was not found in kubeconfig"
            ))
        })?
        .context
        .as_ref()
        .and_then(|ctx| ctx.user.clone());

    let Some(user_name) = user_name else {
        return Ok(());
    };

    let Some(auth_info) = kubeconfig
        .auth_infos
        .iter_mut()
        .find(|auth| auth.name == user_name)
        .and_then(|auth| auth.auth_info.as_mut())
    else {
        return Ok(());
    };

    let Some(exec) = auth_info.exec.as_mut() else {
        return Ok(());
    };

    let Some(original_command) = exec.command.clone() else {
        return Err(crate::EngineError::Other(format!(
            "Blocked kubeconfig exec credential helper for context `{target_context}`: helper command is missing"
        )));
    };
    let resolved = resolve_exec_helper_command_with(&original_command, trusted_paths).map_err(
        |reason| {
            crate::EngineError::Other(format!(
                "Blocked kubeconfig exec credential helper `{original_command}` for context `{target_context}`: {reason}. Install a supported helper in a trusted system location to continue."
            ))
        },
    )?;
    exec.command = Some(resolved.to_string_lossy().into_owned());
    Ok(())
}

fn resolve_exec_helper_command_with<I>(command: &str, trusted_paths: I) -> Result<PathBuf, String>
where
    I: IntoIterator<Item = PathBuf>,
{
    resolve_trusted_binary(command, trusted_paths)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{pin_exec_helper_paths_with, resolve_exec_helper_command_with};
    use kube::config::Kubeconfig;
    use serde_json::json;

    fn kubeconfig_with_exec(command: &str) -> Kubeconfig {
        serde_json::from_value(json!({
            "apiVersion": "v1",
            "kind": "Config",
            "clusters": [
                {
                    "name": "demo-cluster",
                    "cluster": {
                        "server": "https://demo.example.com"
                    }
                }
            ],
            "contexts": [
                {
                    "name": "demo",
                    "context": {
                        "cluster": "demo-cluster",
                        "user": "demo-user"
                    }
                }
            ],
            "current-context": "demo",
            "users": [
                {
                    "name": "demo-user",
                    "user": {
                        "exec": {
                            "apiVersion": "client.authentication.k8s.io/v1beta1",
                            "command": command
                        }
                    }
                }
            ]
        }))
        .expect("test kubeconfig")
    }

    #[test]
    fn resolve_exec_helper_command_accepts_trusted_binary_name() {
        let executable = std::env::current_exe().expect("current test executable");
        let name = executable
            .file_name()
            .and_then(|value| value.to_str())
            .expect("binary name");

        let resolved = resolve_exec_helper_command_with(name, vec![executable.clone()])
            .expect("trusted helper should resolve");

        assert_eq!(resolved, executable.canonicalize().expect("canonical path"));
    }

    #[test]
    fn resolve_exec_helper_command_rejects_relative_paths() {
        let err = resolve_exec_helper_command_with("./malicious-helper", Vec::<PathBuf>::new())
            .expect_err("relative helper should be blocked");

        assert!(err.contains("relative or qualified path"));
    }

    #[test]
    fn pin_exec_helper_paths_rewrites_trusted_exec_commands() {
        let executable = std::env::current_exe().expect("current test executable");
        let name = executable
            .file_name()
            .and_then(|value| value.to_str())
            .expect("binary name")
            .to_string();
        let mut kubeconfig = kubeconfig_with_exec(&name);

        pin_exec_helper_paths_with(&mut kubeconfig, Some("demo"), vec![executable.clone()])
            .expect("trusted exec helper should be accepted");

        let rewritten = kubeconfig.auth_infos[0]
            .auth_info
            .as_ref()
            .and_then(|info| info.exec.as_ref())
            .and_then(|exec| exec.command.clone())
            .expect("exec command");
        assert_eq!(
            rewritten,
            executable
                .canonicalize()
                .expect("canonical path")
                .to_string_lossy()
        );
    }

    #[test]
    fn pin_exec_helper_paths_blocks_untrusted_exec_commands() {
        let mut kubeconfig = kubeconfig_with_exec("./malicious-helper");

        let err = pin_exec_helper_paths_with(&mut kubeconfig, Some("demo"), Vec::<PathBuf>::new())
            .expect_err("untrusted exec helper should be rejected");

        assert!(err
            .to_string()
            .contains("Blocked kubeconfig exec credential helper"));
    }
}
