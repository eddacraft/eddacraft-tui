//! BENCH-016: Policy evaluation scaling scenario.
//!
//! Evaluates a growing set of policy rules against a fixed symbol graph to
//! measure how evaluation time scales with rule-set size.

use std::time::Instant;

use anvil_kernel_types::{SymbolKind, SymbolNode, TrustLevel, Visibility};

use crate::measure::MemoryGuard;
use crate::report::ScenarioResult;

/// Configuration for the policy scaling scenario.
#[derive(Debug, Clone)]
pub struct PolicyScalingConfig {
    /// Number of symbols to evaluate policies against.
    pub symbol_count: usize,
    /// Rule-set sizes to test (ascending).
    pub rule_steps: Vec<usize>,
}

impl Default for PolicyScalingConfig {
    fn default() -> Self {
        Self {
            symbol_count: 1_000,
            rule_steps: vec![10, 50, 100, 500, 1_000, 5_000],
        }
    }
}

/// A simplified policy rule for benchmarking purposes.
#[derive(Debug, Clone)]
pub struct BenchRule {
    pub id: String,
    pub match_kind: Option<SymbolKind>,
    pub match_visibility: Option<Visibility>,
    pub match_trust: Option<TrustLevel>,
    pub file_pattern: Option<String>,
}

/// Generate a set of synthetic rules.
fn generate_rules(count: usize) -> Vec<BenchRule> {
    let kinds = [
        SymbolKind::Function,
        SymbolKind::Class,
        SymbolKind::Module,
        SymbolKind::Export,
    ];
    let visibilities = [Visibility::Public, Visibility::Internal];
    let trust_levels = [
        TrustLevel::Internal,
        TrustLevel::External,
        TrustLevel::Boundary,
    ];

    (0..count)
        .map(|i| BenchRule {
            id: format!("rule_{i:04}"),
            match_kind: if i % 3 == 0 {
                Some(kinds[i % kinds.len()])
            } else {
                None
            },
            match_visibility: if i % 4 == 0 {
                Some(visibilities[i % visibilities.len()])
            } else {
                None
            },
            match_trust: if i % 5 == 0 {
                Some(trust_levels[i % trust_levels.len()])
            } else {
                None
            },
            file_pattern: if i % 7 == 0 {
                Some(format!("src/mod_{}", i % 100))
            } else {
                None
            },
        })
        .collect()
}

/// Generate synthetic symbols to evaluate against.
fn generate_symbols(count: usize) -> Vec<SymbolNode> {
    let kinds = [
        SymbolKind::Function,
        SymbolKind::Class,
        SymbolKind::Module,
        SymbolKind::Export,
    ];

    (0..count)
        .map(|i| SymbolNode {
            id: i as u64,
            kind: kinds[i % kinds.len()],
            name: format!("sym_{i}"),
            visibility: if i % 3 == 0 {
                Visibility::Public
            } else {
                Visibility::Internal
            },
            file: format!("src/mod_{}.ts", i / 10),
            trust_level: TrustLevel::Internal,
            span: None,
        })
        .collect()
}

/// Evaluate a single rule against a symbol. Returns true if the rule matches.
fn evaluate_rule(rule: &BenchRule, symbol: &SymbolNode) -> bool {
    if let Some(kind) = rule.match_kind
        && symbol.kind != kind
    {
        return false;
    }
    if let Some(vis) = rule.match_visibility
        && symbol.visibility != vis
    {
        return false;
    }
    if let Some(trust) = rule.match_trust
        && symbol.trust_level != trust
    {
        return false;
    }
    if let Some(ref pattern) = rule.file_pattern
        && !symbol.file.contains(pattern)
    {
        return false;
    }
    true
}

/// Evaluate all rules against all symbols, returning total violations.
fn evaluate_all(rules: &[BenchRule], symbols: &[SymbolNode]) -> u64 {
    let mut violations = 0u64;
    for rule in rules {
        for symbol in symbols {
            if evaluate_rule(rule, symbol) {
                violations += 1;
            }
        }
    }
    violations
}

/// Run the policy scaling scenario.
pub fn run(config: &PolicyScalingConfig) -> ScenarioResult {
    let mem = MemoryGuard::start();
    let symbols = generate_symbols(config.symbol_count);

    let mut result = ScenarioResult::new("policy_scaling");
    let scenario_start = Instant::now();

    for &rule_count in &config.rule_steps {
        let rules = generate_rules(rule_count);

        let start = Instant::now();
        let violations = evaluate_all(&rules, &symbols);
        let elapsed = start.elapsed();

        let prefix = format!("rules_{rule_count}");
        result.add_metric(
            &format!("{prefix}_eval_ms"),
            elapsed.as_secs_f64() * 1000.0,
            "ms",
        );
        result.add_metric(&format!("{prefix}_violations"), violations as f64, "count");
        let evals_per_sec = if elapsed.as_secs_f64() > 0.0 {
            (rule_count as f64 * config.symbol_count as f64) / elapsed.as_secs_f64()
        } else {
            0.0
        };
        result.add_metric(&format!("{prefix}_evals_per_sec"), evals_per_sec, "evals/s");
    }

    let mem_delta = mem.finish();
    result.set_duration(scenario_start.elapsed());
    result.add_metric("symbol_count", config.symbol_count as f64, "count");
    result.add_memory("policy", &mem_delta);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_evaluation_matches_correctly() {
        let rule = BenchRule {
            id: "test".to_string(),
            match_kind: Some(SymbolKind::Function),
            match_visibility: None,
            match_trust: None,
            file_pattern: None,
        };

        let sym_fn = SymbolNode {
            id: 0,
            kind: SymbolKind::Function,
            name: "f".to_string(),
            visibility: Visibility::Public,
            file: "src/a.ts".to_string(),
            trust_level: TrustLevel::Internal,
            span: None,
        };

        let sym_class = SymbolNode {
            id: 1,
            kind: SymbolKind::Class,
            name: "C".to_string(),
            visibility: Visibility::Public,
            file: "src/b.ts".to_string(),
            trust_level: TrustLevel::Internal,
            span: None,
        };

        assert!(evaluate_rule(&rule, &sym_fn));
        assert!(!evaluate_rule(&rule, &sym_class));
    }

    #[test]
    fn scenario_produces_scaling_metrics() {
        let config = PolicyScalingConfig {
            symbol_count: 50,
            rule_steps: vec![5, 10, 20],
        };

        let result = run(&config);
        assert_eq!(result.scenario, "policy_scaling");
        assert!(result.metrics.iter().any(|m| m.name == "rules_5_eval_ms"));
        assert!(
            result
                .metrics
                .iter()
                .any(|m| m.name == "rules_20_evals_per_sec")
        );
    }

    #[test]
    fn generated_rules_are_diverse() {
        let rules = generate_rules(100);
        let with_kind = rules.iter().filter(|r| r.match_kind.is_some()).count();
        let with_file = rules.iter().filter(|r| r.file_pattern.is_some()).count();

        assert!(with_kind > 0 && with_kind < 100);
        assert!(with_file > 0 && with_file < 100);
    }
}
