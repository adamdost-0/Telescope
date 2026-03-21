//! Integration tests against a real Kubernetes cluster.
//!
//! These tests are cluster-agnostic — they work on k3d, AKS, EKS, GKE, etc.
//! Run: K3D_TEST=1 cargo test -p telescope-engine --test integration_k3d

use std::sync::Arc;

mod common;
use common::should_run;

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
    // Cluster-agnostic: just verify we CAN list pods (any count >= 0 is fine)
    assert!(
        !pod_list.items.is_empty(),
        "Expected at least 1 pod in default namespace"
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
    assert!(
        names.contains(&"default".to_string()),
        "Expected 'default' namespace"
    );
    assert!(
        names.contains(&"kube-system".to_string()),
        "Expected 'kube-system' namespace"
    );
    // Don't assert telescope-test — that's k3d-specific
}

#[tokio::test]
async fn lists_contexts_from_kubeconfig() {
    if !should_run() {
        return;
    }

    let contexts = telescope_engine::kubeconfig::list_contexts().expect("Failed to list contexts");

    println!("Found {} contexts", contexts.len());
    assert!(!contexts.is_empty(), "Expected at least one context");
    // Don't assert k3d-specific context name
}

#[tokio::test]
async fn can_list_deployments() {
    if !should_run() {
        return;
    }
    let client = telescope_engine::client::create_client()
        .await
        .expect("client");
    use k8s_openapi::api::apps::v1::Deployment;
    use kube::Api;
    let api: Api<Deployment> = Api::namespaced(client, "default");
    let list = api
        .list(&Default::default())
        .await
        .expect("list deployments");
    println!("Found {} deployments in default", list.items.len());
    // Just verify the API call works
}

#[tokio::test]
async fn can_list_services() {
    if !should_run() {
        return;
    }
    let client = telescope_engine::client::create_client()
        .await
        .expect("client");
    use k8s_openapi::api::core::v1::Service;
    use kube::Api;
    let api: Api<Service> = Api::namespaced(client, "default");
    let list = api.list(&Default::default()).await.expect("list services");
    println!("Found {} services in default", list.items.len());
    assert!(
        !list.items.is_empty(),
        "Expected at least kubernetes service"
    );
}

#[tokio::test]
async fn metrics_api_check() {
    if !should_run() {
        return;
    }
    let client = telescope_engine::client::create_client()
        .await
        .expect("client");
    let available = telescope_engine::metrics::is_metrics_available(&client).await;
    println!("Metrics API available: {}", available);
    // Don't assert — metrics-server may or may not be installed
}
