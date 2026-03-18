use k8s_openapi::api::core::v1::Secret;
use kube::{Api, Client, ResourceExt};
use telescope_core::ResourceEntry;
use tracing::warn;

const GVK_SECRET: &str = "v1/Secret";
const REDACTED_VALUE: &str = "●●●●●●●●";
const LAST_APPLIED_CONFIG: &str = "kubectl.kubernetes.io/last-applied-configuration";

/// List secrets in a namespace without storing them in the shared cache.
pub async fn list_secrets(client: &Client, namespace: &str) -> crate::Result<Vec<ResourceEntry>> {
    let api: Api<Secret> = Api::namespaced(client.clone(), namespace);
    let secrets = api.list(&Default::default()).await?;

    let mut entries = secrets
        .items
        .iter()
        .filter_map(secret_to_entry)
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

/// Get one secret by namespace and name without storing it in the shared cache.
pub async fn get_secret(
    client: &Client,
    namespace: &str,
    name: &str,
) -> crate::Result<Option<ResourceEntry>> {
    let api: Api<Secret> = Api::namespaced(client.clone(), namespace);
    let secret = api.get_opt(name).await?;
    Ok(secret.as_ref().and_then(secret_to_entry))
}

fn secret_to_entry(secret: &Secret) -> Option<ResourceEntry> {
    let name = secret.metadata.name.as_deref()?.to_string();
    let namespace = secret.namespace().unwrap_or_default();
    let resource_version = secret.resource_version().unwrap_or_default();
    let mut content = serde_json::to_value(secret)
        .map_err(|e| {
            warn!(error = %e, %name, "failed to serialize secret to value");
            e
        })
        .ok()?;
    redact_secret_value(&mut content);

    let content_str = serde_json::to_string(&content)
        .map_err(|e| {
            warn!(error = %e, %name, "failed to serialize secret to string");
            e
        })
        .ok()?;

    Some(ResourceEntry {
        gvk: GVK_SECRET.to_string(),
        namespace,
        name,
        resource_version,
        content: content_str,
        updated_at: telescope_core::now_rfc3339(),
    })
}

fn redact_secret_value(value: &mut serde_json::Value) {
    if let Some(obj) = value.as_object_mut() {
        for field in ["data", "stringData", "binaryData"] {
            if let Some(secret_data) = obj
                .get_mut(field)
                .and_then(serde_json::Value::as_object_mut)
            {
                for entry in secret_data.values_mut() {
                    *entry = serde_json::Value::String(REDACTED_VALUE.to_string());
                }
            }
        }

        if let Some(annotations) = obj
            .get_mut("metadata")
            .and_then(serde_json::Value::as_object_mut)
            .and_then(|metadata| metadata.get_mut("annotations"))
            .and_then(serde_json::Value::as_object_mut)
        {
            if let Some(last_applied) = annotations.get_mut(LAST_APPLIED_CONFIG) {
                *last_applied = serde_json::Value::String(REDACTED_VALUE.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use k8s_openapi::{api::core::v1::Secret, ByteString};
    use kube::api::ObjectMeta;

    use super::{secret_to_entry, REDACTED_VALUE};

    #[test]
    fn secret_to_entry_redacts_secret_data() {
        let mut data = BTreeMap::new();
        data.insert("username".to_string(), ByteString(b"admin".to_vec()));
        data.insert("password".to_string(), ByteString(b"super-secret".to_vec()));

        let secret = Secret {
            metadata: ObjectMeta {
                name: Some("db-creds".to_string()),
                namespace: Some("default".to_string()),
                resource_version: Some("7".to_string()),
                ..Default::default()
            },
            data: Some(data),
            string_data: Some(
                [("token".to_string(), "plain-text".to_string())]
                    .into_iter()
                    .collect(),
            ),
            type_: Some("Opaque".to_string()),
            ..Default::default()
        };

        let entry = secret_to_entry(&secret).expect("entry");
        let parsed: serde_json::Value = serde_json::from_str(&entry.content).expect("json");

        assert_eq!(entry.gvk, "v1/Secret");
        assert_eq!(entry.namespace, "default");
        assert_eq!(entry.name, "db-creds");
        assert_eq!(entry.resource_version, "7");
        assert_eq!(parsed["data"]["username"], REDACTED_VALUE);
        assert_eq!(parsed["data"]["password"], REDACTED_VALUE);
        assert_eq!(parsed["stringData"]["token"], REDACTED_VALUE);
        assert_eq!(parsed["type"], "Opaque");
    }

    #[test]
    fn secret_to_entry_redacts_last_applied_annotation() {
        let secret = Secret {
            metadata: ObjectMeta {
                name: Some("tls-cert".to_string()),
                namespace: Some("default".to_string()),
                annotations: Some(
                    [(
                        "kubectl.kubernetes.io/last-applied-configuration".to_string(),
                        "{\"data\":{\"tls.crt\":\"raw\"}}".to_string(),
                    )]
                    .into_iter()
                    .collect(),
                ),
                ..Default::default()
            },
            ..Default::default()
        };

        let entry = secret_to_entry(&secret).expect("entry");
        let parsed: serde_json::Value = serde_json::from_str(&entry.content).expect("json");

        assert_eq!(
            parsed["metadata"]["annotations"]["kubectl.kubernetes.io/last-applied-configuration"],
            REDACTED_VALUE
        );
    }
}
