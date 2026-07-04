//! INTD-008 + RTAI-006 shared `.anvil.yaml` enforcement-config surface.
//!
//! This module owns the **wire shape** of the `enforcement` block in
//! `.anvil.yaml`. Both the MCP launch shim
//! (`crates/anvil-cli/src/mcp/enforcement.rs`, RTAI-006) and the
//! intercept daemon (`crates/anvil-intercept/src/config.rs`, INTD-008)
//! deserialise the same `EnforcementConfigFile` struct so the two
//! consumers cannot drift on which keys / aliases are accepted. Both
//! resolve the raw `mode` string via the single shared posture type:
//!
//! - Since ADR-098 AD-3 the MCP shim and the daemon both map onto
//!   `anvil_kernel_types::EnforcementMode { Off, Warn, Fence, Interrupt }`
//!   through one shared alias table (`off`/`advisory`/`proceed` → Off,
//!   `warn` → Warn, `fence` → Fence, `interrupt`/`block` → Interrupt).
//!   The pre-AD-3 lossy `fence`/`interrupt` → `Block` collapse the shim
//!   did at parse time is gone: parsing keeps every posture distinct and
//!   the veto is projected at action time (the daemon fences the worktree
//!   or runs the signal ladder; the MCP shim refuses the write).
//!
//! Keeping deserialisation here (and **not** the resolution into
//! either consumer's enum) means:
//!
//! 1. The set of accepted YAML keys is one place. Adding a key
//!    (e.g. INTD-016 `DoS` budgets) is a single proto edit.
//! 2. The forwards-compat policy (silently ignore unknown keys at
//!    the wire layer) is enforced once. RTAI-006's stub already
//!    requires this so workspaces carrying INTD-008-only fields do
//!    not break the MCP shim; INTD-008 inherits the same property.
//! 3. The shared struct does **not** read files. IO and the
//!    project↔user merge sequence are consumer-specific (RTAI-006
//!    is workspace-only today; INTD-008 owns project + user merge
//!    with stricter-wins). Putting `fs::read_to_string` in this
//!    crate would couple the wire shape to a particular IO path.
//!
//! ## Reserved keys
//!
//! INTD-008 declares the following keys but does NOT implement
//! them in this PR — they ship under INTD-016 (`DoS` protection
//! budgets) and are documented here so workspaces can begin to
//! carry them without tripping a forwards-compat warning:
//!
//! - `enforcement.dos.*` (concurrent connection cap, per-connection
//!   RPS, handshake/idle timeouts, max NDJSON frame size).
//!
//! Reserving the keys at the proto layer means the INTD-016
//! implementation does not have to revisit deserialisation — only
//! resolution. For now, any value at `enforcement.dos.*` round-trips
//! through the deserialiser as opaque YAML and is dropped on the
//! INTD-008 / RTAI-006 resolution side.

use serde::Deserialize;

/// Wire-level shape of the `enforcement` block in `.anvil.yaml`.
///
/// Intentionally **not** `#[serde(deny_unknown_fields)]` — both
/// INTD-008 and RTAI-006 must accept workspaces that carry
/// fields the other consumer does not yet understand. Adding a
/// field is always a non-breaking change at this layer; removing
/// or renaming one requires bumping the consumer-side enum and
/// keeping the prior name as a deserialise alias.
///
/// The fields are all `Option<...>` because each consumer needs
/// to distinguish "operator did not set this key" (fall back to
/// the consumer's default) from "operator set this key
/// explicitly" (honour it). For project↔user merging
/// (INTD-008), `None` on the project side defers to the user
/// side; if both are `None`, the consumer applies its default.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct EnforcementConfigFile {
    /// Top-level enforcement strictness.
    ///
    /// Since ADR-098 AD-3 both consumers resolve this string through the
    /// single shared posture type
    /// `anvil_kernel_types::EnforcementMode { Off, Warn, Fence, Interrupt }`
    /// with one alias table (case-folded, trimmed):
    ///
    /// | Raw input                    | Resolved posture |
    /// | ---------------------------- | ---------------- |
    /// | `off` / `advisory` / `proceed` | `Off`          |
    /// | `warn`                       | `Warn`           |
    /// | `fence`                      | `Fence`          |
    /// | `interrupt` / `block`        | `Interrupt`      |
    ///
    /// `off` is a real posture (weakest under stricter-wins,
    /// `off < warn < fence < interrupt`); `block` is an alias for
    /// `interrupt`. There is no parse-time collapse of `fence`/`interrupt`
    /// onto a single veto — the true posture is preserved and the veto is
    /// projected at action time. The daemon and the MCP shim differ only
    /// in their no-config default (the daemon defaults to `Warn`; the MCP
    /// shim to `Interrupt`), which each supplies itself.
    #[serde(default)]
    pub mode: Option<String>,

    /// INTD-008 only. What the daemon does when a file change
    /// cannot be confidently attributed to a single registered
    /// session. Canonical vocabulary: `warn | fence`. Per
    /// `plans/decisions/015-intercept-loop-enforcement.md` AD-3,
    /// ambiguous ownership is **hard-capped at `fence`** as a
    /// code invariant — even if the operator sets `warn` here,
    /// the daemon never interrupts on ambiguous attribution.
    /// RTAI-006 ignores this field.
    #[serde(default)]
    pub on_ambiguous_ownership: Option<String>,

    /// INTD-008 only. When `true`, the daemon evaluates rules and
    /// emits telemetry but never fences or interrupts — the
    /// "observe-only / dry-run" rollout path described in AD-3.
    /// Per-worktree (project-level `.anvil.yaml`) or per-user.
    /// Stricter-wins merging: if either side sets `false`, the
    /// effective value is `false` (enforcement is on); if either
    /// side sets `true` AND the other side does not set `false`,
    /// the effective value is `true`. RTAI-006 ignores this
    /// field — the MCP `validate_write` tool always returns
    /// diagnostics (its "observe-only" path is the `Off` mode,
    /// which is mapped from the `mode` key, not `observe_only`).
    #[serde(default)]
    pub observe_only: Option<bool>,

    /// INTD-016 `DoS` protection budgets — connection cap, RPS,
    /// handshake / idle timeouts, control-lane frame size cap.
    /// Reserved at the proto layer in INTD-008; consumed by the
    /// daemon's `IpcLimits::from_config` in INTD-016. RTAI-006
    /// ignores this field.
    #[serde(default)]
    pub dos: DosConfigFile,

    /// MLP2-024 multi-session caps. Reserved at the proto layer;
    /// consumed by the daemon's session registry on
    /// `register-session`. RTAI-006 ignores this field.
    #[serde(default)]
    pub session: SessionConfigFile,
}

/// INTD-016 `DoS` protection budgets. Reserved at this layer by
/// INTD-008's wave-1 work; the daemon's `IpcLimits` reads these
/// keys in INTD-016.
///
/// Each field is `Option<...>` so an unset value falls through to
/// the daemon's compile-time default. Values that would weaken
/// the daemon (e.g. `max_connections: 0`) are clamped at
/// `IpcLimits::from_config` rather than rejected at this layer —
/// the proto deliberately stays forgiving so an operator's typo
/// never wedges the daemon.
#[derive(Debug, Default, Clone, Copy, PartialEq, Deserialize)]
pub struct DosConfigFile {
    /// Maximum simultaneous driver connections. Default 64; the
    /// listener uses a `tokio::sync::Semaphore` of this size at
    /// the accept loop.
    #[serde(default)]
    pub max_connections: Option<usize>,

    /// Sustained per-connection request rate, requests per second.
    /// Default 100. Drives the token-bucket refill rate.
    #[serde(default)]
    pub rps_sustained: Option<f64>,

    /// Burst capacity for the per-connection token bucket. Default
    /// 1000. The bucket starts full at this value.
    #[serde(default)]
    pub rps_burst: Option<u32>,

    /// Handshake timeout from `accept()` to first manifest frame.
    /// Default 5 s. Uses seconds because operators reason in
    /// seconds; sub-second resolution is overkill for a slow-loris
    /// defence.
    #[serde(default)]
    pub handshake_timeout_seconds: Option<u64>,

    /// Driver-connection idle timeout. Default 60 s. Separate
    /// from session heartbeat TTL — this is the per-connection
    /// quiescence cap.
    #[serde(default)]
    pub idle_timeout_seconds: Option<u64>,

    /// Maximum NDJSON frame size for control-lane methods (every
    /// method that is not `scan_buffer`). Default 64 KiB. The
    /// existing 1 MiB `scan_buffer` payload cap is preserved
    /// separately by the daemon's IPC reader.
    #[serde(default)]
    pub control_frame_max_bytes: Option<usize>,
}

/// MLP2-024 multi-session caps. Sized as a top-level config block
/// (rather than a single scalar) so future per-worktree limits
/// (e.g. burst registration rate, max concurrent fences) land
/// here without another wave of wire-shape churn.
///
/// Stricter-wins on `per_worktree_max`: smaller value wins.
/// Unset on both sides → daemon default of 16 (the MLP2-024
/// design value; rationale below).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct SessionConfigFile {
    /// Maximum number of registered sessions that may share a
    /// single canonicalised worktree. Above this cap, additional
    /// `register-session` calls fail with
    /// `RegistryError::SessionCapExceeded`.
    ///
    /// **Default (when both project and user configs leave this
    /// unset): 16.** Sized off the MLP-014 telemetry trace
    /// (~6 concurrent sub-agents observed in a busy multi-agent
    /// session) with ~3x headroom. A larger cap would let an
    /// adversarial launcher pin unbounded memory in the daemon's
    /// `SessionRegistry`; a smaller cap would refuse legitimate
    /// multi-agent flows. Operators can tighten via project-level
    /// `.anvil.yaml` (`enforcement.session.per_worktree_max: 4`)
    /// or relax via user-level config; the stricter (smaller)
    /// value wins on conflict.
    ///
    /// Setting `0` is rejected at the resolution boundary (it
    /// would refuse every registration on the worktree); the
    /// resolution layer clamps to a minimum of 1, matching the
    /// `IpcLimits::from_config` defensive clamp pattern.
    #[serde(default)]
    pub per_worktree_max: Option<usize>,
}

/// Telemetry stream config (INTD-015 fan-out scope).
///
/// Routed through INTD-008's loader so the same project + user
/// merge applies to subscription scoping as to the enforcement
/// block. This struct is read only by the daemon — RTAI-006 has
/// no telemetry surface.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
pub struct TelemetryConfigFile {
    /// INTD-015 cross-session policy. When `Some(true)`, the
    /// fan-out delivers a **redacted** envelope (`rule_id` +
    /// `hash_of_path`) to subscribers that did not originate the
    /// event. When `Some(false)` or `None`, cross-session events
    /// are dropped entirely — the safe default per the 2026-04-24
    /// council review M5 (security-analyst).
    ///
    /// "Redacted" here means the diagnostic-envelope coordination
    /// spec lines 222-229 form: `rule_id` preserved,
    /// `notification.message` replaced with `[redacted]`,
    /// path-bearing fields hashed. Operators opt in by setting
    /// this flag; INTD-008's stricter-wins merge means a project
    /// `.anvil.yaml` that disables cross-session sharing always
    /// wins over a user-level config that enables it.
    #[serde(default)]
    pub allow_cross_session: Option<bool>,
}

/// Top-level `.anvil.yaml` shape that both consumers
/// deserialise. Wraps [`EnforcementConfigFile`] under the
/// `enforcement` key and (INTD-015) [`TelemetryConfigFile`]
/// under `telemetry`.
///
/// Top-level keys other than the ones declared here are
/// silently ignored (forwards-compat policy: a workspace
/// carrying keys for unrelated subsystems must not break
/// enforcement resolution). INTD-008 may add sibling top-level
/// keys when it lands further config surfaces.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct AnvilConfigFile {
    #[serde(default)]
    pub enforcement: EnforcementConfigFile,
    #[serde(default)]
    pub telemetry: TelemetryConfigFile,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialises_full_enforcement_block() {
        let yaml = r"
enforcement:
  mode: fence
  on_ambiguous_ownership: warn
  observe_only: true
";
        let config: AnvilConfigFile = serde_yaml::from_str(yaml).expect("parse");
        assert_eq!(config.enforcement.mode.as_deref(), Some("fence"));
        assert_eq!(
            config.enforcement.on_ambiguous_ownership.as_deref(),
            Some("warn"),
        );
        assert_eq!(config.enforcement.observe_only, Some(true));
    }

    #[test]
    fn missing_enforcement_block_yields_default() {
        let yaml = "version: 1\n";
        let config: AnvilConfigFile = serde_yaml::from_str(yaml).expect("parse");
        assert_eq!(config.enforcement, EnforcementConfigFile::default());
    }

    #[test]
    fn unknown_top_level_keys_do_not_break_parse() {
        // Forwards-compat: workspaces may carry future config we
        // do not yet implement. Deserialisation must stay forgiving.
        let yaml = r"
enforcement:
  mode: warn
  dos:
    max_connections: 32
unrelated_subsystem:
  enabled: true
";
        let config: AnvilConfigFile = serde_yaml::from_str(yaml).expect("parse");
        assert_eq!(config.enforcement.mode.as_deref(), Some("warn"));
    }

    #[test]
    fn enforcement_block_with_only_partial_fields() {
        // Workspaces commonly only set `mode` — RTAI-006's
        // shipped fixtures look like this. Both fields must be
        // optional so the partial form parses unchanged.
        let yaml = r"
enforcement:
  mode: block
";
        let config: AnvilConfigFile = serde_yaml::from_str(yaml).expect("parse");
        assert_eq!(config.enforcement.mode.as_deref(), Some("block"));
        assert!(config.enforcement.on_ambiguous_ownership.is_none());
        assert!(config.enforcement.observe_only.is_none());
    }

    #[test]
    fn malformed_yaml_returns_error() {
        // Consumers (RTAI-006 / INTD-008) decide whether to fall
        // back to defaults on parse error — the proto layer just
        // surfaces the error.
        let yaml = "this is not: yaml: [";
        let result: Result<AnvilConfigFile, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
    }
}
