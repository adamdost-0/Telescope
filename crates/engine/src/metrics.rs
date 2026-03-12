//! Kubernetes metrics-server integration (metrics.k8s.io/v1beta1).

use kube::Client;
use serde::{Deserialize, Serialize};

/// Aggregated CPU/memory metrics for a single pod.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodMetrics {
    pub name: String,
    pub namespace: String,
    pub containers: Vec<ContainerMetrics>,
    /// Sum of all container CPU usage in millicores.
    pub cpu_millicores: u64,
    /// Sum of all container memory usage in bytes.
    pub memory_bytes: u64,
}

/// CPU/memory metrics for a single container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMetrics {
    pub name: String,
    pub cpu_millicores: u64,
    pub memory_bytes: u64,
}

/// CPU/memory metrics for a node, including allocatable capacity for percentage display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetricsData {
    pub name: String,
    /// Current CPU usage in millicores.
    pub cpu_millicores: u64,
    /// Current memory usage in bytes.
    pub memory_bytes: u64,
    /// Allocatable CPU in millicores (from node status).
    pub cpu_allocatable: u64,
    /// Allocatable memory in bytes (from node status).
    pub memory_allocatable: u64,
    /// CPU usage as percentage of allocatable (0.0–100.0).
    pub cpu_percent: f64,
    /// Memory usage as percentage of allocatable (0.0–100.0).
    pub memory_percent: f64,
}

/// Check whether the metrics-server API is reachable.
pub async fn is_metrics_available(client: &Client) -> bool {
    let req = match http::Request::get("/apis/metrics.k8s.io/v1beta1").body(vec![]) {
        Ok(r) => r,
        Err(_) => return false,
    };
    client.request_text(req).await.is_ok()
}

/// Fetch pod metrics for one namespace, or all namespaces when `namespace` is `None`.
pub async fn get_pod_metrics(
    client: &Client,
    namespace: Option<&str>,
) -> crate::Result<Vec<PodMetrics>> {
    let url = match namespace {
        Some(ns) => format!("/apis/metrics.k8s.io/v1beta1/namespaces/{ns}/pods"),
        None => "/apis/metrics.k8s.io/v1beta1/pods".to_string(),
    };

    let req = http::Request::get(&url)
        .body(vec![])
        .map_err(|e| crate::EngineError::Other(e.to_string()))?;

    let response: serde_json::Value = client
        .request(req)
        .await
        .map_err(|e| crate::EngineError::Other(format!("Metrics API error: {e}")))?;

    let empty = vec![];
    let items = response["items"].as_array().unwrap_or(&empty);

    let metrics: Vec<PodMetrics> = items
        .iter()
        .map(|item| {
            let name = item["metadata"]["name"].as_str().unwrap_or("").to_string();
            let namespace = item["metadata"]["namespace"]
                .as_str()
                .unwrap_or("")
                .to_string();

            let empty_arr = vec![];
            let containers: Vec<ContainerMetrics> = item["containers"]
                .as_array()
                .unwrap_or(&empty_arr)
                .iter()
                .map(|c| ContainerMetrics {
                    name: c["name"].as_str().unwrap_or("").to_string(),
                    cpu_millicores: parse_cpu(c["usage"]["cpu"].as_str().unwrap_or("0")),
                    memory_bytes: parse_memory(c["usage"]["memory"].as_str().unwrap_or("0")),
                })
                .collect();

            let cpu_total = containers.iter().map(|c| c.cpu_millicores).sum();
            let mem_total = containers.iter().map(|c| c.memory_bytes).sum();

            PodMetrics {
                name,
                namespace,
                containers,
                cpu_millicores: cpu_total,
                memory_bytes: mem_total,
            }
        })
        .collect();

    Ok(metrics)
}

/// Fetch node metrics, cross-referenced with node allocatable capacity.
///
/// Queries the metrics-server API for current usage, then fetches each node's
/// allocatable resources to compute usage percentages.
pub async fn get_node_metrics(client: &Client) -> crate::Result<Vec<NodeMetricsData>> {
    let metrics_url = "/apis/metrics.k8s.io/v1beta1/nodes";

    let req = http::Request::get(metrics_url)
        .body(vec![])
        .map_err(|e| crate::EngineError::Other(e.to_string()))?;

    let response: serde_json::Value = client
        .request(req)
        .await
        .map_err(|e| crate::EngineError::Other(format!("Node metrics API error: {e}")))?;

    let empty = vec![];
    let items = response["items"].as_array().unwrap_or(&empty);

    // Fetch node objects for allocatable capacity
    use k8s_openapi::api::core::v1::Node;
    use kube::Api;
    let nodes_api: Api<Node> = Api::all(client.clone());
    let node_list = nodes_api
        .list(&Default::default())
        .await
        .map_err(|e| crate::EngineError::Other(format!("Failed to list nodes: {e}")))?;

    let mut alloc_map: std::collections::HashMap<String, (u64, u64)> =
        std::collections::HashMap::new();
    for node in &node_list.items {
        let name = node.metadata.name.as_deref().unwrap_or("").to_string();
        let alloc = node.status.as_ref().and_then(|s| s.allocatable.as_ref());
        let cpu = alloc
            .and_then(|a| a.get("cpu"))
            .map(|q| parse_cpu(&q.0))
            .unwrap_or(0);
        let mem = alloc
            .and_then(|a| a.get("memory"))
            .map(|q| parse_memory(&q.0))
            .unwrap_or(0);
        alloc_map.insert(name, (cpu, mem));
    }

    let metrics: Vec<NodeMetricsData> = items
        .iter()
        .map(|item| {
            let name = item["metadata"]["name"].as_str().unwrap_or("").to_string();
            let cpu_used = parse_cpu(item["usage"]["cpu"].as_str().unwrap_or("0"));
            let mem_used = parse_memory(item["usage"]["memory"].as_str().unwrap_or("0"));

            let (cpu_alloc, mem_alloc) = alloc_map.get(&name).copied().unwrap_or((0, 0));

            let cpu_pct = if cpu_alloc > 0 {
                (cpu_used as f64 / cpu_alloc as f64) * 100.0
            } else {
                0.0
            };
            let mem_pct = if mem_alloc > 0 {
                (mem_used as f64 / mem_alloc as f64) * 100.0
            } else {
                0.0
            };

            NodeMetricsData {
                name,
                cpu_millicores: cpu_used,
                memory_bytes: mem_used,
                cpu_allocatable: cpu_alloc,
                memory_allocatable: mem_alloc,
                cpu_percent: (cpu_pct * 10.0).round() / 10.0,
                memory_percent: (mem_pct * 10.0).round() / 10.0,
            }
        })
        .collect();

    Ok(metrics)
}

/// Parse a Kubernetes CPU quantity (e.g. `"250m"`, `"1"`, `"500n"`) to millicores.
pub(crate) fn parse_cpu(s: &str) -> u64 {
    if s.ends_with('n') {
        s.trim_end_matches('n')
            .parse::<u64>()
            .unwrap_or(0)
            .saturating_div(1_000_000)
    } else if s.ends_with('m') {
        s.trim_end_matches('m').parse::<u64>().unwrap_or(0)
    } else {
        s.parse::<u64>().unwrap_or(0).saturating_mul(1000)
    }
}

/// Parse a Kubernetes memory quantity (e.g. `"128Mi"`, `"1Gi"`, `"65536Ki"`) to bytes.
pub(crate) fn parse_memory(s: &str) -> u64 {
    if s.ends_with("Ki") {
        s.trim_end_matches("Ki")
            .parse::<u64>()
            .unwrap_or(0)
            .saturating_mul(1024)
    } else if s.ends_with("Mi") {
        s.trim_end_matches("Mi")
            .parse::<u64>()
            .unwrap_or(0)
            .saturating_mul(1024 * 1024)
    } else if s.ends_with("Gi") {
        s.trim_end_matches("Gi")
            .parse::<u64>()
            .unwrap_or(0)
            .saturating_mul(1024 * 1024 * 1024)
    } else {
        s.parse::<u64>().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cpu_nanocores() {
        assert_eq!(parse_cpu("500000000n"), 500); // 500 millicores
        assert_eq!(parse_cpu("1000000n"), 1);
        assert_eq!(parse_cpu("0n"), 0);
    }

    #[test]
    fn parse_cpu_millicores() {
        assert_eq!(parse_cpu("250m"), 250);
        assert_eq!(parse_cpu("1000m"), 1000);
        assert_eq!(parse_cpu("0m"), 0);
    }

    #[test]
    fn parse_cpu_whole_cores() {
        assert_eq!(parse_cpu("1"), 1000);
        assert_eq!(parse_cpu("4"), 4000);
        assert_eq!(parse_cpu("0"), 0);
    }

    #[test]
    fn parse_cpu_invalid() {
        assert_eq!(parse_cpu(""), 0);
        assert_eq!(parse_cpu("abc"), 0);
    }

    #[test]
    fn parse_memory_ki() {
        assert_eq!(parse_memory("1024Ki"), 1024 * 1024);
        assert_eq!(parse_memory("65536Ki"), 65536 * 1024);
    }

    #[test]
    fn parse_memory_mi() {
        assert_eq!(parse_memory("128Mi"), 128 * 1024 * 1024);
        assert_eq!(parse_memory("256Mi"), 256 * 1024 * 1024);
    }

    #[test]
    fn parse_memory_gi() {
        assert_eq!(parse_memory("1Gi"), 1024 * 1024 * 1024);
        assert_eq!(parse_memory("2Gi"), 2 * 1024 * 1024 * 1024);
    }

    #[test]
    fn parse_memory_bytes() {
        assert_eq!(parse_memory("1048576"), 1048576);
        assert_eq!(parse_memory("0"), 0);
    }

    #[test]
    fn parse_memory_invalid() {
        assert_eq!(parse_memory(""), 0);
        assert_eq!(parse_memory("abc"), 0);
    }
}
