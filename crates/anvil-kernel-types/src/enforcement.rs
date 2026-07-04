//! The shared enforcement **posture** — the `enforcement.mode` axis of
//! the two-axis enforcement model (ADR-098 AD-3).
//!
//! A posture is what an operator writes in `.anvil.yaml`
//! (`enforcement.mode`); an *outcome* is the
//! [`crate::diagnostics::ControlDecision`] a surface records when a
//! finding warrants action. Before ADR-098 two parallel vocabularies
//! parsed the same `enforcement.mode` field with different, lossy alias
//! tables — the daemon's `Mode` (`Warn | Fence | Interrupt`) and the MCP
//! shim's `EnforcementMode` (`Block | Warn | Off`, where `fence` /
//! `interrupt` collapsed to `Block` at parse time). Both now die into
//! this one type with one alias table so the two surfaces cannot drift
//! on which values or aliases they accept.
//!
//! ## Alias table
//!
//! Parsing is case-insensitive and whitespace-trimmed. The single table
//! is the union of both legacy tables, with the lossy collapses removed:
//!
//! | Raw `enforcement.mode` value | Resolved [`EnforcementMode`] |
//! | ---------------------------- | ---------------------------- |
//! | `off`                        | [`EnforcementMode::Off`]       |
//! | `advisory` / `proceed`       | [`EnforcementMode::Off`]       |
//! | `warn`                       | [`EnforcementMode::Warn`]      |
//! | `fence`                      | [`EnforcementMode::Fence`]     |
//! | `interrupt` / `block`        | [`EnforcementMode::Interrupt`] |
//! | unknown / missing            | `None` (caller defaults)       |
//!
//! `Off` is a real posture now (it was clamped to `Warn` daemon-side
//! before, because the daemon had no `off` branch). `block` is an alias
//! for `interrupt` (ADR-098 AD-3). `fence` and `interrupt` stay distinct
//! — the lossy `fence → block` collapse the MCP shim did at parse time
//! moves to *action time*, where each surface projects the true decision
//! onto its own response shape.
//!
//! ## Defaults are per-surface, not on this type
//!
//! [`EnforcementMode::default`] is [`EnforcementMode::Warn`] (ADR-002,
//! warnings over blocks) — the daemon's historical default. The MCP shim
//! historically defaulted to a write-vetoing posture (`Block`), so it
//! must supply its own default explicitly ([`EnforcementMode::Interrupt`]
//! via the `block` alias) rather than relying on [`Default`]. Each
//! surface owns its no-config fallback; this type only owns the
//! vocabulary, the alias table, and stricter-wins.
//!
//! ## Stricter-wins ordering
//!
//! The derived [`Ord`] is `Off < Warn < Fence < Interrupt`; a project ↔
//! user config merge keeps the stricter of the two via
//! [`EnforcementMode::stricter`] so a user-level config can never weaken
//! a project's posture.

use crate::diagnostics::ControlDecision;

/// The enforcement posture resolved from `.anvil.yaml`
/// `enforcement.mode`. See the module docs for the alias table and the
/// two-axis model.
///
/// Variants are declared weakest-first so the derived [`Ord`] gives
/// stricter-wins (`Off < Warn < Fence < Interrupt`). Do not rely on the
/// discriminant values — merge with [`EnforcementMode::stricter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum EnforcementMode {
    /// No enforcement: every finding projects to
    /// [`ControlDecision::Allow`]. Posture-only — never itself a
    /// decision. Findings are still returned to the caller; only the
    /// decision is suppressed.
    Off,
    /// Warnings-first (ADR-002): findings are surfaced but never veto the
    /// operation. The default posture — operators opt into stricter
    /// modes.
    #[default]
    Warn,
    /// On a block-worthy finding, veto the operation and fence the
    /// worktree. Escalates to [`ControlDecision::Fence`].
    Fence,
    /// On a block-worthy finding, veto the operation and issue a
    /// process-group interrupt (the strictest posture). Escalates to
    /// [`ControlDecision::Interrupt`]. `block` is an alias for this
    /// posture (ADR-098 AD-3).
    Interrupt,
}

impl EnforcementMode {
    /// Parse a raw `.anvil.yaml` `enforcement.mode` value via the single
    /// shared alias table (see the module docs). Case-insensitive;
    /// whitespace trimmed. Unknown values return `None` so the caller can
    /// apply its own per-surface default.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            // `off` is a real posture now — projects to always-`Allow`.
            // `advisory` / `proceed` were the MCP shim's off-direction
            // aliases; they join `off` rather than clamping to `warn`.
            "off" | "advisory" | "proceed" => Some(Self::Off),
            "warn" => Some(Self::Warn),
            "fence" => Some(Self::Fence),
            // `block` aliases to `interrupt` (ADR-098 AD-3) — the
            // strictest posture, matching the legacy daemon table.
            "interrupt" | "block" => Some(Self::Interrupt),
            _ => None,
        }
    }

    /// Canonical string form (telemetry / status surfaces).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Warn => "warn",
            Self::Fence => "fence",
            Self::Interrupt => "interrupt",
        }
    }

    /// Return the stricter of two postures (project ↔ user merge). A
    /// user-level config can raise but never lower a project's posture.
    #[must_use]
    pub fn stricter(self, other: Self) -> Self {
        if self >= other { self } else { other }
    }

    /// The [`ControlDecision`] this posture escalates a block-worthy
    /// finding to — the action-time projection (ADR-098 AD-3). The
    /// posture is recorded distinctly and only projected onto a decision
    /// when a finding warrants action:
    ///
    /// - [`EnforcementMode::Off`] never escalates → [`ControlDecision::Allow`]
    /// - [`EnforcementMode::Warn`] caps at [`ControlDecision::Warn`]
    /// - [`EnforcementMode::Fence`] → [`ControlDecision::Fence`]
    /// - [`EnforcementMode::Interrupt`] → [`ControlDecision::Interrupt`]
    ///
    /// The `fence`/`interrupt` postures keep their namesake decision —
    /// there is no lossy collapse to `Block`. A surface then projects the
    /// veto onto its own response (MCP refuses the write; the daemon
    /// fences the worktree or runs the signal ladder).
    #[must_use]
    pub const fn escalated_decision(self) -> ControlDecision {
        match self {
            Self::Off => ControlDecision::Allow,
            Self::Warn => ControlDecision::Warn,
            Self::Fence => ControlDecision::Fence,
            Self::Interrupt => ControlDecision::Interrupt,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_warn() {
        // ADR-002 warnings-over-blocks; matches the legacy daemon
        // default. Per-surface defaults (e.g. the MCP shim's veto
        // default) are supplied by the surface, not this type.
        assert_eq!(EnforcementMode::default(), EnforcementMode::Warn);
    }

    #[test]
    fn parse_canonical_values() {
        assert_eq!(EnforcementMode::parse("off"), Some(EnforcementMode::Off));
        assert_eq!(EnforcementMode::parse("warn"), Some(EnforcementMode::Warn));
        assert_eq!(
            EnforcementMode::parse("fence"),
            Some(EnforcementMode::Fence)
        );
        assert_eq!(
            EnforcementMode::parse("interrupt"),
            Some(EnforcementMode::Interrupt)
        );
    }

    #[test]
    fn parse_block_aliases_to_interrupt() {
        // ADR-098 AD-3: `block` is the strictest posture's alias. Matches
        // the legacy daemon table; the legacy MCP table collapsed
        // `block` onto its (now-retired) `Block` variant.
        assert_eq!(
            EnforcementMode::parse("block"),
            Some(EnforcementMode::Interrupt)
        );
        assert_eq!(
            EnforcementMode::parse("  BLOCK  "),
            Some(EnforcementMode::Interrupt)
        );
    }

    #[test]
    fn parse_off_direction_aliases() {
        // `off` is a real posture now; `advisory`/`proceed` join it
        // rather than clamping to `warn` (the legacy daemon behaviour).
        assert_eq!(EnforcementMode::parse("off"), Some(EnforcementMode::Off));
        assert_eq!(
            EnforcementMode::parse("advisory"),
            Some(EnforcementMode::Off)
        );
        assert_eq!(
            EnforcementMode::parse("PROCEED"),
            Some(EnforcementMode::Off)
        );
    }

    #[test]
    fn parse_unknown_and_typos_return_none() {
        assert_eq!(EnforcementMode::parse("nope"), None);
        assert_eq!(EnforcementMode::parse(""), None);
        // Common typos must not silently match a near neighbour.
        assert_eq!(EnforcementMode::parse("interupt"), None);
        assert_eq!(EnforcementMode::parse("fenc"), None);
    }

    #[test]
    fn ord_is_stricter_wins() {
        assert!(EnforcementMode::Off < EnforcementMode::Warn);
        assert!(EnforcementMode::Warn < EnforcementMode::Fence);
        assert!(EnforcementMode::Fence < EnforcementMode::Interrupt);
    }

    #[test]
    fn stricter_picks_highest_posture() {
        assert_eq!(
            EnforcementMode::Off.stricter(EnforcementMode::Warn),
            EnforcementMode::Warn
        );
        assert_eq!(
            EnforcementMode::Warn.stricter(EnforcementMode::Fence),
            EnforcementMode::Fence
        );
        assert_eq!(
            EnforcementMode::Interrupt.stricter(EnforcementMode::Fence),
            EnforcementMode::Interrupt
        );
        assert_eq!(
            EnforcementMode::Fence.stricter(EnforcementMode::Fence),
            EnforcementMode::Fence
        );
    }

    #[test]
    fn escalated_decision_projects_true_posture() {
        // The `fence`/`interrupt` postures keep their namesake decision —
        // no lossy collapse to `Block` (ADR-098 AD-3).
        assert_eq!(
            EnforcementMode::Off.escalated_decision(),
            ControlDecision::Allow
        );
        assert_eq!(
            EnforcementMode::Warn.escalated_decision(),
            ControlDecision::Warn
        );
        assert_eq!(
            EnforcementMode::Fence.escalated_decision(),
            ControlDecision::Fence
        );
        assert_eq!(
            EnforcementMode::Interrupt.escalated_decision(),
            ControlDecision::Interrupt
        );
        // Every escalating posture's projection is a veto; Off/Warn are not.
        assert!(EnforcementMode::Fence.escalated_decision().is_veto());
        assert!(EnforcementMode::Interrupt.escalated_decision().is_veto());
        assert!(!EnforcementMode::Warn.escalated_decision().is_veto());
        assert!(!EnforcementMode::Off.escalated_decision().is_veto());
    }

    #[test]
    fn as_str_round_trips_through_parse() {
        for mode in [
            EnforcementMode::Off,
            EnforcementMode::Warn,
            EnforcementMode::Fence,
            EnforcementMode::Interrupt,
        ] {
            assert_eq!(EnforcementMode::parse(mode.as_str()), Some(mode));
        }
    }
}
