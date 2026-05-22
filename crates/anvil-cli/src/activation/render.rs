//! Rendering for activation diagnostics.
//!
//! Two outputs:
//!
//! - [`render_human`] — block of plain-text lines for terminal display
//!   (used by `anvil status`, `anvil start`, `anvil doctor`).
//! - [`render_json`] — structured JSON block for `--json` output, with a
//!   stable field shape so dashboards and tooling can consume it.
//!
//! Both renderers go through [`super::ProtectionState::headline`] so
//! the headline copy never drifts between surfaces.

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
    let _ = writeln!(out, "  {}", state.headline());
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
    // changes are checked", which over-claimed — this PR ships the
    // baseline contract but does not yet wire watch / check / audit
    // to filter on it. The wording is now future-tense ("future
    // scans will diff against this set") and explicitly notes that
    // the diffing wiring lands in follow-up PRs.
    match (&d.baseline_summary, d.baseline_present) {
        (Some(s), _) if s.secret > 0 => {
            let _ = writeln!(
                out,
                "  baseline: present ({} recorded — {} antipattern, {} secret-shaped)",
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
                "  baseline: present ({} findings recorded; future scans will diff against this set as wiring lands)",
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
            InstallOutcome::Failed { error } => format!("FAILED — {error}"),
        };
        let _ = writeln!(out, "    {}: {line}", client.display_name());
    }
    out
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
        "headline": state.headline(),
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
                "start the intercept daemon with `anvil intercept start --foreground` so pre-write validation can attach; otherwise restart your editor again and re-run `anvil start --verify`."
            }
            DaemonAttestation::Unenforced | DaemonAttestation::NoParticipatingSurface => {
                "the intercept daemon is running but is not enforcing this worktree yet; check `anvil intercept status` for the registered worktree set and re-run `anvil start --verify` after your editor has issued an MCP request."
            }
            DaemonAttestation::StaleHeartbeat => {
                "the intercept daemon's last attestation is stale; restart it with `anvil intercept restart` (or `anvil intercept start --foreground` if it has stopped) and re-run `anvil start --verify`."
            }
            DaemonAttestation::AllSurfacesQuarantined => {
                "the intercept daemon has fenced every session for this worktree; clear the fence with `anvil intercept recover` once you understand the cause, then re-run `anvil start --verify`."
            }
            DaemonAttestation::Warming => {
                "the intercept daemon is transitioning (warming / draining); re-run `anvil start --verify` in a few seconds."
            }
            // `NotProbed` is the genuine pre-restart case — the
            // diagnostic never reached the daemon probe because no
            // client was at `RestartHandshakeVerified` yet.
            // `Promoted` is logically unreachable at this branch
            // (`protection_state()` returns `Protecting` instead of
            // `ReadyRestartRequired` when any client is at
            // `LiveValidation`); keep the original copy as a
            // belt-and-braces fallback rather than panic.
            DaemonAttestation::NotProbed | DaemonAttestation::Promoted => {
                "restart your editor or agent so the MCP server attaches, then re-run `anvil start --verify`."
            }
        }),
        ProtectionState::Watching => {
            // Council remediation: the next step depends on whether
            // any MCP tier is already past `ConfigPresent`. If the
            // server is already configured and startable, telling
            // the user to run `anvil mcp install` is wrong — they
            // need to restart their editor.
            //
            // Invariant (debug-asserted below): `LiveValidation` is
            // unreachable here because `protection_state` returns
            // `Protecting` first when any client is at that tier.
            // The match arms below only handle tiers strictly weaker
            // than `LiveValidation`; if a future refactor breaks the
            // invariant, the assertion fires in debug builds.
            let highest_mcp = d.mcp.values().map(|r| r.tier).max();
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
                    "watch is the save-time fallback — to upgrade to pre-write validation, run `anvil mcp install` for Cursor or Claude Code.",
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
                    "run `anvil start` to wire Cursor and Claude Code MCP paths, or `anvil start --watch` for save-time fallback protection.",
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
                "Anvil does not yet cover this repo's languages in the current release. Architecture / antipattern checks will not produce findings on files in unsupported languages; coverage expands as language packs ship.",
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
    /// every surface `Quarantined`. Recovery routes through
    /// `anvil intercept recover`, not editor restart.
    #[test]
    fn ready_restart_required_with_all_quarantined_points_at_intercept_recover() {
        let d = handshake_verified_diag(
            super::super::daemon_evidence::DaemonAttestation::AllSurfacesQuarantined,
        );
        let h = render_human(&d);
        assert!(
            h.contains("anvil intercept recover"),
            "AllSurfacesQuarantined hint must name `anvil intercept recover`: {h}"
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
    /// The remediation is to restart the daemon, not the editor.
    #[test]
    fn ready_restart_required_with_stale_heartbeat_points_at_daemon_restart() {
        let d = handshake_verified_diag(
            super::super::daemon_evidence::DaemonAttestation::StaleHeartbeat,
        );
        let h = render_human(&d);
        assert!(
            h.contains("anvil intercept restart") || h.contains("anvil intercept start"),
            "StaleHeartbeat hint must name an `anvil intercept` restart/start command: {h}"
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
        // Belt-and-braces: the headline must carry the partial-state
        // language. If a future copy edit drops it, the absence of
        // the note plus a "fully protected"-style headline would be
        // a regression.
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
}
