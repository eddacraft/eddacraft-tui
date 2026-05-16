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
//! via `git diff-tree` + `git show <sha>:<path>`, hands the resulting
//! file paths to [`anvil_checks::antipattern::run_antipattern_check`],
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
    // Council #C-016C CRITICAL: a zero SHA passes `is_hex_sha` but is
    // never a real commit. Refusing here keeps `git diff-tree` from
    // being asked to resolve an impossible object and prevents
    // `l4-validate --range 000...0` from reaching the engine.
    if is_zero_sha(commit_sha) {
        return ValidationVerdict::EngineUnavailable {
            reason: EngineUnavailableReason::BinaryMissing,
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
    let Some(paths) = list_commit_files(repo_root, commit_sha) else {
        // git was unavailable or the SHA didn't resolve — surface as
        // engine-unavailable rather than silently admitting. The
        // pre-push hook collapses this to the legacy
        // `InternalError { TimedOut }` line per ADR-038 §D-6.
        return ValidationVerdict::EngineUnavailable {
            reason: EngineUnavailableReason::BinaryMissing,
        };
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
    let Ok(tmp) = tempfile::TempDir::new() else {
        // Council #C-016D MAJOR: a `/tmp` allocation failure is an
        // infrastructure outage, not a time-budget overrun. Mapping
        // it to `Timeout` would mislead observability tooling.
        // `BinaryMissing` is the catch-all infrastructure-unavailable
        // signal until the trait crate gains a dedicated `IoError`
        // variant.
        return ValidationVerdict::EngineUnavailable {
            reason: EngineUnavailableReason::BinaryMissing,
        };
    };
    let workspace_root = tmp.path().to_path_buf();
    let mut materialised: Vec<String> = Vec::with_capacity(scannable.len());
    for path in &scannable {
        let Some(blob) = read_commit_blob(repo_root, commit_sha, path) else {
            continue;
        };
        let target = workspace_root.join(path);
        if let Some(parent) = target.parent()
            && std::fs::create_dir_all(parent).is_err()
        {
            continue;
        }
        if std::fs::write(&target, blob).is_err() {
            continue;
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
            reason: EngineUnavailableReason::BinaryMissing,
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
fn list_commit_files(repo_root: &Path, sha: &str) -> Option<Vec<String>> {
    if !is_hex_sha(sha) || is_zero_sha(sha) {
        return None;
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
        .ok()
        .inspect(|o| stderr_buf.extend_from_slice(&o.stderr))?;
    if !output.status.success() {
        log_git_failure("diff-tree", sha, &stderr_buf);
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
    )
}

/// `git show <sha>:<path>` — returns the blob bytes at `path` inside
/// the tree of `sha`. Returns `None` when git fails or the path is not
/// in the tree (e.g. deletion). git stderr is forwarded to
/// `tracing::debug!` so production incident debugging can distinguish
/// "git not on PATH" from "object missing from pack".
fn read_commit_blob(repo_root: &Path, sha: &str, path: &str) -> Option<Vec<u8>> {
    if !is_hex_sha(sha) || is_zero_sha(sha) {
        return None;
    }
    // git's `<rev>:<path>` revspec splits on the first `:`. Filenames
    // legitimately containing a colon would mis-parse; skip them
    // rather than feed git an ambiguous spec. Council #C-016G.
    if path.contains(':') {
        return None;
    }
    let spec = format!("{sha}:{path}");
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["show", spec.as_str()])
        .stderr(Stdio::piped())
        .output()
        .ok()?;
    if !output.status.success() {
        log_git_failure("show", &spec, &output.stderr);
        return None;
    }
    Some(output.stdout)
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
        assert!(list_commit_files(&root, "HEAD").is_none());
        assert!(list_commit_files(&root, "--all").is_none());
    }

    /// Council #C-016C: zero SHA is hex-shaped but never a real
    /// commit. `list_commit_files` must refuse before invoking git.
    #[test]
    fn list_commit_files_refuses_zero_sha() {
        let (_tmp, root, _sha) = commit_with_file("x", "f.txt");
        assert!(list_commit_files(&root, &"0".repeat(40)).is_none());
    }

    /// Council #C-016G: filenames containing a colon would mis-parse
    /// `<rev>:<path>`. The engine must refuse to construct the
    /// revspec rather than feed git an ambiguous string.
    #[test]
    fn read_commit_blob_refuses_colon_path() {
        let (_tmp, root, sha) = commit_with_file("body\n", "f.txt");
        assert!(read_commit_blob(&root, &sha, "weird:path.ts").is_none());
    }

    /// `read_commit_blob` round-trips a known body through git.
    #[test]
    fn read_commit_blob_returns_file_bytes() {
        let (_tmp, root, sha) = commit_with_file("body\n", "f.txt");
        let bytes = read_commit_blob(&root, &sha, "f.txt").expect("git show succeeded");
        assert_eq!(bytes, b"body\n");
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

    /// MLP2-016 reopened: an unscannable repo path collapses to
    /// `EngineUnavailable { BinaryMissing }`. Pre-push routes that to
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
                reason: EngineUnavailableReason::BinaryMissing,
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
                reason: EngineUnavailableReason::BinaryMissing,
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
}
