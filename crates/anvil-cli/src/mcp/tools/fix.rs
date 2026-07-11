//! `anvil_fix` — deterministic mechanical fixes for selected antipattern warnings.
//!
//! RMCPF-011 MCP-driver-local composition: applies non-heuristic
//! line-by-line transforms in-process. The Rust handler matches the
//! archived TS fixer contract
//! (`anvil-archive/anvil-mcp-server/src/tools/fix.tool.ts`) for the
//! deterministic patterns the TS server shipped:
//!
//! | Warning | Transform                                                                          |
//! | ------- | ---------------------------------------------------------------------------------- |
//! | `AP-001` | Replace `/* eslint-disable */` line comment with `// eslint-disable-next-line`   |
//! | `AP-003` | Replace `: any` annotation with `: unknown` (skipping strings + comments)        |
//! | `AP-004` | Replace `@ts-ignore` with `@ts-expect-error`                                     |
//!
//! As in the archived TS tool, the transforms are intentionally narrow
//! and may not match `: any` inside template literals or generic
//! parameters — those cases keep the original line so the LLM/operator
//! can review manually. The TS tool surfaced the same limitation in its
//! description.
//!
//! Safety contract matches `anvil_suppress`:
//! - Workspace-relative path required, no `..` or absolute escapes.
//! - Canonicalised path is re-verified to stay inside the workspace.
//! - The read goes through a hardened read-only handle
//!   ([`open_contained_ro_handle`]); only when a change is actually needed
//!   does the write go through a second, independently-hardened read+write
//!   handle ([`open_contained_rw_handle`]). Reading read-only keeps a no-op
//!   fix from needing write permission; each hardened open is containment-
//!   checked, so a symlink swapped in after the check cannot redirect the
//!   write (closes the CIB-145 check-then-use / TOCTOU window).
//! - Cross-process advisory lock (exclusive `create_new` sibling file)
//!   guards concurrent writers on the same host.

use std::fs::{self, OpenOptions};
use std::io::{Read as _, Seek as _, Write as _};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::mcp::tools::shared::{
    WorkspacePathKind, normalise_workspace_relative_path, open_contained_ro_handle,
    open_contained_rw_handle, redact_workspace_root, validate_workspace_root,
};

pub const TOOL_NAME: &str = "anvil_fix";

pub fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "Apply deterministic auto-fixes for known antipattern warnings. Supports AP-001 (broad eslint-disable), AP-003 (explicit any), AP-004 (@ts-ignore). The AP-003 transform is line-by-line and intentionally skips generic/union/string-literal occurrences — review applied changes.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "filePath": {
                    "type": "string",
                    "description": "File path relative to workspaceRoot"
                },
                "warningId": {
                    "type": "string",
                    "description": "Warning/pattern ID (e.g. AP-003, AP-004)"
                },
                "line": {
                    "type": "integer",
                    "description": "Line number of the warning (1-based)"
                },
                "workspaceRoot": {
                    "type": "string",
                    "description": "Absolute path to the project root directory"
                }
            },
            "required": ["filePath", "warningId", "line", "workspaceRoot"],
            "additionalProperties": true
        },
        "annotations": {
            "readOnlyHint": false,
            "destructiveHint": true,
            "idempotentHint": true
        }
    })
}

pub fn call(arguments: &Value) -> Value {
    let payload = match fix_payload(arguments) {
        Ok(payload) => payload,
        Err(error) => json!({ "error": error }),
    };
    tool_result(&payload)
}

fn fix_payload(arguments: &Value) -> Result<Value, String> {
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

    if warning_id.is_empty() {
        return Err("warningId must not be empty".to_string());
    }
    if line < 1 {
        return Err("line must be >= 1".to_string());
    }
    let file_path =
        normalise_workspace_relative_path("filePath", file_path, WorkspacePathKind::Filesystem)?;

    let (server_root, workspace_path) =
        validate_workspace_root(Path::new(workspace_root), &server_root)?;
    let workspace_str = redact_workspace_root(&workspace_path, &server_root);

    let Some(transform) = transform_for(warning_id) else {
        return Ok(json!({
            "fixed": false,
            "reason": format!(
                "No auto-fix available for {warning_id}. Fixable patterns: AP-001, AP-003, AP-004",
            ),
            "filePath": file_path,
            "workspaceRoot": workspace_str,
            "backend": "local",
            "daemonStatus": "not-wired"
        }));
    };

    let absolute = canonicalise_inside_workspace(&workspace_path, &file_path)?;

    apply_fix(
        &absolute,
        &workspace_path,
        line,
        warning_id,
        transform,
        &file_path,
        &workspace_str,
    )
}

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

type LineTransform = fn(&str) -> Option<String>;

fn transform_for(warning_id: &str) -> Option<(&'static str, LineTransform)> {
    match warning_id {
        "AP-001" => Some((
            "Replace broad eslint-disable with eslint-disable-next-line",
            fix_ap_001,
        )),
        "AP-003" => Some(("Replace explicit `any` type with `unknown`", fix_ap_003)),
        "AP-004" => Some(("Replace @ts-ignore with @ts-expect-error", fix_ap_004)),
        _ => None,
    }
}

fn fix_ap_001(line: &str) -> Option<String> {
    // Only transform a `/* eslint-disable */` block comment that sits on its
    // own line. The replacement `// eslint-disable-next-line` is a line-
    // comment directive — it only suppresses the NEXT line. Rewriting an
    // inline `code(); /* eslint-disable */` to `code(); // eslint-disable-
    // next-line` would silently leak the disable to the wrong line and
    // could keep an unintended suppression active forever. Fail closed.
    let trimmed = line.trim_start();
    if !trimmed.starts_with("/*") {
        return None;
    }
    let after = trimmed
        .strip_prefix("/*")
        .map(str::trim_start)
        .and_then(|s| s.strip_prefix("eslint-disable"))
        .map(str::trim)
        .and_then(|s| s.strip_prefix("*/"))
        .map(str::trim_start);
    let remainder = after?;
    if !remainder.is_empty() {
        return None;
    }
    let indent_len = line.len() - trimmed.len();
    Some(format!(
        "{}// eslint-disable-next-line",
        &line[..indent_len],
    ))
}

/// Replace every `@ts-ignore` on the line with `@ts-expect-error`.
///
/// This matches the archived TS tool's regex-based behaviour, which also
/// did not distinguish strings/comments. The replacement is a no-op on
/// non-directive occurrences in TypeScript — `@ts-expect-error` inside a
/// string literal is just data — so the parity matters more than the
/// false positive. If a future Council finding makes string-aware
/// rewriting a hard requirement, swap to the `fix_ap_003` walker.
fn fix_ap_004(line: &str) -> Option<String> {
    if line.contains("@ts-ignore") {
        Some(line.replace("@ts-ignore", "@ts-expect-error"))
    } else {
        None
    }
}

/// Replace `: any` with `: unknown` on a single line, ignoring matches that
/// fall inside string literals or comments. Mirrors the archived TS
/// character-walker so the same edge cases hit the same outcomes.
fn fix_ap_003(line: &str) -> Option<String> {
    let bytes: Vec<char> = line.chars().collect();
    if bytes.is_empty() {
        return None;
    }
    // Quick reject: skip lines that don't carry a `: any` shape at all.
    if !line.contains(": any") && !line.contains(":any") {
        return None;
    }
    // Skip full-line line-comments and JSDoc continuation lines.
    let leading = line.trim_start();
    if leading.starts_with("//") || leading.starts_with("* ") {
        return None;
    }

    let mut result = String::with_capacity(line.len() + 4);
    let mut in_string: Option<char> = None;
    let mut in_block_comment = false;
    let mut i = 0;
    let mut changed = false;
    while i < bytes.len() {
        let ch = bytes[i];
        let next = bytes.get(i + 1).copied();
        if let Some(quote) = in_string {
            result.push(ch);
            if ch == '\\'
                && let Some(escape) = next
            {
                result.push(escape);
                i += 2;
                continue;
            }
            if ch == quote {
                in_string = None;
            }
            i += 1;
            continue;
        }
        if in_block_comment {
            result.push(ch);
            if ch == '*' && next == Some('/') {
                result.push('/');
                i += 2;
                in_block_comment = false;
                continue;
            }
            i += 1;
            continue;
        }
        if ch == '/' && next == Some('/') {
            result.push_str(&line[char_byte_index(&bytes, i)..]);
            break;
        }
        if ch == '/' && next == Some('*') {
            result.push_str("/*");
            in_block_comment = true;
            i += 2;
            continue;
        }
        if ch == '"' || ch == '\'' || ch == '`' {
            in_string = Some(ch);
            result.push(ch);
            i += 1;
            continue;
        }
        if ch == ':' && matches_colon_any(&bytes, i) {
            // Skip the colon and consume the whitespace + `any` token.
            result.push(':');
            i += 1;
            while i < bytes.len() && (bytes[i] == ' ' || bytes[i] == '\t') {
                result.push(bytes[i]);
                i += 1;
            }
            // Now `bytes[i..i+3]` is `any` and `bytes[i+3]` (if present) is
            // not an identifier continuation.
            result.push_str("unknown");
            i += 3;
            changed = true;
            continue;
        }
        result.push(ch);
        i += 1;
    }
    if changed { Some(result) } else { None }
}

fn matches_colon_any(bytes: &[char], i: usize) -> bool {
    // Require `:` followed by zero or more horizontal whitespace and then
    // `any` followed by a non-word boundary.
    if bytes.get(i) != Some(&':') {
        return false;
    }
    let mut j = i + 1;
    while j < bytes.len() && (bytes[j] == ' ' || bytes[j] == '\t') {
        j += 1;
    }
    if j + 3 > bytes.len() {
        return false;
    }
    if bytes[j] != 'a' || bytes[j + 1] != 'n' || bytes[j + 2] != 'y' {
        return false;
    }
    match bytes.get(j + 3) {
        Some(c) if c.is_alphanumeric() || *c == '_' || *c == '$' => false,
        None | Some(_) => true,
    }
}

fn char_byte_index(bytes: &[char], char_idx: usize) -> usize {
    bytes
        .iter()
        .take(char_idx)
        .map(|c| c.len_utf8())
        .sum::<usize>()
}

fn apply_fix(
    path: &Path,
    workspace_path: &Path,
    line_no: i64,
    warning_id: &str,
    transform: (&'static str, LineTransform),
    file_path: &str,
    workspace_str: &str,
) -> Result<Value, String> {
    let (description, fixer) = transform;

    let lock_path = path.with_extension(format!(
        "{}.fix-lock",
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("anvil-fix"),
    ));
    let _lock = AcquiredLock::acquire(&lock_path)?;

    // Read through a hardened READ-ONLY handle first. A no-op fix (the
    // pattern is not present on the target line) must not require write
    // permission — opening read+write up front would regress a `fixed:false`
    // result into a permission error on a read-only file. Each hardened open
    // is independently containment-checked, so escalating to a read+write
    // open below (only when a change is actually needed) preserves the
    // check-then-use (TOCTOU) invariant.
    let mut read_handle = open_contained_ro_handle(path, workspace_path)?;
    let mut content = String::new();
    read_handle
        .read_to_string(&mut content)
        .map_err(|err| format!("filePath read failed: {err}"))?;
    drop(read_handle);
    let mut lines: Vec<String> = content.split('\n').map(String::from).collect();

    let line_idx =
        usize::try_from(line_no - 1).map_err(|_| "line must fit in usize".to_string())?;
    if line_idx >= lines.len() {
        return Err(format!(
            "line {line_no} out of range (file has {} lines)",
            lines.len(),
        ));
    }

    let before = lines[line_idx].clone();
    let after = fixer(&before);

    let Some(after) = after else {
        return Ok(json!({
            "fixed": false,
            "reason": format!("Pattern {warning_id} not found on line {line_no}"),
            "filePath": file_path,
            "workspaceRoot": workspace_str,
            "backend": "local",
            "daemonStatus": "not-wired"
        }));
    };

    lines[line_idx].clone_from(&after);
    let new_content = lines.join("\n");
    // A change is required: open a hardened read+write handle now and write
    // through it. A read-only target correctly errors here — we genuinely
    // cannot apply the fix. This second open is independently containment-
    // checked, so the TOCTOU invariant holds across the read→write boundary.
    let mut write_handle = open_contained_rw_handle(path, workspace_path)?;
    write_handle
        .rewind()
        .map_err(|err| format!("filePath write failed: {err}"))?;
    write_handle
        .set_len(0)
        .map_err(|err| format!("filePath write failed: {err}"))?;
    write_handle
        .write_all(new_content.as_bytes())
        .map_err(|err| format!("filePath write failed: {err}"))?;

    Ok(json!({
        "fixed": true,
        "description": description,
        "filePath": file_path,
        "line": line_no,
        "before": before.trim(),
        "after": after.trim(),
        "warningId": warning_id,
        "workspaceRoot": workspace_str,
        "backend": "local",
        "daemonStatus": "not-wired"
    }))
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
    let text = serde_json::to_string(payload).expect("fix payload serialises");
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
        for key in ["filePath", "warningId", "line", "workspaceRoot"] {
            assert!(required.contains(&json!(key)), "required includes {key}");
        }
    }

    #[test]
    fn unknown_warning_id_returns_unfixed_with_reason() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "UNKNOWN-42",
            "line": 1,
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], false);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["fixed"], false);
        assert!(
            payload["reason"]
                .as_str()
                .unwrap()
                .contains("No auto-fix available")
        );
    }

    #[test]
    fn ap_003_replaces_any_with_unknown() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        write_fixture(workspace.path(), "src/a.ts", "const x: any = 1;\n");

        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-003",
            "line": 1,
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], false);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["fixed"], true);
        assert_eq!(payload["before"], "const x: any = 1;");
        assert_eq!(payload["after"], "const x: unknown = 1;");

        let on_disk =
            std::fs::read_to_string(workspace.path().join("src/a.ts")).expect("file readable");
        assert!(on_disk.contains("const x: unknown = 1;"));
    }

    #[test]
    fn ap_003_does_not_touch_string_literals() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        write_fixture(
            workspace.path(),
            "src/a.ts",
            "const msg = \"warn: any usage detected\";\n",
        );
        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-003",
            "line": 1,
            "workspaceRoot": workspace.path()
        }));
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["fixed"], false);
        assert!(payload["reason"].as_str().unwrap().contains("not found"),);
    }

    #[test]
    fn ap_003_skips_jsdoc_continuation_lines() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        write_fixture(workspace.path(), "src/a.ts", " * type: any (documented)\n");
        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-003",
            "line": 1,
            "workspaceRoot": workspace.path()
        }));
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["fixed"], false);
    }

    #[test]
    fn ap_004_replaces_ts_ignore_with_expect_error() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        write_fixture(
            workspace.path(),
            "src/a.ts",
            "// @ts-ignore legacy\nconst x = 1;\n",
        );

        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-004",
            "line": 1,
            "workspaceRoot": workspace.path()
        }));
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["fixed"], true);
        assert!(
            payload["after"]
                .as_str()
                .unwrap()
                .contains("@ts-expect-error")
        );
        let on_disk =
            std::fs::read_to_string(workspace.path().join("src/a.ts")).expect("file readable");
        assert!(on_disk.contains("@ts-expect-error legacy"));
    }

    #[test]
    fn ap_001_does_not_rewrite_inline_block_comment_after_code() {
        // `code(); /* eslint-disable */` MUST NOT become
        // `code(); // eslint-disable-next-line` — line-comment directives
        // suppress the NEXT line, so rewriting would either leak the
        // suppression to unrelated code or make the disable no-op. Fail
        // closed.
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        write_fixture(
            workspace.path(),
            "src/a.ts",
            "doSomething(); /* eslint-disable */\nrelated();\n",
        );

        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-001",
            "line": 1,
            "workspaceRoot": workspace.path()
        }));
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["fixed"], false);

        // File must be untouched.
        let on_disk =
            std::fs::read_to_string(workspace.path().join("src/a.ts")).expect("file readable");
        assert!(on_disk.contains("doSomething(); /* eslint-disable */"));
    }

    #[test]
    fn ap_001_replaces_block_eslint_disable_with_next_line() {
        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        write_fixture(
            workspace.path(),
            "src/a.ts",
            "/* eslint-disable */\nfunction f() {}\n",
        );

        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-001",
            "line": 1,
            "workspaceRoot": workspace.path()
        }));
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["fixed"], true);
        assert!(
            payload["after"]
                .as_str()
                .unwrap()
                .contains("// eslint-disable-next-line")
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
    fn rejects_workspace_outside_server_root() {
        let other = tempfile::tempdir().expect("foreign workspace exists");
        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-003",
            "line": 1,
            "workspaceRoot": other.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            payload["error"],
            "workspaceRoot must be inside the MCP server root"
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
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert!(payload["error"].as_str().unwrap().contains("out of range"));
    }

    #[cfg(unix)]
    #[test]
    fn noop_fix_on_read_only_file_returns_unfixed_not_error() {
        // Regression: a no-op fix (pattern absent) against a read-only file
        // must return `fixed:false`, NOT a "Permission denied" error. The
        // read goes through a read-only handle, so no write permission is
        // required unless a change is actually applied.
        use std::os::unix::fs::PermissionsExt as _;

        let cwd = std::env::current_dir().expect("cwd accessible");
        let workspace = tempfile::tempdir_in(&cwd).expect("workspace exists");
        // No `: any` on the line -> fixer returns None (no-op).
        let file = write_fixture(workspace.path(), "src/a.ts", "const x: number = 1;\n");
        let mut perms = std::fs::metadata(&file).expect("metadata").permissions();
        perms.set_mode(0o444);
        std::fs::set_permissions(&file, perms).expect("chmod 444");

        let result = call(&json!({
            "filePath": "src/a.ts",
            "warningId": "AP-003",
            "line": 1,
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(
            result["isError"], false,
            "no-op on a read-only file must not error"
        );
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["fixed"], false);
        assert!(payload["reason"].as_str().unwrap().contains("not found"));

        // The file is untouched.
        assert_eq!(
            std::fs::read_to_string(&file).expect("file readable"),
            "const x: number = 1;\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_target_outside_workspace() {
        // Parity with `anvil_suppress`: a workspace-relative path that resolves
        // (at check time) to a symlink pointing outside the workspace must be
        // rejected, not followed. Closes the CIB-145 test-parity gap.
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
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], true);
        let payload: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(payload["error"], "filePath resolves outside workspaceRoot");

        // The outside file is untouched.
        assert_eq!(
            std::fs::read_to_string(&outside_file).expect("outside file readable"),
            "shh"
        );
    }

    #[cfg(unix)]
    #[test]
    fn fix_writes_only_the_in_workspace_target() {
        // Functional guard (NOT a race proof — nothing is swapped here): a
        // real fix rewrites the in-workspace file and never an unrelated file
        // outside the workspace. The genuine post-open swap race proof lives
        // in `shared::handle_write_pins_inode_against_post_open_symlink_swap`.
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
            "workspaceRoot": workspace.path()
        }));
        assert_eq!(result["isError"], false);

        // The real file was rewritten; the outside victim is untouched.
        let on_disk =
            std::fs::read_to_string(workspace.path().join("src/a.ts")).expect("file readable");
        assert!(on_disk.contains("const x: unknown = 1;"));
        assert_eq!(
            std::fs::read_to_string(&victim).expect("victim readable"),
            "DO NOT OVERWRITE"
        );
    }
}
