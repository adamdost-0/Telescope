//! Telescope Kubernetes engine.

pub mod actions;
pub mod audit;
pub mod client;
pub mod crd;
pub mod error;
pub mod exec;
pub mod helm;
pub mod kubeconfig;
pub mod logs;
pub mod metrics;
pub mod namespace;
pub mod portforward;
pub mod watcher;

pub use client::ClusterInfo;
pub use error::{EngineError, Result};
pub use kubeconfig::ClusterContext;
pub use watcher::ResourceWatcher;

use telescope_core::VersionInfo;

pub fn version() -> VersionInfo {
    VersionInfo {
        name: "telescope-engine".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_version_uses_pkg_version() {
        let v = version();
        assert_eq!(v.name, "telescope-engine");
        assert_eq!(v.version, env!("CARGO_PKG_VERSION"));
    }
}
