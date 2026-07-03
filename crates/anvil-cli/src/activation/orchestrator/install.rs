//! MCP install path for the activation orchestrator (LAUNCH-009 part 2).
//!
//! Drives the per-client install step that promotes the diagnostic from
//! `ConfigAbsent` to `RestartRequired` for each client the user picks.
//!
//! ## Two execution modes
//!
//! - **Interactive** (TTY, not `--no-tui`): probe each registered
//!   client, render a [`demand`] `MultiSelect` listing what was found,
//!   pre-selecting `NotPresent` + `SafeDrift` candidates, and let the
//!   user confirm or trim the set. Cancellation (Ctrl-C / `Esc`)
//!   returns an empty selection — the install step becomes a no-op
//!   without aborting the orchestration.
//!
//! - **Non-interactive** (`--no-tui`, no TTY, or CI envs like
//!   `CI=true` / `GIT_DIR` set): auto-install for every `NotPresent`
//!   and `SafeDrift` candidate. No prompt is shown. `UnsafeDrift` is
//!   always skipped with the drift reason recorded in the install
//!   report.
//!
//! `--json` is **not** routed through this module. `anvil start
//! --json` short-circuits to a read-only `activation::verify` probe
//! at `commands/start.rs` so stdout stays a single JSON document
//! (init has its own JSON output that would otherwise concatenate).
//! Users who want a side-effecting `--json` flow run `anvil init
//! --json` and `anvil start --json` separately.
//!
//! ## Drift policy
//!
//! | `DriftClass`          | Interactive default | Non-interactive | Notes                                                           |
//! |-----------------------|---------------------|-----------------|-----------------------------------------------------------------|
//! | `NotPresent`          | pre-selected        | auto-install    | fresh write, no merge needed                                    |
//! | `SafeDrift`           | pre-selected        | auto-install    | rewrite over a recognised anvil entry (likely a version drift)  |
//! | `UpToDate`            | not shown           | skip            | nothing to do                                                   |
//! | `UnsafeDrift`         | not shown           | skip with note  | foreign tool / unknown shape — never overwrite                  |
//!
//! `UnsafeDrift` is hidden from the interactive picker entirely
//! (filtered out before the `MultiSelect` is built). The install
//! gate also independently refuses `UnsafeDrift` regardless of
//! selection, so even a future picker-API change that surfaced it
//! cannot bypass the foreign-tool guard.
//!
//! ## Editor-detection gate (ACTMO-012)
//!
//! [`install_for_clients`] takes an `enabled` set of clients (the editors
//! actually detected on this host, or every client when
//! `--all-mcp-clients` / `ANVIL_ALL_MCP_CLIENTS` is set). A *fresh*
//! `NotPresent` write only happens for an enabled client — so
//! `anvil start` never creates `~/.cursor/mcp.json` for an editor the
//! user does not have (Matt beta smoke). An existing anvil entry (any
//! drift other than `NotPresent`) is always managed regardless of the
//! `enabled` set, so we never orphan a config anvil previously wrote.
//!
//! ## Atomicity
//!
//! Writes go through `util::atomic_write`, which renames a uniquely-named
//! tempfile in the same directory into place. Editor configs that hold
//! sensitive data (Claude Code's `~/.claude.json` carries auth tokens)
//! are written with mode 0o600 on Unix.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::activation::diagnostic::McpClientId;
use crate::activation::mcp_client::claude_code;
use crate::activation::mcp_client::{
    AnvilEntry, ConfigCandidate, ConfigScope, DriftClass, McpClient, ParsedConfig, all_clients,
};
use crate::util::{atomic_write, refuse_if_parent_is_symlink};

/// Outcome for a single client. Returned per-client in the
/// [`InstallReport`] so the renderer can show "installed Cursor at
/// `~/.cursor/mcp.json`; skipped Claude Code (already up to date)".
#[derive(Debug, Clone)]
pub enum InstallOutcome {
    /// Wrote the entry to disk. `path` is the target file; `drift`
    /// captures whether this was a fresh install or a rewrite.
    Installed { path: PathBuf, drift: DriftClass },
    /// Did not write. `reason` is the "why" the orchestrator surfaces.
    Skipped { reason: SkipReason },
    /// Probe / write failed. `error` is the user-readable cause.
    Failed { error: String },
}

#[derive(Debug, Clone)]
pub enum SkipReason {
    /// User toggled this client off in the picker (or never selected
    /// it in non-interactive mode because of drift class).
    UserDeselected,
    /// ACTMO-012: the editor was not detected on this host (no binary on
    /// PATH, no pre-existing editor state) and has no existing anvil
    /// entry to manage, so anvil did not create a config for an editor
    /// the user may never use. Pass `--all-mcp-clients` (or set
    /// `ANVIL_ALL_MCP_CLIENTS`) to wire every supported client anyway.
    EditorNotDetected,
    /// `DriftClass::UnsafeDrift` — refused to overwrite a foreign /
    /// unrecognised entry. `reason` is the drift classifier's message.
    /// Parse errors at probe time also fold into this variant via the
    /// drift-classifier path in `pick_install_target`, so the install
    /// flow always presents one "skipped — unsafe" face to the user.
    UnsafeDrift(String),
    /// Existing entry already matches what we'd write.
    AlreadyUpToDate,
}

#[derive(Debug, Clone, Default)]
pub struct InstallReport {
    pub per_client: BTreeMap<McpClientId, InstallOutcome>,
    /// CIB-164: whether `anvil start` actually installed anvil-managed
    /// commit + push hooks this run (both files present and marker-tagged).
    /// The first-run `verify:` block reads this instead of a `.git`-exists
    /// heuristic so it never claims L3/L4 hook coverage it did not install.
    /// Defaults to `false` (read-only / write-gated / skipped paths).
    pub hooks_active: bool,
}

impl InstallReport {
    /// Aggregated failure message across every client that failed,
    /// or `None` if the report carries zero failures. Used by the
    /// orchestrator to populate `ActivationDiagnostic::last_error`
    /// so the protection state collapses to `Error` when any install
    /// attempt blew up.
    ///
    /// Every failure is included so a JSON consumer (CI dashboard,
    /// SRE tooling) can see all simultaneous failures, not just the
    /// first one. Council remediation: previous `first_failure()`
    /// silently dropped the second failure when both clients failed
    /// to write.
    pub fn aggregated_failure(&self) -> Option<String> {
        let parts: Vec<String> = self
            .per_client
            .iter()
            .filter_map(|(client, outcome)| match outcome {
                InstallOutcome::Failed { error } => Some(format!("[{client}] {error}")),
                _ => None,
            })
            .collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("; "))
        }
    }
}

/// What the picker (and the auto-install gate) sees for each client.
/// Built once by [`collect_candidates`] and consumed by both the
/// interactive picker and the non-interactive auto-install branch.
#[derive(Debug, Clone)]
pub(crate) struct Candidate {
    pub id: McpClientId,
    /// Where we'd write if the user picks this client. Selected from
    /// the client's `config_paths()` list — first existing path wins;
    /// if nothing exists, we default to **global** scope so the entry
    /// works across all workspaces (matches Cursor / Claude Code's
    /// own conventions and avoids polluting the workspace with editor
    /// state files).
    pub target_path: PathBuf,
    pub drift: DriftClass,
    /// Pre-parsed config for the target file, if it exists. Re-used
    /// during install so we don't read + parse twice.
    pub parsed: Option<ParsedConfig>,
}

/// Drive the install step.
///
/// `interactive` selects the picker vs auto-install branch. Caller
/// passes `false` whenever a TTY isn't available, `--no-tui` was set,
/// `--json` was set, or we're in CI; `true` only when we know the user
/// can actually see the prompt.
///
/// Underlying I/O failures during the per-client write are folded into
/// `InstallOutcome::Failed` so the orchestrator can show the partial
/// result rather than aborting the whole flow. The function is
/// infallible at the top level today; the signature stays available
/// for the future spawn-probe step (LAUNCH-009.5) which can fail
/// before the report is built.
pub fn install_for_clients(
    workspace: &Path,
    home: Option<&Path>,
    fresh: &AnvilEntry,
    interactive: bool,
    enabled: &BTreeSet<McpClientId>,
) -> InstallReport {
    let candidates = collect_candidates(workspace, home, fresh);

    // ACTMO-012: a *fresh* MCP write only happens for an editor we
    // actually detected (binary on PATH / pre-existing editor state),
    // or when the user opted into every client (`enabled` carries the
    // resolved set). An existing anvil entry (any drift other than
    // `NotPresent`) is always managed regardless of detection — we never
    // orphan a config anvil previously wrote, and we still refuse
    // `UnsafeDrift`. This is the gate that stops `anvil start` writing
    // `~/.cursor/mcp.json` for an editor the user never used (Matt beta
    // smoke).
    let offerable =
        |c: &Candidate| enabled.contains(&c.id) || !matches!(c.drift, DriftClass::NotPresent);

    // Trim candidates that have no actionable choice. The picker
    // never offers UpToDate (nothing to do) or UnsafeDrift (refused
    // regardless of selection — see the install gate below), nor a
    // fresh write for an undetected editor. All are surfaced in the
    // post-install human render block instead.
    let mut picker_inputs: Vec<&Candidate> = candidates.iter().collect();
    picker_inputs.retain(|c| {
        offerable(c)
            && !matches!(
                c.drift,
                DriftClass::UpToDate | DriftClass::UnsafeDrift { .. }
            )
    });

    let chosen_ids: Vec<McpClientId> = if interactive && !picker_inputs.is_empty() {
        match show_picker(&picker_inputs) {
            Ok(ids) => ids,
            Err(e) => {
                // Picker I/O fault (TTY went away mid-prompt, etc.).
                // Treat as zero selections rather than aborting — the
                // orchestrator's final verify still runs and the
                // diagnostic captures the partial state.
                tracing::warn!(error = %e, "mcp install: picker failed; treating as zero selection");
                Vec::new()
            }
        }
    } else {
        // Non-interactive / nothing to ask: auto-install
        // NotPresent + SafeDrift.
        picker_inputs
            .iter()
            .filter(|c| {
                matches!(
                    c.drift,
                    DriftClass::NotPresent | DriftClass::SafeDrift { .. }
                )
            })
            .map(|c| c.id)
            .collect()
    };

    let mut per_client = BTreeMap::new();
    // Iterate clients alongside candidates so install_one has the
    // McpClient impl in hand; avoids a third walk of `all_clients()`
    // and the unreachable "this is a bug" error arm.
    for (client, candidate) in all_clients().iter().zip(&candidates) {
        debug_assert_eq!(
            client.id(),
            candidate.id,
            "candidate / registry order drift",
        );
        let outcome = match &candidate.drift {
            DriftClass::UpToDate => {
                if candidate.id == McpClientId::ClaudeCode {
                    best_effort_claude_allow_list(&candidate.target_path);
                }
                tracing::debug!(
                    client = %candidate.id,
                    path = %candidate.target_path.display(),
                    "mcp install: skipped — already up to date",
                );
                InstallOutcome::Skipped {
                    reason: SkipReason::AlreadyUpToDate,
                }
            }
            DriftClass::UnsafeDrift { reason } => {
                tracing::warn!(
                    client = %candidate.id,
                    path = %candidate.target_path.display(),
                    reason = %reason,
                    "mcp install: refusing to overwrite — UnsafeDrift",
                );
                InstallOutcome::Skipped {
                    reason: SkipReason::UnsafeDrift(reason.clone()),
                }
            }
            DriftClass::NotPresent | DriftClass::SafeDrift { .. } => {
                if !offerable(candidate) {
                    // Undetected editor with no existing anvil entry —
                    // do not create a config for an editor the user may
                    // never use (ACTMO-012).
                    tracing::debug!(
                        client = %candidate.id,
                        "mcp install: skipped — editor not detected",
                    );
                    InstallOutcome::Skipped {
                        reason: SkipReason::EditorNotDetected,
                    }
                } else if chosen_ids.contains(&candidate.id) {
                    install_one(*client, candidate, fresh)
                } else {
                    tracing::debug!(
                        client = %candidate.id,
                        "mcp install: skipped — user deselected",
                    );
                    InstallOutcome::Skipped {
                        reason: SkipReason::UserDeselected,
                    }
                }
            }
        };
        per_client.insert(candidate.id, outcome);
    }
    InstallReport {
        per_client,
        // CIB-164: the hook-coverage bool is decided by the orchestrator
        // (which owns the hook-install step), not the MCP install path; it
        // is stamped onto the report there. `install_for_clients` never
        // installs hooks, so the honest default here is `false`.
        hooks_active: false,
    }
}

/// Probe each registered client's config paths and produce one
/// [`Candidate`] per client.
///
/// Walks `config_paths()` in priority order; the first existing path
/// wins. If none exist, the candidate defaults to the **global**
/// (last-priority) path because users almost always want their MCP
/// entry to apply across all workspaces, not just the current one.
pub(crate) fn collect_candidates(
    workspace: &Path,
    home: Option<&Path>,
    fresh: &AnvilEntry,
) -> Vec<Candidate> {
    all_clients()
        .iter()
        .map(|client| pick_install_target(*client, workspace, home, fresh))
        .collect()
}

fn pick_install_target(
    client: &dyn McpClient,
    workspace: &Path,
    home: Option<&Path>,
    fresh: &AnvilEntry,
) -> Candidate {
    // Walk config paths in priority order. We mirror `probe_one`'s
    // semantics so the read path and the install path agree on which
    // scope wins:
    //
    // - **anvil entry present at this scope** (drift != NotPresent):
    //   stop and use this scope. The user is using this scope for
    //   anvil; respect that.
    // - **File exists but no anvil entry** (drift == NotPresent):
    //   keep walking. A workspace `.cursor/mcp.json` containing only
    //   other servers must NOT shadow a valid anvil entry at home —
    //   if we stopped here, we would write a duplicate workspace
    //   entry and orphan the home one.
    // - **Parse error / I/O error**: stop. The file is broken and we
    //   refuse to install over it (UnsafeDrift) — falling through to
    //   home would silently install elsewhere while leaving the
    //   broken workspace file as-is.
    // - **NotFound**: keep walking.
    //
    // Council remediation (kernel MAJOR): previous code stopped on
    // any successful parse, regardless of whether the anvil entry was
    // present. That diverged from `probe_one` and caused duplicate
    // entries when workspace had non-anvil servers and home had a
    // valid anvil entry.
    let paths = client.config_paths(workspace, home);
    let mut absent_fallback: Option<(ConfigCandidate, ParsedConfig)> = None;

    for cand in &paths {
        match std::fs::read_to_string(&cand.path) {
            Ok(raw) => match client.parse(&raw) {
                Ok(parsed) => {
                    let drift = client.classify_drift(&parsed, fresh);
                    if matches!(drift, DriftClass::NotPresent) {
                        // File exists but no anvil entry — keep walking
                        // to a higher-priority scope. Remember the
                        // first NotPresent we saw so we have a real
                        // file to merge into if no scope has anvil.
                        if absent_fallback.is_none() {
                            absent_fallback = Some((cand.clone(), parsed));
                        }
                        continue;
                    }
                    return Candidate {
                        id: client.id(),
                        target_path: cand.path.clone(),
                        drift,
                        parsed: Some(parsed),
                    };
                }
                Err(e) => {
                    tracing::warn!(
                        client = %client.id(),
                        path = %cand.path.display(),
                        error = %e.reason(),
                        "mcp install: parse error during candidate probe; classifying as UnsafeDrift",
                    );
                    return Candidate {
                        id: client.id(),
                        target_path: cand.path.clone(),
                        drift: DriftClass::UnsafeDrift {
                            reason: format!("config file is unparseable: {}", e.reason()),
                        },
                        parsed: None,
                    };
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // No file at this scope — keep walking.
            }
            Err(e) => {
                tracing::warn!(
                    client = %client.id(),
                    path = %cand.path.display(),
                    error = %e,
                    "mcp install: I/O error during candidate probe; classifying as UnsafeDrift",
                );
                return Candidate {
                    id: client.id(),
                    target_path: cand.path.clone(),
                    drift: DriftClass::UnsafeDrift {
                        reason: format!("could not read config: {e}"),
                    },
                    parsed: None,
                };
            }
        }
    }

    // No scope had an anvil entry. Prefer merging into the
    // first-priority existing file we saw (NotPresent fallback);
    // failing that, default to the global scope so the entry applies
    // across all workspaces.
    if let Some((cand, parsed)) = absent_fallback {
        return Candidate {
            id: client.id(),
            target_path: cand.path,
            drift: DriftClass::NotPresent,
            parsed: Some(parsed),
        };
    }
    let target = pick_default_scope(&paths, workspace);
    Candidate {
        id: client.id(),
        target_path: target.path,
        drift: DriftClass::NotPresent,
        parsed: None,
    }
}

fn pick_default_scope(paths: &[ConfigCandidate], workspace: &Path) -> ConfigCandidate {
    // Prefer Global; fall back to Workspace; last-resort synthesise a
    // workspace-relative dotfile so the function is total.
    if let Some(global) = paths.iter().find(|c| c.scope == ConfigScope::Global) {
        return global.clone();
    }
    if let Some(ws) = paths.iter().find(|c| c.scope == ConfigScope::Workspace) {
        return ws.clone();
    }
    ConfigCandidate {
        path: workspace.join(".anvil-mcp-fallback.json"),
        scope: ConfigScope::Workspace,
    }
}

fn install_one(
    client: &dyn McpClient,
    candidate: &Candidate,
    fresh: &AnvilEntry,
) -> InstallOutcome {
    let render_result = match &candidate.parsed {
        Some(parsed) => client.merge_and_render(parsed, fresh),
        None => client.render_new(fresh),
    };
    let body = match render_result {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                client = %candidate.id,
                path = %candidate.target_path.display(),
                error = %e,
                "mcp install: render failed",
            );
            return InstallOutcome::Failed {
                error: format!("render: {e}"),
            };
        }
    };

    if let Some(parent) = candidate.target_path.parent()
        && !parent.as_os_str().is_empty()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        tracing::warn!(
            client = %candidate.id,
            parent = %parent.display(),
            error = %e,
            "mcp install: failed to create parent dir",
        );
        return InstallOutcome::Failed {
            error: format!("create parent dir {}: {e}", parent.display()),
        };
    }

    // Symlink-parent guard (LAUNCH-009.5 council remediation, scoped
    // to the MCP install path). Editor configs in `$HOME` carry
    // sensitive data (`.claude.json` has auth tokens); a symlinked
    // `~/.cursor` or `~/.claude.json` parent would let `tempfile_in`
    // write through the link. The guard is opt-in (see
    // `util::refuse_if_parent_is_symlink` doc) so unrelated
    // `atomic_write` callers (`.anvilrc`, snapshots) keep working
    // inside legitimately-symlinked workspace roots.
    if let Err(e) = refuse_if_parent_is_symlink(&candidate.target_path) {
        tracing::warn!(
            client = %candidate.id,
            path = %candidate.target_path.display(),
            error = %format!("{e:#}"),
            "mcp install: refusing to write — parent is a symlink",
        );
        return InstallOutcome::Failed {
            error: format!("{e:#}"),
        };
    }

    // Council remediation (#14): match `mcp_config.rs` and end the
    // file with a trailing newline so editor "format on save" passes
    // don't flip the file back and forth between forms.
    let mut bytes = body.into_bytes();
    if !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    if let Err(e) = atomic_write(&candidate.target_path, &bytes) {
        tracing::warn!(
            client = %candidate.id,
            path = %candidate.target_path.display(),
            error = %format!("{e:#}"),
            "mcp install: write failed",
        );
        return InstallOutcome::Failed {
            error: format!("write {}: {e:#}", candidate.target_path.display()),
        };
    }

    if candidate.id == McpClientId::ClaudeCode {
        best_effort_claude_allow_list(&candidate.target_path);
    }

    tracing::info!(
        client = %candidate.id,
        path = %candidate.target_path.display(),
        drift = ?candidate.drift,
        "mcp install: installed",
    );
    InstallOutcome::Installed {
        path: candidate.target_path.clone(),
        drift: candidate.drift.clone(),
    }
}

/// Merge the `mcp__anvil__*` allow rule into Claude Code's `settings.json` as a
/// best-effort convenience (it suppresses per-write approval prompts; it is not
/// load-bearing for protection). A failure here must never become an
/// `InstallOutcome::Failed`: the MCP server entry in `.claude.json` is already
/// on disk and the daemon-backed spine is unaffected, so failing the whole
/// install would flip activation to a misleading `state: error` and mask an
/// otherwise-healthy posture (Council S2). We log and move on.
fn best_effort_claude_allow_list(mcp_config_path: &Path) {
    if let Err(error) = install_claude_allow_list(mcp_config_path) {
        tracing::warn!(
            client = %McpClientId::ClaudeCode,
            error = %error,
            "mcp install: could not merge Claude allow rule (non-fatal); \
             anvil_validate_write may prompt until it is added manually",
        );
    }
}

fn install_claude_allow_list(mcp_config_path: &Path) -> Result<(), String> {
    let settings_path = claude_code::settings_path_for_mcp_config(mcp_config_path);
    let existing = match std::fs::read_to_string(&settings_path) {
        Ok(raw) => Some(raw),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            return Err(format!(
                "read Claude Code settings {}: {e}",
                settings_path.display()
            ));
        }
    };
    let body = claude_code::render_settings_with_anvil_allow(existing.as_deref()).map_err(|e| {
        format!(
            "render Claude Code settings {}: {e}",
            settings_path.display()
        )
    })?;

    let mut bytes = body.into_bytes();
    if !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    if existing
        .as_ref()
        .is_some_and(|raw| raw.as_bytes() == bytes.as_slice())
    {
        return Ok(());
    }

    if let Some(parent) = settings_path.parent()
        && !parent.as_os_str().is_empty()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        return Err(format!(
            "create Claude Code settings parent {}: {e}",
            parent.display()
        ));
    }
    if let Err(e) = refuse_if_parent_is_symlink(&settings_path) {
        return Err(format!("{e:#}"));
    }
    atomic_write(&settings_path, &bytes).map_err(|e| {
        format!(
            "write Claude Code settings {}: {e:#}",
            settings_path.display()
        )
    })
}

/// Render the `demand::MultiSelect` picker and return the chosen ids.
///
/// Caller filters out `UpToDate` (nothing to do) and `UnsafeDrift`
/// (refused regardless of selection) before calling, so the picker
/// only ever offers actionable installs. `NotPresent` and `SafeDrift`
/// are pre-selected so the user can just hit `Enter` for the obvious
/// choice. `UnsafeDrift` outcomes are surfaced in the post-install
/// human render block instead of the picker.
///
/// Returns `Ok(vec![])` if the user dismisses the prompt without
/// selecting anything (Enter on empty).
///
/// Wraps the call in [`RawModeGuard`] — `demand` enables raw mode via
/// `console::Term` and on its happy path restores it, but a panic /
/// SIGINT / interrupt error during render can leak the raw flag. The
/// next interactive prompt in the same shell session (e.g. the
/// `Log in now?` yes/no in `main::prompt_yes_no`) would then hang
/// because the kernel never delivers `\n` to `read_line`.
fn show_picker(candidates: &[&Candidate]) -> std::io::Result<Vec<McpClientId>> {
    use demand::{DemandOption, MultiSelect};

    let mut picker = MultiSelect::new("Install anvil MCP for these clients?")
        .description("anvil writes a single mcp entry per file. Existing keys are preserved.")
        .filterable(false)
        .min(0)
        .max(candidates.len());

    for candidate in candidates {
        let label = format_picker_label(candidate);
        let preselected = matches!(
            candidate.drift,
            DriftClass::NotPresent | DriftClass::SafeDrift { .. }
        );
        picker = picker.option(
            DemandOption::new(candidate.id)
                .label(&label)
                .selected(preselected),
        );
    }

    let _raw_guard = RawModeGuard;
    match picker.run() {
        Ok(ids) => Ok(ids),
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
            // Ctrl-C: treat as "select nothing", continue the
            // orchestration. The user can re-run `anvil start`.
            Ok(Vec::new())
        }
        Err(e) => Err(e),
    }
}

/// Drop-guard that defensively disables crossterm raw mode whenever it
/// goes out of scope. Used to wrap interactive TUI calls so a panic /
/// `?`-unwind / abnormal exit can't leave the user's terminal stuck
/// in raw mode, which silently breaks every later line-buffered prompt
/// in the same shell session.
struct RawModeGuard;
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

// Picker labels MUST stay on a single terminal row. `demand` 2.0.0's
// `MultiSelect::reposition_and_write` clears `output.lines().count()`
// rows on each redraw — that counts `\n` boundaries, NOT terminal-
// wrapped visual rows. If a label wraps, every keystroke leaves the
// wrapped overflow on screen and stacks a fresh copy of the prompt
// below it. Keep the label short: tilde-shorten the path and reduce
// the drift reason to a one-or-two-word tag. The post-install render
// block (`render::render_install_block`) and `ANVIL_LOG=info` carry
// the full from→to detail.
fn format_picker_label(candidate: &Candidate) -> String {
    let display = candidate.id.display_name();
    let path = display_path_with_home_tilde(&candidate.target_path);
    let state = match &candidate.drift {
        DriftClass::NotPresent => "not configured",
        DriftClass::UpToDate => "already configured",
        DriftClass::SafeDrift { .. } => "update — version drift",
        DriftClass::UnsafeDrift { .. } => "UNSAFE",
    };
    format!("{display}  ({path})  [{state}]")
}

fn display_path_with_home_tilde(path: &Path) -> String {
    display_path_with_home(path, crate::util::user_home_dir().as_deref())
}

fn display_path_with_home(path: &Path, home: Option<&Path>) -> String {
    // Use `Path::strip_prefix` (component-aware) rather than string-prefix
    // matching: a string prefix would mishandle the case where home is
    // `/home/al` and the path starts with `/home/alice/...`, rendering
    // `~ice/...`. The Path-based form requires a full component match.
    if let Some(home) = home
        && let Ok(rest) = path.strip_prefix(home)
    {
        if rest.as_os_str().is_empty() {
            return "~".to_string();
        }
        return format!("~/{}", rest.display());
    }
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activation::mcp_client::AnvilEntry;
    use std::fs;
    use tempfile::TempDir;

    fn fresh() -> AnvilEntry {
        AnvilEntry::local_stdio(PathBuf::from("/usr/local/bin/anvil"))
    }

    /// Every shipping client enabled — equivalent to `--all-mcp-clients`.
    /// These mechanics tests exercise the install path itself, so they
    /// opt into all clients; the editor-detection gate has its own
    /// dedicated tests below.
    fn all_enabled() -> BTreeSet<McpClientId> {
        crate::activation::mcp_client::all_client_ids()
    }

    /// No client enabled — simulates a host where neither editor is
    /// detected (and `--all-mcp-clients` was not passed).
    fn none_enabled() -> BTreeSet<McpClientId> {
        BTreeSet::new()
    }

    fn report_outcome(report: &InstallReport, id: McpClientId) -> &InstallOutcome {
        report
            .per_client
            .get(&id)
            .unwrap_or_else(|| panic!("missing outcome for {id:?}"))
    }

    #[test]
    fn fresh_repo_auto_installs_to_global_scope() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            /* interactive */ false,
            &all_enabled(),
        );

        // Both clients should have written to the home scope (global).
        let cursor_path = home.path().join(".cursor/mcp.json");
        let claude_path = home.path().join(".claude.json");

        match report_outcome(&report, McpClientId::Cursor) {
            InstallOutcome::Installed { path, drift } => {
                assert_eq!(path, &cursor_path);
                assert!(matches!(drift, DriftClass::NotPresent));
            }
            other => panic!("expected Cursor Installed, got {other:?}"),
        }
        match report_outcome(&report, McpClientId::ClaudeCode) {
            InstallOutcome::Installed { path, drift } => {
                assert_eq!(path, &claude_path);
                assert!(matches!(drift, DriftClass::NotPresent));
            }
            other => panic!("expected ClaudeCode Installed, got {other:?}"),
        }
        assert!(cursor_path.exists());
        assert!(claude_path.exists());
        // Re-parse and check the entry actually landed.
        let cursor_raw = fs::read_to_string(&cursor_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&cursor_raw).unwrap();
        assert!(v.get("mcpServers").unwrap().get("anvil").is_some());
        let claude_settings_path = home.path().join(".claude/settings.json");
        let claude_settings_raw = fs::read_to_string(&claude_settings_path).unwrap();
        let claude_settings: serde_json::Value =
            serde_json::from_str(&claude_settings_raw).unwrap();
        assert!(
            claude_settings
                .get("permissions")
                .and_then(|p| p.get("allow"))
                .and_then(serde_json::Value::as_array)
                .unwrap()
                .contains(&serde_json::json!("mcp__anvil__*")),
            "Claude install must allow the anvil MCP tool namespace"
        );
    }

    #[test]
    fn claude_install_preserves_existing_settings_permissions() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let settings_path = home.path().join(".claude/settings.json");
        fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
        fs::write(
            &settings_path,
            r#"{"permissions": {"allow": ["Bash(pnpm test *)"], "deny": ["Read(.env)"]}, "theme": "dark"}"#,
        )
        .unwrap();

        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );
        assert!(matches!(
            report_outcome(&report, McpClientId::ClaudeCode),
            InstallOutcome::Installed { .. }
        ));

        let raw = fs::read_to_string(&settings_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let allow = v
            .get("permissions")
            .and_then(|p| p.get("allow"))
            .and_then(serde_json::Value::as_array)
            .unwrap();
        assert!(allow.contains(&serde_json::json!("Bash(pnpm test *)")));
        assert!(allow.contains(&serde_json::json!("mcp__anvil__*")));
        assert_eq!(
            v.get("permissions").unwrap().get("deny"),
            Some(&serde_json::json!(["Read(.env)"]))
        );
        assert_eq!(v.get("theme"), Some(&serde_json::json!("dark")));
    }

    #[test]
    fn claude_up_to_date_mcp_repairs_missing_allow_rule() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let claude_cfg = r#"{"mcpServers": {"anvil": {"type": "stdio", "command": "/usr/local/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        fs::write(home.path().join(".claude.json"), claude_cfg).unwrap();

        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );
        assert!(matches!(
            report_outcome(&report, McpClientId::ClaudeCode),
            InstallOutcome::Skipped {
                reason: SkipReason::AlreadyUpToDate
            }
        ));

        let raw = fs::read_to_string(home.path().join(".claude/settings.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert!(
            v.get("permissions")
                .and_then(|p| p.get("allow"))
                .and_then(serde_json::Value::as_array)
                .unwrap()
                .contains(&serde_json::json!("mcp__anvil__*"))
        );
    }

    #[test]
    fn claude_allow_list_failure_is_non_fatal_when_mcp_is_up_to_date() {
        // Council S2: a malformed settings.json (here a non-object root) must not
        // turn an otherwise up-to-date MCP install into a Failed outcome and a
        // misleading state: error. The MCP entry is correct; the allow-list is a
        // convenience that degrades to a warning.
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let claude_cfg = r#"{"mcpServers": {"anvil": {"type": "stdio", "command": "/usr/local/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        fs::write(home.path().join(".claude.json"), claude_cfg).unwrap();
        // A JSON array root is not an object → render returns BadRoot.
        fs::create_dir_all(home.path().join(".claude")).unwrap();
        fs::write(home.path().join(".claude/settings.json"), "[1, 2, 3]").unwrap();

        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );

        assert!(matches!(
            report_outcome(&report, McpClientId::ClaudeCode),
            InstallOutcome::Skipped {
                reason: SkipReason::AlreadyUpToDate
            }
        ));
        assert!(
            report.aggregated_failure().is_none(),
            "allow-list failure must not surface as an aggregated install failure"
        );
    }

    #[test]
    fn already_up_to_date_is_skipped_not_rewritten() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        // Pre-populate ~/.cursor/mcp.json with the exact entry we'd
        // install.
        fs::create_dir_all(home.path().join(".cursor")).unwrap();
        let cfg = r#"{"mcpServers": {"anvil": {"command": "/usr/local/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        let cursor_path = home.path().join(".cursor/mcp.json");
        fs::write(&cursor_path, cfg).unwrap();
        let mtime_before = fs::metadata(&cursor_path).unwrap().modified().unwrap();

        // Sleep across mtime granularity so any rewrite would be
        // detectable.
        std::thread::sleep(std::time::Duration::from_millis(1100));

        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );

        match report_outcome(&report, McpClientId::Cursor) {
            InstallOutcome::Skipped {
                reason: SkipReason::AlreadyUpToDate,
            } => {}
            other => panic!("expected AlreadyUpToDate, got {other:?}"),
        }

        let mtime_after = fs::metadata(&cursor_path).unwrap().modified().unwrap();
        assert_eq!(
            mtime_before, mtime_after,
            "must not rewrite up-to-date file"
        );
    }

    #[test]
    fn unsafe_drift_is_skipped_with_reason() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        // Foreign command using our key — must not be overwritten.
        fs::create_dir_all(home.path().join(".cursor")).unwrap();
        let cfg = r#"{"mcpServers": {"anvil": {"command": "/bin/bash", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        let cursor_path = home.path().join(".cursor/mcp.json");
        fs::write(&cursor_path, cfg).unwrap();
        let bytes_before = fs::read(&cursor_path).unwrap();

        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );

        match report_outcome(&report, McpClientId::Cursor) {
            InstallOutcome::Skipped {
                reason: SkipReason::UnsafeDrift(reason),
            } => {
                assert!(reason.contains("/bin/bash"));
            }
            other => panic!("expected UnsafeDrift skip, got {other:?}"),
        }

        let bytes_after = fs::read(&cursor_path).unwrap();
        assert_eq!(bytes_before, bytes_after, "UnsafeDrift must not overwrite");
    }

    #[test]
    fn malformed_config_skipped_via_unsafe_drift_reason() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        fs::create_dir_all(home.path().join(".cursor")).unwrap();
        let cursor_path = home.path().join(".cursor/mcp.json");
        fs::write(&cursor_path, "{not json").unwrap();
        let bytes_before = fs::read(&cursor_path).unwrap();

        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );

        match report_outcome(&report, McpClientId::Cursor) {
            InstallOutcome::Skipped {
                reason: SkipReason::UnsafeDrift(_),
            } => {}
            other => panic!("expected unparseable to skip, got {other:?}"),
        }
        let bytes_after = fs::read(&cursor_path).unwrap();
        assert_eq!(bytes_before, bytes_after, "must not write over broken file");
    }

    #[cfg(unix)]
    #[test]
    fn install_refuses_when_target_parent_is_a_symlink() {
        // LAUNCH-009.5 council remediation: a symlinked `~/.cursor`
        // (or any parent of the target file) would let `tempfile_in`
        // write through the symlink. The install path opts in to
        // `refuse_if_parent_is_symlink`; this test exercises the
        // opt-in end-to-end so a future refactor that drops the call
        // surfaces immediately.
        use std::os::unix::fs::symlink;

        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        // Create a real dir somewhere unrelated, then symlink
        // ~/.cursor → that dir. Without the guard, install would
        // happily write into the symlinked target.
        let real = TempDir::new().unwrap();
        let real_cursor_dir = real.path().join("real-cursor-dir");
        fs::create_dir(&real_cursor_dir).unwrap();
        symlink(&real_cursor_dir, home.path().join(".cursor")).unwrap();

        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );
        match report_outcome(&report, McpClientId::Cursor) {
            InstallOutcome::Failed { error } => {
                assert!(
                    error.contains("symlink"),
                    "Failed message should mention symlink: {error}"
                );
            }
            other => panic!("expected Failed for symlinked parent, got {other:?}"),
        }
        // The real dir must NOT have been written to.
        assert!(
            !real_cursor_dir.join("mcp.json").exists(),
            "no file should have been written through the symlink"
        );
    }

    #[test]
    fn safe_drift_is_rewritten_on_auto_install() {
        // Existing entry has anvil-shaped command at a different path —
        // SafeDrift, eligible for auto-install rewrite.
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        fs::create_dir_all(home.path().join(".cursor")).unwrap();
        let cfg = r#"{"mcpServers": {"anvil": {"command": "/old/path/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        let cursor_path = home.path().join(".cursor/mcp.json");
        fs::write(&cursor_path, cfg).unwrap();

        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );

        match report_outcome(&report, McpClientId::Cursor) {
            InstallOutcome::Installed {
                drift: DriftClass::SafeDrift { .. },
                ..
            } => {}
            other => panic!("expected SafeDrift install, got {other:?}"),
        }

        // Re-read and check the command was updated to the fresh path.
        let raw = fs::read_to_string(&cursor_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let cmd = v
            .get("mcpServers")
            .and_then(|m| m.get("anvil"))
            .and_then(|e| e.get("command"))
            .and_then(|c| c.as_str())
            .unwrap();
        assert_eq!(cmd, "/usr/local/bin/anvil");
    }

    #[test]
    fn install_preserves_unrelated_servers() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        fs::create_dir_all(home.path().join(".cursor")).unwrap();
        let cfg = r#"{"mcpServers": {"other": {"command": "/usr/bin/other", "args": []}}, "topLevelKey": 42}"#;
        let cursor_path = home.path().join(".cursor/mcp.json");
        fs::write(&cursor_path, cfg).unwrap();

        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );
        assert!(matches!(
            report_outcome(&report, McpClientId::Cursor),
            InstallOutcome::Installed { .. }
        ));

        let raw = fs::read_to_string(&cursor_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let servers = v.get("mcpServers").unwrap();
        assert!(servers.get("anvil").is_some(), "anvil entry written");
        assert!(servers.get("other").is_some(), "other server preserved");
        assert_eq!(v.get("topLevelKey"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn install_is_idempotent() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();

        // First run: writes both clients.
        let r1 = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );
        assert!(matches!(
            report_outcome(&r1, McpClientId::Cursor),
            InstallOutcome::Installed { .. }
        ));

        let cursor_path = home.path().join(".cursor/mcp.json");
        let mtime1 = fs::metadata(&cursor_path).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));

        // Second run: must classify as UpToDate and not rewrite.
        let r2 = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );
        assert!(matches!(
            report_outcome(&r2, McpClientId::Cursor),
            InstallOutcome::Skipped {
                reason: SkipReason::AlreadyUpToDate
            }
        ));

        let mtime2 = fs::metadata(&cursor_path).unwrap().modified().unwrap();
        assert_eq!(mtime1, mtime2, "idempotent re-run must not touch mtime");
    }

    #[test]
    fn workspace_with_other_servers_does_not_shadow_home_anvil_entry() {
        // Council remediation (kernel MAJOR): if workspace
        // `.cursor/mcp.json` has only foreign servers (no anvil
        // entry) and home has a valid anvil entry, the install path
        // must not silently install at workspace scope and orphan
        // the home entry.
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();

        // Workspace: other server, no anvil entry → DriftClass::NotPresent
        std::fs::create_dir_all(ws.path().join(".cursor")).unwrap();
        let ws_cfg = r#"{"mcpServers": {"other": {"command": "/usr/bin/other"}}}"#;
        std::fs::write(ws.path().join(".cursor/mcp.json"), ws_cfg).unwrap();

        // Home: matching anvil entry → DriftClass::UpToDate
        std::fs::create_dir_all(home.path().join(".cursor")).unwrap();
        let home_cfg = r#"{"mcpServers": {"anvil": {"command": "/usr/local/bin/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        std::fs::write(home.path().join(".cursor/mcp.json"), home_cfg).unwrap();

        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );
        // Cursor should land on the home config (UpToDate), not
        // install a duplicate at workspace scope.
        match report_outcome(&report, McpClientId::Cursor) {
            InstallOutcome::Skipped {
                reason: SkipReason::AlreadyUpToDate,
            } => {}
            other => panic!("expected AlreadyUpToDate at home scope, got {other:?}"),
        }
        // Workspace config must not have been touched.
        let ws_after = std::fs::read_to_string(ws.path().join(".cursor/mcp.json")).unwrap();
        assert!(
            !ws_after.contains("anvil"),
            "workspace must not have anvil entry written: {ws_after}"
        );
    }

    #[test]
    fn workspace_with_other_servers_falls_through_and_merges_into_workspace_when_no_home_anvil() {
        // Mirror case: workspace has other servers (NotPresent),
        // home has no config at all. We should keep the workspace
        // scope because that's the first existing file we saw, and
        // merge the anvil entry into it (preserving the other
        // server).
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();

        std::fs::create_dir_all(ws.path().join(".cursor")).unwrap();
        let ws_cfg = r#"{"mcpServers": {"other": {"command": "/usr/bin/other"}}}"#;
        let ws_path = ws.path().join(".cursor/mcp.json");
        std::fs::write(&ws_path, ws_cfg).unwrap();

        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );
        match report_outcome(&report, McpClientId::Cursor) {
            InstallOutcome::Installed { path, .. } => {
                assert_eq!(path, &ws_path, "should install at workspace, not home");
            }
            other => panic!("expected Installed at workspace, got {other:?}"),
        }
        let after = std::fs::read_to_string(&ws_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&after).unwrap();
        let servers = v.get("mcpServers").unwrap();
        assert!(servers.get("anvil").is_some(), "anvil entry merged");
        assert!(servers.get("other").is_some(), "other entry preserved");
        assert!(!home.path().join(".cursor/mcp.json").exists());
    }

    #[test]
    fn workspace_existing_config_keeps_workspace_scope() {
        // If a workspace-local config already exists, install at that
        // scope (don't suddenly switch to global on the same machine).
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        fs::create_dir_all(ws.path().join(".cursor")).unwrap();
        let ws_cfg = r#"{"mcpServers": {"other": {"command": "/usr/bin/other"}}}"#;
        let ws_path = ws.path().join(".cursor/mcp.json");
        fs::write(&ws_path, ws_cfg).unwrap();

        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );
        match report_outcome(&report, McpClientId::Cursor) {
            InstallOutcome::Installed { path, .. } => {
                assert_eq!(path, &ws_path);
            }
            other => panic!("expected Installed at workspace scope, got {other:?}"),
        }
        // Home scope should not have been created.
        assert!(!home.path().join(".cursor/mcp.json").exists());
    }

    #[test]
    fn report_aggregated_failure_returns_none_on_clean_run() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );
        assert!(report.aggregated_failure().is_none());
    }

    #[test]
    fn report_aggregated_failure_includes_every_failed_client() {
        // Council remediation: previously `first_failure` returned only
        // the first error in BTreeMap order, dropping subsequent ones.
        // The aggregator must include every Failed outcome.
        let mut report = InstallReport::default();
        report.per_client.insert(
            McpClientId::Cursor,
            InstallOutcome::Failed {
                error: "cursor write blew up".to_string(),
            },
        );
        report.per_client.insert(
            McpClientId::ClaudeCode,
            InstallOutcome::Failed {
                error: "claude write blew up".to_string(),
            },
        );
        let agg = report.aggregated_failure().unwrap();
        assert!(agg.contains("cursor write blew up"));
        assert!(agg.contains("claude write blew up"));
        assert!(agg.contains("cursor"));
        assert!(agg.contains("claude-code"));
    }

    #[test]
    fn collect_candidates_one_per_client() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let cs = collect_candidates(ws.path(), Some(home.path()), &fresh());
        assert_eq!(cs.len(), 2);
        let ids: Vec<_> = cs.iter().map(|c| c.id).collect();
        assert!(ids.contains(&McpClientId::Cursor));
        assert!(ids.contains(&McpClientId::ClaudeCode));
    }

    #[test]
    fn picker_label_format_includes_state_tag() {
        let candidate = Candidate {
            id: McpClientId::Cursor,
            target_path: PathBuf::from("/home/u/.cursor/mcp.json"),
            drift: DriftClass::NotPresent,
            parsed: None,
        };
        let label = format_picker_label(&candidate);
        assert!(label.contains("Cursor"));
        assert!(label.contains(".cursor/mcp.json"));
        assert!(label.contains("not configured"));
    }

    #[test]
    fn picker_label_does_not_embed_long_drift_paths() {
        // demand 2.0.0's MultiSelect redraw uses `output.lines().count()`,
        // which doesn't account for terminal wrapping. Embedding multi-
        // hundred-character drift paths in the label causes wrap, which
        // makes every keystroke stack a fresh copy of the question on
        // screen. The label MUST stay short. See `format_picker_label`.
        let candidate = Candidate {
            id: McpClientId::Cursor,
            target_path: PathBuf::from("/home/u/.cursor/mcp.json"),
            drift: DriftClass::SafeDrift {
                reason: format!(
                    "version drift: existing command `{a}` differs from fresh `{b}`",
                    a = "/home/u/Projects/src/anvil-001.launch-mcp-install/target/debug/anvil",
                    b = "/home/linuxbrew/.linuxbrew/Cellar/anvil/0.6.0-beta/bin/anvil",
                ),
            },
            parsed: None,
        };
        let label = format_picker_label(&candidate);
        assert!(
            !label.contains("/target/debug/anvil"),
            "label must not embed drift path detail (causes wrap → demand redraw bug). got: {label}"
        );
        assert!(
            !label.contains("/Cellar/anvil/"),
            "label must not embed fresh-binary path (causes wrap → demand redraw bug). got: {label}"
        );
        assert!(
            label.len() <= 80,
            "label must fit a standard 80-col terminal to avoid wrap. got {} chars: {label}",
            label.len()
        );
        assert!(label.contains("update"));
    }

    // --- ACTMO-012: editor-aware install gating ---

    #[test]
    fn undetected_editor_with_no_entry_is_skipped_not_written() {
        // The core Matt beta fix: with no editor detected (and no
        // `--all-mcp-clients`), `anvil start` must not write a fresh MCP
        // config for an editor the user never used.
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &none_enabled(),
        );
        for id in [McpClientId::Cursor, McpClientId::ClaudeCode] {
            match report_outcome(&report, id) {
                InstallOutcome::Skipped {
                    reason: SkipReason::EditorNotDetected,
                } => {}
                other => panic!("expected EditorNotDetected for {id:?}, got {other:?}"),
            }
        }
        assert!(
            !home.path().join(".cursor/mcp.json").exists(),
            "must not write a cursor config for an undetected editor"
        );
        assert!(
            !home.path().join(".claude.json").exists(),
            "must not write a claude config for an undetected editor"
        );
    }

    #[test]
    fn detected_editor_installs_while_others_are_skipped() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let only_claude: BTreeSet<McpClientId> = [McpClientId::ClaudeCode].into_iter().collect();
        let report =
            install_for_clients(ws.path(), Some(home.path()), &fresh(), false, &only_claude);
        assert!(
            matches!(
                report_outcome(&report, McpClientId::ClaudeCode),
                InstallOutcome::Installed { .. }
            ),
            "detected Claude Code must install"
        );
        assert!(
            matches!(
                report_outcome(&report, McpClientId::Cursor),
                InstallOutcome::Skipped {
                    reason: SkipReason::EditorNotDetected
                }
            ),
            "undetected Cursor must be skipped"
        );
        assert!(home.path().join(".claude.json").exists());
        assert!(!home.path().join(".cursor/mcp.json").exists());
    }

    #[test]
    fn existing_anvil_entry_is_managed_even_when_editor_not_enabled() {
        // The gate blocks only *fresh* writes. An existing anvil entry
        // (here SafeDrift — anvil-shaped command at a stale path) must
        // still be rewritten even when the editor is not in the enabled
        // set, so we never orphan a config anvil already manages.
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        fs::create_dir_all(home.path().join(".cursor")).unwrap();
        let cfg = r#"{"mcpServers": {"anvil": {"command": "/old/path/anvil", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        fs::write(home.path().join(".cursor/mcp.json"), cfg).unwrap();

        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &none_enabled(),
        );
        match report_outcome(&report, McpClientId::Cursor) {
            InstallOutcome::Installed {
                drift: DriftClass::SafeDrift { .. },
                ..
            } => {}
            other => panic!("existing anvil entry must still be managed, got {other:?}"),
        }
    }

    #[test]
    fn home_tilde_uses_component_match_not_string_prefix() {
        // Regression for the prefix-shortening pitfall flagged in PR
        // review: `home = "/home/al"` must NOT match `/home/alice/...`.
        // Path-based `strip_prefix` requires a component boundary so the
        // unrelated path is returned unchanged.
        let home = PathBuf::from("/home/al");
        let path = PathBuf::from("/home/alice/.cursor/mcp.json");
        assert_eq!(
            display_path_with_home(&path, Some(&home)),
            "/home/alice/.cursor/mcp.json"
        );

        // Sanity: real prefix collapses to ~/...
        let home = PathBuf::from("/home/alice");
        assert_eq!(
            display_path_with_home(&path, Some(&home)),
            "~/.cursor/mcp.json"
        );

        // Path equal to home renders as `~`.
        assert_eq!(display_path_with_home(&home, Some(&home)), "~");

        // No home → unchanged.
        assert_eq!(
            display_path_with_home(&path, None),
            "/home/alice/.cursor/mcp.json"
        );
    }
}
