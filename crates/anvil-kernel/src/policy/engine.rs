use std::collections::HashSet;

use crate::graph::{GraphDelta, SymbolGraph};
use crate::policy::config::ArchitectureConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub policy_id: String,
    pub file: String,
    pub symbol: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ViolationFingerprint {
    policy_id: String,
    file: String,
    symbol: String,
}

pub trait Invariant: Send {
    fn id(&self) -> &str;
    fn evaluate(
        &self,
        delta: &GraphDelta,
        graph: &SymbolGraph,
        config: &ArchitectureConfig,
    ) -> Vec<Violation>;
}

pub struct PolicyEngine {
    invariants: Vec<Box<dyn Invariant>>,
    seen: HashSet<ViolationFingerprint>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            invariants: Vec::new(),
            seen: HashSet::new(),
        }
    }

    pub fn register(&mut self, inv: Box<dyn Invariant>) {
        self.invariants.push(inv);
    }

    pub fn evaluate(
        &mut self,
        delta: &GraphDelta,
        graph: &SymbolGraph,
        config: &ArchitectureConfig,
    ) -> Vec<Violation> {
        let mut results = Vec::new();

        for inv in &self.invariants {
            let violations = inv.evaluate(delta, graph, config);
            for v in violations {
                let fp = ViolationFingerprint {
                    policy_id: v.policy_id.clone(),
                    file: v.file.clone(),
                    symbol: v.symbol.clone(),
                };
                if self.seen.insert(fp) {
                    results.push(v);
                }
            }
        }

        results
    }

    pub fn clear_seen(&mut self) {
        self.seen.clear();
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysViolates {
        id: String,
    }

    impl Invariant for AlwaysViolates {
        fn id(&self) -> &str {
            &self.id
        }

        fn evaluate(
            &self,
            delta: &GraphDelta,
            _graph: &SymbolGraph,
            _config: &ArchitectureConfig,
        ) -> Vec<Violation> {
            if delta.is_empty() {
                return Vec::new();
            }
            vec![Violation {
                policy_id: self.id.clone(),
                file: delta.file.clone(),
                symbol: "test_sym".to_string(),
                message: "test violation".to_string(),
                severity: Severity::High,
            }]
        }
    }

    struct NeverViolates;

    impl Invariant for NeverViolates {
        fn id(&self) -> &str {
            "never"
        }

        fn evaluate(
            &self,
            _delta: &GraphDelta,
            _graph: &SymbolGraph,
            _config: &ArchitectureConfig,
        ) -> Vec<Violation> {
            Vec::new()
        }
    }

    fn test_config() -> ArchitectureConfig {
        ArchitectureConfig { layers: Vec::new() }
    }

    fn non_empty_delta() -> GraphDelta {
        GraphDelta {
            added_symbols: vec![1],
            file: "a.ts".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn register_and_evaluate() {
        let mut engine = PolicyEngine::new();
        engine.register(Box::new(AlwaysViolates {
            id: "test".to_string(),
        }));

        let graph = SymbolGraph::new();
        let violations = engine.evaluate(&non_empty_delta(), &graph, &test_config());

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].policy_id, "test");
        assert_eq!(violations[0].file, "a.ts");
    }

    #[test]
    fn deduplication_by_fingerprint() {
        let mut engine = PolicyEngine::new();
        engine.register(Box::new(AlwaysViolates {
            id: "test".to_string(),
        }));

        let graph = SymbolGraph::new();
        let config = test_config();
        let delta = non_empty_delta();

        let first = engine.evaluate(&delta, &graph, &config);
        assert_eq!(first.len(), 1);

        let second = engine.evaluate(&delta, &graph, &config);
        assert_eq!(second.len(), 0, "duplicate should be suppressed");
    }

    #[test]
    fn multiple_invariants_run_in_sequence() {
        let mut engine = PolicyEngine::new();
        engine.register(Box::new(AlwaysViolates {
            id: "inv-a".to_string(),
        }));
        engine.register(Box::new(AlwaysViolates {
            id: "inv-b".to_string(),
        }));

        let graph = SymbolGraph::new();
        let violations = engine.evaluate(&non_empty_delta(), &graph, &test_config());

        assert_eq!(violations.len(), 2);
        let ids: Vec<&str> = violations.iter().map(|v| v.policy_id.as_str()).collect();
        assert!(ids.contains(&"inv-a"));
        assert!(ids.contains(&"inv-b"));
    }

    #[test]
    fn empty_delta_produces_no_violations() {
        let mut engine = PolicyEngine::new();
        engine.register(Box::new(AlwaysViolates {
            id: "test".to_string(),
        }));

        let graph = SymbolGraph::new();
        let violations = engine.evaluate(&GraphDelta::default(), &graph, &test_config());

        assert!(violations.is_empty());
    }

    #[test]
    fn never_violating_invariant_produces_nothing() {
        let mut engine = PolicyEngine::new();
        engine.register(Box::new(NeverViolates));

        let graph = SymbolGraph::new();
        let violations = engine.evaluate(&non_empty_delta(), &graph, &test_config());

        assert!(violations.is_empty());
    }
}
