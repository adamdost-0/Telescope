#![allow(clippy::unused_async)]
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use anyhow::Context;
use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client};

mod common;
use common::{ensure_fixtures_applied, kube_client, should_run};

async fn find_nginx_pod(client: Client) -> anyhow::Result<String> {
    let pods: Api<Pod> = Api::namespaced(client, "default");
    let lp = kube::api::ListParams::default().labels("app=nginx-test");
    let list = pods.list(&lp).await?;
    let name = list
        .items
        .first()
        .and_then(|p| p.metadata.name.clone())
        .context("no nginx-test pods found")?;
    Ok(name)
}

#[tokio::test]
async fn port_forward_allows_http_get() -> anyhow::Result<()> {
    if !should_run() {
        return Ok(());
    }
    common::init_tracing();
    ensure_fixtures_applied()?;

    let client: kube::Client = kube_client().await?;
    let pod = find_nginx_pod(client.clone()).await?;

    let req = telescope_engine::portforward::PortForwardRequest {
        namespace: "default".into(),
        pod: pod.clone(),
        local_port: 0, // auto-assign free port
        remote_port: 80,
    };

    let local_port = telescope_engine::portforward::start_port_forward(&client, &req).await?;
    assert!(local_port > 0);

    // Connect via plain TCP and issue a minimal HTTP request
    let addr = format!("127.0.0.1:{}", local_port);
    let mut stream = TcpStream::connect(&addr).context("failed to connect to forwarded port")?;
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .context("failed to write HTTP request")?;
    stream.flush().ok();

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).ok();
    let resp = String::from_utf8_lossy(&buf);
    assert!(resp.contains("HTTP/1.1 200"), "unexpected response: {resp}");

    // Give the port-forward task a moment to tear down, then assert active count drops
    tokio::time::sleep(Duration::from_secs(1)).await;
    let active = telescope_engine::portforward::active_forward_count();
    assert!(
        active <= 1,
        "active port forwards should be <=1, got {active}"
    );

    Ok(())
}
