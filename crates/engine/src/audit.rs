//! Structured audit logging for destructive operations.

use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;

#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub context: String,
    pub namespace: String,
    pub action: String,
    pub resource_type: String,
    pub resource_name: String,
    pub result: String,
    pub detail: Option<String>,
}

/// Append an audit entry to the audit log file as a JSON line.
pub fn log_audit(log_path: &str, entry: &AuditEntry) {
    if let Ok(json) = serde_json::to_string(entry) {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = writeln!(file, "{}", json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn log_audit_writes_json_line() {
        let dir = std::env::temp_dir().join("telescope-audit-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test-audit.log");
        let path_str = path.to_string_lossy().to_string();

        // Clean up any previous test run.
        let _ = std::fs::remove_file(&path);

        let entry = AuditEntry {
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            context: "test-ctx".to_string(),
            namespace: "default".to_string(),
            action: "delete".to_string(),
            resource_type: "v1/Pod".to_string(),
            resource_name: "my-pod".to_string(),
            result: "success".to_string(),
            detail: None,
        };

        log_audit(&path_str, &entry);

        let mut contents = String::new();
        std::fs::File::open(&path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(contents.trim()).unwrap();
        assert_eq!(parsed["action"], "delete");
        assert_eq!(parsed["resource_name"], "my-pod");
        assert_eq!(parsed["result"], "success");

        // Append a second entry and verify both lines exist.
        let entry2 = AuditEntry {
            detail: Some("replicas=3".to_string()),
            action: "scale".to_string(),
            ..entry
        };
        log_audit(&path_str, &entry2);

        let mut contents2 = String::new();
        std::fs::File::open(&path)
            .unwrap()
            .read_to_string(&mut contents2)
            .unwrap();
        let lines: Vec<&str> = contents2.trim().lines().collect();
        assert_eq!(lines.len(), 2);

        let parsed2: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(parsed2["detail"], "replicas=3");

        // Clean up.
        let _ = std::fs::remove_file(&path);
    }
}
