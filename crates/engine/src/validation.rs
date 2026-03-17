//! Shared validation helpers for desktop IPC and engine entry points.

use std::collections::HashSet;
use std::sync::OnceLock;

use regex::Regex;
use telescope_core::ResourceEntry;

const MAX_IDENTIFIER_CHARS: usize = 253;
const MAX_EXEC_ARG_CHARS: usize = 4096;
const MAX_EXEC_ARGS: usize = 64;
const MAX_EXEC_COMMAND_CHARS: usize = 16 * 1024;

pub const MAX_REPLICAS: i32 = 10_000;
pub const MAX_NODE_POOL_COUNT: i32 = 1_000;
pub const MAX_DRAIN_GRACE_PERIOD_SECONDS: i64 = 86_400;

fn validate_text(value: &str, field: &str, max_chars: usize) -> crate::Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(crate::EngineError::Other(format!(
            "{field} must not be empty"
        )));
    }

    if trimmed.chars().count() > max_chars {
        return Err(crate::EngineError::Other(format!(
            "{field} must be at most {max_chars} characters"
        )));
    }

    if trimmed.chars().any(char::is_control) {
        return Err(crate::EngineError::Other(format!(
            "{field} must not contain control characters"
        )));
    }

    Ok(trimmed.to_string())
}

fn k8s_name_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^[a-z0-9]([a-z0-9\-\.]*[a-z0-9])?$").expect("valid regex"))
}

fn aks_node_pool_name_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| Regex::new(r"^[a-z][a-z0-9]{0,11}$").expect("valid regex"))
}

fn azure_vm_size_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX
        .get_or_init(|| Regex::new(r"^[A-Za-z]+_[A-Za-z0-9][A-Za-z0-9_.-]*$").expect("valid regex"))
}

fn kubernetes_version_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"^[0-9]+\.[0-9]+(?:\.[0-9]+)?(?:[-+][0-9A-Za-z.]+)?$").expect("valid regex")
    })
}

pub fn validate_identifier(value: &str, field: &str) -> crate::Result<String> {
    let normalized = validate_text(value, field, MAX_IDENTIFIER_CHARS)?;
    if normalized.chars().any(char::is_whitespace) {
        return Err(crate::EngineError::Other(format!(
            "{field} must not contain whitespace"
        )));
    }

    Ok(normalized)
}

pub fn validate_allowed_value(value: &str, field: &str, allowed: &[&str]) -> crate::Result<String> {
    let normalized = validate_identifier(value, field)?;
    if allowed.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(crate::EngineError::Other(format!(
            "{field} must be one of {}",
            allowed.join(", ")
        )))
    }
}

pub fn validate_k8s_name(name: &str) -> crate::Result<()> {
    validate_k8s_name_field(name, "name").map(|_| ())
}

pub fn validate_k8s_name_field(value: &str, field: &str) -> crate::Result<String> {
    let normalized = validate_identifier(value, field)?;
    if !k8s_name_regex().is_match(&normalized) {
        return Err(crate::EngineError::Other(format!(
            "{field} must be a valid Kubernetes name (lowercase alphanumeric, '-', '.', max 253 characters)"
        )));
    }

    Ok(normalized)
}

pub fn validate_namespace(value: &str) -> crate::Result<String> {
    validate_k8s_name_field(value, "namespace")
}

pub fn validate_aks_node_pool_name(value: &str) -> crate::Result<String> {
    let normalized = validate_identifier(value, "pool name")?;
    if !aks_node_pool_name_regex().is_match(&normalized) {
        return Err(crate::EngineError::Other(
            "pool name must be lowercase alphanumeric, start with a letter, and be 1-12 characters"
                .to_string(),
        ));
    }

    Ok(normalized)
}

pub fn validate_aks_vm_size(value: &str) -> crate::Result<String> {
    let normalized = validate_identifier(value, "vmSize")?;
    if !azure_vm_size_regex().is_match(&normalized) {
        return Err(crate::EngineError::Other(
            "vmSize must look like an Azure VM size such as Standard_DS2_v2".to_string(),
        ));
    }

    Ok(normalized)
}

pub fn validate_kubernetes_version(value: &str, field: &str) -> crate::Result<String> {
    let normalized = validate_identifier(value, field)?;
    if !kubernetes_version_regex().is_match(&normalized) {
        return Err(crate::EngineError::Other(format!(
            "{field} must be a Kubernetes version like 1.29.2"
        )));
    }

    Ok(normalized)
}

pub fn validate_optional_kubernetes_version(
    value: Option<&str>,
    field: &str,
) -> crate::Result<Option<String>> {
    value
        .map(|value| validate_kubernetes_version(value, field))
        .transpose()
}

pub fn validate_i32_range(value: i32, field: &str, min: i32, max: i32) -> crate::Result<i32> {
    if !(min..=max).contains(&value) {
        return Err(crate::EngineError::Other(format!(
            "{field} must be between {min} and {max}"
        )));
    }

    Ok(value)
}

pub fn validate_i64_range(value: i64, field: &str, min: i64, max: i64) -> crate::Result<i64> {
    if !(min..=max).contains(&value) {
        return Err(crate::EngineError::Other(format!(
            "{field} must be between {min} and {max}"
        )));
    }

    Ok(value)
}

pub fn validate_autoscaler_bounds(
    enabled: bool,
    min: Option<i32>,
    max: Option<i32>,
) -> crate::Result<(Option<i32>, Option<i32>)> {
    if !enabled {
        if min.is_some() || max.is_some() {
            return Err(crate::EngineError::Other(
                "minCount and maxCount must be omitted when autoscaling is disabled".to_string(),
            ));
        }

        return Ok((None, None));
    }

    let min = min.ok_or_else(|| {
        crate::EngineError::Other("minCount is required when autoscaling is enabled".to_string())
    })?;
    let max = max.ok_or_else(|| {
        crate::EngineError::Other("maxCount is required when autoscaling is enabled".to_string())
    })?;
    let min = validate_i32_range(min, "minCount", 1, MAX_NODE_POOL_COUNT)?;
    let max = validate_i32_range(max, "maxCount", 1, MAX_NODE_POOL_COUNT)?;

    if min > max {
        return Err(crate::EngineError::Other(
            "minCount must be less than or equal to maxCount".to_string(),
        ));
    }

    Ok((Some(min), Some(max)))
}

pub fn validate_aks_availability_zones(zones: &[String]) -> crate::Result<Vec<String>> {
    let mut seen = HashSet::new();
    let mut validated = Vec::with_capacity(zones.len());

    for zone in zones {
        let zone = validate_allowed_value(zone, "availability zone", &["1", "2", "3"])?;
        if !seen.insert(zone.clone()) {
            return Err(crate::EngineError::Other(format!(
                "availability zone {zone} must not be duplicated"
            )));
        }

        validated.push(zone);
    }

    Ok(validated)
}

pub fn validate_taint_effect(effect: &str) -> crate::Result<String> {
    validate_allowed_value(
        effect,
        "effect",
        &["NoSchedule", "PreferNoSchedule", "NoExecute"],
    )
}

pub fn validate_exec_command(command: &[String]) -> crate::Result<Vec<String>> {
    if command.is_empty() {
        return Err(crate::EngineError::Other(
            "command must include at least one program".to_string(),
        ));
    }

    if command.len() > MAX_EXEC_ARGS {
        return Err(crate::EngineError::Other(format!(
            "command must have at most {MAX_EXEC_ARGS} arguments"
        )));
    }

    let mut total_chars = 0usize;
    let mut validated = Vec::with_capacity(command.len());

    for (index, value) in command.iter().enumerate() {
        if index == 0 {
            let program = validate_text(value, "command[0]", MAX_EXEC_ARG_CHARS)?;
            total_chars += program.chars().count();
            validated.push(program);
            continue;
        }

        if value.chars().count() > MAX_EXEC_ARG_CHARS {
            return Err(crate::EngineError::Other(format!(
                "command[{index}] must be at most {MAX_EXEC_ARG_CHARS} characters"
            )));
        }

        if value.chars().any(char::is_control) {
            return Err(crate::EngineError::Other(format!(
                "command[{index}] must not contain control characters"
            )));
        }

        total_chars += value.chars().count();
        validated.push(value.clone());
    }

    if total_chars > MAX_EXEC_COMMAND_CHARS {
        return Err(crate::EngineError::Other(format!(
            "command must be at most {MAX_EXEC_COMMAND_CHARS} characters in total"
        )));
    }

    Ok(validated)
}

pub fn event_matches_involved_object_name(
    entry: &ResourceEntry,
    involved_object_name: &str,
) -> bool {
    serde_json::from_str::<serde_json::Value>(&entry.content)
        .ok()
        .and_then(|value| {
            value
                .get("involvedObject")
                .and_then(|object| object.get("name"))
                .and_then(|name| name.as_str())
                .map(|name| name == involved_object_name)
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event_entry(content: &str) -> ResourceEntry {
        ResourceEntry {
            gvk: "v1/Event".to_string(),
            namespace: "default".to_string(),
            name: "example.123".to_string(),
            resource_version: "1".to_string(),
            content: content.to_string(),
            updated_at: "2026-03-17T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn validate_k8s_name_accepts_dns_subdomain() {
        let name = validate_k8s_name_field("nginx-1.default", "name").expect("valid name");
        assert_eq!(name, "nginx-1.default");
    }

    #[test]
    fn validate_k8s_name_rejects_uppercase() {
        let err = validate_k8s_name_field("InvalidName", "name").expect_err("should reject");
        assert!(err.to_string().contains("must be a valid Kubernetes name"));
    }

    #[test]
    fn validate_pool_name_rejects_hyphens() {
        let err = validate_aks_node_pool_name("system-pool").expect_err("should reject");
        assert!(err.to_string().contains("pool name"));
    }

    #[test]
    fn validate_vm_size_requires_azure_style_shape() {
        let err = validate_aks_vm_size("StandardDS2v2").expect_err("should reject");
        assert!(err.to_string().contains("vmSize"));
    }

    #[test]
    fn validate_kubernetes_version_rejects_non_semver_strings() {
        let err = validate_kubernetes_version("latest", "version").expect_err("should reject");
        assert!(err.to_string().contains("Kubernetes version"));
    }

    #[test]
    fn validate_autoscaler_requires_min_and_max() {
        let err = validate_autoscaler_bounds(true, Some(1), None).expect_err("should require max");
        assert!(err.to_string().contains("maxCount"));
    }

    #[test]
    fn validate_availability_zones_rejects_duplicates() {
        let err = validate_aks_availability_zones(&["1".to_string(), "1".to_string()])
            .expect_err("should reject duplicate zone");
        assert!(err.to_string().contains("must not be duplicated"));
    }

    #[test]
    fn validate_exec_command_rejects_empty_command() {
        let err = validate_exec_command(&[]).expect_err("should reject empty command");
        assert!(err.to_string().contains("at least one program"));
    }

    #[test]
    fn validate_exec_command_allows_shell_script_argument_with_spaces() {
        let validated = validate_exec_command(&[
            "sh".to_string(),
            "-c".to_string(),
            "echo hello world".to_string(),
        ])
        .expect("valid command");

        assert_eq!(validated[0], "sh");
        assert_eq!(validated[2], "echo hello world");
    }

    #[test]
    fn validate_taint_effect_rejects_unknown_effect() {
        let err = validate_taint_effect("ScheduleMe").expect_err("should reject");
        assert!(err.to_string().contains("effect must be one of"));
    }

    #[test]
    fn event_matcher_reads_involved_object_name_exactly() {
        let entry = sample_event_entry(
            r#"{
                "involvedObject": { "name": "pod-a" },
                "metadata": { "annotations": { "name": "pod-b" } }
            }"#,
        );

        assert!(event_matches_involved_object_name(&entry, "pod-a"));
        assert!(!event_matches_involved_object_name(&entry, "pod-b"));
    }

    #[test]
    fn event_matcher_ignores_json_injection_style_needles() {
        let entry = sample_event_entry(
            r#"{
                "involvedObject": { "name": "safe-pod" },
                "metadata": {
                    "annotations": {
                        "name": "x",
                        "secret": "leaked"
                    }
                }
            }"#,
        );

        assert!(!event_matches_involved_object_name(
            &entry,
            r#"x","secret":"leaked"#,
        ));
    }
}
