//! INTD-008 + RTAI-006 shared `.anvil.yaml` enforcement-config surface.
//!
//! This module owns the **wire shape** of the `enforcement` block in
//! `.anvil.yaml`. Both the MCP launch shim
//! (`crates/anvil-cli/src/mcp/enforcement.rs`, RTAI-006) and the
//! intercept daemon (`crates/anvil-intercept/src/config.rs`, INTD-008)
//! deserialise the same `EnforcementConfigFile` struct so the two
//! consumers cannot drift on which keys / aliases are accepted. Each
//! consumer maps the raw strings onto its own resolved enum:
//!
//! - RTAI-006 collapses to `EnforcementMode::{Block, Warn, Off}` —
//!   the `validate_write` tool only needs three decision modes
//!   (block-on-error / always-warn / never-block).
//! - INTD-008 expands to `Mode::{Warn, Fence, Interrupt}` — the daemon
//!   needs to distinguish "fence the worktree" from "interrupt the
//!   process group" semantically. Treating `block` and `interrupt`
//!   as the same fence-on-error semantic is the explicit
//!   reconciliation called out in the wave-1 brief; both consumers
//!   accept the alias set listed below.
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
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
pub struct EnforcementConfigFile {
    /// Top-level enforcement strictness.
    ///
    /// Canonical RTAI-006 vocabulary: `block | warn | off`.
    /// Canonical INTD-008 vocabulary: `warn | fence | interrupt`.
    /// Aliases accepted by both consumers (case-folded, trimmed):
    ///
    /// | Raw input              | RTAI-006 enum | INTD-008 enum |
    /// | ---------------------- | ------------- | ------------- |
    /// | `block`                | `Block`       | `Interrupt`   |
    /// | `interrupt`            | `Block`       | `Interrupt`   |
    /// | `fence`                | `Block`       | `Fence`       |
    /// | `warn`                 | `Warn`        | `Warn`        |
    /// | `off` / `advisory` /   | `Off`         | `Warn` (n/a — |
    /// | `proceed`              |               | INTD-008 has  |
    /// |                        |               | no "off")     |
    ///
    /// INTD-008 has no `off` mode by spec; if a workspace sets
    /// `mode: off` on the daemon side, the daemon treats it as
    /// `warn` (the strictest interpretation the daemon can offer
    /// without changing semantics). RTAI-006 keeps its `Off`
    /// branch — the MCP shim semantics are unchanged.
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
}

/// Top-level `.anvil.yaml` shape that both consumers
/// deserialise. Wraps [`EnforcementConfigFile`] under the
/// `enforcement` key.
///
/// Top-level keys other than `enforcement` are silently
/// ignored (forwards-compat policy: a workspace carrying
/// keys for unrelated subsystems must not break enforcement
/// resolution). INTD-008 may add sibling top-level keys when
/// it lands further config surfaces — until then, only
/// `enforcement` is consumed here.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
pub struct AnvilConfigFile {
    #[serde(default)]
    pub enforcement: EnforcementConfigFile,
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
