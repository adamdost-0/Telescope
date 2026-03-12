//! CRD discovery — list installed Custom Resource Definitions.

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{Api, Client};
use serde::{Deserialize, Serialize};

/// Summary of a single installed CRD.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdInfo {
    /// Full CRD name, e.g. "certificates.cert-manager.io"
    pub name: String,
    /// API group, e.g. "cert-manager.io"
    pub group: String,
    /// Kind, e.g. "Certificate"
    pub kind: String,
    /// Highest-priority served version, e.g. "v1"
    pub version: String,
    /// "Namespaced" or "Cluster"
    pub scope: String,
    /// Plural resource name used in API paths, e.g. "certificates"
    pub plural: String,
    /// Short names for kubectl, e.g. ["cert", "certs"]
    pub short_names: Vec<String>,
}

/// List all CRDs installed on the cluster.
pub async fn list_crds(client: &Client) -> crate::Result<Vec<CrdInfo>> {
    let api: Api<CustomResourceDefinition> = Api::all(client.clone());
    let crds = api.list(&Default::default()).await?;
    Ok(crds
        .items
        .iter()
        .filter_map(|crd| {
            let spec = &crd.spec;
            let served = spec.versions.iter().find(|v| v.served)?;
            Some(CrdInfo {
                name: crd.metadata.name.clone()?,
                group: spec.group.clone(),
                kind: spec.names.kind.clone(),
                version: served.name.clone(),
                scope: format!("{:?}", spec.scope),
                plural: spec.names.plural.clone(),
                short_names: spec.names.short_names.clone().unwrap_or_default(),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crd_info_serializes() {
        let info = CrdInfo {
            name: "certificates.cert-manager.io".into(),
            group: "cert-manager.io".into(),
            kind: "Certificate".into(),
            version: "v1".into(),
            scope: "Namespaced".into(),
            plural: "certificates".into(),
            short_names: vec!["cert".into(), "certs".into()],
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("certificates.cert-manager.io"));
        assert!(json.contains("\"plural\":\"certificates\""));

        let de: CrdInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(de.name, "certificates.cert-manager.io");
        assert_eq!(de.short_names, vec!["cert", "certs"]);
    }
}
