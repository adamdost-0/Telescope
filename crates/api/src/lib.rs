//! API surface (scaffold). gRPC/streaming will live here.

use telescope_core::VersionInfo;

pub fn engine_version() -> VersionInfo {
    telescope_engine::version()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_returns_engine_version() {
        let v = engine_version();
        assert_eq!(v.name, "telescope-engine");
        assert_eq!(v.version, env!("CARGO_PKG_VERSION"));
    }
}
