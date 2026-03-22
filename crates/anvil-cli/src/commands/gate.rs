use std::path::PathBuf;

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

fn run_single_check(name: &str) -> CheckResult {
    match name {
        "lint" => run_check_lint(name),
        "test" => run_check_test(name),
        "secret" => run_check_secret(name),
        _ => CheckResult {
            name: name.to_string(),
            passed: false,
            score: 0.0,
            message: "Check not yet implemented in Rust CLI".to_string(),
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

pub fn run(args: &GateArgs, global: &GlobalArgs) -> Result<()> {
    if args.list_profiles {
        list_profiles();
        return Ok(());
    }

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

    if global.json {
        crate::output::json::print(&result)?;
    } else {
        println!();
        println!("Gate Results");
        println!("{}", "\u{2500}".repeat(40));
        for check in &result.checks {
            let icon = if check.passed { "\u{2713}" } else { "\u{2717}" };
            let status = if check.passed { "PASS" } else { "FAIL" };
            println!("  {icon} {name:<20} {status}", name = check.name);
            if !check.message.is_empty() && (global.verbose || !check.passed) {
                for line in check.message.lines() {
                    println!("    {line}");
                }
            }
        }
        println!();
        if overall {
            println!(
                "\u{2713} All quality gates passed! (score: {:.0}%)",
                result.score
            );
        } else {
            println!(
                "\u{2717} Quality gates failed ({passed_count}/{total} passed, score: {:.0}%)",
                result.score
            );
        }
    }

    if !overall {
        std::process::exit(i32::from(crate::EXIT_GATE_FAIL));
    }

    Ok(())
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

    #[test]
    fn unimplemented_check_reports_failure() {
        let result = run_single_check("coverage");
        assert!(!result.passed);
        assert!(result.message.contains("not yet implemented"));
    }
}
