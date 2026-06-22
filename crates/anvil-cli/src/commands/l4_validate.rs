//! MLP2-046: dedicated `anvil l4-validate <commit-range>` subcommand.
//!
//! The pre-push hook today (`anvil hook pre-push`) already executes the
//! [`anvil_l4`] pipeline against unwitnessed commits as part of git's
//! pre-push contract — reading `<local-ref> <local-sha> <remote-ref>
//! <remote-sha>` lines from stdin. The CI / Marketplace lanes do not
//! sit inside git's hook surface, so they need a binary they can invoke
//! with an explicit commit range and inspect the exit code without
//! teaching CI to forge git's pre-push stdin shape.
//!
//! This subcommand is that binary:
//!
//! ```text
//! anvil l4-validate <REMOTE_SHA>..<LOCAL_SHA> [--branch=<name>]
//! anvil l4-validate <LOCAL_SHA>              [--branch=<name>]   # ancestry walk
//! ```
//!
//! Semantics mirror the pre-push hook 1:1:
//!
//! 1. Walk the commit range with `git rev-list`.
//! 2. Resolve the branch's [`anvil_l4::Policy`] rule.
//! 3. For each unwitnessed commit, decide via [`anvil_l4::Policy::resolve`]
//!    and run [`anvil_l4::validate_at_l4`] against the default
//!    [`crate::l4_engine::CommitAntipatternEngine`] — the real
//!    `anvil-checks` antipattern pipeline run against the commit's
//!    tree via git plumbing. Tests substitute fixture engines via
//!    [`run_with_engine`].
//! 4. Exit non-zero when any commit blocks; otherwise exit zero.
//!
//! The template's `anvil hook pre-push` invocation can swap to
//! `anvil l4-validate` in a follow-up patch — the binary surface is
//! ready today, but the swap is gated on the template-render path
//! catching up (separate APS).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anvil_config::ConfigFormat;
use anvil_hook::is_hex_sha;
use anvil_l4::{
    BlockKind, CommitDecision, EngineUnavailableReason, OnWarn, Policy, Severity,
    ValidationDiagnostic, ValidationEngine, ValidationVerdict, request_for,
};
use anvil_witness::{verify_chain_dag, witness_paths};
use anyhow::{Context, Result};
use clap::Args;

use crate::GlobalArgs;
use crate::l4_engine::default_engine;

/// Exit code returned when `l4-validate` blocks the operation. Matches
/// the pre-push hook's `EXIT_GATE_FAIL`-equivalent exit.
const EXIT_BLOCK: u8 = 2;
/// Exit code returned when `l4-validate` could not run the rule
/// engine (engine unavailable on every unwitnessed commit). Matches
/// the pre-push hook's `InternalError` exit semantics — the operator
/// gets a single advisory line and a non-zero exit so CI knows the
/// run wasn't conclusive.
const EXIT_PENDING: u8 = 3;

#[derive(Debug, Args)]
pub struct L4ValidateArgs {
    /// Commit range to validate. Accepts `<base>..<head>` or a bare
    /// `<head>` SHA. `<head>`-only walks the full ancestry — matches
    /// `git rev-list <head>` shape.
    #[arg(value_name = "RANGE")]
    range: String,
    /// Branch name the policy resolver matches against. Defaults to
    /// the current branch resolved via `git symbolic-ref --short
    /// HEAD`. Operators on detached HEAD or running from a script
    /// where HEAD does not name a branch pass `--branch=` explicitly.
    #[arg(long)]
    branch: Option<String>,
    /// Repo root override. Defaults to the current working directory.
    #[arg(long, value_name = "PATH")]
    repo: Option<PathBuf>,
}

pub fn run(args: &L4ValidateArgs, global: &GlobalArgs) -> Result<()> {
    let engine = default_engine();
    let outcome = run_with_engine(args, global, engine.as_ref())?;
    // Council #1 / MAJOR-2: a block exit MUST surface the per-rule
    // diagnostics on stderr so CI logs explain which rule refused
    // which commit. Render every blocking commit, then exit.
    let mut stderr = std::io::stderr().lock();
    for outcome in &outcome.commits {
        if let CommitVerdict::Block { diagnostics } = &outcome.verdict {
            let _ = render_diagnostics_to(&mut stderr, &outcome.commit_sha, diagnostics);
        }
    }
    if let Some(code) = outcome.exit_code {
        std::process::exit(code.into());
    }
    Ok(())
}

/// MLP2-046: outcome of one `l4-validate` invocation. Returned by
/// [`run_with_engine`] so tests can drive the binary's decision logic
/// without spawning a subprocess. The `exit_code` field collapses the
/// closed-set policy decisions onto the binary's exit-code surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct L4ValidateOutcome {
    /// One verdict per commit walked, in `git rev-list` order
    /// (`new → old`). An empty vector means the range had no
    /// commits to validate (already-pushed range, empty ancestry).
    pub commits: Vec<CommitOutcome>,
    /// `None` on success (no blocks, no engine-unavailable verdicts).
    /// `Some(EXIT_BLOCK)` on any hard refusal; `Some(EXIT_PENDING)`
    /// when every unwitnessed verdict was engine-unavailable and no
    /// commit blocked outright.
    pub exit_code: Option<u8>,
}

/// Per-commit outcome. Carries the originating SHA + the verdict
/// emitted by the policy or the [`ValidationEngine`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitOutcome {
    pub commit_sha: String,
    pub verdict: CommitVerdict,
}

/// MLP2-046 closed-set verdict. Mirrors the pre-push hook's
/// decision tree minus the witness-emit side effect (l4-validate
/// does not write to `refs/notes/anvil-l4` — MLP2-017 owns that).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitVerdict {
    /// Commit has an L3 witness OR the engine ran and allowed it.
    Allow,
    /// Engine ran and refused. `diagnostics` carries the per-rule
    /// detail lines.
    Block {
        diagnostics: Vec<ValidationDiagnostic>,
    },
    /// Branch policy says `Block(UnwitnessedCommit)` outright (no
    /// engine pass).
    UnwitnessedBlock,
    /// Engine declined to run; the binary degrades to a pending
    /// surface (exit 3) on the way out.
    EnginePending { reason: EngineUnavailableReason },
}

/// MLP2-046 production entry that takes an injectable
/// [`ValidationEngine`]. Tests substitute fixtures here; the public
/// [`run`] wraps this with the production default supplied by
/// [`crate::l4_engine::default_engine`].
#[allow(clippy::too_many_lines)]
pub fn run_with_engine(
    args: &L4ValidateArgs,
    _global: &GlobalArgs,
    engine: &dyn ValidationEngine,
) -> Result<L4ValidateOutcome> {
    let repo_root = match &args.repo {
        Some(p) => p.clone(),
        None => std::env::current_dir().context("resolve repo root")?,
    };

    // No-op when the project hasn't opted into anvil: matches the
    // pre-push hook's "Serena rule" exit-zero behaviour.
    if read_project_id(&repo_root).is_none() {
        return Ok(L4ValidateOutcome {
            commits: vec![],
            exit_code: None,
        });
    }

    let Some(policy) = load_policy(&repo_root)? else {
        // No policy file -> nothing to validate.
        return Ok(L4ValidateOutcome {
            commits: vec![],
            exit_code: None,
        });
    };

    let branch = match args.branch.clone() {
        Some(b) => b,
        None => current_branch(&repo_root).unwrap_or_else(|| "HEAD".to_owned()),
    };

    let rule = policy
        .resolve(&branch)
        .with_context(|| format!("resolve policy for branch {branch}"))?;
    let Some(rule) = rule else {
        // Council #4 / MINOR: no rule matches this branch -> admit
        // silently with an empty `commits` vec. Earlier the synthetic
        // outcome placed the branch name in `commit_sha`, which is a
        // type-system lie (the field is a SHA) — a caller piping the
        // value to git would get a ref-resolution error. The
        // no-project-id and no-policy-file branches above already
        // return empty; keep this path consistent.
        return Ok(L4ValidateOutcome {
            commits: vec![],
            exit_code: None,
        });
    };

    let commits = resolve_range(&repo_root, &args.range)
        .with_context(|| format!("resolve range {range}", range = args.range))?;

    // MLP2-062: verify the active + archive witness chain before
    // treating any `commit_sha` line as L3 evidence. Pre-fix the CI /
    // Marketplace surface harvested every recorded `commit_sha` from
    // `anvil/witness/*.ndjson` without checking integrity, so a
    // tampered or forged record could mark an unwitnessed commit as
    // witnessed and silently admit it. Refuse with a non-zero exit
    // when the chain fails to verify; CI carries the underlying
    // verifier error in stderr.
    verify_witness_chain(&repo_root)?;
    let witnessed = collect_witnessed_shas(&repo_root);

    let mut outcomes = Vec::with_capacity(commits.len());
    let mut block = false;
    let mut all_engine_unavailable_when_needed = true;
    let mut needed_engine_at_least_once = false;

    for commit in commits {
        let has_witness = witnessed.contains(&commit);
        let decision = rule.decide_commit(has_witness);
        let verdict = match decision {
            CommitDecision::Allow => CommitVerdict::Allow,
            CommitDecision::Block(BlockKind::UnwitnessedCommit) => CommitVerdict::UnwitnessedBlock,
            CommitDecision::NeedsL4Validation => {
                needed_engine_at_least_once = true;
                let request = request_for(commit.clone(), rule.clone(), &repo_root);
                match engine.validate(&request) {
                    ValidationVerdict::Allow => {
                        all_engine_unavailable_when_needed = false;
                        CommitVerdict::Allow
                    }
                    ValidationVerdict::Block { diagnostics } => {
                        all_engine_unavailable_when_needed = false;
                        // The verdict carries the diagnostics unchanged.
                        // The downstream `block` flag check below decides
                        // whether the binary exit code is `EXIT_BLOCK` or
                        // a silent admit (warn-only + `OnWarn::Allow`).
                        // Both branches of the prior `if warn_only` here
                        // produced the same value; collapsing them keeps
                        // the per-commit verdict honest about the engine
                        // having said "Block" while leaving the exit
                        // decision to the warn-aware logic below.
                        CommitVerdict::Block { diagnostics }
                    }
                    ValidationVerdict::EngineUnavailable { reason } => {
                        CommitVerdict::EnginePending { reason }
                    }
                }
            }
        };
        if matches!(
            &verdict,
            CommitVerdict::UnwitnessedBlock | CommitVerdict::Block { .. }
        ) {
            // The pre-push hook collapses warn-only `Block` into an
            // admit with diagnostics. l4-validate preserves the per-
            // commit verdict but only flips the binary-exit `block`
            // flag when the rule says the verdict refuses (any
            // block diagnostic OR warn-only + OnWarn::Reject).
            if let CommitVerdict::Block { diagnostics } = &verdict {
                let warn_only = !diagnostics.is_empty()
                    && diagnostics.iter().all(|d| d.severity == Severity::Warn);
                if !(warn_only && rule.on_warn == OnWarn::Allow) {
                    block = true;
                }
            } else {
                block = true;
            }
        }
        outcomes.push(CommitOutcome {
            commit_sha: commit,
            verdict,
        });
    }

    let exit_code = if block {
        Some(EXIT_BLOCK)
    } else if needed_engine_at_least_once && all_engine_unavailable_when_needed {
        Some(EXIT_PENDING)
    } else {
        None
    };

    Ok(L4ValidateOutcome {
        commits: outcomes,
        exit_code,
    })
}

/// Read `anvil/project-id` to detect anvil opt-in.
fn read_project_id(repo_root: &Path) -> Option<String> {
    let path = repo_root.join("anvil").join("project-id");
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_owned())
}

/// Load `anvil/policy.{yml,yaml,json,toml}` if present.
fn load_policy(repo_root: &Path) -> Result<Option<Policy>> {
    let candidates: &[(&str, ConfigFormat)] = &[
        ("anvil/policy.yml", ConfigFormat::Yaml),
        ("anvil/policy.yaml", ConfigFormat::Yaml),
        ("anvil/policy.json", ConfigFormat::Json),
        ("anvil/policy.toml", ConfigFormat::Toml),
    ];
    for (rel, format) in candidates {
        let path = repo_root.join(rel);
        if path.exists() {
            // MLP2-063: refuse oversized policy files before
            // `read_to_string` allocates the body. Shares the bounded
            // loader with the pre-push hook so both L4 surfaces honour
            // `anvil_config::MAX_CONFIG_FILE_BYTES`.
            let raw = anvil_config::read_to_string_bounded(&path)
                .with_context(|| format!("read {}", path.display()))?;
            let policy = Policy::parse(&raw, *format, &path)
                .with_context(|| format!("parse {}", path.display()))?;
            return Ok(Some(policy));
        }
    }
    Ok(None)
}

/// MLP2-062: build the witness path list (archive segments first in
/// lexicographic order, then `active.ndjson` if present) and run the
/// DAG-aware verifier across the whole chain. Returns `Ok(())` when
/// the chain is healthy or non-existent (fresh-adoption shape with no
/// witness tree). Errors here propagate to the binary entry point and
/// surface as a non-zero exit so CI cannot treat an unverified chain
/// as L3 evidence.
fn verify_witness_chain(repo_root: &Path) -> Result<()> {
    let paths = witness_paths(repo_root);
    if paths.is_empty() {
        return Ok(());
    }
    let path_refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    verify_chain_dag(&path_refs)
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("witness chain integrity check failed: {e}"))
}

/// Resolve the commit-range argument via `git rev-list`. Accepts:
///
/// - `<base>..<head>` — walks commits strictly between base and head.
/// - `<head>` (bare hex SHA) — walks `<head>`'s full ancestry.
///
/// Anything else is refused before invoking git so a future caller
/// cannot inject revspecs or arbitrary options.
fn resolve_range(repo_root: &Path, range: &str) -> Result<Vec<String>> {
    let (base, head) = match range.find("..") {
        Some(idx) => {
            let base = &range[..idx];
            let head = &range[idx + 2..];
            if !is_hex_sha(base) {
                anyhow::bail!("base in <base>..<head> must be a hex SHA");
            }
            (Some(base), head)
        }
        None => (None, range),
    };
    if !is_hex_sha(head) {
        anyhow::bail!("head must be a hex SHA");
    }

    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo_root).arg("rev-list");
    if let Some(base) = base {
        cmd.arg(format!("{base}..{head}")).arg("--");
    } else {
        cmd.arg(head).arg("--");
    }
    cmd.stderr(Stdio::null());
    let output = cmd
        .output()
        .with_context(|| format!("git rev-list {range}"))?;
    if !output.status.success() {
        anyhow::bail!("git rev-list refused {range}");
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Resolve the current branch via `git symbolic-ref --short HEAD`.
/// Returns `None` when HEAD is detached or git is unavailable; the
/// caller falls back to a stable placeholder string.
fn current_branch(repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["symbolic-ref", "--short", "HEAD"])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if name.is_empty() { None } else { Some(name) }
}

/// Collect every commit SHA that already has an L3 witness across
/// `anvil/witness/active.ndjson` + every archive under
/// `anvil/witness/archive/`. Returns an empty set when no witness
/// tree exists — that's a fresh-adoption shape (no witnessed commits
/// yet), distinct from "git is unreachable".
///
/// MLP2-062: uses the same `witness_paths` ordering as
/// [`verify_witness_chain`] (archive lexicographic, then active) so
/// the integrity check covers exactly the bytes harvested here.
/// Caller MUST invoke `verify_witness_chain` first — pre-MLP2-062
/// callers harvested unverified bytes and could mark forged commits
/// as witnessed.
fn collect_witnessed_shas(repo_root: &Path) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let paths = witness_paths(repo_root);
    for path in paths {
        let Ok(contents) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in contents.lines() {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(s) = value.get("commit").and_then(|v| v.as_str()) {
                    out.insert(s.to_owned());
                } else if let Some(s) = value.get("commit_sha").and_then(|v| v.as_str()) {
                    out.insert(s.to_owned());
                }
            }
        }
    }
    out
}

/// Used by the binary entry point to render the per-commit detail
/// lines to stderr when blocking. Kept module-private so the unit
/// tests do not depend on stderr ordering — they go through
/// `run_with_engine` and inspect the structured `L4ValidateOutcome`
/// directly.
pub(crate) fn render_diagnostics_to<W: Write>(
    sink: &mut W,
    commit: &str,
    diagnostics: &[ValidationDiagnostic],
) -> std::io::Result<()> {
    writeln!(
        sink,
        "anvil: L4 validation failed for {}",
        short_sha(commit)
    )?;
    for diag in diagnostics {
        let severity = match diag.severity {
            Severity::Block => "block",
            Severity::Warn => "warn",
        };
        let message = if diag.message.chars().count() > 200 {
            let mut t: String = diag.message.chars().take(197).collect();
            t.push_str("...");
            t
        } else {
            diag.message.clone()
        };
        writeln!(sink, "  {} ({severity}) — {}", diag.rule_id, message)?;
    }
    Ok(())
}

fn short_sha(sha: &str) -> String {
    let len = sha.len().min(12);
    sha[..len].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_l4::{NoOpValidationEngine, OnBlock, OnNoWitness, Requirement};
    use std::path::PathBuf;

    fn setup_repo(tmp: &tempfile::TempDir) -> PathBuf {
        let root = tmp.path().to_path_buf();
        // Initialise a real git repo so `git rev-list` succeeds.
        Command::new("git")
            .arg("-C")
            .arg(&root)
            .arg("init")
            .arg("--quiet")
            .status()
            .unwrap();
        // Pin author so commits are reproducible.
        for (k, v) in [
            ("user.email", "test@example.com"),
            ("user.name", "Test"),
            ("commit.gpgsign", "false"),
        ] {
            Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(["config", k, v])
                .status()
                .unwrap();
        }
        // Drop a project-id so the binary doesn't no-op.
        std::fs::create_dir_all(root.join("anvil")).unwrap();
        std::fs::write(root.join("anvil/project-id"), "test-project\n").unwrap();
        root
    }

    fn git_commit(root: &Path, msg: &str, body: &str, file: &str) -> String {
        std::fs::write(root.join(file), body).unwrap();
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["add", "."])
            .status()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["commit", "--quiet", "-m", msg])
            .status()
            .unwrap();
        let sha = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout;
        String::from_utf8_lossy(&sha).trim().to_owned()
    }

    fn write_policy(root: &Path, body: &str) {
        std::fs::write(root.join("anvil/policy.yml"), body).unwrap();
    }

    fn validate_at_l4_policy() -> &'static str {
        "branches:\n  - pattern: \"*\"\n    require: l4_or_l3\n    on_no_witness: validate_at_l4\n"
    }

    /// MLP2-046: with the default no-op engine, a fresh commit
    /// without an L3 witness routes to `EnginePending` and the
    /// binary's exit code degrades to `EXIT_PENDING`. Matches the
    /// pre-push hook's pre-MLP2-016 surface.
    #[test]
    fn noop_engine_returns_pending_on_unwitnessed_commit() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = setup_repo(&tmp);
        write_policy(&root, validate_at_l4_policy());
        let sha = git_commit(&root, "first", "x", "f.txt");

        let args = L4ValidateArgs {
            range: sha.clone(),
            branch: Some("main".to_owned()),
            repo: Some(root.clone()),
        };
        let outcome =
            run_with_engine(&args, &GlobalArgs::default(), &NoOpValidationEngine).unwrap();
        assert_eq!(outcome.commits.len(), 1);
        assert_eq!(outcome.commits[0].commit_sha, sha);
        assert_eq!(
            outcome.commits[0].verdict,
            CommitVerdict::EnginePending {
                reason: EngineUnavailableReason::NotImplemented,
            }
        );
        assert_eq!(outcome.exit_code, Some(EXIT_PENDING));
    }

    /// MLP2-046: a fixture engine that allows clears the exit code.
    /// Pin the binary's "happy path" against a regression that flips
    /// the engine-pending path on by default.
    #[test]
    fn allowing_engine_returns_no_exit_code() {
        struct Allowing;
        impl ValidationEngine for Allowing {
            fn validate(&self, _r: &anvil_l4::ValidationRequest) -> ValidationVerdict {
                ValidationVerdict::Allow
            }
        }

        let tmp = tempfile::TempDir::new().unwrap();
        let root = setup_repo(&tmp);
        write_policy(&root, validate_at_l4_policy());
        let sha = git_commit(&root, "first", "x", "f.txt");

        let args = L4ValidateArgs {
            range: sha.clone(),
            branch: Some("main".to_owned()),
            repo: Some(root.clone()),
        };
        let outcome = run_with_engine(&args, &GlobalArgs::default(), &Allowing).unwrap();
        assert_eq!(outcome.commits.len(), 1);
        assert_eq!(outcome.commits[0].verdict, CommitVerdict::Allow);
        assert_eq!(outcome.exit_code, None);
    }

    /// MLP2-046: a fixture engine that blocks flips the binary into
    /// `EXIT_BLOCK`. The diagnostics travel back to the caller for
    /// surfacing.
    #[test]
    fn blocking_engine_returns_exit_block_with_diagnostics() {
        struct Blocking;
        impl ValidationEngine for Blocking {
            fn validate(&self, _r: &anvil_l4::ValidationRequest) -> ValidationVerdict {
                ValidationVerdict::Block {
                    diagnostics: vec![ValidationDiagnostic {
                        rule_id: "secret-detection.aws-key".into(),
                        severity: Severity::Block,
                        message: "AWS access key leaked".into(),
                    }],
                }
            }
        }

        let tmp = tempfile::TempDir::new().unwrap();
        let root = setup_repo(&tmp);
        write_policy(&root, validate_at_l4_policy());
        let sha = git_commit(&root, "first", "x", "f.txt");

        let args = L4ValidateArgs {
            range: sha.clone(),
            branch: Some("main".to_owned()),
            repo: Some(root.clone()),
        };
        let outcome = run_with_engine(&args, &GlobalArgs::default(), &Blocking).unwrap();
        assert_eq!(outcome.commits.len(), 1);
        let CommitVerdict::Block { diagnostics } = &outcome.commits[0].verdict else {
            panic!("expected Block, got {:?}", outcome.commits[0].verdict);
        };
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, "secret-detection.aws-key");
        assert_eq!(outcome.exit_code, Some(EXIT_BLOCK));
    }

    /// MLP2-046: warn-only diagnostics with `OnWarn::Allow` admit
    /// the commit (no exit code) but the per-commit verdict still
    /// carries the diagnostics. Matches the pre-push hook's
    /// Council #C-016A behaviour.
    #[test]
    fn warn_only_diagnostics_with_on_warn_allow_admit_commit() {
        struct Warning;
        impl ValidationEngine for Warning {
            fn validate(&self, _r: &anvil_l4::ValidationRequest) -> ValidationVerdict {
                ValidationVerdict::Block {
                    diagnostics: vec![ValidationDiagnostic {
                        rule_id: "style.unused-import".into(),
                        severity: Severity::Warn,
                        message: "unused import".into(),
                    }],
                }
            }
        }

        let tmp = tempfile::TempDir::new().unwrap();
        let root = setup_repo(&tmp);
        // Default OnWarn for the YAML loader is Allow (per anvil-l4
        // tests). Pin it explicitly.
        write_policy(
            &root,
            "branches:\n  - pattern: \"*\"\n    require: l4_or_l3\n    on_no_witness: validate_at_l4\n    on_warn: allow\n",
        );
        let sha = git_commit(&root, "first", "x", "f.txt");

        let args = L4ValidateArgs {
            range: sha.clone(),
            branch: Some("main".to_owned()),
            repo: Some(root.clone()),
        };
        let outcome = run_with_engine(&args, &GlobalArgs::default(), &Warning).unwrap();
        let CommitVerdict::Block { diagnostics } = &outcome.commits[0].verdict else {
            panic!(
                "expected Block carrier, got {:?}",
                outcome.commits[0].verdict
            );
        };
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Warn);
        // No exit code -- warn-only with OnWarn::Allow admits.
        assert_eq!(outcome.exit_code, None);
    }

    /// MLP2-046: missing project-id no-ops the binary (same Serena
    /// rule as pre-push hook). Returns an empty outcome with no
    /// exit code.
    #[test]
    fn missing_project_id_is_a_no_op() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = setup_repo(&tmp);
        // Remove the project-id we created in setup_repo.
        std::fs::remove_file(root.join("anvil/project-id")).unwrap();
        write_policy(&root, validate_at_l4_policy());

        let args = L4ValidateArgs {
            range: "0".repeat(40),
            branch: Some("main".to_owned()),
            repo: Some(root.clone()),
        };
        let outcome =
            run_with_engine(&args, &GlobalArgs::default(), &NoOpValidationEngine).unwrap();
        assert!(outcome.commits.is_empty());
        assert_eq!(outcome.exit_code, None);
    }

    /// MLP2-046: a malformed range (non-hex SHA) is rejected before
    /// git is invoked. Defence-in-depth: the validation surface is
    /// callable from CI where range strings are untrusted.
    #[test]
    fn non_hex_range_is_refused_before_git_invocation() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = setup_repo(&tmp);
        write_policy(&root, validate_at_l4_policy());

        let args = L4ValidateArgs {
            range: "HEAD~1..HEAD".to_owned(),
            branch: Some("main".to_owned()),
            repo: Some(root.clone()),
        };
        let err =
            run_with_engine(&args, &GlobalArgs::default(), &NoOpValidationEngine).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("must be a hex SHA"), "got: {msg}");
    }

    /// MLP2-046: a policy file with no branch match admits silently
    /// with an empty `commits` vec (Council #4 / MINOR remediation).
    /// Matches the pre-push hook's "branches outside policy coverage
    /// admit silently" behaviour; the `NoPolicyMatch` variant stays
    /// on the public API for callers that want to surface it
    /// explicitly in future, but is no longer used by the default
    /// production path.
    #[test]
    fn unmatched_branch_admits_silently_with_empty_commits() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = setup_repo(&tmp);
        write_policy(
            &root,
            "branches:\n  - pattern: release/*\n    require: l4_or_l3\n    on_no_witness: validate_at_l4\n",
        );
        let sha = git_commit(&root, "first", "x", "f.txt");

        let args = L4ValidateArgs {
            range: sha,
            branch: Some("feature/x".to_owned()),
            repo: Some(root.clone()),
        };
        let outcome =
            run_with_engine(&args, &GlobalArgs::default(), &NoOpValidationEngine).unwrap();
        assert!(
            outcome.commits.is_empty(),
            "unmatched branch must produce no per-commit outcomes; got {:?}",
            outcome.commits,
        );
        assert_eq!(outcome.exit_code, None);
    }

    /// MLP2-046: silence the unused-imports lint by referencing the
    /// re-exports the production code needs.
    #[test]
    fn re_exports_compile() {
        let _ = OnBlock::Reject;
        let _ = OnNoWitness::ValidateAtL4;
        let _ = Requirement::L4OrL3;
    }

    /// MLP2-062: a tampered `active.ndjson` MUST cause `l4-validate`
    /// to error out before any commit is treated as L3-witnessed.
    /// Pre-fix the surface harvested `commit_sha` lines from the
    /// tampered file and a forged record could mark an unwitnessed
    /// commit as witnessed; the CI lane silently admitted it.
    #[test]
    fn run_with_engine_refuses_to_trust_tampered_witness_chain() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = setup_repo(&tmp);
        write_policy(&root, validate_at_l4_policy());
        let sha = git_commit(&root, "first", "x", "f.txt");
        // Tamper: drop a syntactically-broken NDJSON file at the
        // active witness path. The forged line claims to witness
        // `sha`, but the chain hash invariants do not hold.
        let witness_dir = root.join("anvil/witness");
        std::fs::create_dir_all(&witness_dir).unwrap();
        std::fs::write(
            witness_dir.join("active.ndjson"),
            format!(
                "{{\"seq\":1,\"scope\":\"active\",\"kind\":\"witness\",\
                 \"commit_sha\":\"{sha}\",\"prev_line_hash\":\"bogus\"}}\n"
            ),
        )
        .unwrap();

        let args = L4ValidateArgs {
            range: sha.clone(),
            branch: Some("main".to_owned()),
            repo: Some(root.clone()),
        };
        let err =
            run_with_engine(&args, &GlobalArgs::default(), &NoOpValidationEngine).unwrap_err();
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("witness chain integrity check failed"),
            "expected chain-integrity error, got: {rendered}"
        );
    }

    /// MLP2-062 (Council quick review): a forged archive segment
    /// dropped into `anvil/witness/archive/` with a filename that
    /// lexicographically precedes any legitimate segment MUST cause
    /// `l4-validate` to refuse — the chain verifier walks archive
    /// segments first, so a fake fresh-genesis prefix is the most
    /// direct way to forge an L3 witness without touching `active`.
    #[test]
    fn run_with_engine_refuses_forged_genesis_dropped_into_archive() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = setup_repo(&tmp);
        write_policy(&root, validate_at_l4_policy());
        let sha = git_commit(&root, "first", "x", "f.txt");
        // Drop a forged archive segment with a lex-leading filename.
        // The line claims to be a witness for `sha` but cites a
        // genesis anchor without seeding one first, so the DAG walk
        // rejects it.
        let archive_dir = root.join("anvil/witness/archive");
        std::fs::create_dir_all(&archive_dir).unwrap();
        std::fs::write(
            archive_dir.join("active-00000000000000000000-forged-genesis.ndjson"),
            format!(
                "{{\"seq\":1,\"scope\":\"active\",\"kind\":\"witness\",\
                 \"commit_sha\":\"{sha}\",\"prev_line_hash\":\"forged-anchor\"}}\n"
            ),
        )
        .unwrap();

        let args = L4ValidateArgs {
            range: sha.clone(),
            branch: Some("main".to_owned()),
            repo: Some(root.clone()),
        };
        let err =
            run_with_engine(&args, &GlobalArgs::default(), &NoOpValidationEngine).unwrap_err();
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("witness chain integrity check failed"),
            "expected chain-integrity error from forged archive segment, got: {rendered}"
        );
    }

    /// MLP2-063: oversized policy files MUST be refused before
    /// `read_to_string` allocates the body. Shared bounded loader
    /// with the pre-push hook (see hook.rs sibling test).
    #[test]
    fn load_policy_refuses_oversized_yaml() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = setup_repo(&tmp);
        // 1 MiB + padding → just past the cap.
        let cap = usize::try_from(anvil_config::MAX_CONFIG_FILE_BYTES).expect("1 MiB fits usize");
        let mut body = String::with_capacity(cap + 128);
        body.push_str(validate_at_l4_policy());
        let comment_prefix = "# pad ";
        while body.len() <= cap {
            body.push_str(comment_prefix);
            body.push_str(&"x".repeat(64));
            body.push('\n');
        }
        assert!(body.len() as u64 > anvil_config::MAX_CONFIG_FILE_BYTES);
        write_policy(&root, &body);

        let err = load_policy(&root).unwrap_err();
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("exceeds") || rendered.contains("byte limit"),
            "expected FileTooLarge surface in error chain, got: {rendered}"
        );
    }

    /// MLP2-016 reopened (2026-05-15 Council audit). The audit's
    /// load-bearing requirement is that the production default path
    /// runs a real rule engine, not `NoOpValidationEngine`. This
    /// test drives [`run_with_engine`] using the same
    /// [`crate::l4_engine::default_engine`] constructor that the
    /// production [`run`] entry point uses — no fixture injection —
    /// against a commit that carries a known `AP-001`
    /// `eslint-disable` antipattern. The verdict must be
    /// `Block { diagnostics }` carrying the real rule id, proving
    /// the engine actually scanned the commit's tree.
    #[test]
    fn production_default_engine_blocks_known_antipattern() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = setup_repo(&tmp);
        write_policy(
            &root,
            "branches:\n  - pattern: \"*\"\n    require: l4_or_l3\n    on_no_witness: validate_at_l4\n    on_warn: reject\n",
        );
        // A `.ts` file containing a broad eslint-disable directive —
        // matches `AP-001` in the antipattern catalogue.
        let sha = git_commit(
            &root,
            "leak",
            "/* eslint-disable */\nimport { x } from './m';\n",
            "leak.ts",
        );

        let args = L4ValidateArgs {
            range: sha.clone(),
            branch: Some("main".to_owned()),
            repo: Some(root.clone()),
        };
        let engine = crate::l4_engine::default_engine();
        let outcome = run_with_engine(&args, &GlobalArgs::default(), engine.as_ref()).unwrap();
        assert_eq!(outcome.commits.len(), 1);
        let CommitVerdict::Block { diagnostics } = &outcome.commits[0].verdict else {
            panic!(
                "production default engine MUST run real rules — \
                 expected Block, got {:?}",
                outcome.commits[0].verdict,
            );
        };
        assert!(
            diagnostics.iter().any(|d| d.rule_id == "AP-001"),
            "expected AP-001 in real-engine diagnostics, got {diagnostics:?}",
        );
        assert_eq!(outcome.exit_code, Some(EXIT_BLOCK));
    }
}
