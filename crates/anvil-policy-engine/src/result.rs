//! Result post-processing — warnings-over-blocks + new-edges-only (POLENG-005).
//!
//! Raw evaluation yields an arbitrary JSON value ([`crate::EvalResult`]). Anvil
//! policies emit *findings* — an array of objects — and every tier must inherit
//! the same two defaults rather than re-implementing them:
//!
//! - **ADR-002 (warnings over blocks):** a [`Severity::Warning`] never blocks
//!   (exit 0); only a [`Severity::Error`] does. CI opts into stricter behaviour
//!   with [`PostProcessOptions::fail_on_warnings`].
//! - **ADR-003 (new edges only):** a finding whose `fingerprint` is in the
//!   baseline is annotated `baselined` and suppressed — it neither warns nor
//!   blocks, it is only tracked for drift. A finding about a dependency edge in
//!   `input.diff.new_edges` (and not baselined) is annotated `is_new_edge`.
//!
//! [`post_process`] applies both uniformly and computes the process exit code.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::PolicyInput;

/// Severity of a finding. Defaults to [`Severity::Warning`] per ADR-002, so a
/// policy that omits the field gets the non-blocking default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Advisory. Never blocks unless [`PostProcessOptions::fail_on_warnings`].
    #[default]
    Warning,
    /// Hard failure (schema error, crash). Always blocks.
    Error,
}

impl std::fmt::Display for Severity {
    /// Matches the serde wire form (`warning` / `error`) so plain and JSON
    /// output agree.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Warning => "warning",
            Self::Error => "error",
        })
    }
}

/// A single policy finding. The first block of fields is supplied by the
/// policy; `is_new_edge` and `baselined` are computed by [`post_process`] and
/// default to `false` when a raw finding is parsed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    #[serde(default)]
    pub severity: Severity,
    pub message: String,
    /// Importer side of the dependency edge this finding concerns, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Imported side of the dependency edge this finding concerns, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    /// Baseline fingerprint of this finding, if it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,

    /// Computed: the finding concerns an edge introduced by the change set and
    /// is not baselined (ADR-003).
    #[serde(default)]
    pub is_new_edge: bool,
    /// Computed: the finding's fingerprint is in the baseline cohort, so it is
    /// suppressed from save-time output (ADR-003).
    #[serde(default)]
    pub baselined: bool,
}

/// Knobs governing the exit-code policy.
#[derive(Debug, Clone, Copy, Default)]
pub struct PostProcessOptions {
    /// Opt-in stricter mode: a non-baselined warning blocks (exit 1). Off by
    /// default per ADR-002. Wired to the `--fail-on-warnings` CLI flag
    /// (POLENG-007).
    pub fail_on_warnings: bool,
}

/// The post-processed evaluation: annotated findings plus the process exit
/// code derived from ADR-002 / ADR-003.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvalReport {
    pub findings: Vec<Finding>,
    pub exit_code: i32,
}

#[derive(Debug, Error)]
pub enum ResultError {
    #[error("policy result is not a findings array: {0}")]
    Shape(String),
    #[error("could not parse findings: {0}")]
    Parse(String),
}

/// Apply ADR-002 / ADR-003 post-processing to a raw policy result.
///
/// `raw` is expected to be a JSON array of finding objects (Rego sets and
/// arrays both serialise to arrays) or `null`/absent for "no findings". Any
/// other shape is a policy authoring error and returns [`ResultError::Shape`].
pub fn post_process(
    raw: &serde_json::Value,
    input: &PolicyInput,
    opts: PostProcessOptions,
) -> Result<EvalReport, ResultError> {
    let mut findings: Vec<Finding> = match raw {
        serde_json::Value::Null => Vec::new(),
        serde_json::Value::Array(_) => {
            serde_json::from_value(raw.clone()).map_err(|e| ResultError::Parse(e.to_string()))?
        }
        other => return Err(ResultError::Shape(format!("expected array, got {other}"))),
    };

    annotate(&mut findings, input);
    let exit_code = exit_code(&findings, opts);
    Ok(EvalReport {
        findings,
        exit_code,
    })
}

/// Set the computed `baselined` and `is_new_edge` flags on each finding.
fn annotate(findings: &mut [Finding], input: &PolicyInput) {
    for finding in findings {
        finding.baselined = finding
            .fingerprint
            .as_ref()
            .is_some_and(|fp| input.baseline.findings.iter().any(|b| &b.fingerprint == fp));

        finding.is_new_edge = match (&finding.from, &finding.to) {
            (Some(from), Some(to)) => {
                !finding.baselined
                    && input
                        .diff
                        .new_edges
                        .iter()
                        .any(|edge| &edge.from == from && &edge.to == to)
            }
            _ => false,
        };
    }
}

/// ADR-002: exit 0 for warnings, non-zero only for errors; baselined findings
/// (ADR-003) are suppressed and never contribute — including errors, so a
/// historical hard finding that was baselined does not re-block every run.
fn exit_code(findings: &[Finding], opts: PostProcessOptions) -> i32 {
    let active = || findings.iter().filter(|f| !f.baselined);
    let has_error = active().any(|f| f.severity == Severity::Error);
    let has_warning = active().any(|f| f.severity == Severity::Warning);

    i32::from(has_error || (has_warning && opts.fail_on_warnings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{Baseline, BaselineFinding, DependencyEdge, Diff};
    use serde_json::json;

    fn input_with_baseline_and_new_edge() -> PolicyInput {
        PolicyInput {
            diff: Diff {
                changed_files: vec![],
                new_edges: vec![DependencyEdge {
                    from: "src/app.rs".into(),
                    to: "src/net.rs".into(),
                }],
            },
            baseline: Baseline {
                findings: vec![BaselineFinding {
                    rule_id: "r".into(),
                    file_path: "src/legacy.rs".into(),
                    fingerprint: "f00d".into(),
                }],
            },
            ..Default::default()
        }
    }

    #[test]
    fn severity_defaults_to_warning() {
        let f: Finding = serde_json::from_value(json!({"message": "x"})).expect("parse");
        assert_eq!(f.severity, Severity::Warning);
    }

    #[test]
    fn null_result_is_empty_and_passes() {
        let report = post_process(
            &json!(null),
            &PolicyInput::default(),
            PostProcessOptions::default(),
        )
        .expect("post_process");
        assert!(report.findings.is_empty());
        assert_eq!(report.exit_code, 0);
    }

    #[test]
    fn non_array_result_is_a_shape_error() {
        let err = post_process(
            &json!({"message": "oops"}),
            &PolicyInput::default(),
            PostProcessOptions::default(),
        )
        .expect_err("shape");
        assert!(matches!(err, ResultError::Shape(_)));
    }

    #[test]
    fn warnings_do_not_block_by_default_but_do_with_fail_on_warnings() {
        let raw = json!([{ "message": "new edge" }]);
        let input = PolicyInput::default();

        let lenient = post_process(&raw, &input, PostProcessOptions::default()).expect("lenient");
        assert_eq!(lenient.exit_code, 0);

        let strict = post_process(
            &raw,
            &input,
            PostProcessOptions {
                fail_on_warnings: true,
            },
        )
        .expect("strict");
        assert_eq!(strict.exit_code, 1);
    }

    #[test]
    fn errors_always_block() {
        let raw = json!([{ "severity": "error", "message": "schema failure" }]);
        let report =
            post_process(&raw, &PolicyInput::default(), PostProcessOptions::default()).expect("pp");
        assert_eq!(report.exit_code, 1);
    }

    #[test]
    fn baselined_error_is_suppressed_like_a_baselined_warning() {
        // A historical hard finding that has been baselined must not re-block
        // every run (ADR-003): baselined findings are suppressed regardless of
        // severity.
        let raw =
            json!([{ "severity": "error", "message": "legacy schema", "fingerprint": "f00d" }]);
        let report = post_process(
            &raw,
            &input_with_baseline_and_new_edge(),
            PostProcessOptions::default(),
        )
        .expect("pp");
        assert!(report.findings[0].baselined);
        assert_eq!(report.exit_code, 0, "baselined error must not block");
    }

    #[test]
    fn severity_displays_lowercase_matching_json() {
        assert_eq!(Severity::Warning.to_string(), "warning");
        assert_eq!(Severity::Error.to_string(), "error");
    }

    #[test]
    fn baselined_warning_is_suppressed_even_under_fail_on_warnings() {
        let raw = json!([{ "message": "legacy", "fingerprint": "f00d" }]);
        let report = post_process(
            &raw,
            &input_with_baseline_and_new_edge(),
            PostProcessOptions {
                fail_on_warnings: true,
            },
        )
        .expect("pp");
        assert!(report.findings[0].baselined);
        assert!(!report.findings[0].is_new_edge);
        assert_eq!(report.exit_code, 0, "baselined finding must not block");
    }

    #[test]
    fn new_edge_finding_is_annotated() {
        let raw = json!([{ "message": "app -> net", "from": "src/app.rs", "to": "src/net.rs" }]);
        let report = post_process(
            &raw,
            &input_with_baseline_and_new_edge(),
            PostProcessOptions::default(),
        )
        .expect("pp");
        assert!(report.findings[0].is_new_edge);
        assert!(!report.findings[0].baselined);
        assert_eq!(report.findings[0].from.as_deref(), Some("src/app.rs"));
        assert_eq!(report.findings[0].to.as_deref(), Some("src/net.rs"));
        assert_eq!(report.exit_code, 0);
    }

    #[test]
    fn empty_array_is_empty_and_passes() {
        // `null` is "no findings"; `[]` is the same outcome, not a
        // shape error. A policy that emits an empty set must pass.
        let report = post_process(
            &json!([]),
            &PolicyInput::default(),
            PostProcessOptions::default(),
        )
        .expect("empty array");
        assert!(report.findings.is_empty());
        assert_eq!(report.exit_code, 0);
    }

    #[test]
    fn scalar_and_string_results_are_shape_errors() {
        for raw in [json!(42), json!("oops"), json!(true)] {
            let err = post_process(&raw, &PolicyInput::default(), PostProcessOptions::default())
                .expect_err("shape");
            assert!(
                matches!(err, ResultError::Shape(_)),
                "expected Shape for {raw}, got {err:?}"
            );
        }
    }

    #[test]
    fn missing_message_is_a_parse_error_not_a_shape_error() {
        let err = post_process(
            &json!([{ "severity": "warning" }]),
            &PolicyInput::default(),
            PostProcessOptions::default(),
        )
        .expect_err("parse");
        assert!(
            matches!(err, ResultError::Parse(_)),
            "an array of objects with the wrong fields is Parse, not Shape"
        );
    }

    #[test]
    fn unknown_severity_is_a_parse_error() {
        let err = post_process(
            &json!([{ "severity": "fatal", "message": "x" }]),
            &PolicyInput::default(),
            PostProcessOptions::default(),
        )
        .expect_err("parse");
        assert!(matches!(err, ResultError::Parse(_)));
    }

    #[test]
    fn computed_flags_in_raw_input_are_overwritten() {
        // A policy that emits is_new_edge / baselined must not
        // self-certify. post_process recomputes both from the input.
        let raw = json!([{
            "message": "spoof",
            "from": "src/other.rs",
            "to": "src/nowhere.rs",
            "is_new_edge": true,
            "baselined": true
        }]);
        let report = post_process(
            &raw,
            &input_with_baseline_and_new_edge(),
            PostProcessOptions::default(),
        )
        .expect("pp");
        assert!(!report.findings[0].baselined);
        assert!(!report.findings[0].is_new_edge);
    }

    #[test]
    fn baseline_match_is_fingerprint_only() {
        // Contract: "a finding whose fingerprint is in the baseline".
        // rule_id / file_path are not part of the match — a moved
        // file with the same fingerprint stays suppressed.
        let raw = json!([{
            "severity": "error",
            "message": "moved",
            "fingerprint": "f00d",
            "from": "src/new.rs",
            "to": "src/net.rs"
        }]);
        let report = post_process(
            &raw,
            &input_with_baseline_and_new_edge(),
            PostProcessOptions::default(),
        )
        .expect("pp");
        assert!(report.findings[0].baselined);
        assert!(!report.findings[0].is_new_edge);
        assert_eq!(report.exit_code, 0);
    }

    #[test]
    fn matching_path_without_fingerprint_is_not_baselined() {
        let raw = json!([{
            "message": "legacy",
            "from": "src/legacy.rs",
            "to": "src/net.rs"
        }]);
        let report = post_process(
            &raw,
            &input_with_baseline_and_new_edge(),
            PostProcessOptions {
                fail_on_warnings: true,
            },
        )
        .expect("pp");
        assert!(!report.findings[0].baselined);
        assert_eq!(report.exit_code, 1);
    }

    #[test]
    fn different_fingerprint_is_not_baselined() {
        let raw = json!([{
            "severity": "error",
            "message": "new instance",
            "fingerprint": "beef"
        }]);
        let report = post_process(
            &raw,
            &input_with_baseline_and_new_edge(),
            PostProcessOptions::default(),
        )
        .expect("pp");
        assert!(!report.findings[0].baselined);
        assert_eq!(report.exit_code, 1);
    }

    #[test]
    fn new_edge_requires_both_endpoints() {
        let input = input_with_baseline_and_new_edge();
        for raw in [
            json!([{ "message": "from only", "from": "src/app.rs" }]),
            json!([{ "message": "to only", "to": "src/net.rs" }]),
        ] {
            let report = post_process(&raw, &input, PostProcessOptions::default()).expect("pp");
            assert!(
                !report.findings[0].is_new_edge,
                "partial endpoints must not count as a new edge: {raw}"
            );
        }
    }

    #[test]
    fn new_edge_match_is_directed() {
        let raw = json!([{
            "message": "reversed",
            "from": "src/net.rs",
            "to": "src/app.rs"
        }]);
        let report = post_process(
            &raw,
            &input_with_baseline_and_new_edge(),
            PostProcessOptions::default(),
        )
        .expect("pp");
        assert!(!report.findings[0].is_new_edge);
    }

    #[test]
    fn new_edge_does_not_match_when_only_one_side_agrees() {
        let raw = json!([{
            "message": "partial",
            "from": "src/app.rs",
            "to": "src/other.rs"
        }]);
        let report = post_process(
            &raw,
            &input_with_baseline_and_new_edge(),
            PostProcessOptions::default(),
        )
        .expect("pp");
        assert!(!report.findings[0].is_new_edge);
    }

    #[test]
    fn baselined_new_edge_is_not_annotated_is_new_edge() {
        let raw = json!([{
            "message": "legacy edge",
            "from": "src/app.rs",
            "to": "src/net.rs",
            "fingerprint": "f00d"
        }]);
        let report = post_process(
            &raw,
            &input_with_baseline_and_new_edge(),
            PostProcessOptions {
                fail_on_warnings: true,
            },
        )
        .expect("pp");
        assert!(report.findings[0].baselined);
        assert!(!report.findings[0].is_new_edge);
        assert_eq!(report.exit_code, 0);
    }

    #[test]
    fn mixed_error_and_warning_blocks_without_fail_on_warnings() {
        let raw = json!([
            { "message": "warn" },
            { "severity": "error", "message": "hard" }
        ]);
        let report =
            post_process(&raw, &PolicyInput::default(), PostProcessOptions::default()).expect("pp");
        assert_eq!(report.exit_code, 1);
        assert_eq!(report.findings.len(), 2);
        assert_eq!(report.findings[0].severity, Severity::Warning);
        assert_eq!(report.findings[1].severity, Severity::Error);
    }

    #[test]
    fn baselined_error_does_not_mask_an_active_error() {
        let raw = json!([
            { "severity": "error", "message": "legacy", "fingerprint": "f00d" },
            { "severity": "error", "message": "fresh" }
        ]);
        let report = post_process(
            &raw,
            &input_with_baseline_and_new_edge(),
            PostProcessOptions::default(),
        )
        .expect("pp");
        assert!(report.findings[0].baselined);
        assert!(!report.findings[1].baselined);
        assert_eq!(report.exit_code, 1);
    }

    #[test]
    fn baselined_error_plus_active_warning_respects_fail_on_warnings() {
        let raw = json!([
            { "severity": "error", "message": "legacy", "fingerprint": "f00d" },
            { "message": "new warn" }
        ]);
        let input = input_with_baseline_and_new_edge();
        let lenient = post_process(&raw, &input, PostProcessOptions::default()).expect("lenient");
        assert_eq!(lenient.exit_code, 0, "baselined error must not contribute");
        let strict = post_process(
            &raw,
            &input,
            PostProcessOptions {
                fail_on_warnings: true,
            },
        )
        .expect("strict");
        assert_eq!(
            strict.exit_code, 1,
            "active warning must still honour the flag"
        );
    }

    #[test]
    fn annotations_are_per_finding() {
        let raw = json!([
            { "message": "legacy", "fingerprint": "f00d" },
            { "message": "app -> net", "from": "src/app.rs", "to": "src/net.rs" },
            { "message": "unrelated" }
        ]);
        let report = post_process(
            &raw,
            &input_with_baseline_and_new_edge(),
            PostProcessOptions::default(),
        )
        .expect("pp");
        assert_eq!(report.findings.len(), 3);
        assert!(report.findings[0].baselined);
        assert!(!report.findings[0].is_new_edge);
        assert!(!report.findings[1].baselined);
        assert!(report.findings[1].is_new_edge);
        assert!(!report.findings[2].baselined);
        assert!(!report.findings[2].is_new_edge);
        assert_eq!(report.exit_code, 0);
    }

    #[test]
    fn fail_on_warnings_defaults_to_false() {
        assert!(!PostProcessOptions::default().fail_on_warnings);
    }

    #[test]
    fn finding_fields_are_preserved_through_post_process() {
        let raw = json!([{
            "severity": "error",
            "message": "app -> net",
            "from": "src/app.rs",
            "to": "src/net.rs",
            "fingerprint": "cafe"
        }]);
        let report = post_process(
            &raw,
            &input_with_baseline_and_new_edge(),
            PostProcessOptions::default(),
        )
        .expect("pp");
        let finding = &report.findings[0];
        assert_eq!(finding.severity, Severity::Error);
        assert_eq!(finding.message, "app -> net");
        assert_eq!(finding.from.as_deref(), Some("src/app.rs"));
        assert_eq!(finding.to.as_deref(), Some("src/net.rs"));
        assert_eq!(finding.fingerprint.as_deref(), Some("cafe"));
        assert!(finding.is_new_edge);
        assert!(!finding.baselined);
        assert_eq!(report.exit_code, 1);
    }

    /// End-to-end: a policy emits a findings set, and `evaluate_findings`
    /// annotates and exit-codes it under both lenient and strict options.
    #[test]
    fn evaluate_findings_end_to_end() {
        use crate::{Engine, EngineConfig};

        const POLICY: &str = r#"package arch
import rego.v1

findings contains f if {
    some edge in input.diff.new_edges
    f := {
        "message": sprintf("new edge %s -> %s", [edge.from, edge.to]),
        "from": edge.from,
        "to": edge.to,
    }
}
"#;
        let input = input_with_baseline_and_new_edge();

        let mut engine = Engine::new(EngineConfig::default()).expect("engine");
        engine.add_policy("arch.rego", POLICY).expect("add_policy");

        let lenient = engine
            .evaluate_findings(&input, "data.arch.findings", PostProcessOptions::default())
            .expect("lenient");
        assert_eq!(lenient.findings.len(), 1);
        assert!(lenient.findings[0].is_new_edge);
        assert_eq!(lenient.exit_code, 0);

        let strict = engine
            .evaluate_findings(
                &input,
                "data.arch.findings",
                PostProcessOptions {
                    fail_on_warnings: true,
                },
            )
            .expect("strict");
        assert_eq!(strict.exit_code, 1);
    }
}
