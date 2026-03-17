//! Imperative actions against the Kubernetes API (scale, restart, delete, etc.).

use kube::api::{Api, DeleteParams, Patch, PatchParams};
use kube::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteResult {
    pub success: bool,
    pub message: String,
}

/// Delete a namespaced resource by GVK, namespace, and name.
pub async fn delete_resource(
    client: &Client,
    gvk: &str,
    namespace: &str,
    name: &str,
) -> crate::Result<DeleteResult> {
    match gvk {
        "v1/Pod" => {
            let api: Api<k8s_openapi::api::core::v1::Pod> =
                Api::namespaced(client.clone(), namespace);
            api.delete(name, &DeleteParams::default()).await?;
        }
        "apps/v1/Deployment" => {
            let api: Api<k8s_openapi::api::apps::v1::Deployment> =
                Api::namespaced(client.clone(), namespace);
            api.delete(name, &DeleteParams::default()).await?;
        }
        "apps/v1/StatefulSet" => {
            let api: Api<k8s_openapi::api::apps::v1::StatefulSet> =
                Api::namespaced(client.clone(), namespace);
            api.delete(name, &DeleteParams::default()).await?;
        }
        "apps/v1/DaemonSet" => {
            let api: Api<k8s_openapi::api::apps::v1::DaemonSet> =
                Api::namespaced(client.clone(), namespace);
            api.delete(name, &DeleteParams::default()).await?;
        }
        "batch/v1/Job" => {
            let api: Api<k8s_openapi::api::batch::v1::Job> =
                Api::namespaced(client.clone(), namespace);
            api.delete(name, &DeleteParams::default()).await?;
        }
        "batch/v1/CronJob" => {
            let api: Api<k8s_openapi::api::batch::v1::CronJob> =
                Api::namespaced(client.clone(), namespace);
            api.delete(name, &DeleteParams::default()).await?;
        }
        "v1/Service" => {
            let api: Api<k8s_openapi::api::core::v1::Service> =
                Api::namespaced(client.clone(), namespace);
            api.delete(name, &DeleteParams::default()).await?;
        }
        "v1/ConfigMap" => {
            let api: Api<k8s_openapi::api::core::v1::ConfigMap> =
                Api::namespaced(client.clone(), namespace);
            api.delete(name, &DeleteParams::default()).await?;
        }
        "v1/Secret" => {
            let api: Api<k8s_openapi::api::core::v1::Secret> =
                Api::namespaced(client.clone(), namespace);
            api.delete(name, &DeleteParams::default()).await?;
        }
        "networking.k8s.io/v1/Ingress" => {
            let api: Api<k8s_openapi::api::networking::v1::Ingress> =
                Api::namespaced(client.clone(), namespace);
            api.delete(name, &DeleteParams::default()).await?;
        }
        "v1/PersistentVolumeClaim" => {
            let api: Api<k8s_openapi::api::core::v1::PersistentVolumeClaim> =
                Api::namespaced(client.clone(), namespace);
            api.delete(name, &DeleteParams::default()).await?;
        }
        "apps/v1/ReplicaSet" => {
            let api: Api<k8s_openapi::api::apps::v1::ReplicaSet> =
                Api::namespaced(client.clone(), namespace);
            api.delete(name, &DeleteParams::default()).await?;
        }
        _ => {
            return Ok(DeleteResult {
                success: false,
                message: format!("Delete not supported for GVK: {}", gvk),
            });
        }
    }

    Ok(DeleteResult {
        success: true,
        message: format!("Deleted {}/{} in namespace {}", gvk, name, namespace),
    })
}

/// Scale a Deployment or StatefulSet to the desired replica count.
pub async fn scale_resource(
    client: &Client,
    gvk: &str,
    namespace: &str,
    name: &str,
    replicas: i32,
) -> crate::Result<String> {
    let patch = serde_json::json!({
        "spec": {
            "replicas": replicas
        }
    });
    let patch_params = PatchParams::apply("telescope");

    match gvk {
        "apps/v1/Deployment" => {
            let api: Api<k8s_openapi::api::apps::v1::Deployment> =
                Api::namespaced(client.clone(), namespace);
            api.patch(name, &patch_params, &Patch::Merge(&patch))
                .await?;
        }
        "apps/v1/StatefulSet" => {
            let api: Api<k8s_openapi::api::apps::v1::StatefulSet> =
                Api::namespaced(client.clone(), namespace);
            api.patch(name, &patch_params, &Patch::Merge(&patch))
                .await?;
        }
        _ => {
            return Err(crate::EngineError::Other(format!(
                "Scale not supported for {}",
                gvk
            )));
        }
    }

    Ok(format!("Scaled {}/{} to {} replicas", gvk, name, replicas))
}

/// Rollout status for a Deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolloutStatus {
    pub desired: i32,
    pub ready: i32,
    pub updated: i32,
    pub available: i32,
    pub is_complete: bool,
    pub message: String,
}

/// Restart a Deployment, StatefulSet, or DaemonSet by patching the pod template annotation.
pub async fn rollout_restart(
    client: &Client,
    gvk: &str,
    namespace: &str,
    name: &str,
) -> crate::Result<String> {
    let now = {
        use std::time::SystemTime;
        let d = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        format!("{}Z", d.as_secs())
    };
    let patch = serde_json::json!({
        "spec": {
            "template": {
                "metadata": {
                    "annotations": {
                        "telescope.dev/restartedAt": now
                    }
                }
            }
        }
    });
    let patch_params = PatchParams::apply("telescope");

    match gvk {
        "apps/v1/Deployment" => {
            let api: Api<k8s_openapi::api::apps::v1::Deployment> =
                Api::namespaced(client.clone(), namespace);
            api.patch(name, &patch_params, &Patch::Merge(&patch))
                .await?;
        }
        "apps/v1/StatefulSet" => {
            let api: Api<k8s_openapi::api::apps::v1::StatefulSet> =
                Api::namespaced(client.clone(), namespace);
            api.patch(name, &patch_params, &Patch::Merge(&patch))
                .await?;
        }
        "apps/v1/DaemonSet" => {
            let api: Api<k8s_openapi::api::apps::v1::DaemonSet> =
                Api::namespaced(client.clone(), namespace);
            api.patch(name, &patch_params, &Patch::Merge(&patch))
                .await?;
        }
        _ => {
            return Err(crate::EngineError::Other(format!(
                "Rollout restart not supported for {}",
                gvk
            )));
        }
    }

    Ok(format!("Rollout restart initiated for {}/{}", gvk, name))
}

/// Get rollout status for a Deployment or StatefulSet.
pub async fn rollout_status(
    client: &Client,
    gvk: &str,
    namespace: &str,
    name: &str,
) -> crate::Result<RolloutStatus> {
    match gvk {
        "apps/v1/Deployment" => {
            let api: Api<k8s_openapi::api::apps::v1::Deployment> =
                Api::namespaced(client.clone(), namespace);
            let deploy = api.get(name).await?;

            let spec_replicas = deploy.spec.as_ref().and_then(|s| s.replicas).unwrap_or(1);
            let status = deploy.status.as_ref();
            let ready = status.and_then(|s| s.ready_replicas).unwrap_or(0);
            let updated = status.and_then(|s| s.updated_replicas).unwrap_or(0);
            let available = status.and_then(|s| s.available_replicas).unwrap_or(0);
            let generation = deploy.metadata.generation.unwrap_or(0);
            let observed = status.and_then(|s| s.observed_generation).unwrap_or(0);

            let is_complete = generation <= observed
                && updated == spec_replicas
                && ready == spec_replicas
                && available == spec_replicas;

            Ok(RolloutStatus {
                desired: spec_replicas,
                ready,
                updated,
                available,
                is_complete,
                message: if is_complete {
                    "Rollout complete".into()
                } else {
                    format!("{}/{} ready", ready, spec_replicas)
                },
            })
        }
        "apps/v1/StatefulSet" => {
            let api: Api<k8s_openapi::api::apps::v1::StatefulSet> =
                Api::namespaced(client.clone(), namespace);
            let ss = api.get(name).await?;

            let spec_replicas = ss.spec.as_ref().and_then(|s| s.replicas).unwrap_or(1);
            let status = ss.status.as_ref();
            let ready = status.and_then(|s| s.ready_replicas).unwrap_or(0);
            let updated = status.and_then(|s| s.updated_replicas).unwrap_or(0);
            let current = status.and_then(|s| s.current_replicas).unwrap_or(0);
            let generation = ss.metadata.generation.unwrap_or(0);
            let observed = status.and_then(|s| s.observed_generation).unwrap_or(0);

            let is_complete = generation <= observed
                && updated == spec_replicas
                && ready == spec_replicas
                && current == spec_replicas;

            Ok(RolloutStatus {
                desired: spec_replicas,
                ready,
                updated,
                available: current,
                is_complete,
                message: if is_complete {
                    "Rollout complete".into()
                } else {
                    format!("{}/{} ready", ready, spec_replicas)
                },
            })
        }
        _ => Err(crate::EngineError::Other(format!(
            "Rollout status not supported for {}",
            gvk
        ))),
    }
}

/// Result of applying a resource via server-side apply.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResult {
    pub success: bool,
    pub message: String,
    pub result_yaml: Option<String>,
}

pub const MAX_APPLY_RESOURCE_MANIFEST_BYTES: usize = 1024 * 1024;

pub fn validate_apply_resource_content(content: &str) -> crate::Result<()> {
    if content.trim().is_empty() {
        return Err(crate::EngineError::Other(
            "Manifest must not be empty".to_string(),
        ));
    }

    if content.len() > MAX_APPLY_RESOURCE_MANIFEST_BYTES {
        return Err(crate::EngineError::Other(format!(
            "Manifest exceeds maximum size of {MAX_APPLY_RESOURCE_MANIFEST_BYTES} bytes"
        )));
    }

    Ok(())
}

/// Apply a resource from a YAML or JSON string using server-side apply.
pub async fn apply_resource(
    client: &Client,
    content: &str,
    dry_run: bool,
) -> crate::Result<ApplyResult> {
    validate_apply_resource_content(content)?;

    let value: serde_json::Value = serde_json::from_str(content)
        .or_else(|_| serde_yaml::from_str(content))
        .map_err(|e| crate::EngineError::Other(format!("Invalid YAML/JSON: {}", e)))?;

    let api_version = value["apiVersion"]
        .as_str()
        .ok_or_else(|| crate::EngineError::Other("Missing apiVersion".into()))?;
    let kind = value["kind"]
        .as_str()
        .ok_or_else(|| crate::EngineError::Other("Missing kind".into()))?;
    let name = value["metadata"]["name"]
        .as_str()
        .ok_or_else(|| crate::EngineError::Other("Missing metadata.name".into()))?;
    let namespace = value["metadata"]["namespace"].as_str().unwrap_or("default");

    let gvk_str = format!("{}/{}", api_version, kind);

    let mut patch_params = PatchParams::apply("telescope");
    if dry_run {
        patch_params = patch_params.dry_run();
    }
    patch_params.force = true;

    let result_json: String = match gvk_str.as_str() {
        "v1/Pod" => {
            let api: Api<k8s_openapi::api::core::v1::Pod> =
                Api::namespaced(client.clone(), namespace);
            let res = api
                .patch(name, &patch_params, &Patch::Apply(&value))
                .await?;
            serde_json::to_string_pretty(&res).unwrap_or_default()
        }
        "apps/v1/Deployment" => {
            let api: Api<k8s_openapi::api::apps::v1::Deployment> =
                Api::namespaced(client.clone(), namespace);
            let res = api
                .patch(name, &patch_params, &Patch::Apply(&value))
                .await?;
            serde_json::to_string_pretty(&res).unwrap_or_default()
        }
        "apps/v1/StatefulSet"
        | "apps/v1/DaemonSet"
        | "batch/v1/Job"
        | "batch/v1/CronJob"
        | "v1/Service"
        | "v1/ConfigMap"
        | "v1/Secret" => {
            // For brevity, use a generic typed apply for remaining known kinds.
            // Server-side apply works the same way for all of them.
            apply_typed_resource(client, &gvk_str, namespace, name, &patch_params, &value).await?
        }
        _ => {
            return Ok(ApplyResult {
                success: false,
                message: format!("Apply not supported for {}", gvk_str),
                result_yaml: None,
            });
        }
    };

    Ok(ApplyResult {
        success: true,
        message: if dry_run {
            format!("Dry run succeeded for {}/{} in {}", kind, name, namespace)
        } else {
            format!("Applied {}/{} in {}", kind, name, namespace)
        },
        result_yaml: Some(result_json),
    })
}

async fn apply_typed_resource(
    client: &Client,
    gvk_str: &str,
    namespace: &str,
    name: &str,
    patch_params: &PatchParams,
    value: &serde_json::Value,
) -> Result<String, crate::EngineError> {
    match gvk_str {
        "apps/v1/StatefulSet" => {
            let api: Api<k8s_openapi::api::apps::v1::StatefulSet> =
                Api::namespaced(client.clone(), namespace);
            let res = api.patch(name, patch_params, &Patch::Apply(value)).await?;
            Ok(serde_json::to_string_pretty(&res).unwrap_or_default())
        }
        "apps/v1/DaemonSet" => {
            let api: Api<k8s_openapi::api::apps::v1::DaemonSet> =
                Api::namespaced(client.clone(), namespace);
            let res = api.patch(name, patch_params, &Patch::Apply(value)).await?;
            Ok(serde_json::to_string_pretty(&res).unwrap_or_default())
        }
        "batch/v1/Job" => {
            let api: Api<k8s_openapi::api::batch::v1::Job> =
                Api::namespaced(client.clone(), namespace);
            let res = api.patch(name, patch_params, &Patch::Apply(value)).await?;
            Ok(serde_json::to_string_pretty(&res).unwrap_or_default())
        }
        "batch/v1/CronJob" => {
            let api: Api<k8s_openapi::api::batch::v1::CronJob> =
                Api::namespaced(client.clone(), namespace);
            let res = api.patch(name, patch_params, &Patch::Apply(value)).await?;
            Ok(serde_json::to_string_pretty(&res).unwrap_or_default())
        }
        "v1/Service" => {
            let api: Api<k8s_openapi::api::core::v1::Service> =
                Api::namespaced(client.clone(), namespace);
            let res = api.patch(name, patch_params, &Patch::Apply(value)).await?;
            Ok(serde_json::to_string_pretty(&res).unwrap_or_default())
        }
        "v1/ConfigMap" => {
            let api: Api<k8s_openapi::api::core::v1::ConfigMap> =
                Api::namespaced(client.clone(), namespace);
            let res = api.patch(name, patch_params, &Patch::Apply(value)).await?;
            Ok(serde_json::to_string_pretty(&res).unwrap_or_default())
        }
        "v1/Secret" => {
            let api: Api<k8s_openapi::api::core::v1::Secret> =
                Api::namespaced(client.clone(), namespace);
            let res = api.patch(name, patch_params, &Patch::Apply(value)).await?;
            Ok(serde_json::to_string_pretty(&res).unwrap_or_default())
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_parses_yaml_input() {
        let yaml = "apiVersion: v1
kind: ConfigMap
metadata:
  name: test
  namespace: default
data:
  key: value
";
        let value: serde_json::Value = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(value["apiVersion"].as_str().unwrap(), "v1");
        assert_eq!(value["kind"].as_str().unwrap(), "ConfigMap");
    }

    #[test]
    fn validate_apply_resource_content_rejects_oversized_manifest() {
        let manifest = "a".repeat(MAX_APPLY_RESOURCE_MANIFEST_BYTES + 1);
        let err = validate_apply_resource_content(&manifest).unwrap_err();
        assert!(err.to_string().contains("maximum size"));
    }

    #[test]
    fn validate_apply_resource_content_rejects_empty_manifest() {
        let err = validate_apply_resource_content("   ").unwrap_err();
        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    fn delete_rejects_unsupported_gvk() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = kube::Client::try_default().await;
            if client.is_err() {
                return;
            }
            let client = client.unwrap();
            let result =
                delete_resource(&client, "v1/PersistentVolume", "default", "test-pv").await;
            assert!(result.is_ok());
            let delete_result = result.unwrap();
            assert!(!delete_result.success);
            assert!(
                delete_result.message.contains("not supported"),
                "unexpected message: {}",
                delete_result.message
            );
        });
    }

    #[test]
    fn scale_rejects_unsupported_gvk() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // We cannot construct a real kube::Client without a cluster, but we
            // can verify the unsupported-GVK path returns the expected error.
            let client = kube::Client::try_default().await;
            if client.is_err() {
                // No cluster available — just verify the error variant name.
                return;
            }
            let client = client.unwrap();
            let result = scale_resource(&client, "v1/ConfigMap", "default", "test", 1).await;
            assert!(result.is_err());
            let err_msg = format!("{}", result.unwrap_err());
            assert!(
                err_msg.contains("Scale not supported"),
                "unexpected error: {}",
                err_msg
            );
        });
    }
}
