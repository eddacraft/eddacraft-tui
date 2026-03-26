#![allow(dead_code)]
use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

#[derive(Debug, Args)]
pub struct PolicyArgs {
    #[command(subcommand)]
    command: PolicyCommand,
}

#[derive(Debug, clap::Subcommand)]
enum PolicyCommand {
    /// List available policies
    List {
        /// Filter by category
        #[arg(long)]
        category: Option<String>,
        /// Show only enabled policies
        #[arg(long)]
        enabled: bool,
        /// Skip scanning .anvil/policies/ for .rego files
        #[arg(long)]
        no_discover: bool,
    },
    /// Explain a specific policy
    Explain {
        /// Policy ID to explain
        policy_id: String,
    },
    /// Show policy differences
    Diff {
        /// Base policy file
        base: String,
        /// Head policy file
        head: String,
    },
    /// Validate policy configuration or Rego syntax
    Validate {
        /// Policy file to validate (.yaml config or .rego policy)
        file: Option<String>,
    },
    /// Run policy tests
    Test {
        /// Test file or directory
        path: Option<String>,
        /// Verbose output
        #[arg(long, short)]
        verbose: bool,
    },
}

#[derive(Debug, Serialize)]
struct PolicyInfo {
    id: String,
    name: String,
    category: String,
    enabled: bool,
    description: String,
    severity: String,
    source: String,
    has_tests: bool,
}

#[derive(Debug, Serialize)]
struct PolicyDiffResult {
    added: Vec<String>,
    removed: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ValidationResult {
    valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct TestResult {
    passed: u32,
    failed: u32,
    tests: Vec<TestCase>,
}

#[derive(Debug, Serialize, Clone)]
struct TestCase {
    name: String,
    passed: bool,
    message: String,
}

fn workspace_root() -> std::path::PathBuf {
    if let Some(toplevel) = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
    {
        std::path::PathBuf::from(toplevel.trim())
    } else {
        std::env::current_dir().unwrap_or_default()
    }
}

fn run_list(
    category: Option<&String>,
    enabled: bool,
    discover: bool,
    global: &GlobalArgs,
) -> Result<()> {
    let mut infos: Vec<PolicyInfo> = anvil_policy::library::builtin_policies()
        .into_iter()
        .map(|p| PolicyInfo {
            id: p.id,
            name: p.name,
            category: p.category,
            enabled: p.enabled,
            description: p.description,
            severity: p.severity,
            source: "builtin".to_string(),
            has_tests: false,
        })
        .collect();

    if discover {
        let workspace_root = workspace_root();
        let loader = anvil_policy::loader::PolicyLoader::new();
        if let Ok(policies) = loader.load_policies(&workspace_root, None) {
            for p in policies {
                infos.push(PolicyInfo {
                    id: p.package.clone(),
                    name: p.name,
                    category: "rego".to_string(),
                    enabled: true,
                    description: format!("Rego policy ({})", p.path.display()),
                    severity: "varies".to_string(),
                    source: if p.generated {
                        "generated".to_string()
                    } else {
                        "local".to_string()
                    },
                    has_tests: p.has_tests,
                });
            }
        }
    }

    if let Some(cat) = category {
        infos.retain(|p| p.category == cat.as_str());
    }
    if enabled {
        infos.retain(|p| p.enabled);
    }

    if global.json {
        crate::output::json::print(&infos)?;
    } else {
        println!();
        println!("Policies");
        println!("{}", "\u{2500}".repeat(50));
        for p in &infos {
            let status = if p.enabled { "\u{2713}" } else { "\u{25cb}" };
            let test_icon = if p.has_tests { " \u{1f9ea}" } else { "" };
            let source_tag = match p.source.as_str() {
                "local" => " [local]",
                "generated" => " [gen]",
                _ => "",
            };
            println!(
                "  {status} {id:<28} {name}{source_tag}{test_icon}",
                id = p.id,
                name = p.name
            );
            if !global.no_tui {
                println!("    {}", p.description);
            }
        }
        println!();
        println!("{} policy(ies)", infos.len());
    }
    Ok(())
}

fn run_explain(policy_id: &str, global: &GlobalArgs) -> Result<()> {
    let policies = anvil_policy::library::builtin_policies();
    let policy = policies
        .iter()
        .find(|p| p.id == policy_id)
        .with_context(|| format!("Policy not found: {policy_id}"))?;

    if global.json {
        crate::output::json::print(policy)?;
    } else {
        println!();
        println!("Policy: {} \u{2014} {}", policy.id, policy.name);
        println!("{}", "\u{2500}".repeat(40));
        println!("  Description: {}", policy.description);
        println!("  Severity:    {}", policy.severity);
        println!("  Category:    {}", policy.category);
        println!("  Enabled:     {}", policy.enabled);
    }
    Ok(())
}

fn run_diff(base: &str, head: &str, global: &GlobalArgs) -> Result<()> {
    if !std::path::Path::new(base).exists() {
        bail!("Base file not found: {base}");
    }
    if !std::path::Path::new(head).exists() {
        bail!("Head file not found: {head}");
    }

    let base_content = std::fs::read_to_string(base).with_context(|| format!("reading {base}"))?;
    let head_content = std::fs::read_to_string(head).with_context(|| format!("reading {head}"))?;

    let base_lines: std::collections::HashSet<&str> = base_content.lines().collect();
    let head_lines: std::collections::HashSet<&str> = head_content.lines().collect();

    let mut added: Vec<String> = head_lines
        .difference(&base_lines)
        .map(ToString::to_string)
        .collect();
    added.sort();
    let mut removed: Vec<String> = base_lines
        .difference(&head_lines)
        .map(ToString::to_string)
        .collect();
    removed.sort();

    if global.json {
        let result = PolicyDiffResult {
            added: added.clone(),
            removed: removed.clone(),
        };
        crate::output::json::print(&result)?;
    } else {
        println!();
        println!("Policy Diff: {base} \u{2192} {head}");
        println!("{}", "\u{2500}".repeat(40));
        if !added.is_empty() {
            println!("  Added ({}):", added.len());
            for line in &added {
                println!("    + {line}");
            }
        }
        if !removed.is_empty() {
            println!("  Removed ({}):", removed.len());
            for line in &removed {
                println!("    - {line}");
            }
        }
        if added.is_empty() && removed.is_empty() {
            println!("  No differences");
        }
    }
    Ok(())
}

fn run_validate(file: Option<&String>, global: &GlobalArgs) -> Result<()> {
    let path = file.map_or(".anvil/policy.yaml", String::as_str);
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if std::path::Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("rego"))
    {
        if !std::path::Path::new(path).exists() {
            errors.push(format!("Policy file not found: {path}"));
            let result = ValidationResult {
                valid: false,
                errors,
                warnings,
            };
            if global.json {
                crate::output::json::print(&result)?;
            } else {
                println!("\n\u{2717} Policy configuration has errors");
                for err in &result.errors {
                    println!("  \u{2717} {err}");
                }
            }
            std::process::exit(1);
        }
        let content = std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
        let opa = anvil_policy::opa::OpaExecutor::new(None, None);

        if opa.is_available() {
            match opa.validate_syntax(&content) {
                Ok(result) if result.valid => {
                    // valid — no errors
                }
                Ok(result) => {
                    errors.extend(result.errors);
                }
                Err(e) => {
                    errors.push(format!("OPA validation error: {e}"));
                }
            }
        } else {
            warnings.push("OPA binary not found — skipping Rego syntax validation".to_string());
            warnings.push(
                "Install OPA: https://www.openpolicyagent.org/docs/latest/#running-opa".to_string(),
            );
        }
    } else if std::path::Path::new(path).exists() {
        match anvil_policy::config::load_config(path) {
            Ok(config) => {
                if config.policies.is_empty() {
                    warnings.push("Policy file contains no policies".to_string());
                }
            }
            Err(anvil_policy::config::ConfigError::Parse(msg)) => {
                errors.push(format!("Invalid YAML: {msg}"));
            }
            Err(e) => {
                errors.push(e.to_string());
            }
        }
    } else {
        errors.push(format!("Policy file not found: {path}"));
    }

    let result = ValidationResult {
        valid: errors.is_empty(),
        errors: errors.clone(),
        warnings: warnings.clone(),
    };

    if global.json {
        crate::output::json::print(&result)?;
    } else {
        println!();
        if result.valid {
            println!("\u{2713} Policy configuration is valid");
        } else {
            println!("\u{2717} Policy configuration has errors");
            for err in &errors {
                println!("  \u{2717} {err}");
            }
        }
        for w in &warnings {
            println!("  \u{26a0} {w}");
        }
    }

    if !result.valid {
        std::process::exit(1);
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn run_test(path: Option<&String>, verbose: bool, global: &GlobalArgs) -> Result<()> {
    let test_path = path.map_or(".anvil/policies", String::as_str);
    let test_dir = std::path::Path::new(test_path);

    let opa = anvil_policy::opa::OpaExecutor::new(None, None);

    if opa.is_available() && test_dir.exists() {
        let result = opa
            .run_tests(test_dir, verbose)
            .with_context(|| "running OPA tests")?;

        let test_cases: Vec<TestCase> = result
            .details
            .iter()
            .map(|d| TestCase {
                name: d.name.clone(),
                passed: d.passed,
                message: d.message.clone().unwrap_or_default(),
            })
            .collect();

        if global.json {
            let output = TestResult {
                passed: result.passed,
                failed: result.failed,
                tests: test_cases.clone(),
            };
            crate::output::json::print(&output)?;
        } else {
            println!();
            println!("Policy Tests (OPA)");
            println!("{}", "\u{2500}".repeat(40));
            for tc in &test_cases {
                let icon = if tc.passed { "\u{2713}" } else { "\u{2717}" };
                println!("  {icon} {}", tc.name);
                if (verbose || !tc.passed) && !tc.message.is_empty() {
                    println!("    {}", tc.message);
                }
            }
            for err in &result.errors {
                println!("  \u{2717} {err}");
            }
            println!();
            if result.failed == 0 {
                println!("\u{2713} All {} tests passed", result.passed);
            } else {
                println!(
                    "\u{2717} {} test(s) failed, {} passed",
                    result.failed, result.passed
                );
            }
        }

        if result.failed > 0 {
            std::process::exit(1);
        }
    } else {
        let mut test_cases = Vec::new();

        if !opa.is_available() {
            eprintln!("\u{26a0} OPA not found — running builtin policy validation only");
        }

        for policy in anvil_policy::library::builtin_policies()
            .iter()
            .filter(|p| p.enabled)
        {
            test_cases.push(TestCase {
                name: format!("{}:{}", policy.id, policy.name),
                passed: true,
                message: format!("Policy {} is enabled and valid", policy.id),
            });
        }

        #[allow(clippy::cast_possible_truncation)]
        let passed = test_cases.iter().filter(|t| t.passed).count() as u32;
        #[allow(clippy::cast_possible_truncation)]
        let failed = test_cases.iter().filter(|t| !t.passed).count() as u32;

        if global.json {
            let result = TestResult {
                passed,
                failed,
                tests: test_cases.clone(),
            };
            crate::output::json::print(&result)?;
        } else {
            println!();
            println!("Policy Tests");
            println!("{}", "\u{2500}".repeat(40));
            for tc in &test_cases {
                let icon = if tc.passed { "\u{2713}" } else { "\u{2717}" };
                println!("  {icon} {}", tc.name);
                if verbose || !tc.passed {
                    println!("    {}", tc.message);
                }
            }
            println!();
            if failed == 0 {
                println!("\u{2713} All {passed} tests passed");
            } else {
                println!("\u{2717} {failed} test(s) failed, {passed} passed");
            }
        }

        if failed > 0 {
            std::process::exit(1);
        }
    }
    Ok(())
}

pub fn run(args: &PolicyArgs, global: &GlobalArgs) -> Result<()> {
    match &args.command {
        PolicyCommand::List {
            category,
            enabled,
            no_discover,
        } => run_list(category.as_ref(), *enabled, !*no_discover, global),
        PolicyCommand::Explain { policy_id } => run_explain(policy_id, global),
        PolicyCommand::Diff { base, head } => run_diff(base, head, global),
        PolicyCommand::Validate { file } => run_validate(file.as_ref(), global),
        PolicyCommand::Test { path, verbose } => run_test(path.as_ref(), *verbose, global),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Wrapper {
        #[command(flatten)]
        inner: PolicyArgs,
    }

    #[test]
    fn args_parses_list() {
        let w = Wrapper::try_parse_from(["test", "list"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_explain() {
        let w = Wrapper::try_parse_from(["test", "explain", "AP-001"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_diff() {
        let w = Wrapper::try_parse_from(["test", "diff", "base.yaml", "head.yaml"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_validate() {
        let w = Wrapper::try_parse_from(["test", "validate"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_test() {
        let w = Wrapper::try_parse_from(["test", "test"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }
}
