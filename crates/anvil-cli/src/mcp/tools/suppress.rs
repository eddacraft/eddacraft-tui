//! `anvil_suppress` — insert a time-boxed suppression comment.
//!
//! RMCPF-002 classifies `anvil_suppress` as a daemon-RPC translator
//! whose authority is `suppression.apply`. No daemon `suppression.apply`
//! RPC exists yet, and RMCPF-002 forbids inventing one purely for parity
//! prose. This handler therefore ships as the **daemon-RPC translator's
//! correctness-equivalent embedded fallback** — the same shape RMCPF-010
//! used for `anvil_check`. When INTD lands `suppression.apply`, this
//! handler flips to the daemon-RPC translator path without an MCP
//! contract change.
//!
//! Behaviour parity with
//! `anvil-archive/anvil-mcp-server/src/tools/suppress.tool.ts`:
//!
//! - Validates `filePath` is workspace-relative, rejects `..` escapes
//!   and absolute paths, canonicalises the joined path, and re-verifies
//!   it stays inside the workspace (closing the symlink-target escape
//!   vector that hit the RMCPF-010 reviewer). The subsequent read and
//!   write both go through a single containment-checked file handle
//!   ([`open_contained_rw_handle`]), so a symlink swapped in after the
//!   check cannot redirect the write (CIB-145 check-then-use / TOCTOU
//!   hardening).
//! - Sanitises `reason` by replacing `\r\n` characters with a single
//!   space and trimming, so the inserted comment cannot inject newlines
//!   into source code.
//! - Defaults `expiryDays` to 30; clamps to a sane range.
//! - Inserts `// @anvil-ignore-until YYYY-MM-DD <warningId>: <reason>`
//!   above the target line, preserving the target line's indent.
//! - Uses a cross-process advisory file lock (atomic create-with-exclusive
//!   sibling file, same-host) to prevent two concurrent suppression writes
//!   from interleaving.

use std::fs::{self, OpenOptions};
use std::io::{Read as _, Seek as _, Write as _};
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

    // Open ONCE and read + write through the same handle so a symlink swapped
    // in after `canonicalise_inside_workspace` cannot redirect the write.
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
    handle
        .rewind()
        .map_err(|err| format!("filePath write failed: {err}"))?;
    handle
        .set_len(0)
        .map_err(|err| format!("filePath write failed: {err}"))?;
    handle
        .write_all(new_content.as_bytes())
        .map_err(|err| format!("filePath write failed: {err}"))?;

    Ok(SuppressOutcome { comment })
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
}
