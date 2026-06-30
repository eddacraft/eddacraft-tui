//! EVAL-005 — link eval failures to policy guidance.
//!
//! A failing trust-regression run is only actionable if it tells the operator
//! *why* it blocked and *what to do next*. [`guidance_for`] turns a failing
//! [`EvalRunSummary`] into per-finding [`PolicyGuidance`]: the policy context
//! plus remediation-oriented next actions, derived deterministically from the
//! finding's own shape (severity, dependency edge, baseline fingerprint).
//!
//! Guidance is advisory prose, not a policy decision — it never changes a
//! verdict, it explains one. A passing run yields no guidance.

use serde::{Deserialize, Serialize};

use super::port::{EvalFinding, EvalRunSummary, EvalSeverity};

/// Remediation guidance attached to a single failing finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyGuidance {
    /// The policy that produced the finding — the context an operator needs to
    /// locate the rule.
    pub policy: String,
    /// One-line restatement of what the policy flagged.
    pub summary: String,
    /// Ordered, concrete next actions, most direct remedy first.
    pub next_actions: Vec<String>,
}

/// A failing finding paired with its remediation guidance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuidedFinding {
    pub finding: EvalFinding,
    pub guidance: PolicyGuidance,
}

/// Produce guidance for every blocking finding in a failing run. A passing run
/// (exit 0) returns an empty vector — there is nothing to remediate.
///
/// "Blocking" is read from the verdict, not assumed: normally the `error`
/// findings block, but under fail-on-warnings a run can fail with only
/// `warning` findings, and those then carry the guidance.
pub fn guidance_for(summary: &EvalRunSummary) -> Vec<GuidedFinding> {
    if summary.passed() {
        return Vec::new();
    }

    let has_errors = summary.error_count() > 0;
    summary
        .findings
        .iter()
        .filter(|f| {
            if has_errors {
                f.severity == EvalSeverity::Error
            } else {
                // Warnings-as-errors run: the warnings are what blocked.
                f.severity == EvalSeverity::Warning
            }
        })
        .map(|finding| GuidedFinding {
            finding: finding.clone(),
            guidance: guidance_for_finding(&summary.policy, &summary.query, finding),
        })
        .collect()
}

/// Derive guidance for one finding. Deterministic: the same finding always
/// yields the same actions.
fn guidance_for_finding(policy: &str, query: &str, finding: &EvalFinding) -> PolicyGuidance {
    let summary = format!(
        "Policy `{policy}` reported a {} finding: {}",
        finding.severity, finding.message
    );

    let mut next_actions = Vec::new();

    match (&finding.from, &finding.to) {
        (Some(from), Some(to)) => {
            next_actions.push(format!(
                "Remove the dependency from `{from}` to `{to}`, or route it through an approved port/boundary so the edge is permitted."
            ));
        }
        _ => {
            next_actions.push(format!(
                "Address the condition flagged by `{policy}`: {}",
                finding.message
            ));
        }
    }

    if let Some(fp) = &finding.fingerprint {
        next_actions.push(format!(
            "If this is accepted, pre-existing debt, baseline it (fingerprint `{fp}`) with `anvil drift snapshot` so it no longer regresses the gate."
        ));
    }

    next_actions.push(format!(
        "Re-run the suite to confirm the fix: `anvil policy eval --json {policy} --query {query}`."
    ));

    PolicyGuidance {
        policy: policy.to_string(),
        summary,
        next_actions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(
        sev: EvalSeverity,
        msg: &str,
        edge: Option<(&str, &str)>,
        fp: Option<&str>,
    ) -> EvalFinding {
        EvalFinding {
            severity: sev,
            message: msg.into(),
            from: edge.map(|(f, _)| f.into()),
            to: edge.map(|(_, t)| t.into()),
            fingerprint: fp.map(Into::into),
        }
    }

    fn summary(exit: i32, findings: Vec<EvalFinding>) -> EvalRunSummary {
        EvalRunSummary {
            suite: "arch".into(),
            schema_version: "1.0.0".into(),
            policy: "policies/arch_boundary.rego".into(),
            query: "data.anvil.arch.findings".into(),
            findings,
            exit_code: exit,
        }
    }

    #[test]
    fn eval_policy_guidance_passing_run_has_no_guidance() {
        let s = summary(
            0,
            vec![finding(EvalSeverity::Warning, "advisory", None, None)],
        );
        assert!(guidance_for(&s).is_empty());
    }

    #[test]
    fn eval_policy_guidance_failing_edge_finding_has_policy_context_and_actions() {
        let s = summary(
            1,
            vec![finding(
                EvalSeverity::Error,
                "ui imports db directly",
                Some(("src/ui.rs", "src/db.rs")),
                Some("abc123"),
            )],
        );
        let guided = guidance_for(&s);
        assert_eq!(guided.len(), 1);
        let g = &guided[0].guidance;

        // Policy context is present.
        assert_eq!(g.policy, "policies/arch_boundary.rego");
        assert!(g.summary.contains("policies/arch_boundary.rego"));
        assert!(g.summary.contains("ui imports db directly"));

        // Recommended next actions are concrete and remediation-oriented.
        assert!(g.next_actions.len() >= 2);
        assert!(g.next_actions[0].contains("src/ui.rs") && g.next_actions[0].contains("src/db.rs"));
        // The fingerprint yields a baseline suggestion.
        assert!(
            g.next_actions
                .iter()
                .any(|a| a.contains("abc123") && a.contains("baseline"))
        );
        // And a re-run instruction.
        assert!(
            g.next_actions
                .iter()
                .any(|a| a.contains("anvil policy eval"))
        );
    }

    #[test]
    fn eval_policy_guidance_only_guides_blocking_findings() {
        // A failing run with one error and one warning guides only the error.
        let s = summary(
            1,
            vec![
                finding(EvalSeverity::Error, "blocking", None, None),
                finding(EvalSeverity::Warning, "advisory", None, None),
            ],
        );
        let guided = guidance_for(&s);
        assert_eq!(guided.len(), 1);
        assert_eq!(guided[0].finding.message, "blocking");
    }

    #[test]
    fn eval_policy_guidance_warnings_as_errors_run_guides_warnings() {
        // exit 1 with no error findings == fail-on-warnings; the warnings block.
        let s = summary(
            1,
            vec![finding(EvalSeverity::Warning, "noisy", None, Some("w1"))],
        );
        let guided = guidance_for(&s);
        assert_eq!(guided.len(), 1);
        assert_eq!(guided[0].finding.message, "noisy");
        assert!(!guided[0].guidance.next_actions.is_empty());
    }

    #[test]
    fn eval_policy_guidance_non_edge_finding_falls_back_to_message_action() {
        let s = summary(
            1,
            vec![finding(EvalSeverity::Error, "secret detected", None, None)],
        );
        let g = &guidance_for(&s)[0].guidance;
        assert!(g.next_actions[0].contains("secret detected"));
        // No fingerprint -> no baseline suggestion, but still a re-run action.
        assert!(!g.next_actions.iter().any(|a| a.contains("baseline")));
        assert!(
            g.next_actions
                .iter()
                .any(|a| a.contains("anvil policy eval"))
        );
    }
}
