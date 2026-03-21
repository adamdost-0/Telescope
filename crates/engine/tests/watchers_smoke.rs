#![allow(clippy::unused_async)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Context;
use k8s_openapi::api::core::v1::Pod;
use kube::Api;
use serde_json::Value;
use tokio::task::JoinHandle;

mod common;

use common::{
    ensure_fixtures_applied, kube_client, should_run, wait_for_ready_state, wait_for_store_count,
    wait_for_store_get,
};

#[tokio::test]
async fn watcher_syncs_pods_and_services() -> anyhow::Result<()> {
    if !should_run() {
        eprintln!("Skipping k3d watcher test (set K3D_TEST=1)");
        return Ok(());
    }
    common::init_tracing();
    ensure_fixtures_applied()?;

    let client: kube::Client = kube_client().await?;
    let store = Arc::new(Mutex::new(telescope_core::ResourceStore::open(":memory:")?));
    let watcher = telescope_engine::watcher::ResourceWatcher::new(client.clone(), store.clone());

    // We will run two watches (pods + services)
    watcher.register_watches(2);
    let state_rx = watcher.state_receiver();

    let pod_watch: JoinHandle<telescope_engine::Result<()>> = tokio::spawn({
        let watcher = watcher.clone();
        async move { watcher.watch_pods("default").await }
    });
    let svc_watch: JoinHandle<telescope_engine::Result<()>> = tokio::spawn({
        let watcher = watcher.clone();
        async move { watcher.watch_services("default").await }
    });

    // Wait for Ready state, giving watchers time to list + watch
    wait_for_ready_state(state_rx, Duration::from_secs(60)).await?;

    // Ensure store has ingested pods/services
    wait_for_store_count(
        store.clone(),
        "v1/Pod",
        Some("default"),
        1,
        Duration::from_secs(30),
    )
    .await?;
    wait_for_store_count(
        store.clone(),
        "v1/Service",
        Some("default"),
        1,
        Duration::from_secs(30),
    )
    .await?;

    // Cleanly abort watchers to avoid leaking tasks in CI
    pod_watch.abort();
    svc_watch.abort();

    Ok(())
}

#[tokio::test]
async fn watcher_redacts_pod_env_values_in_store() -> anyhow::Result<()> {
    if !should_run() {
        return Ok(());
    }
    common::init_tracing();
    ensure_fixtures_applied()?;

    let client: kube::Client = kube_client().await?;
    let store = Arc::new(Mutex::new(telescope_core::ResourceStore::open(":memory:")?));
    let watcher = telescope_engine::watcher::ResourceWatcher::new(client.clone(), store.clone());
    watcher.register_watches(1);
    let state_rx = watcher.state_receiver();

    let watch_handle = tokio::spawn({
        let watcher = watcher.clone();
        async move { watcher.watch_pods("default").await }
    });

    wait_for_ready_state(state_rx, Duration::from_secs(60)).await?;

    // Create a short-lived pod with inline env value
    let pods: Api<Pod> = Api::namespaced(client.clone(), "default");
    let pod_name = format!(
        "env-redaction-{}",
        uuid::Uuid::new_v4()
            .to_string()
            .chars()
            .take(5)
            .collect::<String>()
    );
    let pod_json = serde_json::json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": pod_name },
        "spec": {
            "containers": [
                {
                    "name": "env-test",
                    "image": "busybox",
                    "command": ["sh", "-c", "sleep 30"],
                    "env": [
                        { "name": "SECRET_VALUE", "value": "supersecret" },
                        { "name": "PLAIN_VALUE", "value": "hello" }
                    ]
                }
            ]
        }
    });
    let pod: Pod = serde_json::from_value(pod_json)?;
    pods.create(&Default::default(), &pod).await?;

    // Wait for the watcher to ingest the pod
    let entry = wait_for_store_get(
        store.clone(),
        "v1/Pod",
        "default",
        &pod_name,
        Duration::from_secs(30),
    )
    .await?;

    // Parse stored JSON and assert env value redacted
    let mut value: Value =
        serde_json::from_str(&entry.content).context("failed to parse stored pod JSON content")?;
    let containers = value
        .get_mut("spec")
        .and_then(|s| s.get_mut("containers"))
        .and_then(Value::as_array_mut)
        .context("spec.containers missing")?;
    let env = containers[0]
        .get("env")
        .and_then(Value::as_array)
        .context("env missing")?;

    let secret_env = env
        .iter()
        .find(|e| e.get("name").and_then(Value::as_str) == Some("SECRET_VALUE"))
        .context("SECRET_VALUE env not found")?;
    assert_eq!(
        secret_env.get("value").and_then(Value::as_str),
        Some("<redacted>")
    );

    let plain_env = env
        .iter()
        .find(|e| e.get("name").and_then(Value::as_str) == Some("PLAIN_VALUE"))
        .context("PLAIN_VALUE env not found")?;
    assert_eq!(
        plain_env.get("value").and_then(Value::as_str),
        Some("hello")
    );

    // Cleanup the pod
    let _ = pods.delete(&pod_name, &Default::default()).await;
    watch_handle.abort();
    Ok(())
}

#[tokio::test]
async fn watcher_handles_namespace_scoping() -> anyhow::Result<()> {
    if !should_run() {
        return Ok(());
    }
    common::init_tracing();
    ensure_fixtures_applied()?;

    let client: kube::Client = kube_client().await?;
    let store = Arc::new(Mutex::new(telescope_core::ResourceStore::open(":memory:")?));
    let watcher = telescope_engine::watcher::ResourceWatcher::new(client.clone(), store.clone());

    watcher.register_watches(2);
    let state_rx = watcher.state_receiver();

    // Watch default and telescope-test namespaces separately
    let default_watch = tokio::spawn({
        let watcher = watcher.clone();
        async move { watcher.watch_pods("default").await }
    });
    let testns_watch = tokio::spawn({
        let watcher = watcher.clone();
        async move { watcher.watch_pods("telescope-test").await }
    });

    wait_for_ready_state(state_rx, Duration::from_secs(60)).await?;

    // default namespace should already have nginx-test pods
    wait_for_store_count(
        store.clone(),
        "v1/Pod",
        Some("default"),
        1,
        Duration::from_secs(30),
    )
    .await?;

    // telescope-test namespace should contain echo-server deployment from fixtures
    wait_for_store_count(
        store.clone(),
        "v1/Pod",
        Some("telescope-test"),
        1,
        Duration::from_secs(60),
    )
    .await?;

    default_watch.abort();
    testns_watch.abort();
    Ok(())
}
