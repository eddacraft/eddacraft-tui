//! MLP2-016 production [`ValidationEngine`] for pre-push / `anvil l4-validate`.
//!
//! Materialises the commit tree (`git diff-tree` + batched `cat-file`), runs
//! antipattern checks, maps findings to [`ValidationVerdict`]. Empty catalogue
//! → `EngineUnavailable` (never silent `Allow`). Non-scannable or delete-only
//! commits `Allow`. Git failures degrade to `EngineUnavailable` (ADR-038).

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anvil_checks::antipattern::{
    AntipatternCheckConfig, Warning, WarningSeverity, patterns_count, run_antipattern_check,
};
use anvil_hook::{is_hex_sha, is_zero_sha};
use anvil_l4::{
    EngineUnavailableReason, ExceptionDisposition, Severity, ValidationDiagnostic,
    ValidationEngine, ValidationRequest, ValidationVerdict, apply_exception_dispositions,
};
use anvil_policy::exceptions::{
    EXCEPTIONS_FILE, ExceptionStore, ExceptionVerdict, verify_exception_at,
};
use chrono::Utc;

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
        validate_commit(
            &request.repo_root,
            &request.commit_sha,
            request.exceptions_tip_sha.as_deref(),
        )
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
fn validate_commit(
    repo_root: &Path,
    commit_sha: &str,
    exceptions_tip: Option<&str>,
) -> ValidationVerdict {
    validate_commit_with_tempdir(
        repo_root,
        commit_sha,
        exceptions_tip,
        tempfile::TempDir::new,
    )
}

fn validate_commit_with_tempdir<F>(
    repo_root: &Path,
    commit_sha: &str,
    exceptions_tip: Option<&str>,
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
    let visible: Vec<&Warning> = result
        .warnings
        .warnings
        .iter()
        .filter(|w| w.suppressed.is_none())
        .collect();
    verdict_for_warnings(repo_root, commit_sha, exceptions_tip, &visible)
}

/// Map the scanner's visible warnings onto the gate verdict, applying
/// tracked policy exceptions (EXCEPT-006, ADR-073) before the verdict
/// forms. Matching runs against the *repo* root's store — the temp
/// workspace holds only materialised blobs — while the warning's
/// workspace-relative path is what exception globs match.
fn verdict_for_warnings(
    repo_root: &Path,
    commit_sha: &str,
    exceptions_tip: Option<&str>,
    visible: &[&Warning],
) -> ValidationVerdict {
    let diagnostics: Vec<ValidationDiagnostic> = visible
        .iter()
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
        return ValidationVerdict::Allow;
    }
    let dispositions = exception_dispositions(repo_root, commit_sha, exceptions_tip, visible);
    let outcome = apply_exception_dispositions(diagnostics, &dispositions);
    for applied in &outcome.applied {
        // The recorded trail of exception use. `tracing` is this
        // surface's established machine-readable channel (see the
        // pre-push hook's engine_unavailable event); durable
        // witness-envelope and capsule inclusion build on the same
        // applied record (EXCEPT-009). An unattributed application is
        // a trust fault, so it records at `warn` — visible under the
        // CLI's default `warn` filter; a clean attributed suppression
        // is authorised success and stays at `info` (ADR-038
        // silent-on-success), also surfaced by the annotated Warn
        // diagnostic the operator sees.
        // `exception_id` is operator-editable store content — log it
        // with `?` (Debug) so embedded control characters are escaped
        // in human-facing output rather than emitted raw.
        if applied.downgraded {
            tracing::warn!(
                target: "anvil::l4_engine",
                kind = "exception_applied_downgraded",
                exception_id = ?applied.exception_id,
                rule_id = %applied.rule_id,
                commit = %short(commit_sha),
                "unattributed policy exception applied to L4 finding (downgraded to warn)",
            );
        } else {
            tracing::info!(
                target: "anvil::l4_engine",
                kind = "exception_applied",
                exception_id = ?applied.exception_id,
                rule_id = %applied.rule_id,
                commit = %short(commit_sha),
                "policy exception applied to L4 finding",
            );
        }
    }
    outcome.verdict
}

/// EXCEPT-006: compute one [`ExceptionDisposition`] per visible
/// warning from the exception store committed in the suppression-
/// authority tip's tree (`anvil/exceptions/store.json`, ADR-073 +
/// ADR-100).
///
/// - The store is read from the **tip commit's tree**, never the
///   worktree: suppression authority must be committed to count
///   (ADR-100). No tip, no store in the tip, or an unreadable /
///   oversized / malformed store blob → no exceptions apply
///   (fail-safe: findings stand, the gate blocks rather than
///   silently admitting).
/// - Revoked / expired / invalid-scope grants never apply
///   (`verify_exception_at` precedence).
/// - When both an attributed and an unattributed grant cover the same
///   finding, the attributed one wins — a clean suppression is
///   preferred over a downgrade, independent of store order.
fn exception_dispositions(
    repo_root: &Path,
    commit_sha: &str,
    exceptions_tip: Option<&str>,
    warnings: &[&Warning],
) -> Vec<ExceptionDisposition> {
    let not_covered = vec![ExceptionDisposition::NotCovered; warnings.len()];
    if warnings.is_empty() {
        return not_covered;
    }
    let Some(tip) = exceptions_tip else {
        return not_covered;
    };
    let store = match load_committed_store(repo_root, tip) {
        StoreFromTip::Absent => return not_covered,
        StoreFromTip::Unreadable(detail) => {
            tracing::warn!(
                target: "anvil::l4_engine",
                kind = "exception_store_unreadable",
                commit = %short(commit_sha),
                tip = %short(tip),
                detail = %detail,
                "committed exception store unreadable; no exceptions applied",
            );
            return not_covered;
        }
        StoreFromTip::Loaded(store) => store,
    };
    let now = Utc::now();
    // Verdicts depend only on the grant and `now`, so classify each
    // grant once (not per warning) and keep only the ones that apply.
    let applicable: Vec<(&anvil_policy::exceptions::PolicyException, ExceptionVerdict)> = store
        .active_exceptions_at(now)
        .into_iter()
        .map(|ex| (ex, verify_exception_at(ex, now)))
        .filter(|(_, verdict)| verdict.applies())
        .collect();
    if applicable.is_empty() {
        return not_covered;
    }
    warnings
        .iter()
        .map(|warning| {
            let mut downgrade: Option<ExceptionDisposition> = None;
            for (exception, verdict) in &applicable {
                if !exception.covers_finding(
                    &warning.id,
                    &warning.location.file,
                    warning.fingerprint.as_deref(),
                ) {
                    continue;
                }
                if verdict.is_downgrade() {
                    downgrade.get_or_insert_with(|| ExceptionDisposition::SuppressedDowngraded {
                        exception_id: exception.id.clone(),
                    });
                } else {
                    return ExceptionDisposition::Suppressed {
                        exception_id: exception.id.clone(),
                    };
                }
            }
            downgrade.unwrap_or(ExceptionDisposition::NotCovered)
        })
        .collect()
}

/// Outcome of reading the exception store from a commit tree.
enum StoreFromTip {
    /// The tip's tree has no `anvil/exceptions/store.json` — an
    /// honest empty store, not an error.
    Absent,
    /// The blob exists but could not be used (git failure, oversized,
    /// malformed JSON). Fail-safe: no exceptions apply.
    Unreadable(String),
    /// The committed store, parsed.
    Loaded(ExceptionStore),
}

/// ADR-100: read `anvil/exceptions/store.json` from `tip`'s tree via
/// the batched blob reader (inherits its sha/path hygiene). The size
/// is checked with `git cat-file -s` **before** the content read, so
/// an oversized committed blob is refused without ever buffering it —
/// genuine parity with the filesystem loader's `Read::take` bound
/// rather than a buffer-then-reject (2026-07-04 council).
fn load_committed_store(repo_root: &Path, tip: &str) -> StoreFromTip {
    match committed_store_size(repo_root, tip) {
        StoreSize::Absent => return StoreFromTip::Absent,
        StoreSize::Oversized(size) => {
            return StoreFromTip::Unreadable(format!(
                "store blob is {size} bytes; refusing past the {} byte bound",
                anvil_policy::exceptions::MAX_STORE_BYTES,
            ));
        }
        StoreSize::Unknown(detail) => return StoreFromTip::Unreadable(detail),
        StoreSize::Within => {}
    }
    let blobs = match read_commit_blobs_batch(repo_root, tip, &[EXCEPTIONS_FILE]) {
        Ok(blobs) => blobs,
        Err(reason) => return StoreFromTip::Unreadable(format!("{reason:?}")),
    };
    let Some(Some(bytes)) = blobs.into_iter().next() else {
        return StoreFromTip::Absent;
    };
    match serde_json::from_slice::<ExceptionStore>(&bytes) {
        Ok(store) => StoreFromTip::Loaded(store),
        Err(e) => StoreFromTip::Unreadable(format!("parse error: {e}")),
    }
}

/// Outcome of the pre-read size probe.
enum StoreSize {
    Absent,
    Within,
    Oversized(u64),
    Unknown(String),
}

/// `git cat-file -s <tip>:anvil/exceptions/store.json` — object size
/// without reading the body. Only a path-missing-in-tree error maps to
/// `Absent` ("does not exist in", "exists on disk, but not in"); an
/// unresolvable TIP ("Not a valid object name") or any other failure
/// is `Unknown` — fail-safe upstream (no exceptions apply) with a
/// traced detail, never silently treated as an honest empty store.
fn committed_store_size(repo_root: &Path, tip: &str) -> StoreSize {
    if !is_hex_sha(tip) || is_zero_sha(tip) {
        return StoreSize::Unknown("tip is not a commit sha".to_string());
    }
    let output = match Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["cat-file", "-s", &format!("{tip}:{EXCEPTIONS_FILE}")])
        .output()
    {
        Ok(output) => output,
        Err(e) => return StoreSize::Unknown(format!("git spawn failed: {e}")),
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("does not exist in") || stderr.contains("exists on disk, but not in") {
            return StoreSize::Absent;
        }
        return StoreSize::Unknown(format!("cat-file -s failed: {}", stderr.trim()));
    }
    match String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
    {
        Ok(size) if size > anvil_policy::exceptions::MAX_STORE_BYTES => StoreSize::Oversized(size),
        Ok(_) => StoreSize::Within,
        Err(e) => StoreSize::Unknown(format!("unparseable cat-file -s output: {e}")),
    }
}

/// `git diff-tree --no-commit-id --name-only -z -r --root
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
/// - `-z` emits NUL-delimited literal path bytes. Without it, git
///   C-quotes paths that contain quotes, tabs, newlines, or other
///   special characters (`"foo\"bar.ts"`), so the scannable-extension
///   filter never matches and L4 admits the commit unexamined
///   (Clawpatch fnd_sig-feat-cli-command-79ebbc42f6-_ddb9293a0c).
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
            "-z",
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
    Ok(split_nul_paths(&output.stdout))
}

/// Split git `-z` path output on NUL. Empty segments (trailing NUL) are
/// dropped. Paths are decoded lossily to UTF-8 so rare non-UTF-8 path
/// bytes still surface rather than aborting the whole commit list.
fn split_nul_paths(stdout: &[u8]) -> Vec<String> {
    stdout
        .split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
        .collect()
}

/// MLP2-068: batched blob fetch. Resolves each path to a blob object
/// ID via `git ls-tree -z` (argv-safe for special characters, including
/// `:`, quotes, tabs, and newlines), then spawns `git cat-file --batch`
/// once with those OIDs and returns a vec aligned with `paths`.
/// Returns `None` for tree misses and non-blob entries, and `Err` for
/// batch-level I/O/tooling failures.
///
/// Pre-MLP2-068, the engine spawned one `git show` per scannable file
/// — a 200-file commit paid ~1–3 s on process startup alone, most of
/// `PRE_PUSH_BUDGET`. This helper amortises the spawn cost across the
/// whole batch.
///
/// Path encoding notes (Clawpatch fnd_sig-feat-cli-command-79ebbc42f6-_ddb9293a0c):
/// - Line-delimited `<sha>:<path>` cat-file requests cannot encode paths
///   that themselves contain newlines, and `<sha>:weird:path.ts` is an
///   ambiguous revspec (Council #C-016G). Feeding blob OIDs avoids both
///   classes of failure while preserving the single-spawn batch.
/// - Non-hex / zero SHA → `Err` before git is invoked.
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

    let oids = resolve_blob_oids(repo_root, sha, paths)?;
    let mut queries: Vec<String> = Vec::with_capacity(paths.len());
    let mut had_query: Vec<bool> = Vec::with_capacity(paths.len());
    for oid in &oids {
        match oid {
            Some(oid) => {
                had_query.push(true);
                queries.push(format!("{oid}\n"));
            }
            None => had_query.push(false),
        }
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

    // `parsed` carries one entry per OID query in the order written to
    // stdin. Drain into slots that produced queries; missing paths stay
    // the pre-set `None`.
    let mut parsed_iter = parsed.into_iter();
    for (slot, queried) in results.iter_mut().zip(had_query.iter()) {
        if *queried {
            *slot = parsed_iter.next().flatten();
        }
    }
    Ok(results)
}

/// Resolve each path to its blob OID under `sha` via `git ls-tree -z`.
/// Paths travel as argv after `--`, so quotes, tabs, newlines, and
/// colons remain unambiguous (unlike `<rev>:<path>` revspecs). Output
/// order from git is tree order, not request order — match by path
/// bytes. Chunks keep argv under platform limits.
fn resolve_blob_oids(
    repo_root: &Path,
    sha: &str,
    paths: &[&str],
) -> Result<Vec<Option<String>>, EngineUnavailableReason> {
    // 64 paths per invocation is well under ARG_MAX even for long paths.
    const CHUNK: usize = 64;
    // Path bytes → oid. Keys are the raw path field from ls-tree so
    // lookups match git's path encoding. Callers pass UTF-8 `&str`
    // paths (from `list_commit_files`); non-UTF-8 path bytes cannot
    // round-trip through that surface and will miss here, which the
    // engine treats as an unreadable scannable path (fail-closed).
    let mut by_path: HashMap<Vec<u8>, String> = HashMap::with_capacity(paths.len());
    for chunk in paths.chunks(CHUNK) {
        let mut cmd = Command::new("git");
        cmd.arg("-C")
            .arg(repo_root)
            .args(["ls-tree", "-z", sha, "--"]);
        for path in chunk {
            cmd.arg(path);
        }
        let output = cmd
            .stderr(Stdio::piped())
            .output()
            .map_err(|err| match err.kind() {
                std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied => {
                    EngineUnavailableReason::BinaryMissing
                }
                _ => EngineUnavailableReason::IoError,
            })?;
        if !output.status.success() {
            log_git_failure("ls-tree", sha, &output.stderr);
            return Err(EngineUnavailableReason::IoError);
        }
        parse_ls_tree_z(&output.stdout, &mut by_path);
    }
    Ok(paths
        .iter()
        .map(|p| by_path.get(p.as_bytes()).cloned())
        .collect())
}

/// Parse `git ls-tree -z` records of the form
/// `<mode> SP <type> SP <object> TAB <file>\0` into `out`. Non-blob
/// entries (trees, commits/gitlinks) are skipped so callers never
/// treat a tree as file content.
fn parse_ls_tree_z(stdout: &[u8], out: &mut HashMap<Vec<u8>, String>) {
    for entry in stdout.split(|&b| b == 0) {
        if entry.is_empty() {
            continue;
        }
        let Some(tab) = entry.iter().position(|&b| b == b'\t') else {
            continue;
        };
        let meta = &entry[..tab];
        let path = &entry[tab + 1..];
        let Ok(meta_str) = std::str::from_utf8(meta) else {
            continue;
        };
        let mut parts = meta_str.split(' ');
        let _mode = parts.next();
        let Some(obj_type) = parts.next() else {
            continue;
        };
        let Some(oid) = parts.next() else {
            continue;
        };
        if parts.next().is_some() {
            // Unexpected extra fields — skip rather than mis-parse.
            continue;
        }
        if obj_type != "blob" {
            continue;
        }
        out.insert(path.to_vec(), oid.to_owned());
    }
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

    /// Clawpatch fnd_sig-feat-cli-command-79ebbc42f6-_ddb9293a0c:
    /// without `-z`, git C-quotes paths containing quotes/tabs/newlines so
    /// the extension filter never sees a scannable suffix. List must return
    /// the literal path bytes.
    #[cfg(unix)]
    #[test]
    fn list_commit_files_returns_literal_special_character_paths() {
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
        let quote_path = "foo\"bar.ts";
        let tab_path = "tab\tname.ts";
        let newline_path = "new\nline.ts";
        std::fs::write(root.join(quote_path), "export const q = 1;\n").unwrap();
        std::fs::write(root.join(tab_path), "export const t = 1;\n").unwrap();
        std::fs::write(root.join(newline_path), "export const n = 1;\n").unwrap();
        run(&["add", "-A"]);
        run(&["commit", "-q", "-m", "special names"]);
        let sha = String::from_utf8(run(&["rev-parse", "HEAD"]).stdout)
            .unwrap()
            .trim()
            .to_string();
        let files = list_commit_files(&root, &sha).expect("diff-tree -z succeeded");
        assert!(
            files.iter().any(|p| p == quote_path),
            "literal quote path missing from {files:?}",
        );
        assert!(
            files.iter().any(|p| p == tab_path),
            "literal tab path missing from {files:?}",
        );
        assert!(
            files.iter().any(|p| p == newline_path),
            "literal newline path missing from {files:?}",
        );
        // Must never surface git's C-quoted form, which ends in `"` not `.ts`.
        assert!(
            files.iter().all(|p| !p.starts_with('"')),
            "quoted path form leaked into listing: {files:?}",
        );
    }

    /// End-to-end: a scannable source file whose name forces git to C-quote
    /// under non-`-z` output must still be inspected. Returning `Allow` would
    /// reintroduce the L4 bypass.
    #[cfg(unix)]
    #[test]
    fn validate_commit_blocks_antipattern_in_special_character_paths() {
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
        let content = "/* eslint-disable */\nimport { x } from './m';\n";
        let quote_path = "leak\"q.ts";
        let newline_path = "leak\nn.ts";
        std::fs::write(root.join(quote_path), content).unwrap();
        std::fs::write(root.join(newline_path), content).unwrap();
        run(&["add", "-A"]);
        run(&["commit", "-q", "-m", "special antipattern"]);
        let sha = String::from_utf8(run(&["rev-parse", "HEAD"]).stdout)
            .unwrap()
            .trim()
            .to_string();
        let verdict = validate_commit(&root, &sha, None);
        let ValidationVerdict::Block { diagnostics } = verdict else {
            panic!("expected Block for special-character paths, got {verdict:?}");
        };
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "AP-001"),
            "expected AP-001 in {diagnostics:?}",
        );
    }

    /// Newline-bearing paths cannot travel through line-delimited
    /// `<sha>:<path>` cat-file requests. OID-based batch must still return
    /// the body, and colon-bearing paths in-tree must resolve (no longer
    /// refused solely because of the colon).
    #[cfg(unix)]
    #[test]
    fn read_commit_blobs_batch_reads_special_character_paths() {
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
        let quote_path = "q\"uote.ts";
        let newline_path = "n\nl.ts";
        let colon_path = "weird:path.ts";
        std::fs::write(root.join(quote_path), b"quote-body\n").unwrap();
        std::fs::write(root.join(newline_path), b"newline-body\n").unwrap();
        std::fs::write(root.join(colon_path), b"colon-body\n").unwrap();
        run(&["add", "-A"]);
        run(&["commit", "-q", "-m", "special blobs"]);
        let sha = String::from_utf8(run(&["rev-parse", "HEAD"]).stdout)
            .unwrap()
            .trim()
            .to_string();
        let bodies = read_commit_blobs_batch(
            &root,
            &sha,
            &[quote_path, newline_path, colon_path, "missing.ts"],
        )
        .expect("batch succeeded");
        assert_eq!(bodies.len(), 4);
        assert_eq!(bodies[0].as_deref(), Some(b"quote-body\n".as_ref()));
        assert_eq!(bodies[1].as_deref(), Some(b"newline-body\n".as_ref()));
        assert_eq!(bodies[2].as_deref(), Some(b"colon-body\n".as_ref()));
        assert!(bodies[3].is_none(), "missing path stays None");
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

    /// Council #C-016G (updated): colon-bearing pathnames used to be
    /// refused because `<rev>:<path>` is ambiguous. The batch helper now
    /// resolves via `ls-tree` + blob OIDs, so a missing colon path still
    /// yields `None` (tree miss) while other valid entries resolve. A
    /// present colon path is covered by
    /// `read_commit_blobs_batch_reads_special_character_paths`.
    #[test]
    fn read_commit_blobs_batch_returns_none_for_colon_path() {
        let (_tmp, root, sha) = commit_with_file("body\n", "f.txt");
        let bodies = read_commit_blobs_batch(&root, &sha, &["weird:path.ts", "f.txt"]).unwrap();
        assert_eq!(bodies.len(), 2);
        assert!(bodies[0].is_none(), "missing colon path must be None");
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
        let verdict = validate_commit(&root, &sha, None);
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
        let verdict = validate_commit(tmp.path(), &"a".repeat(40), None);
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
        let verdict = validate_commit_with_tempdir(&root, &sha, None, || {
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

        let verdict = validate_commit_with_tempdir(&root, &sha, None, || {
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
        let verdict = validate_commit(tmp.path(), &"0".repeat(40), None);
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
        let verdict = validate_commit(&root, &sha, None);
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
        let verdict = validate_commit(&root, &sha, None);
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
        let verdict = validate_commit(&root, &sha, None);
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

    // --- EXCEPT-006: tracked policy exceptions at the L4 gate ---

    use anvil_policy::exceptions::{ExceptionRevocation, PolicyException, WriteOutcome};
    use chrono::Duration;

    /// Content that trips the `AP-001` broad-`eslint-disable`
    /// antipattern — the same fixture
    /// `validate_commit_blocks_on_known_antipattern` pins.
    const AP_001_CONTENT: &str = "/* eslint-disable */\nimport { x } from './m';\n";

    fn exception_for(policy_id: &str, file_pattern: &str) -> PolicyException {
        PolicyException {
            schema_version: String::new(),
            id: String::new(),
            policy_id: policy_id.to_string(),
            file_pattern: file_pattern.to_string(),
            finding_hash: None,
            reason: "test grant".to_string(),
            owner: Some("team-platform".to_string()),
            created_by: Some("alice@example.test".to_string()),
            created_at: Utc::now(),
            expires_at: None,
            revoked: None,
        }
    }

    fn save_exceptions(root: &Path, exceptions: Vec<PolicyException>) {
        let mut store = ExceptionStore::empty();
        for ex in exceptions {
            store.add(ex);
        }
        let outcome = store.save(root).expect("write tracked exception store");
        assert_eq!(outcome, WriteOutcome::Written);
    }

    /// Write the store AND commit it, returning the store commit's sha
    /// — the ADR-100 suppression-authority tip. Mirrors the real
    /// workflow: the grant commit lands after the finding commit and
    /// covers it from the tip.
    fn commit_exceptions(root: &Path, exceptions: Vec<PolicyException>) -> String {
        save_exceptions(root, exceptions);
        git_in(root, &["add", "anvil/exceptions"]);
        git_in(root, &["commit", "-q", "-m", "grant exceptions"]);
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["rev-parse", "HEAD"])
            .output()
            .expect("git available");
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    }

    /// EXCEPT-006: a valid, attributed exception covering the rule and
    /// file suppresses the finding — the gate recomputes to `Allow`.
    #[test]
    fn active_exception_suppresses_finding_and_allows() {
        let (_tmp, root, sha) = commit_with_file(AP_001_CONTENT, "src/leak.ts");
        let tip = commit_exceptions(&root, vec![exception_for("AP-001", "src/**")]);
        let verdict = validate_commit(&root, &sha, Some(&tip));
        assert_eq!(verdict, ValidationVerdict::Allow);
    }

    /// EXCEPT-006 / ADR-073: an unattributed (v0-shape) grant is never
    /// silently honoured — the finding survives at `Warn`, annotated
    /// with the exception id.
    #[test]
    fn unattributed_exception_downgrades_finding_with_annotation() {
        let (_tmp, root, sha) = commit_with_file(AP_001_CONTENT, "src/leak.ts");
        let mut ex = exception_for("AP-001", "src/**");
        ex.owner = None;
        ex.created_by = None;
        let tip = commit_exceptions(&root, vec![ex]);
        let verdict = validate_commit(&root, &sha, Some(&tip));
        let ValidationVerdict::Block { diagnostics } = verdict else {
            panic!("expected Block carrier, got {verdict:?}");
        };
        let ap_001 = diagnostics
            .iter()
            .find(|d| d.rule_id == "AP-001")
            .expect("AP-001 present");
        assert_eq!(ap_001.severity, Severity::Warn);
        assert!(
            ap_001.message.contains("unattributed exception"),
            "downgrade annotation missing: {}",
            ap_001.message,
        );
    }

    /// EXCEPT-006: an expired grant does not apply — the finding
    /// stands unannotated.
    #[test]
    fn expired_exception_leaves_finding_standing() {
        let (_tmp, root, sha) = commit_with_file(AP_001_CONTENT, "src/leak.ts");
        let mut ex = exception_for("AP-001", "src/**");
        ex.expires_at = Some(Utc::now() - Duration::days(1));
        let tip = commit_exceptions(&root, vec![ex]);
        let verdict = validate_commit(&root, &sha, Some(&tip));
        let ValidationVerdict::Block { diagnostics } = verdict else {
            panic!("expected Block, got {verdict:?}");
        };
        let ap_001 = diagnostics
            .iter()
            .find(|d| d.rule_id == "AP-001")
            .expect("AP-001 must stand");
        assert!(
            !ap_001.message.contains("exception"),
            "expired grant must not annotate: {}",
            ap_001.message,
        );
    }

    /// EXCEPT-006: a revoked grant does not apply.
    #[test]
    fn revoked_exception_leaves_finding_standing() {
        let (_tmp, root, sha) = commit_with_file(AP_001_CONTENT, "src/leak.ts");
        let mut ex = exception_for("AP-001", "src/**");
        ex.revoked = Some(ExceptionRevocation {
            revoked_at: Utc::now(),
            revoked_by: "bob".to_string(),
            reason: "no longer needed".to_string(),
        });
        let tip = commit_exceptions(&root, vec![ex]);
        let verdict = validate_commit(&root, &sha, Some(&tip));
        assert!(
            matches!(verdict, ValidationVerdict::Block { .. }),
            "revoked grant must not suppress, got {verdict:?}",
        );
    }

    /// ADR-100 (2026-07-04 council PoC): an uncommitted, worktree-only
    /// grant must NOT satisfy the gate — suppression authority must be
    /// committed in the tip's tree. This is the zero-trace self-grant
    /// regression test.
    #[test]
    fn uncommitted_worktree_grant_does_not_apply() {
        let (_tmp, root, sha) = commit_with_file(AP_001_CONTENT, "src/leak.ts");
        // Worktree-only store: written, never committed.
        save_exceptions(&root, vec![exception_for("AP-001", "src/**")]);
        let verdict = validate_commit(&root, &sha, Some(&sha));
        assert!(
            matches!(verdict, ValidationVerdict::Block { .. }),
            "uncommitted grant must not suppress, got {verdict:?}",
        );
    }

    /// ADR-100: a committed SYMLINK at the store path cannot smuggle
    /// content — git stores the target path string as the blob, which
    /// fails to parse → fail-safe, findings stand.
    #[cfg(unix)]
    #[test]
    fn committed_symlink_store_does_not_apply() {
        let (_tmp, root, sha) = commit_with_file(AP_001_CONTENT, "src/leak.ts");
        let outside = root.join("outside.json");
        save_exceptions(&root, vec![exception_for("AP-001", "src/**")]);
        std::fs::rename(root.join("anvil/exceptions/store.json"), &outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("anvil/exceptions/store.json")).unwrap();
        git_in(&root, &["add", "anvil/exceptions"]);
        git_in(&root, &["commit", "-q", "-m", "symlinked store"]);
        let tip = String::from_utf8(
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
        let verdict = validate_commit(&root, &sha, Some(&tip));
        assert!(
            matches!(verdict, ValidationVerdict::Block { .. }),
            "symlinked committed store must not apply, got {verdict:?}",
        );
    }

    /// ADR-100: no tip means no exceptions — findings stand.
    #[test]
    fn missing_tip_applies_no_exceptions() {
        let (_tmp, root, sha) = commit_with_file(AP_001_CONTENT, "src/leak.ts");
        let _tip = commit_exceptions(&root, vec![exception_for("AP-001", "src/**")]);
        let verdict = validate_commit(&root, &sha, None);
        assert!(
            matches!(verdict, ValidationVerdict::Block { .. }),
            "tip-less validation must apply no exceptions, got {verdict:?}",
        );
    }

    /// EXCEPT-006 council: attributed grant wins over an unattributed
    /// one covering the same finding — clean suppression, independent
    /// of store order. Unattributed first…
    #[test]
    fn attributed_grant_beats_unattributed_listed_after_it() {
        let (_tmp, root, sha) = commit_with_file(AP_001_CONTENT, "src/leak.ts");
        let mut unattributed = exception_for("AP-001", "src/**");
        unattributed.owner = None;
        unattributed.created_by = None;
        let tip = commit_exceptions(&root, vec![unattributed, exception_for("AP-001", "src/**")]);
        let verdict = validate_commit(&root, &sha, Some(&tip));
        assert_eq!(
            verdict,
            ValidationVerdict::Allow,
            "attributed grant must clean-suppress regardless of store order",
        );
    }

    /// …and attributed first (both orders pinned so a future
    /// first-match refactor cannot silently regress the precedence).
    #[test]
    fn attributed_grant_beats_unattributed_listed_before_it() {
        let (_tmp, root, sha) = commit_with_file(AP_001_CONTENT, "src/leak.ts");
        let mut unattributed = exception_for("AP-001", "src/**");
        unattributed.owner = None;
        unattributed.created_by = None;
        let tip = commit_exceptions(&root, vec![exception_for("AP-001", "src/**"), unattributed]);
        let verdict = validate_commit(&root, &sha, Some(&tip));
        assert_eq!(verdict, ValidationVerdict::Allow);
    }

    /// EXCEPT-006 council: a malformed store fails safe — findings
    /// stand, the gate never silently admits on a bookkeeping error.
    #[test]
    fn malformed_exception_store_fails_safe_findings_stand() {
        let (_tmp, root, sha) = commit_with_file(AP_001_CONTENT, "src/leak.ts");
        std::fs::create_dir_all(root.join("anvil/exceptions")).unwrap();
        std::fs::write(root.join("anvil/exceptions/store.json"), "{not json").unwrap();
        git_in(&root, &["add", "anvil/exceptions"]);
        git_in(&root, &["commit", "-q", "-m", "malformed store"]);
        let tip = String::from_utf8(
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
        let verdict = validate_commit(&root, &sha, Some(&tip));
        assert!(
            matches!(verdict, ValidationVerdict::Block { .. }),
            "malformed store must leave findings standing, got {verdict:?}",
        );
    }

    /// EXCEPT-006: exception scope is honoured — a grant for another
    /// directory does not cover this finding.
    #[test]
    fn out_of_scope_exception_leaves_finding_standing() {
        let (_tmp, root, sha) = commit_with_file(AP_001_CONTENT, "src/leak.ts");
        let tip = commit_exceptions(&root, vec![exception_for("AP-001", "vendor/**")]);
        let verdict = validate_commit(&root, &sha, Some(&tip));
        assert!(
            matches!(verdict, ValidationVerdict::Block { .. }),
            "out-of-scope grant must not suppress, got {verdict:?}",
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
            exceptions_tip_sha: None,
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
        let verdict = validate_commit(&root, &sha, None);
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
