//! Structured audit logging for destructive operations.

use serde::Serialize;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;

#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub actor: String,
    pub context: String,
    pub namespace: String,
    pub action: String,
    pub resource_type: String,
    pub resource_name: String,
    pub result: String,
    pub detail: Option<String>,
}

const REDACTED_COMMAND: &str = "[REDACTED]";

fn sanitize_field(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect()
}

fn redact_exec_command_detail() -> String {
    REDACTED_COMMAND.to_string()
}

fn sanitize_entry(entry: &AuditEntry) -> AuditEntry {
    let detail = if entry.action.eq_ignore_ascii_case("exec") {
        entry.detail.as_ref().map(|_| redact_exec_command_detail())
    } else {
        entry.detail.as_deref().map(sanitize_field)
    };

    AuditEntry {
        timestamp: sanitize_field(&entry.timestamp),
        actor: sanitize_field(&entry.actor),
        context: sanitize_field(&entry.context),
        namespace: sanitize_field(&entry.namespace),
        action: sanitize_field(&entry.action),
        resource_type: sanitize_field(&entry.resource_type),
        resource_name: sanitize_field(&entry.resource_name),
        result: sanitize_field(&entry.result),
        detail,
    }
}

fn normalize_actor_component(value: Option<&str>, fallback: &str) -> String {
    let normalized: String = value
        .unwrap_or_default()
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_control() || ch.is_whitespace() || ch == '@' {
                '_'
            } else {
                ch
            }
        })
        .collect();

    if normalized.trim_matches('_').is_empty() {
        fallback.to_string()
    } else {
        normalized
    }
}

fn first_non_empty_env(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        env::var(key)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn resolve_actor_identity_from_sources(user: Option<&str>, host: Option<&str>) -> String {
    let user = normalize_actor_component(user, "unknown");
    let host = normalize_actor_component(host, "local");
    format!("{user}@{host}")
}

/// Resolve an audit actor string in `user@host` form for desktop audit logs.
pub fn resolve_actor_identity() -> String {
    let user = first_non_empty_env(&["USER", "USERNAME", "LOGNAME"]);
    let host = hostname::get()
        .ok()
        .and_then(|value| value.into_string().ok())
        .filter(|value| !value.trim().is_empty())
        .or_else(|| first_non_empty_env(&["HOSTNAME", "COMPUTERNAME", "HOST"]));

    resolve_actor_identity_from_sources(user.as_deref(), host.as_deref())
}

/// Append an audit entry to the audit log file as a JSON line.
pub fn log_audit(log_path: &str, entry: &AuditEntry) -> std::io::Result<()> {
    let json = serde_json::to_string(&sanitize_entry(entry)).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("failed to serialize audit entry: {error}"),
        )
    })?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    writeln!(file, "{json}")
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
            actor: "test-user@example.com".to_string(),
            context: "test-ctx".to_string(),
            namespace: "default".to_string(),
            action: "delete".to_string(),
            resource_type: "v1/Pod".to_string(),
            resource_name: "my-pod".to_string(),
            result: "success".to_string(),
            detail: None,
        };

        log_audit(&path_str, &entry).expect("should write first audit entry");

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
        log_audit(&path_str, &entry2).expect("should append second audit entry");

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

    #[test]
    fn log_audit_returns_write_errors() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir()
            .join(format!("telescope-audit-missing-{unique}"))
            .join("nested")
            .join("audit.log");
        let path_str = path.to_string_lossy().to_string();

        let entry = AuditEntry {
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            actor: "test-user@example.com".to_string(),
            context: "test-ctx".to_string(),
            namespace: "default".to_string(),
            action: "delete".to_string(),
            resource_type: "v1/Pod".to_string(),
            resource_name: "my-pod".to_string(),
            result: "success".to_string(),
            detail: None,
        };

        let err = log_audit(&path_str, &entry).expect_err("missing parent directory should fail");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn log_audit_sanitizes_control_characters() {
        let dir = std::env::temp_dir().join("telescope-audit-sanitize-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test-audit.log");
        let path_str = path.to_string_lossy().to_string();

        let _ = std::fs::remove_file(&path);

        let entry = AuditEntry {
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            actor: "test-user\n@example.com".to_string(),
            context: "test\rctx".to_string(),
            namespace: "default".to_string(),
            action: "delete".to_string(),
            resource_type: "v1/Pod".to_string(),
            resource_name: "my-pod".to_string(),
            result: "success".to_string(),
            detail: Some("line1\nline2\t".to_string()),
        };

        log_audit(&path_str, &entry).expect("should write sanitized entry");

        let mut contents = String::new();
        std::fs::File::open(&path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();

        let lines: Vec<&str> = contents.trim().lines().collect();
        assert_eq!(lines.len(), 1);

        let parsed: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(parsed["actor"], "test-user @example.com");
        assert_eq!(parsed["context"], "test ctx");
        assert_eq!(parsed["detail"], "line1 line2 ");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn resolve_actor_identity_uses_user_and_host_sources() {
        let actor = resolve_actor_identity_from_sources(Some("alice"), Some("workstation"));
        assert_eq!(actor, "alice@workstation");
    }

    #[test]
    fn resolve_actor_identity_sanitizes_components_and_falls_back() {
        let actor = resolve_actor_identity_from_sources(Some("  jane doe  "), Some(" \n "));
        assert_eq!(actor, "jane_doe@local");
    }

    #[test]
    fn log_audit_redacts_exec_commands() {
        let dir = std::env::temp_dir().join("telescope-audit-exec-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test-audit.log");
        let path_str = path.to_string_lossy().to_string();

        let _ = std::fs::remove_file(&path);

        let entry = AuditEntry {
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            actor: "test-user@example.com".to_string(),
            context: "test-ctx".to_string(),
            namespace: "default".to_string(),
            action: "exec".to_string(),
            resource_type: "Pod".to_string(),
            resource_name: "my-pod".to_string(),
            result: "success".to_string(),
            detail: Some("cat /run/secrets/token".to_string()),
        };

        log_audit(&path_str, &entry).expect("should write redacted exec entry");

        let mut contents = String::new();
        std::fs::File::open(&path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(contents.trim()).unwrap();
        assert_eq!(parsed["action"], "exec");
        assert_eq!(parsed["detail"], REDACTED_COMMAND);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn log_audit_does_not_redact_non_exec_actions() {
        let dir = std::env::temp_dir().join("telescope-audit-non-exec-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test-audit.log");
        let path_str = path.to_string_lossy().to_string();

        let _ = std::fs::remove_file(&path);

        let entry = AuditEntry {
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            actor: "test-user@example.com".to_string(),
            context: "test-ctx".to_string(),
            namespace: "default".to_string(),
            action: "delete".to_string(),
            resource_type: "v1/Pod".to_string(),
            resource_name: "my-pod".to_string(),
            result: "success".to_string(),
            detail: Some("replicas=3".to_string()),
        };

        log_audit(&path_str, &entry).expect("should write non-redacted delete entry");

        let mut contents = String::new();
        std::fs::File::open(&path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(contents.trim()).unwrap();
        assert_eq!(parsed["action"], "delete");
        assert_eq!(parsed["detail"], "replicas=3");

        let _ = std::fs::remove_file(&path);
    }
}
