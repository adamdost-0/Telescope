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

#[derive(Debug, Deserialize)]
struct MetricsList<T> {
    items: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct NamespacedMetadata {
    name: String,
    namespace: String,
}

#[derive(Debug, Deserialize)]
struct NamedMetadata {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ResourceUsage {
    cpu: String,
    memory: String,
}

#[derive(Debug, Deserialize)]
struct RawContainerMetrics {
    name: String,
    usage: ResourceUsage,
}

#[derive(Debug, Deserialize)]
struct RawPodMetrics {
    metadata: NamespacedMetadata,
    containers: Vec<RawContainerMetrics>,
}

#[derive(Debug, Deserialize)]
struct RawNodeMetrics {
    metadata: NamedMetadata,
    usage: ResourceUsage,
}

#[derive(Debug, Clone, Copy)]
enum QuantityTarget {
    CpuMillicores,
    Bytes,
}

impl QuantityTarget {
    fn label(self) -> &'static str {
        match self {
            Self::CpuMillicores => "CPU",
            Self::Bytes => "memory",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ParsedDecimal {
    digits: u128,
    scale: u32,
}

impl ParsedDecimal {
    fn parse(raw: &str) -> std::result::Result<Self, &'static str> {
        if raw.is_empty() {
            return Err("quantity is empty");
        }

        if raw.starts_with('-') {
            return Err("negative quantities are not supported");
        }

        let raw = raw.strip_prefix('+').unwrap_or(raw);
        let mut parts = raw.split('.');
        let whole = parts.next().unwrap_or_default();
        let fractional = parts.next().unwrap_or_default();

        if parts.next().is_some() {
            return Err("quantity contains multiple decimal points");
        }

        if whole.is_empty() && fractional.is_empty() {
            return Err("quantity is empty");
        }

        if whole.is_empty() {
            return Err("quantity is missing a whole-number component");
        }

        if !whole.chars().all(|c| c.is_ascii_digit())
            || !fractional.chars().all(|c| c.is_ascii_digit())
        {
            return Err("quantity contains invalid characters");
        }

        let digits = format!("{whole}{fractional}")
            .parse::<u128>()
            .map_err(|_| "quantity exceeds supported precision")?;

        Ok(Self {
            digits,
            scale: fractional.len() as u32,
        })
    }

    fn into_u64(
        self,
        quantity: &str,
        target: QuantityTarget,
        multiplier: u128,
        divisor: u128,
    ) -> crate::Result<u64> {
        let numerator = self
            .digits
            .checked_mul(multiplier)
            .ok_or_else(|| quantity_error(target, quantity, "value exceeds supported range"))?;
        let decimal_divisor = 10u128
            .checked_pow(self.scale)
            .ok_or_else(|| quantity_error(target, quantity, "value exceeds supported precision"))?;
        let denominator = decimal_divisor
            .checked_mul(divisor)
            .ok_or_else(|| quantity_error(target, quantity, "value exceeds supported precision"))?;
        let scaled = numerator / denominator;

        u64::try_from(scaled)
            .map_err(|_| quantity_error(target, quantity, "value exceeds supported range"))
    }
}

fn quantity_error(
    target: QuantityTarget,
    quantity: &str,
    reason: impl AsRef<str>,
) -> crate::EngineError {
    crate::EngineError::Other(format!(
        "Invalid Kubernetes {} quantity \"{}\": {}",
        target.label(),
        quantity,
        reason.as_ref()
    ))
}

fn parse_cpu_value(quantity: &str, context: &str) -> crate::Result<u64> {
    parse_cpu(quantity).map_err(|error| {
        tracing::warn!(
            quantity = quantity,
            context = context,
            error = %error,
            "Failed to parse Kubernetes CPU quantity"
        );
        error
    })
}

fn parse_memory_value(quantity: &str, context: &str) -> crate::Result<u64> {
    parse_memory(quantity).map_err(|error| {
        tracing::warn!(
            quantity = quantity,
            context = context,
            error = %error,
            "Failed to parse Kubernetes memory quantity"
        );
        error
    })
}

fn parse_scaled_quantity(
    quantity: &str,
    raw_number: &str,
    target: QuantityTarget,
    multiplier: u128,
    divisor: u128,
) -> crate::Result<u64> {
    ParsedDecimal::parse(raw_number)
        .map_err(|reason| quantity_error(target, quantity, reason))?
        .into_u64(quantity, target, multiplier, divisor)
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

    let response: MetricsList<RawPodMetrics> = client
        .request(req)
        .await
        .map_err(|e| crate::EngineError::Other(format!("Metrics API error: {e}")))?;

    response
        .items
        .into_iter()
        .map(|item| {
            let name = item.metadata.name;
            let namespace = item.metadata.namespace;

            let containers: Vec<ContainerMetrics> = item
                .containers
                .into_iter()
                .map(|container| {
                    let container_name = container.name;
                    let context = format!("pod {namespace}/{name} container {container_name}");

                    Ok(ContainerMetrics {
                        name: container_name,
                        cpu_millicores: parse_cpu_value(
                            &container.usage.cpu,
                            &format!("{context} cpu"),
                        )?,
                        memory_bytes: parse_memory_value(
                            &container.usage.memory,
                            &format!("{context} memory"),
                        )?,
                    })
                })
                .collect::<crate::Result<_>>()?;

            let cpu_total = containers.iter().map(|c| c.cpu_millicores).sum();
            let mem_total = containers.iter().map(|c| c.memory_bytes).sum();

            Ok(PodMetrics {
                name,
                namespace,
                containers,
                cpu_millicores: cpu_total,
                memory_bytes: mem_total,
            })
        })
        .collect()
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

    let response: MetricsList<RawNodeMetrics> = client
        .request(req)
        .await
        .map_err(|e| crate::EngineError::Other(format!("Node metrics API error: {e}")))?;

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
        let name = node.metadata.name.clone().ok_or_else(|| {
            crate::EngineError::Other("Encountered node without metadata.name".to_string())
        })?;
        let alloc = node
            .status
            .as_ref()
            .and_then(|status| status.allocatable.as_ref())
            .ok_or_else(|| {
                crate::EngineError::Other(format!(
                    "Node {name} did not report allocatable CPU and memory"
                ))
            })?;
        let cpu = alloc
            .get("cpu")
            .ok_or_else(|| {
                crate::EngineError::Other(format!("Node {name} did not report allocatable CPU"))
            })
            .and_then(|quantity| {
                parse_cpu_value(&quantity.0, &format!("node {name} allocatable cpu"))
            })?;
        let mem = alloc
            .get("memory")
            .ok_or_else(|| {
                crate::EngineError::Other(format!("Node {name} did not report allocatable memory"))
            })
            .and_then(|quantity| {
                parse_memory_value(&quantity.0, &format!("node {name} allocatable memory"))
            })?;
        alloc_map.insert(name, (cpu, mem));
    }

    response
        .items
        .into_iter()
        .map(|item| {
            let name = item.metadata.name;
            let cpu_used = parse_cpu_value(&item.usage.cpu, &format!("node {name} usage cpu"))?;
            let mem_used =
                parse_memory_value(&item.usage.memory, &format!("node {name} usage memory"))?;

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

            Ok(NodeMetricsData {
                name,
                cpu_millicores: cpu_used,
                memory_bytes: mem_used,
                cpu_allocatable: cpu_alloc,
                memory_allocatable: mem_alloc,
                cpu_percent: (cpu_pct * 10.0).round() / 10.0,
                memory_percent: (mem_pct * 10.0).round() / 10.0,
            })
        })
        .collect()
}

/// Parse a Kubernetes CPU quantity (e.g. `"250m"`, `"1"`, `"500n"`) to millicores.
///
/// Valid quantities smaller than one millicore are rounded down because the
/// returned representation only tracks whole millicores.
pub(crate) fn parse_cpu(s: &str) -> crate::Result<u64> {
    let quantity = s.trim();
    if let Some(value) = quantity.strip_suffix('n') {
        return parse_scaled_quantity(quantity, value, QuantityTarget::CpuMillicores, 1, 1_000_000);
    }
    if let Some(value) = quantity.strip_suffix('u') {
        return parse_scaled_quantity(quantity, value, QuantityTarget::CpuMillicores, 1, 1_000);
    }
    if let Some(value) = quantity.strip_suffix('m') {
        return parse_scaled_quantity(quantity, value, QuantityTarget::CpuMillicores, 1, 1);
    }
    if quantity
        .chars()
        .last()
        .is_some_and(|ch| ch.is_ascii_alphabetic())
    {
        return Err(quantity_error(
            QuantityTarget::CpuMillicores,
            quantity,
            "unsupported unit suffix",
        ));
    }

    parse_scaled_quantity(quantity, quantity, QuantityTarget::CpuMillicores, 1_000, 1)
}

/// Parse a Kubernetes memory quantity (e.g. `"128Mi"`, `"1Gi"`, `"65536Ki"`) to bytes.
///
/// Valid quantities smaller than one byte are rounded down because the returned
/// representation only tracks whole bytes.
pub(crate) fn parse_memory(s: &str) -> crate::Result<u64> {
    let quantity = s.trim();
    for (suffix, multiplier) in [
        ("Ki", 1_024u128),
        ("Mi", 1_048_576u128),
        ("Gi", 1_073_741_824u128),
        ("Ti", 1_099_511_627_776u128),
        ("Pi", 1_125_899_906_842_624u128),
        ("Ei", 1_152_921_504_606_846_976u128),
        ("K", 1_000u128),
        ("M", 1_000_000u128),
        ("G", 1_000_000_000u128),
        ("T", 1_000_000_000_000u128),
        ("P", 1_000_000_000_000_000u128),
        ("E", 1_000_000_000_000_000_000u128),
    ] {
        if let Some(value) = quantity.strip_suffix(suffix) {
            return parse_scaled_quantity(quantity, value, QuantityTarget::Bytes, multiplier, 1);
        }
    }
    if quantity
        .chars()
        .last()
        .is_some_and(|ch| ch.is_ascii_alphabetic())
    {
        return Err(quantity_error(
            QuantityTarget::Bytes,
            quantity,
            "unsupported unit suffix",
        ));
    }

    parse_scaled_quantity(quantity, quantity, QuantityTarget::Bytes, 1, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cpu_nanocores() {
        assert_eq!(parse_cpu("500000000n").unwrap(), 500); // 500 millicores
        assert_eq!(parse_cpu("1000000n").unwrap(), 1);
        assert_eq!(parse_cpu("0n").unwrap(), 0);
    }

    #[test]
    fn parse_cpu_millicores() {
        assert_eq!(parse_cpu("250m").unwrap(), 250);
        assert_eq!(parse_cpu("1000m").unwrap(), 1000);
        assert_eq!(parse_cpu("0m").unwrap(), 0);
    }

    #[test]
    fn parse_cpu_whole_cores() {
        assert_eq!(parse_cpu("1").unwrap(), 1000);
        assert_eq!(parse_cpu("4").unwrap(), 4000);
        assert_eq!(parse_cpu("0").unwrap(), 0);
    }

    #[test]
    fn parse_cpu_supports_decimal_cores_and_large_nanocores() {
        assert_eq!(parse_cpu("1.5").unwrap(), 1500);
        assert_eq!(
            parse_cpu("99999999999999999999n").unwrap(),
            99_999_999_999_999
        );
    }

    #[test]
    fn parse_cpu_invalid() {
        for quantity in ["", "abc", "-100m", "NaN", "Infinity"] {
            assert!(parse_cpu(quantity).is_err(), "{quantity} should fail");
        }
    }

    #[test]
    fn parse_memory_ki() {
        assert_eq!(parse_memory("1024Ki").unwrap(), 1024 * 1024);
        assert_eq!(parse_memory("65536Ki").unwrap(), 65536 * 1024);
    }

    #[test]
    fn parse_memory_mi() {
        assert_eq!(parse_memory("128Mi").unwrap(), 128 * 1024 * 1024);
        assert_eq!(parse_memory("256Mi").unwrap(), 256 * 1024 * 1024);
    }

    #[test]
    fn parse_memory_gi() {
        assert_eq!(parse_memory("1Gi").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_memory("2Gi").unwrap(), 2 * 1024 * 1024 * 1024);
    }

    #[test]
    fn parse_memory_bytes() {
        assert_eq!(parse_memory("1048576").unwrap(), 1048576);
        assert_eq!(parse_memory("0").unwrap(), 0);
    }

    #[test]
    fn parse_memory_supports_fractional_units() {
        assert_eq!(parse_memory("1.5Gi").unwrap(), 1_610_612_736);
        assert_eq!(parse_memory("0.5Ki").unwrap(), 512);
    }

    #[test]
    fn parse_memory_invalid() {
        for quantity in ["", "abc", "-1Gi", "NaN", "Infinity", "12Zi"] {
            assert!(parse_memory(quantity).is_err(), "{quantity} should fail");
        }
    }
}
