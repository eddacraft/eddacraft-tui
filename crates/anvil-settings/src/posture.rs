//! Protection posture board snapshot (POSBRD-002).
//!
//! Four projection rows with configured / resolved / active cells.
//! Status renders this snapshot; this module does not open config files.

use crate::runtime_state::RuntimeState;
use crate::types::Posture;

/// Canonical ladder scale. Mapping legend data, not a fifth row.
pub const POSTURE_SCALE: [&str; 4] = ["off", "warn", "fence", "interrupt"];

/// Surfaces on the protection posture board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostureSurface {
    McpPreWrite,
    Intercept,
    Gate,
    Acceptance,
}

impl PostureSurface {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::McpPreWrite => "mcp_pre_write",
            Self::Intercept => "intercept",
            Self::Gate => "gate",
            Self::Acceptance => "acceptance",
        }
    }

    #[must_use]
    pub const fn verbs(self) -> VerbFamily {
        match self {
            Self::McpPreWrite | Self::Intercept => VerbFamily::Ladder,
            Self::Gate | Self::Acceptance => VerbFamily::Native,
        }
    }

    /// Placeholder when the source is absent. Not a resolved default.
    #[must_use]
    pub const fn absent_label(self) -> &'static str {
        match self {
            Self::Acceptance => "(none)",
            Self::McpPreWrite | Self::Intercept | Self::Gate => "(unset)",
        }
    }

    /// Human label on the board. Not numbered L0–L5.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::McpPreWrite => "mcp pre-write",
            Self::Intercept => "intercept",
            Self::Gate => "gate",
            Self::Acceptance => "acceptance",
        }
    }

    const fn source(self) -> &'static str {
        match self {
            Self::McpPreWrite | Self::Intercept => "enforcement.mode",
            Self::Gate => "fail-on-warnings",
            Self::Acceptance => "anvil/policy.*",
        }
    }
}

/// Verb family for a row. Gate and acceptance never display ladder values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerbFamily {
    Ladder,
    Native,
}

/// Configured or resolved cell. `value` is `None` when the source is absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostureCell {
    pub value: Option<String>,
    pub source: &'static str,
}

/// SETCON-strict active cell. YAML is never proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveCell {
    pub value: Option<String>,
    pub state: RuntimeState,
}

/// Intercept-row last-action detail. POSBRD-004 fills this; omitted here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastAction {
    pub decision: String,
    pub stage: Option<String>,
    pub observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostureRow {
    pub id: PostureSurface,
    pub verbs: VerbFamily,
    pub configured: PostureCell,
    pub resolved: PostureCell,
    pub active: ActiveCell,
    pub last_action: Option<LastAction>,
}

/// Approximate cross-surface mapping. Data on the snapshot, not a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingLegend {
    pub scale: [&'static str; 4],
    pub warn_equivalents: [&'static str; 2],
    pub interrupt_equivalents: [&'static str; 2],
    /// ADR-098 AD-3: `block` aliases the interrupt posture.
    pub block_alias: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostureSnapshot {
    pub scale: [&'static str; 4],
    pub rows: [PostureRow; 4],
    pub mapping: MappingLegend,
}

/// Declared `.anvil.yaml` `enforcement.mode` plus SETCON-resolved value.
///
/// `resolved` is the SETCON row after constraints (may still be the catalogue
/// default). Surface defaults apply only when the key was not declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnforcementSource {
    pub declared: Option<Posture>,
    pub resolved: Option<Posture>,
    pub intercept_user: Option<Posture>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateSource {
    pub fail_on_warnings: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptanceVerb {
    OnWarn,
    OnBlock,
}

impl AcceptanceVerb {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OnWarn => "on_warn",
            Self::OnBlock => "on_block",
        }
    }
}

/// L4 policy presence. `verb` is the projected native cell when present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptanceSource {
    pub present: bool,
    pub verb: Option<AcceptanceVerb>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceActive {
    pub value: Option<String>,
    pub state: RuntimeState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostureInputs {
    pub enforcement: EnforcementSource,
    pub gate: GateSource,
    pub acceptance: AcceptanceSource,
    pub active: [SurfaceActive; 4],
    /// POSBRD-004: intercept-row detail only, when attested.
    pub last_action: Option<LastAction>,
}

/// Compute the four-row board. Does not read files or invent active evidence.
#[must_use]
pub fn posture_snapshot(inputs: &PostureInputs) -> PostureSnapshot {
    let mcp_resolved = ladder_resolved(
        inputs.enforcement.declared,
        inputs.enforcement.resolved,
        Posture::Interrupt,
        None,
    );
    let intercept_resolved = ladder_resolved(
        inputs.enforcement.declared,
        inputs.enforcement.resolved,
        Posture::Warn,
        inputs.enforcement.intercept_user,
    );
    let configured = inputs.enforcement.declared.map(Posture::as_str);
    let mcp = ladder_row(
        PostureSurface::McpPreWrite,
        configured,
        mcp_resolved,
        &inputs.active[0],
    );
    let mut intercept = ladder_row(
        PostureSurface::Intercept,
        configured,
        intercept_resolved,
        &inputs.active[1],
    );
    intercept.last_action.clone_from(&inputs.last_action);
    let gate = gate_row(inputs.gate, &inputs.active[2]);
    let acceptance = acceptance_row(inputs.acceptance, &inputs.active[3]);
    PostureSnapshot {
        scale: POSTURE_SCALE,
        rows: [mcp, intercept, gate, acceptance],
        mapping: MappingLegend {
            scale: POSTURE_SCALE,
            warn_equivalents: ["warnings-pass", "on_warn"],
            interrupt_equivalents: ["warnings-fail", "on_block"],
            block_alias: "interrupt",
        },
    }
}

fn ladder_resolved(
    declared: Option<Posture>,
    setcon: Option<Posture>,
    surface_default: Posture,
    raise: Option<Posture>,
) -> Posture {
    let mut value = if declared.is_some() {
        setcon.or(declared).unwrap_or(surface_default)
    } else {
        match setcon {
            Some(constrained) if constrained > surface_default => constrained,
            _ => surface_default,
        }
    };
    if let Some(user) = raise
        && user > value
    {
        value = user;
    }
    value
}

fn ladder_row(
    surface: PostureSurface,
    configured: Option<&str>,
    resolved: Posture,
    active: &SurfaceActive,
) -> PostureRow {
    row(
        surface,
        configured.map(str::to_owned),
        Some(resolved.as_str().to_owned()),
        active,
    )
}

fn gate_row(source: GateSource, active: &SurfaceActive) -> PostureRow {
    let configured = source.fail_on_warnings.map(gate_verb);
    let resolved = source.fail_on_warnings.map_or("warnings-pass", gate_verb);
    row(
        PostureSurface::Gate,
        configured.map(str::to_owned),
        Some(resolved.to_owned()),
        active,
    )
}

fn gate_verb(fail_on_warnings: bool) -> &'static str {
    if fail_on_warnings {
        "warnings-fail"
    } else {
        "warnings-pass"
    }
}

fn acceptance_row(source: AcceptanceSource, active: &SurfaceActive) -> PostureRow {
    let configured = source
        .verb
        .filter(|_| source.present)
        .map(AcceptanceVerb::as_str);
    let resolved = if source.present {
        Some(source.verb.unwrap_or(AcceptanceVerb::OnBlock).as_str())
    } else {
        None
    };
    row(
        PostureSurface::Acceptance,
        configured.map(str::to_owned),
        resolved.map(str::to_owned),
        active,
    )
}

fn row(
    surface: PostureSurface,
    configured: Option<String>,
    resolved: Option<String>,
    active: &SurfaceActive,
) -> PostureRow {
    PostureRow {
        id: surface,
        verbs: surface.verbs(),
        configured: PostureCell {
            value: configured,
            source: surface.source(),
        },
        resolved: PostureCell {
            value: resolved,
            source: surface.source(),
        },
        active: ActiveCell {
            value: active.value.clone(),
            state: active.state,
        },
        last_action: None,
    }
}

#[cfg(test)]
mod posture_snapshot_tests {
    use super::*;

    fn unknown_active() -> SurfaceActive {
        SurfaceActive {
            value: None,
            state: RuntimeState::Unknown,
        }
    }

    fn unknown_actives() -> [SurfaceActive; 4] {
        [
            unknown_active(),
            unknown_active(),
            unknown_active(),
            unknown_active(),
        ]
    }

    fn inputs() -> PostureInputs {
        PostureInputs {
            enforcement: EnforcementSource {
                declared: None,
                resolved: Some(Posture::Warn),
                intercept_user: None,
            },
            gate: GateSource {
                fail_on_warnings: None,
            },
            acceptance: AcceptanceSource {
                present: false,
                verb: None,
            },
            active: unknown_actives(),
            last_action: None,
        }
    }

    fn row(snapshot: &PostureSnapshot, id: PostureSurface) -> &PostureRow {
        snapshot
            .rows
            .iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("missing {}", id.id()))
    }

    #[test]
    fn posture_snapshot_absent_key_splits_mcp_interrupt_and_intercept_warn() {
        let snapshot = posture_snapshot(&inputs());
        let mcp = row(&snapshot, PostureSurface::McpPreWrite);
        let intercept = row(&snapshot, PostureSurface::Intercept);
        assert_eq!(mcp.configured.value, None);
        assert_eq!(mcp.resolved.value.as_deref(), Some("interrupt"));
        assert_eq!(intercept.configured.value, None);
        assert_eq!(intercept.resolved.value.as_deref(), Some("warn"));
        assert_eq!(mcp.verbs, VerbFamily::Ladder);
        assert_eq!(intercept.verbs, VerbFamily::Ladder);
        assert_eq!(PostureSurface::McpPreWrite.absent_label(), "(unset)");
        assert_eq!(PostureSurface::Intercept.absent_label(), "(unset)");
    }

    #[test]
    fn posture_snapshot_does_not_copy_resolved_into_configured() {
        let snapshot = posture_snapshot(&inputs());
        for surface in [
            PostureSurface::McpPreWrite,
            PostureSurface::Intercept,
            PostureSurface::Gate,
        ] {
            let cell = row(&snapshot, surface);
            assert_ne!(cell.configured.value, cell.resolved.value);
            assert!(cell.configured.value.is_none());
        }
    }

    #[test]
    fn posture_snapshot_declared_warn_agrees_until_user_raises_intercept() {
        let mut source = inputs();
        source.enforcement.declared = Some(Posture::Warn);
        let snapshot = posture_snapshot(&source);
        assert_eq!(
            row(&snapshot, PostureSurface::McpPreWrite)
                .configured
                .value
                .as_deref(),
            Some("warn")
        );
        assert_eq!(
            row(&snapshot, PostureSurface::McpPreWrite)
                .resolved
                .value
                .as_deref(),
            Some("warn")
        );
        assert_eq!(
            row(&snapshot, PostureSurface::Intercept)
                .resolved
                .value
                .as_deref(),
            Some("warn")
        );

        source.enforcement.intercept_user = Some(Posture::Fence);
        let snapshot = posture_snapshot(&source);
        assert_eq!(
            row(&snapshot, PostureSurface::McpPreWrite)
                .resolved
                .value
                .as_deref(),
            Some("warn"),
            "MCP does not merge user config"
        );
        assert_eq!(
            row(&snapshot, PostureSurface::Intercept)
                .resolved
                .value
                .as_deref(),
            Some("fence")
        );
        assert_eq!(
            row(&snapshot, PostureSurface::Intercept)
                .configured
                .value
                .as_deref(),
            Some("warn"),
            "configured stays the declared key, not the raised value"
        );
    }

    #[test]
    fn posture_snapshot_min_posture_raises_both_ladder_rows() {
        let mut source = inputs();
        source.enforcement.declared = Some(Posture::Warn);
        source.enforcement.resolved = Some(Posture::Interrupt);
        let snapshot = posture_snapshot(&source);
        assert_eq!(
            row(&snapshot, PostureSurface::McpPreWrite)
                .resolved
                .value
                .as_deref(),
            Some("interrupt")
        );
        assert_eq!(
            row(&snapshot, PostureSurface::Intercept)
                .resolved
                .value
                .as_deref(),
            Some("interrupt")
        );
        assert_eq!(
            row(&snapshot, PostureSurface::McpPreWrite)
                .configured
                .value
                .as_deref(),
            Some("warn")
        );
    }

    #[test]
    fn posture_snapshot_user_cannot_lower_intercept() {
        let mut source = inputs();
        source.enforcement.declared = Some(Posture::Interrupt);
        source.enforcement.resolved = Some(Posture::Interrupt);
        source.enforcement.intercept_user = Some(Posture::Off);
        let snapshot = posture_snapshot(&source);
        assert_eq!(
            row(&snapshot, PostureSurface::Intercept)
                .resolved
                .value
                .as_deref(),
            Some("interrupt")
        );
        assert_eq!(
            row(&snapshot, PostureSurface::McpPreWrite)
                .resolved
                .value
                .as_deref(),
            Some("interrupt")
        );
    }

    #[test]
    fn posture_snapshot_gate_defaults_to_warnings_pass() {
        let snapshot = posture_snapshot(&inputs());
        let gate = row(&snapshot, PostureSurface::Gate);
        assert_eq!(gate.verbs, VerbFamily::Native);
        assert_eq!(gate.configured.value, None);
        assert_eq!(gate.resolved.value.as_deref(), Some("warnings-pass"));

        let mut source = inputs();
        source.gate.fail_on_warnings = Some(true);
        let snapshot = posture_snapshot(&source);
        let gate = row(&snapshot, PostureSurface::Gate);
        assert_eq!(gate.configured.value.as_deref(), Some("warnings-fail"));
        assert_eq!(gate.resolved.value.as_deref(), Some("warnings-fail"));
    }

    #[test]
    fn posture_snapshot_acceptance_none_when_policy_absent() {
        let snapshot = posture_snapshot(&inputs());
        let acceptance = row(&snapshot, PostureSurface::Acceptance);
        assert_eq!(acceptance.verbs, VerbFamily::Native);
        assert_eq!(acceptance.configured.value, None);
        assert_eq!(acceptance.resolved.value, None);
        assert_eq!(PostureSurface::Acceptance.absent_label(), "(none)");

        let mut source = inputs();
        source.acceptance.present = true;
        source.acceptance.verb = None;
        let snapshot = posture_snapshot(&source);
        let acceptance = row(&snapshot, PostureSurface::Acceptance);
        assert_eq!(acceptance.configured.value, None);
        assert_eq!(acceptance.resolved.value.as_deref(), Some("on_block"));

        source.acceptance.verb = Some(AcceptanceVerb::OnWarn);
        let snapshot = posture_snapshot(&source);
        let acceptance = row(&snapshot, PostureSurface::Acceptance);
        assert_eq!(acceptance.configured.value.as_deref(), Some("on_warn"));
        assert_eq!(acceptance.resolved.value.as_deref(), Some("on_warn"));
    }

    #[test]
    fn posture_snapshot_active_unknown_without_evidence() {
        let mut source = inputs();
        source.enforcement.declared = Some(Posture::Warn);
        source.enforcement.resolved = Some(Posture::Warn);
        let snapshot = posture_snapshot(&source);
        for cell in &snapshot.rows {
            assert_eq!(cell.active.state, RuntimeState::Unknown);
            assert_eq!(cell.active.value, None);
        }
    }

    #[test]
    fn posture_snapshot_active_uses_injected_evidence_not_resolved() {
        let mut source = inputs();
        source.enforcement.declared = Some(Posture::Warn);
        source.enforcement.resolved = Some(Posture::Warn);
        source.active[1] = SurfaceActive {
            value: Some("fence".into()),
            state: RuntimeState::Drift,
        };
        let snapshot = posture_snapshot(&source);
        let intercept = row(&snapshot, PostureSurface::Intercept);
        assert_eq!(intercept.resolved.value.as_deref(), Some("warn"));
        assert_eq!(intercept.active.value.as_deref(), Some("fence"));
        assert_eq!(intercept.active.state, RuntimeState::Drift);
    }

    #[test]
    fn posture_snapshot_mapping_is_data_not_a_row() {
        let snapshot = posture_snapshot(&inputs());
        assert_eq!(snapshot.rows.len(), 4);
        assert_eq!(
            snapshot
                .rows
                .iter()
                .map(|row| row.id.id())
                .collect::<Vec<_>>(),
            ["mcp_pre_write", "intercept", "gate", "acceptance"]
        );
        assert_eq!(snapshot.scale, POSTURE_SCALE);
        assert_eq!(snapshot.mapping.scale, POSTURE_SCALE);
        assert_eq!(
            snapshot.mapping.warn_equivalents,
            ["warnings-pass", "on_warn"]
        );
        assert_eq!(
            snapshot.mapping.interrupt_equivalents,
            ["warnings-fail", "on_block"]
        );
        assert_eq!(snapshot.mapping.block_alias, "interrupt");
        assert!(snapshot.rows.iter().all(|row| row.last_action.is_none()));
    }

    #[test]
    fn posture_snapshot_last_action_is_intercept_row_only() {
        let mut source = inputs();
        source.last_action = Some(LastAction {
            decision: "interrupt".into(),
            stage: Some("sigkill".into()),
            observed_at: "2026-09-14T12:00:00Z".into(),
        });
        let snapshot = posture_snapshot(&source);
        let intercept = row(&snapshot, PostureSurface::Intercept);
        let action = intercept.last_action.as_ref().expect("attested");
        assert_eq!(action.decision, "interrupt");
        assert_eq!(action.stage.as_deref(), Some("sigkill"));
        assert!(
            snapshot
                .rows
                .iter()
                .filter(|row| row.id != PostureSurface::Intercept)
                .all(|row| row.last_action.is_none())
        );
    }
}
