//! Rendering for activation diagnostics.
//!
//! Two outputs:
//!
//! - [`render_human`] — block of plain-text lines for terminal display
//!   (used by `anvil status`, `anvil start`, `anvil doctor`).
//! - [`render_json`] — structured JSON block for `--json` output, with a
//!   stable field shape so dashboards and tooling can consume it.
//!
//! Both renderers go through [`headline_for`] (which delegates to
//! [`ProtectionState::headline`]) so the headline copy never
//! drifts between surfaces.

use std::fmt::Write as _;

use serde_json::{Value, json};

use super::diagnostic::{ActivationDiagnostic, McpTier};
use super::mcp_client::DriftClass;
use super::orchestrator::{InstallOutcome, InstallReport, SkipReason};
use super::state::ProtectionState;

/// Render the diagnostic as a multi-line plain-text block ending with
/// a newline. Caller decides whether to print to stdout, stderr, or
/// embed inside another report.
#[allow(
    clippy::too_many_lines,
    reason = "linear render of one diagnostic block; splitting per-section helpers would scatter the printed-line ordering across files without any reuse"
)]
pub fn render_human(d: &ActivationDiagnostic) -> String {
    let state = d.protection_state();
    let mut out = String::new();
    out.push_str("ACTIVATION\n");
    let _ = writeln!(out, "  state: {}", state.label());
    let _ = writeln!(out, "  {}", headline_for(state, d));
    if let Some(explanation) = state_explanation(state, d) {
        let _ = writeln!(out, "  meaning: {explanation}");
    }
    let _ = writeln!(out, "  config: {}", d.config.label());

    if d.mcp.is_empty() {
        out.push_str("  mcp: not detected\n");
    } else {
        out.push_str("  mcp:\n");
        for (client, result) in &d.mcp {
            let _ = writeln!(
                out,
                "    {}: {}",
                client.display_name(),
                result.tier.label()
            );
        }
    }

    let _ = writeln!(out, "  watch: {}", d.watch.label());

    // LAUNCH-011: surface the literal "MCP pre-write validation is
    // not attached" note whenever the diagnostic does not have an
    // already-conveys-the-partial-state signal AND the surrounding
    // state makes the note actionable.
    //
    // Council remediation (round 2): suppress the note when MCP is
    // wired-or-live (`RestartRequired+`). At `RestartRequired`, the
    // headline already says "Ready, restart required — restart your
    // editor or agent so the MCP server attaches", which carries the
    // partial-protection message without needing the note. Adding
    // the note there with no `watch: offered` line just produces
    // orphaned watch-fallback copy that nudges the user toward watch
    // when they should restart. The renamed `mcp_pre_write_wired_or_live`
    // predicate is the honest gate for this.
    //
    // Suppressed in five cases:
    // - MCP at `RestartRequired+` (headline + restart hint already
    //   communicate the partial state; note would orphan watch copy)
    // - `Error` — the surface should report the cause, not hedge
    //   with a fallback advisory the user cannot act on until the
    //   error clears
    // - `Unsupported` — the headline + repair hint already explain
    //   the language coverage gap; the watch fallback would not
    //   produce findings on unsupported files, so the note would
    //   over-claim
    // - `NeedsAction` with `ConfigStatus::Absent` — the user has
    //   not run `anvil init` yet; the actionable next step is init,
    //   not watch fallback. The note distracts from that primary
    //   action.
    // - `Protecting` (also covered by `mcp_pre_write_wired_or_live`,
    //   but the explicit match is kept for readability)
    let suppress_note = d.mcp_pre_write_wired_or_live()
        || matches!(
            state,
            ProtectionState::Error | ProtectionState::Unsupported | ProtectionState::Protecting
        )
        || matches!(
            (state, d.config),
            (
                ProtectionState::NeedsAction,
                super::diagnostic::ConfigStatus::Absent
            )
        );
    if !suppress_note {
        out.push_str(
            "  note: MCP pre-write validation is not attached. \
             Watch mode fallback validates saved file changes only — \
             it cannot intercept MCP tool writes before they happen.\n",
        );
    }

    // LAUNCH-010: render the baseline summary when known. Copy never
    // claims the repo is clean — even a zero-finding baseline only
    // says "no findings tracked yet", because the sample is bounded
    // (~50 files). When secret-shaped findings are present, name
    // them as the headline security signal.
    //
    // PR #1293 review fix (Copilot): the original copy said "future
    // changes are checked", which over-claimed — LAUNCH-010 ships the
    // baseline contract but no command consumes it. `Baseline::contains_*`
    // is dead-code-until-wired, and that consumer was never filed while the
    // LAUNCH module was open; it is now tracked as CIB-127 (wire the
    // finding-baseline into `anvil check` / `audit`). Until it lands the
    // copy states plainly that the baseline is recorded but not yet used to
    // filter scans, rather than implying a diff that nothing performs.
    match (&d.baseline_summary, d.baseline_present) {
        (Some(s), _) if s.secret > 0 => {
            let _ = writeln!(
                out,
                "  baseline: present (current posture — {} findings, baselined as-is — {} antipattern, {} secret-shaped)",
                s.total, s.antipattern, s.secret,
            );
            let _ = writeln!(
                out,
                "  note: secret-shaped findings were captured at activation time; activation does not imply the repo is clean of further secrets."
            );
        }
        (Some(s), _) => {
            let _ = writeln!(
                out,
                "  baseline: present (current posture — {} findings, baselined as-is; recorded for reference — not yet used to filter later scans)",
                s.total,
            );
        }
        (None, true) => {
            // File on disk but unreadable — `last_error` is set with
            // the cause; do not over-claim a count we don't have.
            out.push_str("  baseline: present (unreadable — see last_error)\n");
        }
        (None, false) => {
            out.push_str("  baseline: absent\n");
        }
    }

    // Language profile (LAUNCH-015): surface the per-language
    // breakdown so the user sees coverage tier alongside protection
    // state. Surfaces never claim coverage for `unsupported` rows.
    if !d.language_profile.entries.is_empty() {
        out.push_str("  languages:\n");
        for entry in &d.language_profile.entries {
            let _ = writeln!(
                out,
                "    {} ({} {}): {} — {}",
                entry.name,
                entry.files_seen,
                if entry.files_seen == 1 {
                    "file"
                } else {
                    "files"
                },
                entry.coverage_tier.label(),
                entry.basis,
            );
        }
        if d.language_profile.unclassified_files_seen > 0 {
            let _ = writeln!(
                out,
                "    ({} unclassified file{})",
                d.language_profile.unclassified_files_seen,
                if d.language_profile.unclassified_files_seen == 1 {
                    ""
                } else {
                    "s"
                },
            );
        }
    } else if d.all_languages_unsupported {
        // Defensive: if the profile is empty but `all_languages_unsupported`
        // was set externally (e.g. tests), still surface the gap honestly.
        out.push_str("  languages: all detected languages are unsupported in this release\n");
    }

    if let Some(err) = &d.last_error {
        let _ = writeln!(out, "  last_error: {err}");
    }

    if let Some(hint) = repair_hint(state, d) {
        let _ = writeln!(out, "  next: {hint}");
    }

    out
}

/// Render the diagnostic followed by a per-client install summary.
///
/// Used by `anvil start` to surface what the orchestrator just did
/// (or refused to do) — the diagnostic alone shows the after-state
/// tier per client, but it can't tell the user "we wrote the entry
/// just now" vs "you already had it". The install block fills that
/// gap and is the only place `SkipReason::UnsafeDrift` reasons get
/// surfaced to the user.
pub fn render_human_with_install(d: &ActivationDiagnostic, install: &InstallReport) -> String {
    let mut out = render_human(d);
    if install.per_client.is_empty() {
        return out;
    }
    out.push_str("  install:\n");
    for (client, outcome) in &install.per_client {
        // ACTMO-012: do not mention editors the user does not have. An
        // undetected editor with no anvil entry is silently skipped — the
        // install block reflects the user's actual editor(s), not the
        // hardcoded client list.
        if matches!(
            outcome,
            InstallOutcome::Skipped {
                reason: SkipReason::EditorNotDetected,
            }
        ) {
            continue;
        }
        let line = match outcome {
            InstallOutcome::Installed { path, drift } => {
                let kind = match drift {
                    DriftClass::NotPresent => "fresh",
                    DriftClass::SafeDrift { .. } => "rewrote drifted entry",
                    // Unreachable: the install gate refuses UpToDate /
                    // UnsafeDrift before calling install_one. If a
                    // future refactor lets one through, the surface
                    // text stays informative rather than panicking.
                    DriftClass::UpToDate => "rewrote up-to-date entry",
                    DriftClass::UnsafeDrift { .. } => "rewrote unsafe entry",
                };
                format!("installed at {} ({kind})", path.display())
            }
            InstallOutcome::Skipped {
                reason: SkipReason::AlreadyUpToDate,
            } => "skipped — already up to date".to_string(),
            InstallOutcome::Skipped {
                reason: SkipReason::UserDeselected,
            } => "skipped — not selected".to_string(),
            InstallOutcome::Skipped {
                reason: SkipReason::UnsafeDrift(reason),
            } => format!("skipped — refused to overwrite ({reason})"),
            InstallOutcome::Skipped {
                reason: SkipReason::EditorNotDetected,
            } => {
                // Unreachable: filtered out above via `continue`. Kept
                // for match exhaustiveness; degrades to an informative
                // line if that filter is ever removed.
                "skipped — editor not detected".to_string()
            }
            InstallOutcome::Failed { error } => format!("FAILED — {error}"),
        };
        let _ = writeln!(out, "    {}: {line}", client.display_name());
    }
    out
}

/// MLP2-051g: verbose tier-evidence renderer for `anvil start --verify
/// --why` (and the parallel `anvil status --verify --why`). Returns a
/// multi-line block intended for **stderr**, not stdout — scripted
/// consumers parse stdout, so the verbose copy must not perturb the
/// stdout shape `--verify` already produces.
///
/// Derives entirely from existing [`ActivationDiagnostic`] fields plus
/// the [`super::daemon_evidence::DaemonAttestation`] field MLP2-051f
/// added. **No new fields are read off any other struct** — that's the
/// "no schema growth" gate the parent spec
/// (`plans/specs/2026-05-21-activation-daemon-evidence-wireup.md`
/// §"Council Verdicts" item 9) locked.
///
/// **Security guard (spec §"Info-leak guard"):** stderr is
/// terminal-visible; shoulder-surfing is the threat. The renderer
/// MUST NOT surface:
///
/// - raw [`anvil_intercept::registry::SessionRecord`] fields — `pid`,
///   `pgid`, full `agent_tag` lineage. These aren't reachable from
///   `ActivationDiagnostic` today, so the guard is structural rather
///   than runtime-filtered. A future field that does carry them MUST
///   be elided here explicitly.
/// - arbitrary filesystem paths beyond the workspace root. The
///   diagnostic only reads tier labels (`McpTier`,
///   [`super::daemon_evidence::DaemonAttestation`]), so no path
///   substring reaches the output — verified by the `last_error`
///   pass-through being the only string carrier and that field being
///   set by the same orchestrator that already prints it to stdout.
///
/// The `daemon:` sub-line is only emitted under clients at
/// [`McpTier::RestartHandshakeVerified`] — the only tier where
/// [`super::daemon_evidence::DaemonAttestation`] gates promotion. For
/// clients below that tier the daemon state is irrelevant to the
/// remediation step.
pub fn render_human_verbose(d: &ActivationDiagnostic) -> String {
    let mut out = String::new();
    out.push_str("ACTIVATION (verbose)\n");
    let _ = writeln!(out, "  state: {}", d.protection_state().label());
    let _ = writeln!(out, "  config: {}", d.config.label());

    if d.mcp.is_empty() {
        out.push_str("  mcp: not detected\n");
    } else {
        out.push_str("  mcp:\n");
        for (client, result) in &d.mcp {
            let _ = writeln!(out, "    {}:", client.display_name());
            let _ = writeln!(out, "      tier:      {}", result.tier.label());
            let _ = writeln!(out, "      transport: {}", result.transport.label());
            // Per-client tier-evidence sub-steps. Each line answers
            // "what was found at this layer?" and is derived purely
            // from the tier label — McpProbeResult intentionally does
            // not carry config paths or executable paths today (the
            // spec locks "no new fields" — see this function's
            // rustdoc).
            let _ = writeln!(out, "      config:    {}", config_evidence(result.tier));
            let _ = writeln!(out, "      command:   {}", command_evidence(result.tier));
            let _ = writeln!(out, "      handshake: {}", handshake_evidence(result.tier));
            // Daemon attestation only gates `RestartHandshakeVerified`
            // promotion; suppress the line for tiers below that
            // because it would be a misleading "missing piece" hint
            // when the actual missing piece is the restart itself.
            if matches!(result.tier, McpTier::RestartHandshakeVerified) {
                let _ = writeln!(
                    out,
                    "      daemon:    {}",
                    daemon_evidence_label(d.daemon_attestation),
                );
            }
        }
    }

    let _ = writeln!(out, "  watch: {}", d.watch.label());

    match (&d.baseline_summary, d.baseline_present) {
        (Some(s), _) => {
            let _ = writeln!(
                out,
                "  baseline: {} antipattern, {} secret",
                s.antipattern, s.secret,
            );
        }
        (None, true) => out.push_str("  baseline: present (unreadable)\n"),
        (None, false) => out.push_str("  baseline: absent\n"),
    }

    // last_error is the only free-form string carrier on the
    // diagnostic. It is already printed by `render_human` to stdout,
    // so reflecting it here on stderr does not add a new leak
    // surface — the secrecy boundary is the same one stdout already
    // crosses.
    if let Some(err) = &d.last_error {
        let _ = writeln!(out, "  last_error: {err}");
    }

    // Daemon-side context block at the diagnostic level. Operator-
    // facing copy (not the raw enum token) per the spec § "SkipReason
    // vocabulary used by the daemon-evidence tracing is presented in
    // operator-friendly copy".
    let _ = writeln!(
        out,
        "  daemon-attestation: {}",
        daemon_evidence_label(d.daemon_attestation),
    );

    let _ = writeln!(out, "  why: {}", why_summary(d));

    out
}

/// Per-tier label for the "what is wired in the editor config?" line
/// of the verbose block. Pure function of [`McpTier`] so the surface
/// stays in lockstep with tier evolution.
fn config_evidence(tier: McpTier) -> &'static str {
    match tier {
        McpTier::NotDetected => "client not detected",
        McpTier::ConfigAbsent => "anvil entry absent",
        McpTier::ConfigPresent
        | McpTier::RestartRequired
        | McpTier::RestartHandshakeVerified
        | McpTier::ServerStartable
        | McpTier::LiveValidation => "anvil entry present",
    }
}

/// Per-tier label for the "is the configured command resolvable to a
/// real executable?" line.
fn command_evidence(tier: McpTier) -> &'static str {
    match tier {
        McpTier::NotDetected => "n/a (no client)",
        McpTier::ConfigAbsent | McpTier::ConfigPresent => "not yet verified",
        McpTier::RestartRequired
        | McpTier::RestartHandshakeVerified
        | McpTier::ServerStartable
        | McpTier::LiveValidation => "verified",
    }
}

/// Per-tier label for the "did the MCP server respond to the
/// startup handshake?" line.
fn handshake_evidence(tier: McpTier) -> &'static str {
    match tier {
        McpTier::NotDetected => "n/a (no client)",
        McpTier::ConfigAbsent | McpTier::ConfigPresent | McpTier::RestartRequired => {
            "not yet attempted (editor restart pending)"
        }
        McpTier::RestartHandshakeVerified | McpTier::LiveValidation => "ok",
        McpTier::ServerStartable => "server spawns; client wiring not confirmed",
    }
}

/// Operator-friendly copy for a [`super::daemon_evidence::DaemonAttestation`]
/// — never the raw enum token. Mirrors the `SkipReason` vocabulary
/// MLP2-051f documents at the daemon-evidence tracing site so a
/// support engineer chasing the trace finds the same wording in the
/// runbook.
///
/// Command-name policy: copy here MUST only reference subcommands
/// `anvil intercept` actually ships today (`start --foreground`,
/// `status`, `unblock`). `anvil intercept recover` does NOT exist
/// (PR #1909 review caught a draft that named it). The
/// `AllSurfacesQuarantined` recovery path is `stop the daemon and
/// restart with --foreground`, matching the existing
/// `render_human` repair hint that the council corrected on PR
/// #1848.
fn daemon_evidence_label(att: super::daemon_evidence::DaemonAttestation) -> &'static str {
    use super::daemon_evidence::DaemonAttestation;
    match att {
        DaemonAttestation::NotProbed => "not probed (no handshake-verified client to promote)",
        DaemonAttestation::Unreachable => "not running",
        DaemonAttestation::Unenforced => "running but this worktree is not registered",
        DaemonAttestation::StaleHeartbeat => "running but heartbeat is stale",
        DaemonAttestation::AllSurfacesQuarantined => {
            "running but every surface is quarantined (stop and restart with `anvil intercept start --foreground` to clear fence state)"
        }
        DaemonAttestation::Warming => "running but transient — wait briefly and re-run",
        DaemonAttestation::NoParticipatingSurface => {
            "running but this worktree has no participating surface yet"
        }
        DaemonAttestation::Enforced | DaemonAttestation::Promoted => {
            "running and attesting this worktree"
        }
    }
}

/// One-line "what is missing?" summary. Lets the operator skim the
/// verbose block and find the actionable next step without re-reading
/// every tier line.
///
/// Dispatch on [`super::state::ProtectionState`] first — the daemon
/// attestation is only the load-bearing signal for
/// `ReadyRestartRequired`. For other states (`NeedsAction` because
/// config is missing; `Watching` because the user opted into the
/// fallback; `Unsupported` because the language profile is not yet
/// covered; `Error` because activation failed) the missing piece is
/// elsewhere and a daemon-only `why:` line would misdirect
/// remediation. PR #1909 review.
fn why_summary(d: &ActivationDiagnostic) -> &'static str {
    match d.protection_state() {
        ProtectionState::Protecting => {
            "no missing piece — daemon attests this worktree and MCP is live"
        }
        ProtectionState::Error => {
            "activation errored — see `last_error` and re-run after fixing the cause"
        }
        ProtectionState::Unsupported => {
            "this repo's detected languages are not yet covered by anvil — no remediation in this release"
        }
        ProtectionState::Watching => {
            if d.daemon_attestation.attests_worktree() {
                "the intercept daemon attests this worktree; MCP pre-write is optional and can be enabled separately"
            } else {
                "save-time watch fallback is running; for pre-write coverage start the intercept daemon (`anvil intercept start --foreground`) and restart your editor"
            }
        }
        ProtectionState::NeedsAction => why_summary_for_needs_action(d),
        ProtectionState::ReadyRestartRequired => why_summary_for_attestation(d.daemon_attestation),
    }
}

/// `NeedsAction` branch of [`why_summary`] — the missing piece is in
/// the config / install layer, NOT the daemon. Walks the highest MCP
/// tier alongside [`super::diagnostic::ConfigStatus`] so the copy
/// names the layer the operator needs to touch next.
fn why_summary_for_needs_action(d: &ActivationDiagnostic) -> &'static str {
    use super::diagnostic::ConfigStatus;
    match d.config {
        ConfigStatus::Invalid => {
            "fix `.anvilrc` (or `.anvil.<ext>`) — see `last_error` for the parse failure, then re-run `anvil start --verify`"
        }
        ConfigStatus::Absent => {
            "run `anvil init` to write a default config, then `anvil start` to install the MCP entries"
        }
        ConfigStatus::Valid => match d.highest_mcp_tier() {
            None | Some(McpTier::NotDetected | McpTier::ConfigAbsent) => {
                "run `anvil start` to install MCP entries for the detected editor clients, then restart your editor"
            }
            Some(McpTier::ConfigPresent) => {
                "MCP entry is installed — restart your editor so the handshake completes, then re-run `anvil start --verify`"
            }
            Some(McpTier::ServerStartable) => {
                "the MCP server spawns but the editor wiring is not yet confirmed — restart your editor and re-run `anvil start --verify`"
            }
            // RestartRequired / RestartHandshakeVerified / LiveValidation
            // cannot occur under NeedsAction (they would have promoted to
            // ReadyRestartRequired / Protecting upstream). Fall back to a
            // safe diagnostic — never panic, but the matcher exhaustively
            // covers the reachable cases above.
            Some(_) => {
                "activation is in an intermediate state — re-run `anvil start --verify` to refresh the diagnostic"
            }
        },
    }
}

/// `ReadyRestartRequired` branch of [`why_summary`] — the missing
/// piece is daemon-side, so the [`super::daemon_evidence::DaemonAttestation`]
/// drives the copy. Pre-PR #1909-review this was the only summary
/// path; it now only fires when `protection_state()` actually maps
/// to `ReadyRestartRequired`.
///
/// Command-name policy: same as [`daemon_evidence_label`] — only
/// reference subcommands `anvil intercept` actually ships today.
fn why_summary_for_attestation(att: super::daemon_evidence::DaemonAttestation) -> &'static str {
    use super::daemon_evidence::DaemonAttestation;
    match att {
        DaemonAttestation::NotProbed => {
            "restart your editor so the MCP handshake completes; the daemon will be probed afterwards"
        }
        DaemonAttestation::Unreachable => {
            "start the intercept daemon (`anvil intercept start --foreground`) so pre-write validation can attach"
        }
        DaemonAttestation::Unenforced | DaemonAttestation::NoParticipatingSurface => {
            "daemon is running but this worktree is not registered — see `anvil intercept status`"
        }
        DaemonAttestation::StaleHeartbeat => {
            "daemon heartbeat is stale — stop and restart it with `anvil intercept start --foreground`"
        }
        DaemonAttestation::AllSurfacesQuarantined => {
            "every surface is quarantined — stop and restart the daemon with `anvil intercept start --foreground` to clear fence state"
        }
        DaemonAttestation::Warming => {
            "daemon is starting up — wait briefly and re-run `anvil start --verify`"
        }
        DaemonAttestation::Enforced | DaemonAttestation::Promoted => {
            "no missing piece — daemon attests this worktree"
        }
    }
}

/// One-line headline for the human and JSON surfaces. Delegates to
/// [`ProtectionState::headline`] except when the state is
/// `ReadyRestartRequired` and the daemon attestation is
/// [`super::daemon_evidence::DaemonAttestation::Unreachable`]: there the
/// generic "restart your editor or agent" headline is misleading, because no
/// daemon is answering this worktree and another restart cannot change that.
/// DLIFE-006 (#2609, #2583, #1831) replaces it with a terminating
/// diagnosis. Both renderers route through this so the headline copy
/// never drifts between surfaces.
fn headline_for(state: ProtectionState, d: &ActivationDiagnostic) -> &'static str {
    use super::daemon_evidence::DaemonAttestation;

    match (state, d.daemon_attestation) {
        (ProtectionState::ReadyRestartRequired, DaemonAttestation::Unreachable) => {
            "Daemon not reachable — protection cannot graduate until the intercept daemon answers this worktree."
        }
        _ => state.headline(),
    }
}

/// Plain-language explanation for lifecycle labels whose terse `snake_case`
/// value is necessary for machine consumers but too opaque on its own.
fn state_explanation(state: ProtectionState, d: &ActivationDiagnostic) -> Option<&'static str> {
    use super::daemon_evidence::DaemonAttestation;

    match state {
        ProtectionState::ReadyRestartRequired => Some(match d.daemon_attestation {
            DaemonAttestation::NotProbed => {
                "anvil has written the MCP config, but the editor or agent has not attached to it yet. Restart that editor or agent, then run `anvil start --verify` again; restarting the whole machine is not required."
            }
            DaemonAttestation::Unreachable => {
                "The editor or agent has seen anvil's MCP config, but the local intercept daemon is not reachable. Start it with `anvil intercept start --foreground`, then run `anvil start --verify` again."
            }
            DaemonAttestation::Unenforced | DaemonAttestation::NoParticipatingSurface => {
                "The intercept daemon is running, but this worktree is not attached to an enforcing session yet. Check `anvil intercept status`, then run `anvil start --verify` again after the editor issues an MCP request."
            }
            DaemonAttestation::StaleHeartbeat => {
                "The intercept daemon was reachable before, but its heartbeat is stale. Stop that daemon process, start it again with `anvil intercept start --foreground`, then run `anvil start --verify` again."
            }
            DaemonAttestation::AllSurfacesQuarantined => {
                "The intercept daemon fenced every session for this worktree. Stop that daemon process, start it again with `anvil intercept start --foreground`, then run `anvil start --verify` again."
            }
            DaemonAttestation::Warming => {
                "The intercept daemon is starting or settling. Wait a few seconds, then run `anvil start --verify` again."
            }
            DaemonAttestation::Enforced | DaemonAttestation::Promoted => {
                "anvil has enough evidence to protect this worktree, but this view has not refreshed yet. Run `anvil start --verify` again to refresh the state."
            }
        }),
        _ => None,
    }
}

/// Build the JSON value for embedding inside a parent JSON document
/// (e.g. `anvil status --json`'s `activation` field).
///
/// The shape is stable contract — keys are: `state`, `headline`,
/// `config`, `mcp` (array of `{client, tier}` objects), `watch`,
/// `baseline_present`, `last_error`, `all_languages_unsupported`,
/// `repo_languages` (array of `{name, files_seen, coverage_tier, basis}`),
/// and `unclassified_files_seen` (count of files whose extension
/// matched no registry entry). Tooling consumers may rely on this
/// set; downstream PRs add fields, they do not rename or remove
/// existing ones.
///
/// **Per-client install outcomes are intentionally NOT in this contract.**
/// `anvil start --json` short-circuits to a read-only
/// [`super::diagnostic::verify`] probe (see `commands/start.rs`) so
/// stdout stays a single JSON document; the install path is never
/// exercised under `--json`. The non-JSON path
/// (`activation::orchestrator::run`) is the only place install
/// failures occur today.
///
/// What CI consumers see in `--json` mode (read-only probe):
///
/// 1. `state` — collapses to `error` only on **probe-time** failures
///    (e.g. `current_exe()` resolution, malformed editor config files
///    encountered while reading them). Install-time write failures
///    cannot occur here because no install runs.
/// 2. `mcp[].tier` — the observed tier each client has reached
///    (`config_present` / `restart_required` /
///    `restart_handshake_verified` / `server_startable` /
///    `live_validation`). This is the canonical signal for "is anvil
///    wired into this client?" in `--json` mode. Tiers up to and
///    including `restart_required` are derived from on-disk config;
///    `restart_handshake_verified` and `server_startable` reflect a
///    runtime handshake probe done at verify time and are not
///    persisted, so consumers should treat the field as observed
///    state rather than durable state.
/// 3. `last_error` — populated on probe failure with the underlying
///    cause. Empty on a clean read-only probe even when `mcp[].tier`
///    indicates `config_absent`.
///
/// In the non-JSON path (`anvil start` without `--json`), install
/// failures DO surface via `last_error`, which the orchestrator
/// populates from the per-client `InstallReport`. CI hooks running
/// `anvil start && next-step` see this as a non-zero exit code from
/// `commands/start.rs`.
///
/// If a future workflow needs the structured install block in JSON,
/// the design choice is documented in LAUNCH-009.5: either route
/// `--json` through the orchestrator with init's stdout suppressed,
/// or add an `install:` block here and emit two top-level objects
/// (which would break the single-document contract).
///
/// The body uses `serde_json::json!` so the value is constructed
/// directly from primitives — no fallible `to_value` round-trip and
/// no panic path for the binary to inherit.
pub fn render_json(d: &ActivationDiagnostic) -> Value {
    let state = d.protection_state();
    // LAUNCH-009: each MCP client entry carries a `transport` tag from
    // the per-client probe result, so the schema is honest about the
    // transport actually used. v1 always emits `"stdio"` because that's
    // the only transport `AnvilEntry` constructs today; future hosted-
    // MCP-server variants populate `RemoteSse` / `RemoteHttp` from the
    // same source of truth.
    let mcp: Vec<Value> = d
        .mcp
        .iter()
        .map(|(c, r)| {
            json!({
                "client": c.label(),
                "tier": r.tier.label(),
                "transport": r.transport.label(),
            })
        })
        .collect();
    let repo_languages: Vec<Value> = d
        .language_profile
        .entries
        .iter()
        .map(|e| {
            json!({
                "name": e.name,
                "files_seen": e.files_seen,
                "coverage_tier": e.coverage_tier.label(),
                "basis": e.basis,
            })
        })
        .collect();
    // LAUNCH-010: emit the per-kind baseline summary alongside the
    // back-compat `baseline_present` flag. `baseline` is `null` when
    // absent or unreadable; consumers MUST treat a missing key the
    // same as `null`, so adding the key is additive.
    let baseline = d.baseline_summary.as_ref().map(|s| {
        json!({
            "total": s.total,
            "antipattern": s.antipattern,
            "secret": s.secret,
            "created_at": s.created_at,
        })
    });
    json!({
        "state": state.label(),
        "headline": headline_for(state, d),
        "config": d.config.label(),
        "mcp": mcp,
        "watch": d.watch.label(),
        "baseline_present": d.baseline_present,
        "baseline": baseline,
        "last_error": d.last_error,
        "all_languages_unsupported": d.all_languages_unsupported,
        "repo_languages": repo_languages,
        "unclassified_files_seen": d.language_profile.unclassified_files_seen,
    })
}

/// Concrete, actionable next step for the current state. Returning
/// `None` means the surface should not append a "next" line — usually
/// because no actionable hint applies (e.g. already protecting).
fn repair_hint(state: ProtectionState, d: &ActivationDiagnostic) -> Option<&'static str> {
    use super::daemon_evidence::DaemonAttestation;

    match state {
        ProtectionState::Protecting => None,
        ProtectionState::ReadyRestartRequired => Some(match d.daemon_attestation {
            // MLP2-051f: when the orchestrator handshake-pass left at
            // least one client at `RestartHandshakeVerified`, the user
            // has already restarted their editor — the missing piece
            // is the intercept daemon, not another restart. Each
            // attestation branch points at its concrete remediation.
            DaemonAttestation::Unreachable => {
                "no intercept daemon is answering for this worktree, so another editor restart will not help; start the intercept daemon with `anvil intercept start --foreground` (or wait for it to finish starting), then re-run `anvil start --verify`."
            }
            DaemonAttestation::Unenforced | DaemonAttestation::NoParticipatingSurface => {
                "the intercept daemon is running but is not enforcing this worktree yet; check `anvil intercept status` for the registered worktree set and re-run `anvil start --verify` after your editor has issued an MCP request."
            }
            DaemonAttestation::StaleHeartbeat => {
                "the intercept daemon's last attestation is stale; stop it (close its terminal, or end the process via Task Manager / `kill <PID>`) and start it again with `anvil intercept start --foreground`, then re-run `anvil start --verify`."
            }
            DaemonAttestation::AllSurfacesQuarantined => {
                "the intercept daemon has fenced every session for this worktree; stop the daemon (close its terminal, or end the process via Task Manager / `kill <PID>`) and start it again with `anvil intercept start --foreground` to clear fence state, then re-run `anvil start --verify`."
            }
            DaemonAttestation::Warming => {
                "the intercept daemon is transitioning (warming / draining); re-run `anvil start --verify` in a few seconds."
            }
            // `NotProbed` is the genuine pre-restart case — the
            // diagnostic never reached the daemon probe because no
            // client was at `RestartHandshakeVerified` yet.
            // `Enforced` / `Promoted` are logically unreachable at
            // this branch (`protection_state()` returns `Watching` or
            // `Protecting` instead); keep the original copy as a
            // belt-and-braces fallback rather than panic.
            DaemonAttestation::NotProbed
            | DaemonAttestation::Enforced
            | DaemonAttestation::Promoted => {
                "restart your editor or agent so the MCP server attaches, then re-run `anvil start --verify`."
            }
        }),
        ProtectionState::Watching => {
            // Invariant (debug-asserted below): `LiveValidation` is
            // unreachable here because `protection_state` returns
            // `Protecting` first when any client is at that tier.
            let highest_mcp = d.mcp.values().map(|r| r.tier).max();
            let mcp_restart_pending = matches!(
                highest_mcp,
                Some(McpTier::RestartRequired | McpTier::RestartHandshakeVerified)
            );
            if d.daemon_attestation.attests_worktree() {
                // Council S4: the daemon already attests this worktree, so the
                // user is covered now. Only mention an editor restart when an
                // MCP client is actually configured and one restart from live —
                // restarting then promotes to `Protecting` (protection_state
                // maps `LiveValidation` -> `Protecting` ahead of this
                // daemon-attests branch, verified by the unit tests). When no
                // MCP client is configured, restarting would change nothing, so
                // keep MCP framed as an optional upgrade and do not nag
                // (ACTMO-003 — the spine is the protection, MCP is the bonus).
                return Some(if mcp_restart_pending {
                    "the intercept daemon attests this worktree, so you are covered now; your MCP client is configured — restart your editor to upgrade to pre-write protection, then re-run `anvil start --verify`."
                } else {
                    "the intercept daemon is registered for this worktree; MCP pre-write remains optional, and `anvil intercept status` shows the daemon-backed surface."
                });
            }
            // Council remediation: the next step depends on whether
            // any MCP tier is already past `ConfigPresent`. If the
            // server is already configured and startable, telling
            // the user to run `anvil mcp install` is wrong — they
            // need to restart their editor.
            //
            // The match arms below only handle tiers strictly weaker
            // than `LiveValidation`; if a future refactor breaks the
            // invariant, the assertion fires in debug builds.
            debug_assert!(
                !matches!(highest_mcp, Some(McpTier::LiveValidation)),
                "Watching unreachable when MCP at LiveValidation"
            );
            if matches!(
                highest_mcp,
                Some(
                    McpTier::ServerStartable
                        | McpTier::RestartRequired
                        | McpTier::RestartHandshakeVerified,
                )
            ) {
                Some(
                    "watch is the save-time fallback — your MCP server is configured; restart your editor and re-run `anvil start --verify` to upgrade to pre-write validation.",
                )
            } else {
                Some(
                    "watch is the save-time fallback — to upgrade to pre-write validation, run `anvil mcp install` to wire up your MCP-capable editor (for example Cursor or Claude Code).",
                )
            }
        }
        ProtectionState::NeedsAction => {
            if matches!(d.config, super::diagnostic::ConfigStatus::Absent) {
                Some("run `anvil init` to create a config, then `anvil start` to activate.")
            } else if d.mcp.values().all(|r| r.tier < McpTier::ConfigPresent) {
                // LAUNCH-011: nudge toward the explicit watch fallback
                // composition (`anvil start --watch`) so the user does
                // not have to discover the two surfaces independently.
                Some(
                    "run `anvil start` to wire your MCP-capable editor's MCP paths (for example Cursor or Claude Code), or `anvil start --watch` for save-time fallback protection.",
                )
            } else {
                Some("run `anvil start --verify` to re-check activation.")
            }
        }
        ProtectionState::Unsupported => {
            // Round-2 council: do not promise watch runs `secrets
            // only` on unsupported files — that isolation is owned by
            // LAUNCH-016, which has not landed. Keep the copy
            // descriptive and avoid future-tense claims.
            Some(
                "anvil does not yet cover this repo's languages in the current release. Architecture / antipattern checks will not produce findings on files in unsupported languages; coverage expands as language packs ship.",
            )
        }
        ProtectionState::Error => Some(
            "re-run `anvil start --verify` after addressing the cause; activation will not write any state until it can proceed safely.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::super::diagnostic::{
        ActivationDiagnostic, ConfigStatus, McpClientId, McpTier, WatchTier,
    };
    use super::*;
    use std::collections::BTreeMap;

    fn empty() -> ActivationDiagnostic {
        ActivationDiagnostic {
            config: ConfigStatus::Absent,
            mcp: BTreeMap::new(),
            watch: WatchTier::NotRequested,
            baseline_present: false,
            baseline_summary: None,
            last_error: None,
            all_languages_unsupported: false,
            language_profile: super::super::language_profile::RepoLanguageProfile::default(),
            daemon_attestation: super::super::daemon_evidence::DaemonAttestation::NotProbed,
        }
    }

    fn protecting() -> ActivationDiagnostic {
        let mut d = empty();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::Cursor, McpTier::LiveValidation.into());
        d
    }

    fn restart_required() -> ActivationDiagnostic {
        let mut d = empty();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::ClaudeCode, McpTier::RestartRequired.into());
        d
    }

    fn watching() -> ActivationDiagnostic {
        let mut d = empty();
        d.config = ConfigStatus::Valid;
        d.watch = WatchTier::Running;
        d
    }

    fn unsupported() -> ActivationDiagnostic {
        let mut d = empty();
        d.config = ConfigStatus::Valid;
        d.all_languages_unsupported = true;
        d
    }

    fn config_error() -> ActivationDiagnostic {
        let mut d = empty();
        d.config = ConfigStatus::Invalid;
        d
    }

    #[test]
    fn human_render_includes_state_label() {
        let h = render_human(&protecting());
        assert!(h.contains("state: protecting"), "rendered: {h}");
    }

    #[test]
    fn baseline_copy_does_not_promise_an_unwired_diff() {
        // CIB-127: activation writes the finding-baseline but no command
        // consumes it yet. The copy must not imply a diff that nothing
        // performs (the old wording promised "future scans will report new
        // regressions").
        let mut d = protecting();
        d.baseline_present = true;
        d.baseline_summary = Some(super::super::diagnostic::BaselineSummary {
            total: 3,
            antipattern: 3,
            secret: 0,
            created_at: "2026-07-01T00:00:00Z".to_string(),
        });
        let h = render_human(&d);
        assert!(
            h.contains("baseline: present"),
            "baseline line must still render the count: {h}"
        );
        assert!(
            !h.contains("will report new regressions"),
            "copy must not promise an unwired diff: {h}"
        );
        assert!(
            h.contains("not yet used to filter"),
            "copy must state the baseline is recorded but not yet consumed: {h}"
        );
    }

    #[test]
    fn human_render_marks_watch_as_fallback() {
        let h = render_human(&watching());
        let lower = h.to_lowercase();
        assert!(
            lower.contains("fallback") || lower.contains("weaker"),
            "watch render must label fallback nature, got: {h}"
        );
    }

    #[test]
    fn human_render_unsupported_does_not_claim_protection() {
        let h = render_human(&unsupported());
        let lower = h.to_lowercase();
        assert!(
            !lower.contains("protecting"),
            "unsupported render must not contain `protecting`, got: {h}"
        );
        assert!(
            lower.contains("unsupported") || lower.contains("does not yet cover"),
            "unsupported render must name the gap honestly, got: {h}"
        );
    }

    #[test]
    fn human_render_config_error_does_not_claim_coverage() {
        let h = render_human(&config_error());
        let lower = h.to_lowercase();
        assert!(lower.contains("error"), "render must mark error: {h}");
        assert!(
            !lower.contains("protecting") && !lower.contains("watching"),
            "error render must not claim coverage, got: {h}"
        );
    }

    #[test]
    fn json_render_keys_are_stable() {
        let v = render_json(&restart_required());
        let obj = v.as_object().unwrap();
        let expected_keys = [
            "state",
            "headline",
            "config",
            "mcp",
            "watch",
            "baseline_present",
            // LAUNCH-010 added: per-kind baseline summary. Null when
            // absent or unreadable; populated when the file parses.
            "baseline",
            "last_error",
            "all_languages_unsupported",
            // PR 5 (LAUNCH-015) added: pin both the per-language
            // breakdown and the unclassified counter to lock the
            // JSON contract against silent removal.
            "repo_languages",
            "unclassified_files_seen",
        ];
        for key in expected_keys {
            assert!(obj.contains_key(key), "missing key {key} in {v}");
        }
        // Round-2 council: pin the total key count so a future
        // rename adding the old name as an alias is also caught.
        assert_eq!(
            obj.len(),
            expected_keys.len(),
            "JSON output has unexpected keys: {obj:?}"
        );
        assert_eq!(obj["state"], "ready_restart_required");
        assert_eq!(obj["watch"], "not_requested");
        assert!(obj["repo_languages"].is_array());
    }

    #[test]
    fn json_render_emits_one_mcp_entry_per_client() {
        let mut d = empty();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::Cursor, McpTier::ConfigPresent.into());
        d.mcp
            .insert(McpClientId::ClaudeCode, McpTier::RestartRequired.into());
        let v = render_json(&d);
        let arr = v["mcp"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        let labels: Vec<&str> = arr.iter().map(|e| e["client"].as_str().unwrap()).collect();
        assert!(labels.contains(&"cursor"));
        assert!(labels.contains(&"claude-code"));
    }

    #[test]
    fn human_render_for_each_required_scenario() {
        // LAUNCH-008 acceptance: targeted CLI tests cover final-state
        // rendering for at least protected, restart-required, watch
        // fallback, unsupported, and config-error scenarios. Each
        // scenario must render its literal label and no other.
        let cases = [
            ("protecting", render_human(&protecting())),
            ("ready_restart_required", render_human(&restart_required())),
            ("watching", render_human(&watching())),
            ("unsupported", render_human(&unsupported())),
            ("error", render_human(&config_error())),
        ];
        for (expected, render) in &cases {
            assert!(
                render.contains(&format!("state: {expected}")),
                "case {expected} did not produce its literal state line, render:\n{render}"
            );
            for forbidden in [
                "protecting",
                "ready_restart_required",
                "watching",
                "unsupported",
                "error",
            ] {
                if forbidden == *expected {
                    continue;
                }
                let line = format!("state: {forbidden}");
                assert!(
                    !render.contains(&line),
                    "case {expected} also rendered conflicting line {line}, render:\n{render}"
                );
            }
        }
    }

    fn handshake_verified_diag(
        attestation: super::super::daemon_evidence::DaemonAttestation,
    ) -> ActivationDiagnostic {
        let mut d = empty();
        d.config = ConfigStatus::Valid;
        d.mcp.insert(
            McpClientId::ClaudeCode,
            McpTier::RestartHandshakeVerified.into(),
        );
        d.daemon_attestation = attestation;
        d
    }

    /// MLP2-051f §"Failure modes" — daemon unreachable: the user has
    /// restarted, the handshake passed, but the intercept daemon is
    /// not running. The hint must point at `anvil intercept start`,
    /// not "restart your editor again".
    #[test]
    fn ready_restart_required_with_daemon_unreachable_points_at_intercept_start() {
        let d =
            handshake_verified_diag(super::super::daemon_evidence::DaemonAttestation::Unreachable);
        let h = render_human(&d);
        assert!(
            h.contains("anvil intercept start"),
            "Unreachable hint must name `anvil intercept start`: {h}"
        );
    }

    /// DLIFE-006 (#2609, #2583, #1831) — when the daemon is `Unreachable`
    /// the repair hint must read as a terminating end state: it must name
    /// *why* protection cannot graduate (no daemon answering the worktree),
    /// must say that another editor restart will not help, and must NOT
    /// hedge by suggesting "restart your editor again". The only path
    /// forward is starting the daemon, then re-running `--verify`.
    #[test]
    fn ready_restart_required_with_daemon_unreachable_is_terminating_not_a_restart_loop() {
        let d =
            handshake_verified_diag(super::super::daemon_evidence::DaemonAttestation::Unreachable);
        let h = render_human(&d);
        let lower = h.to_lowercase();
        // No restart-again loop: the daemon is down, so restarting the
        // editor cannot change the outcome.
        assert!(
            !lower.contains("restart your editor"),
            "Unreachable hint must not tell the user to restart their editor again: {h}"
        );
        assert!(
            !lower.contains("otherwise"),
            "Unreachable hint must not hedge with an `otherwise restart` clause: {h}"
        );
        // Names why protection cannot graduate, and that restart won't help.
        assert!(
            h.contains("no intercept daemon is answering for this worktree"),
            "Unreachable hint must name why protection cannot graduate: {h}"
        );
        assert!(
            h.contains("will not help"),
            "Unreachable hint must state that another restart will not help: {h}"
        );
        // Still actionable and copy-ready.
        assert!(
            h.contains("anvil intercept start --foreground") && h.contains("anvil start --verify"),
            "Unreachable hint must give the daemon-start command and the re-run command: {h}"
        );
    }

    /// MLP2-051f §"Failure modes" — daemon running but the worktree
    /// has no participating surfaces. The hint must NOT tell the user
    /// to start the daemon (it's already running). Points the operator
    /// at `anvil intercept status` for the registered worktree set.
    #[test]
    fn ready_restart_required_with_unenforced_points_at_intercept_status() {
        let d =
            handshake_verified_diag(super::super::daemon_evidence::DaemonAttestation::Unenforced);
        let h = render_human(&d);
        assert!(
            h.contains("anvil intercept status"),
            "Unenforced hint must name `anvil intercept status`: {h}"
        );
        assert!(
            !h.contains("anvil intercept start"),
            "Unenforced hint must not say `anvil intercept start` (daemon is already running): {h}"
        );
    }

    /// MLP2-051f §"Failure modes" — daemon `DegradedProtection` with
    /// every surface `Quarantined`. Recovery routes through a daemon
    /// restart (cross-platform), NOT `anvil intercept unblock` which
    /// is Linux-only today (Windows bails with "not yet supported").
    /// Post-ship hardening (council 2026-05-22 + Copilot review on
    /// PR #1848): the previous hint pointed at `anvil intercept
    /// recover` which doesn't exist; an interim draft pointed at
    /// `anvil intercept unblock --worktree $(pwd)` which uses
    /// bash-only `$(pwd)` AND fails on Windows.
    #[test]
    fn ready_restart_required_with_all_quarantined_points_at_daemon_restart() {
        let d = handshake_verified_diag(
            super::super::daemon_evidence::DaemonAttestation::AllSurfacesQuarantined,
        );
        let h = render_human(&d);
        assert!(
            h.contains("anvil intercept start --foreground"),
            "AllSurfacesQuarantined hint must name the cross-platform restart path: {h}"
        );
        assert!(
            !h.contains("anvil intercept recover"),
            "AllSurfacesQuarantined hint must NOT mention the non-existent `recover` subcommand: {h}"
        );
        assert!(
            !h.contains("$(pwd)"),
            "AllSurfacesQuarantined hint must NOT use bash-only `$(pwd)`: {h}"
        );
    }

    /// MLP2-051f §"Failure modes" — daemon `Warming` (transient
    /// daemon state — leaving / joining, not yet enforcing). The
    /// remediation is to wait + re-run, not to start anything.
    #[test]
    fn ready_restart_required_with_warming_says_wait_and_re_run() {
        let d = handshake_verified_diag(super::super::daemon_evidence::DaemonAttestation::Warming);
        let h = render_human(&d);
        assert!(
            h.contains("re-run") && h.contains("seconds"),
            "Warming hint must say re-run in a few seconds: {h}"
        );
        assert!(
            !h.contains("anvil intercept start"),
            "Warming hint must not say `anvil intercept start` (daemon is already running): {h}"
        );
    }

    /// MLP2-051f §"Failure modes" — daemon reachable + worktree
    /// promotable, but cardinality gate (≥ 1 Participating surface)
    /// fails. Reuses the `Unenforced` operator-facing message — the
    /// remediation is the same: check `anvil intercept status` for
    /// the registered set.
    #[test]
    fn ready_restart_required_with_no_participating_surface_points_at_intercept_status() {
        let d = handshake_verified_diag(
            super::super::daemon_evidence::DaemonAttestation::NoParticipatingSurface,
        );
        let h = render_human(&d);
        assert!(
            h.contains("anvil intercept status"),
            "NoParticipatingSurface hint must name `anvil intercept status`: {h}"
        );
        assert!(
            !h.contains("anvil intercept start"),
            "NoParticipatingSurface hint must not say `anvil intercept start` (daemon is already running): {h}"
        );
    }

    /// MLP2-051f §"Failure modes" — daemon snapshot heartbeat stale.
    /// The remediation is to stop and restart the daemon, not the
    /// editor. Post-ship hardening (council 2026-05-22): the previous
    /// hint pointed at `anvil intercept restart` which doesn't exist;
    /// the real path is stop + `anvil intercept start --foreground`.
    #[test]
    fn ready_restart_required_with_stale_heartbeat_points_at_daemon_start() {
        let d = handshake_verified_diag(
            super::super::daemon_evidence::DaemonAttestation::StaleHeartbeat,
        );
        let h = render_human(&d);
        assert!(
            h.contains("anvil intercept start --foreground"),
            "StaleHeartbeat hint must name the real `anvil intercept start --foreground` command: {h}"
        );
        assert!(
            !h.contains("anvil intercept restart"),
            "StaleHeartbeat hint must NOT mention the non-existent `restart` subcommand: {h}"
        );
    }

    /// MLP2-051f §"Failure modes" — genuine pre-restart case
    /// (`NotProbed` because no client is at `RestartHandshakeVerified`
    /// yet). The original "restart your editor or agent" copy is the
    /// right answer.
    #[test]
    fn ready_restart_required_with_not_probed_keeps_original_restart_copy() {
        let mut d = empty();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::ClaudeCode, McpTier::RestartRequired.into());
        d.daemon_attestation = super::super::daemon_evidence::DaemonAttestation::NotProbed;
        let h = render_human(&d);
        assert!(
            h.contains("restart your editor"),
            "NotProbed hint must keep the editor-restart copy: {h}"
        );
        assert!(
            !h.contains("anvil intercept"),
            "NotProbed hint must NOT mention `anvil intercept` (the user has not even restarted yet): {h}"
        );
        assert!(
            h.contains("meaning:")
                && h.contains("restarting the whole machine is not required")
                && h.contains("anvil start --verify"),
            "NotProbed render must explain the label and give the exact verification command: {h}"
        );
    }

    #[test]
    fn ready_restart_required_with_daemon_unreachable_explains_exact_recovery() {
        let d =
            handshake_verified_diag(super::super::daemon_evidence::DaemonAttestation::Unreachable);
        let h = render_human(&d);
        assert!(
            h.contains("meaning:")
                && h.contains("local intercept daemon is not reachable")
                && h.contains("anvil intercept start --foreground")
                && h.contains("anvil start --verify"),
            "Unreachable render must explain the label and give copy-ready commands: {h}"
        );
    }

    #[test]
    fn protecting_render_has_no_repair_hint() {
        let h = render_human(&protecting());
        // The repair hint is appended as `  next: ...`; protecting
        // means there is nothing to repair.
        assert!(
            !h.contains("next:"),
            "protecting render must not include a `next:` line, got: {h}"
        );
    }

    #[test]
    fn watching_with_server_startable_hint_says_restart_not_install() {
        // Round-2 council remediation: the Watching hint must not
        // say "run `anvil mcp install`" when the MCP server is
        // already configured and startable. The user needs to
        // restart their editor, not reinstall.
        let mut d = empty();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::Cursor, McpTier::ServerStartable.into());
        d.watch = WatchTier::Running;
        let h = render_human(&d);
        assert!(
            h.contains("restart your editor") && !h.contains("anvil mcp install"),
            "ServerStartable + Watching hint should advise restart, not install: {h}"
        );
    }

    #[test]
    fn daemon_backed_watching_with_restart_required_offers_optional_upgrade_not_loop() {
        let mut d = empty();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::Cursor, McpTier::RestartRequired.into());
        d.daemon_attestation = super::super::daemon_evidence::DaemonAttestation::Enforced;

        let h = render_human(&d);

        assert!(
            h.contains("state: watching"),
            "daemon-attested spine should fall through to watching, got: {h}"
        );
        // Council S4: leads with current coverage and never frames the restart
        // as required — the spine already protects this worktree.
        assert!(
            h.contains("covered now"),
            "daemon-backed watching must affirm current coverage: {h}"
        );
        assert!(
            !h.to_lowercase().contains("restart required")
                && !h.to_lowercase().contains("must restart"),
            "daemon-backed watching must not present a restart as required: {h}"
        );
        // But because an editor restart genuinely promotes to Protecting
        // (protection_state maps LiveValidation -> Protecting before this
        // branch), it is honest to offer the restart as an optional upgrade
        // when an MCP client is configured.
        assert!(
            h.contains("upgrade to pre-write protection"),
            "configured-MCP watching should offer the restart as an optional upgrade: {h}"
        );
    }

    #[test]
    fn daemon_backed_watching_without_mcp_keeps_mcp_optional_and_silent_on_restart() {
        // The other half of Council S4: with no MCP client configured, a
        // restart would change nothing, so the copy must stay non-nagging.
        let mut d = empty();
        d.config = ConfigStatus::Valid;
        d.daemon_attestation = super::super::daemon_evidence::DaemonAttestation::Enforced;

        let h = render_human(&d);

        assert!(h.contains("state: watching"), "got: {h}");
        assert!(
            h.contains("intercept daemon") && h.contains("MCP pre-write remains optional"),
            "no-MCP daemon-backed watching must keep MCP framed as optional: {h}"
        );
        assert!(
            !h.contains("restart your editor"),
            "no-MCP daemon-backed watching must not nag an editor restart: {h}"
        );
    }

    #[test]
    fn watching_without_mcp_hint_says_install() {
        let mut d = empty();
        d.config = ConfigStatus::Valid;
        d.watch = WatchTier::Running;
        let h = render_human(&d);
        assert!(
            h.contains("anvil mcp install"),
            "Watching without MCP should advise install: {h}"
        );
    }

    #[test]
    fn unsupported_hint_does_not_promise_secrets_via_watch() {
        // Round-2 council: the Unsupported hint must not over-claim
        // that watch runs secrets-only on unsupported files — that
        // isolation is owned by LAUNCH-016 and is not yet wired.
        // Round-3 council: pin against any combined claim of "secret"
        // + "watch" coverage so a future copy edit cannot reintroduce
        // the over-claim under different wording.
        let h = render_human(&unsupported()).to_lowercase();
        let mentions_secrets = h.contains("secret");
        let mentions_watch = h.contains("watch");
        assert!(
            !(mentions_secrets && mentions_watch),
            "Unsupported hint must not pair `secret` and `watch` to imply coverage: {h}"
        );
    }

    // ---- LAUNCH-011: explicit fallback honesty ----------------------

    /// Helper: a `Watching` diagnostic where MCP is genuinely below
    /// `RestartRequired` — the most common shape `anvil start --watch`
    /// renders before the kernel watcher takes over.
    fn watching_no_mcp() -> ActivationDiagnostic {
        let mut d = empty();
        d.config = ConfigStatus::Valid;
        d.watch = WatchTier::Running;
        d.mcp
            .insert(McpClientId::Cursor, McpTier::ConfigAbsent.into());
        d.mcp
            .insert(McpClientId::ClaudeCode, McpTier::ConfigAbsent.into());
        d
    }

    /// Helper: `WatchTier::Offered` shape produced by `verify` on a
    /// fresh repo where MCP cannot pre-write attach.
    fn watch_offered_no_mcp() -> ActivationDiagnostic {
        let mut d = empty();
        d.config = ConfigStatus::Valid;
        d.watch = WatchTier::Offered;
        d.mcp
            .insert(McpClientId::Cursor, McpTier::ConfigAbsent.into());
        d
    }

    #[test]
    fn human_render_states_pre_write_not_attached_when_watching() {
        let h = render_human(&watching_no_mcp());
        assert!(
            h.contains("MCP pre-write validation is not attached"),
            "watching render must explicitly say MCP pre-write validation is \
             not attached, got:\n{h}"
        );
        assert!(
            h.contains("save-time fallback") || h.contains("saved file changes"),
            "watching render must label fallback as save-time only, got:\n{h}"
        );
        assert!(
            !h.contains("fully protected"),
            "watching render must NEVER claim full protection, got:\n{h}"
        );
        assert!(
            !h.contains("MCP attached"),
            "watching render must not claim MCP attached, got:\n{h}"
        );
    }

    #[test]
    fn human_render_states_pre_write_not_attached_when_watch_offered() {
        let h = render_human(&watch_offered_no_mcp());
        assert!(
            h.contains("watch: offered"),
            "rendered watch tier line must show offered, got:\n{h}"
        );
        assert!(
            h.contains("MCP pre-write validation is not attached"),
            "offered fallback render must explicitly say MCP pre-write \
             validation is not attached, got:\n{h}"
        );
    }

    #[test]
    fn human_render_omits_fallback_note_when_protecting() {
        // Adversarial guard: live MCP must not get the partial-protection
        // note appended — that would sow doubt where the diagnostic has
        // literal evidence of pre-write validation.
        let h = render_human(&protecting());
        assert!(
            !h.contains("MCP pre-write validation is not attached"),
            "protecting render must not include the fallback note, got:\n{h}"
        );
    }

    #[test]
    fn human_render_omits_fallback_note_when_ready_restart_required() {
        // Council round-2 remediation: the `ReadyRestartRequired`
        // headline already says "restart your editor or agent so the
        // MCP server attaches", which honestly communicates the
        // partial state. Appending the watch-fallback note at this
        // tier produces orphaned copy ("watch fallback exists" with
        // no `watch: offered` line to act on), nudging the user
        // toward watch when they should restart. The honesty contract
        // is preserved by the headline; the note is redundant here.
        let h = render_human(&restart_required());
        assert!(
            !h.contains("MCP pre-write validation is not attached"),
            "ready_restart_required render must NOT include the \
             partial-protection note — the headline already \
             communicates the restart-required state, got:\n{h}"
        );
        // Belt-and-braces: for this fixture (NotProbed attestation) the
        // headline must carry the partial-state language. If a future
        // copy edit drops it, the absence of the note plus a "fully
        // protected"-style headline would be a regression. (The
        // `Unreachable` sub-case deliberately uses a different,
        // terminating headline — see `headline_for` and DLIFE-006.)
        let lower = h.to_lowercase();
        assert!(
            lower.contains("restart") && (lower.contains("attach") || lower.contains("mcp server")),
            "ready_restart_required headline must communicate the \
             restart-pending state directly, got:\n{h}"
        );
    }

    #[test]
    fn human_render_omits_fallback_note_on_error() {
        // Error states should report the cause, not hedge with a
        // separate fallback advisory. The user's first action is to
        // fix the error, not run a watcher on top of it.
        let h = render_human(&config_error());
        assert!(
            !h.contains("MCP pre-write validation is not attached"),
            "error render must not append the fallback note, got:\n{h}"
        );
    }

    #[test]
    fn human_render_omits_fallback_note_on_unsupported() {
        // Council remediation: on `Unsupported` the watch fallback
        // would not produce findings on out-of-scope files, so the
        // note over-claims fallback coverage. The Unsupported headline
        // and repair hint already explain the gap honestly.
        let h = render_human(&unsupported());
        assert!(
            !h.contains("MCP pre-write validation is not attached"),
            "unsupported render must not append the fallback note — \
             watch would produce no findings on out-of-scope files, \
             got:\n{h}"
        );
    }

    #[test]
    fn human_render_omits_fallback_note_on_needs_action_with_absent_config() {
        // Council remediation: on `NeedsAction` with `ConfigStatus::
        // Absent`, the user's primary next step is `anvil init`. The
        // partial-protection note distracts from that action by
        // advertising a fallback that has no configuration to honour.
        let d = empty(); // config absent + no MCP + nothing else
        let h = render_human(&d);
        assert!(
            !h.contains("MCP pre-write validation is not attached"),
            "needs_action with absent config must defer to init copy, \
             not advertise the watch fallback, got:\n{h}"
        );
    }

    #[test]
    fn needs_action_hint_points_at_start_watch_for_fallback() {
        // LAUNCH-011: when MCP is below `ConfigPresent`, the repair
        // hint should advertise the composed `anvil start --watch`
        // entry-point (a single command produces fallback protection)
        // rather than asking the user to discover `anvil watch`
        // separately.
        let mut d = empty();
        d.config = ConfigStatus::Valid;
        let h = render_human(&d);
        assert!(
            h.contains("anvil start --watch"),
            "NeedsAction hint must advertise `anvil start --watch`, got:\n{h}"
        );
    }

    #[test]
    fn json_render_propagates_offered_watch_tier() {
        // The `watch` field carries the literal label so machine
        // consumers can distinguish "available" (`offered`) from
        // "not part of the picture" (`not_requested`).
        let v = render_json(&watch_offered_no_mcp());
        assert_eq!(v["watch"], "offered");
    }

    // ---------------------------------------------------------------
    // MLP2-051g: `anvil start --verify --why` verbose tier-evidence
    // renderer. Pinned contracts (from the parent spec
    // `plans/specs/2026-05-21-activation-daemon-evidence-wireup.md`
    // §"Council Verdicts" item 9 and the MLP2-051g APS body):
    //
    // - Output starts with `ACTIVATION (verbose)` so an operator
    //   reading stderr can tell at a glance which surface produced
    //   the block.
    // - The flag does NOT perturb the plain `render_human` stdout
    //   block — same diagnostic input must produce identical
    //   `render_human` output regardless of whether the caller also
    //   asks for `render_human_verbose`.
    // - DaemonAttestation surfaces as operator-friendly copy, never
    //   the raw enum tokens (`Unreachable`, `NotProbed`, etc.) the
    //   tracing site emits — support engineers map the trace token
    //   to the runbook copy here.
    // - The `daemon:` per-client line is gated on
    //   `RestartHandshakeVerified`; clients below that tier don't
    //   get a "daemon: …" sub-line because the daemon isn't the
    //   missing piece for them.
    // - Security guard: never leaks raw SessionRecord-shaped fields
    //   (`pid`, `pgid`, `agent_tag` lineage), and never surfaces
    //   filesystem paths beyond what `render_human` already prints.
    // ---------------------------------------------------------------

    #[test]
    fn verbose_render_header_marks_the_block() {
        let h = render_human_verbose(&restart_required());
        assert!(
            h.starts_with("ACTIVATION (verbose)\n"),
            "verbose render must open with the labelled header so stderr is unambiguous, got:\n{h}"
        );
    }

    #[test]
    fn verbose_render_does_not_perturb_render_human_stdout() {
        // The flag is additive — `render_human` (stdout) must be
        // byte-identical with or without the verbose block being
        // generated alongside. Pins the "scripted consumers of
        // `anvil start --verify` parse stdout" contract from the
        // parent spec.
        let cases = [
            restart_required(),
            protecting(),
            watching(),
            unsupported(),
            config_error(),
            handshake_verified_diag(super::super::daemon_evidence::DaemonAttestation::Unreachable),
        ];
        for d in cases {
            let stdout_alone = render_human(&d);
            let _verbose = render_human_verbose(&d);
            let stdout_again = render_human(&d);
            assert_eq!(
                stdout_alone, stdout_again,
                "render_human must be byte-identical regardless of whether \
                 render_human_verbose is also called",
            );
        }
    }

    #[test]
    fn verbose_render_emits_operator_friendly_daemon_copy_not_raw_tokens() {
        // Raw enum tokens like "Unreachable", "Promoted",
        // "AllSurfacesQuarantined" must not leak — operators see the
        // runbook copy instead. Pinned per spec §"SkipReason
        // vocabulary used by the daemon-evidence tracing is presented
        // in operator-friendly copy (not the raw enum tokens)".
        let raw_tokens = [
            "NotProbed",
            "Unreachable",
            "Unenforced",
            "StaleHeartbeat",
            "AllSurfacesQuarantined",
            "Warming",
            "NoParticipatingSurface",
            "Promoted",
        ];
        let attestations = [
            super::super::daemon_evidence::DaemonAttestation::NotProbed,
            super::super::daemon_evidence::DaemonAttestation::Unreachable,
            super::super::daemon_evidence::DaemonAttestation::Unenforced,
            super::super::daemon_evidence::DaemonAttestation::StaleHeartbeat,
            super::super::daemon_evidence::DaemonAttestation::AllSurfacesQuarantined,
            super::super::daemon_evidence::DaemonAttestation::Warming,
            super::super::daemon_evidence::DaemonAttestation::NoParticipatingSurface,
            super::super::daemon_evidence::DaemonAttestation::Promoted,
        ];
        for att in attestations {
            let d = handshake_verified_diag(att);
            let h = render_human_verbose(&d);
            for token in raw_tokens {
                assert!(
                    !h.contains(token),
                    "verbose render must not leak raw enum token {token:?} for \
                     attestation {att:?}, got:\n{h}"
                );
            }
        }
    }

    #[test]
    fn verbose_render_daemon_line_is_only_emitted_when_handshake_verified() {
        // For clients below RestartHandshakeVerified, the daemon
        // attestation is not the missing piece (the missing piece is
        // the restart). Surfacing a "daemon: not running" line under
        // a ConfigPresent client would misdirect remediation.
        let mut below = empty();
        below.config = ConfigStatus::Valid;
        below
            .mcp
            .insert(McpClientId::ClaudeCode, McpTier::ConfigPresent.into());
        below.daemon_attestation = super::super::daemon_evidence::DaemonAttestation::Unreachable;
        let h_below = render_human_verbose(&below);
        // Find the indented per-client block and assert no "daemon:"
        // sub-line appears under it. The diagnostic-level
        // `daemon-attestation:` line is fine — it sits at the
        // ACTIVATION block scope, not under the client.
        let client_block_has_daemon_subline = h_below
            .lines()
            .any(|line| line.trim_start().starts_with("daemon:"));
        assert!(
            !client_block_has_daemon_subline,
            "ConfigPresent client must not get a `daemon:` per-client sub-line, got:\n{h_below}"
        );

        let verified =
            handshake_verified_diag(super::super::daemon_evidence::DaemonAttestation::Unreachable);
        let h_verified = render_human_verbose(&verified);
        let verified_has_daemon_subline = h_verified
            .lines()
            .any(|line| line.trim_start().starts_with("daemon:"));
        assert!(
            verified_has_daemon_subline,
            "RestartHandshakeVerified client must get a `daemon:` per-client sub-line, got:\n{h_verified}"
        );
    }

    #[test]
    fn verbose_render_security_guard_no_raw_session_fields() {
        // Info-leak guard per spec §"Info-leak guard (security)".
        // The renderer reads only `ActivationDiagnostic` +
        // `DaemonAttestation` — none of which carry raw pid / pgid /
        // agent_tag lineage today. The test plants the same names as
        // substrings of last_error to confirm the structural guard:
        // free-form strings the orchestrator chooses to put on
        // `last_error` flow through (already a stdout surface), but
        // there's no separate code path that could smuggle session
        // fields in.
        let mut d =
            handshake_verified_diag(super::super::daemon_evidence::DaemonAttestation::Unenforced);
        // No PID / PGID / lineage on the diagnostic at all — guard
        // confirms the renderer does not invent them.
        let h = render_human_verbose(&d);
        // These token strings would be load-bearing for a real
        // attacker shoulder-surfing a stderr block. Confirm the
        // renderer never emits them as field labels.
        for forbidden in ["pid=", "pgid=", "agent_tag=", "lineage="] {
            assert!(
                !h.contains(forbidden),
                "verbose render must not surface field label {forbidden:?}, got:\n{h}"
            );
        }
        // last_error pass-through is allowed (already a stdout
        // surface — no new disclosure boundary). Pin that the prefix
        // is reflected verbatim so an operator chasing a stderr
        // trace finds the same string.
        d.last_error = Some("orchestrator: probe failed".to_string());
        let h2 = render_human_verbose(&d);
        assert!(
            h2.contains("last_error: orchestrator: probe failed"),
            "last_error must pass through to verbose render, got:\n{h2}",
        );
    }

    #[test]
    fn verbose_render_state_line_matches_plain_render() {
        // The state label is the load-bearing handle a support
        // engineer uses to correlate the verbose stderr block with
        // the stdout headline. Pin that the same diagnostic produces
        // the same `state:` line in both renders.
        let cases = [
            ("protecting", protecting()),
            ("ready_restart_required", restart_required()),
            ("watching", watching()),
            ("unsupported", unsupported()),
            ("error", config_error()),
        ];
        for (label, d) in cases {
            let v = render_human_verbose(&d);
            assert!(
                v.contains(&format!("state: {label}")),
                "verbose render must carry the state label `{label}`, got:\n{v}"
            );
        }
    }

    #[test]
    fn verbose_render_why_summary_names_missing_piece_per_attestation() {
        // Each attestation under `ReadyRestartRequired` maps to a
        // distinct "what's missing?" remediation copy so an operator
        // skimming the verbose block finds the next step without
        // re-reading every tier line. Pinned so a future tightening
        // of one branch does not silently collapse two branches'
        // copy.
        //
        // Command-name policy (PR #1909 review): copy MUST only
        // reference subcommands `anvil intercept` actually ships
        // today — `start --foreground`, `status`, `unblock`. The
        // earlier draft of this test pinned `anvil intercept recover`
        // and bare `anvil intercept start`; both were non-existent
        // / broken invocations the council had already corrected on
        // PR #1848. The test now pins the corrected commands.
        let expected: &[(super::super::daemon_evidence::DaemonAttestation, &str)] = &[
            (
                super::super::daemon_evidence::DaemonAttestation::Unreachable,
                "anvil intercept start --foreground",
            ),
            (
                super::super::daemon_evidence::DaemonAttestation::Unenforced,
                "anvil intercept status",
            ),
            (
                super::super::daemon_evidence::DaemonAttestation::AllSurfacesQuarantined,
                "anvil intercept start --foreground",
            ),
            (
                super::super::daemon_evidence::DaemonAttestation::StaleHeartbeat,
                "anvil intercept start --foreground",
            ),
        ];
        for (att, needle) in expected {
            let d = handshake_verified_diag(*att);
            let h = render_human_verbose(&d);
            assert!(
                h.contains(needle),
                "attestation {att:?} verbose render must name `{needle}`, got:\n{h}"
            );
        }
    }

    /// PR #1909 review (finding 2): `anvil intercept recover` does
    /// NOT exist as a subcommand today. Confirm no code path in the
    /// verbose renderer emits it; the recovery path is `stop and
    /// restart with anvil intercept start --foreground`.
    #[test]
    fn verbose_render_never_names_nonexistent_intercept_recover() {
        let attestations = [
            super::super::daemon_evidence::DaemonAttestation::NotProbed,
            super::super::daemon_evidence::DaemonAttestation::Unreachable,
            super::super::daemon_evidence::DaemonAttestation::Unenforced,
            super::super::daemon_evidence::DaemonAttestation::StaleHeartbeat,
            super::super::daemon_evidence::DaemonAttestation::AllSurfacesQuarantined,
            super::super::daemon_evidence::DaemonAttestation::Warming,
            super::super::daemon_evidence::DaemonAttestation::NoParticipatingSurface,
            super::super::daemon_evidence::DaemonAttestation::Promoted,
        ];
        for att in attestations {
            let d = handshake_verified_diag(att);
            let h = render_human_verbose(&d);
            assert!(
                !h.contains("anvil intercept recover"),
                "attestation {att:?} verbose render must not name the \
                 nonexistent `anvil intercept recover` subcommand, got:\n{h}"
            );
        }
    }

    /// PR #1909 review (finding 3): `anvil intercept start` without
    /// `--foreground` bails today (backgrounded launch arrives with
    /// INTD-002). Every render path that names `anvil intercept
    /// start` must include `--foreground` so operators don't hit an
    /// immediate error.
    #[test]
    fn verbose_render_intercept_start_hints_always_include_foreground() {
        let attestations = [
            super::super::daemon_evidence::DaemonAttestation::NotProbed,
            super::super::daemon_evidence::DaemonAttestation::Unreachable,
            super::super::daemon_evidence::DaemonAttestation::Unenforced,
            super::super::daemon_evidence::DaemonAttestation::StaleHeartbeat,
            super::super::daemon_evidence::DaemonAttestation::AllSurfacesQuarantined,
            super::super::daemon_evidence::DaemonAttestation::Warming,
            super::super::daemon_evidence::DaemonAttestation::NoParticipatingSurface,
            super::super::daemon_evidence::DaemonAttestation::Promoted,
        ];
        for att in attestations {
            let d = handshake_verified_diag(att);
            let h = render_human_verbose(&d);
            // Use the whole verbose block: if any line mentions
            // `anvil intercept start`, the same line MUST include
            // `--foreground` (the bare form bails on launch).
            for line in h.lines() {
                if line.contains("anvil intercept start") {
                    assert!(
                        line.contains("--foreground"),
                        "attestation {att:?} verbose render names bare `anvil intercept start` \
                         (no `--foreground`) — that invocation bails. Line:\n{line}"
                    );
                }
            }
        }
    }

    /// PR #1909 review (finding 4): the `why:` summary must derive
    /// from `protection_state()`, NOT solely from
    /// `daemon_attestation`. For `NeedsAction` (e.g. config absent),
    /// `daemon_attestation` is typically `NotProbed`, so a
    /// daemon-only summary would tell the user to restart their
    /// editor when the actual missing piece is `anvil init` / MCP
    /// install. Pinned so a future refactor cannot silently
    /// regress to the daemon-only dispatch.
    #[test]
    fn verbose_render_why_summary_dispatches_on_protection_state_not_only_daemon() {
        use super::super::diagnostic::ConfigStatus;

        // `NeedsAction` with absent config → `anvil init` is the
        // missing piece, NOT an editor restart. The
        // `daemon_attestation` here is the default `NotProbed`, which
        // the pre-fix code would have used to suggest a restart.
        let mut needs_action_no_config = empty();
        needs_action_no_config.config = ConfigStatus::Absent;
        let h = render_human_verbose(&needs_action_no_config);
        let why_line = h
            .lines()
            .find(|l| l.trim_start().starts_with("why:"))
            .unwrap_or_else(|| panic!("verbose render must include a `why:` line, got:\n{h}"));
        assert!(
            why_line.contains("anvil init"),
            "NeedsAction (no config) why: must name `anvil init`, got: {why_line}"
        );
        assert!(
            !why_line.contains("restart your editor"),
            "NeedsAction (no config) why: must NOT tell the user to restart the editor \
             — the missing piece is config, not the MCP handshake. Got: {why_line}"
        );

        // `Protecting` → no missing piece. Pre-fix the daemon-only
        // dispatch would have surfaced whatever attestation copy
        // happened to be set (probably `Promoted`, but the branch
        // structure was misleading).
        let h_protecting = render_human_verbose(&protecting());
        let why_protecting = h_protecting
            .lines()
            .find(|l| l.trim_start().starts_with("why:"))
            .expect("protecting render has a why: line");
        assert!(
            why_protecting.to_lowercase().contains("no missing piece"),
            "Protecting why: must say `no missing piece`, got: {why_protecting}"
        );

        // `Unsupported` → coverage gap; no remediation in this
        // release. Pre-fix this fell through to the daemon-only
        // dispatch.
        let h_unsup = render_human_verbose(&unsupported());
        let why_unsup = h_unsup
            .lines()
            .find(|l| l.trim_start().starts_with("why:"))
            .expect("unsupported render has a why: line");
        assert!(
            why_unsup.to_lowercase().contains("not yet covered"),
            "Unsupported why: must surface the coverage gap, got: {why_unsup}"
        );

        // `ReadyRestartRequired` still routes through the daemon
        // attestation — that's the one state where the attestation
        // is the load-bearing signal.
        let h_rrr = render_human_verbose(&handshake_verified_diag(
            super::super::daemon_evidence::DaemonAttestation::Unreachable,
        ));
        let why_rrr = h_rrr
            .lines()
            .find(|l| l.trim_start().starts_with("why:"))
            .expect("ready_restart_required render has a why: line");
        assert!(
            why_rrr.contains("anvil intercept start --foreground"),
            "ReadyRestartRequired + Unreachable why: must name `anvil intercept start --foreground`, got: {why_rrr}"
        );
    }
}
