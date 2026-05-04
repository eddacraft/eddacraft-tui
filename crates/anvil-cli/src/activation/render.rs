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
use super::state::ProtectionState;

/// Render the diagnostic as a multi-line plain-text block ending with
/// a newline. Caller decides whether to print to stdout, stderr, or
/// embed inside another report.
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
        for (client, tier) in &d.mcp {
            let _ = writeln!(out, "    {}: {}", client.display_name(), tier.label());
        }
    }

    let _ = writeln!(out, "  watch: {}", d.watch.label());
    let _ = writeln!(
        out,
        "  baseline: {}",
        if d.baseline_present {
            "present"
        } else {
            "absent"
        }
    );

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

/// Build the JSON value for embedding inside a parent JSON document
/// (e.g. `anvil status --json`'s `activation` field).
///
/// The shape is stable contract — keys are: `state`, `headline`,
/// `config`, `mcp` (array of `{client, tier}` objects), `watch`,
/// `baseline_present`, `last_error`, `all_languages_unsupported`,
/// `repo_languages` (array of `{name, files_seen, coverage_tier, basis}`).
/// Tooling consumers may rely on this set; downstream PRs add fields,
/// they do not rename or remove existing ones.
///
/// The body uses `serde_json::json!` so the value is constructed
/// directly from primitives — no fallible `to_value` round-trip and
/// no panic path for the binary to inherit.
pub fn render_json(d: &ActivationDiagnostic) -> Value {
    let state = d.protection_state();
    let mcp: Vec<Value> = d
        .mcp
        .iter()
        .map(|(c, t)| json!({"client": c.label(), "tier": t.label()}))
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
    json!({
        "state": state.label(),
        "headline": state.headline(),
        "config": d.config.label(),
        "mcp": mcp,
        "watch": d.watch.label(),
        "baseline_present": d.baseline_present,
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
    match state {
        ProtectionState::Protecting => None,
        ProtectionState::ReadyRestartRequired => Some(
            "restart your editor or agent so the MCP server attaches, then re-run `anvil start --verify`.",
        ),
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
            let highest_mcp = d.mcp.values().copied().max();
            debug_assert!(
                !matches!(highest_mcp, Some(McpTier::LiveValidation)),
                "Watching unreachable when MCP at LiveValidation"
            );
            if matches!(
                highest_mcp,
                Some(McpTier::ServerStartable | McpTier::RestartRequired)
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
            } else if d.mcp.values().all(|t| *t < McpTier::ConfigPresent) {
                Some(
                    "run `anvil start` to wire Cursor and Claude Code MCP paths, or `anvil watch` for save-time fallback.",
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
            last_error: None,
            all_languages_unsupported: false,
            language_profile: super::super::language_profile::RepoLanguageProfile::default(),
        }
    }

    fn protecting() -> ActivationDiagnostic {
        let mut d = empty();
        d.config = ConfigStatus::Valid;
        d.mcp.insert(McpClientId::Cursor, McpTier::LiveValidation);
        d
    }

    fn restart_required() -> ActivationDiagnostic {
        let mut d = empty();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::ClaudeCode, McpTier::RestartRequired);
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
        d.mcp.insert(McpClientId::Cursor, McpTier::ConfigPresent);
        d.mcp
            .insert(McpClientId::ClaudeCode, McpTier::RestartRequired);
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
        d.mcp.insert(McpClientId::Cursor, McpTier::ServerStartable);
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
}
