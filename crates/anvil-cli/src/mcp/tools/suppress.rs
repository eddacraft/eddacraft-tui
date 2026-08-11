//! `anvil_suppress` — insert a time-boxed inline suppression comment.

use std::fs::{self, OpenOptions};
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use chrono::{Duration, Utc};
use serde_json::{Value, json};

use crate::mcp::tools::shared::{
    WorkspacePathKind, normalise_workspace_relative_path, open_contained_rw_handle,
    redact_workspace_root, validate_workspace_root,
};

pub const TOOL_NAME: &str = "anvil_suppress";

/// Hard limit on the suppression reason so a hostile caller cannot drop a
/// kilobyte of payload into source. Matches the practical reason length used
/// by `anvil suppress` CLI surfaces.
const MAX_REASON_LEN: usize = 512;

/// Hard cap on `expiryDays` so a caller cannot insert a suppression that
/// effectively never expires. Matches the closed-set semantics
/// MLP-009 / ADR-004 imply for time-boxed suppressions.
const MAX_EXPIRY_DAYS: i64 = 365;

pub fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Insert a time-boxed suppression comment for a specific warning. Requires a reason; defaults expiry to 30 days, max 365.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "filePath": {
                    "type": "string",
                    "description": "File path relative to workspaceRoot"
                },
                "warningId": {
                    "type": "string",
                    "description": "Warning ID to suppress (e.g., AP-003)"
                },
                "line": {
                    "type": "integer",
                    "description": "Line number to suppress (1-based)"
                },
                "reason": {
                    "type": "string",
                    "description": "Reason for suppression (mandatory, max 512 chars)"
                },
                "expiryDays": {
                    "type": "integer",
                    "description": "Days until suppression expires (default 30, max 365)"
                },
                "workspaceRoot": {
                    "type": "string",
                    "description": "Absolute path to the project root directory"
                }
            },
            "required": ["filePath", "warningId", "line", "reason", "workspaceRoot"],
            "additionalProperties": true
        },
        "annotations": {
            "readOnlyHint": false,
            "destructiveHint": true,
            "idempotentHint": false
        }
    })
}

pub fn call(arguments: &Value) -> Value {
    let payload = match suppress_payload(arguments) {
        Ok(payload) => payload,
        Err(error) => json!({ "error": error }),
    };
    tool_result(&payload)
}

fn suppress_payload(arguments: &Value) -> Result<Value, String> {
    let server_root = std::env::current_dir()
        .map_err(|err| format!("MCP server cwd is not accessible: {err}"))?;
    let workspace_root = arguments
        .get("workspaceRoot")
        .and_then(Value::as_str)
        .ok_or_else(|| "workspaceRoot is required".to_string())?;
    let file_path = arguments
        .get("filePath")
        .and_then(Value::as_str)
        .ok_or_else(|| "filePath is required".to_string())?;
    let warning_id = arguments
        .get("warningId")
        .and_then(Value::as_str)
        .ok_or_else(|| "warningId is required".to_string())?;
    let line = arguments
        .get("line")
        .and_then(Value::as_i64)
        .ok_or_else(|| "line is required".to_string())?;
    let reason = arguments
        .get("reason")
        .and_then(Value::as_str)
        .ok_or_else(|| "reason is required".to_string())?;
    let expiry_days = arguments
        .get("expiryDays")
        .and_then(Value::as_i64)
        .unwrap_or(30);

    if warning_id.is_empty() {
        return Err("warningId must not be empty".to_string());
    }
    // warningId is interpolated verbatim into a source-code comment. Any
    // newline or carriage return would let a caller inject a second
    // attacker-controlled line into the file, bypassing the `reason`
    // sanitiser. Fail loudly rather than silently strip — a legitimate
    // caller never has `\r` or `\n` in a warning ID.
    if warning_id.contains('\r') || warning_id.contains('\n') {
        return Err("warningId must not contain newline characters".to_string());
    }
    // Same defence for control characters that would break the parser
    // matching `@anvil-ignore-until <date> <id>: <reason>`. We restrict the
    // ID to printable ASCII without spaces, colons, or backticks so the
    // resulting comment is unambiguous to the suppression scanner.
    if warning_id
        .chars()
        .any(|c| c.is_control() || c == ' ' || c == ':' || c == '`')
    {
        return Err(
            "warningId must be printable ASCII without spaces, colons, or backticks".to_string(),
        );
    }
    if reason.trim().is_empty() {
        return Err("reason must not be empty".to_string());
    }
    if reason.len() > MAX_REASON_LEN {
        return Err(format!(
            "reason exceeds {MAX_REASON_LEN}-byte limit ({} bytes)",
            reason.len(),
        ));
    }
    if line < 1 {
        return Err("line must be >= 1".to_string());
    }
    if !(1..=MAX_EXPIRY_DAYS).contains(&expiry_days) {
        return Err(format!(
            "expiryDays must be between 1 and {MAX_EXPIRY_DAYS}",
        ));
    }
    let file_path =
        normalise_workspace_relative_path("filePath", file_path, WorkspacePathKind::Filesystem)?;

    let (server_root, workspace_path) =
        validate_workspace_root(Path::new(workspace_root), &server_root)?;
    let workspace_str = redact_workspace_root(&workspace_path, &server_root);

    let absolute = canonicalise_inside_workspace(&workspace_path, &file_path)?;

    let sanitised_reason = sanitise_reason(reason);
    let expiry_str = expiry_date(expiry_days)?;

    let outcome = insert_suppression_comment(
        &absolute,
        &workspace_path,
        line,
        warning_id,
        &sanitised_reason,
        &expiry_str,
    )?;

    let comment = outcome.comment;
    Ok(json!({
        "suppressed": true,
        "filePath": file_path,
        "line": line,
        "comment": comment,
        "expiryDate": expiry_str,
        "warningId": warning_id,
        "workspaceRoot": workspace_str,
        "backend": "embedded",
        "daemonStatus": "not-wired"
    }))
}

/// Compute the `YYYY-MM-DD` expiry date `expiry_days` from now, failing
/// closed on arithmetic overflow rather than silently wrapping.
fn expiry_date(expiry_days: i64) -> Result<String, String> {
    let expiry = Utc::now()
        .checked_add_signed(Duration::days(expiry_days))
        .ok_or_else(|| "expiry date overflow".to_string())?;
    Ok(expiry.format("%Y-%m-%d").to_string())
}

fn sanitise_reason(reason: &str) -> String {
    reason
        .chars()
        .map(|c| if c == '\r' || c == '\n' { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Resolve a workspace-relative file path to a canonical absolute path,
/// re-verifying that the canonicalised result is still inside the workspace.
fn canonicalise_inside_workspace(workspace_path: &Path, relative: &str) -> Result<PathBuf, String> {
    let joined = workspace_path.join(relative);
    let resolved = joined
        .canonicalize()
        .map_err(|err| format!("filePath is not accessible: {err}"))?;
    if !resolved.starts_with(workspace_path) {
        return Err("filePath resolves outside workspaceRoot".to_string());
    }
    Ok(resolved)
}

struct SuppressOutcome {
    comment: String,
}

/// Insert a `@anvil-ignore-until` comment above `line` in `path`.
///
/// Returns the inserted comment text (with the indent that was matched on
/// the target line, so it lines up visually with the warning's source).
fn insert_suppression_comment(
    path: &Path,
    workspace_path: &Path,
    line: i64,
    warning_id: &str,
    reason: &str,
    expiry_str: &str,
) -> Result<SuppressOutcome, String> {
    // Cross-process advisory lock via `OpenOptions::create_new` on a sibling
    // file: any process on this host that respects the lock is excluded, not
    // just this process. It is not a cross-machine flock — but for a same-host
    // MCP server it stops two concurrent `tools/call` handlers from racing on
    // the same file. The archived TS tool used the same shape.
    let lock_path = path.with_extension(format!(
        "{}.lock",
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("anvil-suppress"),
    ));
    let _lock = AcquiredLock::acquire(&lock_path)?;

    // Open a hardened read+write handle to read the current contents and to
    // probe writability (a read-only target must fail before any write). The
    // replacement itself is path-based (temp + rename). On Unix the open
    // uses O_NOFOLLOW to reject a final-component symlink at check time;
    // Windows has no portable O_NOFOLLOW equivalent here.
    let mut handle = open_contained_rw_handle(path, workspace_path)?;
    let mut content = String::new();
    handle
        .read_to_string(&mut content)
        .map_err(|err| format!("filePath read failed: {err}"))?;
    let mut lines: Vec<String> = content.split('\n').map(String::from).collect();

    let line_idx = usize::try_from(line - 1).map_err(|_| "line must fit in usize".to_string())?;
    if line_idx >= lines.len() {
        return Err(format!(
            "line {line} out of range (file has {} lines)",
            lines.len(),
        ));
    }

    let indent: String = lines[line_idx]
        .chars()
        .take_while(|c| c.is_whitespace() && *c != '\n')
        .collect();
    let comment = format!("{indent}// @anvil-ignore-until {expiry_str} {warning_id}: {reason}");

    lines.insert(line_idx, comment.clone());

    let new_content = lines.join("\n");
    // Drop the open handle before rename. On Windows an open destination
    // blocks `persist`; on every platform the replacement is path-based
    // (POSIX rename replaces a symlink at the destination rather than
    // following it). The earlier open still probes writability; on Unix it
    // also rejects a final-component symlink via O_NOFOLLOW.
    drop(handle);
    // Write via temp-file + rename so a failed write cannot leave the
    // original truncated or partially rewritten (data-loss on
    // ENOSPC/quota/I/O).
    replace_file_contents_atomic(path, new_content.as_bytes())?;

    Ok(SuppressOutcome { comment })
}

/// Replace `path` with `new_content` without ever truncating the original
/// first. Content is written to a sibling temporary file, flushed/synced,
/// then renamed over the destination. On any failure before a successful
/// replace the original file is left byte-for-byte unchanged.
fn replace_file_contents_atomic(path: &Path, new_content: &[u8]) -> Result<(), String> {
    let dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let mut tmp = tempfile::Builder::new()
        .prefix(".anvil-suppress-")
        .tempfile_in(dir)
        .map_err(|err| format!("filePath write failed: {err}"))?;

    tmp.write_all(new_content)
        .map_err(|err| format!("filePath write failed: {err}"))?;
    tmp.flush()
        .map_err(|err| format!("filePath write failed: {err}"))?;
    // Surface delayed ENOSPC/quota/I/O before we replace the destination.
    // `flush` alone is not enough on many filesystems.
    tmp.as_file()
        .sync_all()
        .map_err(|err| format!("filePath write failed: {err}"))?;

    // Preserve the original mode/ACL-ish permissions when the destination
    // already exists. A fresh tempfile may otherwise land with restrictive
    // default perms (e.g. 0o600) that would silently tighten source files.
    if let Ok(meta) = fs::metadata(path) {
        tmp.as_file()
            .set_permissions(meta.permissions())
            .map_err(|err| format!("filePath write failed: {err}"))?;
    }

    // On Windows, `rename` fails if the destination already exists, so move
    // the original aside first. If the new content cannot be installed, restore
    // the backup — never delete the original until the replacement is durable.
    #[cfg(windows)]
    {
        // Include the target file name so concurrent suppress ops on different
        // files in the same directory cannot collide on a pid-only backup path.
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("target");
        let backup = dir.join(format!(
            ".anvil-suppress-backup-{}-{}",
            std::process::id(),
            file_name
        ));
        match fs::rename(path, &backup) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(format!("filePath write failed: {err}")),
        }
        if let Err(err) = tmp.persist(path) {
            // Best-effort restore of the pre-replace content.
            let _ = fs::rename(&backup, path);
            let _ = fs::remove_file(&backup);
            return Err(format!("filePath write failed: {}", err.error));
        }
        let _ = fs::remove_file(&backup);
        Ok(())
    }

    #[cfg(not(windows))]
    {
        tmp.persist(path)
            .map_err(|err| format!("filePath write failed: {}", err.error))?;
        Ok(())
    }
}

struct AcquiredLock {
    path: PathBuf,
}

impl AcquiredLock {
    fn acquire(path: &Path) -> Result<Self, String> {
        match OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(mut handle) => {
                let _ = write!(handle, "{}", std::process::id());
                Ok(Self {
                    path: path.to_path_buf(),
                })
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                Err("filePath is locked by another writer".to_string())
            }
            Err(err) => Err(format!("filePath lock failed: {err}")),
        }
    }
}

impl Drop for AcquiredLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn tool_result(payload: &Value) -> Value {
    let text = serde_json::to_string(payload).expect("suppress payload serialises");
    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "isError": payload.get("error").is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_fixture(dir: &Path, rel: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent dirs created");
        }
        std::fs::write(&path, content).expect("fixture written");
        path
    }

    #[test]
    fn descriptor_advertises_required_fields() {
        let descriptor = descriptor();
        assert_eq!(descriptor["name"], TOOL_NAME);
        let required = descriptor["inputSchema"]["required"]
            .as_array()
            .expect("required is an array");
        for key in ["filePath", "warningId", "line", "reason", "workspaceRoot"] {
            assert!(required.contains(&json!(key)), "required includes {key}");
        }
    }

    #[test]
    fn rejects_missing_reason() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-003",
            "line": 1,
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["error"], "reason is required");
    }

    #[test]
    fn rejects_blank_reason() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-003",
            "line": 1,
            "reason": "   ",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["error"], "reason must not be empty");
    }

    #[test]
    fn rejects_reason_over_limit() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let reason = "x".repeat(MAX_REASON_LEN + 1);
        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-003",
            "line": 1,
            "reason": reason,
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert!(
            payload["error"]
                .as_str()
                .unwrap()
                .contains("exceeds 512-byte limit")
        );
    }

    #[test]
    fn rejects_parent_dir_escape() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let result = call(&json!({
            "filePath": "../etc/passwd",
            "warningId": "AP-003",
            "line": 1,
            "reason": "test",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            payload["error"],
            "filePath must not escape the workspace via \"..\""
        );
    }

    #[test]
    fn rejects_absolute_file_path() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let result = call(&json!({
            "filePath": "/etc/passwd",
            "warningId": "AP-003",
            "line": 1,
            "reason": "test",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            payload["error"],
            "filePath must be a workspace-relative path"
        );
    }

    #[cfg(windows)]
    #[test]
    fn rejects_windows_drive_relative_file_path() {
        // `C:foo` is drive-relative on Windows (a Prefix component with no
        // root): both `is_absolute` and `has_root` are false, yet `join`
        // anchors it to drive C's cwd and escapes the workspace. The
        // first-component check must still reject it.
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let result = call(&json!({
            "filePath": "C:foo",
            "warningId": "AP-003",
            "line": 1,
            "reason": "test",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            payload["error"],
            "filePath must be a workspace-relative path"
        );
    }

    #[test]
    fn rejects_expiry_days_over_limit() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-003",
            "line": 1,
            "reason": "ok",
            "expiryDays": 9999,
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert!(
            payload["error"]
                .as_str()
                .unwrap()
                .contains("expiryDays must be"),
        );
    }

    #[test]
    fn rejects_line_out_of_range() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        write_fixture(workspace.path(), "src/a.ts", "const x: any = 1;\n");
        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-003",
            "line": 999,
            "reason": "TODO clean up",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert!(payload["error"].as_str().unwrap().contains("out of range"));
    }

    #[test]
    fn inserts_comment_above_target_line_with_indent() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        write_fixture(
            workspace.path(),
            "src/a.ts",
            "function f() {\n    const x: any = 1;\n}\n",
        );

        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-003",
            "line": 2,
            "reason": "legacy contract — schedule cleanup in TICKET-123",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], false);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["suppressed"], true);
        assert_eq!(payload["warningId"], "AP-003");
        assert_eq!(payload["line"], 2);
        let comment = payload["comment"].as_str().unwrap();
        assert!(comment.starts_with("    // @anvil-ignore-until "));
        assert!(comment.contains("AP-003: legacy contract"));
        assert_eq!(payload["backend"], "embedded");
        assert_eq!(payload["daemonStatus"], "not-wired");

        let file_content =
            std::fs::read_to_string(workspace.path().join("src/a.ts")).expect("file readable");
        // The new comment is line 2, the original `const x: any = 1;` is now line 3.
        let lines: Vec<&str> = file_content.split('\n').collect();
        assert!(lines[1].contains("@anvil-ignore-until"));
        assert!(lines[2].contains("const x: any = 1;"));
    }

    #[test]
    fn sanitises_newlines_from_reason() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        write_fixture(workspace.path(), "src/a.ts", "const x: any = 1;\n");

        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-003",
            "line": 1,
            "reason": "first line\nsecond line\rinjected",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], false);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        let comment = payload["comment"].as_str().unwrap();
        assert!(!comment.contains('\n'));
        assert!(!comment.contains('\r'));
        assert!(comment.contains("first line second line injected"));
    }

    #[test]
    fn rejects_warning_id_with_newline_to_block_comment_injection() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        write_fixture(workspace.path(), "src/a.ts", "const x: any = 1;\n");

        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-003\nmalicious: injected line",
            "line": 1,
            "reason": "legit",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            payload["error"],
            "warningId must not contain newline characters"
        );

        // File must be untouched.
        let on_disk =
            std::fs::read_to_string(workspace.path().join("src/a.ts")).expect("file readable");
        assert_eq!(on_disk, "const x: any = 1;\n");
    }

    #[test]
    fn rejects_warning_id_with_control_characters() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-003: foo",
            "line": 1,
            "reason": "legit",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert!(
            payload["error"]
                .as_str()
                .unwrap()
                .contains("printable ASCII")
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_target_outside_workspace() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let outside_dir = tempfile::tempdir_in(&cwd).expect("outside dir exists");
        let outside_file = outside_dir.path().join("secret.txt");
        std::fs::write(&outside_file, "shh").expect("outside file writes");

        let link = workspace.path().join("escape.ts");
        std::os::unix::fs::symlink(&outside_file, &link).expect("symlink created");

        let result = call(&json!({
            "filePath": "escape.ts",
            "warningId": "AP-003",
            "line": 1,
            "reason": "blocked",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["error"], "filePath resolves outside workspaceRoot");
    }

    #[cfg(unix)]
    #[test]
    fn suppress_writes_only_the_in_workspace_target() {
        // Functional guard (NOT a race proof — nothing is swapped here): a
        // real suppression is inserted into the in-workspace file and never an
        // unrelated file outside the workspace. The genuine post-open swap
        // race proof lives in
        // `shared::handle_write_pins_inode_against_post_open_symlink_swap`.
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        write_fixture(workspace.path(), "src/a.ts", "const x: any = 1;\n");
        let outside_dir = tempfile::tempdir_in(&cwd).expect("outside dir exists");
        let victim = outside_dir.path().join("victim.txt");
        std::fs::write(&victim, "DO NOT OVERWRITE").expect("victim writes");

        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-003",
            "line": 1,
            "reason": "blocked",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], false);

        let on_disk =
            std::fs::read_to_string(workspace.path().join("src/a.ts")).expect("file readable");
        assert!(on_disk.contains("@anvil-ignore-until"));
        assert_eq!(
            std::fs::read_to_string(&victim).expect("victim readable"),
            "DO NOT OVERWRITE"
        );
    }

    #[test]
    fn atomic_replace_success_updates_content() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let path = write_fixture(workspace.path(), "src/a.ts", "original\n");
        replace_file_contents_atomic(&path, b"replacement\n").expect("atomic replace succeeds");
        assert_eq!(
            std::fs::read_to_string(&path).expect("file readable"),
            "replacement\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replace_failure_preserves_original_bytes() {
        // Simulate a write failure after the target already exists (e.g. the
        // directory cannot accept a new tempfile because it is not writable).
        // The original must remain byte-for-byte unchanged — the truncate-
        // then-write path would have already wiped it.
        use std::os::unix::fs::PermissionsExt as _;

        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let dir = workspace.path().join("src");
        std::fs::create_dir_all(&dir).expect("src dir");
        let path = dir.join("a.ts");
        let original = "const x: any = 1; // preserve me\n";
        std::fs::write(&path, original).expect("fixture written");

        // Drop write bit on the parent so tempfile_in fails. Opening the file
        // itself for write would still succeed (file remains mode 0644).
        let mut dir_perms = std::fs::metadata(&dir).expect("dir meta").permissions();
        dir_perms.set_mode(0o555);
        std::fs::set_permissions(&dir, dir_perms).expect("chmod dir 555");

        let err = replace_file_contents_atomic(&path, b"should not land\n")
            .expect_err("write into non-writable parent must fail");
        assert!(
            err.contains("filePath write failed"),
            "unexpected error: {err}"
        );

        // Restore dir perms before asserts/cleanup so we can read and so
        // TempDir Drop can remove the tree.
        let mut restore = std::fs::metadata(&dir).expect("dir meta").permissions();
        restore.set_mode(0o755);
        std::fs::set_permissions(&dir, restore).expect("chmod dir 755");

        assert_eq!(
            std::fs::read_to_string(&path).expect("file readable"),
            original,
            "failed atomic replace must not truncate or partially rewrite the target"
        );
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replace_preserves_existing_permissions() {
        use std::os::unix::fs::PermissionsExt as _;

        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let path = write_fixture(workspace.path(), "src/a.ts", "before\n");
        let mut perms = std::fs::metadata(&path).expect("metadata").permissions();
        perms.set_mode(0o640);
        std::fs::set_permissions(&path, perms).expect("chmod 640");

        replace_file_contents_atomic(&path, b"after\n").expect("atomic replace succeeds");
        let mode = std::fs::metadata(&path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            mode, 0o640,
            "replacement must keep original mode, got {mode:o}"
        );
        assert_eq!(std::fs::read_to_string(&path).expect("readable"), "after\n");
    }

    #[cfg(unix)]
    #[test]
    fn suppress_failure_path_does_not_mutate_source() {
        // Parent of the target is not writable, so the pre-write lock (and any
        // subsequent tempfile create) fails before mutation. Complements
        // `atomic_replace_failure_preserves_original_bytes`, which covers the
        // write-helper failure mode after the target already exists.
        use std::os::unix::fs::PermissionsExt as _;

        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let original = "const x: any = 1;\n";
        let file = write_fixture(workspace.path(), "src/a.ts", original);
        let parent = file.parent().expect("parent");

        let mut dir_perms = std::fs::metadata(parent).expect("dir meta").permissions();
        dir_perms.set_mode(0o555);
        std::fs::set_permissions(parent, dir_perms).expect("chmod parent 555");

        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-003",
            "line": 1,
            "reason": "must not wipe source on write failure",
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);

        let mut restore = std::fs::metadata(parent).expect("dir meta").permissions();
        restore.set_mode(0o755);
        std::fs::set_permissions(parent, restore).expect("chmod parent 755");

        assert_eq!(
            std::fs::read_to_string(&file).expect("file readable"),
            original,
            "failed suppress must not leave the source truncated or partially rewritten"
        );
    }
}
