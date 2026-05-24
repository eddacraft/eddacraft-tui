//! `anvil policy eval` — evaluate a Rego policy through the POLENG engine
//! (POLENG-007).
//!
//! Loads a `.rego` policy and an optional `PolicyInput` JSON document, runs the
//! query through `anvil_policy_engine`, and emits a JSON or plain report plus
//! an exit code. When the query resolves to a findings array, ADR-002/003
//! post-processing applies: warnings exit 0 by default, errors and (with
//! `--fail-on-warnings`) non-baselined warnings exit non-zero.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;

use anvil_policy_engine::{
    Coverage, Engine, EngineConfig, Finding, PolicyInput, PostProcessOptions, Trace,
};

use crate::GlobalArgs;
use crate::output;

#[derive(Debug, Args)]
pub struct EvalArgs {
    /// Path to the `.rego` policy file to evaluate.
    policy: PathBuf,

    /// Path to a JSON `PolicyInput` document. Defaults to an empty input.
    #[arg(long, value_name = "PATH")]
    input: Option<PathBuf>,

    /// Rego query to evaluate. Point at a findings rule
    /// (e.g. `data.arch.findings`) for ADR-002/003 gating semantics.
    #[arg(long, value_name = "QUERY", default_value = "data")]
    query: String,

    /// Render line coverage for the evaluation.
    #[arg(long)]
    explain: bool,

    /// Explain a finding by its 0-based index: render the evaluation trace and
    /// highlight that finding. (Per-finding trace is limited by the engine —
    /// see POLENG-006 — so the trace shown is the query-level trace.)
    #[arg(long, value_name = "INDEX")]
    why: Option<usize>,

    /// Treat warnings as blocking: exit non-zero on any non-baselined warning.
    #[arg(long)]
    fail_on_warnings: bool,
}

#[derive(Debug, Serialize)]
struct EvalOutput {
    policy: String,
    query: String,
    /// The raw query result. Present only for non-findings queries; for a
    /// findings query the canonical representation is `findings`, so the raw
    /// array is not duplicated here.
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<serde_json::Value>,
    findings: Vec<Finding>,
    exit_code: i32,
    /// The finding index `--why` focused on, echoed for JSON consumers.
    #[serde(skip_serializing_if = "Option::is_none")]
    why: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    coverage: Option<Coverage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace: Option<Trace>,
}

/// Human-readable kind of a JSON value, for the non-findings diagnostic.
fn value_kind(value: Option<&serde_json::Value>) -> &'static str {
    match value {
        None => "no result (undefined)",
        Some(serde_json::Value::Null) => "null",
        Some(serde_json::Value::Bool(_)) => "a boolean",
        Some(serde_json::Value::Number(_)) => "a number",
        Some(serde_json::Value::String(_)) => "a string",
        Some(serde_json::Value::Array(_)) => "an array",
        Some(serde_json::Value::Object(_)) => "an object",
    }
}

pub fn run(args: &EvalArgs, global: &GlobalArgs) -> Result<()> {
    let policy_display = args.policy.display().to_string();
    let policy_source = fs::read_to_string(&args.policy)
        .with_context(|| format!("reading policy file `{policy_display}`"))?;

    let input = match &args.input {
        Some(path) => {
            let raw = fs::read_to_string(path)
                .with_context(|| format!("reading input file `{}`", path.display()))?;
            serde_json::from_str::<PolicyInput>(&raw).with_context(|| {
                format!("parsing `{}` as a PolicyInput document", path.display())
            })?
        }
        None => PolicyInput::default(),
    };

    let mut engine = Engine::new(EngineConfig {
        collect_coverage: args.explain,
        collect_trace: args.why.is_some(),
        ..Default::default()
    })
    .context("constructing policy engine")?;
    anvil_policy_engine::builtins::register_all(&mut engine)
        .context("registering first-party builtins")?;
    engine
        .add_policy(policy_display.clone(), policy_source)
        .with_context(|| format!("loading policy `{policy_display}`"))?;

    let result = engine
        .eval(&input, &args.query)
        .with_context(|| format!("evaluating query `{}`", args.query))?;

    // Apply findings post-processing only when the query resolves to a
    // findings-shaped result (array or absent); otherwise surface the raw value.
    let raw_value = result.value.clone();
    let post_processable = matches!(
        raw_value,
        None | Some(serde_json::Value::Null | serde_json::Value::Array(_))
    );

    // A non-array result from a query the user is gating on is a policy
    // authoring error; fail loudly rather than silently passing (exit 0).
    if !post_processable && args.fail_on_warnings {
        anyhow::bail!(
            "query `{}` returned {}, not a findings array; --fail-on-warnings requires a findings query",
            args.query,
            value_kind(raw_value.as_ref()),
        );
    }

    let (findings, exit_code, value) = if post_processable {
        let raw = raw_value.unwrap_or(serde_json::Value::Null);
        let report = anvil_policy_engine::result::post_process(
            &raw,
            &input,
            PostProcessOptions {
                fail_on_warnings: args.fail_on_warnings,
            },
        )
        .context("post-processing findings")?;
        // `findings` is canonical; drop the raw array so the report does not
        // carry the same data twice.
        (report.findings, report.exit_code, None)
    } else {
        (Vec::new(), 0, raw_value)
    };

    // Validate --why against the findings actually produced.
    if let Some(idx) = args.why
        && idx >= findings.len()
    {
        anyhow::bail!(
            "--why {idx}: no finding at that index ({} finding(s) produced)",
            findings.len(),
        );
    }

    let coverage = result.coverage().cloned();
    let trace = result.trace().cloned();

    let output = EvalOutput {
        policy: policy_display,
        query: args.query.clone(),
        value,
        findings,
        exit_code,
        why: args.why,
        coverage,
        trace,
    };

    if global.json {
        crate::output::json::print(&output)?;
    } else {
        render_plain(&output);
    }

    // ADR-002: exit non-zero only when the engine says so. The report is
    // already printed, so signal the exit code without reprinting.
    if output.exit_code != 0 {
        return Err(output::AlreadyReported.into());
    }
    Ok(())
}

fn render_plain(output: &EvalOutput) {
    use crate::output::plain;

    plain::blank();
    plain::section("Policy evaluation");
    plain::label("policy", &output.policy);
    plain::label("query", &output.query);

    if let Some(value) = &output.value {
        let rendered = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
        plain::label("result", rendered);
    }

    if output.findings.is_empty() {
        plain::info("no findings");
    } else {
        plain::section("Findings");
        for (idx, finding) in output.findings.iter().enumerate() {
            let tag = if finding.baselined {
                "baselined"
            } else if finding.is_new_edge {
                "new-edge"
            } else {
                "active"
            };
            // Mark the finding `--why` focused on.
            let marker = if output.why == Some(idx) { '>' } else { ' ' };
            println!(
                "  {marker} [{idx}] {sev} ({tag}) {msg}",
                sev = finding.severity,
                msg = finding.message
            );
        }
    }

    if let Some(coverage) = &output.coverage {
        plain::blank();
        print!("{}", coverage.explain());
    }
    if let Some(trace) = &output.trace {
        plain::blank();
        if let Some(idx) = output.why {
            plain::label("why", format!("finding [{idx}]"));
        }
        print!("{}", trace.explain());
    }

    plain::blank();
    plain::label("exit code", output.exit_code);
}
