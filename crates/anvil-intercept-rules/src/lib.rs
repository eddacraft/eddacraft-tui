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

use std::path::Path;

use serde::{Deserialize, Serialize};

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
    pub line: Option<u32>,
}

impl RuleDecision {
    /// Convenience constructor for the common "allow with no metadata" case.
    #[must_use]
    pub fn allow() -> Self {
        Self::Allow
    }

    /// Convenience constructor for an interrupt with rule id + message.
    #[must_use]
    pub fn interrupt(rule_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Interrupt(InterruptReason {
            rule_id: rule_id.into(),
            message: message.into(),
            line: None,
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
/// **Latency:** implementations MUST execute in microseconds to
/// hundreds of microseconds. No graph recomputation, no network calls,
/// no expensive AST analysis. See `plans/modules/intercept-rules.aps.md`
/// "Out of Scope" for the full list.
pub trait InterceptRule: Send + Sync {
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
}

#[cfg(test)]
mod tests {
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
            line: Some(4),
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
}
