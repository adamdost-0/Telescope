//! Kubernetes engine (scaffold).

use telescope_core::VersionInfo;

pub fn version() -> VersionInfo {
    VersionInfo {
        name: "telescope-engine".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}
