use std::fs::Metadata;
use std::path::{Path, PathBuf};

/// Resolve a helper command to a canonical path from a fixed set of trusted binaries.
///
/// This blocks relative paths and any binary that is not installed in one of the
/// provided trusted locations.
pub fn resolve_trusted_binary<I>(command: &str, trusted_paths: I) -> Result<PathBuf, String>
where
    I: IntoIterator<Item = PathBuf>,
{
    let command = command.trim();
    if command.is_empty() {
        return Err("helper command must not be empty".to_string());
    }

    let trusted = trusted_paths
        .into_iter()
        .filter_map(|candidate| validate_binary_file(&candidate).ok())
        .collect::<Vec<_>>();

    let requested = Path::new(command);
    if requested.is_absolute() {
        let resolved = validate_binary_file(requested)?;
        if trusted.iter().any(|candidate| candidate == &resolved) {
            return Ok(resolved);
        }

        return Err(format!(
            "helper path {} is not in Telescope's trusted installation list",
            resolved.display()
        ));
    }

    if is_relative_or_qualified_path(requested, command) {
        return Err(format!(
            "helper command `{command}` must be a trusted binary name, not a relative or qualified path"
        ));
    }

    trusted
        .into_iter()
        .find(|candidate| {
            candidate
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case(command))
        })
        .ok_or_else(|| {
            format!(
                "helper command `{command}` was not found in Telescope's trusted installation list"
            )
        })
}

fn is_relative_or_qualified_path(requested: &Path, command: &str) -> bool {
    requested.components().count() > 1 || command.contains('/') || command.contains('\\')
}

fn validate_binary_file(path: &Path) -> Result<PathBuf, String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("failed to resolve helper path {}: {error}", path.display()))?;
    let metadata = std::fs::metadata(&canonical).map_err(|error| {
        format!(
            "failed to inspect helper path {}: {error}",
            canonical.display()
        )
    })?;

    if !metadata.is_file() {
        return Err(format!("helper path {} is not a file", canonical.display()));
    }

    validate_binary_permissions(&canonical, &metadata)?;
    Ok(canonical)
}

#[cfg(unix)]
fn validate_binary_permissions(path: &Path, metadata: &Metadata) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mode = metadata.permissions().mode();
    if mode & 0o111 == 0 {
        return Err(format!("helper path {} is not executable", path.display()));
    }
    if mode & 0o022 != 0 {
        return Err(format!(
            "helper path {} must not be group or world writable",
            path.display()
        ));
    }

    Ok(())
}

#[cfg(not(unix))]
fn validate_binary_permissions(_path: &Path, _metadata: &Metadata) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::resolve_trusted_binary;

    #[test]
    fn resolve_trusted_binary_accepts_trusted_absolute_path() {
        let executable = std::env::current_exe().expect("current test executable");
        let resolved = resolve_trusted_binary(
            executable.to_str().expect("utf-8 path"),
            vec![executable.clone()],
        )
        .expect("trusted absolute path should resolve");

        assert_eq!(resolved, executable.canonicalize().expect("canonical path"));
    }

    #[test]
    fn resolve_trusted_binary_accepts_trusted_binary_name() {
        let executable = std::env::current_exe().expect("current test executable");
        let name = executable
            .file_name()
            .and_then(|name| name.to_str())
            .expect("binary name");

        let resolved = resolve_trusted_binary(name, vec![executable.clone()])
            .expect("trusted binary name should resolve");

        assert_eq!(resolved, executable.canonicalize().expect("canonical path"));
    }

    #[test]
    fn resolve_trusted_binary_rejects_relative_paths() {
        let err = resolve_trusted_binary("./malicious-helper", Vec::<PathBuf>::new())
            .expect_err("relative path should be blocked");

        assert!(err.contains("relative or qualified path"));
    }

    #[test]
    fn resolve_trusted_binary_rejects_untrusted_absolute_paths() {
        let executable = std::env::current_exe().expect("current test executable");
        let err = resolve_trusted_binary(
            executable.to_str().expect("utf-8 path"),
            Vec::<PathBuf>::new(),
        )
        .expect_err("untrusted absolute path should be blocked");

        assert!(err.contains("trusted installation list"));
    }
}
