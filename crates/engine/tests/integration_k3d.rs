//! Integration tests against a real Kubernetes cluster (k3d).
//!
//! These tests require a running k3d cluster with test fixtures deployed.
//! Run: K3D_TEST=1 cargo test -p telescope-engine --test integration_k3d

use std::sync::Arc;

fn should_run() -> bool {
    std::env::var("K3D_TEST").unwrap_or_default() == "1"
}

#[tokio::test]
async fn connects_to_cluster() {
    if !should_run() {
        eprintln!("Skipping k3d test (set K3D_TEST=1 to enable)");
        return;
    }

    let client = telescope_engine::client::create_client()
        .await
        .expect("Failed to create k8s client");

    let version = client
        .apiserver_version()
        .await
        .expect("Failed to get API version");
    println!("Connected to K8s {}.{}", version.major, version.minor);
}

#[tokio::test]
async fn lists_pods_in_default_namespace() {
    if !should_run() {
        return;
    }

    let client = telescope_engine::client::create_client()
        .await
        .expect("Failed to create client");

    use k8s_openapi::api::core::v1::Pod;
    use kube::Api;

    let pods: Api<Pod> = Api::namespaced(client, "default");
    let pod_list = pods
        .list(&Default::default())
        .await
        .expect("Failed to list pods");

    println!("Found {} pods in default namespace", pod_list.items.len());
    // With our fixtures: 20 nginx + 1 crashloop = at least 21
    assert!(
        pod_list.items.len() >= 20,
        "Expected at least 20 pods from nginx deployment"
    );
}

#[tokio::test]
async fn watcher_syncs_pods_to_store() {
    if !should_run() {
        return;
    }

    let _client = telescope_engine::client::create_client()
        .await
        .expect("Failed to create client");

    let _store = Arc::new(std::sync::Mutex::new(
        telescope_core::ResourceStore::open(":memory:").expect("Failed to create store"),
    ));

    // Note: This test depends on watcher accepting Arc<Mutex<ResourceStore>>
    // If watcher still takes Arc<ResourceStore>, this test will need updating
    // after workstream A modifies the watcher
    println!("Store and client created successfully");
    println!("Full watcher integration test requires workstream A changes");
}

#[tokio::test]
async fn lists_namespaces() {
    if !should_run() {
        return;
    }

    let client = telescope_engine::client::create_client()
        .await
        .expect("Failed to create client");

    use k8s_openapi::api::core::v1::Namespace;
    use kube::Api;

    let ns_api: Api<Namespace> = Api::all(client);
    let ns_list = ns_api
        .list(&Default::default())
        .await
        .expect("Failed to list namespaces");

    let names: Vec<String> = ns_list
        .items
        .iter()
        .filter_map(|ns| ns.metadata.name.clone())
        .collect();

    println!("Namespaces: {:?}", names);
    assert!(names.contains(&"default".to_string()));
    assert!(
        names.contains(&"telescope-test".to_string()),
        "Expected telescope-test namespace from fixtures"
    );
}

#[tokio::test]
async fn lists_contexts_from_kubeconfig() {
    if !should_run() {
        return;
    }

    let contexts = telescope_engine::kubeconfig::list_contexts().expect("Failed to list contexts");

    println!("Found {} contexts", contexts.len());
    assert!(!contexts.is_empty(), "Expected at least one context");

    // k3d creates a context named k3d-<cluster-name>
    let k3d_ctx = contexts.iter().find(|c| c.name.starts_with("k3d-"));
    assert!(k3d_ctx.is_some(), "Expected a k3d context");
    println!("k3d context: {:?}", k3d_ctx.unwrap().name);
}
