//! Rule-firing trace as a result field (POLENG-006).
//!
//! ## Scope and a regorus limitation
//!
//! The intent is a structured rule-firing trace ("which rules fired, in what
//! order, with which bindings") to back the OPAE debugger and
//! `anvil policy eval --why <finding-id>`. regorus 0.10.0 does **not** expose
//! that through its public API: the internal `traces` buffer only collects
//! strings emitted by the `trace()` builtin and has no getter on `Engine`.
//!
//! What regorus *does* surface is the query's variable bindings
//! (`QueryResult.bindings`). This module captures those so the facade has a
//! real trace surface today, and the [`Trace`] type is shaped so a richer
//! rule-firing trace can populate it without an API break once upstream
//! support (or a vendored evaluator hook) lands. The constraint is recorded on
//! POLENG-006 in `plans/modules/policy-engine.aps.md`.

use serde::Serialize;

/// Evaluation trace. Today this is the variable bindings regorus surfaces per
/// query result; see the module docs for the rule-firing-order limitation.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Trace {
    /// Variable bindings produced by the evaluated query, in result order.
    /// Empty for queries that bind no variables (e.g. a direct `data.x.y`
    /// reference).
    pub bindings: Vec<serde_json::Value>,
}

impl Trace {
    /// Capture the bindings regorus exposes for each query result.
    pub(crate) fn from_results(results: &regorus::QueryResults) -> Self {
        Self {
            bindings: results
                .result
                .iter()
                .map(|qr| {
                    // Serialising a `regorus::Value` is infallible in practice;
                    // if it ever fails, surface a visible sentinel rather than
                    // silently dropping the binding to `null`.
                    serde_json::to_value(&qr.bindings).unwrap_or_else(
                        |e| serde_json::json!({ "__trace_serialization_error__": e.to_string() }),
                    )
                })
                .collect(),
        }
    }

    /// Plain-text rendering for `anvil policy eval --why`.
    pub fn explain(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::from("trace:\n");
        let non_empty: Vec<&serde_json::Value> = self
            .bindings
            .iter()
            .filter(|b| !matches!(b, serde_json::Value::Null) && !is_empty_object(b))
            .collect();
        if non_empty.is_empty() {
            out.push_str(
                "  (no variable bindings; rule-firing trace is not exposed by the engine)\n",
            );
            return out;
        }
        for (i, bindings) in non_empty.iter().enumerate() {
            let _ = writeln!(out, "  result {i}: {bindings}");
        }
        out
    }
}

fn is_empty_object(value: &serde_json::Value) -> bool {
    value.as_object().is_some_and(serde_json::Map::is_empty)
}

#[cfg(test)]
mod tests {
    use crate::{Engine, EngineConfig, PolicyInput};

    #[test]
    fn trace_captures_query_bindings() {
        let mut engine = Engine::new(EngineConfig {
            collect_trace: true,
            ..Default::default()
        })
        .expect("engine");
        engine
            .add_policy("t.rego", "package t\nimport rego.v1\nnums := [1, 2, 3]\n")
            .expect("add_policy");

        // A query with a variable produces a binding regorus surfaces.
        let result = engine
            .eval(&PolicyInput::default(), "x = data.t.nums[_]")
            .expect("eval");
        let trace = result.trace().expect("trace collected");
        assert!(
            !trace.bindings.is_empty(),
            "expected variable bindings, got {trace:?}"
        );
        assert!(trace.explain().contains("result"));
    }

    #[test]
    fn trace_absent_when_not_requested() {
        let mut engine = Engine::new(EngineConfig::default()).expect("engine");
        engine
            .add_policy("t.rego", "package t\nimport rego.v1\nx := 1\n")
            .expect("add_policy");
        let result = engine
            .eval(&PolicyInput::default(), "data.t.x")
            .expect("eval");
        assert!(result.trace().is_none());
    }
}
