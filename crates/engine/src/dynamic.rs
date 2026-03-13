//! Dynamic Custom Resource instance operations via kube-rs `DynamicObject`.

use kube::{
    api::{
        ApiResource, DeleteParams, DynamicObject, GroupVersionKind, ListParams, Patch, PatchParams,
    },
    Api, Client,
};
use telescope_core::store::ResourceEntry;

fn gvk_string(group: &str, version: &str, kind: &str) -> String {
    if group.is_empty() {
        format!("{version}/{kind}")
    } else {
        format!("{group}/{version}/{kind}")
    }
}

fn api_resource(group: &str, version: &str, kind: &str, plural: &str) -> ApiResource {
    ApiResource::from_gvk_with_plural(&GroupVersionKind::gvk(group, version, kind), plural)
}

fn entry_from_dynamic(
    group: &str,
    version: &str,
    kind: &str,
    object: &DynamicObject,
) -> Option<ResourceEntry> {
    Some(ResourceEntry {
        gvk: gvk_string(group, version, kind),
        namespace: object.metadata.namespace.clone().unwrap_or_default(),
        name: object.metadata.name.clone()?,
        resource_version: object.metadata.resource_version.clone().unwrap_or_default(),
        content: serde_json::to_string(object).ok()?,
        updated_at: String::new(),
    })
}

pub async fn resolve_dynamic_kind(
    client: &Client,
    group: &str,
    version: &str,
    plural: &str,
) -> crate::Result<String> {
    crate::crd::list_crds(client)
        .await?
        .into_iter()
        .find(|crd| crd.group == group && crd.version == version && crd.plural == plural)
        .map(|crd| crd.kind)
        .ok_or_else(|| {
            crate::EngineError::Other(format!(
                "Unable to resolve kind for {group}/{version}/{plural}"
            ))
        })
}

pub async fn list_dynamic_resources(
    client: &Client,
    group: &str,
    version: &str,
    kind: &str,
    plural: &str,
    namespace: Option<&str>,
) -> crate::Result<Vec<ResourceEntry>> {
    let ar = api_resource(group, version, kind, plural);
    let api: Api<DynamicObject> = match namespace.filter(|ns| !ns.is_empty()) {
        Some(ns) => Api::namespaced_with(client.clone(), ns, &ar),
        None => Api::all_with(client.clone(), &ar),
    };
    let list = api.list(&ListParams::default()).await?;
    let mut items = list
        .items
        .iter()
        .filter_map(|item| entry_from_dynamic(group, version, kind, item))
        .collect::<Vec<_>>();
    items.sort_by(|a, b| a.namespace.cmp(&b.namespace).then(a.name.cmp(&b.name)));
    Ok(items)
}

pub async fn get_dynamic_resource(
    client: &Client,
    group: &str,
    version: &str,
    kind: &str,
    plural: &str,
    namespace: Option<&str>,
    name: &str,
) -> crate::Result<Option<ResourceEntry>> {
    let ar = api_resource(group, version, kind, plural);
    let api: Api<DynamicObject> = match namespace.filter(|ns| !ns.is_empty()) {
        Some(ns) => Api::namespaced_with(client.clone(), ns, &ar),
        None => Api::all_with(client.clone(), &ar),
    };
    let object = api.get_opt(name).await?;
    Ok(object
        .as_ref()
        .and_then(|item| entry_from_dynamic(group, version, kind, item)))
}

#[allow(clippy::too_many_arguments)]
pub async fn apply_dynamic_resource(
    client: &Client,
    group: &str,
    version: &str,
    kind: &str,
    plural: &str,
    namespace: Option<&str>,
    manifest: &str,
    dry_run: bool,
) -> crate::Result<crate::actions::ApplyResult> {
    let mut value: serde_json::Value = serde_json::from_str(manifest)
        .or_else(|_| serde_yaml::from_str(manifest))
        .map_err(|e| crate::EngineError::Other(format!("Invalid YAML/JSON: {e}")))?;

    let expected_api_version = if group.is_empty() {
        version.to_string()
    } else {
        format!("{group}/{version}")
    };

    let api_version = value["apiVersion"]
        .as_str()
        .ok_or_else(|| crate::EngineError::Other("Missing apiVersion".into()))?;
    if api_version != expected_api_version {
        return Err(crate::EngineError::Other(format!(
            "Manifest apiVersion {api_version} does not match expected {expected_api_version}"
        )));
    }

    let manifest_kind = value["kind"]
        .as_str()
        .ok_or_else(|| crate::EngineError::Other("Missing kind".into()))?;
    if manifest_kind != kind {
        return Err(crate::EngineError::Other(format!(
            "Manifest kind {manifest_kind} does not match expected {kind}"
        )));
    }

    if let Some(default_namespace) = namespace.filter(|ns| !ns.is_empty()) {
        let metadata = value
            .as_object_mut()
            .ok_or_else(|| crate::EngineError::Other("Manifest must be a JSON object".into()))?
            .entry("metadata")
            .or_insert_with(|| serde_json::json!({}));
        let metadata = metadata
            .as_object_mut()
            .ok_or_else(|| crate::EngineError::Other("metadata must be an object".into()))?;
        metadata
            .entry("namespace")
            .or_insert_with(|| serde_json::Value::String(default_namespace.to_string()));
    }

    let name = value["metadata"]["name"]
        .as_str()
        .ok_or_else(|| crate::EngineError::Other("Missing metadata.name".into()))?;
    let object_namespace = value["metadata"]["namespace"].as_str();

    let ar = api_resource(group, version, kind, plural);
    let api: Api<DynamicObject> = match object_namespace.filter(|ns| !ns.is_empty()) {
        Some(ns) => Api::namespaced_with(client.clone(), ns, &ar),
        None => Api::all_with(client.clone(), &ar),
    };

    let mut patch_params = PatchParams::apply("telescope");
    if dry_run {
        patch_params = patch_params.dry_run();
    }
    patch_params.force = true;

    let applied = api
        .patch(name, &patch_params, &Patch::Apply(&value))
        .await?;

    let scope_display = object_namespace.unwrap_or("cluster scope");
    Ok(crate::actions::ApplyResult {
        success: true,
        message: if dry_run {
            format!("Dry run succeeded for {kind}/{name} in {scope_display}")
        } else {
            format!("Applied {kind}/{name} in {scope_display}")
        },
        result_yaml: serde_json::to_string_pretty(&applied).ok(),
    })
}

pub async fn delete_dynamic_resource(
    client: &Client,
    group: &str,
    version: &str,
    kind: &str,
    plural: &str,
    namespace: &str,
    name: &str,
) -> crate::Result<crate::actions::DeleteResult> {
    let ar = api_resource(group, version, kind, plural);
    let api: Api<DynamicObject> = if namespace.is_empty() {
        Api::all_with(client.clone(), &ar)
    } else {
        Api::namespaced_with(client.clone(), namespace, &ar)
    };
    api.delete(name, &DeleteParams::default()).await?;

    Ok(crate::actions::DeleteResult {
        success: true,
        message: if namespace.is_empty() {
            format!("Deleted {kind}/{name} in cluster scope")
        } else {
            format!("Deleted {kind}/{name} in namespace {namespace}")
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gvk_string_formats_core_and_grouped_resources() {
        assert_eq!(gvk_string("", "v1", "ConfigMap"), "v1/ConfigMap");
        assert_eq!(
            gvk_string("example.com", "v1alpha1", "Widget"),
            "example.com/v1alpha1/Widget"
        );
    }
}
