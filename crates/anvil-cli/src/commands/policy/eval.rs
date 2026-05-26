//! `anvil policy eval` — evaluate a Rego policy through the POLENG engine
//! (POLENG-007).
//!
//! Loads a `.rego` policy and an optional `PolicyInput` JSON document, runs the
//! query through `anvil_policy_engine`, and emits a JSON or plain report plus
//! an exit code. When the query resolves to a findings array, ADR-002/003
//! post-processing applies: warnings exit 0 by default, errors and (with
//! `--fail-on-warnings`) non-baselined warnings exit non-zero.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;

use anvil_policy_engine::{
    Coverage, Engine, EngineConfig, Finding, PolicyInput, PostProcessOptions, Trace,
};

use crate::GlobalArgs;
use crate::output;

/// POLENG-009 resource bound: reject oversized policy/input files before
/// reading them into memory. The policy cap mirrors regorus's 1 MiB default;
/// the input document may legitimately be larger (a whole repo's state).
const MAX_POLICY_BYTES: u64 = 1 << 20; // 1 MiB
const MAX_INPUT_BYTES: u64 = 8 << 20; // 8 MiB

/// Read a regular file, refusing it if it exceeds `max` bytes. The size is
/// checked via metadata before the read so an oversized file never lands in
/// memory. Non-regular files are rejected outright: `metadata().len()` reports
/// 0 for `/proc` entries, FIFOs, and device files like `/dev/zero`, which would
/// otherwise dodge the cap and hang or exhaust memory in `read_to_string`.
fn read_capped(path: &Path, max: u64, what: &str) -> Result<String> {
    let meta =
        fs::metadata(path).with_context(|| format!("reading {what} `{}`", path.display()))?;
    if !meta.file_type().is_file() {
        anyhow::bail!("{what} `{}` is not a regular file", path.display());
    }
    if meta.len() > max {
        anyhow::bail!(
            "{what} `{}` is {} bytes, over the {max}-byte limit",
            path.display(),
            meta.len(),
        );
    }
    fs::read_to_string(path).with_context(|| format!("reading {what} `{}`", path.display()))
}

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

#[tracing::instrument(
    name = "policy_eval",
    skip_all,
    fields(policy = %args.policy.display(), query = %args.query)
)]
pub fn run(args: &EvalArgs, global: &GlobalArgs) -> Result<()> {
    let policy_display = args.policy.display().to_string();
    let policy_source = read_capped(&args.policy, MAX_POLICY_BYTES, "policy file")?;
    let policy_bytes = policy_source.len();

    let mut input_bytes = 0usize;
    let input = match &args.input {
        Some(path) => {
            let raw = read_capped(path, MAX_INPUT_BYTES, "input file")?;
            input_bytes = raw.len();
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

    let eval_start = std::time::Instant::now();
    let result = engine
        .eval(&input, &args.query)
        .with_context(|| format!("evaluating query `{}`", args.query))?;
    let eval_elapsed = eval_start.elapsed();

    // Decide between findings semantics and a raw value by *shape*, via
    // `post_process` (a findings array parses; anything else does not). This
    // keeps legitimate non-findings queries (`data.pkg.list`, a scalar, an
    // object) working as raw-value evaluations instead of erroring.
    let raw_value = result.value.clone();
    let raw_for_pp = raw_value.clone().unwrap_or(serde_json::Value::Null);
    let opts = PostProcessOptions {
        fail_on_warnings: args.fail_on_warnings,
    };

    // An array containing *any* object is intended as findings: a list of
    // scalars (`["a", "b"]`) is a legitimate non-findings value, but the moment
    // an object appears the result is findings-shaped. Using `any` (not `all`)
    // closes the bypass where a malformed-findings policy smuggles a non-object
    // element (e.g. `[{…}, null]`) to dodge the gate.
    let looks_like_findings = matches!(
        &raw_for_pp,
        serde_json::Value::Array(items) if items.iter().any(serde_json::Value::is_object)
    );

    let (findings, exit_code, value) =
        match anvil_policy_engine::result::post_process(&raw_for_pp, &input, opts) {
            // Findings-shaped: `findings` is canonical, so drop the raw array.
            Ok(report) => (report.findings, report.exit_code, None),
            // A findings-shaped array that fails post-processing (a finding
            // missing `message`, a bad severity, a stray non-object element) is
            // a broken policy — hard error always, even without
            // `--fail-on-warnings`, so it can never silently pass a gate. Any
            // error variant counts, so a future `ResultError` can't reopen the
            // silent-pass hole.
            Err(e) if looks_like_findings => {
                // CIB-017: a post-processing failure is a gate-relevant eval
                // failure — surface it as a structured event, not just an anyhow
                // chain. The `policy`/`query` span fields give context; the full
                // reason (which can embed the rendered JSON value) stays in the
                // returned `e`, not the log line, so the event stays bounded.
                tracing::warn!("policy eval failed: malformed findings array");
                return Err(e).with_context(|| {
                    format!("query `{}` returned a malformed findings array", args.query)
                });
            }
            // Not findings-shaped, but the user is gating on it — fail loudly
            // rather than silently passing (exit 0) on a non-findings query.
            Err(e) if args.fail_on_warnings => {
                tracing::warn!("policy eval failed: --fail-on-warnings on a non-findings query");
                return Err(e).with_context(|| {
                    format!(
                        "query `{}` is not a findings query; --fail-on-warnings needs one",
                        args.query
                    )
                });
            }
            // Legitimately non-findings (a list of scalars, a scalar, an
            // object): surface the raw value, no gating.
            Err(_) => (Vec::new(), 0, raw_value),
        };

    // Structured eval summary for operators (CIB-017). Quiet by default; opt in
    // with `ANVIL_LOG=debug` (or `RUST_LOG=debug`; ANVIL_LOG wins if both set).
    // Fields, not an anyhow chain, so a CI/prod failure is diagnosable.
    let eval_ms = u64::try_from(eval_elapsed.as_millis()).unwrap_or(u64::MAX);
    tracing::debug!(
        policy_bytes,
        input_bytes,
        eval_ms,
        findings = findings.len(),
        exit_code,
        "policy eval complete"
    );

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
