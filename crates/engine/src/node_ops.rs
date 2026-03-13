//! Node operations: cordon, uncordon, drain, and taint management.

use k8s_openapi::api::core::v1::{Node, Pod, Taint};
use k8s_openapi::api::policy::v1::Eviction;
use kube::api::{Api, ListParams, ObjectMeta, Patch, PatchParams, PostParams};
use kube::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Result of a drain operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrainResult {
    pub success: bool,
    pub message: String,
    pub evicted_pods: Vec<String>,
    pub skipped_pods: Vec<String>,
}

/// Options for the drain operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrainOptions {
    #[serde(default = "default_grace_period")]
    pub grace_period: i64,
    #[serde(default = "default_true")]
    pub ignore_daemonsets: bool,
    #[serde(default)]
    pub force: bool,
}

fn default_grace_period() -> i64 {
    30
}
fn default_true() -> bool {
    true
}

impl Default for DrainOptions {
    fn default() -> Self {
        Self {
            grace_period: 30,
            ignore_daemonsets: true,
            force: false,
        }
    }
}

/// Mark a node as unschedulable (cordon).
pub async fn cordon_node(client: &Client, name: &str) -> crate::Result<String> {
    let api: Api<Node> = Api::all(client.clone());
    let patch = serde_json::json!({ "spec": { "unschedulable": true } });
    let patch_params = PatchParams::apply("telescope");
    api.patch(name, &patch_params, &Patch::Merge(&patch))
        .await?;
    info!("Node {} cordoned", name);
    Ok(format!("Node {} cordoned", name))
}

/// Mark a node as schedulable (uncordon).
pub async fn uncordon_node(client: &Client, name: &str) -> crate::Result<String> {
    let api: Api<Node> = Api::all(client.clone());
    let patch = serde_json::json!({ "spec": { "unschedulable": false } });
    let patch_params = PatchParams::apply("telescope");
    api.patch(name, &patch_params, &Patch::Merge(&patch))
        .await?;
    info!("Node {} uncordoned", name);
    Ok(format!("Node {} uncordoned", name))
}

/// Add a taint to a node.
pub async fn add_taint(
    client: &Client,
    node_name: &str,
    key: &str,
    value: &str,
    effect: &str,
) -> crate::Result<String> {
    let api: Api<Node> = Api::all(client.clone());
    let node = api.get(node_name).await?;
    let mut taints = node
        .spec
        .as_ref()
        .and_then(|s| s.taints.clone())
        .unwrap_or_default();

    // Remove existing taint with the same key to avoid duplicates.
    taints.retain(|t| t.key != key);
    taints.push(Taint {
        key: key.to_string(),
        value: Some(value.to_string()),
        effect: effect.to_string(),
        time_added: None,
    });

    let patch = serde_json::json!({ "spec": { "taints": taints } });
    let patch_params = PatchParams::apply("telescope");
    api.patch(node_name, &patch_params, &Patch::Merge(&patch))
        .await?;
    info!("Taint {}={}:{} added to {}", key, value, effect, node_name);
    Ok(format!(
        "Taint {}={}:{} added to {}",
        key, value, effect, node_name
    ))
}

/// Remove a taint from a node by key.
pub async fn remove_taint(client: &Client, node_name: &str, key: &str) -> crate::Result<String> {
    let api: Api<Node> = Api::all(client.clone());
    let node = api.get(node_name).await?;
    let taints: Vec<Taint> = node
        .spec
        .as_ref()
        .and_then(|s| s.taints.clone())
        .unwrap_or_default()
        .into_iter()
        .filter(|t| t.key != key)
        .collect();

    let patch = serde_json::json!({
        "spec": {
            "taints": if taints.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::to_value(&taints).unwrap_or(serde_json::Value::Null)
            }
        }
    });
    let patch_params = PatchParams::apply("telescope");
    api.patch(node_name, &patch_params, &Patch::Merge(&patch))
        .await?;
    info!("Taint {} removed from {}", key, node_name);
    Ok(format!("Taint {} removed from {}", key, node_name))
}

/// Drain a node: cordon it, then evict eligible pods.
pub async fn drain_node(
    client: &Client,
    name: &str,
    options: &DrainOptions,
) -> crate::Result<DrainResult> {
    // Step 1: cordon the node.
    cordon_node(client, name).await?;

    // Step 2: list pods on this node.
    let pod_api: Api<Pod> = Api::all(client.clone());
    let lp = ListParams::default().fields(&format!("spec.nodeName={}", name));
    let pod_list = pod_api.list(&lp).await?;

    let mut evicted = Vec::new();
    let mut skipped = Vec::new();

    for pod in pod_list {
        let pod_name = pod.metadata.name.clone().unwrap_or_default();
        let pod_ns = pod
            .metadata
            .namespace
            .clone()
            .unwrap_or_else(|| "default".to_string());

        // Skip mirror pods (created by kubelet).
        if let Some(annotations) = &pod.metadata.annotations {
            if annotations.contains_key("kubernetes.io/config.mirror") {
                skipped.push(format!("{}/{} (mirror pod)", pod_ns, pod_name));
                continue;
            }
        }

        // Skip DaemonSet pods if ignore_daemonsets is set.
        if options.ignore_daemonsets {
            if let Some(owner_refs) = &pod.metadata.owner_references {
                if owner_refs.iter().any(|o| o.kind == "DaemonSet") {
                    skipped.push(format!("{}/{} (DaemonSet)", pod_ns, pod_name));
                    continue;
                }
            }
        }

        // Evict the pod.
        let ns_api: Api<Pod> = Api::namespaced(client.clone(), &pod_ns);
        let eviction = Eviction {
            metadata: ObjectMeta {
                name: Some(pod_name.clone()),
                namespace: Some(pod_ns.clone()),
                ..Default::default()
            },
            delete_options: Some(
                k8s_openapi::apimachinery::pkg::apis::meta::v1::DeleteOptions {
                    grace_period_seconds: Some(options.grace_period),
                    ..Default::default()
                },
            ),
        };

        match ns_api
            .create_subresource::<Eviction, Eviction>(
                "eviction",
                &pod_name,
                &PostParams::default(),
                &eviction,
            )
            .await
        {
            Ok(_) => evicted.push(format!("{}/{}", pod_ns, pod_name)),
            Err(e) => {
                if options.force {
                    skipped.push(format!("{}/{} (eviction failed: {})", pod_ns, pod_name, e));
                } else {
                    return Ok(DrainResult {
                        success: false,
                        message: format!("Failed to evict pod {}/{}: {}", pod_ns, pod_name, e),
                        evicted_pods: evicted,
                        skipped_pods: skipped,
                    });
                }
            }
        }
    }

    let msg = format!(
        "Node {} drained: {} evicted, {} skipped",
        name,
        evicted.len(),
        skipped.len()
    );
    info!("{}", msg);
    Ok(DrainResult {
        success: true,
        message: msg,
        evicted_pods: evicted,
        skipped_pods: skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_options_defaults() {
        let opts = DrainOptions::default();
        assert_eq!(opts.grace_period, 30);
        assert!(opts.ignore_daemonsets);
        assert!(!opts.force);
    }

    #[test]
    fn drain_options_deserialize() {
        let json = r#"{"grace_period": 60, "ignore_daemonsets": false, "force": true}"#;
        let opts: DrainOptions = serde_json::from_str(json).unwrap();
        assert_eq!(opts.grace_period, 60);
        assert!(!opts.ignore_daemonsets);
        assert!(opts.force);
    }

    #[test]
    fn drain_result_serializes() {
        let result = DrainResult {
            success: true,
            message: "Drained".into(),
            evicted_pods: vec!["default/pod-1".into()],
            skipped_pods: vec!["kube-system/kube-proxy (DaemonSet)".into()],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("evicted_pods"));
        assert!(json.contains("skipped_pods"));
    }
}
