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
    /// Validate policy configuration
    Validate {
        /// Policy file to validate
        file: Option<String>,
    },
    /// Run policy tests
    Test {
        /// Test file or directory
        path: Option<String>,
    },
}

#[derive(Debug, Serialize)]
struct PolicyEntry {
    id: String,
    name: String,
    category: String,
    enabled: bool,
    description: String,
    severity: String,
}

#[derive(Debug, Serialize)]
struct PolicyExplanation {
    id: String,
    name: String,
    description: String,
    severity: String,
    category: String,
    enabled: bool,
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
struct TestCase {
    name: String,
    passed: bool,
    message: String,
}

#[derive(Debug, Serialize)]
struct TestResult {
    passed: u32,
    failed: u32,
    tests: Vec<TestCase>,
}

fn policy_catalogue() -> Vec<PolicyEntry> {
    let mut policies: Vec<PolicyEntry> = anvil_policy::library::builtin_policies()
        .into_iter()
        .map(|p| PolicyEntry {
            id: p.id,
            name: p.name,
            category: p.category,
            enabled: p.enabled,
            description: p.description,
            severity: p.severity,
        })
        .collect();

    policies.push(PolicyEntry {
        id: "ARCH-001".into(),
        name: "Cross-layer import".into(),
        category: "architecture".into(),
        enabled: true,
        description: "Detects imports violating layer boundaries (managed by architecture check)"
            .into(),
        severity: "error".into(),
    });
    policies.push(PolicyEntry {
        id: "ARCH-002".into(),
        name: "Circular dependency".into(),
        category: "architecture".into(),
        enabled: true,
        description: "Detects circular import dependencies (managed by architecture check)".into(),
        severity: "error".into(),
    });

    policies
}

#[allow(clippy::too_many_lines)]
pub fn run(args: &PolicyArgs, global: &GlobalArgs) -> Result<()> {
    match &args.command {
        PolicyCommand::List { category, enabled } => {
            let mut policies = policy_catalogue();
            if let Some(cat) = category {
                policies.retain(|p| p.category == *cat);
            }
            if *enabled {
                policies.retain(|p| p.enabled);
            }

            if global.json {
                crate::output::json::print(&policies)?;
            } else {
                crate::output::plain::blank();
                crate::output::plain::section("Policies");
                for p in &policies {
                    let status = if p.enabled { "\u{2713}" } else { "\u{25cb}" };
                    println!("  {status} {id:<10} {name}", id = p.id, name = p.name);
                    println!("    {}", p.description);
                }
                crate::output::plain::blank();
                println!("{} policy(ies)", policies.len());
            }
        }
        PolicyCommand::Explain { policy_id } => {
            let policies = policy_catalogue();
            let policy = policies
                .iter()
                .find(|p| p.id == *policy_id)
                .with_context(|| format!("Policy not found: {policy_id}"))?;

            let explanation = PolicyExplanation {
                id: policy.id.clone(),
                name: policy.name.clone(),
                description: policy.description.clone(),
                severity: policy.severity.clone(),
                category: policy.category.clone(),
                enabled: policy.enabled,
            };

            if global.json {
                crate::output::json::print(&explanation)?;
            } else {
                crate::output::plain::blank();
                println!("Policy: {} -- {}", explanation.id, explanation.name);
                crate::output::plain::blank();
                crate::output::plain::label("Description", &explanation.description);
                crate::output::plain::label("Severity", &explanation.severity);
                crate::output::plain::label("Category", &explanation.category);
                crate::output::plain::label("Enabled", explanation.enabled);
            }
        }
        PolicyCommand::Diff { base, head } => {
            if !std::path::Path::new(base).exists() {
                bail!("Base file not found: {base}");
            }
            if !std::path::Path::new(head).exists() {
                bail!("Head file not found: {head}");
            }

            let base_content =
                std::fs::read_to_string(base).with_context(|| format!("reading {base}"))?;
            let head_content =
                std::fs::read_to_string(head).with_context(|| format!("reading {head}"))?;

            let base_lines: std::collections::HashSet<&str> = base_content.lines().collect();
            let head_lines: std::collections::HashSet<&str> = head_content.lines().collect();

            let mut added: Vec<String> = head_lines
                .difference(&base_lines)
                .map(std::string::ToString::to_string)
                .collect();
            added.sort();
            let mut removed: Vec<String> = base_lines
                .difference(&head_lines)
                .map(std::string::ToString::to_string)
                .collect();
            removed.sort();

            if global.json {
                let result = PolicyDiffResult {
                    added: added.clone(),
                    removed: removed.clone(),
                };
                crate::output::json::print(&result)?;
            } else {
                crate::output::plain::blank();
                println!("Policy Diff: {base} -> {head}");
                crate::output::plain::blank();
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
        }
        PolicyCommand::Validate { file } => {
            let path = file.as_deref().unwrap_or(".anvil/policy.yaml");
            if std::path::Path::new(path)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("rego"))
            {
                bail!(
                    "Rego file validation requires OPA (not yet supported in Rust CLI). Use 'opa check {path}' directly."
                );
            }
            let mut errors = Vec::new();
            let mut warnings = Vec::new();

            if std::path::Path::new(path).exists() {
                let content =
                    std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
                if content.trim().is_empty() {
                    warnings.push("Policy file is empty".to_string());
                } else {
                    match serde_yaml::from_str::<serde_yaml::Value>(&content) {
                        Ok(value) => {
                            if value.get("policies").is_none() {
                                warnings.push("No 'policies' key found in config".to_string());
                            }
                        }
                        Err(e) => {
                            errors.push(format!("Invalid YAML: {e}"));
                        }
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
                crate::output::plain::blank();
                if result.valid {
                    crate::output::plain::success("Policy configuration is valid");
                } else {
                    crate::output::plain::error("Policy configuration has errors");
                    for err in &errors {
                        crate::output::plain::error(err);
                    }
                }
                for w in &warnings {
                    crate::output::plain::warn(w);
                }
            }

            if !result.valid {
                std::process::exit(1);
            }
        }
        PolicyCommand::Test { path } => {
            let test_path = path.as_deref().unwrap_or(".anvil/policies");

            if !std::path::Path::new(test_path).exists() {
                if global.json {
                    let result = TestResult {
                        passed: 0,
                        failed: 0,
                        tests: vec![],
                    };
                    crate::output::json::print(&result)?;
                    std::process::exit(1);
                } else {
                    crate::output::plain::blank();
                    crate::output::plain::warn(&format!(
                        "No policy test directory found at '{test_path}'"
                    ));
                    bail!("Policy test execution is not yet implemented");
                }
            }

            let test_files: Vec<String> = walkdir::WalkDir::new(test_path)
                .into_iter()
                .filter_map(std::result::Result::ok)
                .filter(|e| e.file_type().is_file())
                .map(|e| e.path().to_string_lossy().to_string())
                .collect();

            if test_files.is_empty() {
                if global.json {
                    let result = TestResult {
                        passed: 0,
                        failed: 0,
                        tests: vec![],
                    };
                    crate::output::json::print(&result)?;
                    std::process::exit(1);
                } else {
                    crate::output::plain::blank();
                    crate::output::plain::warn("No test files found");
                    bail!("Policy test execution is not yet implemented");
                }
            }

            bail!(
                "Policy test execution is not yet implemented. Found {} test file(s) in '{test_path}' but cannot execute them yet.",
                test_files.len()
            );
        }
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
