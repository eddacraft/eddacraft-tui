//! Save-time / pre-write policy input adapter (OPAE-006).
//!
//! A [`PrewriteInput`] is the single, deterministic assembly of the facts a
//! save-time / pre-write policy evaluation needs: the changed paths (with their
//! [change kinds](crate::context::assertion::ChangeKind)), the
//! [workflow phase](crate::context::WorkflowPhase), a bag of config values, and
//! optional [graph facts](GraphFacts). It exists so the two policy surfaces that
//! run at the pre-write boundary — Rego packs (which consume a
//! [`PolicyInput`]) and CPOL assertions (which consume an [`AssertionContext`])
//! — evaluate the **same** change set without either side recomputing it.
//!
//! ## Single source, two projections
//!
//! [`PrewriteInput::to_policy_input`] and [`PrewriteInput::to_assertion_context`]
//! are pure projections of the one normalised fact set, so the two surfaces are
//! guaranteed to see identical changed paths. Build once, project twice.
//!
//! ## Determinism contract (ADR-040 D-2)
//!
//! Like the CPOL adapters, this builder **transforms facts it is handed** — it
//! never reads the clock, the filesystem, or the environment.
//! [`PrewriteInput::from_parts`] normalises ordering (changed paths sorted and
//! de-duplicated, config keyed by a [`BTreeMap`]) so equal facts always yield an
//! equal input regardless of the order they arrive in.
//!
//! ## Pre-write boundary (ADR-098 AD-4)
//!
//! Per ADR-098 AD-4 the pre-write boundary is existing off-daemon surfaces only
//! (MCP `anvil_validate_write`, `anvil gate`, CI); no policy family joins the
//! daemon's `validate_paths` hot path, and `regorus` / `anvil-policy*` stay
//! forbidden on the resident daemon. This adapter builds the input those
//! off-daemon surfaces evaluate; it does not itself evaluate anything.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::EngineConfig;
use crate::context::{AssertionContext, ChangedPath, WorkflowPhase};
use crate::input::{Diff, PolicyInput, RepoState};

/// A tight default pre-write eval budget.
///
/// Deliberately far below the 10 s CLI facade ceiling: pre-write evaluation sits
/// on an interactive save path, so it must be fail-open and quick (ADR-098 AD-5).
const DEFAULT_PREWRITE_BUDGET: Duration = Duration::from_millis(250);

/// Optional graph-derived facts about the change set.
///
/// A minimal, plain data contract the daemon or CLI can populate later (e.g.
/// from the dependency graph). Every field defaults, and the shape is
/// **additive-friendly** — deserialisation ignores unknown fields (no
/// `deny_unknown_fields`) so a newer producer carrying extra facts does not
/// break an older consumer. This adapter carries the facts; it does not compute
/// them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphFacts {
    /// Boundary / module names touching each changed path, keyed by path.
    #[serde(default)]
    pub boundaries: BTreeMap<String, Vec<String>>,
    /// Number of dependents on each changed path, keyed by path.
    #[serde(default)]
    pub dependents: BTreeMap<String, u64>,
}

impl GraphFacts {
    /// Every path named anywhere in the graph facts, in sorted key order.
    fn paths(&self) -> impl Iterator<Item = &String> {
        self.boundaries.keys().chain(self.dependents.keys())
    }
}

/// The pre-write evaluation budget.
///
/// Carries the maximum wall-clock duration a single pre-write policy evaluation
/// may take, projected onto [`EngineConfig::eval_timeout`] by
/// [`engine_config`](Self::engine_config).
///
/// **Fail-open contract (ADR-098 AD-5):** the pre-write budget is tight and
/// fail-open — when a policy evaluation exceeds it, evaluation degrades to
/// `warn` + log and **never** blocks the write. This type only *carries* the
/// budget; the degrade-to-warn behaviour is implemented by the enforcement
/// layer, not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrewriteBudget {
    /// Maximum wall-clock duration a single pre-write eval may take.
    pub max_eval: Duration,
}

impl Default for PrewriteBudget {
    fn default() -> Self {
        Self {
            max_eval: DEFAULT_PREWRITE_BUDGET,
        }
    }
}

impl PrewriteBudget {
    /// A budget with the given maximum eval duration.
    #[must_use]
    pub fn new(max_eval: Duration) -> Self {
        Self { max_eval }
    }

    /// Project the budget onto an [`EngineConfig`], carrying `max_eval` through
    /// as the engine's [`eval_timeout`](EngineConfig::eval_timeout). Other
    /// engine knobs stay at their defaults.
    #[must_use]
    pub fn engine_config(&self) -> EngineConfig {
        EngineConfig {
            eval_timeout: Some(self.max_eval),
            ..Default::default()
        }
    }
}

/// The complete, deterministic pre-write policy evaluation input.
///
/// Built once from handed-in facts via [`from_parts`](Self::from_parts), then
/// projected into the two evaluation surfaces via
/// [`to_policy_input`](Self::to_policy_input) and
/// [`to_assertion_context`](Self::to_assertion_context). See the
/// [module docs](self) for the determinism and boundary contracts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrewriteInput {
    /// The workflow phase this evaluation runs in.
    pub phase: WorkflowPhase,
    /// The change set, sorted by `(path, kind)` and de-duplicated.
    pub changed_paths: Vec<ChangedPath>,
    /// Config values, keyed for deterministic lookup and ordering.
    pub config: BTreeMap<String, String>,
    /// Optional graph-derived facts about the change set.
    pub graph: GraphFacts,
    /// The pre-write eval budget (fail-open per ADR-098 AD-5).
    pub budget: PrewriteBudget,
}

impl PrewriteInput {
    /// Assemble a pre-write input from explicitly-supplied facts.
    ///
    /// `changed_paths` and `config` may arrive in any order and may contain
    /// duplicates; the result normalises both (sort + de-duplicate paths;
    /// duplicate config keys resolve to the lexicographically greatest value)
    /// so it is deterministic regardless of input order — matching
    /// [`AssertionContext::from_parts`]. Purely a transform of its arguments —
    /// no I/O (ADR-040 D-2).
    #[must_use]
    pub fn from_parts(
        phase: WorkflowPhase,
        changed_paths: impl IntoIterator<Item = ChangedPath>,
        config: impl IntoIterator<Item = (String, String)>,
        graph: GraphFacts,
        budget: PrewriteBudget,
    ) -> Self {
        let mut changed_paths: Vec<ChangedPath> = changed_paths.into_iter().collect();
        changed_paths.sort();
        changed_paths.dedup();
        // Sort pairs before collecting so a duplicate key resolves to the
        // lexicographically greatest value regardless of input order (a plain
        // collect lets iteration order pick the winner).
        let mut config_pairs: Vec<(String, String)> = config.into_iter().collect();
        config_pairs.sort();
        let config: BTreeMap<String, String> = config_pairs.into_iter().collect();
        Self {
            phase,
            changed_paths,
            config,
            graph,
            budget,
        }
    }

    /// Project onto the facade's [`PolicyInput`] for Rego pack evaluation.
    ///
    /// The change set becomes `diff.changed_files` (distinct paths, sorted);
    /// `repo_state.files` is the union of the changed paths and any path named
    /// in the [graph facts](GraphFacts). Reads only `self` — deterministic and
    /// recomputation-free.
    #[must_use]
    pub fn to_policy_input(&self) -> PolicyInput {
        // Paths are already sorted by `(path, kind)`, so equal path strings are
        // adjacent and `dedup` collapses a path that changed under two kinds
        // into a single entry.
        let mut changed_files: Vec<String> =
            self.changed_paths.iter().map(|c| c.path.clone()).collect();
        changed_files.dedup();

        let mut files: Vec<String> = changed_files.clone();
        files.extend(self.graph.paths().cloned());
        files.sort();
        files.dedup();

        PolicyInput {
            repo_state: RepoState {
                files,
                edges: Vec::new(),
            },
            diff: Diff {
                changed_files,
                new_edges: Vec::new(),
            },
            ..Default::default()
        }
    }

    /// Project onto an [`AssertionContext`] for CPOL assertion evaluation.
    ///
    /// Preserves the per-path change kinds (which [`PolicyInput`] cannot carry)
    /// and the config bag. Reads only `self` — deterministic and
    /// recomputation-free.
    #[must_use]
    pub fn to_assertion_context(&self) -> AssertionContext {
        AssertionContext::from_parts(
            self.phase,
            self.changed_paths.iter().cloned(),
            self.config.iter().map(|(k, v)| (k.clone(), v.clone())),
        )
    }

    /// The [`EngineConfig`] carrying this input's fail-open eval budget.
    #[must_use]
    pub fn engine_config(&self) -> EngineConfig {
        self.budget.engine_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::assertion::ChangeKind;

    fn changed(path: &str, kind: ChangeKind) -> ChangedPath {
        ChangedPath::new(path, kind)
    }

    fn sample() -> PrewriteInput {
        PrewriteInput::from_parts(
            WorkflowPhase::Save,
            [
                changed("src/b.rs", ChangeKind::Modified),
                changed("src/a.rs", ChangeKind::Added),
            ],
            [("signed".to_string(), "true".to_string())],
            GraphFacts::default(),
            PrewriteBudget::default(),
        )
    }

    #[test]
    fn policy_prewrite_input_normalises_paths_and_config_order_independently() {
        let forward = PrewriteInput::from_parts(
            WorkflowPhase::Save,
            [
                changed("src/b.rs", ChangeKind::Modified),
                changed("src/a.rs", ChangeKind::Added),
                changed("src/b.rs", ChangeKind::Modified),
            ],
            [
                ("k".to_string(), "first".to_string()),
                ("k".to_string(), "second".to_string()),
            ],
            GraphFacts::default(),
            PrewriteBudget::default(),
        );
        let reversed = PrewriteInput::from_parts(
            WorkflowPhase::Save,
            [
                changed("src/a.rs", ChangeKind::Added),
                changed("src/b.rs", ChangeKind::Modified),
            ],
            [
                ("k".to_string(), "second".to_string()),
                ("k".to_string(), "first".to_string()),
            ],
            GraphFacts::default(),
            PrewriteBudget::default(),
        );
        // Same facts, different arrival order → equal input (determinism).
        assert_eq!(forward, reversed);
        assert_eq!(forward.changed_paths.len(), 2);
        assert_eq!(forward.changed_paths[0].path, "src/a.rs");
        assert_eq!(forward.config.get("k").map(String::as_str), Some("second"));
    }

    #[test]
    fn policy_prewrite_input_projections_agree_on_changed_paths() {
        let input = sample();
        let pack_paths = input.to_policy_input().diff.changed_files;
        let mut assertion_paths: Vec<String> = input
            .to_assertion_context()
            .changed_paths
            .into_iter()
            .map(|c| c.path)
            .collect();
        assertion_paths.dedup();
        // The single-source guarantee: both surfaces see identical changed paths.
        assert_eq!(pack_paths, assertion_paths);
        assert_eq!(pack_paths, vec!["src/a.rs".to_string(), "src/b.rs".into()]);
    }

    #[test]
    fn policy_prewrite_input_to_assertion_context_preserves_change_kinds() {
        let ctx = sample().to_assertion_context();
        assert_eq!(ctx.phase, WorkflowPhase::Save);
        assert_eq!(ctx.changed_paths[0].kind, ChangeKind::Added);
        assert_eq!(ctx.changed_paths[1].kind, ChangeKind::Modified);
        assert_eq!(ctx.config.get("signed").map(String::as_str), Some("true"));
    }

    #[test]
    fn policy_prewrite_input_policy_input_dedups_same_path_two_kinds() {
        // A path changed under two kinds must appear once in changed_files.
        let input = PrewriteInput::from_parts(
            WorkflowPhase::Commit,
            [
                changed("src/x.rs", ChangeKind::Added),
                changed("src/x.rs", ChangeKind::Modified),
            ],
            [],
            GraphFacts::default(),
            PrewriteBudget::default(),
        );
        assert_eq!(
            input.to_policy_input().diff.changed_files,
            vec!["src/x.rs".to_string()]
        );
    }

    #[test]
    fn policy_prewrite_input_repo_state_unions_graph_fact_paths() {
        let mut graph = GraphFacts::default();
        graph
            .boundaries
            .insert("crates/db/src/lib.rs".into(), vec!["db".into()]);
        graph.dependents.insert("crates/api/src/lib.rs".into(), 3);
        let input = PrewriteInput::from_parts(
            WorkflowPhase::Save,
            [changed("src/a.rs", ChangeKind::Modified)],
            [],
            graph,
            PrewriteBudget::default(),
        );
        let files = input.to_policy_input().repo_state.files;
        assert_eq!(
            files,
            vec![
                "crates/api/src/lib.rs".to_string(),
                "crates/db/src/lib.rs".into(),
                "src/a.rs".into(),
            ]
        );
    }

    #[test]
    fn policy_prewrite_input_budget_projects_onto_engine_timeout() {
        let input = PrewriteInput::from_parts(
            WorkflowPhase::Save,
            [],
            [],
            GraphFacts::default(),
            PrewriteBudget::new(Duration::from_millis(75)),
        );
        assert_eq!(
            input.engine_config().eval_timeout,
            Some(Duration::from_millis(75))
        );
    }

    #[test]
    fn policy_prewrite_input_default_budget_is_tight() {
        // Fail-open on an interactive path: the default must be far below the
        // 10 s CLI facade ceiling.
        assert!(PrewriteBudget::default().max_eval <= Duration::from_millis(500));
    }

    #[test]
    fn policy_prewrite_input_graph_facts_round_trip_and_ignore_unknown_fields() {
        let mut graph = GraphFacts::default();
        graph
            .boundaries
            .insert("src/a.rs".into(), vec!["core".into(), "io".into()]);
        graph.dependents.insert("src/a.rs".into(), 7);
        let json = serde_json::to_string(&graph).expect("serialise");
        let restored: GraphFacts = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(restored, graph);

        // Additive-friendly: an unknown field from a newer producer is ignored,
        // not rejected.
        let forward_compat = r#"{"boundaries":{},"dependents":{},"clusters":["z"]}"#;
        let parsed: GraphFacts = serde_json::from_str(forward_compat).expect("forward-compat");
        assert_eq!(parsed, GraphFacts::default());
    }

    #[test]
    fn policy_prewrite_input_budget_round_trips_through_json() {
        let budget = PrewriteBudget::new(Duration::from_millis(125));
        let json = serde_json::to_string(&budget).expect("serialise");
        let restored: PrewriteBudget = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(restored, budget);
    }
}
