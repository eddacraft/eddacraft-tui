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

    /// Render the evaluation trace, optionally naming the finding (0-based
    /// index) to focus on.
    #[arg(long, value_name = "FINDING")]
    why: Option<String>,

    /// Treat warnings as blocking: exit non-zero on any non-baselined warning.
    #[arg(long)]
    fail_on_warnings: bool,
}

#[derive(Debug, Serialize)]
struct EvalOutput {
    policy: String,
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<serde_json::Value>,
    findings: Vec<Finding>,
    exit_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    coverage: Option<Coverage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace: Option<Trace>,
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
    let value = result.value.clone();
    let post_processable = matches!(
        value,
        None | Some(serde_json::Value::Null | serde_json::Value::Array(_))
    );
    let (findings, exit_code) = if post_processable {
        let raw = value.clone().unwrap_or(serde_json::Value::Null);
        let report = anvil_policy_engine::result::post_process(
            &raw,
            &input,
            PostProcessOptions {
                fail_on_warnings: args.fail_on_warnings,
            },
        )
        .context("post-processing findings")?;
        (report.findings, report.exit_code)
    } else {
        (Vec::new(), 0)
    };

    let coverage = result.coverage().cloned();
    let trace = result.trace().cloned();

    let output = EvalOutput {
        policy: policy_display,
        query: args.query.clone(),
        value,
        findings,
        exit_code,
        coverage,
        trace,
    };

    if global.json {
        crate::output::json::print(&output)?;
    } else {
        render_plain(&output, args.why.as_deref());
    }

    // ADR-002: exit non-zero only when the engine says so. The report is
    // already printed, so signal the exit code without reprinting.
    if output.exit_code != 0 {
        return Err(output::AlreadyReported.into());
    }
    Ok(())
}

fn render_plain(output: &EvalOutput, why: Option<&str>) {
    crate::output::plain::blank();
    crate::output::plain::section("Policy evaluation");
    crate::output::plain::label("policy", &output.policy);
    crate::output::plain::label("query", &output.query);

    if let Some(value) = &output.value {
        let rendered = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
        crate::output::plain::label("result", rendered);
    }

    if output.findings.is_empty() {
        crate::output::plain::info("no findings");
    } else {
        crate::output::plain::section("Findings");
        for (idx, finding) in output.findings.iter().enumerate() {
            let tag = if finding.baselined {
                "baselined"
            } else if finding.is_new_edge {
                "new-edge"
            } else {
                "active"
            };
            println!(
                "  [{idx}] {sev:?} ({tag}) {msg}",
                sev = finding.severity,
                msg = finding.message
            );
        }
    }

    if let Some(coverage) = &output.coverage {
        crate::output::plain::blank();
        print!("{}", coverage.explain());
    }
    if let Some(trace) = &output.trace {
        crate::output::plain::blank();
        if let Some(focus) = why {
            crate::output::plain::label("why", focus);
        }
        print!("{}", trace.explain());
    }

    crate::output::plain::blank();
    crate::output::plain::label("exit code", output.exit_code);
}
