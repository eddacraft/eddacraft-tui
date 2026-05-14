//! Shared helpers for MCP tools.
//!
//! The pre-RMCPF-010 status tool, plus the new check/gate tools, all share
//! the same workspace-containment, redaction, and warning-shape concerns.
//! Keeping the helpers in one place stops the per-tool copies from drifting
//! apart and makes any future change (e.g. tighter symlink containment, new
//! redaction rule) land in a single location.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use anvil_checks::antipattern::{Warning, WarningCategory, WarningSeverity};

/// Maximum number of entries any MCP tool will accept in a file array.
/// Stops a hostile or buggy caller from forcing an arbitrarily large
/// `Vec::with_capacity` and per-entry path-join allocation before the
/// scanner runs.
pub const MAX_FILE_ENTRIES: usize = 10_000;

/// Validate that `workspace_root` resolves to a directory inside
/// `server_root`. Returns the canonicalised `(server_root, workspace_root)`
/// pair on success.
pub fn validate_workspace_root(
    workspace_root: &Path,
    server_root: &Path,
) -> Result<(PathBuf, PathBuf), String> {
    if !workspace_root.is_absolute() {
        return Err("workspaceRoot must be an absolute path".to_string());
    }
    if !workspace_root.exists() {
        return Err("workspaceRoot does not exist".to_string());
    }
    if !workspace_root.is_dir() {
        return Err("workspaceRoot must be a directory".to_string());
    }
    let server_root = server_root
        .canonicalize()
        .map_err(|err| format!("MCP server root is not accessible: {err}"))?;
    let workspace_root = workspace_root
        .canonicalize()
        .map_err(|err| format!("workspaceRoot is not accessible: {err}"))?;
    if !workspace_root.starts_with(&server_root) {
        return Err("workspaceRoot must be inside the MCP server root".to_string());
    }
    Ok((server_root, workspace_root))
}

/// Redact an absolute workspace path to a server-root-relative form.
pub fn redact_workspace_root(workspace_root: &Path, server_root: &Path) -> String {
    let relative = workspace_root
        .strip_prefix(server_root)
        .expect("workspace root containment must be validated before redaction");

    if relative.as_os_str().is_empty() {
        ".".to_string()
    } else {
        relative.to_string_lossy().replace('\\', "/")
    }
}

/// Collect the workspace-relative file list from a JSON array, rejecting
/// shapes that the MCP tool surface should not accept: non-strings, empty
/// entries, absolute paths, `..` escapes, and arrays larger than
/// [`MAX_FILE_ENTRIES`].
///
/// `field` is the name of the JSON property (e.g. `"files"` or
/// `"targetFiles"`) so the error messages stay parity-shaped with the
/// archived TS contract.
pub fn collect_relative_files(files: &[Value], field: &str) -> Result<Vec<String>, String> {
    if files.len() > MAX_FILE_ENTRIES {
        return Err(format!(
            "{field} must contain at most {MAX_FILE_ENTRIES} entries"
        ));
    }
    let mut out = Vec::with_capacity(files.len());
    for (index, entry) in files.iter().enumerate() {
        let path = entry
            .as_str()
            .ok_or_else(|| format!("{field}[{index}] must be a string"))?;
        if path.is_empty() {
            return Err(format!("{field}[{index}] must not be empty"));
        }
        let candidate = Path::new(path);
        if candidate.is_absolute() {
            return Err(format!(
                "{field}[{index}] must be a workspace-relative path"
            ));
        }
        if candidate
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(format!(
                "{field}[{index}] must not escape the workspace via \"..\""
            ));
        }
        out.push(path.to_string());
    }
    Ok(out)
}

/// Resolve workspace-relative file entries to canonical absolute paths
/// that are guaranteed to live inside `workspace_root`. Symlinks are
/// followed and their targets are re-validated, so a relative entry that
/// points at a symlink targeting `/etc/passwd` is rejected rather than
/// silently read.
///
/// Missing files are tolerated with the empty string at their slot so the
/// scanner's own "file not found" path can produce a parity-shaped
/// response — exactly mirroring the archived TS tool, which delegates to
/// `GateRunner.analyzeFiles` without pre-checking existence.
pub fn resolve_workspace_files(
    workspace_root: &Path,
    relative: &[String],
    field: &str,
) -> Result<Vec<String>, String> {
    let mut absolute = Vec::with_capacity(relative.len());
    for (index, rel) in relative.iter().enumerate() {
        let joined = workspace_root.join(rel);
        let resolved = match joined.canonicalize() {
            Ok(path) => path,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                // Forward the workspace-joined path verbatim. The scanner
                // surfaces a per-file read error in its own envelope.
                absolute.push(joined.to_string_lossy().to_string());
                continue;
            }
            Err(err) => {
                return Err(format!("{field}[{index}] is not accessible: {err}"));
            }
        };
        if !resolved.starts_with(workspace_root) {
            return Err(format!("{field}[{index}] resolves outside workspaceRoot"));
        }
        absolute.push(resolved.to_string_lossy().to_string());
    }
    Ok(absolute)
}

/// Build the `warnings` array shared by `anvil_check` and `anvil_gate`.
pub fn build_warnings_array(warnings: &[Warning]) -> Vec<Value> {
    warnings
        .iter()
        .map(|w| {
            json!({
                "id": w.id,
                "severity": severity_str(w.severity),
                "title": w.title,
                "message": w.message,
                "explanation": w.explanation,
                "suggestion": w.suggestion,
                "nudge": w.nudge,
                "category": category_str(w.category),
                "location": {
                    "file": w.location.file,
                    "line": w.location.line,
                    "column": w.location.column,
                    "endLine": w.location.end_line,
                    "endColumn": w.location.end_column,
                }
            })
        })
        .collect()
}

#[must_use]
pub fn severity_str(severity: WarningSeverity) -> &'static str {
    match severity {
        WarningSeverity::Error => "error",
        WarningSeverity::Warning => "warning",
        WarningSeverity::Info => "info",
    }
}

#[must_use]
pub fn category_str(category: WarningCategory) -> &'static str {
    match category {
        WarningCategory::AntiPattern => "anti-pattern",
        WarningCategory::Boundary => "boundary",
        WarningCategory::Architecture => "architecture",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_rejects_arrays_larger_than_max() {
        let oversize: Vec<Value> = (0..=MAX_FILE_ENTRIES)
            .map(|i| Value::String(format!("src/file_{i}.ts")))
            .collect();

        let err = collect_relative_files(&oversize, "files").unwrap_err();

        assert!(err.contains("at most"));
    }

    #[test]
    fn collect_rejects_non_string_entries() {
        let err = collect_relative_files(&[Value::Bool(true)], "files").unwrap_err();

        assert_eq!(err, "files[0] must be a string");
    }

    #[test]
    fn collect_rejects_empty_entries() {
        let err =
            collect_relative_files(&[Value::String(String::new())], "targetFiles").unwrap_err();

        assert_eq!(err, "targetFiles[0] must not be empty");
    }

    #[test]
    fn collect_rejects_parent_dir_components() {
        let err =
            collect_relative_files(&[Value::String("../escape".to_string())], "files").unwrap_err();

        assert_eq!(err, "files[0] must not escape the workspace via \"..\"");
    }

    #[test]
    fn resolve_rejects_symlink_targets_outside_workspace() {
        #[cfg(unix)]
        {
            let cwd = std::env::current_dir().expect("test cwd is accessible");
            let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
            let outside_dir = tempfile::tempdir_in(&cwd).expect("outside dir exists");
            let outside_file = outside_dir.path().join("secret.txt");
            std::fs::write(&outside_file, "shh").expect("outside file writes");

            let link = workspace.path().join("link.ts");
            std::os::unix::fs::symlink(&outside_file, &link).expect("symlink created");

            let workspace_canonical = workspace
                .path()
                .canonicalize()
                .expect("workspace canonicalises");
            let err =
                resolve_workspace_files(&workspace_canonical, &["link.ts".to_string()], "files")
                    .unwrap_err();

            assert_eq!(err, "files[0] resolves outside workspaceRoot");
        }
    }

    #[test]
    fn resolve_tolerates_missing_files_so_scanner_emits_parity_error() {
        let cwd = std::env::current_dir().expect("test cwd is accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let workspace_canonical = workspace
            .path()
            .canonicalize()
            .expect("workspace canonicalises");

        let resolved = resolve_workspace_files(
            &workspace_canonical,
            &["src/does-not-exist.ts".to_string()],
            "files",
        )
        .expect("missing file is tolerated");

        let expected = workspace_canonical
            .join("src/does-not-exist.ts")
            .to_string_lossy()
            .to_string();
        assert_eq!(resolved, vec![expected]);
    }

    #[test]
    fn resolve_keeps_inside_workspace_symlinks() {
        #[cfg(unix)]
        {
            let cwd = std::env::current_dir().expect("test cwd is accessible");
            let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
            let workspace_canonical = workspace
                .path()
                .canonicalize()
                .expect("workspace canonicalises");
            let real = workspace_canonical.join("real.ts");
            std::fs::write(&real, "export const x = 1;\n").expect("real file writes");
            let link = workspace_canonical.join("link.ts");
            std::os::unix::fs::symlink(&real, &link).expect("symlink created");

            let resolved =
                resolve_workspace_files(&workspace_canonical, &["link.ts".to_string()], "files")
                    .expect("in-workspace symlink resolves");

            assert_eq!(resolved.len(), 1);
            assert!(resolved[0].ends_with("real.ts"));
        }
    }
}
