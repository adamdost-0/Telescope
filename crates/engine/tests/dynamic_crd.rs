#![allow(clippy::unused_async)]
use std::time::Duration;

use anyhow::Context;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::{
    CustomResourceDefinition, CustomResourceDefinitionCondition,
};
use kube::Api;

mod common;
use common::{kube_client, should_run, wait_for_condition};

const CRD_NAME: &str = "foos.example.com";
const CRD_GROUP: &str = "example.com";
const CRD_VERSION: &str = "v1";
const CRD_KIND: &str = "Foo";
const CRD_PLURAL: &str = "foos";

fn crd_manifest_json() -> serde_json::Value {
    serde_json::json!({
        "apiVersion": "apiextensions.k8s.io/v1",
        "kind": "CustomResourceDefinition",
        "metadata": { "name": CRD_NAME },
        "spec": {
            "group": CRD_GROUP,
            "versions": [
                {
                    "name": CRD_VERSION,
                    "served": true,
                    "storage": true,
                    "schema": {
                        "openAPIV3Schema": {
                            "type": "object",
                            "properties": {
                                "spec": {
                                    "type": "object",
                                    "properties": {
                                        "message": { "type": "string" }
                                    }
                                }
                            }
                        }
                    }
                }
            ],
            "scope": "Namespaced",
            "names": {
                "plural": CRD_PLURAL,
                "singular": "foo",
                "kind": CRD_KIND,
                "shortNames": ["fo"],
            }
        }
    })
}

fn cr_manifest_json(name: &str) -> serde_json::Value {
    serde_json::json!({
        "apiVersion": format!("{}/{}", CRD_GROUP, CRD_VERSION),
        "kind": CRD_KIND,
        "metadata": { "name": name, "namespace": "default" },
        "spec": { "message": "hello, dynamic" }
    })
}

#[tokio::test]
async fn dynamic_crd_apply_and_get() -> anyhow::Result<()> {
    if !should_run() {
        return Ok(());
    }
    common::init_tracing();

    let client: kube::Client = kube_client().await?;
    let crd_api: Api<CustomResourceDefinition> = Api::all(client.clone());
    let crd: CustomResourceDefinition = serde_json::from_value(crd_manifest_json())?;

    match crd_api.create(&Default::default(), &crd).await {
        Ok(_) => {}
        Err(kube::Error::Api(e)) if e.code == 409 => {
            // Already exists
        }
        Err(e) => return Err(e.into()),
    }

    // Wait for CRD to be Established
    wait_for_condition(Duration::from_secs(60), Duration::from_millis(500), || {
        let crd_api = crd_api.clone();
        async move {
            let fetched = crd_api.get(CRD_NAME).await?;
            let conditions: &[CustomResourceDefinitionCondition] = fetched
                .status
                .as_ref()
                .and_then(|s| s.conditions.as_deref())
                .unwrap_or(&[]);
            let established = conditions
                .iter()
                .any(|c| c.type_ == "Established" && c.status == "True");
            Ok(established)
        }
    })
    .await?;

    // Resolve dynamic kind from engine helper
    let resolved_kind = telescope_engine::dynamic::resolve_dynamic_kind(
        &client,
        CRD_GROUP,
        CRD_VERSION,
        CRD_PLURAL,
    )
    .await
    .context("resolve_dynamic_kind failed")?;
    assert_eq!(resolved_kind, CRD_KIND);

    // Apply a CR instance via dynamic helper
    let name = format!(
        "foo-{}",
        uuid::Uuid::new_v4()
            .to_string()
            .chars()
            .take(5)
            .collect::<String>()
    );
    let manifest = cr_manifest_json(&name);
    let manifest_str = serde_json::to_string(&manifest)?;
    telescope_engine::dynamic::apply_dynamic_resource(
        &client,
        CRD_GROUP,
        CRD_VERSION,
        CRD_KIND,
        CRD_PLURAL,
        Some("default"),
        &manifest_str,
        false,
    )
    .await
    .context("apply_dynamic_resource failed")?;

    // Fetch it back via dynamic get
    let fetched = telescope_engine::dynamic::get_dynamic_resource(
        &client,
        CRD_GROUP,
        CRD_VERSION,
        CRD_KIND,
        CRD_PLURAL,
        Some("default"),
        &name,
    )
    .await
    .context("get_dynamic_resource failed")?;
    let entry = fetched.context("dynamic resource not found after apply")?;
    let value: serde_json::Value = serde_json::from_str(&entry.content)?;
    let spec_msg = value
        .get("spec")
        .and_then(|s| s.get("message"))
        .and_then(|m| m.as_str())
        .unwrap_or("");
    assert_eq!(spec_msg, "hello, dynamic");

    // List should include our instance
    let listed = telescope_engine::dynamic::list_dynamic_resources(
        &client,
        CRD_GROUP,
        CRD_VERSION,
        CRD_KIND,
        CRD_PLURAL,
        Some("default"),
    )
    .await
    .context("list_dynamic_resources failed")?;
    let found = listed.iter().any(|obj| obj.name == name);
    assert!(found, "expected dynamic list to include {name}");

    // Cleanup the instance
    telescope_engine::dynamic::delete_dynamic_resource(
        &client,
        CRD_GROUP,
        CRD_VERSION,
        CRD_KIND,
        CRD_PLURAL,
        "default",
        &name,
    )
    .await
    .ok();

    Ok(())
}
