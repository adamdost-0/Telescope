#![allow(dead_code)]
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use kube::Client;
use tokio::time::sleep;
use tracing::info;

use telescope_core::ResourceStore;

/// Initialize tracing subscriber once (idempotent across tests).
pub fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
}

/// Environment gate for k3d-based integration tests.
pub fn should_run() -> bool {
    std::env::var("K3D_TEST").unwrap_or_default() == "1"
}

/// Create a Kubernetes client via the engine helper.
pub async fn kube_client() -> anyhow::Result<Client> {
    Ok(telescope_engine::client::create_client().await?)
}

/// Apply a fixture file using kubectl (e.g., `tools/k3d-fixtures/nginx-deployment.yaml`).
pub fn kubectl_apply(path: &str) -> anyhow::Result<()> {
    let status = Command::new("kubectl")
        .args(["apply", "-f", path])
        .status()
        .map_err(|e| anyhow::anyhow!("failed to spawn kubectl apply: {e}"))?;
    if !status.success() {
        anyhow::bail!("kubectl apply -f {path} failed with status {status}");
    }
    Ok(())
}

/// Delete a fixture using kubectl; ignores NotFound errors.
pub fn kubectl_delete(path: &str) -> anyhow::Result<()> {
    let status = Command::new("kubectl")
        .args(["delete", "-f", path, "--ignore-not-found=true"])
        .status()
        .map_err(|e| anyhow::anyhow!("failed to spawn kubectl delete: {e}"))?;
    if !status.success() {
        anyhow::bail!("kubectl delete -f {path} failed with status {status}");
    }
    Ok(())
}

/// Wait for a condition up to a timeout, polling every interval.
pub async fn wait_for_condition<F, Fut>(
    timeout_dur: Duration,
    poll_every: Duration,
    mut f: F,
) -> anyhow::Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<bool>>,
{
    use tokio::time::Instant;
    let deadline = Instant::now() + timeout_dur;
    loop {
        if f().await? {
            return Ok(());
        }
        if Instant::now() >= deadline {
            anyhow::bail!("timed out waiting for condition");
        }
        sleep(poll_every).await;
    }
}

/// Poll the ResourceStore until it has at least `min` entries for the given GVK.
pub async fn wait_for_store_count(
    store: Arc<std::sync::Mutex<ResourceStore>>,
    gvk: &str,
    namespace: Option<&str>,
    min: u64,
    timeout_dur: Duration,
) -> anyhow::Result<()> {
    wait_for_condition(timeout_dur, Duration::from_millis(500), || {
        let store = store.clone();
        let gvk = gvk.to_string();
        async move {
            let count = store
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .count(&gvk, namespace)
                .map_err(|e| anyhow::anyhow!("count query failed: {e}"))?;
            Ok(count >= min)
        }
    })
    .await
}

/// Helper to wait for a specific resource to appear in the store.
pub async fn wait_for_store_get(
    store: Arc<std::sync::Mutex<ResourceStore>>,
    gvk: &str,
    namespace: &str,
    name: &str,
    timeout_dur: Duration,
) -> anyhow::Result<telescope_core::ResourceEntry> {
    wait_for_condition(timeout_dur, Duration::from_millis(500), || {
        let store = store.clone();
        let gvk = gvk.to_string();
        let namespace = namespace.to_string();
        let name = name.to_string();
        async move {
            let entry = store
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&gvk, &namespace, &name)
                .map_err(|e| anyhow::anyhow!("get query failed: {e}"))?;
            Ok(entry.is_some())
        }
    })
    .await?;

    let entry = store
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(gvk, namespace, name)
        .map_err(|e| anyhow::anyhow!("get query failed: {e}"))?;

    entry.ok_or_else(|| anyhow::anyhow!("resource {gvk}/{namespace}/{name} not found after wait"))
}

/// Wait for the ResourceWatcher to report Ready state.
pub async fn wait_for_ready_state(
    mut rx: tokio::sync::watch::Receiver<telescope_core::ConnectionState>,
    timeout_dur: Duration,
) -> anyhow::Result<()> {
    use telescope_core::ConnectionState;
    use tokio::time::Instant;

    let deadline = Instant::now() + timeout_dur;
    loop {
        if *rx.borrow() == ConnectionState::Ready {
            return Ok(());
        }
        if Instant::now() >= deadline {
            anyhow::bail!("timed out waiting for Ready state");
        }
        if rx.changed().await.is_err() {
            // sender dropped; bail out rather than spinning forever
            anyhow::bail!("connection state channel closed before Ready");
        }
    }
}

/// Ensure k3d fixtures are applied (idempotent).
pub fn ensure_fixtures_applied() -> anyhow::Result<()> {
    // Apply all manifests in tools/k3d-fixtures
    kubectl_apply("tools/k3d-fixtures/").map(|_| info!("fixtures applied"))?;
    Ok(())
}
