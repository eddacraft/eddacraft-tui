//! INTR-001: `InterceptRule` trait — the contract every hot-path rule
//! implements so the intercept daemon can compose and short-circuit them.
//!
//! The trait is intentionally object-safe so the rule registry (INTR-006)
//! can hold a `Vec<Box<dyn InterceptRule>>`. Inputs are a per-file
//! evaluation context (path, change kind, optional in-memory content) so
//! rules can be reused across:
//!
//! - the intercept daemon's on-disk path (INTD-005), which reads file
//!   content from the filesystem before evaluation, and
//! - the RMCP / RTAI mid-edit and pre-write paths, which supply
//!   caller-provided proposed content directly.
//!
//! See `plans/modules/intercept-rules.aps.md` for module scope and
//! `plans/decisions/015-intercept-loop-enforcement.md` for the broader
//! enforcement design.

#![forbid(unsafe_code)]

pub mod antipattern;
pub mod config;
pub mod path_deny;
pub mod reasoning;
pub mod regex_content;
pub mod registry;
pub mod secret;

use std::num::NonZeroU32;
use std::path::Path;

use anvil_kernel_types::diagnostics::KnownMode;
use anvil_kernel_types::{Category, Diagnostic, DiagnosticSource, Location, Mode, Severity};
use serde::{Deserialize, Serialize};

pub use antipattern::AntipatternScanRule;
pub use config::{
    InterceptRulesConfig, RuleConfigError, registry_from_value, registry_from_workspace,
};
pub use path_deny::{PathDenyConfig, PathDenyError, PathDenyListRule};
pub use reasoning::LaunchReasoningPatternRule;
pub use regex_content::{RegexContentConfig, RegexContentError, RegexContentRule};
pub use registry::{
    RegistryDecision, RegistryError, RegistryMode, RuleRegistry, ScopedEvaluation, ScopedRuleSkip,
    TOUCHED_NODE_SKIP_REASON,
};
pub use secret::SecretDetectionRule;

/// The kind of file change being evaluated. Mirrors the kernel watcher's
/// `ChangeKind` shape so the daemon's adapter is a 1:1 map; declared
/// locally to keep the rules crate dep-light (no `anvil-kernel` edge).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeKind {
    Created,
    Modified,
    Removed,
}

/// Per-file input passed to a rule's `evaluate` call. Borrows everything
/// that can be borrowed so a single change can be fed to many rules
/// without cloning.
#[derive(Debug)]
pub struct RuleInput<'a> {
    /// Workspace-relative or absolute path of the changed file.
    pub path: &'a Path,
    /// What happened to the file.
    pub change_kind: ChangeKind,
    /// File content to evaluate, if available.
    ///
    /// `None` for `Removed` changes, oversize files (>1 MiB cap per
    /// INTD-005), or paths the caller chose not to read. Rules that
    /// require content MUST early-return [`RuleDecision::Allow`] when
    /// this is `None` — content-bearing checks are not run on missing
    /// content.
    pub content: Option<&'a [u8]>,
}

impl RuleInput<'_> {
    /// CIB-006: touched-node predicate for risk-tiered validation.
    ///
    /// Answers whether a rule with the given `needs_content`
    /// declaration has at least one declared input that overlaps a
    /// change confined to a single touched node, where `self` is the
    /// node-scoped input built for that change (path preserved,
    /// `content` reduced to the touched node's content).
    ///
    /// - **Path-input rules** (`needs_content == false`) always
    ///   overlap: however small the change, it is still a write to
    ///   `self.path`, so a rule keyed on path or change kind (for
    ///   example a path deny-list) must keep firing on the tiered
    ///   path. Skipping it could skip a check that catches a real
    ///   risk.
    /// - **Content-input rules** overlap when the scoped content is
    ///   present. When the caller could not materialise the touched
    ///   node's content, the rule is reported as non-overlapping so
    ///   the caller records an explicit skip (with a reason) instead
    ///   of relying on the silent content-skip inside
    ///   [`RuleRegistry::evaluate`].
    ///
    /// The predicate is deliberately conservative: it can only answer
    /// `false` when the rule would have no input at all to evaluate.
    /// It must never be used to drop a rule that could produce a new
    /// finding from the touched node.
    #[must_use]
    pub fn overlaps_touched_node(&self, needs_content: bool) -> bool {
        !needs_content || self.content.is_some()
    }
}

/// The decision a rule produces for a single [`RuleInput`].
///
/// The v1 enforcement contract is binary: a rule either lets the change
/// through (`Allow`) or short-circuits the pipeline with an interrupt
/// reason (`Interrupt`). Severity-aware decisions (`warn` vs `block`)
/// are layered on at the daemon's enforcement-mode adapter (INTD-008),
/// not in the rule trait.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum RuleDecision {
    /// The rule found no violation; evaluation continues.
    Allow,
    /// The rule found a violation; the pipeline short-circuits.
    Interrupt(InterruptReason),
}

/// Why a rule interrupted. Stable, machine-readable, surface-agnostic —
/// the daemon, MCP shim, and editor drivers all map this onto their own
/// canonical diagnostic envelopes (AIGUARD-002).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterruptReason {
    /// Stable identifier of the rule that fired (e.g. `"secret-detection"`).
    pub rule_id: String,
    /// Operator-visible summary of the violation.
    pub message: String,
    /// Optional 1-based line number inside `path` where the violation
    /// was found. `None` for path-only rules and for content-bearing
    /// rules that cannot localise.
    ///
    /// Typed `NonZeroU32` so the 1-based invariant is unrepresentable
    /// rather than merely asserted: a `line: 0` (almost always an
    /// off-by-one from a 0-based parser index) cannot be constructed,
    /// and serde rejects it on deserialise instead of letting a phantom
    /// `line: 0` diagnostic through.
    pub line: Option<NonZeroU32>,
}

impl RuleDecision {
    /// Convenience constructor for the common "allow with no metadata" case.
    #[must_use]
    pub fn allow() -> Self {
        Self::Allow
    }

    /// Convenience constructor for an interrupt with rule id + message.
    /// `line` is `None`; use [`RuleDecision::interrupt_at`] when the
    /// rule can localise the violation to a specific line.
    #[must_use]
    pub fn interrupt(rule_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Interrupt(InterruptReason {
            rule_id: rule_id.into(),
            message: message.into(),
            line: None,
        })
    }

    /// Convenience constructor for an interrupt that knows the 1-based
    /// line number of the violation. Content-scanning rules
    /// (secret-detection, regex-content) are expected to use this.
    ///
    /// **Panics** if `line == 0`. The line number is contractually
    /// 1-based; passing `0` is almost always an off-by-one bug from a
    /// rule that mistakenly forwarded a 0-based parser index. The
    /// registry (INTR-006) wraps every rule call in `catch_unwind`, so
    /// a misbehaving rule that violates this precondition is isolated
    /// from the daemon's tokio task — but the assertion still surfaces
    /// the bug to the rule author rather than letting it serialise as
    /// a phantom `line: 0` diagnostic.
    #[must_use]
    pub fn interrupt_at(rule_id: impl Into<String>, message: impl Into<String>, line: u32) -> Self {
        let line = NonZeroU32::new(line).expect(
            "RuleDecision::interrupt_at requires a 1-based line number; \
             got 0 — convert from a 0-based parser index by adding 1, \
             or use RuleDecision::interrupt() if the rule cannot localise.",
        );
        Self::Interrupt(InterruptReason {
            rule_id: rule_id.into(),
            message: message.into(),
            line: Some(line),
        })
    }
}

/// The contract every hot-path rule implements.
///
/// **Object-safety:** the trait is `dyn`-compatible — only `&self`
/// methods, no generic methods, no associated types. The registry
/// (INTR-006) relies on this to hold heterogeneous rules behind
/// `Box<dyn InterceptRule>`.
///
/// **Lifetime:** the trait is bound `+ 'static` so rules stored in the
/// registry own their data. Rules that want to borrow from outer
/// scopes cannot be boxed — they must take ownership (e.g. clone the
/// config they need into the rule struct).
///
/// **Latency:** implementations MUST execute in microseconds to
/// hundreds of microseconds. No graph recomputation, no network calls,
/// no expensive AST analysis. See `plans/modules/intercept-rules.aps.md`
/// "Out of Scope" for the full list.
///
/// **Panic policy:** implementations MUST NOT panic. The registry
/// (INTR-006) is the layer responsible for *enforcing* this — every
/// `evaluate` call there is expected to be wrapped in
/// `std::panic::catch_unwind` so a misbehaving rule cannot abort the
/// daemon's tokio task. The
/// `panicking_rule_unwinds_via_catch_unwind` test in this crate pins
/// the contract: a panicking `Box<dyn InterceptRule>` does unwind
/// through dyn dispatch, so the registry's `catch_unwind` wrapper is
/// the correct (and only) place to isolate it.
pub trait InterceptRule: Send + Sync + 'static {
    /// Stable identifier the registry uses for ordering, dedup, and
    /// observability. MUST be globally unique across registered rules.
    fn rule_id(&self) -> &str;

    /// Whether this rule needs file content to make a decision. The
    /// registry uses this to skip content reads when no content-bearing
    /// rule is registered.
    fn needs_content(&self) -> bool;

    /// Evaluate `input` and return a decision. MUST NOT panic on
    /// malformed input — return `Allow` and let a higher layer log if
    /// rule data is unusable.
    fn evaluate(&self, input: &RuleInput<'_>) -> RuleDecision;

    /// Return canonical diagnostics for `input` in `mode`.
    ///
    /// Rules with richer diagnostic metadata should override this. The
    /// default preserves the existing interrupt semantics and maps the
    /// first interrupt into a generic diagnostic so all rules can take
    /// part in diagnostic-returning surfaces without a parallel trait.
    fn diagnostics(&self, input: &RuleInput<'_>, mode: &Mode) -> Vec<Diagnostic> {
        self.diagnostics_with_limit(input, mode, usize::MAX)
    }

    /// Return at most `limit` canonical diagnostics for `input` in `mode`.
    ///
    /// The default maps the first interrupt into one diagnostic. Rules that
    /// can emit many diagnostics should override this so callers can enforce
    /// response-size budgets before allocating the full finding set.
    fn diagnostics_with_limit(
        &self,
        input: &RuleInput<'_>,
        mode: &Mode,
        limit: usize,
    ) -> Vec<Diagnostic> {
        if limit == 0 {
            return Vec::new();
        }
        match self.evaluate(input) {
            RuleDecision::Allow => Vec::new(),
            RuleDecision::Interrupt(reason) => {
                vec![interrupt_reason_to_diagnostic(
                    input.path,
                    reason,
                    mode.clone(),
                )]
            }
        }
    }
}

fn interrupt_reason_to_diagnostic(path: &Path, reason: InterruptReason, mode: Mode) -> Diagnostic {
    let path = path.to_string_lossy();
    Diagnostic::new(
        format!(
            "diag_intercept_{}_{}_{}_{}",
            mode_id_part(&mode),
            sanitise_id_part(path.as_ref()),
            reason.line.map_or(0, NonZeroU32::get),
            sanitise_id_part(&reason.rule_id)
        ),
        Severity::Error,
        reason.message,
        Location {
            file: path.into_owned(),
            line: reason.line.map(NonZeroU32::get),
            column: None,
            end_line: None,
            end_column: None,
        },
        Category::Other,
        DiagnosticSource {
            rule_id: reason.rule_id,
            source_module: "anvil-intercept-rules".to_string(),
        },
        mode,
    )
}

pub(crate) fn sanitise_id_part(value: &str) -> String {
    let sanitised = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    if sanitised.is_empty() {
        "unknown".to_string()
    } else {
        sanitised
    }
}

pub(crate) fn mode_id_part(mode: &Mode) -> String {
    match mode {
        Mode::Known(KnownMode::SaveTime) => "save_time".to_owned(),
        Mode::Known(KnownMode::MidEdit) => "mid_edit".to_owned(),
        Mode::Known(KnownMode::Gate) => "gate".to_owned(),
        Mode::Known(KnownMode::Watch) => "watch".to_owned(),
        Mode::Unknown(value) => sanitise_id_part(value),
    }
}

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::path::PathBuf;

    use super::*;

    /// Compile-time proof that `InterceptRule` is object-safe. If this
    /// fails to compile, the trait grew a generic method or `Self`-by-
    /// value receiver and lost its dyn-compat — the registry breaks.
    #[test]
    fn intercept_rule_is_object_safe() {
        let rules: Vec<Box<dyn InterceptRule>> = Vec::new();
        assert!(rules.is_empty());
    }

    /// Stub rule used by the trait-shape tests below. Lives inside the
    /// test module so production code stays free of test-only types.
    struct StubRule {
        id: &'static str,
        needs_content: bool,
        decision: RuleDecision,
    }

    impl InterceptRule for StubRule {
        fn rule_id(&self) -> &str {
            self.id
        }

        fn needs_content(&self) -> bool {
            self.needs_content
        }

        fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
            self.decision.clone()
        }
    }

    fn input_for<'a>(path: &'a Path, content: Option<&'a [u8]>) -> RuleInput<'a> {
        RuleInput {
            path,
            change_kind: ChangeKind::Modified,
            content,
        }
    }

    #[test]
    fn allow_decision_round_trips_through_dyn_dispatch() {
        let rule: Box<dyn InterceptRule> = Box::new(StubRule {
            id: "stub-allow",
            needs_content: false,
            decision: RuleDecision::allow(),
        });

        let path = PathBuf::from("src/lib.rs");
        let decision = rule.evaluate(&input_for(&path, None));

        assert_eq!(decision, RuleDecision::Allow);
        assert_eq!(rule.rule_id(), "stub-allow");
        assert!(!rule.needs_content());
    }

    #[test]
    fn interrupt_decision_carries_rule_id_and_message() {
        let rule: Box<dyn InterceptRule> = Box::new(StubRule {
            id: "stub-interrupt",
            needs_content: true,
            decision: RuleDecision::interrupt("stub-interrupt", "stub fired"),
        });

        let path = PathBuf::from("src/lib.rs");
        let body = b"const TOKEN: &str = \"abcd1234\";\n";
        let decision = rule.evaluate(&input_for(&path, Some(body)));

        match decision {
            RuleDecision::Interrupt(reason) => {
                assert_eq!(reason.rule_id, "stub-interrupt");
                assert_eq!(reason.message, "stub fired");
                assert_eq!(reason.line, None);
            }
            RuleDecision::Allow => panic!("expected Interrupt, got Allow"),
        }
        assert!(rule.needs_content());
    }

    #[test]
    fn interrupt_at_decision_carries_line_number() {
        let decision = RuleDecision::interrupt_at("secret-detection", "potential secret", 4);
        match decision {
            RuleDecision::Interrupt(reason) => {
                assert_eq!(reason.rule_id, "secret-detection");
                assert_eq!(reason.message, "potential secret");
                assert_eq!(reason.line, NonZeroU32::new(4));
            }
            RuleDecision::Allow => panic!("expected Interrupt, got Allow"),
        }
    }

    /// `interrupt_at(.., 0)` panics — the constructor's 1-based
    /// contract is enforced at runtime so a rule that accidentally
    /// forwards a 0-based parser index surfaces the bug immediately
    /// rather than silently emitting a phantom `line: 0` diagnostic.
    #[test]
    #[should_panic(expected = "1-based")]
    fn interrupt_at_with_zero_line_panics() {
        let _ = RuleDecision::interrupt_at("secret-detection", "potential secret", 0);
    }

    /// CIB-006: the touched-node predicate never excludes a rule whose
    /// declared inputs include the path — every change, however small,
    /// is still a write to that path, so path-scoped rules (e.g. a
    /// path deny-list) must keep firing on the risk-tiered path.
    #[test]
    fn touched_node_predicate_path_rules_always_overlap() {
        let path = PathBuf::from("config.json");
        let with_content = input_for(&path, Some(b"\"value\""));
        let without_content = input_for(&path, None);

        assert!(with_content.overlaps_touched_node(false));
        assert!(
            without_content.overlaps_touched_node(false),
            "a path-input rule must overlap even when no scoped content exists",
        );
    }

    /// CIB-006: content-input rules overlap the touched node only when
    /// the scoped content was materialised. The `false` answer lets a
    /// caller record an explicit skip (with a reason) instead of the
    /// silent content-skip inside the registry loop.
    #[test]
    fn touched_node_predicate_content_rules_require_scoped_content() {
        let path = PathBuf::from("config.json");
        let with_content = input_for(&path, Some(b"\"value\""));
        let without_content = input_for(&path, None);

        assert!(with_content.overlaps_touched_node(true));
        assert!(!without_content.overlaps_touched_node(true));
    }

    #[test]
    fn rule_input_carries_change_kind_and_optional_content() {
        let path = PathBuf::from("src/removed.rs");
        let removed = RuleInput {
            path: &path,
            change_kind: ChangeKind::Removed,
            content: None,
        };
        assert_eq!(removed.change_kind, ChangeKind::Removed);
        assert!(removed.content.is_none());

        let body = b"hello";
        let created = RuleInput {
            path: &path,
            change_kind: ChangeKind::Created,
            content: Some(body),
        };
        assert_eq!(created.change_kind, ChangeKind::Created);
        assert_eq!(created.content, Some(body.as_slice()));
    }

    #[test]
    fn rule_decision_serialises_with_decision_tag() {
        let allow = serde_json::to_value(RuleDecision::Allow).expect("serialise allow");
        assert_eq!(allow, serde_json::json!({ "decision": "allow" }));

        let interrupt = RuleDecision::Interrupt(InterruptReason {
            rule_id: "secret-detection".into(),
            message: "potential secret".into(),
            line: NonZeroU32::new(4),
        });
        let payload = serde_json::to_value(&interrupt).expect("serialise interrupt");
        assert_eq!(
            payload,
            serde_json::json!({
                "decision": "interrupt",
                "rule_id": "secret-detection",
                "message": "potential secret",
                "line": 4,
            })
        );
    }

    #[test]
    fn interrupt_reason_rejects_zero_line_on_deserialise() {
        // The 1-based line invariant is now carried by the type
        // (`Option<NonZeroU32>`), so a `line: 0` that slipped past a
        // producing rule cannot round-trip — serde refuses to build the
        // phantom diagnostic rather than letting it through.
        let zero_line = serde_json::json!({
            "decision": "interrupt",
            "rule_id": "secret-detection",
            "message": "potential secret",
            "line": 0,
        });
        let parsed: Result<RuleDecision, _> = serde_json::from_value(zero_line);
        assert!(
            parsed.is_err(),
            "line: 0 must be rejected on deserialise, got {parsed:?}"
        );

        // A genuine 1-based line still round-trips cleanly.
        let one_line = serde_json::json!({
            "decision": "interrupt",
            "rule_id": "secret-detection",
            "message": "potential secret",
            "line": 1,
        });
        let parsed: RuleDecision =
            serde_json::from_value(one_line).expect("1-based line round-trips");
        match parsed {
            RuleDecision::Interrupt(reason) => {
                assert_eq!(reason.line, NonZeroU32::new(1));
            }
            RuleDecision::Allow => panic!("expected Interrupt"),
        }
    }

    /// A rule that violates the trait's "MUST NOT panic" contract.
    /// Used to pin the panic-isolation contract: `Box<dyn
    /// InterceptRule>` does propagate panics through dyn dispatch, so
    /// the registry (INTR-006) is the layer that must wrap every
    /// `evaluate` call in `catch_unwind` to keep a misbehaving rule
    /// from aborting the daemon's tokio task. If this test ever stops
    /// failing the unwind, the trait surface has changed in a way
    /// that breaks the registry's isolation strategy.
    struct PanickingRule;

    impl InterceptRule for PanickingRule {
        fn rule_id(&self) -> &'static str {
            "panic-rule"
        }

        fn needs_content(&self) -> bool {
            false
        }

        fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
            panic!("rule misbehaviour");
        }
    }

    #[test]
    fn panicking_rule_unwinds_via_catch_unwind() {
        let rule: Box<dyn InterceptRule> = Box::new(PanickingRule);
        let path = PathBuf::from("test.rs");
        let result = catch_unwind(AssertUnwindSafe(|| rule.evaluate(&input_for(&path, None))));
        assert!(
            result.is_err(),
            "panicking rule must surface as Err — registry (INTR-006) is responsible for wrapping evaluate() in catch_unwind",
        );
    }
}
