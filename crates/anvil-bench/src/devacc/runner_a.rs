//! Tier A deterministic scripted tool-trace runner (no LLM).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anvil_graph_cache::estimate_gctx_tokens;
use serde::Deserialize;
use serde_json::Value;

use super::catalogue::{ScenarioDef, load_catalogue};
use super::fixture::{fixture_root, list_relative, read_fixture_file, synthetic_content};
use super::report::DevaccReport;
use super::resolve_repo_root;

#[derive(Debug, Clone)]
pub struct RunTierAOptions {
    pub repo_root: Option<PathBuf>,
    pub scenario_filter: Option<String>,
    pub arm_filter: Option<String>,
    pub out_dir: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct ToolScript {
    steps: Vec<Value>,
}

struct Accumulators {
    tool_result_tokens: u64,
    file_read_tokens: u64,
    gctx_tokens: u64,
    gold_ok: bool,
    expected_block: Option<bool>,
}

impl Default for Accumulators {
    fn default() -> Self {
        Self {
            tool_result_tokens: 0,
            file_read_tokens: 0,
            gctx_tokens: 0,
            gold_ok: true,
            expected_block: None,
        }
    }
}

pub fn run_tier_a(opts: &RunTierAOptions) -> Result<Vec<DevaccReport>, String> {
    let root = resolve_repo_root(opts.repo_root.as_deref())?;
    let cat = load_catalogue(&root.join("benchmarks/devacc/catalogue.yaml"))?;
    let scripts_root = root.join("benchmarks/devacc");

    let mut reports = Vec::new();
    for sc in &cat.scenarios {
        if !sc.tiers.iter().any(|t| t == "A") {
            continue;
        }
        if !scenario_matches(sc, opts.scenario_filter.as_deref()) {
            continue;
        }
        for arm in &sc.arms {
            if let Some(af) = opts.arm_filter.as_deref()
                && arm != af
            {
                continue;
            }
            let Some(script_rel) = sc.tier_a_scripts.get(arm) else {
                continue;
            };
            let script_path = scripts_root.join(script_rel);
            reports.push(execute_script(&root, sc, arm, &script_path)?);
        }
    }

    if reports.is_empty() {
        return Err("no Tier A scenarios matched filters".into());
    }
    if let Some(out) = &opts.out_dir {
        write_reports(out, &reports)?;
    }
    Ok(reports)
}

fn scenario_matches(sc: &ScenarioDef, filter: Option<&str>) -> bool {
    match filter {
        None => true,
        Some(f) => sc.id == f || sc.id.ends_with(f),
    }
}

fn execute_script(
    root: &Path,
    sc: &ScenarioDef,
    arm: &str,
    script_path: &Path,
) -> Result<DevaccReport, String> {
    let start = Instant::now();
    let text = fs::read_to_string(script_path)
        .map_err(|e| format!("read script {}: {e}", script_path.display()))?;
    let script: ToolScript =
        serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", script_path.display()))?;

    let fixture = fixture_root(root, &sc.fixture);
    if !fixture.is_dir() {
        return Err(format!("fixture missing: {}", fixture.display()));
    }

    let mut report = DevaccReport::new_base(&sc.id, arm, "A");
    report.label.clone_from(&sc.label);
    let mut acc = Accumulators::default();

    for step in &script.steps {
        apply_step(step, &fixture, arm, &mut report, &mut acc, script_path)?;
    }

    finalise_report(&mut report, &acc, start, script_path)?;
    Ok(report)
}

fn apply_step(
    step: &Value,
    fixture: &Path,
    arm: &str,
    report: &mut DevaccReport,
    acc: &mut Accumulators,
    script_path: &Path,
) -> Result<(), String> {
    let tool = step
        .get("tool")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("step missing tool in {}", script_path.display()))?;
    report.tool_calls += 1;

    match tool {
        "list_dir" => {
            let path = step["path"].as_str().unwrap_or(".");
            let payload = list_relative(fixture, path)?.join("\n");
            acc.tool_result_tokens += estimate_tokens(&payload)?;
        }
        "read_file" => {
            let path = step["path"].as_str().ok_or("read_file missing path")?;
            let content = read_fixture_file(fixture, path)?;
            let tok = estimate_tokens(&content)?;
            acc.tool_result_tokens += tok;
            acc.file_read_tokens += tok;
            report.file_reads += 1;
        }
        "gctx_payload" => {
            let payload_rel = step["payload"]
                .as_str()
                .ok_or("gctx_payload missing payload")?;
            let content = read_fixture_file(fixture, payload_rel)?;
            let tok = estimate_tokens(&content)?;
            acc.tool_result_tokens += tok;
            acc.gctx_tokens += tok;
            report.gctx_calls += 1;
        }
        "validate_write" => {
            report.validate_calls += 1;
            let decision = step["decision"].as_str().unwrap_or("allow");
            if decision == "block" {
                report.blocked_writes += 1;
            }
            let resp = format!(r#"{{"decision":"{decision}"}}"#);
            acc.tool_result_tokens += estimate_tokens(&resp)?;
        }
        "propose_write" => {
            let key = step["content_key"].as_str().unwrap_or("");
            let body = synthetic_content(key);
            acc.tool_result_tokens += estimate_tokens(body)?;
            if !step["lands"].as_bool().unwrap_or(true) {
                report.blocked_writes += 1;
            }
        }
        "assert_gold" => {
            apply_assert_gold(step, fixture, arm, acc)?;
        }
        other => {
            return Err(format!(
                "unknown tool {other} in {}",
                script_path.display()
            ));
        }
    }
    Ok(())
}

fn apply_assert_gold(
    step: &Value,
    fixture: &Path,
    arm: &str,
    acc: &mut Accumulators,
) -> Result<(), String> {
    let gold_rel = step["gold"].as_str().ok_or("assert_gold missing gold")?;
    let gold_path = fixture.join(gold_rel);
    if !gold_path.is_file() {
        acc.gold_ok = false;
        return Ok(());
    }
    let gold_text = fs::read_to_string(&gold_path).map_err(|e| e.to_string())?;
    acc.tool_result_tokens += estimate_tokens(&gold_text)?;
    let Ok(v) = serde_json::from_str::<Value>(&gold_text) else {
        return Ok(());
    };
    if let Some(b) = v.get("expect_block_full_accel").and_then(Value::as_bool)
        && (arm == "full-accel" || arm == "validate-only")
    {
        acc.expected_block = Some(b);
    }
    if let Some(b) = v.get("expect_block_control").and_then(Value::as_bool)
        && arm == "control"
    {
        acc.expected_block = Some(b);
    }
    if v.get("expect_block").and_then(Value::as_bool) == Some(false) {
        acc.expected_block = Some(false);
    }
    Ok(())
}

fn finalise_report(
    report: &mut DevaccReport,
    acc: &Accumulators,
    start: Instant,
    script_path: &Path,
) -> Result<(), String> {
    let mut success = acc.gold_ok;
    if let Some(expect_block) = acc.expected_block {
        let did_block = report.blocked_writes > 0;
        if expect_block && !did_block {
            success = false;
        }
        if !expect_block && did_block {
            success = false;
            report.false_block_rate = Some(1.0);
        } else if !expect_block {
            report.false_block_rate = Some(0.0);
        }
    }

    report.task_success = success;
    report.rubric_score = if success { 1.0 } else { 0.0 };
    report.tokens_tool_results = acc.tool_result_tokens;
    report.tokens_file_reads = acc.file_read_tokens;
    report.tokens_gctx = acc.gctx_tokens;
    report.tokens_in = acc.tool_result_tokens;
    report.tokens_out = 0;
    report.tokens_total = acc.tool_result_tokens;
    report.wall_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    report.turns = 1;
    report.notes = Some(format!("script={}", script_path.display()));
    report.validate_shape()
}

fn estimate_tokens(text: &str) -> Result<u64, String> {
    let est = estimate_gctx_tokens(text, None).map_err(|e| e.to_string())?;
    Ok(u64::try_from(est.tokens).unwrap_or(u64::MAX))
}

fn write_reports(out_dir: &Path, reports: &[DevaccReport]) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    for (i, r) in reports.iter().enumerate() {
        let name = format!(
            "{:03}_{}_{}_{}.json",
            i,
            r.scenario.replace(':', "-"),
            r.arm,
            r.tier
        );
        let path = out_dir.join(name);
        let json = serde_json::to_string_pretty(r).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())?;
    }
    let index = serde_json::to_string_pretty(reports).map_err(|e| e.to_string())?;
    fs::write(out_dir.join("index.json"), index).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::report::token_reduction_vs_control;
    use super::*;

    #[test]
    fn devacc_scn_navigate_tier_a() {
        let reports = run_tier_a(&RunTierAOptions {
            repo_root: None,
            scenario_filter: Some("DEVACC-SCN-01".into()),
            arm_filter: None,
            out_dir: None,
        })
        .expect("run scn-01");
        assert!(reports.len() >= 2);
        let control = reports.iter().find(|r| r.arm == "control").unwrap();
        let gctx = reports.iter().find(|r| r.arm == "gctx-only").unwrap();
        assert!(control.task_success);
        assert!(gctx.task_success);
        assert!(
            gctx.tokens_total < control.tokens_total,
            "gctx {} should be < control {}",
            gctx.tokens_total,
            control.tokens_total
        );
        let red = token_reduction_vs_control(control, gctx).unwrap();
        assert!(red > 0.0, "expected positive reduction, got {red}");
    }

    #[test]
    fn devacc_scn_guard_blocks_secret() {
        let reports = run_tier_a(&RunTierAOptions {
            repo_root: None,
            scenario_filter: Some("DEVACC-SCN-30".into()),
            arm_filter: None,
            out_dir: None,
        })
        .expect("run scn-30");
        let control = reports.iter().find(|r| r.arm == "control").unwrap();
        let accel = reports.iter().find(|r| r.arm == "full-accel").unwrap();
        assert!(control.task_success);
        assert!(accel.task_success);
        assert_eq!(control.blocked_writes, 0);
        assert!(accel.blocked_writes > 0);
    }

    #[test]
    fn devacc_scn_edit_ceiling() {
        let reports = run_tier_a(&RunTierAOptions {
            repo_root: None,
            scenario_filter: Some("DEVACC-SCN-10".into()),
            arm_filter: None,
            out_dir: None,
        })
        .expect("run scn-10");
        assert!(reports.iter().all(|r| r.task_success));
        assert!(reports.iter().any(|r| r.label.as_deref() == Some("ceiling")));
    }
}
