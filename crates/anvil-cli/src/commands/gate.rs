use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::Args;
use regex::Regex;
use serde::Serialize;

use crate::GlobalArgs;

#[derive(Debug, Args)]
#[allow(clippy::struct_excessive_bools)]
pub struct GateArgs {
    /// Plan file to run gates against (omit for full codebase scan)
    #[allow(dead_code)] // scaffold: plan-scoped gating not yet wired
    plan: Option<String>,

    /// Gate profile: dev, ci, production
    #[arg(long, short)]
    profile: Option<String>,

    /// Comma-separated list of checks to skip
    #[arg(long)]
    skip_checks: Option<String>,

    /// Only run specified checks (comma-separated)
    #[arg(long)]
    only_checks: Option<String>,

    /// Stop on first check failure
    #[arg(long)]
    fail_fast: bool,

    /// Show real-time progress
    #[arg(long)]
    progress: bool,

    /// List available gate profiles
    #[arg(long)]
    list_profiles: bool,

    /// Disable caching
    #[arg(long)]
    #[allow(dead_code)] // scaffold: cache bypass not yet wired
    no_cache: bool,
}

const PROFILES: &[(&str, &str, &[&str])] = &[
    (
        "dev",
        "Development mode \u{2014} skips coverage and dependency checks",
        &["coverage", "dependency"],
    ),
    ("ci", "CI mode \u{2014} runs all checks", &[]),
    (
        "production",
        "Production mode \u{2014} runs all checks with strict thresholds",
        &[],
    ),
];

const AVAILABLE_CHECKS: &[&str] = &[
    "lint",
    "test",
    "coverage",
    "dependency",
    "secret",
    "architecture",
    "policy",
];

#[derive(Debug, Serialize)]
struct GateResult {
    overall: bool,
    score: f64,
    checks: Vec<CheckResult>,
    duration_ms: u64,
}

#[derive(Debug, Serialize)]
struct CheckResult {
    name: String,
    passed: bool,
    score: f64,
    message: String,
}

/// Best-effort workspace root detection via `git rev-parse`.
fn workspace_root() -> PathBuf {
    std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            let s = String::from_utf8(o.stdout).ok()?;
            Some(PathBuf::from(s.trim()))
        })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn run_check_lint(name: &str) -> CheckResult {
    let root = workspace_root();
    let output = std::process::Command::new("pnpm")
        .args(["lint:check"])
        .current_dir(&root)
        .output();
    match output {
        Ok(o) if o.status.success() => CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: "No lint errors".to_string(),
        },
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            CheckResult {
                name: name.to_string(),
                passed: false,
                score: 0.0,
                message: format!("Lint errors found\n{stdout}\n{stderr}"),
            }
        }
        Err(e) => CheckResult {
            name: name.to_string(),
            passed: false,
            score: 0.0,
            message: format!("Failed to run lint: {e}"),
        },
    }
}

fn run_check_test(name: &str) -> CheckResult {
    let root = workspace_root();
    let output = std::process::Command::new("pnpm")
        .args(["test"])
        .current_dir(&root)
        .output();
    match output {
        Ok(o) if o.status.success() => CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: "All tests passed".to_string(),
        },
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            CheckResult {
                name: name.to_string(),
                passed: false,
                score: 0.0,
                message: format!("Tests failed\n{stdout}\n{stderr}"),
            }
        }
        Err(e) => CheckResult {
            name: name.to_string(),
            passed: false,
            score: 0.0,
            message: format!("Failed to run tests: {e}"),
        },
    }
}

const SECRET_SCAN_IGNORE: &[&str] = &[
    "node_modules",
    "dist",
    "build",
    "target",
    ".git",
    ".anvil",
    "coverage",
];

fn run_check_secret(name: &str) -> CheckResult {
    let root = workspace_root();
    let mut found = Vec::new();
    let secret_patterns: Vec<Regex> = [
        r"AKIA[0-9A-Z]{16}",
        r"sk-[a-zA-Z0-9]{48}",
        r"ghp_[a-zA-Z0-9]{36}",
    ]
    .iter()
    .filter_map(|p| Regex::new(p).ok())
    .collect();

    for entry in walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !SECRET_SCAN_IGNORE.iter().any(|&ig| name == ig)
        })
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let file_name = path.file_name().map(|f| f.to_string_lossy());

        // Check extension-based files and dotfiles like .env*
        let scannable = if let Some(ref fname) = file_name {
            fname.starts_with(".env")
        } else {
            false
        } || path.extension().is_some_and(|ext| {
            let ext_str = ext.to_string_lossy();
            matches!(
                &*ext_str,
                "ts" | "js" | "rs" | "json" | "yaml" | "yml" | "toml" | "env"
            )
        });

        if !scannable {
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(path) {
            for (line_no, line) in content.lines().enumerate() {
                for re in &secret_patterns {
                    if re.is_match(line) {
                        found.push(format!("{}:{}", path.display(), line_no + 1));
                    }
                }
            }
        }
    }
    if found.is_empty() {
        CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: "No hardcoded secrets found".to_string(),
        }
    } else {
        CheckResult {
            name: name.to_string(),
            passed: false,
            score: 0.0,
            message: format!(
                "Potential secrets found in {} location(s):\n{}",
                found.len(),
                found.join("\n")
            ),
        }
    }
}

const DEFAULT_COVERAGE_THRESHOLD: f64 = 80.0;

fn run_check_coverage(project_root: &Path, threshold: f64) -> CheckResult {
    let lcov_path = project_root.join("coverage/lcov.info");
    let cobertura_path = project_root.join("coverage/cobertura.xml");

    if lcov_path.exists() {
        match std::fs::read_to_string(&lcov_path) {
            Ok(content) => {
                let mut total_lines: u64 = 0;
                let mut hit_lines: u64 = 0;
                for line in content.lines() {
                    if let Some(val) = line.strip_prefix("LF:") {
                        if let Ok(n) = val.trim().parse::<u64>() {
                            total_lines += n;
                        }
                    } else if let Some(val) = line.strip_prefix("LH:")
                        && let Ok(n) = val.trim().parse::<u64>()
                    {
                        hit_lines += n;
                    }
                }
                if total_lines == 0 {
                    return CheckResult {
                        name: "coverage".to_string(),
                        passed: true,
                        score: 100.0,
                        message: "Coverage report empty (no lines tracked). Skipping.".to_string(),
                    };
                }
                #[allow(clippy::cast_precision_loss)]
                let pct = (hit_lines as f64 / total_lines as f64) * 100.0;
                let passed = pct >= threshold;
                CheckResult {
                    name: "coverage".to_string(),
                    passed,
                    score: pct,
                    message: format!("Line coverage: {pct:.1}% (threshold: {threshold:.0}%)"),
                }
            }
            Err(e) => CheckResult {
                name: "coverage".to_string(),
                passed: false,
                score: 0.0,
                message: format!("Failed to read lcov.info: {e}"),
            },
        }
    } else if cobertura_path.exists() {
        match std::fs::read_to_string(&cobertura_path) {
            Ok(content) => {
                // Extract line-rate="X.XX" attribute from cobertura XML
                let rate = Regex::new(r#"line-rate="([0-9.]+)""#)
                    .ok()
                    .and_then(|re| re.captures(&content))
                    .and_then(|cap| cap.get(1))
                    .and_then(|m| m.as_str().parse::<f64>().ok());
                match rate {
                    Some(r) => {
                        let pct = r * 100.0;
                        let passed = pct >= threshold;
                        CheckResult {
                            name: "coverage".to_string(),
                            passed,
                            score: pct,
                            message: format!(
                                "Line coverage: {pct:.1}% (threshold: {threshold:.0}%)"
                            ),
                        }
                    }
                    None => CheckResult {
                        name: "coverage".to_string(),
                        passed: false,
                        score: 0.0,
                        message: "Failed to parse line-rate from cobertura.xml".to_string(),
                    },
                }
            }
            Err(e) => CheckResult {
                name: "coverage".to_string(),
                passed: false,
                score: 0.0,
                message: format!("Failed to read cobertura.xml: {e}"),
            },
        }
    } else {
        CheckResult {
            name: "coverage".to_string(),
            passed: true,
            score: 100.0,
            message:
                "No coverage report found (coverage/lcov.info or coverage/cobertura.xml). Skipping."
                    .to_string(),
        }
    }
}

const BLOCKED_NPM_PACKAGES: &[&str] = &[
    "event-stream",
    "flatmap-stream",
    "ua-parser-js",
    "colors",
    "faker",
    "node-ipc",
];

fn run_check_dependency(project_root: &Path) -> CheckResult {
    let npm_lock = project_root.join("package-lock.json");
    let cargo_lock = project_root.join("Cargo.lock");

    let has_npm = npm_lock.exists();
    let has_cargo = cargo_lock.exists();

    if !has_npm && !has_cargo {
        return CheckResult {
            name: "dependency".to_string(),
            passed: true,
            score: 100.0,
            message: "No lockfile found (package-lock.json or Cargo.lock). Skipping.".to_string(),
        };
    }

    let mut blocked_found: Vec<String> = Vec::new();

    if has_npm {
        match std::fs::read_to_string(&npm_lock) {
            Ok(content) => {
                for pkg in BLOCKED_NPM_PACKAGES {
                    let pattern = format!("\"node_modules/{pkg}\"");
                    if content.contains(&pattern) {
                        blocked_found.push((*pkg).to_string());
                    }
                }
            }
            Err(e) => {
                return CheckResult {
                    name: "dependency".to_string(),
                    passed: false,
                    score: 0.0,
                    message: format!("Failed to read {}: {e}", npm_lock.display()),
                };
            }
        }
    }

    // Cargo.lock scanning can be extended later; for now only npm is checked.

    if blocked_found.is_empty() {
        CheckResult {
            name: "dependency".to_string(),
            passed: true,
            score: 100.0,
            message: "No blocked dependencies found".to_string(),
        }
    } else {
        CheckResult {
            name: "dependency".to_string(),
            passed: false,
            score: 0.0,
            message: format!("Blocked dependencies found: {}", blocked_found.join(", ")),
        }
    }
}

fn run_check_architecture(project_root: &Path) -> CheckResult {
    let config_path = project_root.join(".anvil/architecture.yaml");

    if !config_path.exists() {
        return CheckResult {
            name: "architecture".to_string(),
            passed: true,
            score: 100.0,
            message: "No architecture config found (.anvil/architecture.yaml). Skipping."
                .to_string(),
        };
    }

    let definition = match anvil_architecture::parse_architecture_definition(project_root) {
        Ok(def) => def,
        Err(e) => {
            return CheckResult {
                name: "architecture".to_string(),
                passed: false,
                score: 0.0,
                message: format!("Architecture validation failed: {e}"),
            };
        }
    };

    match anvil_architecture::validate(project_root, &definition) {
        Ok(result) => {
            if result.valid {
                CheckResult {
                    name: "architecture".to_string(),
                    passed: true,
                    score: 100.0,
                    message: "Architecture config is valid".to_string(),
                }
            } else {
                let msgs: Vec<String> = result
                    .violations
                    .iter()
                    .map(|v| {
                        let boundary_name =
                            v.boundary.as_ref().map_or("unknown", |b| b.name.as_str());
                        let message = v
                            .boundary
                            .as_ref()
                            .map_or("boundary violation", |b| b.message.as_str());
                        format!("{}: {} ({})", boundary_name, message, v.edge.from)
                    })
                    .collect();
                CheckResult {
                    name: "architecture".to_string(),
                    passed: false,
                    score: 0.0,
                    message: format!(
                        "{} violation(s):\n{}",
                        result.violations.len(),
                        msgs.join("\n")
                    ),
                }
            }
        }
        Err(e) => CheckResult {
            name: "architecture".to_string(),
            passed: false,
            score: 0.0,
            message: format!("Architecture validation failed: {e}"),
        },
    }
}

fn run_check_policy(project_root: &Path) -> CheckResult {
    let policy_dir = project_root.join(".anvil/policies");

    if !policy_dir.exists() || !policy_dir.is_dir() {
        return CheckResult {
            name: "policy".to_string(),
            passed: true,
            score: 100.0,
            message: "No policy bundle found (.anvil/policies/). Skipping.".to_string(),
        };
    }

    let evaluator = anvil_policy::evaluator::Evaluator::new(None);
    let input = serde_json::json!({});

    match evaluator.evaluate(project_root, &input, None) {
        Ok(result) => {
            if result.passed {
                CheckResult {
                    name: "policy".to_string(),
                    passed: true,
                    score: 100.0,
                    message: format!("{} policies evaluated, no violations", result.checks_run),
                }
            } else {
                let msgs: Vec<String> = result
                    .violations
                    .iter()
                    .map(|v| format!("[{}] {}: {}", v.severity, v.policy_id, v.message))
                    .collect();
                CheckResult {
                    name: "policy".to_string(),
                    passed: false,
                    score: 0.0,
                    message: format!(
                        "{} violation(s):\n{}",
                        result.violations.len(),
                        msgs.join("\n")
                    ),
                }
            }
        }
        Err(anvil_policy::evaluator::EvalError::OpaNotAvailable) => CheckResult {
            name: "policy".to_string(),
            passed: true,
            score: 100.0,
            message: "OPA not installed. Skipping policy evaluation.".to_string(),
        },
        Err(e) => CheckResult {
            name: "policy".to_string(),
            passed: false,
            score: 0.0,
            message: format!("Policy evaluation failed: {e}"),
        },
    }
}

fn run_single_check(name: &str) -> CheckResult {
    let root = workspace_root();
    match name {
        "lint" => run_check_lint(name),
        "test" => run_check_test(name),
        "secret" => run_check_secret(name),
        "coverage" => run_check_coverage(&root, DEFAULT_COVERAGE_THRESHOLD),
        "dependency" => run_check_dependency(&root),
        "architecture" => run_check_architecture(&root),
        "policy" => run_check_policy(&root),
        _ => CheckResult {
            name: name.to_string(),
            passed: false,
            score: 0.0,
            message: format!("Unknown check: {name}"),
        },
    }
}

fn list_profiles() {
    println!();
    println!("Available Gate Profiles");
    println!();
    for (name, desc, skips) in PROFILES {
        println!("  {name}");
        println!("    {desc}");
        if !skips.is_empty() {
            println!("    Skips: {}", skips.join(", "));
        }
        println!();
    }
    println!("Usage: anvil gate [plan] --profile dev");
}

fn resolve_profile_skips(profile: Option<&str>) -> Result<std::collections::HashSet<&str>> {
    let Some(name) = profile else {
        return Ok(std::collections::HashSet::new());
    };
    for (pname, _, skips) in PROFILES {
        if *pname == name {
            return Ok(skips.iter().copied().collect());
        }
    }
    let valid: Vec<&str> = PROFILES.iter().map(|(n, _, _)| *n).collect();
    bail!(
        "unknown profile '{name}', valid profiles: {}",
        valid.join(", ")
    );
}

fn validate_check_names(names: &std::collections::HashSet<&str>) -> Result<()> {
    let unknown: Vec<&&str> = names
        .iter()
        .filter(|n| !AVAILABLE_CHECKS.contains(n))
        .collect();
    if !unknown.is_empty() {
        let unknown_str: Vec<&str> = unknown.into_iter().copied().collect();
        bail!(
            "unknown check(s): {}; available: {}",
            unknown_str.join(", "),
            AVAILABLE_CHECKS.join(", ")
        );
    }
    Ok(())
}

/// Run all gate checks with default settings and return TUI-ready data.
pub fn collect_gate_data() -> anvil_tui::surfaces::gate::GateResult {
    let start = std::time::Instant::now();
    let default_args = GateArgs {
        plan: None,
        profile: None,
        skip_checks: None,
        only_checks: None,
        fail_fast: false,
        progress: false,
        list_profiles: false,
        no_cache: false,
    };
    let checks = run_checks(&default_args).unwrap_or_default();

    let passed_count = checks.iter().filter(|c| c.passed).count();
    let total = checks.len();
    let overall = checks.iter().all(|c| c.passed);
    #[allow(clippy::cast_precision_loss)]
    let score = if total > 0 {
        passed_count as f64 / total as f64
    } else {
        1.0
    };
    let elapsed = start.elapsed().as_millis();

    let tui_checks: Vec<anvil_tui::surfaces::gate::GateCheck> = checks
        .into_iter()
        .map(|c| {
            let status = if c.passed {
                anvil_tui::surfaces::gate::GateCheckStatus::Passed
            } else {
                anvil_tui::surfaces::gate::GateCheckStatus::Failed
            };
            anvil_tui::surfaces::gate::GateCheck {
                id: c.name.clone(),
                name: c.name,
                status,
                score: c.score / 100.0,
                message: c.message,
                details: None,
                file: None,
                line: None,
            }
        })
        .collect();

    anvil_tui::surfaces::gate::GateResult {
        plan_id: "cli".to_string(),
        overall_passed: overall,
        score,
        checks: tui_checks,
        duration_ms: u64::try_from(elapsed).unwrap_or(u64::MAX),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

fn run_checks(args: &GateArgs) -> Result<Vec<CheckResult>> {
    let profile_skips = resolve_profile_skips(args.profile.as_deref())?;

    let mut skip_set: std::collections::HashSet<&str> = args
        .skip_checks
        .as_deref()
        .map(|s| s.split(',').map(str::trim).collect())
        .unwrap_or_default();
    skip_set.extend(&profile_skips);

    let only_set: Option<std::collections::HashSet<&str>> = args
        .only_checks
        .as_deref()
        .map(|s| s.split(',').map(str::trim).collect());

    validate_check_names(&skip_set)?;
    if let Some(ref only_s) = only_set {
        validate_check_names(only_s)?;
    }

    let mut checks = Vec::new();
    for check_name in AVAILABLE_CHECKS {
        if skip_set.contains(check_name) {
            continue;
        }
        if let Some(ref only_s) = only_set
            && !only_s.contains(check_name)
        {
            continue;
        }

        if args.progress {
            eprintln!("  \u{25b6} {check_name} running...");
        }

        let result = run_single_check(check_name);

        if args.progress {
            let icon = if result.passed {
                "\u{2713}"
            } else {
                "\u{2717}"
            };
            eprintln!("  {icon} {check_name}");
        }

        checks.push(result);

        if args.fail_fast && !checks.last().unwrap().passed {
            break;
        }
    }
    Ok(checks)
}

/// Run gate checks and return whether all gates passed.
///
/// Returns `Ok(true)` when every check passes and `Ok(false)` when at
/// least one check fails (caller maps this to `EXIT_GATE_FAIL`).
pub fn run(args: &GateArgs, global: &GlobalArgs) -> Result<bool> {
    use crate::output::OutputMode;

    if args.list_profiles {
        list_profiles();
        return Ok(true);
    }

    let mode = OutputMode::from_global(global);

    let start = std::time::Instant::now();
    let checks = run_checks(args)?;

    let passed_count = checks.iter().filter(|c| c.passed).count();
    let total = checks.len();
    let overall = checks.iter().all(|c| c.passed);
    let score = if total > 0 {
        #[allow(clippy::cast_precision_loss)]
        {
            (passed_count as f64 / total as f64) * 100.0
        }
    } else {
        100.0
    };

    let elapsed = start.elapsed().as_millis();
    let result = GateResult {
        overall,
        score,
        checks,
        duration_ms: u64::try_from(elapsed).unwrap_or(u64::MAX),
    };

    match mode {
        OutputMode::Json => {
            crate::output::json::print(&result)?;
        }
        OutputMode::Plain | OutputMode::Tui => {
            // TUI surface for gate is not yet implemented; fall back to plain.
            use crate::output::plain;

            plain::header("Gate Results");
            plain::section("Checks");
            for check in &result.checks {
                if check.passed {
                    plain::success(&format!("{:<20} PASS", check.name));
                } else {
                    plain::error(&format!("{:<20} FAIL", check.name));
                }
                if !check.message.is_empty() && (global.verbose || !check.passed) {
                    for line in check.message.lines() {
                        plain::dim(&format!("  {line}"));
                    }
                }
            }
            plain::blank();
            if overall {
                plain::success(&format!(
                    "All quality gates passed! (score: {:.0}%)",
                    result.score,
                ));
            } else {
                plain::error(&format!(
                    "Quality gates failed ({passed_count}/{total} passed, score: {:.0}%)",
                    result.score,
                ));
            }
        }
    }

    Ok(overall)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Wrapper {
        #[command(flatten)]
        inner: GateArgs,
    }

    #[test]
    fn args_parses_empty() {
        let w = Wrapper::try_parse_from(["test"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_with_plan() {
        let w = Wrapper::try_parse_from(["test", "plan.aps.md"]).unwrap();
        assert_eq!(w.inner.plan.as_deref(), Some("plan.aps.md"));
    }

    #[test]
    fn args_parses_profile() {
        let w = Wrapper::try_parse_from(["test", "--profile", "dev"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_list_profiles() {
        let w = Wrapper::try_parse_from(["test", "--list-profiles"]).unwrap();
        assert!(w.inner.list_profiles);
    }

    #[test]
    fn resolve_profile_dev_skips_coverage_and_dependency() {
        let skips = resolve_profile_skips(Some("dev")).unwrap();
        assert!(skips.contains("coverage"));
        assert!(skips.contains("dependency"));
    }

    #[test]
    fn resolve_profile_unknown_errors() {
        assert!(resolve_profile_skips(Some("bogus")).is_err());
    }

    #[test]
    fn resolve_profile_none_returns_empty() {
        let skips = resolve_profile_skips(None).unwrap();
        assert!(skips.is_empty());
    }

    // ── Coverage check tests ──────────────────────────────────────────

    #[test]
    fn coverage_no_report_skips() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = run_check_coverage(tmp.path(), 80.0);
        assert!(result.passed);
        assert!(result.message.contains("Skipping"));
    }

    #[test]
    fn coverage_lcov_above_threshold() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cov_dir = tmp.path().join("coverage");
        std::fs::create_dir_all(&cov_dir).unwrap();
        std::fs::write(
            cov_dir.join("lcov.info"),
            "SF:src/main.rs\nLF:100\nLH:90\nend_of_record\n",
        )
        .unwrap();
        let result = run_check_coverage(tmp.path(), 80.0);
        assert!(result.passed);
        assert!(result.message.contains("90.0%"));
    }

    #[test]
    fn coverage_lcov_below_threshold() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cov_dir = tmp.path().join("coverage");
        std::fs::create_dir_all(&cov_dir).unwrap();
        std::fs::write(
            cov_dir.join("lcov.info"),
            "SF:src/main.rs\nLF:100\nLH:50\nend_of_record\n",
        )
        .unwrap();
        let result = run_check_coverage(tmp.path(), 80.0);
        assert!(!result.passed);
        assert!(result.message.contains("50.0%"));
    }

    #[test]
    fn coverage_cobertura_above_threshold() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cov_dir = tmp.path().join("coverage");
        std::fs::create_dir_all(&cov_dir).unwrap();
        std::fs::write(
            cov_dir.join("cobertura.xml"),
            r#"<?xml version="1.0"?><coverage line-rate="0.95"></coverage>"#,
        )
        .unwrap();
        let result = run_check_coverage(tmp.path(), 80.0);
        assert!(result.passed);
        assert!(result.message.contains("95.0%"));
    }

    // ── Dependency check tests ──────────────────────────────────────

    #[test]
    fn dependency_no_lockfile_skips() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = run_check_dependency(tmp.path());
        assert!(result.passed);
        assert!(result.message.contains("Skipping"));
    }

    #[test]
    fn dependency_clean_lockfile_passes() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("package-lock.json"),
            r#"{"lockfileVersion":3,"packages":{"node_modules/express":{}}}"#,
        )
        .unwrap();
        let result = run_check_dependency(tmp.path());
        assert!(result.passed);
        assert!((result.score - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn dependency_blocked_package_fails() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("package-lock.json"),
            r#"{"lockfileVersion":3,"packages":{"node_modules/event-stream":{"version":"4.0.1"}}}"#,
        )
        .unwrap();
        let result = run_check_dependency(tmp.path());
        assert!(!result.passed);
        assert!(result.message.contains("event-stream"));
    }

    // ── Architecture check tests ────────────────────────────────────

    #[test]
    fn architecture_no_config_skips() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = run_check_architecture(tmp.path());
        assert!(result.passed);
        assert!(result.message.contains("Skipping"));
    }

    #[test]
    fn architecture_valid_config_passes() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(
            anvil_dir.join("architecture.yaml"),
            "boundaries:\n  - name: core\n    path: src/core\n",
        )
        .unwrap();
        let result = run_check_architecture(tmp.path());
        assert!(result.passed);
    }

    #[test]
    fn architecture_invalid_yaml_fails() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(anvil_dir.join("architecture.yaml"), "bad: [unclosed").unwrap();
        let result = run_check_architecture(tmp.path());
        assert!(!result.passed);
    }

    // ── Policy check tests ──────────────────────────────────────────

    #[test]
    fn policy_no_bundle_skips() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = run_check_policy(tmp.path());
        assert!(result.passed);
        assert!(result.message.contains("Skipping"));
    }

    #[test]
    fn policy_with_bundle_but_no_opa_skips() {
        let tmp = tempfile::TempDir::new().unwrap();
        let policy_dir = tmp.path().join(".anvil/policies");
        std::fs::create_dir_all(&policy_dir).unwrap();
        std::fs::write(policy_dir.join("test.rego"), "package test\n").unwrap();
        let result = run_check_policy(tmp.path());
        // OPA is not installed in test environment, so it should skip gracefully
        assert!(result.passed);
        assert!(
            result.message.contains("OPA not installed")
                || result.message.contains("policies evaluated")
        );
    }
}
