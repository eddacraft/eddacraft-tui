//! POLENG-004 repeatable-eval guarantee.
//!
//! A representative policy evaluated 100× over identical input must yield
//! byte-identical output. A fresh engine is built each iteration so the test
//! also proves the result does not depend on residual engine state.

use anvil_policy_engine::input::{DependencyEdge, Diff, RepoState};
use anvil_policy_engine::{Engine, EngineConfig, PolicyInput};

/// Reads repo state and the diff, emits a set of new-edge warnings, and folds
/// everything into a single decision object — enough Rego surface (sets,
/// comprehensions, sprintf, count) to catch non-deterministic ordering.
const REPRESENTATIVE_POLICY: &str = r#"package arch
import rego.v1

violations contains msg if {
    some edge in input.diff.new_edges
    msg := sprintf("new edge %s -> %s", [edge.from, edge.to])
}

decision := {
    "schema": input.schema_version,
    "file_count": count(input.repo_state.files),
    "violations": violations,
}
"#;

fn representative_input() -> PolicyInput {
    PolicyInput {
        repo_state: RepoState {
            files: vec!["a.rs".into(), "b.rs".into(), "c.rs".into()],
            edges: vec![DependencyEdge {
                from: "a.rs".into(),
                to: "b.rs".into(),
            }],
        },
        diff: Diff {
            changed_files: vec!["a.rs".into()],
            new_edges: vec![
                DependencyEdge {
                    from: "a.rs".into(),
                    to: "c.rs".into(),
                },
                DependencyEdge {
                    from: "b.rs".into(),
                    to: "c.rs".into(),
                },
            ],
        },
        ..Default::default()
    }
}

#[test]
fn repeatable_eval_is_byte_identical_over_100_runs() {
    let input = representative_input();
    let mut baseline: Option<String> = None;

    for i in 0..100 {
        let mut engine = Engine::new(EngineConfig::default()).expect("engine");
        engine
            .add_policy("arch.rego", REPRESENTATIVE_POLICY)
            .expect("add_policy");
        let result = engine.eval(&input, "data.arch.decision").expect("eval");
        let json = serde_json::to_string(&result.value).expect("serialise");

        match &baseline {
            None => baseline = Some(json),
            Some(first) => assert_eq!(&json, first, "eval run {i} diverged from run 0"),
        }
    }

    // Sanity: the policy actually produced a decision over *both* edges (not an
    // empty/undefined result, and not a single-edge result that would make the
    // determinism check pass vacuously).
    let decision = baseline.expect("at least one run");
    assert!(decision.contains("\"file_count\":3"), "got: {decision}");
    assert!(
        decision.contains("new edge a.rs -> c.rs"),
        "got: {decision}"
    );
    assert!(
        decision.contains("new edge b.rs -> c.rs"),
        "both new edges must appear: {decision}"
    );
}
