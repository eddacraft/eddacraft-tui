//! Unified remediation-first policy guidance (OPAE-005).
//!
//! A policy failure can originate from three producers: a Rego **pack eval
//! finding** ([`crate::result::Finding`]), a CPOL **assertion violation**
//! ([`crate::context::AssertionGuidance`]), or an **IO-risk finding**
//! ([`crate::io_risk::RiskGuidance`]). Each already carries a remediation-first
//! shape in its own vocabulary. This module folds all three into a single
//! [`PolicyGuidance`] output so a caller (CLI, gate, CI summary) renders one
//! shape regardless of source — the module's "unified guidance" contract.
//!
//! ## What the shape carries
//!
//! Leading with *how to fix it*, a [`PolicyGuidance`] carries the rule id, the
//! [`PolicySource`] it came from (a closed enum), an optional rationale, the
//! changed-code [`context`](PolicyGuidance::context) (the offending path(s) and
//! span when known), the remediation text, and static-but-parameterised
//! exception guidance naming `anvil exception grant` with the rule id.
//!
//! ## What it deliberately does not carry
//!
//! - **No severity or blocking flag.** As with [`crate::context::guidance`] and
//!   [`crate::io_risk::guidance`], whether a failure blocks is a posture
//!   decision owned by the enforcement layer (OPAE-007), not a property of the
//!   guidance. Keeping it off the guidance stops the two from drifting.
//! - **No exceptions-store wiring.** The exception guidance is a *text* contract
//!   only: it tells the author which command to run. It does not consult,
//!   grant, or apply an exception.
//!
//! ## Determinism and wire form
//!
//! Construction normalises the changed-code context (sorted and de-duplicated)
//! so equal inputs yield an equal guidance, and the type round-trips cleanly
//! through serde with absent optionals skip-serialised. User-facing text uses UK
//! spelling.

use serde::{Deserialize, Serialize};

use crate::context::AssertionGuidance;
use crate::io_risk::{RiskGuidance, RiskGuidanceCode};
use crate::result::Finding;

/// The producer a policy failure originated from.
///
/// A **closed** set: a policy failure is always a pack finding, an assertion
/// violation, or a scanner finding. The wire form is externally-tagged
/// kebab-case (`{"pack":"…"}` / `{"assertion":"…"}` / `{"scanner":"…"}`),
/// mirroring [`crate::context::assertion::AssertionCondition`]. Variants are
/// added, never renamed: the tag set is part of the guidance contract.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolicySource {
    /// A Rego pack finding — names the pack id or policy file it came from.
    Pack(String),
    /// A CPOL assertion violation — names the violated assertion id.
    Assertion(String),
    /// An IO-risk scanner finding — names the scanner that produced it.
    Scanner(String),
}

/// A byte span within a file, when a producer localised the offending region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Span {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}

/// A single piece of changed-code context: an offending path and, when a
/// producer localised it, the span within that path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CodeContext {
    /// The offending repo-relative path (or a producer's source label).
    pub path: String,
    /// The offending span within [`path`](Self::path), when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

impl CodeContext {
    /// A whole-path context with no localised span.
    #[must_use]
    pub fn path(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            span: None,
        }
    }

    /// A context localised to a span within a path.
    #[must_use]
    pub fn spanned(path: impl Into<String>, span: Span) -> Self {
        Self {
            path: path.into(),
            span: Some(span),
        }
    }
}

/// One unified, remediation-first explanation of a policy failure.
///
/// Built from any of the three producers via [`From<&AssertionGuidance>`],
/// [`PolicyGuidance::from_risk_guidance`], or
/// [`PolicyGuidance::from_pack_finding`]. See the [module docs](self) for what
/// the shape carries and, deliberately, does not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyGuidance {
    /// The id of the rule, assertion, or risk category that failed.
    pub rule_id: String,
    /// Where the failure came from (closed [`PolicySource`] set).
    pub source: PolicySource,
    /// Why the rule exists, when the producer supplied a rationale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    /// The offending changed-code context, sorted and de-duplicated. Empty when
    /// the producer named no location.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<CodeContext>,
    /// How to fix it — remediation-first guidance from the producer.
    pub remediation: String,
    /// How to request an exception — a static, parameterised hint naming
    /// `anvil exception grant` with the rule id. Text only: this does not wire
    /// the exceptions store.
    pub exception_guidance: String,
}

impl PolicyGuidance {
    /// Build guidance from a Rego pack eval [`Finding`].
    ///
    /// A [`Finding`] carries the offending dependency edge (`from`/`to`) but
    /// not its rule identity, pack source, or remediation — those live in pack
    /// metadata — so the caller supplies them. Both sides of the edge, when
    /// present, become changed-code context.
    #[must_use]
    pub fn from_pack_finding(
        finding: &Finding,
        rule_id: impl Into<String>,
        pack: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        let context = [finding.from.as_ref(), finding.to.as_ref()]
            .into_iter()
            .flatten()
            .map(|p| CodeContext::path(p.clone()))
            .collect();
        Self::build(
            rule_id.into(),
            PolicySource::Pack(pack.into()),
            None,
            context,
            remediation.into(),
        )
    }

    /// Build guidance from an IO-risk [`RiskGuidance`].
    ///
    /// The rule id is the finding's category code (e.g. `prompt-injection`);
    /// the caller supplies the scanner name that produced it. The finding's
    /// source label, when present, becomes changed-code context.
    #[must_use]
    pub fn from_risk_guidance(guidance: &RiskGuidance, scanner: impl Into<String>) -> Self {
        let context = guidance
            .source
            .as_ref()
            .map(|s| CodeContext::path(s.clone()))
            .into_iter()
            .collect();
        Self::build(
            risk_rule_id(guidance.code),
            PolicySource::Scanner(scanner.into()),
            None,
            context,
            guidance.remediation.clone(),
        )
    }

    /// Shared constructor: normalises the context ordering and derives the
    /// exception guidance from the rule id.
    fn build(
        rule_id: String,
        source: PolicySource,
        rationale: Option<String>,
        mut context: Vec<CodeContext>,
        remediation: String,
    ) -> Self {
        context.sort();
        context.dedup();
        let rationale = rationale.and_then(|r| {
            let trimmed = r.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });
        let exception_guidance = exception_hint(&rule_id);
        Self {
            rule_id,
            source,
            rationale,
            context,
            remediation,
            exception_guidance,
        }
    }
}

impl From<&AssertionGuidance> for PolicyGuidance {
    /// A CPOL assertion violation is self-describing: its `assertion_id` is both
    /// the rule id and the [`PolicySource::Assertion`] name, and it already
    /// carries rationale, remediation, and the offending path.
    fn from(guidance: &AssertionGuidance) -> Self {
        let context = guidance
            .path
            .as_ref()
            .map(|p| CodeContext::path(p.clone()))
            .into_iter()
            .collect();
        Self::build(
            guidance.assertion_id.clone(),
            PolicySource::Assertion(guidance.assertion_id.clone()),
            guidance.rationale.clone(),
            context,
            guidance.remediation.clone(),
        )
    }
}

impl From<AssertionGuidance> for PolicyGuidance {
    fn from(guidance: AssertionGuidance) -> Self {
        Self::from(&guidance)
    }
}

/// The static, parameterised exception hint for a rule.
///
/// Names `anvil exception grant` with the rule id so an author knows exactly
/// how to request an exception. Text only — it does not wire the exceptions
/// store. UK spelling.
fn exception_hint(rule_id: &str) -> String {
    format!(
        "To request an exception for rule `{rule_id}`, run \
         `anvil exception grant {rule_id}` and record the justification; \
         the finding is suppressed only once the exception is granted and honoured."
    )
}

/// The rule id for a risk finding is its category code in kebab-case wire form
/// (e.g. `prompt-injection`), obtained via the code's own serde form so it stays
/// in step with [`RiskGuidanceCode`] rather than duplicating its spelling.
fn risk_rule_id(code: RiskGuidanceCode) -> String {
    serde_json::to_value(code)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::guidance::GuidanceCode;
    use crate::io_risk::guidance::guidance_for_finding;
    use crate::pack::PolicySeverity;
    use crate::result::Severity;
    use anvil_kernel_types::io_risk::{
        Confidence, RiskCategory, RiskFinding, RiskLocation, RiskSeverity,
    };

    fn assertion_guidance() -> AssertionGuidance {
        AssertionGuidance {
            code: GuidanceCode::ChangedPathNotConfined,
            severity: PolicySeverity::High,
            assertion_id: "confine-to-src".into(),
            path: Some("scripts/deploy.sh".into()),
            message: "assertion `confine-to-src` failed: path escapes".into(),
            remediation: "Move the change under crates/ or split it out.".into(),
            rationale: Some("Small scoped changes keep the blast radius small.".into()),
        }
    }

    fn risk_guidance() -> RiskGuidance {
        guidance_for_finding(
            &RiskFinding::new(
                RiskCategory::PromptInjection,
                RiskSeverity::High,
                Confidence::High,
                "untrusted marker present",
                "Neutralise the flagged content.",
            )
            .with_location(RiskLocation {
                source: Some("prompt:user".into()),
                start: Some(0),
                end: Some(4),
            }),
        )
    }

    fn pack_finding() -> Finding {
        Finding {
            severity: Severity::Error,
            message: "import crosses an architecture boundary".into(),
            from: Some("crates/app/src/ui.rs".into()),
            to: Some("crates/app/src/db.rs".into()),
            fingerprint: Some("a1b2c3d4".into()),
            is_new_edge: true,
            baselined: false,
        }
    }

    #[test]
    fn policy_guidance_contract_from_assertion_guidance_maps_every_field() {
        let g = PolicyGuidance::from(&assertion_guidance());
        assert_eq!(g.rule_id, "confine-to-src");
        assert_eq!(g.source, PolicySource::Assertion("confine-to-src".into()));
        assert_eq!(
            g.rationale.as_deref(),
            Some("Small scoped changes keep the blast radius small.")
        );
        assert_eq!(g.context, vec![CodeContext::path("scripts/deploy.sh")]);
        assert_eq!(
            g.remediation,
            "Move the change under crates/ or split it out."
        );
    }

    #[test]
    fn policy_guidance_contract_from_risk_guidance_uses_category_as_rule_id() {
        let g = PolicyGuidance::from_risk_guidance(&risk_guidance(), "prompt-scanner");
        assert_eq!(g.rule_id, "prompt-injection");
        assert_eq!(g.source, PolicySource::Scanner("prompt-scanner".into()));
        assert!(g.rationale.is_none());
        assert_eq!(g.context, vec![CodeContext::path("prompt:user")]);
        assert_eq!(g.remediation, "Neutralise the flagged content.");
    }

    #[test]
    fn policy_guidance_contract_from_pack_finding_carries_both_edge_sides() {
        let g = PolicyGuidance::from_pack_finding(
            &pack_finding(),
            "arch-boundary",
            "policies/arch_boundary.rego",
            "Route the call through the public API instead.",
        );
        assert_eq!(g.rule_id, "arch-boundary");
        assert_eq!(
            g.source,
            PolicySource::Pack("policies/arch_boundary.rego".into())
        );
        // Both edge endpoints appear, sorted deterministically.
        assert_eq!(
            g.context,
            vec![
                CodeContext::path("crates/app/src/db.rs"),
                CodeContext::path("crates/app/src/ui.rs"),
            ]
        );
        assert_eq!(
            g.remediation,
            "Route the call through the public API instead."
        );
    }

    #[test]
    fn policy_guidance_contract_unifies_three_producers_into_one_vocabulary() {
        // The acceptance criterion: one output type over the three producers.
        let from_assertion = PolicyGuidance::from(&assertion_guidance());
        let from_risk = PolicyGuidance::from_risk_guidance(&risk_guidance(), "prompt-scanner");
        let from_pack = PolicyGuidance::from_pack_finding(
            &pack_finding(),
            "arch-boundary",
            "policies/arch_boundary.rego",
            "Fix it.",
        );
        for g in [&from_assertion, &from_risk, &from_pack] {
            assert!(!g.rule_id.is_empty(), "every producer yields a rule id");
            assert!(
                !g.remediation.is_empty(),
                "every producer yields remediation text"
            );
            assert!(
                g.exception_guidance.contains("anvil exception grant"),
                "every producer yields exception guidance: {}",
                g.exception_guidance
            );
        }
        // The source discriminates the producer.
        assert!(matches!(from_assertion.source, PolicySource::Assertion(_)));
        assert!(matches!(from_risk.source, PolicySource::Scanner(_)));
        assert!(matches!(from_pack.source, PolicySource::Pack(_)));
    }

    #[test]
    fn policy_guidance_contract_exception_hint_names_command_and_rule() {
        let g = PolicyGuidance::from(&assertion_guidance());
        assert!(
            g.exception_guidance
                .contains("anvil exception grant confine-to-src")
        );
        assert!(g.exception_guidance.contains("confine-to-src"));
        // UK spelling in user-facing text.
        assert!(g.exception_guidance.contains("honoured"));
    }

    #[test]
    fn policy_guidance_contract_context_is_deterministically_ordered_and_deduped() {
        // A finding whose edge endpoints arrive out of order must sort, and a
        // duplicated endpoint must collapse.
        let finding = Finding {
            from: Some("z/late.rs".into()),
            to: Some("a/early.rs".into()),
            ..pack_finding()
        };
        let g = PolicyGuidance::from_pack_finding(&finding, "r", "p", "fix");
        assert_eq!(
            g.context,
            vec![
                CodeContext::path("a/early.rs"),
                CodeContext::path("z/late.rs"),
            ]
        );

        let self_edge = Finding {
            from: Some("same.rs".into()),
            to: Some("same.rs".into()),
            ..pack_finding()
        };
        let g = PolicyGuidance::from_pack_finding(&self_edge, "r", "p", "fix");
        assert_eq!(g.context, vec![CodeContext::path("same.rs")]);
    }

    #[test]
    fn policy_guidance_contract_round_trips_through_json() {
        let original = PolicyGuidance::from(&assertion_guidance());
        let json = serde_json::to_string(&original).expect("serialise");
        let restored: PolicyGuidance = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(original, restored);
    }

    #[test]
    fn policy_guidance_contract_source_wire_form_is_kebab_case_externally_tagged() {
        let json = serde_json::to_string(&PolicySource::Pack("p.rego".into())).expect("serialise");
        assert_eq!(json, r#"{"pack":"p.rego"}"#);
        let json = serde_json::to_string(&PolicySource::Assertion("a".into())).expect("serialise");
        assert_eq!(json, r#"{"assertion":"a"}"#);
        let json = serde_json::to_string(&PolicySource::Scanner("s".into())).expect("serialise");
        assert_eq!(json, r#"{"scanner":"s"}"#);
    }

    #[test]
    fn policy_guidance_contract_absent_optionals_are_skip_serialised() {
        // A risk finding with no location and no rationale: `rationale` and
        // `context` must be omitted from the wire form, not emitted as null/[].
        let bare = guidance_for_finding(&RiskFinding::new(
            RiskCategory::UnsafeInput,
            RiskSeverity::Medium,
            Confidence::Low,
            "m",
            "r",
        ));
        let g = PolicyGuidance::from_risk_guidance(&bare, "scanner");
        let json = serde_json::to_string(&g).expect("serialise");
        assert!(!json.contains("\"rationale\""), "{json}");
        assert!(!json.contains("\"context\""), "{json}");
        // But the required fields are always present.
        assert!(json.contains("\"rule_id\""), "{json}");
        assert!(json.contains("\"exception_guidance\""), "{json}");
        let restored: PolicyGuidance = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(restored, g);
    }

    #[test]
    fn policy_guidance_contract_blank_rationale_is_normalised_to_none() {
        let mut ag = assertion_guidance();
        ag.rationale = Some("   ".into());
        let g = PolicyGuidance::from(&ag);
        assert!(g.rationale.is_none(), "blank rationale must normalise away");
    }

    #[test]
    fn policy_guidance_contract_span_survives_round_trip() {
        let ctx = CodeContext::spanned("src/lib.rs", Span { start: 10, end: 42 });
        let json = serde_json::to_string(&ctx).expect("serialise");
        let restored: CodeContext = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(restored, ctx);
        assert_eq!(restored.span, Some(Span { start: 10, end: 42 }));
    }
}
