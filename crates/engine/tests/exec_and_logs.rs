#![allow(clippy::unused_async)]
use std::time::Duration;

use anyhow::Context;
use futures::{io::AsyncBufReadExt, StreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client};

use tracing::info;

mod common;
use common::{ensure_fixtures_applied, kube_client, should_run};

async fn find_pod_with_label(
    client: Client,
    namespace: &str,
    label_selector: &str,
) -> anyhow::Result<String> {
    let pods: Api<Pod> = Api::namespaced(client, namespace);
    let lp = kube::api::ListParams::default().labels(label_selector);
    let list = pods.list(&lp).await?;
    let name = list
        .items
        .first()
        .and_then(|p| p.metadata.name.clone())
        .context("no pods found for selector")?;
    Ok(name)
}

#[tokio::test]
async fn exec_command_returns_output() -> anyhow::Result<()> {
    if !should_run() {
        return Ok(());
    }
    common::init_tracing();
    ensure_fixtures_applied()?;

    let client: kube::Client = kube_client().await?;
    let pod_name = find_pod_with_label(client.clone(), "default", "app=nginx-test").await?;

    let req = telescope_engine::exec::ExecRequest {
        namespace: "default".into(),
        pod: pod_name.clone(),
        container: None,
        command: vec!["/bin/sh".into(), "-c".into(), "hostname".into()],
    };

    let result = telescope_engine::exec::exec_command(&client, &req).await?;
    info!(stdout = %result.stdout, success = result.success, "exec result");
    assert!(result.success, "exec reported non-success status");
    assert!(
        !result.stdout.trim().is_empty(),
        "hostname output was empty"
    );
    Ok(())
}

#[tokio::test]
async fn stream_pod_logs_yields_lines() -> anyhow::Result<()> {
    if !should_run() {
        return Ok(());
    }
    common::init_tracing();
    ensure_fixtures_applied()?;

    let client: kube::Client = kube_client().await?;
    let pod_name = find_pod_with_label(client.clone(), "default", "app=nginx-test").await?;

    let req = telescope_engine::logs::LogRequest {
        namespace: "default".into(),
        pod: pod_name.clone(),
        container: None,
        previous: false,
        tail_lines: Some(10),
        follow: true,
    };

    let reader = telescope_engine::logs::stream_pod_logs(&client, &req).await?;

    let mut lines = Vec::new();
    let mut lines_stream = reader.lines();
    let timeout = tokio::time::sleep(Duration::from_secs(10));
    tokio::pin!(timeout);
    loop {
        tokio::select! {
            _ = &mut timeout => { break; }
            maybe_line = lines_stream.next() => {
                match maybe_line {
                    Some(Ok(line)) => {
                        lines.push(line);
                        if lines.len() >= 3 { break; }
                    }
                    Some(Err(e)) => return Err(e.into()),
                    None => break,
                }
            }
        }
    }

    assert!(
        !lines.is_empty(),
        "expected to receive at least one log line from pod {pod_name}"
    );
    Ok(())
}
