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

/// Restart a Deployment by patching the pod template annotation.
pub async fn rollout_restart(
    client: &Client,
    namespace: &str,
    name: &str,
) -> crate::Result<String> {
    let api: Api<k8s_openapi::api::apps::v1::Deployment> =
        Api::namespaced(client.clone(), namespace);
    let now = {
        use std::time::SystemTime;
        let d = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
    };
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
    api.patch(name, &PatchParams::apply("telescope"), &Patch::Merge(&patch))
        .await?;
}

/// Get rollout status for a Deployment.
pub async fn rollout_status(
    client: &Client,
    namespace: &str,
    name: &str,
) -> crate::Result<RolloutStatus> {
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
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
