//! Coverage as a first-class result field (POLENG-006).
//!
//! `regorus` gathers line coverage natively (behind its `coverage` feature,
//! enabled by this crate). This module reshapes its report into a facade type
//! so downstream consumers — the OPAE debugger, POLFED federation reporting,
//! and the `anvil policy eval --explain` surface — depend only on
//! `anvil_policy_engine`, never on `regorus`.

use serde::Serialize;

/// Line coverage for a single policy source file. Line numbers are 1-based.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileCoverage {
    pub path: String,
    /// Lines that were evaluated, ascending.
    pub covered: Vec<u32>,
    /// Lines that were not evaluated, ascending.
    pub not_covered: Vec<u32>,
}

/// Line-level coverage across every policy source loaded for an evaluation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Coverage {
    pub files: Vec<FileCoverage>,
}

impl Coverage {
    /// Reshape a `regorus` coverage report into the facade type. `BTreeSet`
    /// iteration is already ascending, so the resulting vectors are sorted.
    pub(crate) fn from_regorus(report: &regorus::coverage::Report) -> Self {
        Self {
            files: report
                .files
                .iter()
                .map(|file| FileCoverage {
                    path: file.path.clone(),
                    covered: file.covered.iter().copied().collect(),
                    not_covered: file.not_covered.iter().copied().collect(),
                })
                .collect(),
        }
    }

    /// Plain-text (no ANSI) summary for `anvil policy eval --explain`: per file,
    /// the covered/total line count and the uncovered line numbers.
    pub fn explain(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::from("coverage:\n");
        if self.files.is_empty() {
            out.push_str("  (no coverage data)\n");
            return out;
        }
        for file in &self.files {
            let covered = file.covered.len();
            let total = covered + file.not_covered.len();
            let _ = writeln!(out, "  {}: {covered}/{total} lines", file.path);
            if !file.not_covered.is_empty() {
                let _ = writeln!(
                    out,
                    "    uncovered: {}",
                    file.not_covered
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use crate::{Engine, EngineConfig, PolicyInput};

    #[test]
    fn coverage_reports_covered_and_uncovered_lines() {
        let mut engine = Engine::new(EngineConfig {
            collect_coverage: true,
            ..Default::default()
        })
        .expect("engine");

        // `unused` is never reached when querying `reached`, so its line is
        // not covered while `reached` is.
        engine
            .add_policy(
                "cov.rego",
                "package c\nimport rego.v1\n\nreached := 1\n\nunused := 2\n",
            )
            .expect("add_policy");

        let result = engine
            .eval(&PolicyInput::default(), "data.c.reached")
            .expect("eval");
        let coverage = result.coverage().expect("coverage collected");
        let file = coverage
            .files
            .iter()
            .find(|f| f.path == "cov.rego")
            .expect("file in report");

        assert!(
            file.covered.contains(&4),
            "line 4 (reached) covered: {file:?}"
        );
        assert!(
            file.not_covered.contains(&6),
            "line 6 (unused) not covered: {file:?}"
        );
        assert!(coverage.explain().contains("cov.rego"));
    }

    #[test]
    fn coverage_absent_when_not_requested() {
        let mut engine = Engine::new(EngineConfig::default()).expect("engine");
        engine
            .add_policy("c.rego", "package c\nimport rego.v1\nx := 1\n")
            .expect("add_policy");
        let result = engine
            .eval(&PolicyInput::default(), "data.c.x")
            .expect("eval");
        assert!(result.coverage().is_none());
    }
}
