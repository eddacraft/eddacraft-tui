//! Inlined GitHub Actions workflow template for per-PR L4 validation
//! plus the CI merge gate (`anvil gate --profile ci`).
//!
//! Copied into `.github/workflows/anvil.yml` at adoption; ADR-037 default-on.

/// Inlined template for the per-PR L4-validation plus CI merge-gate
/// GitHub workflow.
///
/// Public so the activation orchestrator (`anvil start` / `anvil
/// baseline`) can copy it into `.github/workflows/anvil.yml` at
/// adoption time. ADR-037 §D-5: active by default; operator
/// disables by commenting out the `pull_request` trigger.
/// ADR-127 / GTAO-006: L4 and `anvil gate --profile ci` are not substitutes.
///
/// `#[allow(dead_code)]` because the call site lives in the
/// activation orchestrator (deferred follow-up). The template is
/// exercised by [`tests::anvil_workflow_template_is_valid_yaml_shape`].
#[must_use]
#[allow(dead_code)]
pub fn anvil_workflow_template() -> &'static str {
    include_str!("../templates/anvil-workflow.yml")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The activation orchestrator copies the template byte-for-byte
    /// into a target repo. If the template's YAML shape changes
    /// silently — name, on-trigger, jobs key, the install
    /// placeholder, or the `anvil hook pre-push` invocation — that
    /// regression should fail this pin first rather than be caught
    /// in a downstream repo that adopted anvil days ago. The
    /// substrings asserted here are the contract the docs and the
    /// Marketplace action commit to.
    #[test]
    fn anvil_workflow_template_is_valid_yaml_shape() {
        let t = anvil_workflow_template();
        assert!(t.contains("name: anvil"), "template names the workflow");
        assert!(
            t.contains("on:") && t.contains("pull_request:"),
            "template triggers on pull_request"
        );
        assert!(
            t.contains("workflow_dispatch:"),
            "template supports manual dispatch with policy + fail-on-warning inputs",
        );
        assert!(
            t.contains("eddacraft/anvil-action@v1"),
            "template references the future Marketplace action so adopters know the swap",
        );
        assert!(
            t.contains("anvil hook pre-push"),
            "template invokes the developer-side pre-push surface for parity",
        );
        assert!(
            t.contains("placeholder"),
            "install step is clearly marked as a placeholder pending MLP-010 Marketplace publish",
        );
        assert!(
            t.contains("fetch-depth: 0"),
            "checkout must fetch the full PR commit range for pre-push to walk",
        );
    }

    /// MLP-010 documents inputs the future Marketplace action will
    /// accept. The template's `workflow_dispatch` block mirrors
    /// those input names so manual runs against the placeholder
    /// install path produce the same shape an `eddacraft/anvil-
    /// action@v1` invocation will. We pin the **exact GitHub Actions
    /// expression forms** the run step references so a refactor
    /// that breaks the expression syntax (e.g., switching to
    /// `inputs['fail-on-warning']` without testing it, or renaming
    /// the input) fails this contract test before CI ever runs the
    /// workflow against a real PR.
    #[test]
    fn anvil_workflow_template_advertises_documented_inputs() {
        let t = anvil_workflow_template();
        assert!(
            t.contains("policy:"),
            "policy input documents anvil/policy.yml path override",
        );
        assert!(
            t.contains("fail-on-warning:"),
            "fail-on-warning input documents the warning → failure escalation",
        );
        // Pin the exact expression forms — GitHub Actions accepts
        // both `inputs.foo` and `inputs['foo']`, but kebab-case
        // input names CANNOT use the dotted form because of the
        // hyphen, so `fail-on-warning` MUST be indexed via the
        // `inputs.fail-on-warning` operator form (which the
        // Actions runtime parses as `inputs['fail-on-warning']`
        // internally). A refactor that breaks this in either
        // direction would fail.
        assert!(
            t.contains("${{ github.event.inputs.policy || 'anvil/policy.yml' }}"),
            "policy expression form: must reference github.event.inputs.policy with a string default",
        );
        assert!(
            t.contains("${{ github.event.inputs.fail-on-warning || 'false' }}"),
            "fail-on-warning expression form: must reference github.event.inputs.fail-on-warning with a string default",
        );
    }

    /// Sanity check that the placeholder install path uses HTTPS
    /// (per the air-gapped / supply-chain doctrine) and not plain
    /// HTTP. Adopters running this template against a corporate
    /// mirror should be on guard for downgrade rewrites; this is a
    /// belt-and-braces pin so the template doesn't ship with `http://`.
    #[test]
    fn anvil_workflow_template_uses_https_install_only() {
        let t = anvil_workflow_template();
        assert!(
            !t.contains("http://"),
            "template must not include http:// — supply-chain doctrine",
        );
        assert!(
            t.contains("https://"),
            "template uses https:// for the placeholder install URL",
        );
    }

    /// The action's job-level permissions should be read-only —
    /// L4 validation never writes back to the repo from CI (the L4
    /// witness goes to `refs/notes/anvil-l4` from the daemon side
    /// per ADR-037 §D-7, not from this workflow). Pin so a future
    /// change to grant write access surfaces in review.
    #[test]
    fn anvil_workflow_template_uses_read_only_permissions() {
        let t = anvil_workflow_template();
        assert!(
            t.contains("contents: read"),
            "template grants read-only access to repo contents",
        );
        assert!(
            !t.contains("contents: write"),
            "template must not grant write access to repo contents",
        );
    }

    /// GTAO-006 / ADR-127: the adopter pull-request template must run the
    /// full merge gate in CI *in addition to* L4 pre-push. The two jobs
    /// are distinct layers, not substitutes. The L5 audit sibling does
    /// not share this hole (nightly `anvil audit-chain`).
    #[test]
    fn anvil_workflow_template_runs_ci_gate_in_addition_to_l4() {
        let t = anvil_workflow_template();
        assert!(
            t.contains("anvil gate --profile ci"),
            "template must invoke anvil gate --profile ci as the merge judgement",
        );
        assert!(
            t.contains("anvil hook pre-push"),
            "L4 pre-push job must remain a separate layer",
        );
        assert!(
            t.contains("\n  validate:\n") && t.contains("\n  gate:\n"),
            "L4 (validate) and merge gate (gate) must be separate jobs",
        );
        let lower = t.to_ascii_lowercase();
        assert!(
            lower.contains("not substitutes") || lower.contains("not a substitute"),
            "template comments must state the two jobs are not substitutes",
        );
        let validate_job = t
            .split_once("\n  validate:")
            .and_then(|(_, rest)| rest.split_once("\n  gate:"))
            .map(|(job, _)| job)
            .unwrap_or("");
        let gate_job = t
            .split_once("\n  gate:")
            .map(|(_, rest)| rest)
            .unwrap_or("");
        assert!(
            !validate_job.contains("anvil gate --profile ci"),
            "merge gate must not be folded into the L4 validate job",
        );
        assert!(
            validate_job.contains("anvil hook pre-push"),
            "L4 pre-push must live in the validate job",
        );
        assert!(
            validate_job.contains("replace with eddacraft/anvil-action@v1"),
            "Marketplace L4 swap belongs on the validate install step",
        );
        assert!(
            gate_job.contains("anvil gate --profile ci"),
            "merge gate command must live in the gate job",
        );
        assert!(
            !gate_job.contains("anvil hook pre-push"),
            "L4 pre-push must not be folded into the gate job",
        );
        assert!(
            !gate_job.contains("replace with eddacraft/anvil-action@v1"),
            "Marketplace L4 action must not be the gate install swap",
        );
        assert!(
            gate_job.contains("ANVIL_LICENSE: ${{ secrets.ANVIL_LICENSE }}"),
            "gate job must bind ANVIL_LICENSE via env; anvil gate is licence-gated",
        );
    }
}
