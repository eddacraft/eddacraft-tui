//! EXCEPT-004: `anvil exception grant|revoke|list|show|verify|migrate`.
//!
//! Operator surface for the tracked exception store
//! (`anvil/exceptions/store.json`, ADR-073). Writes are **explicit-only**
//! and go through [`ExceptionStore::update`], the EXCEPT-007 locked
//! load-modify-save primitive — no evaluation or check command writes
//! the store implicitly, so checks never dirty a worktree.
//!
//! Contract points (2026-06-08 council):
//! - `grant` refuses to mint an unattributed record: attribution comes
//!   from `--owner` and/or the repository's git identity
//!   (`user.email`, falling back to `user.name`). The ADR-073
//!   downgrade path for unattributed grants exists for *legacy* v0
//!   data, not as something this CLI produces.
//! - `revoke` soft-deletes: the record stays in the store with a
//!   revocation audit trail; nothing is erased.
//! - On [`WriteOutcome::SkippedReadOnly`] the command warns and exits
//!   non-zero (an explicit write that did not persist is not a
//!   success); the underlying I/O error — deliberately conflating
//!   read-only checkouts with permission misconfiguration — surfaces
//!   under `--verbose`.
//! - `verify` surfaces the EXCEPT-005 verdicts (active / unattributed
//!   / expired / revoked / invalid-scope) without enforcing anything;
//!   it always exits zero (warnings over blocks, ADR-002).
//! - Attribution is **advisory**: `created_by`/`revoked_by` come from
//!   local git config, which the author controls — reviewers should
//!   verify it against PR authorship, exactly as with commit authors.
//! - `--json` mode emits exactly one JSON document on stdout; human
//!   warnings fold into the document instead of preceding it.
//! - Writes honour the pre-release project-write gate: a candidate
//!   binary under a non-default install root refuses to mutate a real
//!   checkout unless explicitly permitted.

use std::path::Path;

use anvil_policy::exceptions::{
    ExceptionRevocation, ExceptionStore, ExceptionVerdict, MigrateOutcome, PolicyException,
    WriteOutcome, is_visibly_blank, verify_exception_at,
};
use anyhow::{Context, Result, bail};
use chrono::{DateTime, Duration, Utc};
use clap::{Args, Subcommand};
use serde::Serialize;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct ExceptionArgs {
    #[command(subcommand)]
    command: ExceptionCommand,
}

#[derive(Debug, Subcommand)]
enum ExceptionCommand {
    /// Grant a scoped, attributed policy exception.
    Grant {
        /// Policy or rule identifier the exception suppresses.
        #[arg(long)]
        policy: String,
        /// Justification recorded on the grant.
        #[arg(long)]
        reason: String,
        /// Glob scope (e.g. "src/legacy/**"). Empty means every file.
        #[arg(long, default_value = "")]
        scope: String,
        /// Accountable team or owner. Defaults to unset; the grant is
        /// attributed via git identity when omitted.
        #[arg(long)]
        owner: Option<String>,
        /// Days until the grant expires. Omitted means it never expires.
        #[arg(long, value_name = "DAYS")]
        expires_in_days: Option<u32>,
        /// Pin the grant to one concrete finding instance by its hash.
        #[arg(long, value_name = "HASH")]
        finding_hash: Option<String>,
    },
    /// Revoke a granted exception, preserving the audit trail.
    Revoke {
        /// Exception identifier (see `list`).
        id: String,
        /// Justification recorded on the revocation.
        #[arg(long)]
        reason: String,
    },
    /// List exceptions in the tracked store with their verdicts.
    List,
    /// Show one exception in full.
    Show {
        /// Exception identifier (see `list`).
        id: String,
    },
    /// Verify every exception: scope, expiry, revocation, attribution.
    Verify,
    /// Copy the legacy local store into the tracked store (one-time,
    /// non-destructive).
    Migrate,
}

pub fn run(args: &ExceptionArgs, global: &GlobalArgs) -> Result<()> {
    let root = crate::util::workspace_root()?;
    match &args.command {
        ExceptionCommand::Grant {
            policy,
            reason,
            scope,
            owner,
            expires_in_days,
            finding_hash,
        } => {
            let request = GrantRequest {
                policy_id: policy.clone(),
                reason: reason.clone(),
                file_pattern: scope.clone(),
                owner: owner.clone(),
                created_by: git_identity(&root),
                expires_in_days: *expires_in_days,
                finding_hash: finding_hash.clone(),
            };
            crate::install_root::ensure_project_write_allowed("exception grant")?;
            grant_command(&root, request, global)
        }
        ExceptionCommand::Revoke { id, reason } => {
            crate::install_root::ensure_project_write_allowed("exception revoke")?;
            let Some(actor) = git_identity(&root) else {
                bail!(
                    "revocation needs an actor: set git user.email (or user.name) so the \
                     audit trail records who revoked the grant"
                );
            };
            let write = run_revoke(&root, id, reason, actor)?;
            finish_write(&write, global)?;
            if global.json {
                crate::output::json::print(&serde_json::json!({ "revoked": id }))?;
            } else {
                crate::output::plain::success(&format!("revoked exception {id}"));
            }
            Ok(())
        }
        ExceptionCommand::List => {
            let views = run_list(&root)?;
            if global.json {
                crate::output::json::print(&views)?;
            } else if views.is_empty() {
                crate::output::plain::info("no exceptions in the tracked store");
            } else {
                render_table(&views);
            }
            Ok(())
        }
        ExceptionCommand::Verify => {
            let views = run_list(&root)?;
            let counts = verdict_counts(&views);
            if global.json {
                crate::output::json::print(&serde_json::json!({
                    "exceptions": views,
                    "counts": counts,
                }))?;
            } else if views.is_empty() {
                crate::output::plain::info("no exceptions in the tracked store");
            } else {
                render_table(&views);
                let summary = counts
                    .iter()
                    .map(|(verdict, n)| format!("{n} {verdict}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                crate::output::plain::info(&summary);
            }
            Ok(())
        }
        ExceptionCommand::Show { id } => {
            let view = run_show(&root, id)?;
            if global.json {
                crate::output::json::print(&view)?;
            } else {
                render_full(&view);
            }
            Ok(())
        }
        ExceptionCommand::Migrate => {
            crate::install_root::ensure_project_write_allowed("exception migrate")?;
            migrate_command(&root, global)
        }
    }
}

/// `grant` arm body: run the core, fold warnings into the JSON
/// document or print them ahead of the plain success line.
fn grant_command(root: &Path, request: GrantRequest, global: &GlobalArgs) -> Result<()> {
    let outcome = run_grant(root, request)?;
    if !global.json {
        for warning in &outcome.warnings {
            crate::output::plain::warn(warning);
        }
    }
    finish_write(&outcome.write, global)?;
    if global.json {
        crate::output::json::print(&outcome)?;
    } else {
        crate::output::plain::success(&format!("granted exception {}", outcome.id));
        crate::output::plain::dim(
            "commit anvil/exceptions/store.json for the grant to take effect at the gate — \
             gates read the store from the pushed commit's tree, never the worktree",
        );
    }
    Ok(())
}

/// `migrate` arm body: honour `--json` with a single outcome token,
/// keep the read-only degrade diagnosable in plain mode.
fn migrate_command(root: &Path, global: &GlobalArgs) -> Result<()> {
    let outcome = ExceptionStore::migrate(root)
        .context("migrate legacy exception store into the tracked tree")?;
    if global.json {
        if let MigrateOutcome::SkippedReadOnly { detail } = &outcome {
            // Explicit-write contract: a migration that did not persist
            // must not exit zero; the error (with detail) travels on
            // stderr, keeping stdout JSON-pure.
            bail!(
                "exception store migration skipped (read-only worktree or permission \
                 misconfiguration): {detail}"
            );
        }
        let token = match &outcome {
            MigrateOutcome::Migrated => "migrated",
            MigrateOutcome::NothingToDo => "nothing-to-do",
            MigrateOutcome::SkippedReadOnly { .. } => unreachable!("handled above"),
        };
        crate::output::json::print(&serde_json::json!({ "outcome": token }))?;
        return Ok(());
    }
    match outcome {
        MigrateOutcome::Migrated => crate::output::plain::success(
            "legacy exceptions copied into anvil/exceptions/store.json (legacy file \
             left in place — remove it once the tracked store is committed)",
        ),
        MigrateOutcome::NothingToDo => crate::output::plain::info(
            "nothing to migrate: no legacy store, or the tracked store already exists",
        ),
        MigrateOutcome::SkippedReadOnly { detail } => {
            crate::output::plain::warn("worktree is read-only — migration skipped");
            if global.verbose {
                crate::output::plain::dim(&format!("underlying error: {detail}"));
            } else {
                crate::output::plain::dim("re-run with --verbose for the underlying error");
            }
            bail!("exception store migration skipped");
        }
    }
    Ok(())
}

/// Inputs for one grant. Separated from clap so tests drive the core
/// without argument parsing or a real git identity.
#[derive(Debug)]
struct GrantRequest {
    policy_id: String,
    reason: String,
    file_pattern: String,
    owner: Option<String>,
    created_by: Option<String>,
    expires_in_days: Option<u32>,
    finding_hash: Option<String>,
}

/// Result of a grant: the derived stable id, authoring-time warnings,
/// and the store write outcome the caller must surface.
#[derive(Debug, Serialize)]
struct GrantOutcome {
    id: String,
    verdict: &'static str,
    warnings: Vec<String>,
    #[serde(skip)]
    write: WriteOutcome,
}

/// One exception as rendered by `list`/`show`/`verify`: the stored
/// record plus its verdict at evaluation time.
#[derive(Debug, Serialize)]
struct ExceptionView {
    id: String,
    policy_id: String,
    scope: String,
    verdict: &'static str,
    reason: String,
    owner: Option<String>,
    created_by: Option<String>,
    created_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    revoked_by: Option<String>,
}

fn run_grant(root: &Path, request: GrantRequest) -> Result<GrantOutcome> {
    // is_visibly_blank (shared with the store's own attribution check)
    // treats zero-width/format characters as blank, so `--owner
    // $'\u200b'` cannot masquerade as attribution.
    let non_blank = |value: &Option<String>| {
        value
            .as_deref()
            .map(str::trim)
            .filter(|s| !is_visibly_blank(s))
            .map(str::to_string)
    };
    let owner = non_blank(&request.owner);
    let created_by = non_blank(&request.created_by);
    if owner.is_none() && created_by.is_none() {
        bail!(
            "grant needs attribution: pass --owner, or configure git user.email (or \
             user.name) so the record names who created it"
        );
    }
    let now = Utc::now();
    let exception = PolicyException {
        schema_version: String::new(),
        id: String::new(),
        policy_id: request.policy_id,
        file_pattern: request.file_pattern.trim().to_string(),
        finding_hash: non_blank(&request.finding_hash),
        reason: request.reason,
        owner,
        created_by,
        created_at: now,
        expires_at: request
            .expires_in_days
            .map(|days| now + Duration::days(i64::from(days))),
        revoked: None,
    };
    let verdict = verify_exception_at(&exception, now);
    if verdict == ExceptionVerdict::InvalidScope {
        bail!(
            "scope {:?} is not a parseable glob — the grant would never apply; fix the \
             pattern and re-run",
            exception.file_pattern,
        );
    }
    let mut warnings = Vec::new();
    if exception.file_pattern.is_empty() || exception.file_pattern == "**" {
        warnings.push(
            "scope covers every file in the repository — prefer the narrowest glob that \
             fits the deviation"
                .to_string(),
        );
    }
    if exception.expires_at.is_none() {
        warnings.push(
            "grant never expires — consider --expires-in-days so it lapses when the \
             deviation should be fixed"
                .to_string(),
        );
    }
    let mut id = String::new();
    let mut duplicate = false;
    let write = ExceptionStore::update(root, |store| {
        store.add(exception.clone());
        id = store
            .exceptions
            .last()
            .map(|ex| ex.id.clone())
            .unwrap_or_default();
        // Ids are derived from (policy, scope, created_at, hash); a
        // colliding grant would create two records that list/show
        // cannot tell apart and revoke would only half-kill. Refuse
        // and leave the store as it was.
        if store.exceptions[..store.exceptions.len() - 1]
            .iter()
            .any(|ex| ex.id == id)
        {
            duplicate = true;
            store.exceptions.pop();
        }
    })
    .context("exception store write refused")?;
    if duplicate {
        bail!("an identical grant ({id}) already exists — revoke it first if it needs replacing");
    }
    Ok(GrantOutcome {
        id,
        verdict: verdict.as_str(),
        warnings,
        write,
    })
}

fn run_revoke(root: &Path, id: &str, reason: &str, revoked_by: String) -> Result<WriteOutcome> {
    let store = ExceptionStore::load(root).context("load exception store")?;
    let matches: Vec<&PolicyException> = store.exceptions.iter().filter(|ex| ex.id == id).collect();
    if matches.len() > 1 {
        bail!(
            "{} records share id {id} — the store is ambiguous; fix anvil/exceptions/store.json \
             before revoking (a duplicate id can hide a decoy grant)",
            matches.len(),
        );
    }
    let Some(existing) = matches.first() else {
        bail!("no exception with id {id} in the store — `anvil exception list` shows ids");
    };
    if existing.revoked.is_some() {
        bail!("exception {id} is already revoked; the original audit trail stands");
    }
    let revocation = ExceptionRevocation {
        revoked_at: Utc::now(),
        revoked_by,
        reason: reason.to_string(),
    };
    let mut applied = false;
    let mut ambiguous = false;
    let write = ExceptionStore::update(root, |store| {
        // Re-check ambiguity under the lock: a concurrent writer could
        // have introduced a duplicate id since the pre-check, and a
        // first-match revoke would half-kill it.
        if store.exceptions.iter().filter(|ex| ex.id == id).count() > 1 {
            ambiguous = true;
            return;
        }
        if let Some(ex) = store
            .exceptions
            .iter_mut()
            .find(|ex| ex.id == id && ex.revoked.is_none())
        {
            ex.revoked = Some(revocation.clone());
            applied = true;
        }
    })
    .context("exception store write refused")?;
    if ambiguous {
        bail!(
            "multiple records share id {id} — the store is ambiguous; fix \
             anvil/exceptions/store.json before revoking"
        );
    }
    if !applied && write == WriteOutcome::Written {
        bail!("exception {id} changed concurrently; re-run the revocation");
    }
    Ok(write)
}

fn run_list(root: &Path) -> Result<Vec<ExceptionView>> {
    let store = ExceptionStore::load(root).context("load exception store")?;
    let now = Utc::now();
    Ok(store.exceptions.iter().map(|ex| view_of(ex, now)).collect())
}

fn run_show(root: &Path, id: &str) -> Result<ExceptionView> {
    let mut matches: Vec<ExceptionView> = run_list(root)?
        .into_iter()
        .filter(|view| view.id == id)
        .collect();
    if matches.len() > 1 {
        bail!(
            "{} records share id {id} — the store is ambiguous; fix anvil/exceptions/store.json",
            matches.len(),
        );
    }
    matches.pop().with_context(|| {
        format!("no exception with id {id} in the store — `anvil exception list` shows ids")
    })
}

fn view_of(exception: &PolicyException, now: DateTime<Utc>) -> ExceptionView {
    ExceptionView {
        id: exception.id.clone(),
        policy_id: exception.policy_id.clone(),
        scope: exception.file_pattern.clone(),
        verdict: verify_exception_at(exception, now).as_str(),
        reason: exception.reason.clone(),
        owner: exception.owner.clone(),
        created_by: exception.created_by.clone(),
        created_at: exception.created_at,
        expires_at: exception.expires_at,
        revoked_by: exception.revoked.as_ref().map(|r| r.revoked_by.clone()),
    }
}

/// Repository git identity for attribution: `user.email`, falling back
/// to `user.name`. `None` when neither is configured (or git is
/// unavailable) — the caller decides whether that is fatal.
fn git_identity(root: &Path) -> Option<String> {
    for key in ["user.email", "user.name"] {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["config", "--get", key])
            .output()
            .ok()?;
        if output.status.success() {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

/// Surface a store write outcome. `Written` is silent success; a
/// read-only skip warns (with the underlying I/O error under
/// `--verbose`) and errors out — an explicit write that did not
/// persist must not exit zero.
fn finish_write(write: &WriteOutcome, global: &GlobalArgs) -> Result<()> {
    match write {
        WriteOutcome::Written => Ok(()),
        WriteOutcome::SkippedReadOnly { detail } => {
            if global.json {
                // Stream purity: no plain text on stdout in JSON mode —
                // the error (with detail) travels on stderr.
                bail!(
                    "exception store write skipped (read-only worktree or permission \
                     misconfiguration): {detail}"
                );
            }
            crate::output::plain::warn(
                "store not written: the worktree is read-only or permissions refuse the \
                 governance tree",
            );
            if global.verbose {
                crate::output::plain::dim(&format!("underlying error: {detail}"));
            } else {
                crate::output::plain::dim("re-run with --verbose for the underlying error");
            }
            bail!("exception store write skipped");
        }
    }
}

/// Per-verdict counts for `verify`'s summary, in verdict-precedence
/// order (stable output for scripts diffing the summary).
fn verdict_counts(views: &[ExceptionView]) -> Vec<(&'static str, usize)> {
    [
        "revoked",
        "expired",
        "invalid-scope",
        "unattributed",
        "active",
    ]
    .into_iter()
    .map(|verdict| {
        (
            verdict,
            views.iter().filter(|v| v.verdict == verdict).count(),
        )
    })
    .filter(|(_, n)| *n > 0)
    .collect()
}

/// Neutralise control and zero-width/format characters before a store
/// field reaches the terminal: the tracked store is a multi-author,
/// git-tracked file, so `reason`/`owner`/`scope`/`id` are untrusted
/// render input (ANSI escapes, bidi overrides). Mirrors the capsule
/// renderer's display-unsafe discipline.
fn sanitise_field(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_control() || (is_visibly_blank(&c.to_string()) && !c.is_whitespace()) {
                '\u{FFFD}'
            } else {
                c
            }
        })
        .collect()
}

fn render_table(views: &[ExceptionView]) {
    crate::output::plain::section("Exceptions");
    for view in views {
        let expiry = view.expires_at.map_or_else(
            || "never".to_string(),
            |at| at.format("%Y-%m-%d").to_string(),
        );
        println!(
            "  {}  {:<10}  {:<9}  expires {}  {}",
            sanitise_field(&view.id),
            sanitise_field(&view.policy_id),
            view.verdict,
            expiry,
            if view.scope.is_empty() {
                "(all files)".to_string()
            } else {
                sanitise_field(&view.scope)
            },
        );
    }
}

fn render_full(view: &ExceptionView) {
    crate::output::plain::section(&sanitise_field(&view.id));
    crate::output::plain::label("policy", sanitise_field(&view.policy_id));
    crate::output::plain::label(
        "scope",
        if view.scope.is_empty() {
            "(all files)".to_string()
        } else {
            sanitise_field(&view.scope)
        },
    );
    crate::output::plain::label("verdict", view.verdict);
    crate::output::plain::label("reason", sanitise_field(&view.reason));
    if let Some(owner) = &view.owner {
        crate::output::plain::label("owner", sanitise_field(owner));
    }
    if let Some(created_by) = &view.created_by {
        crate::output::plain::label("created by", sanitise_field(created_by));
    }
    crate::output::plain::label("created at", view.created_at.to_rfc3339());
    if let Some(expires_at) = &view.expires_at {
        crate::output::plain::label("expires at", expires_at.to_rfc3339());
    }
    if let Some(revoked_by) = &view.revoked_by {
        crate::output::plain::label("revoked by", sanitise_field(revoked_by));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn request(policy: &str, scope: &str) -> GrantRequest {
        GrantRequest {
            policy_id: policy.to_string(),
            reason: "test grant".to_string(),
            file_pattern: scope.to_string(),
            owner: Some("team-platform".to_string()),
            created_by: Some("alice@example.test".to_string()),
            expires_in_days: Some(30),
            finding_hash: None,
        }
    }

    /// `grant` persists an attributed record through the locked update
    /// primitive and reports the derived stable id.
    #[test]
    fn grant_writes_attributed_record_to_tracked_store() {
        let tmp = TempDir::new().unwrap();
        let outcome = run_grant(tmp.path(), request("AP-001", "src/legacy/**")).unwrap();
        assert!(!outcome.id.is_empty());
        assert_eq!(outcome.verdict, "active");
        assert_eq!(outcome.write, WriteOutcome::Written);
        let store = ExceptionStore::load(tmp.path()).unwrap();
        assert_eq!(store.exceptions.len(), 1);
        let ex = &store.exceptions[0];
        assert_eq!(ex.policy_id, "AP-001");
        assert_eq!(ex.owner.as_deref(), Some("team-platform"));
        assert_eq!(ex.created_by.as_deref(), Some("alice@example.test"));
        assert!(ex.expires_at.is_some());
        assert_eq!(ex.id, outcome.id);
    }

    /// The CLI never mints an unattributed grant: no owner and no git
    /// identity refuses with an actionable message.
    #[test]
    fn grant_refuses_unattributed_record() {
        let tmp = TempDir::new().unwrap();
        let mut req = request("AP-001", "");
        req.owner = None;
        req.created_by = None;
        let err = run_grant(tmp.path(), req).expect_err("unattributed grant must refuse");
        assert!(err.to_string().contains("attribut"), "{err}");
        assert!(
            ExceptionStore::load(tmp.path())
                .unwrap()
                .exceptions
                .is_empty(),
            "nothing may be written on refusal",
        );
    }

    /// Blank attribution (whitespace owner, no identity) is refused the
    /// same way — `owner: " "` must not smuggle an unattributed grant
    /// past the gate (mirrors the EXCEPT-005 blank-attribution rule).
    #[test]
    fn grant_refuses_blank_attribution() {
        let tmp = TempDir::new().unwrap();
        let mut req = request("AP-001", "");
        req.owner = Some("   ".to_string());
        req.created_by = None;
        run_grant(tmp.path(), req).expect_err("blank attribution must refuse");
    }

    /// An unparseable scope glob refuses at authoring time — the store
    /// never carries a grant that evaluation would classify
    /// invalid-scope.
    #[test]
    fn grant_refuses_invalid_scope_glob() {
        let tmp = TempDir::new().unwrap();
        let err = run_grant(tmp.path(), request("AP-001", "src/[oops"))
            .expect_err("invalid glob must refuse");
        assert!(err.to_string().contains("scope"), "{err}");
    }

    /// Authoring-time breadth warnings: an empty scope covers the whole
    /// repository and a grant without expiry never lapses — both are
    /// legal but must be called out.
    #[test]
    fn grant_warns_on_repo_wide_scope_and_no_expiry() {
        let tmp = TempDir::new().unwrap();
        let mut req = request("AP-001", "");
        req.expires_in_days = None;
        let outcome = run_grant(tmp.path(), req).unwrap();
        assert!(
            outcome.warnings.iter().any(|w| w.contains("every file")),
            "repo-wide scope warning missing: {:?}",
            outcome.warnings,
        );
        assert!(
            outcome.warnings.iter().any(|w| w.contains("never expires")),
            "no-expiry warning missing: {:?}",
            outcome.warnings,
        );
    }

    /// `revoke` soft-deletes: the record stays with an audit trail and
    /// subsequent verification reports `revoked`.
    #[test]
    fn revoke_soft_deletes_with_audit_trail() {
        let tmp = TempDir::new().unwrap();
        let outcome = run_grant(tmp.path(), request("AP-001", "src/**")).unwrap();
        let write = run_revoke(
            tmp.path(),
            &outcome.id,
            "no longer needed",
            "bob@example.test".to_string(),
        )
        .unwrap();
        assert_eq!(write, WriteOutcome::Written);
        let store = ExceptionStore::load(tmp.path()).unwrap();
        assert_eq!(store.exceptions.len(), 1, "revocation must not erase");
        let revoked = store.exceptions[0].revoked.as_ref().expect("audit trail");
        assert_eq!(revoked.revoked_by, "bob@example.test");
        assert_eq!(revoked.reason, "no longer needed");
    }

    /// Revoking an unknown id refuses without touching the store;
    /// revoking twice refuses (the first revocation is the record).
    #[test]
    fn revoke_refuses_unknown_and_double_revocation() {
        let tmp = TempDir::new().unwrap();
        let outcome = run_grant(tmp.path(), request("AP-001", "src/**")).unwrap();
        run_revoke(tmp.path(), "exc_missing", "x", "bob".to_string())
            .expect_err("unknown id must refuse");
        let _ = run_revoke(tmp.path(), &outcome.id, "first", "bob".to_string()).unwrap();
        run_revoke(tmp.path(), &outcome.id, "second", "carol".to_string())
            .expect_err("double revocation must refuse");
        let store = ExceptionStore::load(tmp.path()).unwrap();
        assert_eq!(
            store.exceptions[0].revoked.as_ref().unwrap().revoked_by,
            "bob",
            "the original audit trail must survive a refused second revocation",
        );
    }

    /// `list`/`verify` render every record with its verdict; `show`
    /// finds one by id.
    #[test]
    fn list_and_show_report_verdicts() {
        let tmp = TempDir::new().unwrap();
        let kept = run_grant(tmp.path(), request("AP-001", "src/**")).unwrap();
        let gone = run_grant(tmp.path(), request("AP-002", "lib/**")).unwrap();
        let _ = run_revoke(tmp.path(), &gone.id, "cleanup", "bob".to_string()).unwrap();
        let views = run_list(tmp.path()).unwrap();
        assert_eq!(views.len(), 2);
        let kept_view = views.iter().find(|v| v.id == kept.id).unwrap();
        assert_eq!(kept_view.verdict, "active");
        let gone_view = views.iter().find(|v| v.id == gone.id).unwrap();
        assert_eq!(gone_view.verdict, "revoked");
        assert_eq!(gone_view.revoked_by.as_deref(), Some("bob"));
        let shown = run_show(tmp.path(), &kept.id).unwrap();
        assert_eq!(shown.policy_id, "AP-001");
        run_show(tmp.path(), "exc_missing").expect_err("unknown id must refuse");
    }

    /// Zero-width-only attribution refuses like blank attribution —
    /// `--owner $'\u{200B}'` must not mint an "attributed" grant.
    #[test]
    fn grant_refuses_invisible_unicode_attribution() {
        let tmp = TempDir::new().unwrap();
        let mut req = request("AP-001", "");
        req.owner = Some("\u{200B}\u{2060}".to_string());
        req.created_by = None;
        run_grant(tmp.path(), req).expect_err("invisible attribution must refuse");
    }

    /// An identical grant (same policy/scope/instant → same derived id)
    /// refuses instead of planting an indistinguishable duplicate that
    /// a later revoke would only half-kill.
    #[test]
    fn grant_refuses_duplicate_id() {
        let tmp = TempDir::new().unwrap();
        let now = chrono::Utc::now();
        // Drive the store directly so both records share created_at
        // (the id-derivation input that collides).
        let mut first = PolicyException {
            schema_version: String::new(),
            id: String::new(),
            policy_id: "AP-001".to_string(),
            file_pattern: "src/**".to_string(),
            finding_hash: None,
            reason: "test grant".to_string(),
            owner: Some("team-platform".to_string()),
            created_by: None,
            created_at: now,
            expires_at: None,
            revoked: None,
        };
        let outcome = ExceptionStore::update(tmp.path(), |store| {
            store.add(first.clone());
            first.id = store.exceptions.last().unwrap().id.clone();
        })
        .unwrap();
        assert_eq!(outcome, WriteOutcome::Written);
        // Plant a decoy sharing the id (differing reason) and assert
        // the id-addressed verbs refuse the ambiguity.
        let outcome = ExceptionStore::update(tmp.path(), |store| {
            let mut decoy = first.clone();
            decoy.reason = "decoy".to_string();
            store.exceptions.push(decoy);
        })
        .unwrap();
        assert_eq!(outcome, WriteOutcome::Written);
        let err = run_revoke(tmp.path(), &first.id, "cleanup", "bob".to_string())
            .expect_err("ambiguous id must refuse revocation");
        assert!(err.to_string().contains("ambiguous"), "{err}");
        run_show(tmp.path(), &first.id).expect_err("ambiguous id must refuse show");
    }

    /// Store fields are render-sanitised: control and zero-width
    /// characters cannot reach the terminal from list/show output.
    #[test]
    fn sanitise_field_neutralises_escapes() {
        let hostile = "ok\u{1b}[31mred\u{200B}end";
        let cleaned = sanitise_field(hostile);
        assert!(!cleaned.contains('\u{1b}'));
        assert!(!cleaned.contains('\u{200B}'));
        assert!(cleaned.contains("ok"), "{cleaned}");
        assert!(cleaned.contains("red"), "{cleaned}");
    }

    /// A legacy-origin store refuses writes with the migrate hint
    /// (ADR-073 explicit-promotion discipline) — the CLI surfaces the
    /// store error rather than silently promoting.
    #[test]
    fn grant_on_legacy_store_surfaces_migrate_requirement() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".anvil")).unwrap();
        std::fs::write(
            tmp.path().join(".anvil/exceptions.json"),
            r#"{"exceptions":[]}"#,
        )
        .unwrap();
        let err = run_grant(tmp.path(), request("AP-001", ""))
            .expect_err("legacy origin must refuse until migrated");
        let chain = format!("{err:#}").to_lowercase();
        assert!(chain.contains("migrate"), "{chain}");
    }
}
