//! `anvil start` activation orchestration (LAUNCH-006 / LAUNCH-009).
//!
//! Composes the read-safe / idempotent primitives the activation flow
//! ships today:
//!
//! 1. Probe `activation::verify`. If `.anvilrc` is absent, call
//!    `commands::init::run_in` (which writes the default config AND runs
//!    the LAUNCH-004 post-init first-scan inline).
//! 2. Register the current worktree with the intercept daemon when it is live.
//!    This is the MCP-independent activation spine from ACTMO-002; failure is
//!    non-fatal and the diagnostic remains the source of truth.
//! 3. **MCP install (LAUNCH-009 part 2).** Probe each registered MCP
//!    client (Cursor, Claude Code), classify drift, and either prompt
//!    the user with a [`demand`] picker (interactive) or auto-install
//!    the obvious cases (non-interactive). See [`install`] for the
//!    drift policy and atomicity guarantees.
//! 4. Re-probe and return the diagnostic for the caller to render.
//!
//! **Deliberately NOT in this orchestrator** (owned by diagnostic probes /
//! LAUNCH-011 — the tasks that own the safe versions of these steps):
//! - **Server startable spawn probe.** `activation::verify` owns the
//!   read-only MCP handshake and promotes `RestartRequired` to
//!   `RestartHandshakeVerified` when the installed entry serves MCP.
//! - **Watch fallback spawn.** LAUNCH-011 owns the in-process / detached
//!   watcher that lets `start` end in the `watching` state. Until then,
//!   `WatchTier::NotRequested` is the honest answer.
//! - **Doctor composition.** `anvil doctor` bails on any check failure;
//!   inside `start`'s composed flow that propagates as a hard error and
//!   strips the user of a `ProtectionState` literal.
//!
//! **First-run marker:** `anvil start` does NOT write
//! `.anvil/first-run`. `anvil welcome` keeps sole ownership of that
//! marker so the two surfaces don't fight for first-run state.

use std::collections::BTreeSet;
use std::io::{IsTerminal, Write as _};
use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::GlobalArgs;
use crate::activation::baseline;
use crate::activation::detect_agents::{self, AgentKind, DetectionEnv, RealDetectionEnv};
use crate::activation::diagnostic::{
    ActivationDiagnostic, ConfigStatus, McpClientId, verify_with_home,
};
use crate::activation::identity;
use crate::commands::{hooks, init};
use crate::registration::{self, WorktreeRegistration};
use crate::services::sample_analyser;

pub mod install;

pub use install::{InstallOutcome, InstallReport, SkipReason};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpInstallPolicy {
    Install,
    Skip,
}

/// Run the orchestration on `root` under `mcp_install_policy` and return the
/// final diagnostic alongside the install report.
///
/// The caller is responsible for rendering — the orchestrator is mute on
/// the activation diagnostic itself so unit tests can assert against the
/// returned struct without parsing stdout. Init's own output (config
/// success copy + first-scan summary) goes to stdout when init runs;
/// re-runs against an existing config produce no init output.
pub(crate) fn run_with_mcp_policy(
    root: &Path,
    global: &GlobalArgs,
    mcp_install_policy: McpInstallPolicy,
    force_all_mcp_clients: bool,
) -> anyhow::Result<(ActivationDiagnostic, InstallReport)> {
    let home = crate::util::user_home_dir();
    let enabled = resolve_enabled_clients(&RealDetectionEnv, force_all_mcp_clients);
    run_with_home_and_policy(root, home.as_deref(), global, mcp_install_policy, &enabled)
}

fn run_with_home_and_policy(
    root: &Path,
    home: Option<&Path>,
    global: &GlobalArgs,
    mcp_install_policy: McpInstallPolicy,
    enabled: &BTreeSet<McpClientId>,
) -> anyhow::Result<(ActivationDiagnostic, InstallReport)> {
    run_with_home_and_registration(
        root,
        home,
        global,
        registration::register_worktree_with_daemon,
        mcp_install_policy,
        enabled,
    )
}

/// Map a detected [`AgentKind`] to its [`McpClientId`], or `None` for
/// agents anvil detects but does not (yet) install an MCP entry for.
fn agent_to_mcp_client(kind: AgentKind) -> Option<McpClientId> {
    match kind {
        AgentKind::ClaudeCode => Some(McpClientId::ClaudeCode),
        AgentKind::Cursor => Some(McpClientId::Cursor),
        // Aider / Windsurf / Codex are detected for the "AI tools
        // detected" summary but have no v1 MCP client impl.
        AgentKind::Aider | AgentKind::Windsurf | AgentKind::Codex => None,
    }
}

/// ACTMO-012: resolve which MCP clients are eligible for a *fresh*
/// install on this host.
///
/// `force_all` (the `--all-mcp-clients` flag or a non-empty
/// `ANVIL_ALL_MCP_CLIENTS`) returns every shipping client, preserving the
/// pre-ACTMO-012 "wire both editors" behaviour for power users who want
/// each editor pre-configured. Otherwise the set is the editors actually
/// detected on this host (binary on PATH / pre-existing editor state),
/// so `anvil start` never writes `~/.cursor/mcp.json` for an editor the
/// user does not have. Editors with an existing anvil entry are still
/// managed by the install path regardless of this set — see
/// `install_for_clients`.
fn resolve_enabled_clients(env: &dyn DetectionEnv, force_all: bool) -> BTreeSet<McpClientId> {
    // Read `ANVIL_ALL_MCP_CLIENTS` through the injected `DetectionEnv`
    // (presence-based, like `ANVIL_NO_MCP`: any non-empty value opts in)
    // so unit tests stay hermetic — `RealDetectionEnv::env` reads the
    // process environment in production; stubs return `None`.
    let env_opt_in = env
        .env("ANVIL_ALL_MCP_CLIENTS")
        .is_some_and(|value| !value.is_empty());
    if force_all || env_opt_in {
        return crate::activation::mcp_client::all_client_ids();
    }
    detect_agents::detect_all(env)
        .detected
        .iter()
        .filter_map(|a| agent_to_mcp_client(a.kind))
        .collect()
}

#[allow(clippy::too_many_lines)]
fn run_with_home_and_registration(
    root: &Path,
    home: Option<&Path>,
    global: &GlobalArgs,
    register_worktree: impl FnOnce(&Path) -> WorktreeRegistration,
    mcp_install_policy: McpInstallPolicy,
    enabled: &BTreeSet<McpClientId>,
) -> anyhow::Result<(ActivationDiagnostic, InstallReport)> {
    // DISTRIB-006 (ADR-060): under a non-default ANVIL_HOME without
    // `--touch-project-state`, activation runs in a read-only posture — it still
    // verifies, installs MCP entries into the candidate's own home, and produces
    // a diagnostic, but it does NOT seed durable per-project state into the real
    // repo (`.anvilrc`, `anvil/project-id`, `.gitattributes`, GitHub workflows,
    // baseline). These are state the production binary reads; an unreleased
    // candidate must not write them silently. On an already-activated repo every
    // one of these is a write-if-absent no-op anyway, so the gate only changes
    // behaviour on a fresh repo — exactly where silent seeding would be wrong.
    let project_writes_gated = crate::install_root::project_writes_gated();
    if project_writes_gated {
        eprintln!(
            "anvil: ANVIL_HOME override active without --touch-project-state — \
             activation runs read-only; project-id, .gitattributes, workflows, and \
             baseline will not be written to this project. Pass --touch-project-state \
             to persist."
        );
    }

    // Step 1 — write `.anvilrc` if absent.
    let initial = verify_with_home(root, home);
    if matches!(initial.config, ConfigStatus::Absent) && !project_writes_gated {
        let args = init::InitArgs { force: false };
        init::run_in(&args, global, root).context("init step of `anvil start` failed")?;
    }

    // Step 1a — establish project identity (MLP-001 / A7.2).
    //
    // Writes `anvil/project-id` (UUID v7) if absent. Idempotent on
    // re-run. This is the foundation for the v1 multi-layer protection
    // architecture — every witness line, every cross-machine federation,
    // every fork relationship anchors on this UUID.
    //
    // Failures here MUST NOT propagate (orchestrator pattern). The
    // identity file is a future-architecture-positioning aid for the
    // current release; without it, existing protection paths (MCP,
    // daemon, watch) still work unchanged.
    //
    // Council C-3 / C-9: surface the failure to the user. The
    // `tracing::warn!` alone is invisible at default log levels. We
    // also emit a single noise-disciplined eprintln! so the user can
    // see something went wrong, AND attach the structured `path`
    // field for log consumers.
    let project_id_path = identity::project_id_path(root);
    if !project_writes_gated
        && let Err(e) = identity::ensure_project_id(root, env!("CARGO_PKG_VERSION"))
    {
        tracing::warn!(
            error = %e,
            path = %project_id_path.display(),
            "orchestrator: failed to establish anvil/project-id; continuing without",
        );
        eprintln!(
            "anvil: could not write {} ({e}); future MLP features will be unavailable",
            project_id_path.display()
        );
    }

    // Step 1a-b — pre-position `.gitattributes` for v1 witness chain
    // (council C-7 / Pragmatic Finding 6 / spec §5.1).
    //
    // MLP-002 (witness chain) hard-depends on `merge=union -text` for
    // `anvil/witnessed.ndjson` and the manifest. Adding the attribute
    // line at adoption time means MLP-002 can ship without forcing a
    // separate `.gitattributes` migration. Idempotent — only appends
    // if the line is missing. Failures non-propagating, same pattern
    // as identity.
    if !project_writes_gated && let Err(e) = ensure_witness_gitattributes(root) {
        tracing::warn!(
            error = %e,
            "orchestrator: failed to update .gitattributes for witness chain; continuing without",
        );
    }

    let interactive = is_interactive(global);

    // Step 1a-c — offer GitHub Actions workflow installation
    // (MLP2-043 / MLP2-053).
    //
    // GitHub Actions workflows change repo behaviour and may consume
    // customer CI minutes. Interactive activation presents a pre-ticked
    // list so Enter accepts the recommended defaults and Space opts out;
    // non-interactive activation skips them instead of writing silently.
    // Writes remain write-if-absent so re-running activation never
    // clobbers operator edits.
    if !project_writes_gated && let Err(e) = ensure_github_actions_workflows(root, interactive) {
        tracing::warn!(
            error = %e,
            "orchestrator: failed to install GitHub Actions workflows; continuing without",
        );
        eprintln!("anvil: could not install GitHub Actions workflows ({e}); continuing");
    }

    // Step 1a-d — install ADR-038 commit/push hook coverage as part of the
    // MCP-optional activation spine (ACTMO-005). Hook install is durable
    // project state, so it follows the same gated-write posture as the rest of
    // activation. Failure is non-fatal: MCP and daemon-backed save-time
    // validation can still run, and the operator gets an explicit warning.
    if !project_writes_gated && let Err(e) = hooks::install_activation_hooks_silent(root) {
        tracing::warn!(
            error = %e,
            "orchestrator: failed to install activation git hooks; continuing without",
        );
        eprintln!("anvil: could not install git hooks ({e}); continuing");
    }

    // Step 1b — write `.anvil/baseline.json` if absent (LAUNCH-010).
    // The baseline captures the set of antipattern + secret findings
    // present at first activation so future scans (post-LAUNCH-010
    // PRs across watch / check) can surface only NEW findings. We
    // write it only when absent — this is the activation-time
    // snapshot; subsequent `anvil start` runs are idempotent.
    //
    // Failures here MUST NOT propagate. The baseline is a future-
    // change-tracking aid, not a blocker for activation. A failed
    // write logs and continues; the diagnostic's
    // `baseline_present == false` is the honest signal.
    // DISTRIB-006 (ADR-060): the activation baseline write is part of the gated
    // read-only posture above — skipped under a non-default ANVIL_HOME without
    // `--touch-project-state` so a candidate cannot seed a real project's
    // baseline. `baseline_present == false` stays the honest signal.
    if !project_writes_gated
        && !baseline::baseline_exists(root)
        && let Some(scan) = sample_analyser::run_baseline_scan(root)
    {
        let new_baseline = baseline::build_baseline(&scan.warnings, &scan.secrets);
        if let Err(e) = baseline::write_baseline(root, &new_baseline) {
            tracing::warn!(
                error = %e,
                "orchestrator: failed to write activation baseline; continuing without",
            );
        }
    }

    // ACTMO-016 (ADR-094 decision 4): only register cwd when it is a
    // registerable Git worktree. Outside one (a bare repo, inside `.git`, or
    // not a repo at all) `anvil start` stays honest — it does not register a
    // junk session keyed to e.g. $HOME; the daemon is still ensured by the
    // caller, and `start.rs` surfaces the "no worktree registered" guidance.
    match registration::registerable_worktree(root) {
        Err(reason) => {
            tracing::info!(
                error = %reason,
                "orchestrator: cwd is not a registerable worktree; daemon ensured, cwd not registered",
            );
        }
        Ok(_) => match register_worktree(root) {
            WorktreeRegistration::Registered | WorktreeRegistration::Refreshed => {}
            WorktreeRegistration::DaemonUnavailable => {
                tracing::debug!(
                    "orchestrator: daemon unavailable for activation worktree registration; continuing",
                );
            }
            WorktreeRegistration::Fenced(message) | WorktreeRegistration::CapExceeded(message) => {
                tracing::warn!(
                    error = %message,
                    "orchestrator: activation worktree registration refused; continuing",
                );
            }
            WorktreeRegistration::Rejected(error) => {
                tracing::warn!(
                    error = %error,
                    "orchestrator: activation worktree registration rejected; continuing",
                );
            }
        },
    }

    // Step 2 — install MCP entries for the user-selected (or auto-
    // selected) clients. The install module handles drift, picker UX,
    // and atomic writes; failures are folded into the report rather
    // than propagated, so the orchestrator always returns a final
    // diagnostic the user can act on.
    let install_report = match mcp_install_policy {
        McpInstallPolicy::Skip => InstallReport::default(),
        McpInstallPolicy::Install => match std::env::current_exe() {
            Ok(exe) => {
                let fresh = crate::activation::mcp_client::AnvilEntry::local_stdio(exe);
                install::install_for_clients(root, home, &fresh, interactive, enabled)
            }
            Err(e) => {
                // current_exe failed — verify_with_home will also report
                // last_error, so we don't shadow that signal. Skip install
                // entirely.
                tracing::warn!(
                    error = %e,
                    "orchestrator: could not resolve current_exe; MCP install skipped",
                );
                InstallReport::default()
            }
        },
    };

    // Step 3 — final probe. The diagnostic absorbs the install side
    // effects (e.g. tiers should now read `RestartRequired` for the
    // clients we just wrote) so the caller can render a single source
    // of truth.
    let mut diagnostic = verify_with_home(root, home);

    // Surface every install failure on the diagnostic so
    // `protection_state()` collapses to `Error` and JSON consumers
    // see all simultaneous failures, not just the first one.
    if let Some(err) = install_report.aggregated_failure() {
        diagnostic.last_error = Some(format!("MCP install failed: {err}"));
    }

    Ok((diagnostic, install_report))
}

/// Pre-position `.gitattributes` lines for the v1 witness chain
/// (council C-7 / spec §5.1).
///
/// Adds `merge=union -text` for `anvil/witnessed.ndjson` and the
/// manifest if not already present. Idempotent — searches for the
/// exact line before appending so re-running `anvil start` doesn't
/// duplicate. Creates `.gitattributes` if it doesn't exist.
///
/// This is foundation for MLP-002. The orchestrator writes the
/// attribute at adoption time so when MLP-002 lands, parallel
/// branches' witness writes naturally union-merge instead of
/// producing conflicts.
fn ensure_witness_gitattributes(root: &Path) -> std::io::Result<()> {
    // Per spec §5.1 + ADR-037 §D-3, the active witness file lives at
    // `anvil/witness/active.ndjson` (not the deprecated top-level
    // `anvil/witnessed.ndjson` shorthand that appeared in early drafts).
    // Pre-position both the active file and the manifest with
    // `merge=union -text` so MLP-002 lands without requiring a separate
    // `.gitattributes` migration.
    const WITNESS_LINES: &[&str] = &[
        "anvil/witness/active.ndjson merge=union -text",
        "anvil/witness/manifest/chain.ndjson merge=union -text",
    ];

    let path = root.join(".gitattributes");
    let existing = if path.exists() {
        std::fs::read_to_string(&path)?
    } else {
        String::new()
    };

    let mut to_append = String::new();
    for line in WITNESS_LINES {
        if !existing
            .lines()
            .any(|existing_line| existing_line.trim() == *line)
        {
            if to_append.is_empty() && !existing.is_empty() && !existing.ends_with('\n') {
                to_append.push('\n');
            }
            to_append.push_str(line);
            to_append.push('\n');
        }
    }

    if to_append.is_empty() {
        return Ok(()); // Idempotent — nothing to do.
    }

    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    f.write_all(to_append.as_bytes())?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum WorkflowTemplate {
    PrValidation,
    Audit,
}

impl WorkflowTemplate {
    fn target_path(self, root: &Path) -> PathBuf {
        let workflows_dir = root.join(".github").join("workflows");
        match self {
            Self::PrValidation => workflows_dir.join("anvil.yml"),
            Self::Audit => workflows_dir.join("anvil-audit.yml"),
        }
    }

    fn label(self, root: &Path) -> String {
        let target = workflow_display_path(root, &self.target_path(root));
        match self {
            Self::PrValidation => format!("PR validation ({target}) [pull_request]"),
            Self::Audit => format!("Nightly audit ({target}) [schedule]"),
        }
    }

    fn contents(self) -> &'static str {
        match self {
            Self::PrValidation => crate::commands::anvil_action::anvil_workflow_template(),
            Self::Audit => crate::commands::audit_chain::audit_workflow_template(),
        }
    }
}

impl std::fmt::Display for WorkflowTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PrValidation => f.write_str("PR validation"),
            Self::Audit => f.write_str("Nightly audit"),
        }
    }
}

fn workflow_display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Offer GitHub Actions workflow installation.
///
/// Interactive sessions show a pre-selected list of absent workflow files. A
/// plain Enter accepts the default set; toggling entries off opts out. In
/// non-interactive sessions we skip entirely so customer repos are not modified
/// without an operator seeing the list.
fn ensure_github_actions_workflows(
    root: &Path,
    interactive: bool,
) -> std::io::Result<Vec<PathBuf>> {
    let candidates = pending_workflows(root);
    if candidates.is_empty() || !interactive {
        return Ok(Vec::new());
    }

    let selected = show_workflow_picker(root, &candidates)?;
    let written = install_selected_workflows(root, &selected)?;
    for path in &written {
        eprintln!(
            "anvil: installed GitHub Actions workflow {}",
            workflow_display_path(root, path),
        );
    }
    Ok(written)
}

fn pending_workflows(root: &Path) -> Vec<WorkflowTemplate> {
    [WorkflowTemplate::PrValidation, WorkflowTemplate::Audit]
        .into_iter()
        .filter(|workflow| !workflow.target_path(root).exists())
        .collect()
}

fn show_workflow_picker(
    root: &Path,
    candidates: &[WorkflowTemplate],
) -> std::io::Result<Vec<WorkflowTemplate>> {
    use demand::{DemandOption, MultiSelect};

    let mut picker = MultiSelect::new("Install or enable GitHub Actions workflows?")
        .description("Selected workflows are written only if absent. Space toggles; Enter accepts.")
        .filterable(false)
        .min(0)
        .max(candidates.len());

    for workflow in candidates {
        let label = workflow.label(root);
        picker = picker.option(DemandOption::new(*workflow).label(&label).selected(true));
    }

    let _raw_guard = WorkflowRawModeCleanupGuard;
    match picker.run() {
        Ok(workflows) => Ok(workflows),
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

struct WorkflowRawModeCleanupGuard;
impl Drop for WorkflowRawModeCleanupGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// Drop selected GitHub Actions workflow templates into `.github/workflows/`.
///
/// Write-if-absent semantics — once a file exists we never touch it, so an
/// operator who edits triggers or swaps the install step keeps their changes
/// across re-runs of `anvil start` / `anvil baseline`. The
/// `.github/workflows/` parent is created if missing.
///
/// Errors propagate to the caller so the orchestrator can decide
/// whether to log + continue. Operators without write access to `.github/`
/// should not have activation hard-fail on this step.
fn install_selected_workflows(
    root: &Path,
    selected: &[WorkflowTemplate],
) -> std::io::Result<Vec<PathBuf>> {
    let workflows_dir = root.join(".github").join("workflows");
    let mut written = Vec::new();
    let mut workflows_dir_created = false;

    for workflow in selected {
        let target = workflow.target_path(root);
        if existing_workflow_target(&target)? {
            continue; // Idempotent — never clobber an existing file.
        }
        refuse_workflow_parent_symlinks(root)?;
        if !workflows_dir_created {
            std::fs::create_dir_all(&workflows_dir)?;
            refuse_workflow_parent_symlinks(root)?;
            workflows_dir_created = true;
        }
        if existing_workflow_target(&target)? {
            continue;
        }
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        file.write_all(workflow.contents().as_bytes())?;
        written.push(target);
    }
    Ok(written)
}

fn existing_workflow_target(target: &Path) -> std::io::Result<bool> {
    match std::fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "refusing to write workflow through symlink: {}",
                target.display()
            ),
        )),
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

fn refuse_workflow_parent_symlinks(root: &Path) -> std::io::Result<()> {
    for path in [root.join(".github"), root.join(".github/workflows")] {
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "refusing to write workflow through symlink: {}",
                        path.display()
                    ),
                ));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Decide whether to surface the interactive picker. We require:
/// - not `--json` (defensive: in practice `commands/start.rs` short-
///   circuits to read-only verify on `--json` so the orchestrator
///   never runs in that mode, but the gate stays here so any future
///   caller of `run_with_home` under `--json` cannot accidentally
///   prompt)
/// - not `--no-tui` (explicit user opt-out)
/// - stdin is a TTY (`demand` reads keystrokes from stdin)
/// - stderr is a TTY (`demand` renders the prompt to stderr; piping
///   stderr to a file would render the prompt invisibly while still
///   consuming keystrokes)
/// - not running under a known non-interactive shell context
///   (`CI=true`, `GIT_DIR` set, `ANVIL_NO_PROMPT`, etc. — see
///   [`crate::is_non_interactive_env`])
///
/// Council remediation: previously checked `stdout.is_terminal()`, which
/// misclassified `anvil start | tee log.txt` (auto-installs silently)
/// and `echo "" | anvil start` (picker hangs on closed stdin). The new
/// check matches the convention in `commands/tutorial.rs:41` and the
/// auth-prompt gate in `main.rs:413`.
fn is_interactive(global: &GlobalArgs) -> bool {
    !global.json
        && !global.no_tui
        && std::io::stdin().is_terminal()
        && std::io::stderr().is_terminal()
        && !crate::is_non_interactive_env()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GlobalArgs;
    use crate::activation::diagnostic::McpClientId;
    use tempfile::TempDir;

    fn default_global() -> GlobalArgs {
        // `--no-tui` forces the non-interactive auto-install branch
        // so unit tests don't try to summon a picker.
        GlobalArgs {
            no_tui: true,
            ..Default::default()
        }
    }

    /// Minimal [`DetectionEnv`] stub for the enabled-client resolution
    /// tests — only `has_binary` matters here.
    struct StubDetectionEnv {
        binaries: std::collections::HashSet<String>,
    }
    impl StubDetectionEnv {
        fn with_binary(name: &str) -> Self {
            let mut binaries = std::collections::HashSet::new();
            binaries.insert(name.to_string());
            Self { binaries }
        }
    }
    impl DetectionEnv for StubDetectionEnv {
        fn has_binary(&self, name: &str) -> bool {
            self.binaries.contains(name)
        }
        fn path_exists(&self, _path: &str) -> bool {
            false
        }
        fn env(&self, _name: &str) -> Option<String> {
            None
        }
        fn home_dir(&self) -> Option<String> {
            None
        }
    }

    #[test]
    fn agent_to_mcp_client_maps_only_clients_with_impls() {
        assert_eq!(
            agent_to_mcp_client(AgentKind::ClaudeCode),
            Some(McpClientId::ClaudeCode)
        );
        assert_eq!(
            agent_to_mcp_client(AgentKind::Cursor),
            Some(McpClientId::Cursor)
        );
        // Detected for the "AI tools detected" line, but no v1 MCP impl.
        assert_eq!(agent_to_mcp_client(AgentKind::Aider), None);
        assert_eq!(agent_to_mcp_client(AgentKind::Windsurf), None);
        assert_eq!(agent_to_mcp_client(AgentKind::Codex), None);
    }

    #[test]
    fn resolve_enabled_clients_force_all_returns_every_client() {
        // `force_all` short-circuits before any detection or env read.
        let env = StubDetectionEnv {
            binaries: std::collections::HashSet::new(),
        };
        let enabled = resolve_enabled_clients(&env, /* force_all */ true);
        assert_eq!(enabled, crate::activation::mcp_client::all_client_ids());
    }

    #[test]
    fn resolve_enabled_clients_scopes_to_detected_editor() {
        // Hermetic: the stub's `env()` returns `None`, so the
        // `ANVIL_ALL_MCP_CLIENTS` opt-in never fires regardless of the
        // real process environment. Only the editor whose binary is on
        // PATH is enabled.
        let env = StubDetectionEnv::with_binary("claude");
        let enabled = resolve_enabled_clients(&env, /* force_all */ false);
        assert!(enabled.contains(&McpClientId::ClaudeCode));
        assert!(
            !enabled.contains(&McpClientId::Cursor),
            "undetected Cursor must not be enabled"
        );
    }

    fn run_in_isolated(
        root: &Path,
        home: &Path,
        global: &GlobalArgs,
    ) -> (ActivationDiagnostic, InstallReport) {
        run_with_home_for_test(root, Some(home), global).expect("orchestrator should succeed")
    }

    fn run_with_home_for_test(
        root: &Path,
        home: Option<&Path>,
        global: &GlobalArgs,
    ) -> anyhow::Result<(ActivationDiagnostic, InstallReport)> {
        run_with_home_and_registration(
            root,
            home,
            global,
            |_| WorktreeRegistration::DaemonUnavailable,
            McpInstallPolicy::Install,
            &crate::activation::mcp_client::all_client_ids(),
        )
    }

    #[test]
    fn orchestrator_writes_config_when_absent() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        // Pre-condition: no config.
        let pre = verify_with_home(dir.path(), Some(home.path()));
        assert!(matches!(pre.config, ConfigStatus::Absent));

        let (diag, _report) = run_in_isolated(dir.path(), home.path(), &global);

        assert!(
            dir.path().join(".anvilrc").exists(),
            "orchestrator should write .anvilrc on a fresh repo"
        );
        assert!(matches!(diag.config, ConfigStatus::Valid));
    }

    #[test]
    fn orchestrator_writes_project_id_when_absent() {
        // A7.2 / MLP-001 — ensure orchestrator establishes project
        // identity on first run. The file is foundation for v1
        // multi-layer protection but does not affect current-release
        // behaviour beyond writing the tracked file.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        run_in_isolated(dir.path(), home.path(), &global);

        let project_id_path = dir.path().join("anvil/project-id");
        assert!(
            project_id_path.exists(),
            "orchestrator should write anvil/project-id on a fresh repo"
        );
        let contents = std::fs::read_to_string(&project_id_path).unwrap();
        assert!(contents.contains("project_uuid:"));
    }

    #[test]
    fn orchestrator_project_id_is_idempotent() {
        // A7.2 / MLP-001 — re-running anvil start must not mint a new
        // UUID; the existing project-id is the stable identity.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        run_in_isolated(dir.path(), home.path(), &global);
        let first = std::fs::read_to_string(dir.path().join("anvil/project-id")).unwrap();

        run_in_isolated(dir.path(), home.path(), &global);
        let second = std::fs::read_to_string(dir.path().join("anvil/project-id")).unwrap();

        assert_eq!(
            first, second,
            "orchestrator must not rewrite anvil/project-id on re-run"
        );
    }

    #[test]
    fn orchestrator_writes_witness_gitattributes() {
        // Council C-7 — `.gitattributes` must include the witness file
        // merge=union lines so MLP-002 can ship without forcing a
        // separate migration.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        run_in_isolated(dir.path(), home.path(), &global);

        let attrs = std::fs::read_to_string(dir.path().join(".gitattributes")).unwrap();
        assert!(
            attrs.contains("anvil/witness/active.ndjson merge=union -text"),
            ".gitattributes must include witness file merge=union line. got:\n{attrs}"
        );
        assert!(
            attrs.contains("anvil/witness/manifest/chain.ndjson merge=union -text"),
            ".gitattributes must include manifest merge=union line. got:\n{attrs}"
        );
    }

    #[test]
    fn orchestrator_gitattributes_is_idempotent() {
        // Re-running `anvil start` must not duplicate lines in
        // .gitattributes.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        run_in_isolated(dir.path(), home.path(), &global);
        let first = std::fs::read_to_string(dir.path().join(".gitattributes")).unwrap();

        run_in_isolated(dir.path(), home.path(), &global);
        let second = std::fs::read_to_string(dir.path().join(".gitattributes")).unwrap();

        assert_eq!(
            first, second,
            "orchestrator must not duplicate .gitattributes lines on re-run"
        );
    }

    #[test]
    fn orchestrator_gitattributes_preserves_user_lines() {
        // Pre-existing `.gitattributes` content must survive; we
        // append, never overwrite.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        std::fs::write(dir.path().join(".gitattributes"), "*.txt text\n").unwrap();

        run_in_isolated(dir.path(), home.path(), &global);

        let attrs = std::fs::read_to_string(dir.path().join(".gitattributes")).unwrap();
        assert!(
            attrs.starts_with("*.txt text\n"),
            "user's existing .gitattributes lines must be preserved"
        );
        assert!(attrs.contains("anvil/witness/active.ndjson merge=union -text"));
    }

    #[test]
    fn orchestrator_installs_managed_git_hooks_when_repo_present() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        init_git_repo(dir.path());

        run_in_isolated(dir.path(), home.path(), &global);

        for hook in ["pre-commit", "pre-push"] {
            let path = dir.path().join(".git/hooks").join(hook);
            let raw = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read installed hook {}: {e}", path.display()));
            assert!(
                raw.contains("# @anvil-managed"),
                "{hook} must be installed as an anvil-managed hook; got:\n{raw}",
            );
        }
    }

    /// MLP2-038 — end-to-end proof that the `merge=union -text` line the
    /// orchestrator writes actually causes git to union-merge witness file
    /// appends from parallel branches without producing conflict markers.
    /// The existing tests at this site cover the **file content** the
    /// orchestrator writes; this one drives a real `git merge` to confirm
    /// the validation requirement in `plans/modules/multilayer-protection-v2.aps.md`
    /// (Group H, MLP2-038) holds end-to-end.
    #[test]
    fn orchestrator_gitattributes_unions_parallel_witness_appends() {
        use std::process::Command;

        // Skip when the test runner has no `git` on PATH; the rest of the
        // workspace requires git for normal operation so a missing binary
        // means the host is mis-configured rather than a CI signal we want
        // to fail on.
        let git_probe = Command::new("git").arg("--version").output();
        if !matches!(&git_probe, Ok(out) if out.status.success()) {
            eprintln!("skipping MLP2-038 union-merge test: `git --version` failed ({git_probe:?})");
            return;
        }

        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // Run the same `.gitattributes` writer the orchestrator runs.
        // Going through the full `run_with_home` would also write
        // `.anvilrc`, `anvil/project-id`, `.anvil/baseline.json`, etc.,
        // which we'd then have to stage; the union-merge property is a
        // property of the `.gitattributes` content only, so we call the
        // narrow writer directly.
        ensure_witness_gitattributes(root).expect("write .gitattributes");

        // Bring up a minimal commit-capable git repo. The committer
        // identity is local-only so the test can't accidentally pick up
        // the dev's real `user.name` / `user.email`.
        let run_git = |args: &[&str]| -> std::process::Output {
            Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .output()
                .unwrap_or_else(|e| panic!("git {args:?} failed to spawn: {e}"))
        };
        let must = |args: &[&str]| {
            let out = run_git(args);
            assert!(
                out.status.success(),
                "git {args:?} failed: stdout={:?} stderr={:?}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        };

        // `-b main` keeps the default-branch name deterministic across
        // host git defaults (`master` on older git, `main` on newer).
        must(&["init", "-q", "-b", "main"]);
        must(&["config", "user.email", "mlp2-038@anvil.test"]);
        must(&["config", "user.name", "MLP2-038 fixture"]);
        // Disable signing so the test passes on hosts with commit.gpgsign=true.
        must(&["config", "commit.gpgsign", "false"]);

        // Stage the .gitattributes plus an empty witness file as the
        // shared ancestor commit.
        let witness_rel = "anvil/witness/active.ndjson";
        let witness_path = root.join(witness_rel);
        std::fs::create_dir_all(witness_path.parent().unwrap()).unwrap();
        std::fs::write(&witness_path, "").unwrap();
        must(&["add", ".gitattributes", witness_rel]);
        must(&["commit", "-q", "-m", "base"]);

        // Branch A: append a row attributed to attribution "a".
        must(&["checkout", "-q", "-b", "branch-a"]);
        append_line(&witness_path, "{\"who\":\"a\",\"n\":1}\n");
        must(&["commit", "-q", "-am", "branch-a row"]);

        // Branch B (off main, not off A): append a different row.
        must(&["checkout", "-q", "main"]);
        must(&["checkout", "-q", "-b", "branch-b"]);
        append_line(&witness_path, "{\"who\":\"b\",\"n\":2}\n");
        must(&["commit", "-q", "-am", "branch-b row"]);

        // Merge A then B back into main. Each merge exercises the
        // `merge=union -text` attribute on a real divergent append.
        must(&["checkout", "-q", "main"]);
        must(&["merge", "-q", "--no-edit", "branch-a"]);
        let merge_out = run_git(&["merge", "--no-edit", "branch-b"]);
        assert!(
            merge_out.status.success(),
            "merge of branch-b into main must succeed under `merge=union -text`. \
             stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&merge_out.stdout),
            String::from_utf8_lossy(&merge_out.stderr)
        );

        let merged = std::fs::read_to_string(&witness_path).unwrap();
        assert!(
            !merged.contains("<<<<<<<")
                && !merged.contains("=======")
                && !merged.contains(">>>>>>>"),
            "merged witness file must not contain conflict markers:\n{merged}"
        );
        assert!(
            merged.contains("{\"who\":\"a\",\"n\":1}"),
            "merged file must retain branch-a row:\n{merged}"
        );
        assert!(
            merged.contains("{\"who\":\"b\",\"n\":2}"),
            "merged file must retain branch-b row:\n{merged}"
        );
    }

    fn append_line(path: &Path, line: &str) {
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .unwrap_or_else(|e| panic!("open {} for append: {e}", path.display()));
        f.write_all(line.as_bytes())
            .unwrap_or_else(|e| panic!("append to {}: {e}", path.display()));
    }

    fn init_git_repo(root: &Path) {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["init", "-q", "-b", "main"])
            .output()
            .unwrap_or_else(|e| panic!("git init failed to spawn: {e}"));
        assert!(
            out.status.success(),
            "git init failed: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
    }

    #[test]
    fn orchestrator_continues_when_project_id_write_fails() {
        // A7.2 — failures to establish project-id MUST NOT propagate.
        // Simulate by pre-creating `anvil/project-id` as a directory,
        // which makes both write-as-file and parse impossible. The
        // orchestrator should log a warning and finish successfully so
        // the user still gets MCP install + diagnostic.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        std::fs::create_dir_all(dir.path().join("anvil/project-id")).unwrap();

        let result = run_with_home_for_test(dir.path(), Some(home.path()), &global);
        assert!(
            result.is_ok(),
            "orchestrator must not fail when anvil/project-id is unwritable: {result:?}"
        );
    }

    #[test]
    fn orchestrator_attempts_daemon_worktree_registration() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();
        let called = std::cell::Cell::new(false);

        // ACTMO-016: registration is gated on a registerable Git worktree, so
        // the closure only fires when cwd is one. Make the dir a real worktree.
        git_init(dir.path());

        run_with_home_and_registration(
            dir.path(),
            Some(home.path()),
            &global,
            |_root| {
                called.set(true);
                WorktreeRegistration::Registered
            },
            McpInstallPolicy::Install,
            &crate::activation::mcp_client::all_client_ids(),
        )
        .expect("orchestrator should continue after registration");

        assert!(
            called.get(),
            "orchestrator must register the activation worktree"
        );
    }

    /// ACTMO-016: outside a registerable worktree, the orchestrator does not
    /// invoke the registration closure (no junk session keyed to e.g. $HOME),
    /// yet still completes successfully.
    #[test]
    fn orchestrator_skips_registration_outside_a_worktree() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();
        let called = std::cell::Cell::new(false);

        run_with_home_and_registration(
            dir.path(),
            Some(home.path()),
            &global,
            |_root| {
                called.set(true);
                WorktreeRegistration::Registered
            },
            McpInstallPolicy::Install,
            &crate::activation::mcp_client::all_client_ids(),
        )
        .expect("orchestrator should continue without registering");

        assert!(!called.get(), "a non-worktree dir must not be registered");
    }

    /// Initialise a minimal Git worktree so the registerable-worktree gate
    /// (ACTMO-016) treats the directory as registerable.
    fn git_init(dir: &Path) {
        for args in [
            ["init", "-q"].as_slice(),
            ["config", "user.email", "t@t"].as_slice(),
            ["config", "user.name", "t"].as_slice(),
        ] {
            let ok = std::process::Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(args)
                .output()
                .expect("run git")
                .status
                .success();
            assert!(ok, "git {args:?} failed");
        }
    }

    #[test]
    fn orchestrator_skips_init_when_config_valid() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        // Run once to write config + install.
        run_in_isolated(dir.path(), home.path(), &global);
        let mtime_before = std::fs::metadata(dir.path().join(".anvilrc"))
            .unwrap()
            .modified()
            .unwrap();

        // Idempotency check: file mtime must not change on a re-run.
        // Sleep a beat to make any rewrite detectable across filesystems
        // with one-second mtime granularity (e.g. HFS+).
        std::thread::sleep(std::time::Duration::from_millis(1100));
        run_in_isolated(dir.path(), home.path(), &global);
        let mtime_after = std::fs::metadata(dir.path().join(".anvilrc"))
            .unwrap()
            .modified()
            .unwrap();

        assert_eq!(
            mtime_before, mtime_after,
            "orchestrator must not rewrite .anvilrc on idempotent re-run"
        );
    }

    #[test]
    fn orchestrator_auto_installs_in_no_tui_mode() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        let (_diag, report) = run_in_isolated(dir.path(), home.path(), &global);

        // Both clients should have been auto-installed at home scope.
        assert!(
            matches!(
                report.per_client.get(&McpClientId::Cursor),
                Some(InstallOutcome::Installed { .. })
            ),
            "Cursor must auto-install in --no-tui mode"
        );
        assert!(
            matches!(
                report.per_client.get(&McpClientId::ClaudeCode),
                Some(InstallOutcome::Installed { .. })
            ),
            "Claude Code must auto-install in --no-tui mode"
        );
        assert!(home.path().join(".cursor/mcp.json").exists());
        assert!(home.path().join(".claude.json").exists());
    }

    #[test]
    fn orchestrator_skips_mcp_install_when_policy_skip() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        let (_diag, report) = run_with_home_and_registration(
            dir.path(),
            Some(home.path()),
            &global,
            |_| WorktreeRegistration::DaemonUnavailable,
            McpInstallPolicy::Skip,
            &crate::activation::mcp_client::all_client_ids(),
        )
        .expect("orchestrator should succeed with MCP install skipped");

        assert!(
            report.per_client.is_empty(),
            "skip policy must not report per-client MCP writes"
        );
        assert!(
            !home.path().join(".cursor/mcp.json").exists(),
            "skip policy must not write Cursor MCP config"
        );
        assert!(
            !home.path().join(".claude.json").exists(),
            "skip policy must not write Claude Code MCP config"
        );
    }

    #[test]
    fn orchestrator_diagnostic_reflects_post_install_state() {
        // After install, the diagnostic re-probe must show the
        // RestartRequired tier — that's the whole point of the install
        // step. Without it, `anvil start` would land on NeedsAction
        // even though we just wired both clients.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        let (diag, _report) = run_in_isolated(dir.path(), home.path(), &global);
        let cursor_tier = diag.mcp.get(&McpClientId::Cursor).map(|r| r.tier);
        let claude_tier = diag.mcp.get(&McpClientId::ClaudeCode).map(|r| r.tier);

        assert_eq!(
            cursor_tier,
            Some(crate::activation::diagnostic::McpTier::RestartRequired),
            "Cursor tier should advance to RestartRequired after install"
        );
        assert_eq!(
            claude_tier,
            Some(crate::activation::diagnostic::McpTier::RestartRequired),
            "Claude Code tier should advance to RestartRequired after install"
        );
    }

    #[test]
    fn orchestrator_returns_ready_restart_required_after_install() {
        // The composed flow's headline outcome: a fresh repo with no
        // editor wired ends at `ReadyRestartRequired` once both clients
        // have an entry on disk. (Was `NeedsAction` before LAUNCH-009
        // part 2 — that test moved into `*_skips_install_when_*` cases.)
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        let (diag, _report) = run_in_isolated(dir.path(), home.path(), &global);
        let state = diag.protection_state();
        assert!(
            matches!(
                state,
                crate::activation::state::ProtectionState::ReadyRestartRequired
                    | crate::activation::state::ProtectionState::Unsupported
            ),
            "post-install fresh repo should land at ready_restart_required \
             (or unsupported when no covered languages), got {state:?}"
        );
    }

    #[test]
    fn orchestrator_writes_baseline_when_absent() {
        // LAUNCH-010: a fresh repo with at least one analysable file
        // must end with `.anvil/baseline.json` on disk, populated
        // with whatever findings the activation scan saw.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        // Plant a `.ts` file so the antipattern scanner has something
        // to scan; even if it produces zero findings, the baseline
        // writer still runs and writes an empty fingerprint set.
        std::fs::write(
            dir.path().join("hello.ts"),
            "export const greet = () => console.log('hi');\n",
        )
        .unwrap();

        let baseline_path = crate::activation::baseline::baseline_path(dir.path());
        assert!(
            !baseline_path.exists(),
            "precondition: baseline must be absent on a fresh repo"
        );

        let (diag, _) = run_in_isolated(dir.path(), home.path(), &global);

        assert!(
            baseline_path.exists(),
            "orchestrator must write baseline.json on first activation"
        );
        assert!(
            diag.baseline_present,
            "diagnostic must report baseline_present after first activation"
        );
        assert!(
            diag.baseline_summary.is_some(),
            "diagnostic must carry a parsed baseline summary"
        );
    }

    #[test]
    fn orchestrator_baseline_write_is_idempotent() {
        // LAUNCH-010: re-running activation must NOT rewrite an
        // existing baseline. The activation snapshot is captured once
        // — refreshing requires the user to delete the file and re-
        // run start.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        std::fs::write(
            dir.path().join("hello.ts"),
            "export const greet = () => console.log('hi');\n",
        )
        .unwrap();

        run_in_isolated(dir.path(), home.path(), &global);
        let baseline_path = crate::activation::baseline::baseline_path(dir.path());
        let mtime_before = std::fs::metadata(&baseline_path)
            .unwrap()
            .modified()
            .unwrap();

        // Sleep a beat so any rewrite would be detectable on filesystems
        // with one-second mtime granularity (mirrors the existing
        // `orchestrator_skips_init_when_config_valid` pattern).
        std::thread::sleep(std::time::Duration::from_millis(1100));
        run_in_isolated(dir.path(), home.path(), &global);
        let mtime_after = std::fs::metadata(&baseline_path)
            .unwrap()
            .modified()
            .unwrap();

        assert_eq!(
            mtime_before, mtime_after,
            "orchestrator must not rewrite baseline.json on re-run"
        );
    }

    #[test]
    fn orchestrator_records_findings_in_baseline() {
        // LAUNCH-010 spec: a fixture repo with a finding-shaped line
        // must produce a baseline whose total > 0.
        //
        // PR #1293 review fix (Copilot): the test relies on
        // antipattern findings, not secret findings. `@ts-ignore` is
        // AP-004 in the compiled registry and `: any` is AP-003 —
        // both are TS-shape and predate recent registry churn. The
        // earlier comment incorrectly named an "AWS access key"
        // approach; the actual fixture deliberately avoids the
        // secret-scanner allowlist (which captures `EXAMPLE`
        // patterns) so the assertion stays deterministic regardless
        // of allowlist evolution.
        use crate::activation::baseline;

        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        std::fs::write(
            dir.path().join("leak.ts"),
            "// @ts-ignore\nconst x: any = 5;\n",
        )
        .unwrap();

        run_in_isolated(dir.path(), home.path(), &global);
        let b = baseline::read_baseline(dir.path())
            .expect("baseline read must succeed")
            .expect("baseline must be present");
        assert!(
            b.total() > 0,
            "baseline must contain at least one fingerprint, got: {b:?}"
        );
    }

    #[test]
    fn orchestrator_does_not_overwrite_unsafe_drift() {
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        // Pre-populate Cursor with a foreign-tool entry that uses our
        // server name.
        std::fs::create_dir_all(home.path().join(".cursor")).unwrap();
        let cfg = r#"{"mcpServers": {"anvil": {"command": "/bin/bash", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        std::fs::write(home.path().join(".cursor/mcp.json"), cfg).unwrap();
        let bytes_before = std::fs::read(home.path().join(".cursor/mcp.json")).unwrap();

        let (_diag, report) = run_in_isolated(dir.path(), home.path(), &global);

        match report.per_client.get(&McpClientId::Cursor) {
            Some(InstallOutcome::Skipped {
                reason: SkipReason::UnsafeDrift(_),
            }) => {}
            other => panic!("expected Cursor UnsafeDrift skip, got {other:?}"),
        }

        let bytes_after = std::fs::read(home.path().join(".cursor/mcp.json")).unwrap();
        assert_eq!(bytes_before, bytes_after, "must not overwrite UnsafeDrift");
    }

    // ---- MLP2-053: audit-chain workflow installation -------------------

    #[test]
    fn orchestrator_does_not_write_github_actions_without_interactive_consent() {
        // MLP2-043 / MLP2-053 — GitHub Actions workflows change repo
        // behaviour and consume customer CI minutes, so non-interactive
        // activation must never add them silently.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        run_in_isolated(dir.path(), home.path(), &global);

        let action_target = dir.path().join(".github/workflows/anvil.yml");
        let audit_target = dir.path().join(".github/workflows/anvil-audit.yml");
        assert!(
            !action_target.exists(),
            "orchestrator must not write .github/workflows/anvil.yml without consent"
        );
        assert!(
            !audit_target.exists(),
            "orchestrator must not write .github/workflows/anvil-audit.yml without consent"
        );
    }

    #[test]
    fn workflow_install_is_idempotent() {
        // MLP2-053 — re-running activation must not rewrite an existing
        // `.github/workflows/anvil-audit.yml`. Operators are expected to
        // edit the file in-place (e.g. comment out the `schedule` block);
        // we must never clobber that. Asserting content equality (not
        // mtime) lets the test run in microseconds — the sibling
        // `orchestrator_audit_workflow_preserves_user_edits` test
        // proves the same property for a user-edited file; this one
        // pins it for the orchestrator's own template.
        let dir = TempDir::new().unwrap();

        install_selected_workflows(dir.path(), &[WorkflowTemplate::Audit]).unwrap();
        let target = dir.path().join(".github/workflows/anvil-audit.yml");
        let before = std::fs::read_to_string(&target).unwrap();

        install_selected_workflows(dir.path(), &[WorkflowTemplate::Audit]).unwrap();
        let after = std::fs::read_to_string(&target).unwrap();

        assert_eq!(
            before, after,
            "orchestrator must not rewrite anvil-audit.yml on re-run"
        );
    }

    #[test]
    fn workflow_install_writes_selected_templates() {
        let dir = TempDir::new().unwrap();

        let written = install_selected_workflows(
            dir.path(),
            &[WorkflowTemplate::PrValidation, WorkflowTemplate::Audit],
        )
        .unwrap();

        let action_target = dir.path().join(".github/workflows/anvil.yml");
        let audit_target = dir.path().join(".github/workflows/anvil-audit.yml");
        assert_eq!(written, vec![action_target.clone(), audit_target.clone()]);
        assert_eq!(
            std::fs::read_to_string(&action_target).unwrap(),
            crate::commands::anvil_action::anvil_workflow_template(),
        );
        assert_eq!(
            std::fs::read_to_string(&audit_target).unwrap(),
            crate::commands::audit_chain::audit_workflow_template(),
        );
    }

    #[cfg(unix)]
    #[test]
    fn workflow_install_refuses_symlinked_workflows_dir() {
        use std::os::unix::fs::symlink;

        let dir = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".github")).unwrap();
        symlink(outside.path(), dir.path().join(".github/workflows")).unwrap();

        let err = install_selected_workflows(dir.path(), &[WorkflowTemplate::Audit])
            .expect_err("workflow install must reject symlinked workflow directory");

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            !outside.path().join("anvil-audit.yml").exists(),
            "must not write outside repo through a symlinked workflow directory",
        );
    }

    #[cfg(unix)]
    #[test]
    fn workflow_install_refuses_symlinked_target() {
        use std::os::unix::fs::symlink;

        let dir = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        let workflows = dir.path().join(".github/workflows");
        std::fs::create_dir_all(&workflows).unwrap();
        symlink(
            outside.path().join("anvil-audit.yml"),
            workflows.join("anvil-audit.yml"),
        )
        .unwrap();

        let err = install_selected_workflows(dir.path(), &[WorkflowTemplate::Audit])
            .expect_err("workflow install must reject symlinked workflow target");

        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            !outside.path().join("anvil-audit.yml").exists(),
            "must not write outside repo through a symlinked workflow target",
        );
    }

    #[test]
    fn orchestrator_audit_workflow_preserves_user_edits() {
        // MLP2-053 — operators routinely customise the workflow (e.g.
        // bump the schedule, swap the install step). Re-running `anvil
        // start` must leave their edits intact.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let global = default_global();

        let workflows = dir.path().join(".github/workflows");
        std::fs::create_dir_all(&workflows).unwrap();
        let target = workflows.join("anvil-audit.yml");
        let user_content = "# customised by operator\nname: anvil-audit-custom\n";
        std::fs::write(&target, user_content).unwrap();

        run_in_isolated(dir.path(), home.path(), &global);

        let after = std::fs::read_to_string(&target).unwrap();
        assert_eq!(
            after, user_content,
            "orchestrator must not overwrite user-edited anvil-audit.yml"
        );
    }
}
