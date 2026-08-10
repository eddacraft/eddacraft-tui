//! Tier B on-demand agent runner.
//!
//! Headless driver decision (DEVACC-007): **custom MCP host protocol** with a
//! built-in `dry-run` mode. Live model drivers attach via env-configured
//! external command; they are never the default and require credentials.
//!
//! ```text
//! ANVIL_DEVACC_DRIVER=dry-run|external
//! ANVIL_DEVACC_EXTERNAL_CMD=...   # when driver=external
//! ANVIL_DEVACC_MODEL=...
//! ANVIL_DEVACC_N=10
//! ```

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use super::catalogue::load_catalogue;
use super::report::DevaccReport;
use super::resolve_repo_root;
use super::runner_a::{RunTierAOptions, run_tier_a};

#[derive(Debug, Clone)]
pub struct RunTierBOptions {
    pub repo_root: Option<PathBuf>,
    pub scenario_filter: Option<String>,
    pub arm_filter: Option<String>,
    pub out_dir: Option<PathBuf>,
    /// When true (default for smoke), emit dry-run records without a model.
    pub dry_run: bool,
}

pub fn run_tier_b(opts: &RunTierBOptions) -> Result<Vec<DevaccReport>, String> {
    // Explicit env wins; otherwise --live selects external and --dry-run selects dry-run.
    let driver = match std::env::var("ANVIL_DEVACC_DRIVER") {
        Ok(v) if !v.is_empty() => v,
        _ if opts.dry_run => "dry-run".into(),
        _ => "external".into(),
    };

    match driver.as_str() {
        "dry-run" => run_dry(opts),
        "external" => run_external(opts),
        other => Err(format!(
            "unknown ANVIL_DEVACC_DRIVER={other}; expected dry-run|external"
        )),
    }
}

/// Dry-run: reuses Tier A scripts where present to produce schema-valid Tier B
/// *shaped* records labelled dry-run. SCN-40 has no Tier A script — emits a
/// composite dry-run from SCN-01 + SCN-10 + SCN-32 token sums as a scaffold.
fn run_dry(opts: &RunTierBOptions) -> Result<Vec<DevaccReport>, String> {
    let root = resolve_repo_root(opts.repo_root.as_deref())?;
    let cat = load_catalogue(&root.join("benchmarks/devacc/catalogue.yaml"))?;
    let model = std::env::var("ANVIL_DEVACC_MODEL").ok();

    let mut out = Vec::new();
    for sc in &cat.scenarios {
        if !sc.tiers.iter().any(|t| t == "B") {
            continue;
        }
        if let Some(ref f) = opts.scenario_filter
            && &sc.id != f
            && !sc.id.ends_with(f.as_str())
        {
            continue;
        }

        if sc.id == "DEVACC-SCN-40" {
            out.extend(dry_scn40(&root, sc.arms.as_slice(), model.as_deref())?);
            continue;
        }

        // Prefer Tier A scripted ceiling as dry-run scaffold for paired arms.
        if sc.tiers.iter().any(|t| t == "A") {
            for arm in &sc.arms {
                if let Some(ref af) = opts.arm_filter
                    && arm != af
                {
                    continue;
                }
                if !sc.tier_a_scripts.contains_key(arm) {
                    continue;
                }
                let mut a_reports = run_tier_a(&RunTierAOptions {
                    repo_root: Some(root.clone()),
                    scenario_filter: Some(sc.id.clone()),
                    arm_filter: Some(arm.clone()),
                    out_dir: None,
                })?;
                for mut r in a_reports.drain(..) {
                    r.tier = "B".into();
                    r.model = model.clone().or_else(|| Some("dry-run".into()));
                    r.notes = Some(format!(
                        "dry-run scaffold from Tier A script; not live agent evidence. {}",
                        r.notes.unwrap_or_default()
                    ));
                    r.validate_shape()?;
                    out.push(r);
                }
            }
        }
    }

    if out.is_empty() {
        return Err("no Tier B scenarios matched filters".into());
    }

    if let Some(ref dir) = opts.out_dir {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let path = dir.join("tier-b-dry-run.json");
        let json = serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

fn dry_scn40(
    root: &std::path::Path,
    arms: &[String],
    model: Option<&str>,
) -> Result<Vec<DevaccReport>, String> {
    let mut reports = Vec::new();
    for arm in arms {
        // Composite: sum navigate + edit + guard tax from Tier A
        let mut total = 0u64;
        let mut tools = 0u64;
        for scn in ["DEVACC-SCN-01", "DEVACC-SCN-10", "DEVACC-SCN-32"] {
            let arm_for = if scn == "DEVACC-SCN-01" && arm == "full-accel" {
                "full-accel"
            } else if scn == "DEVACC-SCN-01" && arm == "control" {
                "control"
            } else {
                arm.as_str()
            };
            // SCN-01 full-accel uses same script as gctx-only
            let arm_key = if scn == "DEVACC-SCN-01" && arm_for == "full-accel" {
                "gctx-only"
            } else {
                arm_for
            };
            let part = run_tier_a(&RunTierAOptions {
                repo_root: Some(root.to_path_buf()),
                scenario_filter: Some(scn.into()),
                arm_filter: Some(arm_key.into()),
                out_dir: None,
            })?;
            if let Some(r) = part.first() {
                total += r.tokens_total;
                tools += r.tool_calls;
            }
        }
        let mut r = DevaccReport::new_base("DEVACC-SCN-40", arm, "B");
        r.model = Some(model.unwrap_or("dry-run").into());
        r.task_success = true;
        r.rubric_score = 1.0;
        r.tokens_total = total;
        r.tokens_in = total;
        r.tokens_tool_results = total;
        r.tool_calls = tools;
        r.turns = 5;
        r.wall_ms = 0;
        r.notes =
            Some("dry-run composite scaffold (SCN-01+10+32); not publishable hero evidence".into());
        r.validate_shape()?;
        reports.push(r);
    }
    Ok(reports)
}

fn run_external(opts: &RunTierBOptions) -> Result<Vec<DevaccReport>, String> {
    let cmd = std::env::var("ANVIL_DEVACC_EXTERNAL_CMD").map_err(|_| {
        "ANVIL_DEVACC_DRIVER=external requires ANVIL_DEVACC_EXTERNAL_CMD".to_string()
    })?;
    let model = std::env::var("ANVIL_DEVACC_MODEL")
        .map_err(|_| "ANVIL_DEVACC_DRIVER=external requires ANVIL_DEVACC_MODEL".to_string())?;
    let root = resolve_repo_root(opts.repo_root.as_deref())?;
    let start = Instant::now();

    let mut command = Command::new("sh");
    command.arg("-c").arg(&cmd);
    command.env("ANVIL_DEVACC_MODEL", &model);
    command.env("ANVIL_DEVACC_REPO", root.as_os_str());
    if let Some(ref sc) = opts.scenario_filter {
        command.env("ANVIL_DEVACC_SCENARIO", sc);
    }
    if let Some(ref arm) = opts.arm_filter {
        command.env("ANVIL_DEVACC_ARM", arm);
    }
    if let Some(ref out) = opts.out_dir {
        command.env("ANVIL_DEVACC_OUT", out.as_os_str());
        fs::create_dir_all(out).map_err(|e| e.to_string())?;
    }

    let status = command
        .status()
        .map_err(|e| format!("failed to spawn external driver: {e}"))?;
    if !status.success() {
        return Err(format!("external driver exited {status}"));
    }

    // Expect JSON array at $ANVIL_DEVACC_OUT/external-results.json
    let out = opts
        .out_dir
        .clone()
        .unwrap_or_else(|| root.join("benchmark-results/devacc-external"));
    let path = out.join("external-results.json");
    if !path.is_file() {
        return Err(format!("external driver did not write {}", path.display()));
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut reports: Vec<DevaccReport> =
        serde_json::from_str(&text).map_err(|e| format!("parse external results: {e}"))?;
    for r in &mut reports {
        if r.model.is_none() {
            r.model = Some(model.clone());
        }
        r.wall_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        r.validate_shape()?;
    }
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devacc_tier_b_dry_run_smoke() {
        let reports = run_tier_b(&RunTierBOptions {
            repo_root: None,
            scenario_filter: Some("DEVACC-SCN-01".into()),
            arm_filter: None,
            out_dir: None,
            dry_run: true,
        })
        .expect("dry-run");
        assert!(!reports.is_empty());
        assert!(reports.iter().all(|r| r.tier == "B"));
        assert!(reports.iter().all(|r| r.model.is_some()));
    }

    #[test]
    fn devacc_tier_b_scn40_dry_run() {
        let reports = run_tier_b(&RunTierBOptions {
            repo_root: None,
            scenario_filter: Some("DEVACC-SCN-40".into()),
            arm_filter: None,
            out_dir: None,
            dry_run: true,
        })
        .expect("scn40");
        assert!(reports.iter().any(|r| r.scenario == "DEVACC-SCN-40"));
        assert!(reports.iter().all(|r| {
            r.notes
                .as_deref()
                .is_some_and(|n| n.contains("not publishable"))
        }));
    }
}
