//! INTR-006: rule registry.
//!
//! Composes a `Vec<Box<dyn InterceptRule>>` into an ordered evaluation
//! pipeline. The registry is the layer responsible for:
//!
//! - **Evaluation ordering** — rules fire in registration order.
//! - **Short-circuit semantics** — in enforce mode, the first
//!   [`RuleDecision::Interrupt`] terminates evaluation and is returned.
//! - **Observe-only mode** — interrupts are logged via the panic /
//!   tracing path but the registry's overall decision stays
//!   [`RegistryDecision::Allow`]. Useful for "shadow" rollouts where
//!   an operator wants to see what would have fired without breaking
//!   anyone's flow.
//! - **Panic isolation** — every `evaluate` call is wrapped in
//!   [`std::panic::catch_unwind`]. The trait contract says rules MUST
//!   NOT panic; the registry is the layer that *enforces* that
//!   contract, so a misbehaving rule cannot abort the daemon's tokio
//!   task. A panicking rule is treated as if it returned `Allow`.
//!
//!   **`catch_unwind` is effective in release too.** It only works
//!   when the binary unwinds rather than aborts on panic, and the
//!   Anvil workspace's `[profile.release]` sets `panic = "unwind"`
//!   (see the top-level `Cargo.toml`, per ADR-051 — Accepted). ADR-051
//!   chose unwind precisely because `anvil` processes untrusted input
//!   and a panic must surface as a structured error rather than a
//!   `SIGABRT`. Consequently this isolation holds in release builds as
//!   well as debug / test — a panicking rule is caught and treated as
//!   `Allow` regardless of profile. (The trait contract still asks
//!   rules to be panic-free by construction as the long-term answer,
//!   but the registry no longer depends on it for crash-safety.)
//! - **Cached rule ids** — every rule's [`InterceptRule::rule_id`] is
//!   sampled once at registration time and stored alongside the rule.
//!   The hot path never calls `rule_id()` again, so a misbehaving
//!   rule that panics in `rule_id` cannot crash evaluation. The
//!   cached id is also the canonical answer for dedup checks, the
//!   `rule_ids()` accessor, log output, and the `InterruptReason`
//!   normalisation step (a rule that returns an `InterruptReason`
//!   with a mismatched `rule_id` has its id rewritten to the cached
//!   one — observability invariants are non-negotiable).
//! - **Duplicate detection** — registering two rules with the same
//!   [`InterceptRule::rule_id`] is a programmer error and surfaces as
//!   [`RegistryError::DuplicateRuleId`] rather than silently
//!   replacing the older registration.
//!
//! See `plans/modules/intercept-rules.aps.md` task INTR-006.

use std::collections::HashSet;
use std::panic::{AssertUnwindSafe, catch_unwind};

use anvil_kernel_types::{Diagnostic, Mode};

use crate::{InterceptRule, InterruptReason, RuleDecision, RuleInput};

/// The decision the registry produces for a single [`RuleInput`].
///
/// Distinct from [`RuleDecision`] because the registry layer has more
/// to say than a single rule does — in observe-only mode an interrupt
/// is recorded but does not flip the overall decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryDecision {
    /// All registered rules returned `Allow`, or the registry is in
    /// observe-only mode and any interrupts have been logged but not
    /// enforced.
    Allow,
    /// Enforce mode: the first rule that returned `Interrupt`. The
    /// pipeline short-circuited at this point — later rules were not
    /// evaluated.
    Interrupt(InterruptReason),
}

/// Errors surfaced by the registry's mutating API.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    /// Two rules tried to register the same `rule_id`. Rule ids MUST
    /// be globally unique per the trait contract — this would cause
    /// the second registration to silently overwrite the first
    /// (or, worse, evaluate twice with confusingly identical
    /// diagnostics) so we reject it.
    #[error("rule id already registered: {0}")]
    DuplicateRuleId(String),
}

/// Mode flag that controls whether interrupts terminate evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RegistryMode {
    /// Default: the first interrupt short-circuits and is returned.
    #[default]
    Enforce,
    /// Interrupts are logged but the overall decision is `Allow` and
    /// every rule still gets a chance to evaluate. Used for shadow
    /// rollouts and pre-flight diagnostics.
    ObserveOnly,
}

/// A registered rule plus the id we sampled from it at registration.
/// Holding the cached id alongside the trait object means the hot path
/// never has to call `rule_id()` again — see the module-level note on
/// cached rule ids for why that matters.
struct RegisteredRule {
    id: String,
    rule: Box<dyn InterceptRule>,
}

/// Ordered pipeline of [`InterceptRule`] instances. Cheap to iterate;
/// not internally synchronised — callers needing shared ownership wrap
/// it in `Arc<RuleRegistry>`. The registry is read-only after
/// construction in v1; reload-on-change lands with INTR-007.
pub struct RuleRegistry {
    rules: Vec<RegisteredRule>,
    mode: RegistryMode,
}

impl std::fmt::Debug for RuleRegistry {
    /// `Box<dyn InterceptRule>` is not `Debug`, so the auto-derive is
    /// out. We surface the mode and the cached rule ids (which stand
    /// in for the trait objects); `finish_non_exhaustive` documents
    /// that the `rules` storage isn't printed verbatim.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuleRegistry")
            .field("mode", &self.mode)
            .field("rule_ids", &self.rule_ids())
            .finish_non_exhaustive()
    }
}

impl RuleRegistry {
    /// Empty registry in [`RegistryMode::Enforce`] mode.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            mode: RegistryMode::Enforce,
        }
    }

    /// Empty registry in [`RegistryMode::ObserveOnly`] mode.
    #[must_use]
    pub fn observe_only() -> Self {
        Self {
            rules: Vec::new(),
            mode: RegistryMode::ObserveOnly,
        }
    }

    /// Bulk constructor. Preserves the input order. Rejects with
    /// [`RegistryError::DuplicateRuleId`] if any two entries share a
    /// `rule_id`.
    pub fn with_rules(rules: Vec<Box<dyn InterceptRule>>) -> Result<Self, RegistryError> {
        let mut registry = Self::new();
        for rule in rules {
            registry.register(rule)?;
        }
        Ok(registry)
    }

    /// Switch the registry into observe-only mode. Builder-style for
    /// callers that constructed via [`with_rules`].
    #[must_use]
    pub fn into_observe_only(mut self) -> Self {
        self.mode = RegistryMode::ObserveOnly;
        self
    }

    /// Append a rule to the pipeline. Order of registration is the
    /// order of evaluation.
    ///
    /// `rule_id()` is sampled once here and cached. The hot path never
    /// calls back into `rule_id()`, so even if a misbehaving rule
    /// panics in `rule_id()` later (against the trait contract), the
    /// registry's cached value keeps observability and dedup intact.
    pub fn register(&mut self, rule: Box<dyn InterceptRule>) -> Result<(), RegistryError> {
        let id = rule.rule_id().to_owned();
        if self.rules.iter().any(|existing| existing.id == id) {
            return Err(RegistryError::DuplicateRuleId(id));
        }
        self.rules.push(RegisteredRule { id, rule });
        Ok(())
    }

    /// Number of registered rules.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Whether the registry holds zero rules.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Current evaluation mode.
    #[must_use]
    pub fn mode(&self) -> RegistryMode {
        self.mode
    }

    /// `true` if any registered rule reports
    /// [`InterceptRule::needs_content`]. The daemon (INTD-005) reads
    /// this to decide whether to fault file content into the input —
    /// if no rule needs content, the read is skipped entirely.
    #[must_use]
    pub fn any_needs_content(&self) -> bool {
        self.rules.iter().any(|r| r.rule.needs_content())
    }

    /// Registered rule ids, in evaluation order. Cheap helper for
    /// status surfaces (`anvil intercept status`, INTD-011). Returns
    /// the ids cached at registration, not live `rule_id()` calls.
    #[must_use]
    pub fn rule_ids(&self) -> Vec<&str> {
        self.rules.iter().map(|r| r.id.as_str()).collect()
    }

    /// Evaluate `input` against the pipeline.
    ///
    /// Enforce mode: returns [`RegistryDecision::Interrupt`] from the
    /// first rule that fires; later rules are not called.
    ///
    /// Rules that require content are skipped when `input.content` is
    /// unavailable. Observe-only mode calls every remaining applicable
    /// rule regardless of interrupts. Interrupts are emitted on `stderr`
    /// and the returned decision is always [`RegistryDecision::Allow`].
    /// This is the "shadow rollout" path. (`tracing` is intentionally
    /// not a dep of this crate; if you wire one in, both this path and
    /// the panic path become `tracing::warn!` candidates — the eprintln
    /// calls are the minimum-dep fallback.)
    ///
    /// Panicking rules are isolated via `catch_unwind`: a panic from
    /// `evaluate` is caught, reported on stderr, and treated as if the
    /// rule had returned `Allow`. The workspace's release profile is
    /// `panic = "unwind"` (ADR-051), so this isolation holds in release
    /// as well as debug / test. See the module-level note for the
    /// broader story.
    ///
    /// `InterruptReason.rule_id` is normalised to the registry's
    /// cached id for the firing rule before returning or logging. If
    /// the rule emitted a different id, the registry's cached value
    /// wins — observability and dedup invariants must not depend on
    /// the rule getting its own id right.
    pub fn evaluate(&self, input: &RuleInput<'_>) -> RegistryDecision {
        for entry in &self.rules {
            let cached_id = entry.id.as_str();
            if input.content.is_none() && entry.rule.needs_content() {
                continue;
            }
            let Ok(decision) = catch_unwind(AssertUnwindSafe(|| entry.rule.evaluate(input))) else {
                // The trait contract says rules MUST NOT panic.
                // Surface the violation loudly and treat as Allow —
                // escalating a buggy rule into an Interrupt would
                // amplify the bug into a workflow break. The panic
                // payload is consumed here; reconstructing the
                // message portably across types is fraught.
                eprintln!(
                    "anvil-intercept-rules: rule {cached_id:?} panicked during evaluate; \
                     treating as Allow (rule contract violation)",
                );
                continue;
            };
            if let RuleDecision::Interrupt(mut reason) = decision {
                // Normalise: the registry is the canonical source for
                // the firing rule's id. A rule that emits a mismatched
                // id (accident or otherwise) gets its id overwritten
                // with the cached value so dedup and log output stay
                // correct.
                if reason.rule_id != cached_id {
                    reason.rule_id.clear();
                    reason.rule_id.push_str(cached_id);
                }
                match self.mode {
                    RegistryMode::Enforce => return RegistryDecision::Interrupt(reason),
                    RegistryMode::ObserveOnly => {
                        eprintln!(
                            "anvil-intercept-rules: observe-only — rule {cached_id:?} \
                             would interrupt: {} (line: {:?})",
                            reason.message, reason.line,
                        );
                        // Carry on evaluating so every violation is
                        // observable. Final decision is still Allow.
                    }
                }
            }
        }
        RegistryDecision::Allow
    }

    /// Evaluate `input` and return canonical diagnostics using the same
    /// ordering, content-skip, panic-isolation, and short-circuit rules
    /// as [`Self::evaluate`].
    #[must_use]
    pub fn diagnostics(&self, input: &RuleInput<'_>, mode: &Mode) -> Vec<Diagnostic> {
        self.diagnostics_with_limit(input, mode, usize::MAX)
    }

    /// Evaluate `input` and return at most `limit` canonical diagnostics.
    #[must_use]
    pub fn diagnostics_with_limit(
        &self,
        input: &RuleInput<'_>,
        mode: &Mode,
        limit: usize,
    ) -> Vec<Diagnostic> {
        if limit == 0 {
            return Vec::new();
        }
        let mut observe_only = Vec::new();
        let mut remaining = limit;
        for entry in &self.rules {
            let cached_id = entry.id.as_str();
            if input.content.is_none() && entry.rule.needs_content() {
                continue;
            }
            let Ok(mut diagnostics) = catch_unwind(AssertUnwindSafe(|| {
                entry.rule.diagnostics_with_limit(input, mode, remaining)
            })) else {
                eprintln!(
                    "anvil-intercept-rules: rule {cached_id:?} panicked during diagnostics; \
                     treating as no diagnostics (rule contract violation)",
                );
                continue;
            };
            if diagnostics.is_empty() {
                continue;
            }
            diagnostics.truncate(remaining);
            for diagnostic in &mut diagnostics {
                if diagnostic.source.rule_id != cached_id {
                    diagnostic.source.rule_id.clear();
                    diagnostic.source.rule_id.push_str(cached_id);
                }
            }

            match self.mode {
                RegistryMode::Enforce => return diagnostics,
                RegistryMode::ObserveOnly => {
                    remaining = remaining.saturating_sub(diagnostics.len());
                    observe_only.extend(diagnostics);
                    if remaining == 0 {
                        return observe_only;
                    }
                }
            }
        }
        observe_only
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Bulk-construct a deduplicated rule list, returning the ids that
/// would have collided. Useful for config-driven loading where the
/// caller wants to surface duplicates to the user instead of stopping
/// at the first one. Currently unused by INTR-006 itself but kept here
/// because INTR-007 (`.anvil.yaml` loading) is the obvious caller.
#[doc(hidden)]
pub fn deduplicate_ids(ids: impl IntoIterator<Item = String>) -> (Vec<String>, Vec<String>) {
    let mut seen: HashSet<String> = HashSet::new();
    let mut kept = Vec::new();
    let mut duplicates = Vec::new();
    for id in ids {
        if seen.insert(id.clone()) {
            kept.push(id);
        } else {
            duplicates.push(id);
        }
    }
    (kept, duplicates)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::ChangeKind;

    // ----- Stub rules used by the tests below. -----------------------

    /// Always returns the configured decision; counts how many times
    /// it was evaluated so tests can pin short-circuit behaviour.
    struct StubRule {
        id: &'static str,
        decision: RuleDecision,
        needs_content: bool,
        calls: AtomicUsize,
    }

    impl StubRule {
        fn new(id: &'static str, decision: RuleDecision) -> Self {
            Self {
                id,
                decision,
                needs_content: false,
                calls: AtomicUsize::new(0),
            }
        }

        fn needing_content(mut self) -> Self {
            self.needs_content = true;
            self
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl InterceptRule for StubRule {
        fn rule_id(&self) -> &'static str {
            self.id
        }

        fn needs_content(&self) -> bool {
            self.needs_content
        }

        fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.decision.clone()
        }
    }

    /// Always panics on evaluate — used to pin the registry's panic
    /// isolation contract.
    struct PanickingRule;

    impl InterceptRule for PanickingRule {
        fn rule_id(&self) -> &'static str {
            "panicking"
        }

        fn needs_content(&self) -> bool {
            false
        }

        fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
            panic!("PanickingRule::evaluate intentionally panicked");
        }
    }

    /// Records every input it sees so the order-of-evaluation tests
    /// can assert what came in. Decision is configurable.
    struct RecordingRule {
        id: &'static str,
        decision: RuleDecision,
        seen: Mutex<Vec<PathBuf>>,
    }

    impl RecordingRule {
        fn new(id: &'static str, decision: RuleDecision) -> Self {
            Self {
                id,
                decision,
                seen: Mutex::new(Vec::new()),
            }
        }

        fn paths(&self) -> Vec<PathBuf> {
            self.seen.lock().unwrap().clone()
        }
    }

    impl InterceptRule for RecordingRule {
        fn rule_id(&self) -> &'static str {
            self.id
        }

        fn needs_content(&self) -> bool {
            false
        }

        fn evaluate(&self, input: &RuleInput<'_>) -> RuleDecision {
            self.seen.lock().unwrap().push(input.path.to_path_buf());
            self.decision.clone()
        }
    }

    fn input(path: &Path) -> RuleInput<'_> {
        RuleInput {
            path,
            change_kind: ChangeKind::Modified,
            content: None,
        }
    }

    // ----- Behavioural tests. ----------------------------------------

    #[test]
    fn empty_registry_returns_allow() {
        let registry = RuleRegistry::new();
        let path = PathBuf::from("src/lib.rs");
        assert_eq!(registry.evaluate(&input(&path)), RegistryDecision::Allow);
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(!registry.any_needs_content());
        assert!(registry.rule_ids().is_empty());
    }

    #[test]
    fn all_allow_rules_returns_allow() {
        let mut registry = RuleRegistry::new();
        registry
            .register(Box::new(StubRule::new("a", RuleDecision::allow())))
            .expect("register a");
        registry
            .register(Box::new(StubRule::new("b", RuleDecision::allow())))
            .expect("register b");

        let path = PathBuf::from("src/lib.rs");
        assert_eq!(registry.evaluate(&input(&path)), RegistryDecision::Allow);
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn content_rules_are_skipped_when_content_is_unavailable() {
        let content_rule: &'static StubRule = Box::leak(Box::new(
            StubRule::new("content", RuleDecision::interrupt("content", "blocked"))
                .needing_content(),
        ));

        let mut registry = RuleRegistry::new();
        registry
            .register(Box::new(StubRefRule(content_rule)))
            .expect("register content rule");

        let path = PathBuf::from("src/lib.rs");

        assert_eq!(registry.evaluate(&input(&path)), RegistryDecision::Allow);
        assert_eq!(content_rule.calls(), 0);
    }

    #[test]
    fn enforce_mode_short_circuits_on_first_interrupt() {
        let first_allow = StubRule::new("first", RuleDecision::allow());
        let interrupt = StubRule::new(
            "second",
            RuleDecision::interrupt("second", "blocked by second"),
        );
        let third_allow = StubRule::new("third", RuleDecision::allow());

        // Hand the registry rules but keep references to the stubs so
        // we can inspect call counts after evaluation. Rust's ownership
        // means we use `Arc` or `Box::leak` for that — using
        // `Arc<StubRule>` impl InterceptRule below would require the
        // trait to be implementable on Arc, which it is not. Easiest:
        // `Box::leak` for the call-count probe. The leaked memory dies
        // with the test process.
        let first_ref: &'static StubRule = Box::leak(Box::new(first_allow));
        let interrupt_ref: &'static StubRule = Box::leak(Box::new(interrupt));
        let third_ref: &'static StubRule = Box::leak(Box::new(third_allow));

        let mut registry = RuleRegistry::new();
        registry
            .register(Box::new(StubRefRule(first_ref)))
            .expect("register first");
        registry
            .register(Box::new(StubRefRule(interrupt_ref)))
            .expect("register interrupt");
        registry
            .register(Box::new(StubRefRule(third_ref)))
            .expect("register third");

        let path = PathBuf::from("src/lib.rs");
        let decision = registry.evaluate(&input(&path));
        assert_eq!(
            decision,
            RegistryDecision::Interrupt(InterruptReason {
                rule_id: "second".into(),
                message: "blocked by second".into(),
                line: None,
            }),
        );

        assert_eq!(first_ref.calls(), 1);
        assert_eq!(interrupt_ref.calls(), 1);
        assert_eq!(
            third_ref.calls(),
            0,
            "rules after the first interrupt MUST NOT evaluate in enforce mode",
        );
    }

    /// `StubRule` doesn't implement `InterceptRule` for `&StubRule`
    /// directly (the trait's `'static` bound means we can't ship a
    /// `&` impl), so the short-circuit test wraps a leaked reference
    /// in this thin newtype. Tests-only.
    struct StubRefRule(&'static StubRule);
    impl InterceptRule for StubRefRule {
        fn rule_id(&self) -> &str {
            self.0.rule_id()
        }
        fn needs_content(&self) -> bool {
            self.0.needs_content()
        }
        fn evaluate(&self, input: &RuleInput<'_>) -> RuleDecision {
            self.0.evaluate(input)
        }
    }

    #[test]
    fn rules_evaluate_in_registration_order() {
        let recording_a = RecordingRule::new("a", RuleDecision::allow());
        let recording_b = RecordingRule::new("b", RuleDecision::allow());
        let a_ref: &'static RecordingRule = Box::leak(Box::new(recording_a));
        let b_ref: &'static RecordingRule = Box::leak(Box::new(recording_b));

        let mut registry = RuleRegistry::new();
        registry
            .register(Box::new(RecordingRefRule(a_ref)))
            .unwrap();
        registry
            .register(Box::new(RecordingRefRule(b_ref)))
            .unwrap();

        // Evaluate two distinct paths; we want each rule to see them
        // both and in the same order.
        for p in ["one.rs", "two.rs"] {
            let path = PathBuf::from(p);
            let _ = registry.evaluate(&input(&path));
        }

        assert_eq!(
            a_ref.paths(),
            vec![PathBuf::from("one.rs"), PathBuf::from("two.rs")]
        );
        assert_eq!(
            b_ref.paths(),
            vec![PathBuf::from("one.rs"), PathBuf::from("two.rs")]
        );
        assert_eq!(registry.rule_ids(), vec!["a", "b"]);
    }

    struct RecordingRefRule(&'static RecordingRule);
    impl InterceptRule for RecordingRefRule {
        fn rule_id(&self) -> &str {
            self.0.rule_id()
        }
        fn needs_content(&self) -> bool {
            self.0.needs_content()
        }
        fn evaluate(&self, input: &RuleInput<'_>) -> RuleDecision {
            self.0.evaluate(input)
        }
    }

    #[test]
    fn duplicate_rule_id_at_register_is_rejected() {
        let mut registry = RuleRegistry::new();
        registry
            .register(Box::new(StubRule::new("dup", RuleDecision::allow())))
            .expect("first wins");

        let err = registry
            .register(Box::new(StubRule::new("dup", RuleDecision::allow())))
            .expect_err("second must lose");
        assert_eq!(err, RegistryError::DuplicateRuleId("dup".into()));
        assert_eq!(registry.len(), 1, "the second insert must not be retained");
    }

    #[test]
    fn duplicate_rule_id_at_with_rules_is_rejected() {
        let rules: Vec<Box<dyn InterceptRule>> = vec![
            Box::new(StubRule::new("dup", RuleDecision::allow())),
            Box::new(StubRule::new("other", RuleDecision::allow())),
            Box::new(StubRule::new("dup", RuleDecision::allow())),
        ];
        let err = RuleRegistry::with_rules(rules).expect_err("must reject the duplicate");
        assert_eq!(err, RegistryError::DuplicateRuleId("dup".into()));
    }

    #[test]
    fn panicking_rule_is_isolated_and_evaluation_continues() {
        let after_panic =
            StubRule::new("after", RuleDecision::interrupt("after", "after still ran"));
        let after_ref: &'static StubRule = Box::leak(Box::new(after_panic));

        let mut registry = RuleRegistry::new();
        registry
            .register(Box::new(PanickingRule))
            .expect("register panicker");
        registry
            .register(Box::new(StubRefRule(after_ref)))
            .expect("register after");

        let path = PathBuf::from("src/lib.rs");
        let decision = registry.evaluate(&input(&path));

        // The panic was caught and the rule treated as Allow, so the
        // next rule got its chance — and it short-circuited as
        // expected. Without isolation the panic would have escaped
        // the call entirely.
        assert_eq!(
            decision,
            RegistryDecision::Interrupt(InterruptReason {
                rule_id: "after".into(),
                message: "after still ran".into(),
                line: None,
            }),
        );
        assert_eq!(after_ref.calls(), 1);
    }

    #[test]
    fn panicking_rule_with_no_followup_returns_allow() {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(PanickingRule)).unwrap();

        let path = PathBuf::from("src/lib.rs");
        assert_eq!(registry.evaluate(&input(&path)), RegistryDecision::Allow);
    }

    #[test]
    fn observe_only_mode_logs_but_returns_allow() {
        let interrupt = StubRule::new(
            "would-interrupt",
            RuleDecision::interrupt("would-interrupt", "shadow only"),
        );
        let next = StubRule::new("next", RuleDecision::allow());
        let interrupt_ref: &'static StubRule = Box::leak(Box::new(interrupt));
        let next_ref: &'static StubRule = Box::leak(Box::new(next));

        let mut registry = RuleRegistry::observe_only();
        registry
            .register(Box::new(StubRefRule(interrupt_ref)))
            .unwrap();
        registry.register(Box::new(StubRefRule(next_ref))).unwrap();

        assert_eq!(registry.mode(), RegistryMode::ObserveOnly);

        let path = PathBuf::from("src/lib.rs");
        let decision = registry.evaluate(&input(&path));

        assert_eq!(
            decision,
            RegistryDecision::Allow,
            "observe-only must downgrade an Interrupt to Allow",
        );
        assert_eq!(interrupt_ref.calls(), 1);
        assert_eq!(
            next_ref.calls(),
            1,
            "observe-only must keep evaluating after a would-be interrupt",
        );
    }

    #[test]
    fn into_observe_only_flips_mode_after_construction() {
        let registry = RuleRegistry::new();
        assert_eq!(registry.mode(), RegistryMode::Enforce);
        let registry = registry.into_observe_only();
        assert_eq!(registry.mode(), RegistryMode::ObserveOnly);
    }

    #[test]
    fn any_needs_content_reflects_registered_rules() {
        let mut registry = RuleRegistry::new();
        assert!(!registry.any_needs_content());
        registry
            .register(Box::new(StubRule::new("a", RuleDecision::allow())))
            .unwrap();
        assert!(
            !registry.any_needs_content(),
            "a content-free rule must not flip the flag",
        );
        registry
            .register(Box::new(
                StubRule::new("b", RuleDecision::allow()).needing_content(),
            ))
            .unwrap();
        assert!(
            registry.any_needs_content(),
            "registering a content-bearing rule must surface the flag",
        );
    }

    #[test]
    fn with_rules_preserves_input_order() {
        let rules: Vec<Box<dyn InterceptRule>> = vec![
            Box::new(StubRule::new("first", RuleDecision::allow())),
            Box::new(StubRule::new("second", RuleDecision::allow())),
            Box::new(StubRule::new("third", RuleDecision::allow())),
        ];
        let registry = RuleRegistry::with_rules(rules).expect("no dups");
        assert_eq!(registry.rule_ids(), vec!["first", "second", "third"]);
    }

    /// A rule that emits an `InterruptReason` with a mismatched
    /// `rule_id` (against its own registered id) gets its id
    /// overwritten with the cached value — observability and dedup
    /// invariants don't trust the rule to get its own id right.
    #[test]
    fn interrupt_reason_rule_id_normalised_to_registered_id() {
        struct LiarRule;
        impl InterceptRule for LiarRule {
            fn rule_id(&self) -> &'static str {
                "liar"
            }
            fn needs_content(&self) -> bool {
                false
            }
            fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
                // Lie about which rule fired — the registry must
                // overwrite this with the cached "liar" id.
                RuleDecision::interrupt("imposter", "i am not who i claim to be")
            }
        }

        let mut registry = RuleRegistry::new();
        registry.register(Box::new(LiarRule)).unwrap();
        let path = PathBuf::from("src/lib.rs");
        let decision = registry.evaluate(&input(&path));
        match decision {
            RegistryDecision::Interrupt(reason) => {
                assert_eq!(
                    reason.rule_id, "liar",
                    "registry must normalise to the cached id, not trust the rule",
                );
                assert_eq!(reason.message, "i am not who i claim to be");
            }
            RegistryDecision::Allow => panic!("expected Interrupt, got Allow"),
        }
    }

    /// `rule_ids()` returns the cached ids — even after a rule's
    /// `rule_id()` would (in principle) drift. We can't easily inject
    /// drift on a `Box<dyn InterceptRule>` without unsafe, but the
    /// API contract is "registered ids, not live `rule_id()` calls",
    /// so this test documents the surface guarantee.
    #[test]
    fn rule_ids_returns_cached_values_in_registration_order() {
        let registry = RuleRegistry::with_rules(vec![
            Box::new(StubRule::new("alpha", RuleDecision::allow())),
            Box::new(StubRule::new("beta", RuleDecision::allow())),
            Box::new(StubRule::new("gamma", RuleDecision::allow())),
        ])
        .expect("no dups");
        assert_eq!(registry.rule_ids(), vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn deduplicate_ids_helper_partitions_input() {
        let (kept, duplicates) = deduplicate_ids([
            "a".to_owned(),
            "b".to_owned(),
            "a".to_owned(),
            "c".to_owned(),
            "b".to_owned(),
        ]);
        assert_eq!(kept, vec!["a", "b", "c"]);
        assert_eq!(duplicates, vec!["a", "b"]);
    }
}
