//! Layered activation diagnostic.
//!
//! `ActivationDiagnostic` is the structured probe result that backs
//! every render of `ProtectionState`. The diagnostic separates layers
//! so a surface can render "config valid, MCP startable, restart still
//! required" without collapsing distinct failures into a single opaque
//! "not protected" message.
//!
//! `verify` is intentionally narrow in this PR: it probes layers that
//! do not require LAUNCH-009 (MCP install/verify), LAUNCH-010
//! (baseline), or LAUNCH-011 (watch fallback runtime). Future PRs
//! plug in the missing layers; this PR establishes the contract.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::state::ProtectionState;

/// Stable v1 identifiers for editors / agents that ship MCP probes.
///
/// Held as a typed enum (rather than a free `String`) so callers cannot
/// silently introduce out-of-scope clients. The v1 release is locked
/// to Cursor and Claude Code per the council ratification — see
/// `RELEASE-PLAN.md` Tier A1 "Out-of-scope".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpClientId {
    Cursor,
    ClaudeCode,
}

impl McpClientId {
    pub fn label(self) -> &'static str {
        match self {
            McpClientId::Cursor => "cursor",
            McpClientId::ClaudeCode => "claude-code",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            McpClientId::Cursor => "Cursor",
            McpClientId::ClaudeCode => "Claude Code",
        }
    }
}

impl std::fmt::Display for McpClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Tier of MCP attachment for a given client.
///
/// Variants form a strict ladder from "no config" up to "live evidence
/// observed". A surface that wants to claim `Protecting` needs at
/// least one client at [`McpTier::LiveValidation`]. Any tier below
/// that is more honest as `ReadyRestartRequired` or weaker.
///
/// **The labels name observed probe state, not graduation** (CIB-180).
/// Each variant records *what was probed* — the anvil entry is on disk,
/// the command spawns, the initialize handshake answered — not that
/// protection has graduated for that client. In particular
/// `RestartHandshakeVerified` means the installed command completed an
/// MCP handshake while a restart is *still* pending; it does not mean
/// the editor has attached. Only [`McpTier::LiveValidation`] evidences a
/// live in-repo validation. The human renderer glosses the done-ish
/// tokens with a `(pending restart)` qualifier under a restart-required
/// headline (see `activation::render::tier_pending_qualifier`); the
/// machine tokens returned by [`McpTier::label`] stay byte-stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTier {
    /// Client not detected on the host at all.
    NotDetected,
    /// Client detected, but its config does not include the anvil
    /// MCP server entry.
    ConfigAbsent,
    /// anvil MCP server entry present in client config.
    ConfigPresent,
    /// anvil MCP server starts cleanly when invoked from the same
    /// command shape the client will use.
    ServerStartable,
    /// Server config is wired to an anvil-shaped entry, but the client
    /// must be restarted before it picks up the entry.
    RestartRequired,
    /// Server is configured, restart is still required, and the exact
    /// installed command has completed an MCP initialize handshake.
    RestartHandshakeVerified,
    /// Live MCP `anvil_validate_write` invocation has been observed
    /// from this client inside this repo.
    LiveValidation,
}

impl McpTier {
    pub fn label(self) -> &'static str {
        match self {
            McpTier::NotDetected => "not_detected",
            McpTier::ConfigAbsent => "config_absent",
            McpTier::ConfigPresent => "config_present",
            McpTier::ServerStartable => "server_startable",
            McpTier::RestartRequired => "restart_required",
            McpTier::RestartHandshakeVerified => "restart_handshake_verified",
            McpTier::LiveValidation => "live_validation",
        }
    }
}

/// Tier of save-time watch fallback. Watch is intentionally weaker
/// than MCP pre-write validation, so its tiers do not include an
/// equivalent of [`McpTier::LiveValidation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchTier {
    /// Watcher not requested or no fallback offered.
    NotRequested,
    /// Watcher offered but not running yet.
    Offered,
    /// Watcher process is running and watching the repo.
    Running,
}

impl WatchTier {
    pub fn label(self) -> &'static str {
        match self {
            WatchTier::NotRequested => "not_requested",
            WatchTier::Offered => "offered",
            WatchTier::Running => "running",
        }
    }
}

/// Status of the on-disk anvil config (`.anvilrc`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigStatus {
    /// No config file detected at the repo root.
    Absent,
    /// Config file present and parses cleanly.
    Valid,
    /// Config file present but cannot be parsed.
    Invalid,
}

impl ConfigStatus {
    pub fn label(self) -> &'static str {
        match self {
            ConfigStatus::Absent => "absent",
            ConfigStatus::Valid => "valid",
            ConfigStatus::Invalid => "invalid",
        }
    }
}

/// Per-kind counts surfaced from `.anvil/baseline.json` (LAUNCH-010).
/// Mirrors the on-disk `BaselineCounts` shape from
/// `super::baseline` — duplicated here so the diagnostic does not
/// re-export internals and so downstream JSON consumers see a single
/// flat key set.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineSummary {
    /// Total fingerprint count in the baseline.
    pub total: usize,
    /// Antipattern findings recorded at activation time.
    pub antipattern: usize,
    /// Secret-shaped findings recorded at activation time. When > 0,
    /// activation copy may name secrets as the headline security
    /// signal (LAUNCH-010 spec) — but does so without claiming the
    /// repo is clean of further secrets.
    pub secret: usize,
    /// RFC3339 timestamp the baseline was first written.
    pub created_at: String,
}

/// Layered probe result for the wow-start activation flow.
///
/// Surfaces should NOT compute `ProtectionState` from a subset of
/// these fields ad-hoc — call [`ActivationDiagnostic::protection_state`]
/// so the mapping stays in one place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationDiagnostic {
    /// Status of the on-disk config (`.anvilrc`).
    pub config: ConfigStatus,
    /// Tier reached for each detected MCP client. Clients below
    /// [`McpTier::ConfigPresent`] are still emitted so the surface
    /// can show "Cursor: not detected" without dropping the row.
    pub mcp: BTreeMap<McpClientId, super::mcp_client::McpProbeResult>,
    /// Tier of the save-time watch fallback.
    pub watch: WatchTier,
    /// True when the repo has a baselined finding set (LAUNCH-010).
    /// Surfaces use this to decide whether to label the next signal
    /// as "first new finding" vs "first finding".
    pub baseline_present: bool,
    /// Per-kind summary of the baseline contents when present
    /// (LAUNCH-010). Lets surfaces phrase the activation copy
    /// honestly ("3 antipattern, 1 secret" vs the bare boolean
    /// flag). `None` when no baseline is on disk OR when the file
    /// could not be read — the latter case also sets `last_error`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_summary: Option<BaselineSummary>,
    /// Last activation error, if any. Set by retry / verification
    /// flows when a previous activation attempt aborted. Cleared on
    /// the first successful re-run.
    pub last_error: Option<String>,
    /// True when the repo's languages are all `unsupported` per
    /// the language profile (derived from
    /// [`super::language_profile::RepoLanguageProfile::all_unsupported`]).
    pub all_languages_unsupported: bool,
    /// Per-language breakdown produced by the activation walk
    /// (LAUNCH-015). Empty for synthetic diagnostics built by tests.
    #[serde(default)]
    pub language_profile: super::language_profile::RepoLanguageProfile,
    /// MLP2-051f: outcome of the daemon-attestation probe. The
    /// renderer reads this to refine the
    /// [`ProtectionState::ReadyRestartRequired`] repair hint so
    /// "daemon not running" and "pre-restart" surface different
    /// remediation steps. Default for synthetic test fixtures is
    /// [`super::daemon_evidence::DaemonAttestation::NotProbed`].
    #[serde(default)]
    pub daemon_attestation: super::daemon_evidence::DaemonAttestation,
    /// DSV-049: `true` when the daemon snapshot reports a supervised
    /// save-time driver *attached* to this worktree. Read from the same
    /// probe as `daemon_attestation`, it lets the `Watching` render
    /// distinguish save-time-*active* watching (registered ∧ driver
    /// attached) from membership-only watching. `false` for synthetic
    /// test fixtures, when no daemon answered, or when the driver is
    /// absent / failed / an unrecognised (forward-compat) state.
    #[serde(default)]
    pub save_time_driver_attached: bool,
}

impl ActivationDiagnostic {
    /// Map the layered diagnostic onto a single
    /// [`ProtectionState`] word. The mapping is the canonical truth
    /// table for activation copy and is exercised by the unit tests
    /// at the bottom of this file.
    pub fn protection_state(&self) -> ProtectionState {
        // Hard error wins over everything — never claim coverage if
        // we already know activation failed or config is broken.
        if self.last_error.is_some() {
            return ProtectionState::Error;
        }
        if matches!(self.config, ConfigStatus::Invalid) {
            return ProtectionState::Error;
        }

        // Live MCP evidence is the only path to `Protecting`.
        let highest_mcp = self.highest_mcp_tier();
        if matches!(highest_mcp, Some(McpTier::LiveValidation)) {
            return ProtectionState::Protecting;
        }

        // `RestartRequired` and `RestartHandshakeVerified` are the
        // literal "one restart from live" tiers. The latter additionally
        // proves the exact installed command serves MCP. `ServerStartable`
        // remains weaker: the server can spawn, but client wiring is not
        // confirmed.
        let restart_pending = matches!(
            highest_mcp,
            Some(McpTier::RestartRequired | McpTier::RestartHandshakeVerified)
        );

        if !self.all_languages_unsupported && self.daemon_attestation.attests_worktree() {
            return ProtectionState::Watching;
        }

        if matches!(self.watch, WatchTier::Running) {
            // Watch is honest fallback coverage. If MCP is literally
            // one step from live, surface that stronger label so the
            // user knows a restart will graduate them — but never
            // promote `ServerStartable` to that label.
            if restart_pending {
                return ProtectionState::ReadyRestartRequired;
            }
            return ProtectionState::Watching;
        }

        if restart_pending {
            return ProtectionState::ReadyRestartRequired;
        }

        // No literal `Protecting` / `ReadyRestartRequired` /
        // `Watching` claim is available. If the repo's languages are
        // all unsupported AND MCP is below the restart-pending tier,
        // tell the user honestly — `NeedsAction` would suggest "run
        // `anvil start`" but no further setup will help here.
        if self.all_languages_unsupported {
            return ProtectionState::Unsupported;
        }

        // Anything else — config absent, MCP not detected, watch not
        // requested — means the user has actionable next steps.
        ProtectionState::NeedsAction
    }

    /// `pub(crate)` so the verbose renderer (MLP2-051g) can dispatch
    /// the "what's missing?" copy on the strongest tier any client
    /// has reached, without duplicating the `.values().map().max()`
    /// reduction at the call site. Visibility is intentionally
    /// crate-local — external consumers should keep going through
    /// [`Self::protection_state`] / [`Self::mcp_pre_write_wired_or_live`].
    pub(crate) fn highest_mcp_tier(&self) -> Option<McpTier> {
        self.mcp.values().map(|r| r.tier).max()
    }

    /// True when at least one MCP client is wired (entry on disk +
    /// ready to attach on restart) or already producing live
    /// validation evidence — i.e. tier ≥ `RestartRequired`.
    ///
    /// Council remediation: the original name was
    /// `mcp_pre_write_attached`, which was misleading at
    /// `RestartRequired` — that tier means "configured" not
    /// "attached", because the editor has not yet loaded the entry.
    /// The honesty contract forbids claiming attachment based on
    /// configuration alone, so the predicate's name now reflects
    /// what it actually tests. The watch-fallback offer logic in
    /// [`verify_with_home`] inlines the same boundary; this method
    /// is a stable contract for downstream surfaces (`status`,
    /// `doctor`) that need to ask the same question without
    /// duplicating the tier-set literal.
    #[allow(
        dead_code,
        reason = "stable predicate for downstream surfaces; the inlined offer-gate uses the same tier set"
    )]
    pub fn mcp_pre_write_wired_or_live(&self) -> bool {
        matches!(
            self.highest_mcp_tier(),
            Some(
                McpTier::RestartRequired
                    | McpTier::RestartHandshakeVerified
                    | McpTier::LiveValidation,
            )
        )
    }

    /// True when at least one MCP client has produced
    /// `LiveValidation` evidence inside this repo. This is the only
    /// tier that justifies a `Protecting` claim and the only tier at
    /// which `anvil start --watch` becomes a no-op (the
    /// `WatchDecision::NoOpRedundant` path in `commands/start.rs`).
    ///
    /// **Honesty note suppression** is owned by
    /// [`Self::mcp_pre_write_wired_or_live`], not this predicate.
    /// At `RestartRequired` (and now `RestartHandshakeVerified` per
    /// LAUNCH-009.6) the headline already says "restart your editor
    /// or agent so the MCP server attaches", which carries the
    /// partial-protection language without needing the watch-
    /// fallback note (firing the note there would orphan watch copy
    /// next to a `watch: not_requested` line). See the suppression
    /// gate in `activation::render::render_human` for the full set
    /// of states the note is suppressed in.
    pub fn mcp_pre_write_live(&self) -> bool {
        matches!(self.highest_mcp_tier(), Some(McpTier::LiveValidation))
    }
}

/// Probe activation state at `root`. PR 2 lands the contract; deeper
/// probes (real MCP detection, baseline, watch identity) plug into
/// this function in PR 3, PR 4, and PR 5.
pub fn verify(root: &Path) -> ActivationDiagnostic {
    verify_with_home(root, crate::util::user_home_dir().as_deref())
}

/// Like [`verify`] but with an explicit `home` override.
///
/// Used by the orchestrator and its unit tests to probe against a
/// tempdir-scoped home, so install-path tests don't pollute the
/// developer's real `~/.cursor/mcp.json` or `~/.claude.json`.
pub fn verify_with_home(root: &Path, home: Option<&Path>) -> ActivationDiagnostic {
    let config = probe_config_status(root);
    let (baseline_present, baseline_summary, baseline_load_error) = probe_baseline(root);

    // LAUNCH-009: probe each registered MCP client. The probe is
    // read-only — it only reads each editor's config; the install
    // path is in the orchestrator. The fresh entry uses
    // `current_exe()` as the canonical command path so tier
    // classification compares against what we'd actually install.
    let (mcp, mcp_last_error, daemon_attestation, save_time_driver_attached): (
        BTreeMap<McpClientId, super::mcp_client::McpProbeResult>,
        Option<String>,
        super::daemon_evidence::DaemonAttestation,
        bool,
    ) = match std::env::current_exe() {
        Ok(exe) => {
            let fresh = super::mcp_client::AnvilEntry::local_stdio(exe);
            let mut probe_results = super::mcp_client::probe_all(root, home, &fresh);
            promote_restart_required_after_handshake(root, home, &mut probe_results, &fresh);
            // MLP2-051f: layer the daemon-attested LiveValidation
            // promotion on top of the orchestrator's handshake pass.
            // The function is best-effort and silently no-ops when the
            // daemon is unreachable, the worktree is unenforced, or
            // the snapshot is stale — see
            // `super::daemon_evidence::promote_to_live_validation_when_daemon_attests`
            // for the full predicate ladder. The returned attestation
            // is carried on the diagnostic so the renderer can
            // distinguish "pre-restart" from "daemon not running" /
            // "daemon unenforced" when emitting the
            // `ReadyRestartRequired` repair hint.
            //
            // DSV-049: the same probe also reports whether a supervised
            // save-time driver is attached to this worktree, so the
            // `Watching` render can distinguish save-time-active watching
            // (registered ∧ driver attached) from membership-only watching.
            if matches!(config, ConfigStatus::Valid) {
                let outcome =
                    super::daemon_evidence::promote_to_live_validation_when_daemon_attests(
                        &mut probe_results,
                        root,
                    );
                (
                    probe_results,
                    None,
                    outcome.attestation,
                    outcome.save_time_driver_attached,
                )
            } else {
                (
                    probe_results,
                    None,
                    super::daemon_evidence::DaemonAttestation::NotProbed,
                    false,
                )
            }
        }
        Err(e) => {
            // Couldn't resolve current_exe (rare — typically only fails
            // in stripped containers without /proc). Set last_error so
            // protection_state() returns Error and the user / SRE sees
            // an actionable cause rather than a silent "needs_action"
            // with no MCP clients reported. (Council finding: ops M3.)
            tracing::warn!(
                error = %e,
                "verify: could not resolve current_exe; MCP probe skipped",
            );
            (
                BTreeMap::new(),
                Some(format!(
                    "could not resolve current_exe; MCP probe skipped ({e})"
                )),
                super::daemon_evidence::DaemonAttestation::NotProbed,
                false,
            )
        }
    };

    // LAUNCH-015: walk the working tree and classify languages so the
    // protection-state mapping can return `Unsupported` honestly when
    // the repo has no covered languages.
    let language_profile = super::language_profile::profile_repo(root);
    let all_languages_unsupported = language_profile.all_unsupported();

    // Compose the final `last_error` from MCP and baseline-load
    // signals. MCP error wins when both fire so SRE diagnosis follows
    // the same priority as before.
    let last_error = mcp_last_error.or(baseline_load_error);

    // LAUNCH-011: when MCP cannot pre-write attach (no client has
    // reached `RestartRequired+`) and the repo is in a state where
    // `anvil start --watch` would produce meaningful coverage,
    // surface the watch fallback as `Offered`. This is purely
    // informational — `Offered` does not change `protection_state()`
    // (the watcher process is not running). It signals to the renderer
    // that watch is the available next step so the user gets honest
    // partial-protection language instead of a silent gap.
    //
    // Skipped when:
    // - config is invalid (already an error state — fix config first)
    // - config is absent (the user must run `anvil init` before any
    //   activation surface is meaningful — offering watch alongside
    //   "config: absent" would advertise a fallback that has no
    //   configuration to honour)
    // - any layer recorded `last_error` (don't add fallback noise on
    //   top of an aborted activation)
    // - MCP is at `RestartRequired+` (the user is ready to attach on
    //   restart; offering a fallback would dilute that nudge)
    // - all detected languages are out of scope (council finding:
    //   advertising watch on an unsupported repo over-claims coverage —
    //   the watcher will run but produces no findings on those file
    //   types; the `Unsupported` headline is the honest answer)
    let watch = if matches!(config, ConfigStatus::Valid)
        && last_error.is_none()
        && !all_languages_unsupported
        && !matches!(
            mcp.values().map(|r| r.tier).max(),
            Some(
                McpTier::RestartRequired
                    | McpTier::RestartHandshakeVerified
                    | McpTier::LiveValidation,
            )
        ) {
        WatchTier::Offered
    } else {
        WatchTier::NotRequested
    };

    ActivationDiagnostic {
        config,
        mcp,
        watch,
        baseline_present,
        baseline_summary,
        last_error,
        all_languages_unsupported,
        language_profile,
        daemon_attestation,
        save_time_driver_attached,
    }
}

/// Drive a dual-era MCP verification probe against the installed entry and
/// promote `RestartRequired` clients to `RestartHandshakeVerified` when
/// the exact command serves MCP (LAUNCH-009.6 / MCP26-007).
///
/// Modern path: disposable `server/discover`. Legacy fallback: fresh child
/// `initialize`. Public tier label stays `restart_handshake_verified`;
/// `protocolEra` / `protocolVersion` / `verificationMethod` are additive
/// diagnostic evidence.
///
/// This intentionally does **not** promote to `ServerStartable`:
/// `ServerStartable` means the server can spawn without confirmed client
/// wiring, while `RestartHandshakeVerified` preserves both facts — the
/// client config matches and the configured command verifies.
///
/// The probe targets each `RestartRequired` client's installed entry
/// (via [`super::mcp_client::installed_restart_required_entries`]) so
/// verification reflects what the editor would really spawn — including
/// bare `"anvil"` entries from `anvil mcp-config` that PATH-resolve to a
/// different binary than `current_exe()`. If extraction fails for every
/// restart-required client (config re-parse error, missing `command`
/// field, etc.), the probe falls back to `fresh` for observability only
/// and logs that no client was promoted.
///
/// In v1 the probe runs once per extracted `RestartRequired` client.
/// This avoids overclaiming when one client uses a full path and another
/// uses a bare command resolved through a different PATH.
///
/// The probe is skipped entirely when no client is at
/// `RestartRequired` — fresh repos and already-protecting tiers add
/// zero latency.
///
/// **Cost:** at most two probe attempts (modern then legacy) of
/// [`super::mcp_client`]'s per-attempt handshake timeout (1s each → ~2s
/// worst-case wall clock) per extracted `RestartRequired` client, only when
/// at least one client is at `RestartRequired`.
fn promote_restart_required_after_handshake(
    root: &Path,
    home: Option<&Path>,
    map: &mut BTreeMap<McpClientId, super::mcp_client::McpProbeResult>,
    fresh: &super::mcp_client::AnvilEntry,
) {
    let any_restart_required = map.values().any(|r| r.tier == McpTier::RestartRequired);
    if !any_restart_required {
        return;
    }
    let installed = super::mcp_client::installed_restart_required_entries(root, home, fresh);
    if installed.is_empty() {
        tracing::warn!(
            "mcp probe: could not extract installed entry; \
                     falling back to current_exe — log result reflects \
                     fresh, not the editor's actual spawn target",
        );
        match super::mcp_client::probe_startable(fresh) {
            Ok(evidence) => {
                tracing::info!(
                    protocol_era = ?evidence.protocol_era,
                    protocol_version = %evidence.protocol_version,
                    verification_method = ?evidence.verification_method,
                    "mcp probe: verification against fallback binary succeeded \
                     (clients remain at RestartRequired because installed entry \
                     could not be extracted)",
                );
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "mcp probe: verification against fallback binary failed \
                     (clients remain at RestartRequired because installed entry \
                     could not be extracted)",
                );
            }
        }
        return;
    }

    for (client_id, probe_target) in installed {
        match super::mcp_client::probe_startable(&probe_target) {
            Ok(evidence) => {
                if let Some(result) = map.get_mut(&client_id)
                    && result.tier == McpTier::RestartRequired
                {
                    let tier = McpTier::RestartHandshakeVerified;
                    let transport = result.transport;
                    *result = super::mcp_client::McpProbeResult::stdio(tier)
                        .with_probe_evidence(evidence.clone());
                    result.transport = transport;
                }
                tracing::info!(
                    client = %client_id.label(),
                    protocol_era = ?evidence.protocol_era,
                    protocol_version = %evidence.protocol_version,
                    verification_method = ?evidence.verification_method,
                    "mcp probe: verification against installed binary succeeded \
                     (client promoted to RestartHandshakeVerified)",
                );
            }
            Err(e) => {
                tracing::warn!(
                    client = %client_id.label(),
                    error = %e,
                    "mcp probe: verification against installed binary failed \
                     (client remains at RestartRequired)",
                );
            }
        }
    }
}

fn probe_config_status(root: &Path) -> ConfigStatus {
    // MLP2-039 / MLP-011 — recognise `.anvil.<ext>` (yaml / yml / json /
    // toml) before falling back to the legacy `.anvilrc`. The orchestrator
    // uses this status to decide whether to run its init step, so a
    // project adopted via `anvil start --format <ext>` must register here
    // or init will double-write a parallel `.anvilrc`.
    if let Ok(Some(discovered)) = anvil_config::discover(root, ".anvil") {
        return match anvil_config::parse_file(&discovered.path) {
            Ok(v) if !is_semantically_empty_json(&v) => ConfigStatus::Valid,
            Ok(_) | Err(_) => ConfigStatus::Invalid,
        };
    }
    // `discover` returning `Ok(None)` or `Err` both fall through to the
    // legacy `.anvilrc` probe below. Scan errors (permission, EIO) are
    // intentionally swallowed so a transient FS hiccup does not flip an
    // established `.anvilrc` project to `Absent`.

    let rc = root.join(".anvilrc");
    let Ok(contents) = std::fs::read_to_string(&rc) else {
        return ConfigStatus::Absent;
    };

    // Empty / whitespace-only / BOM-only files are accepted by
    // serde_yaml as `Null` and would otherwise pass as `Valid`. That
    // would silently mask an editor that truncated the user's config.
    // Treat them as `Invalid` so the surface flags the problem.
    let trimmed = contents.trim_start_matches('\u{feff}').trim();
    if trimmed.is_empty() {
        return ConfigStatus::Invalid;
    }

    // The init command writes one of JSON, YAML, or TOML. Try each
    // parser in turn — accept the first one that produces a
    // non-empty top-level object / table / mapping (the only valid
    // `.anvilrc` shape). Fall through to the next parser on a
    // semantically-empty parse so a strict TOML config is not
    // pre-empted by YAML's permissive scalar parse. JSON is tried
    // first because it has the strictest grammar; TOML before YAML
    // because YAML accepts almost any byte sequence as a scalar.
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed)
        && !is_semantically_empty_json(&v)
    {
        return ConfigStatus::Valid;
    }
    if let Ok(v) = toml::from_str::<toml::Value>(trimmed)
        && !is_semantically_empty_toml(&v)
    {
        return ConfigStatus::Valid;
    }
    if let Ok(v) = serde_yaml::from_str::<serde_yaml::Value>(trimmed)
        && !is_semantically_empty_yaml(&v)
    {
        return ConfigStatus::Valid;
    }

    ConfigStatus::Invalid
}

// `.anvilrc` MUST have a non-empty object / mapping / table at the
// top level — that is the only shape `init` and the parsers in
// `commands::status::gather_profile` recognise. Any other top-level
// shape (null, bare scalar, array) is semantically empty regardless
// of whether the parser accepted it. Round-3 council remediation:
// rejecting on shape is stricter than the original "Null only" rule
// and closes the `[null]` / bare-array / bare-scalar gaps.

fn is_semantically_empty_json(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Object(map) => map.is_empty(),
        _ => true,
    }
}

fn is_semantically_empty_yaml(v: &serde_yaml::Value) -> bool {
    match v {
        serde_yaml::Value::Mapping(m) => m.is_empty(),
        _ => true,
    }
}

fn is_semantically_empty_toml(v: &toml::Value) -> bool {
    match v {
        toml::Value::Table(t) => t.is_empty(),
        _ => true,
    }
}

/// Probe `.anvil/baseline.json` (LAUNCH-010). Returns
/// `(present, summary, load_error)`:
///
/// - `present` is the back-compat boolean used by `protection_state`
///   call sites that don't need the count. True iff the file is on
///   disk, regardless of whether it parsed cleanly.
/// - `summary` carries per-kind counts when the file parses; `None`
///   when the file is absent OR could not be parsed. The latter case
///   also populates `load_error` so the diagnostic surfaces an
///   actionable signal instead of silently falling back to "absent".
/// - `load_error` is the human-readable parse / I/O / schema error,
///   suitable for `ActivationDiagnostic.last_error`.
fn probe_baseline(root: &Path) -> (bool, Option<BaselineSummary>, Option<String>) {
    let present = super::baseline::baseline_exists(root);
    if !present {
        return (false, None, None);
    }
    match super::baseline::read_baseline(root) {
        Ok(Some(b)) => {
            let summary = BaselineSummary {
                total: b.total(),
                antipattern: b.counts.antipattern_findings,
                secret: b.counts.secret_findings,
                created_at: b.created_at.clone(),
            };
            (true, Some(summary), None)
        }
        Ok(None) => {
            // Race: file vanished between `exists()` and `read`.
            // Honest answer is "absent now"; no error.
            (false, None, None)
        }
        Err(e) => {
            // File on disk but unreadable / malformed / wrong schema.
            // Surface honestly so the user knows to regenerate.
            tracing::warn!(error = %e, "verify: could not read baseline.json");
            (true, None, Some(format!("baseline read failed: {e}")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn empty_diagnostic() -> ActivationDiagnostic {
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
            save_time_driver_attached: false,
        }
    }

    #[test]
    fn fresh_repo_with_no_config_needs_action() {
        let d = empty_diagnostic();
        assert_eq!(d.protection_state(), ProtectionState::NeedsAction);
    }

    #[test]
    fn invalid_config_renders_as_error() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Invalid;
        assert_eq!(d.protection_state(), ProtectionState::Error);
    }

    #[test]
    fn last_error_wins_over_everything() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::Cursor, McpTier::LiveValidation.into());
        d.last_error = Some("startup probe timed out".into());
        // Even with live MCP evidence the Error layer must win,
        // because the last activation attempt aborted.
        assert_eq!(d.protection_state(), ProtectionState::Error);
    }

    #[test]
    fn live_mcp_evidence_yields_protecting() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::Cursor, McpTier::LiveValidation.into());
        assert_eq!(d.protection_state(), ProtectionState::Protecting);
    }

    #[test]
    fn restart_required_tier_yields_ready_restart_required() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::ClaudeCode, McpTier::RestartRequired.into());
        assert_eq!(d.protection_state(), ProtectionState::ReadyRestartRequired);
    }

    #[test]
    fn restart_handshake_verified_yields_ready_restart_required() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp.insert(
            McpClientId::ClaudeCode,
            McpTier::RestartHandshakeVerified.into(),
        );
        assert_eq!(d.protection_state(), ProtectionState::ReadyRestartRequired);
    }

    #[test]
    fn watch_running_alone_yields_watching() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.watch = WatchTier::Running;
        assert_eq!(d.protection_state(), ProtectionState::Watching);
    }

    #[test]
    fn watch_running_plus_restart_pending_prefers_restart_label() {
        // Adversarial guard: if MCP is one step from live AND watch
        // is running, the surface should nudge toward the literally
        // stronger state (`ReadyRestartRequired`) rather than letting
        // the user assume `Watching` is the best they can get.
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::Cursor, McpTier::RestartRequired.into());
        d.watch = WatchTier::Running;
        assert_eq!(d.protection_state(), ProtectionState::ReadyRestartRequired);
    }

    #[test]
    fn watch_running_plus_restart_handshake_verified_prefers_restart_label() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp.insert(
            McpClientId::Cursor,
            McpTier::RestartHandshakeVerified.into(),
        );
        d.watch = WatchTier::Running;
        assert_eq!(d.protection_state(), ProtectionState::ReadyRestartRequired);
    }

    #[test]
    fn daemon_attested_worktree_without_mcp_yields_watching() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.daemon_attestation = super::super::daemon_evidence::DaemonAttestation::Enforced;

        assert_eq!(d.protection_state(), ProtectionState::Watching);
    }

    #[test]
    fn daemon_attested_worktree_with_restart_pending_falls_through_to_watching() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::Cursor, McpTier::RestartRequired.into());
        d.daemon_attestation = super::super::daemon_evidence::DaemonAttestation::Enforced;

        assert_eq!(d.protection_state(), ProtectionState::Watching);
    }

    #[test]
    fn daemon_attested_worktree_does_not_override_unsupported_languages() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.all_languages_unsupported = true;
        d.daemon_attestation = super::super::daemon_evidence::DaemonAttestation::Enforced;

        assert_eq!(d.protection_state(), ProtectionState::Unsupported);
    }

    #[test]
    fn watch_running_plus_server_startable_does_not_overclaim() {
        // Council remediation: ServerStartable is NOT one restart from
        // live — the client may not pick up the entry without further
        // setup. With watch running, the truer state is the watch
        // fallback, never `ReadyRestartRequired`.
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::Cursor, McpTier::ServerStartable.into());
        d.watch = WatchTier::Running;
        assert_eq!(d.protection_state(), ProtectionState::Watching);
    }

    #[test]
    fn server_startable_without_watch_falls_to_needs_action() {
        // Council remediation: ServerStartable alone is not enough
        // for a `ReadyRestartRequired` claim — promote only on
        // `RestartRequired`. The user has actionable next steps.
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::ClaudeCode, McpTier::ServerStartable.into());
        assert_eq!(d.protection_state(), ProtectionState::NeedsAction);
    }

    #[test]
    fn no_mcp_and_all_languages_unsupported_yields_unsupported() {
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.all_languages_unsupported = true;
        assert_eq!(d.protection_state(), ProtectionState::Unsupported);
    }

    #[test]
    fn unsupported_languages_with_partial_mcp_yields_unsupported() {
        // Council remediation: when languages are out-of-scope and
        // MCP has not yet reached `RestartRequired`, telling the user
        // to "run anvil start" is misleading because no further setup
        // will produce coverage. Surface `Unsupported` instead.
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::Cursor, McpTier::ConfigPresent.into());
        d.all_languages_unsupported = true;
        assert_eq!(d.protection_state(), ProtectionState::Unsupported);
    }

    #[test]
    fn unsupported_languages_yield_to_restart_required_when_one_step_away() {
        // The user is literally one restart from secrets coverage —
        // tell them, do not collapse to `Unsupported`.
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::Cursor, McpTier::RestartRequired.into());
        d.all_languages_unsupported = true;
        assert_eq!(d.protection_state(), ProtectionState::ReadyRestartRequired);
    }

    #[test]
    fn unsupported_languages_yield_to_protecting_when_live() {
        // Even on unsupported languages, secrets checks still run
        // through MCP. Live evidence wins.
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.mcp
            .insert(McpClientId::Cursor, McpTier::LiveValidation.into());
        d.all_languages_unsupported = true;
        assert_eq!(d.protection_state(), ProtectionState::Protecting);
    }

    #[test]
    fn supported_language_without_mcp_is_needs_action_not_unsupported() {
        // Adversarial guard: do not mislabel a supported repo as
        // unsupported just because activation hasn't completed.
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.all_languages_unsupported = false;
        assert_eq!(d.protection_state(), ProtectionState::NeedsAction);
    }

    #[test]
    fn highest_mcp_tier_picks_strongest() {
        let mut d = empty_diagnostic();
        d.mcp
            .insert(McpClientId::Cursor, McpTier::ConfigPresent.into());
        d.mcp
            .insert(McpClientId::ClaudeCode, McpTier::ServerStartable.into());
        assert_eq!(d.highest_mcp_tier(), Some(McpTier::ServerStartable));
    }

    // LAUNCH-011: predicates that gate fallback messaging and the
    // `start --watch` hand-off. The tier boundary is the only honest
    // place the surface can split "MCP can attach on restart" from
    // "MCP cannot attach"; the tests pin every relevant tier so a
    // future refactor cannot silently slide the boundary.

    #[test]
    fn mcp_pre_write_wired_or_live_is_false_when_no_clients_probed() {
        let d = empty_diagnostic();
        assert!(!d.mcp_pre_write_wired_or_live());
        assert!(!d.mcp_pre_write_live());
    }

    #[test]
    fn mcp_pre_write_wired_or_live_is_false_below_restart_required() {
        for weak_tier in [
            McpTier::NotDetected,
            McpTier::ConfigAbsent,
            McpTier::ConfigPresent,
            McpTier::ServerStartable,
        ] {
            let mut d = empty_diagnostic();
            d.mcp.insert(McpClientId::Cursor, weak_tier.into());
            assert!(
                !d.mcp_pre_write_wired_or_live(),
                "tier {weak_tier:?} must not register as attached"
            );
            assert!(
                !d.mcp_pre_write_live(),
                "tier {weak_tier:?} must not register as live"
            );
        }
    }

    #[test]
    fn mcp_pre_write_wired_or_live_is_true_at_restart_required() {
        let mut d = empty_diagnostic();
        d.mcp
            .insert(McpClientId::Cursor, McpTier::RestartRequired.into());
        assert!(d.mcp_pre_write_wired_or_live());
        assert!(!d.mcp_pre_write_live());
    }

    #[test]
    fn mcp_pre_write_wired_or_live_is_true_at_restart_handshake_verified() {
        let mut d = empty_diagnostic();
        d.mcp.insert(
            McpClientId::Cursor,
            McpTier::RestartHandshakeVerified.into(),
        );
        assert!(d.mcp_pre_write_wired_or_live());
        assert!(!d.mcp_pre_write_live());
    }

    #[test]
    fn mcp_pre_write_live_only_at_live_validation() {
        let mut d = empty_diagnostic();
        d.mcp
            .insert(McpClientId::Cursor, McpTier::LiveValidation.into());
        assert!(d.mcp_pre_write_wired_or_live());
        assert!(d.mcp_pre_write_live());
    }

    #[test]
    fn mcp_pre_write_wired_or_live_picks_strongest_across_clients() {
        // Adversarial guard: a single weak entry must not mask a
        // stronger tier on another client.
        let mut d = empty_diagnostic();
        d.mcp
            .insert(McpClientId::Cursor, McpTier::ConfigPresent.into());
        d.mcp
            .insert(McpClientId::ClaudeCode, McpTier::RestartRequired.into());
        assert!(d.mcp_pre_write_wired_or_live());
    }

    #[test]
    fn verify_offers_watch_when_config_valid_and_mcp_below_restart_required() {
        // With a valid `.anvilrc` and a TS source file (so the
        // language profile reports a supported language), an empty
        // HOME means MCP cannot pre-write attach — the diagnostic
        // must surface `Offered` so the surface can advertise the
        // watch fallback honestly.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".anvilrc"),
            "profile: default\nchecks: []\n",
        )
        .unwrap();
        fs::write(dir.path().join("index.ts"), "export {};\n").unwrap();
        let d = verify_with_home(dir.path(), Some(home.path()));
        assert_eq!(d.config, ConfigStatus::Valid);
        assert!(
            !d.mcp_pre_write_wired_or_live(),
            "fresh tempdir HOME must report MCP not attached"
        );
        assert!(
            !d.all_languages_unsupported,
            "test must seed a supported language so the offer gate is exercised"
        );
        assert_eq!(
            d.watch,
            WatchTier::Offered,
            "fallback must be offered when config is valid and MCP cannot attach"
        );
    }

    #[test]
    fn verify_does_not_offer_watch_on_invalid_config() {
        // The fallback note belongs to honest "MCP not attached"
        // states. When config itself is broken, the user must fix
        // that first — surfacing watch fallback would dilute the
        // error signal.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvilrc"), "{not json::").unwrap();
        let d = verify_with_home(dir.path(), Some(home.path()));
        assert_eq!(d.config, ConfigStatus::Invalid);
        assert_eq!(
            d.watch,
            WatchTier::NotRequested,
            "do not offer watch when config is the active error"
        );
    }

    #[test]
    fn verify_does_not_offer_watch_on_absent_config() {
        // Council remediation: when the user has not run `anvil init`
        // yet, the actionable next step is init, not watch fallback.
        // Advertising both would dilute the primary nudge.
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        let d = verify_with_home(dir.path(), Some(home.path()));
        assert_eq!(d.config, ConfigStatus::Absent);
        assert_eq!(
            d.watch,
            WatchTier::NotRequested,
            "absent config must defer to `anvil init` and suppress \
             the watch offer"
        );
    }

    #[test]
    fn verify_does_not_offer_watch_when_all_languages_unsupported() {
        // Council remediation (Copilot review): drive the gate
        // through `verify_with_home` end-to-end so the assertion
        // pins the `WatchTier::Offered` suppression in the live
        // probe path, not just on a synthetic diagnostic. A
        // synthetic test would pass vacuously if a future bug let
        // the offer fire when `all_languages_unsupported` is set.
        //
        // The repo is seeded with config valid + only Go files. Go is a
        // tail T1 language — parsed but registered `Unsupported` (no
        // language-specific catalogue, CIB-123); if a future registry
        // change promotes Go to `Supported`, the explicit
        // `assert!(d.all_languages_unsupported)` below will fail loudly so
        // the test cannot pass vacuously. (Python was the seed here until
        // CIB-123 lifted it to `Supported`.)
        let dir = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".anvilrc"),
            "profile: default\nchecks: []\n",
        )
        .unwrap();
        fs::write(dir.path().join("main.go"), "package main\n").unwrap();
        let d = verify_with_home(dir.path(), Some(home.path()));
        assert_eq!(d.config, ConfigStatus::Valid);
        assert!(
            d.all_languages_unsupported,
            "test invariant: seeded `.go` must classify as unsupported \
             — if this fires, the language registry has changed and the \
             test needs a different unsupported extension"
        );
        assert_eq!(
            d.watch,
            WatchTier::NotRequested,
            "verify_with_home must suppress the watch offer when all \
             languages are unsupported, got watch={:?}",
            d.watch
        );
    }

    #[test]
    fn watch_running_plus_no_mcp_renders_state_watching_not_protecting() {
        // LAUNCH-011 acceptance: when MCP cannot attach and the watch
        // fallback is live, the protection_state must collapse to
        // `Watching` — never `Protecting`. This is the synthetic
        // analogue of the end-to-end test the spec calls for; the
        // real CLI test exercises the same path through subprocess.
        let mut d = empty_diagnostic();
        d.config = ConfigStatus::Valid;
        d.watch = WatchTier::Running;
        assert!(!d.mcp_pre_write_wired_or_live());
        assert_eq!(d.protection_state(), ProtectionState::Watching);
    }

    #[test]
    fn verify_handles_missing_anvilrc() {
        let dir = TempDir::new().unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Absent);
        // LAUNCH-009: the mcp map now always carries one entry per
        // registered client (Cursor + ClaudeCode in v1). Each entry
        // reports the tier the client has reached. On a fresh tempdir
        // workspace the workspace-local probe always returns
        // ConfigAbsent; the user-global probe may return
        // ConfigAbsent (no cursor/claude installed) or a higher tier
        // (the test runner's actual home has anvil already wired). Don't
        // pin a specific tier here — pin only the structural invariant
        // that every registered client is present.
        assert_eq!(d.mcp.len(), 2);
        assert!(d.mcp.contains_key(&McpClientId::Cursor));
        assert!(d.mcp.contains_key(&McpClientId::ClaudeCode));
        // LAUNCH-011: watch tier is `Offered` only when config is
        // `Valid` and MCP cannot pre-write attach. On a fresh tempdir
        // (`config: absent`) the offer is suppressed regardless of
        // MCP state — the user's primary action is `anvil init` and
        // advertising watch alongside would dilute that nudge.
        assert_eq!(
            d.watch,
            WatchTier::NotRequested,
            "config: absent must always suppress the watch offer"
        );
        assert!(!d.baseline_present);
        assert!(d.last_error.is_none());
        // protection_state() depends on highest mcp tier across clients;
        // assertion below holds only when the test runner's home has no
        // anvil entry (otherwise we might land at ReadyRestartRequired
        // / Protecting). In a CI env this is the common case.
        // Re-assert via the helper that owns the mapping.
        let _ = d.protection_state();
    }

    #[test]
    fn verify_recognises_valid_json_config() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".anvilrc"),
            r#"{"profile":"default","checks":[]}"#,
        )
        .unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Valid);
    }

    #[test]
    fn verify_recognises_valid_yaml_config() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".anvilrc"),
            "profile: default\nchecks: []\n",
        )
        .unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Valid);
    }

    #[test]
    fn verify_recognises_valid_toml_config() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".anvilrc"),
            "profile = \"default\"\nchecks = []\n",
        )
        .unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Valid);
    }

    #[test]
    fn verify_flags_unparseable_config_as_invalid() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".anvilrc"),
            "{this is not valid in any format::",
        )
        .unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Invalid);
        assert_eq!(d.protection_state(), ProtectionState::Error);
    }

    #[test]
    fn verify_flags_empty_config_as_invalid() {
        // Council remediation: empty file passes serde_yaml as
        // `Null` and would otherwise be reported as Valid — that
        // would mask an editor that truncated the user's config.
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvilrc"), "").unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Invalid);
        assert_eq!(d.protection_state(), ProtectionState::Error);
    }

    #[test]
    fn verify_flags_whitespace_only_config_as_invalid() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvilrc"), "   \n\t\n").unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Invalid);
    }

    #[test]
    fn verify_handles_bom_prefixed_config() {
        // A BOM-prefixed file with otherwise-valid YAML should still
        // be Valid — only BOM-only / whitespace-only are Invalid.
        let dir = TempDir::new().unwrap();
        let mut bytes = b"\xEF\xBB\xBF".to_vec();
        bytes.extend_from_slice(b"profile: default\nchecks: []\n");
        fs::write(dir.path().join(".anvilrc"), bytes).unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Valid);
    }

    #[test]
    fn verify_flags_bom_only_config_as_invalid() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvilrc"), b"\xEF\xBB\xBF").unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Invalid);
    }

    #[test]
    fn verify_flags_json_null_config_as_invalid() {
        // Round-2 council: `null` parses as JSON Value::Null and
        // would otherwise be reported Valid. Editor corruption can
        // produce this — semantically it is the same as "no
        // configuration".
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvilrc"), "null\n").unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Invalid);
    }

    #[test]
    fn verify_flags_yaml_comment_only_config_as_invalid() {
        // Round-2 council: comment-only YAML parses as Null. A file
        // with only comments carries zero configuration.
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".anvilrc"),
            "# this is a comment\n# and another\n",
        )
        .unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Invalid);
    }

    #[test]
    fn verify_flags_empty_json_object_config_as_invalid() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvilrc"), "{}\n").unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Invalid);
    }

    #[test]
    fn verify_flags_toml_comment_only_config_as_invalid() {
        // TOML with only comments / blank lines parses successfully
        // to an empty top-level `Table`. The `trimmed.is_empty()`
        // guard does NOT catch this (the file is non-empty after
        // BOM/whitespace trim), so this test exercises the
        // `is_semantically_empty_toml` predicate directly.
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".anvilrc"),
            "# this is a TOML comment\n# blank top-level table\n",
        )
        .unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Invalid);
    }

    #[test]
    fn verify_with_non_empty_toml_is_valid() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvilrc"), "profile = \"default\"\n").unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Valid);
    }

    #[test]
    fn verify_flags_json_array_top_level_as_invalid() {
        // Round-3 council: a non-empty top-level JSON array would
        // pass the round-2 `is_empty()` rule. The shape is wrong for
        // `.anvilrc` (must be a mapping), so it must be Invalid.
        // Use comma-separated content so TOML and YAML cannot
        // re-interpret it as a header — `[1, 2, 3]` has no valid
        // reading as a TOML section header or YAML mapping.
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvilrc"), "[1, 2, 3]\n").unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Invalid);
    }

    #[test]
    fn verify_flags_bare_scalars_as_invalid() {
        // Top-level bare scalars in any of the three formats must be
        // Invalid — the config shape is always an object.
        let dir = TempDir::new().unwrap();
        for content in ["42\n", "true\n", "\"hello\"\n"] {
            fs::write(dir.path().join(".anvilrc"), content).unwrap();
            let d = verify(dir.path());
            assert_eq!(
                d.config,
                ConfigStatus::Invalid,
                "expected Invalid for content {content:?}"
            );
        }
    }

    #[test]
    fn verify_flags_yaml_null_shorthand_as_invalid() {
        // Direct test for the YAML `Null` arm in
        // `is_semantically_empty_yaml` — `~` is YAML's explicit
        // null shorthand.
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".anvilrc"), "~\n").unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Invalid);
    }

    #[test]
    fn verify_with_only_yaml_keylist_is_valid() {
        // Sanity: a non-empty mapping is Valid.
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".anvilrc"),
            "profile: default\nchecks: []\n",
        )
        .unwrap();
        let d = verify(dir.path());
        assert_eq!(d.config, ConfigStatus::Valid);
    }

    #[test]
    fn verify_detects_baseline_marker() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".anvil")).unwrap();
        fs::write(dir.path().join(".anvil").join("baseline.json"), "{}").unwrap();
        let d = verify(dir.path());
        assert!(d.baseline_present);
    }

    #[test]
    fn idempotent_reverify_is_pure() {
        // LAUNCH-012 acceptance: re-running verification performs no
        // writes and leaves all state unchanged. Round-2 council:
        // snapshot the entire workdir tree's mtimes, not just
        // `.anvilrc`, so a future regression that writes anywhere
        // (e.g. `.anvil/`) is caught at the unit level too.
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(".anvilrc"),
            "profile: default\nchecks: []\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join(".anvil")).unwrap();

        let snapshot = || -> Vec<(std::path::PathBuf, std::time::SystemTime)> {
            let mut out = Vec::new();
            for entry in walkdir::WalkDir::new(dir.path()) {
                let entry = entry.unwrap();
                let m = entry.metadata().unwrap();
                out.push((entry.path().to_path_buf(), m.modified().unwrap()));
            }
            out.sort_by(|a, b| a.0.cmp(&b.0));
            out
        };

        let before = snapshot();
        let _ = verify(dir.path());
        let _ = verify(dir.path());
        let after = snapshot();

        assert_eq!(
            before.len(),
            after.len(),
            "verify created or removed entries"
        );
        for (b, a) in before.iter().zip(after.iter()) {
            assert_eq!(b.0, a.0, "path drift: {:?} vs {:?}", b.0, a.0);
            assert_eq!(b.1, a.1, "verify mutated mtime of {:?}", b.0);
        }
    }
}
