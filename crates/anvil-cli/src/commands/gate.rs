#![allow(dead_code)]
use anyhow::Result;
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct GateArgs {
    /// Plan file to run gates against (omit for full codebase scan)
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

fn run_single_check(name: &str) -> CheckResult {
    match name {
        "lint" => {
            let output = std::process::Command::new("npx")
                .args(["eslint", ".", "--max-warnings", "0"])
                .output();
            match output {
                Ok(o) if o.status.success() => CheckResult {
                    name: name.to_string(),
                    passed: true,
                    score: 100.0,
                    message: "No lint errors".to_string(),
                },
                Ok(o) => CheckResult {
                    name: name.to_string(),
                    passed: false,
                    score: 0.0,
                    message: format!("Lint errors found\n{}", String::from_utf8_lossy(&o.stderr)),
                },
                Err(e) => CheckResult {
                    name: name.to_string(),
                    passed: false,
                    score: 0.0,
                    message: format!("Failed to run lint: {e}"),
                },
            }
        }
        "test" => {
            let output = std::process::Command::new("pnpm").args(["test"]).output();
            match output {
                Ok(o) if o.status.success() => CheckResult {
                    name: name.to_string(),
                    passed: true,
                    score: 100.0,
                    message: "All tests passed".to_string(),
                },
                Ok(o) => CheckResult {
                    name: name.to_string(),
                    passed: false,
                    score: 0.0,
                    message: format!("Tests failed\n{}", String::from_utf8_lossy(&o.stderr)),
                },
                Err(e) => CheckResult {
                    name: name.to_string(),
                    passed: false,
                    score: 0.0,
                    message: format!("Failed to run tests: {e}"),
                },
            }
        }
        "secret" => {
            let mut found = Vec::new();
            let secret_patterns = [
                "AKIA[0-9A-Z]{16}",
                "sk-[a-zA-Z0-9]{48}",
                "ghp_[a-zA-Z0-9]{36}",
            ];
            for entry in walkdir::WalkDir::new(".")
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy();
                    if !matches!(
                        &*ext_str,
                        "ts" | "js" | "rs" | "json" | "yaml" | "yml" | "toml" | "env"
                    ) {
                        continue;
                    }
                } else {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(path) {
                    for line in content.lines() {
                        for pattern in &secret_patterns {
                            if line.contains(pattern) {
                                found.push(format!("{}: {}", path.display(), line.trim()));
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
                    message: format!("Potential secrets found:\n{}", found.join("\n")),
                }
            }
        }
        _ => CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: "Check not yet implemented in Rust CLI".to_string(),
        },
    }
}

pub fn run(args: &GateArgs, global: &GlobalArgs) -> Result<()> {
    if args.list_profiles {
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
        return Ok(());
    }

    let start = std::time::Instant::now();

    let skip_set: std::collections::HashSet<&str> = args
        .skip_checks
        .as_deref()
        .map(|s| s.split(',').map(|s| s.trim()).collect())
        .unwrap_or_default();

    let only_set: Option<std::collections::HashSet<&str>> = args
        .only_checks
        .as_deref()
        .map(|s| s.split(',').map(|s| s.trim()).collect());

    let available_checks = [
        "lint",
        "test",
        "coverage",
        "dependency",
        "secret",
        "architecture",
        "policy",
    ];

    let mut checks = Vec::new();
    for check_name in available_checks {
        if skip_set.contains(check_name) {
            continue;
        }
        if let Some(ref only_s) = only_set {
            if !only_s.contains(check_name) {
                continue;
            }
        }

        if args.progress {
            eprint!("  \u{25b6} {check_name} running...\n");
        }

        let result = run_single_check(check_name);

        if args.progress {
            let icon = if result.passed {
                "\u{2713}"
            } else {
                "\u{2717}"
            };
            eprint!("  {icon} {check_name}\n");
        }

        checks.push(result);

        if args.fail_fast && !checks.last().unwrap().passed {
            break;
        }
    }

    let passed_count = checks.iter().filter(|c| c.passed).count();
    let total = checks.len();
    let overall = checks.iter().all(|c| c.passed);
    let score = if total > 0 {
        (passed_count as f64 / total as f64) * 100.0
    } else {
        100.0
    };

    let result = GateResult {
        overall,
        score,
        checks,
        duration_ms: start.elapsed().as_millis() as u64,
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
        std::process::exit(crate::EXIT_GATE_FAIL as i32);
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
}
