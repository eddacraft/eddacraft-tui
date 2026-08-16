//! MCP install step for activation (LAUNCH-009).
//!
//! Interactive: `MultiSelect` of detected clients (unticked by default).
//! Non-interactive / CI: auto-install `NotPresent` and `SafeDrift` only.
//! `UnsafeDrift` never overwritten. Fresh writes limited to enabled/detected
//! editors (ACTMO-012). Atomic writes via `util::atomic_write`.

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
    /// TUI mode deferred the selection to the activation surface; no legacy
    /// picker was shown, so this must not be described as a user deselection
    /// (the activation TUI owns the consent step).
    ConsentDeferredToTui,
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
    /// Daily MCP self-heal is pinned; drifted owned entries are left
    /// unchanged. First-time `NotPresent` installs still proceed.
    HealPinned,
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

/// Consent/selection mode for MCP installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstallConsentMode {
    /// Use the existing `demand` picker.
    DemandPicker,
    /// Headless/plain non-interactive path: install safe defaults.
    AutoInstall,
    /// ACTTUI-002 seam: the activation TUI owns consent, so this layer must not
    /// invoke `demand` and must not silently auto-install while the replacement
    /// widget is still pending (ACTTUI-004).
    DeferToTui,
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
    pub scope: ConfigScope,
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
    let consent_mode = if interactive {
        InstallConsentMode::DemandPicker
    } else {
        InstallConsentMode::AutoInstall
    };
    install_for_clients_with_consent_mode(workspace, home, fresh, consent_mode, enabled)
}

pub(crate) fn install_for_clients_with_consent_mode(
    workspace: &Path,
    home: Option<&Path>,
    fresh: &AnvilEntry,
    consent_mode: InstallConsentMode,
    enabled: &BTreeSet<McpClientId>,
) -> InstallReport {
    install_for_clients_with_selection(
        workspace,
        home,
        fresh,
        enabled,
        InstallSelection::Mode(consent_mode),
    )
}

/// Apply the exact MCP client set returned by the activation TUI.
///
/// Unlike [`install_for_clients`], this path never auto-selects safe drift or
/// fresh candidates: an empty selection is a deliberate no-write decision.
pub(crate) fn install_selected_clients(
    workspace: &Path,
    home: Option<&Path>,
    fresh: &AnvilEntry,
    enabled: &BTreeSet<McpClientId>,
    selected: &BTreeMap<McpClientId, Candidate>,
) -> InstallReport {
    install_for_clients_with_selection(
        workspace,
        home,
        fresh,
        enabled,
        InstallSelection::Explicit(selected),
    )
}

/// Summary of an ensure-only MCP pass (ADR-114 bare path).
#[derive(Debug, Clone)]
pub(crate) struct McpEnsureSummary {
    pub report: InstallReport,
    /// Count of clients with `UpToDate` or `SafeDrift` (already owned).
    pub managed: usize,
    /// Count of `NotPresent` candidates when nothing is managed (recovery).
    pub absent_for_recovery: usize,
}

/// ADR-114 bare ensure: repair already-owned MCP entries only.
///
/// - `SafeDrift` → rewrite in place (ADR-044 ownership)
/// - `UpToDate` → no write
/// - `NotPresent` → never install (recovery is `anvil start`)
/// - `UnsafeDrift` → never overwrite
///
/// Returns the install report plus how many candidates were still
/// `NotPresent` (so the caller can emit one recovery line).
pub(crate) fn ensure_existing_mcp_entries(
    workspace: &Path,
    home: Option<&Path>,
    fresh: &AnvilEntry,
) -> McpEnsureSummary {
    let candidates = collect_candidates(workspace, home, fresh);
    let heal_pinned = crate::commands::mcp_heal::heal_policy().is_pinned();
    let mut selected = BTreeMap::new();
    let mut not_present = 0usize;
    let mut managed = 0usize;
    for candidate in &candidates {
        match &candidate.drift {
            DriftClass::SafeDrift { .. } => {
                managed += 1;
                if heal_pinned {
                    continue;
                }
                selected.insert(candidate.id, candidate.clone());
            }
            DriftClass::UpToDate => {
                managed += 1;
            }
            DriftClass::NotPresent => {
                not_present += 1;
            }
            DriftClass::UnsafeDrift { .. } => {}
        }
    }
    let enabled: BTreeSet<McpClientId> = selected.keys().copied().collect();
    let report = if selected.is_empty() {
        InstallReport::default()
    } else {
        install_selected_clients(workspace, home, fresh, &enabled, &selected)
    };
    // Surface "not installed" only when nothing managed is present.
    let absent_for_recovery = if managed == 0 { not_present } else { 0 };
    McpEnsureSummary {
        report,
        managed,
        absent_for_recovery,
    }
}

#[derive(Debug, Clone, Copy)]
enum InstallSelection<'a> {
    Mode(InstallConsentMode),
    Explicit(&'a BTreeMap<McpClientId, Candidate>),
}

fn install_for_clients_with_selection(
    workspace: &Path,
    home: Option<&Path>,
    fresh: &AnvilEntry,
    enabled: &BTreeSet<McpClientId>,
    selection: InstallSelection<'_>,
) -> InstallReport {
    install_with_selection_and_picker(workspace, home, fresh, enabled, selection, show_picker)
}

/// Seam for the interactive branch: `picker` renders the `demand`
/// `MultiSelect` in production ([`show_picker`]) and is injected by unit
/// tests so the Enter-without-tick and tick-to-install flows are testable
/// without a TTY.
fn install_with_selection_and_picker(
    workspace: &Path,
    home: Option<&Path>,
    fresh: &AnvilEntry,
    enabled: &BTreeSet<McpClientId>,
    selection: InstallSelection<'_>,
    picker: impl FnOnce(&[&Candidate]) -> std::io::Result<Vec<McpClientId>>,
) -> InstallReport {
    let candidates = collect_candidates(workspace, home, fresh);
    let chosen_ids = resolve_chosen_ids(&candidates, enabled, selection, picker);
    let heal_pinned = crate::commands::mcp_heal::heal_policy().is_pinned();

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
        let outcome = install_candidate_outcome(
            *client,
            candidate,
            fresh,
            enabled,
            &chosen_ids,
            selection,
            heal_pinned,
        );
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

fn candidate_offerable(candidate: &Candidate, enabled: &BTreeSet<McpClientId>) -> bool {
    enabled.contains(&candidate.id) || !matches!(candidate.drift, DriftClass::NotPresent)
}

fn resolve_chosen_ids(
    candidates: &[Candidate],
    enabled: &BTreeSet<McpClientId>,
    selection: InstallSelection<'_>,
    picker: impl FnOnce(&[&Candidate]) -> std::io::Result<Vec<McpClientId>>,
) -> Vec<McpClientId> {
    let picker_inputs = candidates
        .iter()
        .filter(|candidate| {
            candidate_offerable(candidate, enabled)
                && !matches!(
                    candidate.drift,
                    DriftClass::UpToDate | DriftClass::UnsafeDrift { .. }
                )
        })
        .collect::<Vec<_>>();

    match selection {
        InstallSelection::Explicit(selected) => selected.keys().copied().collect(),
        InstallSelection::Mode(InstallConsentMode::DemandPicker) if !picker_inputs.is_empty() => {
            picker(&picker_inputs).unwrap_or_else(|error| {
                tracing::warn!(%error, "mcp install: picker failed; treating as zero selection");
                Vec::new()
            })
        }
        InstallSelection::Mode(
            InstallConsentMode::DemandPicker | InstallConsentMode::AutoInstall,
        ) => picker_inputs
            .iter()
            .filter(|candidate| {
                matches!(
                    candidate.drift,
                    DriftClass::NotPresent | DriftClass::SafeDrift { .. }
                )
            })
            .map(|candidate| candidate.id)
            .collect(),
        InstallSelection::Mode(InstallConsentMode::DeferToTui) => Vec::new(),
    }
}

fn install_candidate_outcome(
    client: &dyn McpClient,
    candidate: &Candidate,
    fresh: &AnvilEntry,
    enabled: &BTreeSet<McpClientId>,
    chosen_ids: &[McpClientId],
    selection: InstallSelection<'_>,
    heal_pinned: bool,
) -> InstallOutcome {
    if let InstallSelection::Explicit(expected) = selection
        && let Some(expected) = expected
            .get(&candidate.id)
            .filter(|expected| !same_consent_target(expected, candidate))
    {
        return InstallOutcome::Failed {
            error: format!(
                "consent offer changed before apply: approved {}, now {}; re-run `anvil start`",
                expected.target_path.display(),
                candidate.target_path.display(),
            ),
        };
    }

    if matches!(candidate.drift, DriftClass::SafeDrift { .. }) && heal_pinned {
        return InstallOutcome::Skipped {
            reason: SkipReason::HealPinned,
        };
    }

    match &candidate.drift {
        DriftClass::UpToDate => {
            if candidate.id == McpClientId::ClaudeCode
                && matches!(
                    selection,
                    InstallSelection::Mode(
                        InstallConsentMode::DemandPicker | InstallConsentMode::AutoInstall
                    )
                )
            {
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
        DriftClass::NotPresent | DriftClass::SafeDrift { .. }
            if !candidate_offerable(candidate, enabled) =>
        {
            tracing::debug!(
                client = %candidate.id,
                "mcp install: skipped — editor not detected",
            );
            InstallOutcome::Skipped {
                reason: SkipReason::EditorNotDetected,
            }
        }
        DriftClass::NotPresent | DriftClass::SafeDrift { .. }
            if chosen_ids.contains(&candidate.id) =>
        {
            install_one(
                client,
                candidate,
                fresh,
                !matches!(selection, InstallSelection::Explicit(_)),
            )
        }
        DriftClass::NotPresent | DriftClass::SafeDrift { .. } => InstallOutcome::Skipped {
            reason: unchosen_skip_reason(selection, candidate.id),
        },
    }
}

fn same_consent_target(expected: &Candidate, actual: &Candidate) -> bool {
    expected.id == actual.id
        && expected.target_path == actual.target_path
        && expected.scope == actual.scope
        && expected.drift == actual.drift
}

/// Skip reason for an offerable client that was not chosen for install.
///
/// In TUI mode no legacy picker was shown — consent is owned by the activation
/// surface — so the client is recorded as [`SkipReason::ConsentDeferredToTui`]
/// rather than [`SkipReason::UserDeselected`], which would misrepresent an
/// unshown picker as an explicit user deselection.
fn unchosen_skip_reason(selection: InstallSelection<'_>, client: McpClientId) -> SkipReason {
    if matches!(
        selection,
        InstallSelection::Mode(InstallConsentMode::DeferToTui)
    ) {
        tracing::debug!(%client, "mcp install: skipped — consent deferred to activation TUI");
        SkipReason::ConsentDeferredToTui
    } else {
        tracing::debug!(%client, "mcp install: skipped — user deselected");
        SkipReason::UserDeselected
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
    // Walk config paths like probe_one: return on anvil entry or parse/I/O error
    // (UnsafeDrift — never overwrite broken foreign files); continue on NotFound
    // or parse-ok-without-anvil entry (keeps walking higher-priority scopes).
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
                        scope: cand.scope,
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
                        scope: cand.scope,
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
                    scope: cand.scope,
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
            scope: cand.scope,
            drift: DriftClass::NotPresent,
            parsed: Some(parsed),
        };
    }
    let target = pick_default_scope(&paths, workspace);
    Candidate {
        id: client.id(),
        target_path: target.path,
        scope: target.scope,
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
    refresh_claude_allow_list: bool,
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

    if candidate.id == McpClientId::ClaudeCode && refresh_claude_allow_list {
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

/// Build the `(client, label, selected)` tuples backing the interactive
/// picker.
///
/// Every candidate defaults to `selected = false` (CIB-184): a plain
/// Enter-through selects nothing and writes no editor config, so a hurried
/// operator never silently hands an editor an MCP entry. Ticking a client
/// is the explicit consent — the same posture as the activation TUI
/// consent surface and the workflow picker (CIB-165).
fn mcp_picker_options(candidates: &[&Candidate]) -> Vec<(McpClientId, String, bool)> {
    candidates
        .iter()
        .map(|candidate| (candidate.id, format_picker_label(candidate), false))
        .collect()
}

/// Render the `demand::MultiSelect` picker and return the chosen ids.
///
/// Caller filters out `UpToDate` (nothing to do) and `UnsafeDrift`
/// (refused regardless of selection) before calling, so the picker
/// only ever offers actionable installs. Every offered candidate starts
/// unticked (CIB-184): a plain Enter writes no editor config, and
/// ticking a client is the explicit consent. `UnsafeDrift` outcomes are
/// surfaced in the post-install human render block instead of the
/// picker.
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

    eprintln!("anvil: press Enter to skip MCP client install");
    let mut picker = MultiSelect::new("Install anvil MCP for these clients?")
        .description(
            "Nothing is selected by default — press Enter to skip. Space ticks a client; \
             Enter writes one anvil MCP entry per ticked client (existing keys are preserved).",
        )
        .filterable(false)
        .min(0)
        .max(candidates.len());

    for (id, label, selected) in mcp_picker_options(candidates) {
        picker = picker.option(DemandOption::new(id).label(&label).selected(selected));
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

    /// ONSW-003 / ONSW-006: bare ensure never writes `NotPresent` MCP entries;
    /// auto-install on `start` still would.
    #[test]
    fn ensure_existing_does_not_install_not_present() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let summary = ensure_existing_mcp_entries(ws.path(), Some(home.path()), &fresh());

        assert_eq!(summary.managed, 0, "fresh home has no owned entries");
        assert!(
            summary.absent_for_recovery >= 1,
            "NotPresent clients must surface recovery"
        );
        assert!(
            summary.report.per_client.is_empty(),
            "ensure-only must not produce install outcomes on NotPresent"
        );
        assert!(
            !home.path().join(".cursor/mcp.json").exists(),
            "ensure must not create Cursor MCP config"
        );
        assert!(
            !home.path().join(".claude.json").exists(),
            "ensure must not create Claude MCP config"
        );
    }

    #[test]
    fn ensure_existing_repairs_safe_drift_only() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let cursor_path = home.path().join(".cursor/mcp.json");
        fs::create_dir_all(cursor_path.parent().unwrap()).unwrap();
        // Anvil entry with a different command path → SafeDrift vs fresh().
        fs::write(
            &cursor_path,
            r#"{
  "mcpServers": {
    "anvil": {
      "command": "/old/path/anvil",
      "args": ["mcp", "serve", "--stdio"]
    }
  }
}"#,
        )
        .unwrap();

        let summary = ensure_existing_mcp_entries(ws.path(), Some(home.path()), &fresh());
        assert!(summary.managed >= 1, "SafeDrift counts as managed");
        assert_eq!(
            summary.absent_for_recovery, 0,
            "when managed, do not claim not-installed"
        );
        match summary.report.per_client.get(&McpClientId::Cursor) {
            Some(InstallOutcome::Installed { .. }) => {}
            other => panic!("expected Cursor SafeDrift rewrite, got {other:?}"),
        }
        let raw = fs::read_to_string(&cursor_path).unwrap();
        assert!(
            raw.contains("/usr/local/bin/anvil"),
            "SafeDrift rewrite should land fresh path: {raw}"
        );
    }

    #[test]
    fn preferred_fresh_rewrites_cellar_owned_entry_to_anvil() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let cursor_path = home.path().join(".cursor/mcp.json");
        fs::create_dir_all(cursor_path.parent().unwrap()).unwrap();
        fs::write(
            &cursor_path,
            r#"{
  "mcpServers": {
    "anvil": {
      "command": "/opt/homebrew/Cellar/anvil/0.9.2-beta/bin/anvil",
      "args": ["mcp", "serve", "--stdio"]
    }
  }
}"#,
        )
        .unwrap();

        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &AnvilEntry::preferred_stdio(),
            false,
            &all_enabled(),
        );
        match report_outcome(&report, McpClientId::Cursor) {
            InstallOutcome::Installed {
                drift: DriftClass::SafeDrift { .. },
                ..
            } => {}
            other => panic!("expected Cellar SafeDrift rewrite, got {other:?}"),
        }
        let raw = fs::read_to_string(&cursor_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let cmd = v["mcpServers"]["anvil"]["command"].as_str().unwrap();
        assert_eq!(cmd, "anvil");
        assert!(
            !raw.contains("Cellar"),
            "rewritten entry must not keep a Cellar path: {raw}"
        );
    }

    #[test]
    fn ensure_existing_rewrites_cellar_path_to_preferred_anvil() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let cursor_path = home.path().join(".cursor/mcp.json");
        fs::create_dir_all(cursor_path.parent().unwrap()).unwrap();
        fs::write(
            &cursor_path,
            r#"{
  "mcpServers": {
    "anvil": {
      "command": "/opt/homebrew/Cellar/anvil/0.9.2-beta/bin/anvil",
      "args": ["mcp", "serve", "--stdio"]
    }
  }
}"#,
        )
        .unwrap();

        let summary = ensure_existing_mcp_entries(
            ws.path(),
            Some(home.path()),
            &AnvilEntry::preferred_stdio(),
        );
        match summary.report.per_client.get(&McpClientId::Cursor) {
            Some(InstallOutcome::Installed { .. }) => {}
            other => panic!("expected ensure to rewrite Cellar entry, got {other:?}"),
        }
        let raw = fs::read_to_string(&cursor_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(v["mcpServers"]["anvil"]["command"], "anvil");
    }

    #[test]
    fn preferred_fresh_writes_bare_anvil_not_absolute() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let report = install_for_clients(
            ws.path(),
            Some(home.path()),
            &AnvilEntry::preferred_stdio(),
            false,
            &all_enabled(),
        );
        assert!(matches!(
            report_outcome(&report, McpClientId::Cursor),
            InstallOutcome::Installed { .. }
        ));
        let raw = fs::read_to_string(home.path().join(".cursor/mcp.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(v["mcpServers"]["anvil"]["command"], "anvil");
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
    fn explicit_tui_selection_installs_only_selected_client() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let selected = collect_candidates(ws.path(), Some(home.path()), &fresh())
            .into_iter()
            .filter(|candidate| candidate.id == McpClientId::Cursor)
            .map(|candidate| (candidate.id, candidate))
            .collect();

        let report = install_selected_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            &all_enabled(),
            &selected,
        );

        assert!(home.path().join(".cursor/mcp.json").exists());
        assert!(!home.path().join(".claude.json").exists());
        assert!(matches!(
            report_outcome(&report, McpClientId::Cursor),
            InstallOutcome::Installed { .. }
        ));
        assert!(matches!(
            report_outcome(&report, McpClientId::ClaudeCode),
            InstallOutcome::Skipped {
                reason: SkipReason::UserDeselected
            }
        ));
    }

    #[test]
    fn empty_explicit_tui_selection_writes_nothing() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();

        let report = install_selected_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            &all_enabled(),
            &BTreeMap::new(),
        );

        assert!(!home.path().join(".cursor/mcp.json").exists());
        assert!(!home.path().join(".claude.json").exists());
        assert!(report.per_client.values().all(|outcome| matches!(
            outcome,
            InstallOutcome::Skipped {
                reason: SkipReason::UserDeselected
            }
        )));
    }

    #[test]
    fn empty_explicit_tui_selection_does_not_refresh_claude_allow_list() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );
        let settings = home.path().join(".claude/settings.json");
        fs::remove_file(&settings).unwrap();

        install_selected_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            &all_enabled(),
            &BTreeMap::new(),
        );

        assert!(
            !settings.exists(),
            "an empty explicit selection must not recreate Claude settings"
        );
    }

    #[test]
    fn deferred_tui_probe_never_refreshes_claude_allow_list() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        install_for_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            false,
            &all_enabled(),
        );
        let settings = home.path().join(".claude/settings.json");
        fs::remove_file(&settings).unwrap();

        install_for_clients_with_consent_mode(
            ws.path(),
            Some(home.path()),
            &fresh(),
            InstallConsentMode::DeferToTui,
            &all_enabled(),
        );

        assert!(
            !settings.exists(),
            "the pre-surface deferred probe must not recreate Claude settings"
        );
    }

    #[test]
    fn explicit_tui_selection_rejects_a_scope_change_before_apply() {
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let selected = collect_candidates(ws.path(), Some(home.path()), &fresh())
            .into_iter()
            .filter(|candidate| candidate.id == McpClientId::Cursor)
            .map(|candidate| (candidate.id, candidate))
            .collect();
        let workspace_config = ws.path().join(".cursor/mcp.json");
        fs::create_dir_all(workspace_config.parent().unwrap()).unwrap();
        fs::write(&workspace_config, "{\"mcpServers\":{}}\n").unwrap();

        let report = install_selected_clients(
            ws.path(),
            Some(home.path()),
            &fresh(),
            &all_enabled(),
            &selected,
        );

        assert!(matches!(
            report_outcome(&report, McpClientId::Cursor),
            InstallOutcome::Failed { error }
                if error.contains("consent offer changed before apply")
        ));
        assert!(!home.path().join(".cursor/mcp.json").exists());
        assert_eq!(
            fs::read_to_string(workspace_config).unwrap(),
            "{\"mcpServers\":{}}\n"
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
        assert_eq!(
            cs.len(),
            crate::activation::agent_registry::AgentClientId::all().len()
        );
        let ids: Vec<_> = cs.iter().map(|c| c.id).collect();
        assert!(ids.contains(&McpClientId::Cursor));
        assert!(ids.contains(&McpClientId::ClaudeCode));
        assert!(ids.contains(&McpClientId::Grok));
        assert!(ids.contains(&McpClientId::Codex));
    }

    #[test]
    fn picker_label_format_includes_state_tag() {
        let candidate = Candidate {
            id: McpClientId::Cursor,
            target_path: PathBuf::from("/home/u/.cursor/mcp.json"),
            scope: ConfigScope::Global,
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
            scope: ConfigScope::Global,
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

    // --- CIB-184: demand picker consent posture ---

    #[test]
    fn mcp_picker_options_default_every_candidate_unticked() {
        // CIB-184 — every offerable drift class (NotPresent, SafeDrift)
        // must start unticked, so a hurried Enter-through selects nothing
        // and writes no editor config. Ticking a client is the explicit
        // consent, matching the activation TUI posture and the workflow
        // picker (CIB-165).
        let not_present = Candidate {
            id: McpClientId::Cursor,
            target_path: PathBuf::from("/home/u/.cursor/mcp.json"),
            scope: ConfigScope::Global,
            drift: DriftClass::NotPresent,
            parsed: None,
        };
        let safe_drift = Candidate {
            id: McpClientId::ClaudeCode,
            target_path: PathBuf::from("/home/u/.claude.json"),
            scope: ConfigScope::Global,
            drift: DriftClass::SafeDrift {
                reason: "version drift".to_string(),
            },
            parsed: None,
        };
        let candidates = [&not_present, &safe_drift];

        let options = mcp_picker_options(&candidates);

        // The returned options must correspond 1:1 to the input candidates,
        // in order — otherwise a helper that duplicated or dropped a client
        // could still pass the unticked check below.
        assert_eq!(options.len(), candidates.len());
        for ((id, _label, selected), candidate) in options.iter().zip(candidates.iter()) {
            assert_eq!(
                *id, candidate.id,
                "picker options must match the input candidates 1:1 and in order",
            );
            assert!(
                !selected,
                "picker option for {id} must default to unticked (CIB-184)",
            );
        }
    }

    #[test]
    fn demand_picker_enter_without_tick_writes_nothing() {
        // CIB-184 — Enter with no explicit tick returns an empty selection
        // from the picker; the install step must write no MCP config and no
        // Claude allow-list, and record the offerable clients as
        // user-deselected.
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();

        let report = install_with_selection_and_picker(
            ws.path(),
            Some(home.path()),
            &fresh(),
            &all_enabled(),
            InstallSelection::Mode(InstallConsentMode::DemandPicker),
            |offered| {
                assert!(
                    !offered.is_empty(),
                    "fresh repo must offer candidates to the picker"
                );
                Ok(Vec::new())
            },
        );

        assert!(
            !home.path().join(".cursor/mcp.json").exists(),
            "Enter without a tick must not write the Cursor config"
        );
        assert!(
            !home.path().join(".claude.json").exists(),
            "Enter without a tick must not write the Claude Code config"
        );
        assert!(
            !home.path().join(".claude/settings.json").exists(),
            "Enter without a tick must not refresh the Claude allow-list"
        );
        assert!(report.per_client.values().all(|outcome| matches!(
            outcome,
            InstallOutcome::Skipped {
                reason: SkipReason::UserDeselected
            }
        )));
    }

    #[test]
    fn demand_picker_ticked_selection_installs_only_ticked() {
        // Ticking a client remains the explicit consent: only the ticked
        // client is written, the rest are recorded as user-deselected.
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();

        let report = install_with_selection_and_picker(
            ws.path(),
            Some(home.path()),
            &fresh(),
            &all_enabled(),
            InstallSelection::Mode(InstallConsentMode::DemandPicker),
            |_offered| Ok(vec![McpClientId::Cursor]),
        );

        assert!(home.path().join(".cursor/mcp.json").exists());
        assert!(!home.path().join(".claude.json").exists());
        assert!(matches!(
            report_outcome(&report, McpClientId::Cursor),
            InstallOutcome::Installed { .. }
        ));
        assert!(matches!(
            report_outcome(&report, McpClientId::ClaudeCode),
            InstallOutcome::Skipped {
                reason: SkipReason::UserDeselected
            }
        ));
    }

    #[test]
    fn demand_picker_never_offers_unsafe_drift() {
        // UnsafeDrift stays out of the picker and stays refused even if
        // everything the picker does offer is ticked.
        let ws = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        fs::create_dir_all(home.path().join(".cursor")).unwrap();
        let cfg = r#"{"mcpServers": {"anvil": {"command": "/bin/bash", "args": ["mcp", "serve", "--stdio"], "env": {}}}}"#;
        let cursor_path = home.path().join(".cursor/mcp.json");
        fs::write(&cursor_path, cfg).unwrap();
        let bytes_before = fs::read(&cursor_path).unwrap();

        let report = install_with_selection_and_picker(
            ws.path(),
            Some(home.path()),
            &fresh(),
            &all_enabled(),
            InstallSelection::Mode(InstallConsentMode::DemandPicker),
            |offered| {
                assert!(
                    offered
                        .iter()
                        .all(|candidate| candidate.id != McpClientId::Cursor),
                    "UnsafeDrift candidates must be filtered out of the picker"
                );
                Ok(offered.iter().map(|candidate| candidate.id).collect())
            },
        );

        match report_outcome(&report, McpClientId::Cursor) {
            InstallOutcome::Skipped {
                reason: SkipReason::UnsafeDrift(_),
            } => {}
            other => panic!("expected UnsafeDrift skip, got {other:?}"),
        }
        let bytes_after = fs::read(&cursor_path).unwrap();
        assert_eq!(bytes_before, bytes_after, "UnsafeDrift must not overwrite");
        assert!(matches!(
            report_outcome(&report, McpClientId::ClaudeCode),
            InstallOutcome::Installed { .. }
        ));
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
