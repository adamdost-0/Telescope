//! API surface (scaffold). gRPC/streaming will live here.

use telescope_core::VersionInfo;

pub fn engine_version() -> VersionInfo {
  telescope_engine::version()
}
