//! First-party builtins surface v1 (POLENG-003).
//!
//! Exposes Anvil's data sources as deterministic Rego builtins so policy
//! authors query plan, repo, and baseline state declaratively instead of
//! threading the whole input document through every rule:
//!
//! | Builtin                            | Arity | Returns                                   |
//! | ---------------------------------- | ----- | ----------------------------------------- |
//! | `anvil.repo_state()`               | 0     | the repo-state object                     |
//! | `anvil.plan(path)`                 | 1     | the plan file at `path`, or `null`        |
//! | `anvil.decision(id)`               | 1     | the decision entry `id`, or `null`        |
//! | `anvil.is_new_edge(from, to)`      | 2     | `true` if the change set adds `from→to`   |
//! | `anvil.baseline_contains(finger)`  | 1     | `true` if `finger` is in the baseline     |
//!
//! Every builtin is [`DeterminismClass::Pure`]: its result is a function of the
//! current [`PolicyInput`] and its arguments only (POLENG-004). Lookups that
//! find nothing return JSON `null` (`anvil.plan` / `anvil.decision`); set
//! membership returns a boolean.

use std::sync::Arc;

use crate::determinism::{Builtin, BuiltinError, DeterminismClass};
use crate::input::PolicyInput;
use crate::{Engine, EngineError};

/// Register all v1 first-party builtins on `engine`. Call sites (the CLI in
/// POLENG-007, downstream crates) use this rather than registering piecemeal.
pub fn register_all(engine: &mut Engine) -> Result<(), EngineError> {
    engine.register_builtin(Arc::new(RepoState))?;
    engine.register_builtin(Arc::new(Plan))?;
    engine.register_builtin(Arc::new(Decision))?;
    engine.register_builtin(Arc::new(IsNewEdge))?;
    engine.register_builtin(Arc::new(BaselineContains))?;
    Ok(())
}

/// Extract a required string argument, or report a typed error. `regorus`
/// already enforces arity at the call site; the [`BuiltinError::Arity`] arm
/// guards direct/unit calls and any future dynamic registration path.
fn str_arg<'a>(
    name: &'static str,
    args: &'a [serde_json::Value],
    idx: usize,
) -> Result<&'a str, BuiltinError> {
    match args.get(idx) {
        Some(serde_json::Value::String(s)) => Ok(s),
        Some(other) => Err(BuiltinError::Invalid {
            name: name.to_string(),
            message: format!("argument {idx} must be a string, got {other}"),
        }),
        None => Err(BuiltinError::Arity {
            name: name.to_string(),
            expected: u8::try_from(idx + 1).unwrap_or(u8::MAX),
            got: args.len(),
        }),
    }
}

fn to_value(
    name: &'static str,
    value: &impl serde::Serialize,
) -> Result<serde_json::Value, BuiltinError> {
    serde_json::to_value(value).map_err(|e| BuiltinError::Invalid {
        name: name.to_string(),
        message: e.to_string(),
    })
}

/// `anvil.repo_state()` — the current repo-state object.
struct RepoState;
impl Builtin for RepoState {
    fn name(&self) -> &'static str {
        "anvil.repo_state"
    }
    fn arity(&self) -> u8 {
        0
    }
    fn determinism(&self) -> DeterminismClass {
        DeterminismClass::Pure
    }
    fn call(
        &self,
        input: &PolicyInput,
        _args: &[serde_json::Value],
    ) -> Result<serde_json::Value, BuiltinError> {
        to_value(self.name(), &input.repo_state)
    }
}

/// `anvil.plan(path)` — the plan file at `path`, or `null` if none.
struct Plan;
impl Builtin for Plan {
    fn name(&self) -> &'static str {
        "anvil.plan"
    }
    fn arity(&self) -> u8 {
        1
    }
    fn determinism(&self) -> DeterminismClass {
        DeterminismClass::Pure
    }
    fn call(
        &self,
        input: &PolicyInput,
        args: &[serde_json::Value],
    ) -> Result<serde_json::Value, BuiltinError> {
        let path = str_arg(self.name(), args, 0)?;
        match input.plans.iter().find(|p| p.path == path) {
            Some(plan) => to_value(self.name(), plan),
            None => Ok(serde_json::Value::Null),
        }
    }
}

/// `anvil.decision(id)` — the decision entry `id`, or `null` if none.
struct Decision;
impl Builtin for Decision {
    fn name(&self) -> &'static str {
        "anvil.decision"
    }
    fn arity(&self) -> u8 {
        1
    }
    fn determinism(&self) -> DeterminismClass {
        DeterminismClass::Pure
    }
    fn call(
        &self,
        input: &PolicyInput,
        args: &[serde_json::Value],
    ) -> Result<serde_json::Value, BuiltinError> {
        let id = str_arg(self.name(), args, 0)?;
        match input.decisions.iter().find(|d| d.id == id) {
            Some(decision) => to_value(self.name(), decision),
            None => Ok(serde_json::Value::Null),
        }
    }
}

/// `anvil.is_new_edge(from, to)` — `true` if the change set introduces the
/// dependency edge `from → to` (ADR-003 "new edges only").
struct IsNewEdge;
impl Builtin for IsNewEdge {
    fn name(&self) -> &'static str {
        "anvil.is_new_edge"
    }
    fn arity(&self) -> u8 {
        2
    }
    fn determinism(&self) -> DeterminismClass {
        DeterminismClass::Pure
    }
    fn call(
        &self,
        input: &PolicyInput,
        args: &[serde_json::Value],
    ) -> Result<serde_json::Value, BuiltinError> {
        let from = str_arg(self.name(), args, 0)?;
        let to = str_arg(self.name(), args, 1)?;
        let found = input
            .diff
            .new_edges
            .iter()
            .any(|edge| edge.from == from && edge.to == to);
        Ok(serde_json::Value::Bool(found))
    }
}

/// `anvil.baseline_contains(fingerprint)` — `true` if a baselined finding with
/// that fingerprint exists, so policies can exclude pre-existing findings.
struct BaselineContains;
impl Builtin for BaselineContains {
    fn name(&self) -> &'static str {
        "anvil.baseline_contains"
    }
    fn arity(&self) -> u8 {
        1
    }
    fn determinism(&self) -> DeterminismClass {
        DeterminismClass::Pure
    }
    fn call(
        &self,
        input: &PolicyInput,
        args: &[serde_json::Value],
    ) -> Result<serde_json::Value, BuiltinError> {
        let fingerprint = str_arg(self.name(), args, 0)?;
        let found = input
            .baseline
            .findings
            .iter()
            .any(|f| f.fingerprint == fingerprint);
        Ok(serde_json::Value::Bool(found))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{
        Baseline, BaselineFinding, DecisionEntry, DependencyEdge, Diff, PlanFile,
        RepoState as RepoStateDoc,
    };
    use crate::{EngineConfig, PolicyInput};
    use serde_json::json;

    fn fixture() -> PolicyInput {
        PolicyInput {
            repo_state: RepoStateDoc {
                files: vec!["src/app.rs".into(), "src/db.rs".into()],
                edges: vec![DependencyEdge {
                    from: "src/app.rs".into(),
                    to: "src/db.rs".into(),
                }],
            },
            plans: vec![PlanFile {
                path: "plans/modules/policy-engine.aps.md".into(),
                id: Some("POLENG".into()),
                status: Some("In Progress".into()),
            }],
            decisions: vec![DecisionEntry {
                id: "040".into(),
                title: Some("Adopt regorus".into()),
                status: Some("Accepted".into()),
            }],
            diff: Diff {
                changed_files: vec!["src/app.rs".into()],
                new_edges: vec![DependencyEdge {
                    from: "src/app.rs".into(),
                    to: "src/net.rs".into(),
                }],
            },
            baseline: Baseline {
                findings: vec![BaselineFinding {
                    rule_id: "anti-pattern:x".into(),
                    file_path: "src/legacy.rs".into(),
                    fingerprint: "f00dcafe".into(),
                }],
            },
            ..Default::default()
        }
    }

    #[test]
    fn repo_state_returns_the_repo_object() {
        let out = RepoState.call(&fixture(), &[]).expect("call");
        assert_eq!(out["files"], json!(["src/app.rs", "src/db.rs"]));
        assert_eq!(out["edges"][0]["to"], json!("src/db.rs"));
    }

    #[test]
    fn plan_found_returns_object_absent_returns_null() {
        let input = fixture();
        let found = Plan
            .call(&input, &[json!("plans/modules/policy-engine.aps.md")])
            .expect("found");
        assert_eq!(found["id"], json!("POLENG"));

        let absent = Plan
            .call(&input, &[json!("plans/nope.md")])
            .expect("absent");
        assert_eq!(absent, serde_json::Value::Null);
    }

    #[test]
    fn plan_rejects_non_string_argument() {
        let err = Plan.call(&fixture(), &[json!(42)]).expect_err("malformed");
        assert!(matches!(err, BuiltinError::Invalid { .. }), "got {err:?}");
    }

    #[test]
    fn decision_found_and_absent() {
        let input = fixture();
        assert_eq!(
            Decision.call(&input, &[json!("040")]).expect("found")["status"],
            json!("Accepted")
        );
        assert_eq!(
            Decision.call(&input, &[json!("999")]).expect("absent"),
            serde_json::Value::Null
        );
    }

    #[test]
    fn is_new_edge_true_false_and_malformed() {
        let input = fixture();
        assert_eq!(
            IsNewEdge
                .call(&input, &[json!("src/app.rs"), json!("src/net.rs")])
                .expect("present"),
            json!(true)
        );
        assert_eq!(
            IsNewEdge
                .call(&input, &[json!("src/app.rs"), json!("src/db.rs")])
                .expect("absent"),
            json!(false)
        );
        // Missing second argument → arity error.
        let err = IsNewEdge
            .call(&input, &[json!("src/app.rs")])
            .expect_err("arity");
        assert!(
            matches!(err, BuiltinError::Arity { expected: 2, .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn baseline_contains_true_false() {
        let input = fixture();
        assert_eq!(
            BaselineContains
                .call(&input, &[json!("f00dcafe")])
                .expect("present"),
            json!(true)
        );
        assert_eq!(
            BaselineContains
                .call(&input, &[json!("deadbeef")])
                .expect("absent"),
            json!(false)
        );
    }

    /// End-to-end: registered builtins resolve and read the live input
    /// document when called from a real policy.
    #[test]
    fn builtins_resolve_end_to_end_through_a_policy() {
        let mut engine = Engine::new(EngineConfig::default()).expect("engine");
        register_all(&mut engine).expect("register_all");
        engine
            .add_policy(
                "p.rego",
                r#"package p
import rego.v1

# A new edge into an un-baselined target is a fresh violation.
fresh_net_edge if {
    anvil.is_new_edge("src/app.rs", "src/net.rs")
    not anvil.baseline_contains("f00dcafe-missing")
}

known_files := anvil.repo_state().files
"#,
            )
            .expect("add_policy");

        let input = fixture();
        assert_eq!(
            engine
                .eval(&input, "data.p.fresh_net_edge")
                .expect("eval")
                .value,
            Some(json!(true))
        );
        assert_eq!(
            engine
                .eval(&input, "data.p.known_files")
                .expect("eval")
                .value,
            Some(json!(["src/app.rs", "src/db.rs"]))
        );
    }
}
