//! MLP2-016 production [`ValidationEngine`] implementation.
//!
//! The pre-push hook and the `anvil l4-validate` subcommand both route
//! [`anvil_l4::CommitDecision::NeedsL4Validation`] through a
//! [`ValidationEngine`] trait. v1 (MLP2-016 initial wave) bound
//! [`anvil_l4::NoOpValidationEngine`] as the production default,
//! preserving the pre-MLP2-016 surface byte-for-byte (single
//! `InternalError { TimedOut }` + admit push) until a real engine
//! landed. The 2026-05-15 Council audit reopened MLP2-016 because
//! production still bound the no-op, so the typed pipeline was
//! evidence-only — no commit had ever been blocked by a real rule.
//!
//! This module is the real engine. It materialises the commit's tree
//! via `git diff-tree` + `git cat-file --batch` (MLP2-068: one batched
//! blob fetch per commit instead of N+1 `git show` spawns), hands the
//! resulting file paths to [`anvil_checks::antipattern::run_antipattern_check`],
//! and maps the resulting per-rule findings onto
//! [`ValidationDiagnostic`] entries the hook surfaces under
//! [`ValidationVerdict::Block`]. Git plumbing failures degrade to
//! [`ValidationVerdict::EngineUnavailable`] so the hook's
//! "internal failures never block the user" surface (ADR-038 §D-6)
//! stays intact.
//!
//! ## Production binding
//!
//! - [`commands::hook::run_pre_push`] binds
//!   [`CommitAntipatternEngine`] (was `NoOpValidationEngine`).
//! - [`commands::l4_validate::run`] binds it through
//!   [`default_engine`].
//!
//! ## Empty-catalogue degradation (Council #C-016B CRITICAL)
//!
//! `anvil_checks::antipattern` loads its rule catalogue from
//! `patterns/compiled/registry.json` resolved via an upward walk from
//! CWD then from the executable's directory. An installed binary
//! without an accessible registry returns an empty catalogue. Before
//! the audit fix, the engine would scan with zero rules and return
//! `Allow` — silent no-op enforcement masquerading as "the engine
//! ran". [`validate_commit`] now refuses to run when
//! `patterns_count() == 0`, returning
//! `EngineUnavailable { BinaryMissing }` so the hook emits a
//! `ValidationPending` line instead of silently admitting.
//!
//! ## Deliberate Allow paths
//!
//! - A commit that touches only non-scannable extensions (e.g. only
//!   `.md` / `.txt`) admits. The antipattern catalogue targets
//!   source files; nothing else can fire.
//! - A commit that only deletes files (no add/modify) admits.
//!   Antipattern rules detect bad code being introduced; you cannot
//!   carry an antipattern in a deletion. The engine uses
//!   `diff-tree --diff-filter=ACMR` to drop pure-deletion entries
//!   before they hit the scanner so the "I tried to scan a deleted
//!   file" silent skip cannot regress into a way to wave commits
//!   through.
//!
//! ## On-warn surface
//!
//! The engine maps `WarningSeverity::Error` → `Severity::Block` and
//! `WarningSeverity::{Warning, Info}` → `Severity::Warn`. The branch
//! rule's `OnWarn` knob decides whether `Severity::Warn` upgrades to a
//! block; under the default `OnWarn::Allow` policy a Warning-severity
//! antipattern (e.g. the `AP-001` `eslint-disable` rule) surfaces a
//! diagnostic but admits the push. This is intentional — operators
//! must opt into stricter routing per branch — and is pinned by the
//! `warn_only_antipattern_admits_under_on_warn_allow` test below.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anvil_checks::antipattern::{
    AntipatternCheckConfig, WarningSeverity, patterns_count, run_antipattern_check,
};
use anvil_hook::{is_hex_sha, is_zero_sha};
use anvil_l4::{
    EngineUnavailableReason, Severity, ValidationDiagnostic, ValidationEngine, ValidationRequest,
    ValidationVerdict,
};

/// MLP2-016 production engine.
///
/// Stateless — every [`validate`](Self::validate) call materialises the
/// commit's tree fresh into a temp directory and runs the antipattern
/// catalogue against it. The temp directory is dropped at end of call,
/// so no on-disk state survives the validation request.
#[derive(Debug, Default, Clone, Copy)]
pub struct CommitAntipatternEngine;

impl ValidationEngine for CommitAntipatternEngine {
    fn validate(&self, request: &ValidationRequest) -> ValidationVerdict {
        validate_commit(&request.repo_root, &request.commit_sha)
    }
}

/// MLP2-016 default engine constructor. The hook and `l4-validate` bind
/// this. Tests that want to drive the production default path
/// (audit-required) call this and pass the result to the existing
/// `_with_engine` entry points.
#[must_use]
pub fn default_engine() -> Box<dyn ValidationEngine> {
    Box::new(CommitAntipatternEngine)
}

/// Core validation pipeline, factored out so tests can drive it
/// without constructing a [`ValidationRequest`].
fn validate_commit(repo_root: &Path, commit_sha: &str) -> ValidationVerdict {
    validate_commit_with_tempdir(repo_root, commit_sha, tempfile::TempDir::new)
}

fn validate_commit_with_tempdir<F>(
    repo_root: &Path,
    commit_sha: &str,
    make_tempdir: F,
) -> ValidationVerdict
where
    F: FnOnce() -> std::io::Result<tempfile::TempDir>,
{
    // Council #C-016C CRITICAL: a zero SHA passes `is_hex_sha` but is
    // never a real commit. Refusing here keeps `git diff-tree` from
    // being asked to resolve an impossible object and prevents
    // `l4-validate --range 000...0` from reaching the engine.
    if is_zero_sha(commit_sha) {
        return ValidationVerdict::EngineUnavailable {
            reason: EngineUnavailableReason::IoError,
        };
    }
    // Council #C-016B CRITICAL: refuse to run with an empty rule
    // catalogue. The hook collapses `EngineUnavailable` to a
    // `ValidationPending` line so the operator sees that L4 is not
    // enforcing, instead of silent admission.
    if patterns_count() == 0 {
        tracing::warn!(
            target: "anvil::l4_engine",
            commit = %short(commit_sha),
            "antipattern catalogue is empty; refusing to validate",
        );
        return ValidationVerdict::EngineUnavailable {
            reason: EngineUnavailableReason::BinaryMissing,
        };
    }
    let paths = match list_commit_files(repo_root, commit_sha) {
        Ok(paths) => paths,
        Err(reason) => {
            // git binary resolution or repository/object access failed.
            // Preserve the specific reason so missing tooling remains
            // distinct from local I/O outages.
            return ValidationVerdict::EngineUnavailable { reason };
        }
    };
    if paths.is_empty() {
        return ValidationVerdict::Allow;
    }
    let config = AntipatternCheckConfig::default();
    // Filter to scannable extensions BEFORE materialising blobs so a
    // commit that touches a 100 MB binary with a non-scannable
    // extension doesn't pay the `git show` allocation cost.
    let scannable: Vec<&String> = paths
        .iter()
        .filter(|p| config.extensions.iter().any(|ext| p.ends_with(ext)))
        .collect();
    if scannable.is_empty() {
        return ValidationVerdict::Allow;
    }
    let Ok(tmp) = make_tempdir() else {
        // Council #C-016D MAJOR: a `/tmp` allocation failure is an
        // infrastructure outage, not missing tooling or a time-budget
        // overrun. MLP2-069 gives observability a dedicated reason.
        return ValidationVerdict::EngineUnavailable {
            reason: EngineUnavailableReason::IoError,
        };
    };
    let workspace_root = tmp.path().to_path_buf();
    let mut materialised: Vec<String> = Vec::with_capacity(scannable.len());
    // MLP2-068: batch the blob fetch into a single `git cat-file
    // --batch` invocation so a 200-file commit pays one git spawn,
    // not 200.
    let path_refs: Vec<&str> = scannable.iter().map(|s| s.as_str()).collect();
    let blobs = match read_commit_blobs_batch(repo_root, commit_sha, &path_refs) {
        Ok(blobs) => blobs,
        Err(reason) => return ValidationVerdict::EngineUnavailable { reason },
    };
    for (path, blob_opt) in scannable.iter().zip(blobs) {
        let Some(blob) = blob_opt else {
            return ValidationVerdict::EngineUnavailable {
                reason: EngineUnavailableReason::IoError,
            };
        };
        let target = workspace_root.join(path);
        if let Some(parent) = target.parent()
            && std::fs::create_dir_all(parent).is_err()
        {
            return ValidationVerdict::EngineUnavailable {
                reason: EngineUnavailableReason::IoError,
            };
        }
        if std::fs::write(&target, blob).is_err() {
            return ValidationVerdict::EngineUnavailable {
                reason: EngineUnavailableReason::IoError,
            };
        }
        materialised.push(target.to_string_lossy().into_owned());
    }
    if materialised.is_empty() {
        // Every scannable path failed to materialise (corrupt blob,
        // disk-full tmpdir, racing rename). Distinct from "no
        // scannable files" — surface as engine-unavailable so the
        // operator sees a `ValidationPending` line rather than a
        // silent admit. Council #C-016E.
        return ValidationVerdict::EngineUnavailable {
            reason: EngineUnavailableReason::IoError,
        };
    }
    let path_refs: Vec<&str> = materialised.iter().map(String::as_str).collect();
    let workspace_str = workspace_root.to_string_lossy().into_owned();
    let result = run_antipattern_check(&path_refs, &config, Some(&workspace_str));
    let diagnostics: Vec<ValidationDiagnostic> = result
        .warnings
        .warnings
        .iter()
        .filter(|w| w.suppressed.is_none())
        .map(|w| ValidationDiagnostic {
            rule_id: w.id.clone(),
            severity: match w.severity {
                WarningSeverity::Error => Severity::Block,
                WarningSeverity::Warning | WarningSeverity::Info => Severity::Warn,
            },
            message: truncate_message(&w.message),
        })
        .collect();
    if diagnostics.is_empty() {
        ValidationVerdict::Allow
    } else {
        ValidationVerdict::Block { diagnostics }
    }
}

/// `git diff-tree --no-commit-id --name-only -r --root
///   --diff-filter=ACMR <sha>` — returns paths added/changed by the
/// commit, relative to repo root.
///
/// - `--root` makes initial commits report their full tree instead of
///   an empty list, so the engine validates the first commit of a
///   project rather than waving it through.
/// - `--diff-filter=ACMR` drops pure deletions: antipattern rules
///   target code being introduced, so a delete-only commit has no
///   scannable content. Without the filter, the loop body's silent
///   `continue` on `git show <sha>:<deleted-path>` failure would let
///   any delete-only commit collapse to `materialised.is_empty()`
///   without surfacing why (Council #C-016F).
fn list_commit_files(repo_root: &Path, sha: &str) -> Result<Vec<String>, EngineUnavailableReason> {
    if !is_hex_sha(sha) || is_zero_sha(sha) {
        return Err(EngineUnavailableReason::IoError);
    }
    let mut stderr_buf = Vec::new();
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args([
            "diff-tree",
            "--no-commit-id",
            "--name-only",
            "-r",
            "--root",
            "--diff-filter=ACMR",
            sha,
            "--",
        ])
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied => {
                EngineUnavailableReason::BinaryMissing
            }
            _ => EngineUnavailableReason::IoError,
        })?;
    stderr_buf.extend_from_slice(&output.stderr);
    if !output.status.success() {
        log_git_failure("diff-tree", sha, &stderr_buf);
        return Err(EngineUnavailableReason::IoError);
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect())
}

/// MLP2-068: batched blob fetch. Spawns `git cat-file --batch` once,
/// pipes `<sha>:<path>` revspecs for every entry in `paths` over
/// stdin, and returns a vec aligned with `paths` carrying the blob
/// bytes. Returns `None` for per-path refusals or tree misses, and
/// `Err` for batch-level I/O/tooling failures.
///
/// Pre-MLP2-068, the engine spawned one `git show` per scannable file
/// — a 200-file commit paid ~1–3 s on process startup alone, most of
/// `PRE_PUSH_BUDGET`. This helper amortises the spawn cost across the
/// whole batch.
///
/// Guards preserved from the singular `read_commit_blob` it replaces:
/// - Non-hex / zero SHA → entire result is `None` per input, git is
///   never invoked.
/// - Path containing `:` → that entry yields `None` without travelling
///   to git, so `<rev>:<path>` revspec parsing stays unambiguous
///   (Council #C-016G).
///
/// git stderr is forwarded to `tracing::debug!` on invocation failure
/// so production incident debugging can still distinguish "git not on
/// PATH" from "object missing from pack" (MLP2-016 surface intact).
fn read_commit_blobs_batch(
    repo_root: &Path,
    sha: &str,
    paths: &[&str],
) -> Result<Vec<Option<Vec<u8>>>, EngineUnavailableReason> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    if !is_hex_sha(sha) || is_zero_sha(sha) {
        return Err(EngineUnavailableReason::IoError);
    }

    // Build the per-input query list. Paths containing `:` are
    // recorded as refused up-front (per Council #C-016G) and excluded
    // from the stdin batch. `had_query[i]` is `true` iff `paths[i]`
    // produced a stdin line — the parse-output scatter relies on this
    // boolean, not a stored index, to keep the order invariant
    // explicit: each `true` slot consumes exactly one parsed entry in
    // the order git emitted them.
    let mut queries: Vec<String> = Vec::with_capacity(paths.len());
    let mut had_query: Vec<bool> = Vec::with_capacity(paths.len());
    for path in paths {
        if path.contains(':') {
            had_query.push(false);
            continue;
        }
        had_query.push(true);
        queries.push(format!("{sha}:{path}\n"));
    }

    let mut results: Vec<Option<Vec<u8>>> = vec![None; paths.len()];
    if queries.is_empty() {
        return Ok(results);
    }

    let mut child = match Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            return Err(match err.kind() {
                std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied => {
                    EngineUnavailableReason::BinaryMissing
                }
                _ => EngineUnavailableReason::IoError,
            });
        }
    };

    // Stream stdin from a worker thread so a full stdout pipe (large
    // blobs) cannot deadlock against a full stdin pipe. wait_with_output
    // drains stdout/stderr concurrently once we call it, but only if we
    // are not blocked writing stdin first.
    let query_count = queries.len();
    let mut stdin = child.stdin.take().expect("piped stdin");
    let writer = std::thread::spawn(move || {
        for q in &queries {
            if stdin.write_all(q.as_bytes()).is_err() {
                break;
            }
        }
        // Closing stdin signals end-of-input to cat-file --batch so it
        // exits cleanly.
        drop(stdin);
    });

    let Ok(output) = child.wait_with_output() else {
        let _ = writer.join();
        return Err(EngineUnavailableReason::IoError);
    };
    let _ = writer.join();
    if !output.status.success() {
        log_git_failure("cat-file --batch", sha, &output.stderr);
        return Err(EngineUnavailableReason::IoError);
    }

    let Some(parsed) = parse_batch_stdout(&output.stdout, query_count) else {
        log_git_failure("cat-file --batch (parse)", sha, &output.stderr);
        return Err(EngineUnavailableReason::IoError);
    };

    // `parsed` carries one entry per query in the same order they were
    // written to stdin. Drain it into the slots that produced queries,
    // leaving the refused (colon-path) slots as the pre-set `None`.
    // `.flatten()` collapses "iterator exhausted" and "parsed entry is
    // None" — both map to the same fail-safe outcome.
    let mut parsed_iter = parsed.into_iter();
    for (slot, queried) in results.iter_mut().zip(had_query.iter()) {
        if *queried {
            *slot = parsed_iter.next().flatten();
        }
    }
    Ok(results)
}

/// Parse the streaming `git cat-file --batch` stdout into a vec of
/// `expected` entries. Each record is either:
///
/// - `<oid> SP <type> SP <size> LF <size bytes> LF` (object found —
///   `<oid>`/`<type>`/`<size>` each contain no whitespace, so the hit
///   header is exactly three space-separated fields)
/// - `<input> SP missing LF` (revspec did not resolve; `<input>` is
///   echoed verbatim and may contain spaces because filenames legally
///   do, so miss detection anchors on the trailing ` missing` suffix)
///
/// All-or-nothing: any framing error returns `None` for the whole
/// batch. A corrupt mid-stream frame leaves the cursor at an unknown
/// offset, so partial recovery is not safe. The caller degrades to
/// `EngineUnavailable`, which is fail-safe (never silently admits).
///
/// Non-blob types (tree / commit / tag — produced when a path is a
/// submodule gitlink or an unexpected revspec) are reported as `None`
/// for that slot rather than passed through as file content, so the
/// antipattern scanner never sees raw tree or commit bytes
/// masquerading as a source file. The body bytes are still consumed
/// so subsequent records stay aligned.
fn parse_batch_stdout(stdout: &[u8], expected: usize) -> Option<Vec<Option<Vec<u8>>>> {
    let mut out: Vec<Option<Vec<u8>>> = Vec::with_capacity(expected);
    let mut cursor = 0usize;
    while out.len() < expected {
        let rel = stdout.get(cursor..)?.iter().position(|&b| b == b'\n')?;
        let header = &stdout[cursor..cursor + rel];
        cursor += rel + 1;
        let header_str = std::str::from_utf8(header).ok()?;
        // Miss / error lines echo the input (which may contain spaces)
        // followed by ` <reason>`. Suffix-match catches the ` missing`
        // (and defensively the rarer ` ambiguous`) tail without
        // misclassifying a hit header — hit headers are exactly three
        // whitespace-free fields and so never end in ` missing`.
        if header_str.ends_with(" missing") || header_str.ends_with(" ambiguous") {
            out.push(None);
            continue;
        }
        // Hit form: exactly three space-separated, whitespace-free
        // fields. Anything else is unrecognised — return `None` for
        // the whole batch rather than risk reading garbage as body
        // bytes.
        let parts: Vec<&str> = header_str.split(' ').collect();
        if parts.len() != 3 {
            return None;
        }
        let obj_type = parts[1];
        let size: usize = parts[2].parse().ok()?;
        let body_end = cursor.checked_add(size)?;
        if body_end > stdout.len() {
            return None;
        }
        if obj_type == "blob" {
            out.push(Some(stdout[cursor..body_end].to_vec()));
        } else {
            // Non-blob (tree / commit / tag) — discard the body but
            // keep cursor aligned for the next record.
            out.push(None);
        }
        cursor = body_end;
        // Each object body is terminated by a single LF — consume it.
        if stdout.get(cursor) != Some(&b'\n') {
            return None;
        }
        cursor += 1;
    }
    Some(out)
}

/// Forward captured git stderr to `tracing::debug!` so a production
/// incident has a machine-readable trail of why the engine degraded
/// to `EngineUnavailable`. Stays at `debug` level so normal pre-push
/// flow does not flood the operator's terminal.
fn log_git_failure(op: &str, target: &str, stderr_bytes: &[u8]) {
    let err = String::from_utf8_lossy(stderr_bytes);
    let trimmed = err.trim();
    if trimmed.is_empty() {
        tracing::debug!(target: "anvil::l4_engine", op, target = %target, "git invocation failed");
    } else {
        tracing::debug!(
            target: "anvil::l4_engine",
            op,
            target = %target,
            stderr = %trimmed,
            "git invocation failed",
        );
    }
}

/// First 12 chars of a SHA for tracing — pins line length so trace
/// events stay grep-friendly.
fn short(sha: &str) -> String {
    let len = sha.len().min(12);
    sha[..len].to_string()
}

/// The [`ValidationDiagnostic`] contract caps messages at 200 chars
/// (the hook truncates anyway, but the engine is the producer of the
/// contract value — truncate at the source so observability tooling
/// downstream sees the same shape).
fn truncate_message(raw: &str) -> String {
    if raw.chars().count() <= 200 {
        return raw.to_owned();
    }
    let mut t: String = raw.chars().take(197).collect();
    t.push_str("...");
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Build a real git repo with one initial commit containing
    /// `content` at `path`. Returns `(tempdir, repo_root, sha)`.
    /// Kept local rather than promoted to a fixture helper — the
    /// engine module is the only consumer.
    fn commit_with_file(content: &str, path: &str) -> (TempDir, PathBuf, String) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let run = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(args)
                .output()
                .expect("git available");
            assert!(out.status.success(), "git {args:?} failed: {out:?}");
            out
        };
        run(&["init", "-q", "-b", "main"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        run(&["config", "commit.gpgsign", "false"]);
        if let Some(parent) = std::path::Path::new(path).parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(root.join(parent)).unwrap();
        }
        std::fs::write(root.join(path), content).unwrap();
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "first"]);
        let sha = String::from_utf8(run(&["rev-parse", "HEAD"]).stdout)
            .unwrap()
            .trim()
            .to_string();
        (tmp, root, sha)
    }

    fn git_in(root: &Path, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .expect("git available");
        assert!(out.status.success(), "git {args:?} failed: {out:?}");
    }

    /// `list_commit_files` honours `--root` so the initial commit's
    /// files are visible. Pin against a regression that drops the
    /// flag and silently waves first commits through.
    #[test]
    fn list_commit_files_returns_initial_commit_paths() {
        let (_tmp, root, sha) = commit_with_file("export const x = 1;\n", "src/foo.ts");
        let files = list_commit_files(&root, &sha).expect("git diff-tree succeeded");
        assert!(
            files.iter().any(|p| p == "src/foo.ts"),
            "expected src/foo.ts in {files:?}",
        );
    }

    /// Non-hex SHA inputs are refused before invoking git. Defence in
    /// depth — the SHA travels in from policy resolution and the
    /// engine should never feed a revspec or path to `git show`.
    #[test]
    fn list_commit_files_refuses_non_hex_sha() {
        let (_tmp, root, _sha) = commit_with_file("x", "f.txt");
        assert!(list_commit_files(&root, "HEAD").is_err());
        assert!(list_commit_files(&root, "--all").is_err());
    }

    /// Council #C-016C: zero SHA is hex-shaped but never a real
    /// commit. `list_commit_files` must refuse before invoking git.
    #[test]
    fn list_commit_files_refuses_zero_sha() {
        let (_tmp, root, _sha) = commit_with_file("x", "f.txt");
        assert!(list_commit_files(&root, &"0".repeat(40)).is_err());
    }

    /// Council #C-016G: filenames containing a colon would mis-parse
    /// `<rev>:<path>`. The batch helper must yield `None` for those
    /// entries rather than feed git an ambiguous string. Other valid
    /// entries in the same batch must still resolve.
    #[test]
    fn read_commit_blobs_batch_returns_none_for_colon_path() {
        let (_tmp, root, sha) = commit_with_file("body\n", "f.txt");
        let bodies = read_commit_blobs_batch(&root, &sha, &["weird:path.ts", "f.txt"]).unwrap();
        assert_eq!(bodies.len(), 2);
        assert!(bodies[0].is_none(), "colon path must be refused");
        assert_eq!(bodies[1].as_deref(), Some(b"body\n".as_ref()));
    }

    /// MLP2-068: `read_commit_blobs_batch` round-trips a known body
    /// through a single `git cat-file --batch` invocation.
    #[test]
    fn read_commit_blobs_batch_returns_file_bytes() {
        let (_tmp, root, sha) = commit_with_file("body\n", "f.txt");
        let bodies = read_commit_blobs_batch(&root, &sha, &["f.txt"]).unwrap();
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0].as_deref(), Some(b"body\n".as_ref()));
    }

    /// MLP2-068: multiple paths return in input order from a single
    /// git invocation. This is the core contract that lets
    /// `validate_commit` pay O(1) git spawns instead of O(N).
    #[test]
    fn read_commit_blobs_batch_returns_aligned_bodies() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let run = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(args)
                .output()
                .expect("git available");
            assert!(out.status.success(), "git {args:?} failed: {out:?}");
            out
        };
        run(&["init", "-q", "-b", "main"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        run(&["config", "commit.gpgsign", "false"]);
        std::fs::write(root.join("a.ts"), "alpha\n").unwrap();
        std::fs::write(root.join("b.ts"), "bravo\n").unwrap();
        std::fs::write(root.join("c.ts"), "charlie\n").unwrap();
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "three"]);
        let sha = String::from_utf8(run(&["rev-parse", "HEAD"]).stdout)
            .unwrap()
            .trim()
            .to_string();
        let bodies = read_commit_blobs_batch(&root, &sha, &["c.ts", "a.ts", "b.ts"]).unwrap();
        assert_eq!(bodies.len(), 3);
        assert_eq!(bodies[0].as_deref(), Some(b"charlie\n".as_ref()));
        assert_eq!(bodies[1].as_deref(), Some(b"alpha\n".as_ref()));
        assert_eq!(bodies[2].as_deref(), Some(b"bravo\n".as_ref()));
    }

    /// MLP2-068: paths not present in the commit's tree surface as
    /// `None` alongside paths that resolve, so the per-input alignment
    /// invariant survives partial misses.
    #[test]
    fn read_commit_blobs_batch_returns_none_for_missing_path() {
        let (_tmp, root, sha) = commit_with_file("body\n", "f.txt");
        let bodies = read_commit_blobs_batch(&root, &sha, &["f.txt", "missing.ts"]).unwrap();
        assert_eq!(bodies.len(), 2);
        assert_eq!(bodies[0].as_deref(), Some(b"body\n".as_ref()));
        assert!(bodies[1].is_none());
    }

    /// MLP2-068: zero SHA is hex-shaped but never a real commit. The
    /// batch helper refuses before invoking git and returns all `None`
    /// so the engine collapses to `EngineUnavailable` upstream.
    #[test]
    fn read_commit_blobs_batch_refuses_zero_sha() {
        let (_tmp, root, _sha) = commit_with_file("x", "f.txt");
        let err = read_commit_blobs_batch(&root, &"0".repeat(40), &["f.txt", "g.txt"])
            .expect_err("zero sha refused");
        assert_eq!(err, EngineUnavailableReason::IoError);
    }

    /// MLP2-068: non-hex SHA is refused before invoking git.
    #[test]
    fn read_commit_blobs_batch_refuses_non_hex_sha() {
        let (_tmp, root, _sha) = commit_with_file("x", "f.txt");
        let err = read_commit_blobs_batch(&root, "HEAD", &["f.txt"]).expect_err("HEAD refused");
        assert_eq!(err, EngineUnavailableReason::IoError);
    }

    /// MLP2-068: blob bodies are returned as raw bytes — binary
    /// content (NUL bytes, embedded LFs) round-trips exactly and the
    /// next record in the same batch stays aligned. A bug in the
    /// streaming parser that mis-reads body length would corrupt the
    /// second slot, so the dual-file fixture pins the alignment
    /// invariant against the size-driven body read.
    #[test]
    fn read_commit_blobs_batch_round_trips_binary_content() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let run = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(args)
                .output()
                .expect("git available");
            assert!(out.status.success(), "git {args:?} failed: {out:?}");
            out
        };
        run(&["init", "-q", "-b", "main"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        run(&["config", "commit.gpgsign", "false"]);
        let binary: &[u8] = b"\x00\x01\x02 line\n\x00 more\n\xff\xfe";
        let no_newline: &[u8] = b"no-trailing-newline";
        std::fs::write(root.join("bin.dat"), binary).unwrap();
        std::fs::write(root.join("plain.txt"), no_newline).unwrap();
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "two"]);
        let sha = String::from_utf8(run(&["rev-parse", "HEAD"]).stdout)
            .unwrap()
            .trim()
            .to_string();
        let bodies = read_commit_blobs_batch(&root, &sha, &["bin.dat", "plain.txt"]).unwrap();
        assert_eq!(bodies.len(), 2);
        assert_eq!(bodies[0].as_deref(), Some(binary));
        assert_eq!(bodies[1].as_deref(), Some(no_newline));
    }

    /// MLP2-068 Council: a non-blob object (tree / commit / tag) at
    /// the parsed position must surface as `None`, not as raw
    /// non-source bytes the antipattern scanner would happily ingest.
    /// Exercise the parser directly because constructing a real
    /// submodule fixture in a unit test is heavyweight; the parser is
    /// the single chokepoint protecting the scanner.
    #[test]
    fn parse_batch_stdout_refuses_non_blob_object() {
        let stdout = b"deadbeef tree 5\nabcde\ndeadbeef blob 4\nbody\n";
        let parsed = parse_batch_stdout(stdout, 2).expect("framed correctly");
        assert_eq!(parsed.len(), 2);
        assert!(
            parsed[0].is_none(),
            "tree-typed record must not be materialised as blob bytes",
        );
        assert_eq!(parsed[1].as_deref(), Some(b"body".as_ref()));
    }

    /// MLP2-068: `git cat-file --batch` echoes the input verbatim on a
    /// miss. A path that legitimately contains a space — and ends with
    /// the word "missing" — would produce a miss header ending in
    /// ` missing missing`. Pin the parser against treating this as a
    /// hit (which would try to parse "missing" as a size).
    #[test]
    fn parse_batch_stdout_handles_path_ending_in_missing() {
        let stdout = b"deadbeef:weird missing missing\ndeadbeef blob 2\nok\n";
        let parsed = parse_batch_stdout(stdout, 2).expect("framed correctly");
        assert_eq!(parsed.len(), 2);
        assert!(parsed[0].is_none(), "miss line classified as miss");
        assert_eq!(parsed[1].as_deref(), Some(b"ok".as_ref()));
    }

    /// MLP2-068: empty input list does not spawn git at all. Pin the
    /// empty-fast-path to keep the engine's "no scannable files"
    /// short-circuit honest.
    #[test]
    fn read_commit_blobs_batch_empty_input_returns_empty_vec() {
        let (_tmp, root, sha) = commit_with_file("body\n", "f.txt");
        let bodies = read_commit_blobs_batch(&root, &sha, &[]).unwrap();
        assert!(bodies.is_empty());
    }

    /// MLP2-016 reopened: a commit with no scannable files surfaces
    /// `Allow`. The hook treats this as "the engine ran and the
    /// commit passed", admitting the push without an
    /// `engine_unavailable` accumulation.
    #[test]
    fn validate_commit_allows_when_no_scannable_files() {
        let (_tmp, root, sha) = commit_with_file("plain text\n", "README.txt");
        let verdict = validate_commit(&root, &sha);
        assert_eq!(verdict, ValidationVerdict::Allow);
    }

    /// MLP2-069: an unscannable repo path collapses to
    /// `EngineUnavailable { IoError }`. Pre-push routes that to
    /// the legacy `InternalError { TimedOut }` line + admit-push
    /// surface per ADR-038 §D-6.
    #[test]
    fn validate_commit_returns_engine_unavailable_when_git_fails() {
        let tmp = TempDir::new().unwrap();
        // No git init -> `git diff-tree` fails.
        let verdict = validate_commit(tmp.path(), &"a".repeat(40));
        assert_eq!(
            verdict,
            ValidationVerdict::EngineUnavailable {
                reason: EngineUnavailableReason::IoError,
            }
        );
    }

    /// MLP2-069: tempdir allocation failure is an infrastructure I/O
    /// outage, not missing tooling. Pin the injected failure so
    /// production observability can distinguish disk/tmp exhaustion
    /// from "git not on PATH".
    #[test]
    fn tempdir_failure_reports_io_error() {
        let (_tmp, root, sha) = commit_with_file("export const x = 1;\n", "src/foo.ts");
        let verdict = validate_commit_with_tempdir(&root, &sha, || {
            Err(std::io::Error::other("tempdir allocation failed"))
        });
        assert_eq!(
            verdict,
            ValidationVerdict::EngineUnavailable {
                reason: EngineUnavailableReason::IoError,
            }
        );
    }

    /// MLP2-069: a partial scannable-file materialisation failure is
    /// fail-safe. Scanning only the files that happened to materialise
    /// could return `Allow` while skipping the file that contains the
    /// offending code.
    #[test]
    fn partial_materialisation_failure_reports_io_error() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let run = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(args)
                .output()
                .expect("git available");
            assert!(out.status.success(), "git {args:?} failed: {out:?}");
            out
        };
        run(&["init", "-q", "-b", "main"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        run(&["config", "commit.gpgsign", "false"]);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("good.ts"), "export const x = 1;\n").unwrap();
        std::fs::write(root.join("src/bad.ts"), "export const y = 2;\n").unwrap();
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "materialisation failure"]);
        let sha = String::from_utf8(run(&["rev-parse", "HEAD"]).stdout)
            .unwrap()
            .trim()
            .to_string();

        let verdict = validate_commit_with_tempdir(&root, &sha, || {
            let workspace = TempDir::new()?;
            std::fs::write(workspace.path().join("src"), "not a directory")?;
            Ok(workspace)
        });
        assert_eq!(
            verdict,
            ValidationVerdict::EngineUnavailable {
                reason: EngineUnavailableReason::IoError,
            }
        );
    }

    /// Council #C-016C: zero SHA bypasses `git` entirely with a
    /// dedicated `EngineUnavailable` reply.
    #[test]
    fn validate_commit_refuses_zero_sha() {
        let tmp = TempDir::new().unwrap();
        let verdict = validate_commit(tmp.path(), &"0".repeat(40));
        assert_eq!(
            verdict,
            ValidationVerdict::EngineUnavailable {
                reason: EngineUnavailableReason::IoError,
            }
        );
    }

    /// Council #C-016F: a delete-only commit produces no scannable
    /// additions, so `list_commit_files` (with `--diff-filter=ACMR`)
    /// returns an empty list and the engine admits. Pin the
    /// intentional admit so a future regression that drops the
    /// filter and silently fails the per-blob fetch is visible.
    #[test]
    fn validate_commit_admits_delete_only_commit() {
        let (_tmp, root, _initial) = commit_with_file("export const x = 1;\n", "src/foo.ts");
        std::fs::remove_file(root.join("src/foo.ts")).unwrap();
        git_in(&root, &["add", "-A"]);
        git_in(&root, &["commit", "-q", "-m", "delete"]);
        let sha = String::from_utf8(
            Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();
        let verdict = validate_commit(&root, &sha);
        assert_eq!(verdict, ValidationVerdict::Allow);
    }

    /// MLP2-016 reopened: a commit carrying a broad `eslint-disable`
    /// directive triggers `AP-001` from the antipattern catalogue.
    /// The engine surfaces a real `Block { diagnostics }` rather
    /// than `EngineUnavailable`, proving the production default
    /// runs rules (audit requirement).
    #[test]
    fn validate_commit_blocks_on_known_antipattern() {
        let content = "/* eslint-disable */\nimport { x } from './m';\n";
        let (_tmp, root, sha) = commit_with_file(content, "src/leak.ts");
        let verdict = validate_commit(&root, &sha);
        let ValidationVerdict::Block { diagnostics } = verdict else {
            panic!("expected Block, got {verdict:?}");
        };
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "AP-001"),
            "expected AP-001 in {diagnostics:?}",
        );
    }

    /// Council #C-016H MAJOR: `AP-001` is a `Warning`-severity rule
    /// in the registry. The engine maps it to `Severity::Warn`. With
    /// `OnWarn::Allow` (the policy default), the hook surfaces the
    /// diagnostic but admits the push — this is intentional. Pin the
    /// per-diagnostic severity so the on-warn surface stays honest:
    /// "production runs real rules" does not imply "production
    /// blocks every rule."
    #[test]
    fn warn_only_antipattern_admits_under_on_warn_allow() {
        let content = "/* eslint-disable */\nimport { x } from './m';\n";
        let (_tmp, root, sha) = commit_with_file(content, "src/leak.ts");
        let verdict = validate_commit(&root, &sha);
        let ValidationVerdict::Block { diagnostics } = verdict else {
            panic!("expected Block carrier, got {verdict:?}");
        };
        let ap_001 = diagnostics
            .iter()
            .find(|d| d.rule_id == "AP-001")
            .expect("AP-001 present");
        assert_eq!(
            ap_001.severity,
            Severity::Warn,
            "AP-001 must surface as Warn so OnWarn::Allow can admit; \
             a future severity flip to Block would silently change \
             default-policy semantics",
        );
    }

    /// MLP2-016 reopened: `default_engine` returns a real engine —
    /// not the no-op. This is the audit's load-bearing assertion:
    /// the production default constructor must produce something
    /// other than `EngineUnavailable { NotImplemented }`.
    #[test]
    fn default_engine_runs_real_rules_not_no_op() {
        let content = "/* eslint-disable */\nimport { x } from './m';\n";
        let (_tmp, root, sha) = commit_with_file(content, "src/leak.ts");
        let engine = default_engine();
        let request = ValidationRequest {
            commit_sha: sha,
            branch_rule: anvil_l4::BranchRule {
                pattern: "main".to_string(),
                require: anvil_l4::Requirement::L4OrL3,
                on_no_witness: anvil_l4::OnNoWitness::ValidateAtL4,
                on_block: anvil_l4::OnBlock::Reject,
                on_warn: anvil_l4::OnWarn::Reject,
            },
            repo_root: root,
        };
        let verdict = engine.validate(&request);
        // Must not be `EngineUnavailable { NotImplemented }` — that
        // was the pre-fix surface. Either `Allow`, `Block`, or a
        // different `EngineUnavailable` reason are all acceptable
        // ("the engine ran or tried to"), but `NotImplemented`
        // would mean someone re-bound `NoOpValidationEngine`.
        if let ValidationVerdict::EngineUnavailable {
            reason: EngineUnavailableReason::NotImplemented,
        } = verdict
        {
            panic!("production default re-bound NoOpValidationEngine");
        }
    }

    /// MLP2-068: a synthesised 200-file commit must validate well
    /// within the 2 s `PRE_PUSH_BUDGET`. Before the batch path, this
    /// would pay 200× `git show` spawns (~5–15 ms each) and burn most
    /// of the budget before any rule fired. With `git cat-file --batch`
    /// the engine pays one git process per `validate_commit` call.
    /// 1.0 s is the threshold — comfortably under 2 s but far above
    /// the batch path's actual cost, so the test is not flake-prone
    /// on contended CI hardware yet a regression that drops the batch
    /// helper (re-introducing per-file spawns) trips it cleanly.
    #[test]
    fn validate_commit_handles_200_file_commit_under_budget() {
        use std::time::{Duration, Instant};
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let run = |args: &[&str]| {
            let out = Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(args)
                .output()
                .expect("git available");
            assert!(out.status.success(), "git {args:?} failed: {out:?}");
            out
        };
        run(&["init", "-q", "-b", "main"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        run(&["config", "commit.gpgsign", "false"]);
        std::fs::create_dir_all(root.join("src")).unwrap();
        for i in 0..200 {
            let body = format!("export const x{i} = {i};\n");
            std::fs::write(root.join(format!("src/f{i:03}.ts")), body).unwrap();
        }
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "200 files"]);
        let sha = String::from_utf8(run(&["rev-parse", "HEAD"]).stdout)
            .unwrap()
            .trim()
            .to_string();
        let start = Instant::now();
        let verdict = validate_commit(&root, &sha);
        let elapsed = start.elapsed();
        // The fixture intentionally contains no antipattern triggers,
        // so the verdict is `Allow`. The point of the test is wall
        // clock, but pin the verdict shape too so a regression that
        // breaks blob alignment (e.g. wrong-content materialised
        // under wrong path) becomes visible.
        assert!(
            matches!(verdict, ValidationVerdict::Allow),
            "expected Allow for 200 clean files, got {verdict:?}",
        );
        assert!(
            elapsed < Duration::from_secs(1),
            "200-file validation took {elapsed:?}, expected < 1.0 s; \
             regression likely re-introduces per-file git spawns",
        );
    }
}
