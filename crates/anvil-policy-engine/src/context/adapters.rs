//! Deterministic context adapters and assertion evaluation (CPOL-002).
//!
//! An [`AssertionContext`] is the payload an [`Assertion`] evaluates against:
//! the changed paths (with their [change kinds](ChangeKind)), the current
//! [workflow phase](WorkflowPhase), and a bag of config values.
//!
//! ## Determinism contract (ADR-040 D-2)
//!
//! Adapters **transform data they are handed** — they never read the clock, the
//! filesystem, or the environment at build time. [`AssertionContext::from_parts`]
//! and [`AssertionContext::from_policy_input`] take every fact as an argument, so
//! the same inputs always yield the same payload. Ordering is normalised
//! (changed paths sorted and de-duplicated, config keyed by a [`BTreeMap`]) so
//! two contexts built from the same facts compare equal and evaluation is
//! order-independent.

use std::collections::BTreeMap;

use globset::{GlobBuilder, GlobMatcher};

use crate::PolicyInput;
use crate::context::assertion::{
    Assertion, AssertionCondition, AssertionError, AssertionScope, ChangeKind, WorkflowPhase,
};

/// A single changed path and the kind of change it underwent.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChangedPath {
    /// Repo-relative path (matching [`PolicyInput`]'s convention).
    pub path: String,
    /// How the path changed.
    pub kind: ChangeKind,
}

impl ChangedPath {
    /// Construct a changed path.
    pub fn new(path: impl Into<String>, kind: ChangeKind) -> Self {
        Self {
            path: path.into(),
            kind,
        }
    }
}

/// The deterministic payload an [`Assertion`] evaluates against.
///
/// Built only from data handed to the constructors — never from ambient state
/// (ADR-040 D-2). `changed_paths` is sorted and de-duplicated; `config` is a
/// [`BTreeMap`] so key order is normalised.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssertionContext {
    /// The workflow phase this evaluation runs in.
    pub phase: WorkflowPhase,
    /// The change set, sorted by `(path, kind)` and de-duplicated.
    pub changed_paths: Vec<ChangedPath>,
    /// Config values, keyed for deterministic lookup and ordering.
    pub config: BTreeMap<String, String>,
}

impl AssertionContext {
    /// Build a context from explicitly-supplied facts.
    ///
    /// `changed_paths` and `config` may arrive in any order and may contain
    /// duplicates; the result normalises both (sort + de-duplicate paths;
    /// duplicate config keys resolve to the lexicographically greatest value)
    /// so it is deterministic regardless of input order. Purely a transform of
    /// its arguments — no I/O.
    pub fn from_parts(
        phase: WorkflowPhase,
        changed_paths: impl IntoIterator<Item = ChangedPath>,
        config: impl IntoIterator<Item = (String, String)>,
    ) -> Self {
        let mut changed_paths: Vec<ChangedPath> = changed_paths.into_iter().collect();
        changed_paths.sort();
        changed_paths.dedup();
        // Sort pairs before inserting so duplicate keys resolve to the
        // lexicographically greatest value regardless of input order — a
        // plain `collect` lets the caller's iteration order pick the
        // winner, breaking the order-independence contract.
        let mut config_pairs: Vec<(String, String)> = config.into_iter().collect();
        config_pairs.sort();
        let config: BTreeMap<String, String> = config_pairs.into_iter().collect();
        Self {
            phase,
            changed_paths,
            config,
        }
    }

    /// Build a context from a [`PolicyInput`] plus the ambient facts the input
    /// does not carry (the workflow phase and config).
    ///
    /// [`PolicyInput`]'s diff records *which* files changed but not *how*, so
    /// every changed file is adapted as [`ChangeKind::Modified`] — the neutral
    /// kind. Callers with richer change-kind information should use
    /// [`from_parts`](Self::from_parts) instead. This reads only the passed-in
    /// `input` (no I/O), so it is deterministic.
    pub fn from_policy_input(
        input: &PolicyInput,
        phase: WorkflowPhase,
        config: impl IntoIterator<Item = (String, String)>,
    ) -> Self {
        let changed = input
            .diff
            .changed_files
            .iter()
            .map(|path| ChangedPath::new(path.clone(), ChangeKind::Modified));
        Self::from_parts(phase, changed, config)
    }

    /// The count of changed paths, optionally restricted to one change kind.
    fn changed_path_count(&self, kind: Option<ChangeKind>) -> u64 {
        self.changed_paths
            .iter()
            .filter(|c| kind.is_none_or(|k| c.kind == k))
            .count() as u64
    }
}

/// The result of evaluating an [`Assertion`] against an [`AssertionContext`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssertionEvaluation {
    /// The assertion's scope excludes this context; it was not evaluated.
    OutOfScope,
    /// Every condition held.
    Satisfied,
    /// A condition was not met; the assertion is violated.
    Violated(Violation),
}

/// Why an assertion was violated: the first unmet condition and its context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Index of the first unmet condition in [`Assertion::conditions`].
    pub condition_index: usize,
    /// The unmet condition (cloned for downstream guidance).
    pub condition: AssertionCondition,
    /// A human-readable detail of what failed. UK spelling.
    pub detail: String,
    /// The offending changed path, when the condition names one.
    pub offending_path: Option<String>,
}

/// Evaluate an assertion against a context.
///
/// Validates `assertion` first ([`Assertion::validate`]): a malformed assertion
/// — a blank glob, an uncompilable glob, empty conditions — is a fail-closed
/// [`AssertionError`], not a weird runtime non-match. Otherwise returns
/// [`AssertionEvaluation::OutOfScope`] when the assertion's scope does not cover
/// this context (see [`in_scope`]), or evaluates every condition in declared
/// order and returns [`AssertionEvaluation::Satisfied`] or the first
/// [`Violation`]. Pure over its two arguments.
pub fn evaluate(
    assertion: &Assertion,
    context: &AssertionContext,
) -> Result<AssertionEvaluation, AssertionError> {
    // Fail-closed boundary: validation compiles every glob, so the matching
    // below operates only on known-good, linear-time patterns.
    assertion.validate()?;

    if !in_scope(&assertion.scope, context) {
        return Ok(AssertionEvaluation::OutOfScope);
    }

    for (index, condition) in assertion.conditions.iter().enumerate() {
        if let Err(failure) = check_condition(condition, context) {
            return Ok(AssertionEvaluation::Violated(Violation {
                condition_index: index,
                condition: condition.clone(),
                detail: failure.detail,
                offending_path: failure.offending_path,
            }));
        }
    }

    Ok(AssertionEvaluation::Satisfied)
}

/// Whether a scope covers a context.
///
/// A scope covers the context when its phase axis is unrestricted (empty) or
/// includes the context phase, **and** its path axis is unrestricted (empty) or
/// at least one changed path matches one of the scope globs. An empty change set
/// against a path-restricted scope is out of scope: there is nothing the
/// assertion applies to.
///
/// Assumes the scope globs have been validated (compiled) — the normal path via
/// [`evaluate`]. A glob that fails to compile here is conservatively treated as
/// matching nothing.
#[must_use]
pub fn in_scope(scope: &AssertionScope, context: &AssertionContext) -> bool {
    let phase_ok = scope.phases.is_empty() || scope.phases.contains(&context.phase);
    if !phase_ok {
        return false;
    }
    if scope.paths.is_empty() {
        return true;
    }
    let matchers: Vec<GlobMatcher> = scope
        .paths
        .iter()
        .filter_map(|glob| compile_glob(glob).ok())
        .collect();
    context
        .changed_paths
        .iter()
        .any(|changed| matchers.iter().any(|m| m.is_match(&changed.path)))
}

/// A condition check failure: what failed and (optionally) the offending path.
struct ConditionFailure {
    detail: String,
    offending_path: Option<String>,
}

/// Check a single condition against the context, returning the failure detail
/// when it does not hold. Glob-bearing conditions compile their pattern once;
/// callers reach this only after [`evaluate`] validated compilability.
fn check_condition(
    condition: &AssertionCondition,
    context: &AssertionContext,
) -> Result<(), ConditionFailure> {
    match condition {
        AssertionCondition::ChangedPathsConfinedTo(spec) => {
            let matcher = compile_glob(&spec.glob).ok();
            // An uncompilable glob (unreachable post-validation) confines
            // nothing, so every path is treated as escaping — fail closed.
            match context
                .changed_paths
                .iter()
                .find(|c| !matcher.as_ref().is_some_and(|m| m.is_match(&c.path)))
            {
                None => Ok(()),
                Some(escaping) => Err(ConditionFailure {
                    detail: format!(
                        "changed path `{}` is outside the permitted area `{}`",
                        escaping.path, spec.glob
                    ),
                    offending_path: Some(escaping.path.clone()),
                }),
            }
        }
        AssertionCondition::ChangedPathsExclude(spec) => {
            let matcher = compile_glob(&spec.glob).ok();
            match context
                .changed_paths
                .iter()
                .find(|c| matcher.as_ref().is_some_and(|m| m.is_match(&c.path)))
            {
                None => Ok(()),
                Some(hit) => Err(ConditionFailure {
                    detail: format!(
                        "changed path `{}` matches the protected pattern `{}`",
                        hit.path, spec.glob
                    ),
                    offending_path: Some(hit.path.clone()),
                }),
            }
        }
        AssertionCondition::ChangedPathCount(spec) => {
            let actual = context.changed_path_count(spec.change_kind);
            if spec.op.holds(actual, spec.value) {
                Ok(())
            } else {
                let scope = match spec.change_kind {
                    Some(kind) => format!("{kind:?} changed paths"),
                    None => "changed paths".to_string(),
                };
                Err(ConditionFailure {
                    detail: format!(
                        "{scope}: count {actual} does not satisfy {:?} {}",
                        spec.op, spec.value
                    ),
                    offending_path: None,
                })
            }
        }
        AssertionCondition::ConfigEquals(spec) => match context.config.get(&spec.key) {
            Some(actual) if actual == &spec.value => Ok(()),
            Some(actual) => Err(ConditionFailure {
                detail: format!(
                    "config `{}` is `{actual}`, expected `{}`",
                    spec.key, spec.value
                ),
                offending_path: None,
            }),
            None => Err(ConditionFailure {
                detail: format!(
                    "config `{}` is not set, expected `{}`",
                    spec.key, spec.value
                ),
                offending_path: None,
            }),
        },
        AssertionCondition::ConfigPresent(spec) => match context.config.get(&spec.key) {
            Some(value) if !value.trim().is_empty() => Ok(()),
            _ => Err(ConditionFailure {
                detail: format!("config `{}` is not set", spec.key),
                offending_path: None,
            }),
        },
    }
}

/// Compile a repo-relative path glob into a matcher.
///
/// Uses the single workspace glob dialect: `globset` with
/// `literal_separator(true)`, so `*` and `?` do not cross `/` and only `**`
/// spans directories — mirroring `anvil-kernel`'s watch pattern filter and the
/// other repo-relative matchers (`anvil-intercept-rules`, `anvil-l4`).
/// Linear-time matching (no catastrophic backtracking). Returns the
/// `globset::Error` on an invalid pattern; callers surface it as a fail-closed
/// [`AssertionError`] at the validation boundary rather than as a silent
/// non-match.
pub(crate) fn compile_glob(pattern: &str) -> Result<GlobMatcher, globset::Error> {
    GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .map(|glob| glob.compile_matcher())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::assertion::{
        AssertionScope, ChangedPathCountSpec, Comparison, ConfigKey, ConfigMatch, PathGlob,
    };
    use crate::input::Diff;
    use crate::pack::PolicySeverity;

    fn assertion_with(scope: AssertionScope, conditions: Vec<AssertionCondition>) -> Assertion {
        Assertion {
            id: "a".into(),
            title: "t".into(),
            scope,
            conditions,
            outcome: PolicySeverity::High,
            rationale: "r".into(),
            remediation: "fix".into(),
        }
    }

    /// Evaluate a well-formed assertion, unwrapping the validation boundary.
    fn eval(assertion: &Assertion, context: &AssertionContext) -> AssertionEvaluation {
        evaluate(assertion, context).expect("assertion_with builds valid assertions")
    }

    #[test]
    fn assertion_context_from_parts_sorts_and_dedups_deterministically() {
        let a = AssertionContext::from_parts(
            WorkflowPhase::Commit,
            [
                ChangedPath::new("src/b.rs", ChangeKind::Modified),
                ChangedPath::new("src/a.rs", ChangeKind::Added),
                ChangedPath::new("src/b.rs", ChangeKind::Modified),
            ],
            [("k".to_string(), "v".to_string())],
        );
        let b = AssertionContext::from_parts(
            WorkflowPhase::Commit,
            [
                ChangedPath::new("src/a.rs", ChangeKind::Added),
                ChangedPath::new("src/b.rs", ChangeKind::Modified),
            ],
            [("k".to_string(), "v".to_string())],
        );
        // Duplicate collapsed, order normalised: two facts sets compare equal.
        assert_eq!(a, b);
        assert_eq!(a.changed_paths.len(), 2);
        assert_eq!(a.changed_paths[0].path, "src/a.rs");
    }

    #[test]
    fn assertion_context_from_policy_input_maps_changed_files_to_modified() {
        let input = PolicyInput {
            diff: Diff {
                changed_files: vec!["src/z.rs".into(), "src/a.rs".into()],
                new_edges: vec![],
            },
            ..Default::default()
        };
        let ctx = AssertionContext::from_policy_input(&input, WorkflowPhase::Save, []);
        assert_eq!(ctx.changed_paths.len(), 2);
        // Sorted deterministically regardless of input order.
        assert_eq!(ctx.changed_paths[0].path, "src/a.rs");
        assert!(
            ctx.changed_paths
                .iter()
                .all(|c| c.kind == ChangeKind::Modified)
        );
    }

    #[test]
    fn assertion_context_default_phase_is_manual() {
        assert_eq!(AssertionContext::default().phase, WorkflowPhase::Manual);
    }

    #[test]
    fn assertion_context_out_of_scope_by_phase() {
        let assertion = assertion_with(
            AssertionScope {
                paths: vec![],
                phases: vec![WorkflowPhase::Push],
            },
            vec![AssertionCondition::ConfigPresent(ConfigKey {
                key: "k".into(),
            })],
        );
        let ctx = AssertionContext::from_parts(WorkflowPhase::Save, [], []);
        assert_eq!(eval(&assertion, &ctx), AssertionEvaluation::OutOfScope);
    }

    #[test]
    fn assertion_context_out_of_scope_by_path() {
        let assertion = assertion_with(
            AssertionScope {
                paths: vec!["docs/**".into()],
                phases: vec![],
            },
            vec![AssertionCondition::ConfigPresent(ConfigKey {
                key: "k".into(),
            })],
        );
        let ctx = AssertionContext::from_parts(
            WorkflowPhase::Commit,
            [ChangedPath::new("src/a.rs", ChangeKind::Modified)],
            [],
        );
        assert_eq!(eval(&assertion, &ctx), AssertionEvaluation::OutOfScope);
    }

    #[test]
    fn assertion_context_in_scope_when_axes_unrestricted() {
        let assertion = assertion_with(
            AssertionScope::default(),
            vec![AssertionCondition::ConfigPresent(ConfigKey {
                key: "owner".into(),
            })],
        );
        let ctx = AssertionContext::from_parts(
            WorkflowPhase::Manual,
            [],
            [("owner".to_string(), "team".to_string())],
        );
        assert_eq!(eval(&assertion, &ctx), AssertionEvaluation::Satisfied);
    }

    #[test]
    fn assertion_context_violation_reports_index_and_offending_path() {
        let assertion = assertion_with(
            AssertionScope::default(),
            vec![
                AssertionCondition::ConfigPresent(ConfigKey {
                    key: "owner".into(),
                }),
                AssertionCondition::ChangedPathsExclude(PathGlob {
                    glob: "**/Cargo.lock".into(),
                }),
            ],
        );
        let ctx = AssertionContext::from_parts(
            WorkflowPhase::Commit,
            [
                ChangedPath::new("crates/x/Cargo.lock", ChangeKind::Modified),
                ChangedPath::new("crates/x/src/lib.rs", ChangeKind::Modified),
            ],
            [("owner".to_string(), "team".to_string())],
        );
        let AssertionEvaluation::Violated(v) = eval(&assertion, &ctx) else {
            panic!("expected a violation");
        };
        assert_eq!(v.condition_index, 1);
        assert_eq!(v.offending_path.as_deref(), Some("crates/x/Cargo.lock"));
    }

    #[test]
    fn assertion_context_confined_to_reports_escaping_path() {
        let assertion = assertion_with(
            AssertionScope::default(),
            vec![AssertionCondition::ChangedPathsConfinedTo(PathGlob {
                glob: "crates/**".into(),
            })],
        );
        let ctx = AssertionContext::from_parts(
            WorkflowPhase::Commit,
            [
                ChangedPath::new("crates/x/src/lib.rs", ChangeKind::Modified),
                ChangedPath::new("scripts/deploy.sh", ChangeKind::Added),
            ],
            [],
        );
        let AssertionEvaluation::Violated(v) = eval(&assertion, &ctx) else {
            panic!("expected a violation");
        };
        assert_eq!(v.condition_index, 0);
        assert_eq!(v.offending_path.as_deref(), Some("scripts/deploy.sh"));
    }

    #[test]
    fn assertion_context_threshold_by_change_kind() {
        let assertion = assertion_with(
            AssertionScope::default(),
            vec![AssertionCondition::ChangedPathCount(ChangedPathCountSpec {
                op: Comparison::AtMost,
                value: 1,
                change_kind: Some(ChangeKind::Added),
            })],
        );
        let ctx = AssertionContext::from_parts(
            WorkflowPhase::Commit,
            [
                ChangedPath::new("a.rs", ChangeKind::Added),
                ChangedPath::new("b.rs", ChangeKind::Added),
                ChangedPath::new("c.rs", ChangeKind::Modified),
            ],
            [],
        );
        // Two Added paths exceed the at-most-1 bound; Modified is not counted.
        let AssertionEvaluation::Violated(v) = eval(&assertion, &ctx) else {
            panic!("expected a violation");
        };
        assert_eq!(v.condition_index, 0);
        assert!(v.offending_path.is_none());
        assert!(v.detail.contains("count 2"), "detail: {}", v.detail);
    }

    #[test]
    fn assertion_context_config_equals_mismatch_and_missing() {
        let assertion = assertion_with(
            AssertionScope::default(),
            vec![AssertionCondition::ConfigEquals(ConfigMatch {
                key: "signed".into(),
                value: "true".into(),
            })],
        );

        let mismatch = AssertionContext::from_parts(
            WorkflowPhase::Commit,
            [ChangedPath::new("a.rs", ChangeKind::Modified)],
            [("signed".to_string(), "false".to_string())],
        );
        assert!(matches!(
            eval(&assertion, &mismatch),
            AssertionEvaluation::Violated(_)
        ));

        let missing = AssertionContext::from_parts(
            WorkflowPhase::Commit,
            [ChangedPath::new("a.rs", ChangeKind::Modified)],
            [],
        );
        assert!(matches!(
            eval(&assertion, &missing),
            AssertionEvaluation::Violated(_)
        ));
    }

    #[test]
    fn assertion_context_duplicate_config_keys_resolve_order_independently() {
        let a = AssertionContext::from_parts(
            WorkflowPhase::Save,
            std::iter::empty(),
            [
                ("k".to_string(), "first".to_string()),
                ("k".to_string(), "second".to_string()),
            ],
        );
        let b = AssertionContext::from_parts(
            WorkflowPhase::Save,
            std::iter::empty(),
            [
                ("k".to_string(), "second".to_string()),
                ("k".to_string(), "first".to_string()),
            ],
        );
        assert_eq!(
            a, b,
            "duplicate-key resolution must not depend on input order"
        );
        assert_eq!(a.config.get("k").map(String::as_str), Some("second"));
    }

    #[test]
    fn assertion_context_evaluation_is_order_independent() {
        // Same facts supplied in a different order yield the same evaluation.
        let assertion = assertion_with(
            AssertionScope::default(),
            vec![AssertionCondition::ChangedPathsConfinedTo(PathGlob {
                glob: "src/**".into(),
            })],
        );
        let forward = AssertionContext::from_parts(
            WorkflowPhase::Commit,
            [
                ChangedPath::new("src/a.rs", ChangeKind::Modified),
                ChangedPath::new("src/b.rs", ChangeKind::Added),
            ],
            [],
        );
        let reversed = AssertionContext::from_parts(
            WorkflowPhase::Commit,
            [
                ChangedPath::new("src/b.rs", ChangeKind::Added),
                ChangedPath::new("src/a.rs", ChangeKind::Modified),
            ],
            [],
        );
        assert_eq!(eval(&assertion, &forward), eval(&assertion, &reversed));
        assert_eq!(eval(&assertion, &forward), AssertionEvaluation::Satisfied);
    }

    /// Match helper over the compiled workspace glob dialect.
    fn matches(glob: &str, path: &str) -> bool {
        compile_glob(glob).expect("valid glob").is_match(path)
    }

    #[test]
    fn assertion_context_glob_matches_star_semantics() {
        // `*` and `?` do not cross `/`; only `**` spans directories
        // (globset literal_separator(true) — the workspace dialect).
        assert!(matches("src/*.rs", "src/a.rs"));
        assert!(!matches("src/*.rs", "src/sub/a.rs"));
        assert!(matches("src/**", "src/sub/a.rs"));
        assert!(matches("**/*.rs", "a.rs"));
        assert!(matches("**/*.rs", "src/deep/a.rs"));
        assert!(matches("**/Cargo.lock", "crates/x/Cargo.lock"));
        assert!(matches("Cargo.lock", "Cargo.lock"));
        assert!(!matches("Cargo.lock", "Cargo.toml"));
        assert!(matches("src/?.rs", "src/a.rs"));
        assert!(!matches("src/?.rs", "src/ab.rs"));
    }

    #[test]
    fn assertion_context_invalid_glob_is_validation_error_not_silent_nonmatch() {
        // An unclosed character class is an invalid glob: evaluation must
        // fail closed with InvalidGlob, not silently treat it as no-match.
        let assertion = assertion_with(
            AssertionScope::default(),
            vec![AssertionCondition::ChangedPathsExclude(PathGlob {
                glob: "src/[unclosed".into(),
            })],
        );
        let ctx = AssertionContext::from_parts(
            WorkflowPhase::Commit,
            [ChangedPath::new("src/a.rs", ChangeKind::Modified)],
            [],
        );
        assert!(matches!(
            evaluate(&assertion, &ctx),
            Err(AssertionError::InvalidGlob { .. })
        ));
    }

    #[test]
    fn assertion_context_pathological_glob_is_linear_not_redos() {
        // The hand-rolled recursive matcher was exponential: `("a*"*n)+"b"`
        // against `"a"*n` hung near n=40. globset is linear — this completes
        // instantly (the test finishing at all is the assertion).
        let n = 40;
        let pattern = "a*".repeat(n) + "b";
        let text = "a".repeat(n);
        let matcher = compile_glob(&pattern).expect("valid glob");
        assert!(!matcher.is_match(&text));
    }
}
