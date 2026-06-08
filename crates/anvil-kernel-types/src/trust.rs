//! Trust and policy graph contract (GV2-012).
//!
//! The vocabulary the **trust/policy graph** owns: trust level, side-effect
//! surfaces, data classifications, invariant guards, policy evidence, and
//! override sources. The trust graph is a separate graph that *joins* to the
//! semantic graph by [`SymbolIdentity`] — it never embeds a semantic node
//! (spine spec: "the raw semantic graph ... it _joins_ to it via symbol
//! identity"). The store and the posture-delta logic live in
//! `eddacraft-anvil-graph-cache::trust`; this module is the typed contract both
//! sides agree on.
//!
//! # Scope guard
//!
//! These are **declarative classifications joined to a symbol**, not the result
//! of full interprocedural data-flow analysis (GV2-012 explicitly excludes it).
//! A producer (e.g. the import heuristic in `graph_cache::trust::annotate_trust`)
//! tags a symbol with the surfaces/classes it can determine from bounded, local
//! evidence; richer producers can refine the same fields later without changing
//! the contract.
//!
//! # Privacy
//!
//! No type here carries source text. Evidence locates code by
//! [`SymbolIdentity`] plus an optional no-text [`ByteRange`] span (byte offsets
//! only, privacy verdict PV-7(e)). Session/worktree and APS/provenance anchors
//! are join-time-only and never appear here (privacy verdict PV-3).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::graph::{ByteRange, SymbolIdentity};

/// How trusted a symbol's code is, on the single trust axis.
///
/// The ordering is *not* a severity ranking — it is the declared set of
/// classifications. `Unknown` is the conservative default for an
/// unclassified symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TrustLevel {
    #[default]
    Unknown,
    Internal,
    Boundary,
    External,
    Privileged,
}

/// A category of observable side effect a symbol's code may perform.
///
/// A bounded vocabulary. The v0.8 import heuristic populates the four
/// module-derived surfaces ([`Network`](Self::Network),
/// [`Filesystem`](Self::Filesystem), [`Process`](Self::Process),
/// [`Crypto`](Self::Crypto)); [`Environment`](Self::Environment) is part of the
/// contract for the config-access producer and is populated when that producer
/// exists (parallel to deferred [`ByteRange`] span population). New surfaces are
/// an additive, schema-versioned change, never a silent extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SideEffectSurface {
    /// Network I/O — `net`, `http`, `https`, sockets.
    Network,
    /// Filesystem access — `fs`.
    Filesystem,
    /// Process / subprocess control — `child_process`, spawn, exec.
    Process,
    /// Cryptographic operations — `crypto`.
    Crypto,
    /// Process environment / configuration reads or writes.
    Environment,
}

/// Sensitivity class of the data a symbol handles.
///
/// A bounded vocabulary, ordered least-to-most sensitive so a max over a set is
/// the dominant class. `Unknown` is the conservative default; population is a
/// producer concern.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
pub enum DataClassification {
    #[default]
    Unknown,
    Public,
    Internal,
    Confidential,
    Secret,
}

/// A policy invariant that watches a symbol's trust posture.
///
/// Each variant names a guard that already exists in the save-time policy
/// surface (`graph_cache::incremental` / `certify`), so the contract describes
/// real enforcement rather than aspirational ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum InvariantGuard {
    /// A new external dependency edge was introduced at this symbol's file
    /// (`NewDependencyIntroduction`).
    NewDependencyIntroduction,
    /// A trust escalation onto the privileged surface (`PrivilegeExpansion`).
    PrivilegeExpansion,
    /// The public API / export surface widened (export-surface diff).
    ApiSurfaceExpansion,
}

/// Where a trust/policy classification came from — so a verdict is auditable.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
pub enum OverrideSource {
    /// Derived by Anvil's built-in heuristics (e.g. `annotate_trust`).
    #[default]
    Heuristic,
    /// Set by repository / workspace configuration.
    Configuration,
    /// Carried forward from a recorded baseline.
    Baseline,
    /// Declared by an inline source annotation.
    Annotation,
}

/// A resolved location in source: a workspace-root-relative file and an
/// optional no-text byte span. The product of resolving [`PolicyEvidence`]
/// back to the code it was derived from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    /// Workspace-root-relative path (the file of the evidence's symbol).
    pub file: String,
    /// Byte-offset span within `file`, when a span-producing pass has run.
    pub span: Option<ByteRange>,
}

/// One piece of evidence backing a trust/policy classification, anchored to the
/// code it was derived from so a verdict can be explained.
///
/// Resolves back to source via the symbol's [`SymbolIdentity`] (always — the
/// semantic↔trust join key) and an optional no-text [`ByteRange`] span
/// (populated when a span-producing pass has run). Carries **no source text**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyEvidence {
    /// The symbol this evidence is about — the semantic↔trust join key.
    pub symbol: SymbolIdentity,
    /// What the evidence asserts about the symbol.
    pub kind: EvidenceKind,
    /// Where the assertion came from.
    pub source: OverrideSource,
    /// Byte-offset span locating the evidence within `symbol.file`. `None`
    /// until a span-producing pass populates it (parallel to deferred semantic
    /// span population, ADR-075 A′ slice).
    pub span: Option<ByteRange>,
}

impl PolicyEvidence {
    /// Resolve this evidence back to a source location.
    ///
    /// Always yields the symbol's file (the deterministic semantic↔trust join);
    /// the span is carried through when present. This is the "policy evidence
    /// resolves back to source spans" contract.
    #[must_use]
    pub fn resolve(&self) -> SourceLocation {
        SourceLocation {
            file: self.symbol.file.clone(),
            span: self.span,
        }
    }
}

/// What a [`PolicyEvidence`] record asserts about its symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceKind {
    /// The symbol was assigned this trust level.
    Trust(TrustLevel),
    /// The symbol reaches this side-effect surface.
    SideEffect(SideEffectSurface),
    /// The symbol handles data of this class.
    DataClass(DataClassification),
    /// The symbol is watched by this invariant guard.
    Guard(InvariantGuard),
}

/// The complete trust/policy classification for one symbol — the payload the
/// trust graph stores against a [`SymbolIdentity`].
///
/// Joins to the semantic graph by identity; it never embeds the semantic node.
/// Sets are [`BTreeSet`]s so a profile has one deterministic serialisation
/// (Anvil determinism invariant).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyProfile {
    /// Trust level on the single trust axis.
    pub trust: TrustLevel,
    /// Side-effect surfaces the symbol reaches.
    pub side_effects: BTreeSet<SideEffectSurface>,
    /// Data sensitivity classes the symbol handles.
    pub data_classes: BTreeSet<DataClassification>,
    /// Invariant guards watching the symbol.
    pub guards: BTreeSet<InvariantGuard>,
    /// Evidence backing the classification, each resolvable to source.
    pub evidence: Vec<PolicyEvidence>,
    /// Where the classification came from.
    pub override_source: OverrideSource,
}

impl Default for PolicyProfile {
    /// The conservative unclassified profile: `Unknown` trust, no surfaces,
    /// no classes, no guards, no evidence, heuristic source.
    fn default() -> Self {
        Self {
            trust: TrustLevel::Unknown,
            side_effects: BTreeSet::new(),
            data_classes: BTreeSet::new(),
            guards: BTreeSet::new(),
            evidence: Vec::new(),
            override_source: OverrideSource::Heuristic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::SymbolKind;

    fn ident(file: &str, name: &str) -> SymbolIdentity {
        SymbolIdentity {
            file: file.to_string(),
            kind: SymbolKind::Function,
            name: name.to_string(),
            ordinal: 0,
        }
    }

    // --- TrustLevel (preserved from the pre-GV2-012 contract) ---

    #[test]
    fn default_is_unknown() {
        assert_eq!(TrustLevel::default(), TrustLevel::Unknown);
    }

    #[test]
    fn all_variants_are_distinct() {
        let variants = [
            TrustLevel::Unknown,
            TrustLevel::Internal,
            TrustLevel::Boundary,
            TrustLevel::External,
            TrustLevel::Privileged,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b, "variants at index {i} and {j} should differ");
                }
            }
        }
    }

    #[test]
    fn copy_semantics() {
        let a = TrustLevel::Privileged;
        let b = a; // Copy, not move
        assert_eq!(a, b);
    }

    #[test]
    fn debug_format_contains_variant_name() {
        assert_eq!(format!("{:?}", TrustLevel::Unknown), "Unknown");
        assert_eq!(format!("{:?}", TrustLevel::External), "External");
    }

    #[test]
    fn serde_round_trip_all_variants() {
        let variants = [
            TrustLevel::Unknown,
            TrustLevel::Internal,
            TrustLevel::Boundary,
            TrustLevel::External,
            TrustLevel::Privileged,
        ];
        for variant in &variants {
            let json = serde_json::to_string(variant).expect("serialise");
            let back: TrustLevel = serde_json::from_str(&json).expect("deserialise");
            assert_eq!(*variant, back);
        }
    }

    #[test]
    fn deserialise_invalid_variant_fails() {
        let result = serde_json::from_str::<TrustLevel>("\"Untrusted\"");
        assert!(result.is_err());
    }

    #[test]
    fn deserialise_from_known_json_pins_wire_format() {
        // `TrustLevel` is serialised to wire/snapshot; a variant rename must
        // break this frozen-string fixture, not pass silently.
        let level: TrustLevel = serde_json::from_str("\"Boundary\"").expect("deserialise");
        assert_eq!(level, TrustLevel::Boundary);
    }

    // --- Contract vocabulary defaults ---

    #[test]
    fn data_classification_defaults_to_unknown() {
        assert_eq!(DataClassification::default(), DataClassification::Unknown);
    }

    #[test]
    fn override_source_defaults_to_heuristic() {
        assert_eq!(OverrideSource::default(), OverrideSource::Heuristic);
    }

    #[test]
    fn data_classification_orders_least_to_most_sensitive() {
        assert!(DataClassification::Public < DataClassification::Secret);
        assert!(DataClassification::Confidential < DataClassification::Secret);
    }

    #[test]
    fn policy_profile_default_is_unclassified() {
        let p = PolicyProfile::default();
        assert_eq!(p.trust, TrustLevel::Unknown);
        assert!(p.side_effects.is_empty());
        assert!(p.data_classes.is_empty());
        assert!(p.guards.is_empty());
        assert!(p.evidence.is_empty());
        assert_eq!(p.override_source, OverrideSource::Heuristic);
    }

    // --- Evidence resolves back to source ---

    #[test]
    fn evidence_resolves_to_symbol_file_without_span() {
        let ev = PolicyEvidence {
            symbol: ident("src/pay.ts", "chargeCard"),
            kind: EvidenceKind::Trust(TrustLevel::Boundary),
            source: OverrideSource::Heuristic,
            span: None,
        };
        let loc = ev.resolve();
        assert_eq!(loc.file, "src/pay.ts");
        assert_eq!(loc.span, None);
    }

    #[test]
    fn evidence_resolves_to_span_when_present() {
        let span = ByteRange { start: 10, end: 42 };
        let ev = PolicyEvidence {
            symbol: ident("src/pay.ts", "chargeCard"),
            kind: EvidenceKind::SideEffect(SideEffectSurface::Network),
            source: OverrideSource::Heuristic,
            span: Some(span),
        };
        let loc = ev.resolve();
        assert_eq!(loc.file, "src/pay.ts");
        assert_eq!(loc.span, Some(span));
    }

    // --- Determinism / serde ---

    #[test]
    fn profile_serialisation_is_deterministic() {
        let mut a = PolicyProfile {
            trust: TrustLevel::Privileged,
            ..PolicyProfile::default()
        };
        // Insert surfaces out of order; BTreeSet must canonicalise them.
        a.side_effects.insert(SideEffectSurface::Network);
        a.side_effects.insert(SideEffectSurface::Crypto);
        a.side_effects.insert(SideEffectSurface::Filesystem);

        let mut b = PolicyProfile {
            trust: TrustLevel::Privileged,
            ..PolicyProfile::default()
        };
        b.side_effects.insert(SideEffectSurface::Filesystem);
        b.side_effects.insert(SideEffectSurface::Network);
        b.side_effects.insert(SideEffectSurface::Crypto);

        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap(),
            "insertion order must not affect serialisation"
        );
    }

    #[test]
    fn profile_round_trips_through_json() {
        let mut p = PolicyProfile {
            trust: TrustLevel::Privileged,
            side_effects: BTreeSet::new(),
            data_classes: BTreeSet::new(),
            guards: BTreeSet::new(),
            evidence: vec![PolicyEvidence {
                symbol: ident("a.ts", "f"),
                kind: EvidenceKind::Guard(InvariantGuard::PrivilegeExpansion),
                source: OverrideSource::Configuration,
                span: Some(ByteRange { start: 1, end: 2 }),
            }],
            override_source: OverrideSource::Configuration,
        };
        p.side_effects.insert(SideEffectSurface::Filesystem);
        p.data_classes.insert(DataClassification::Secret);
        p.guards.insert(InvariantGuard::PrivilegeExpansion);

        let json = serde_json::to_string(&p).unwrap();
        let back: PolicyProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn evidence_kind_variants_round_trip() {
        let kinds = [
            EvidenceKind::Trust(TrustLevel::External),
            EvidenceKind::SideEffect(SideEffectSurface::Process),
            EvidenceKind::DataClass(DataClassification::Confidential),
            EvidenceKind::Guard(InvariantGuard::NewDependencyIntroduction),
        ];
        for k in kinds {
            let json = serde_json::to_string(&k).unwrap();
            let back: EvidenceKind = serde_json::from_str(&json).unwrap();
            assert_eq!(k, back);
        }
    }
}
