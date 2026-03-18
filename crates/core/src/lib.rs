//! Shared domain types.

pub mod connection;
pub mod store;

pub use connection::{ConnectionEvent, ConnectionState};
pub use store::{now_rfc3339, ResourceEntry, ResourceStore};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionInfo {
    pub name: String,
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_info_roundtrip_json() {
        let v = VersionInfo {
            name: "telescope".to_string(),
            version: "0.0.1".to_string(),
        };
        let s = serde_json::to_string(&v).expect("serialize");
        let back: VersionInfo = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(v, back);
    }
}
