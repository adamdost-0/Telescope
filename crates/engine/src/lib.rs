//! Kubernetes engine (scaffold).

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
