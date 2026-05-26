use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;

mod eval;

#[derive(Debug, Args)]
pub struct PolicyArgs {
    #[command(subcommand)]
    command: PolicyCommand,
}

#[derive(Debug, clap::Subcommand)]
enum PolicyCommand {
    /// Evaluate a Rego policy against an input document (POLENG)
    Eval(eval::EvalArgs),
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
        /// List discovered test files
        #[arg(long)]
        list_files: bool,
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
    skipped: u32,
    tests: Vec<TestCase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    files: Option<Vec<String>>,
}

fn policy_test_file_walker(test_path: &str) -> ignore::Walk {
    ignore::WalkBuilder::new(test_path)
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .build()
}

fn collect_policy_test_files(test_path: &str) -> Vec<String> {
    let mut files: Vec<String> = policy_test_file_walker(test_path)
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();
    files.sort();
    files
}

fn count_policy_test_files(test_path: &str) -> usize {
    policy_test_file_walker(test_path)
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
        .count()
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
        PolicyCommand::Eval(eval_args) => return eval::run(eval_args, global),
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
            let mut errors = Vec::new();
            let mut warnings = Vec::new();

            if !std::path::Path::new(path).exists() {
                errors.push(format!("Policy file not found: {path}"));
            } else if std::path::Path::new(path)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("rego"))
            {
                errors.push(format!(
                    "Rego file validation requires OPA (not yet supported). \
                     Use 'opa check {path}' directly."
                ));
            } else {
                match anvil_policy::config::load_config(path) {
                    Ok(config) => {
                        if config.policies.is_empty() {
                            warnings.push("Policy file contains no policies".to_string());
                        }
                    }
                    Err(anvil_policy::config::ConfigError::Parse(msg)) => {
                        errors.push(format!("Invalid policy config: {msg}"));
                    }
                    Err(e) => {
                        errors.push(e.to_string());
                    }
                }
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
                return Err(crate::output::AlreadyReported.into());
            }
        }
        PolicyCommand::Test { path, list_files } => {
            let test_path = path.as_deref().unwrap_or(".anvil/policies");

            if !std::path::Path::new(test_path).exists() {
                if global.json {
                    let result = TestResult {
                        passed: 0,
                        failed: 0,
                        skipped: 0,
                        tests: vec![],
                        warning: Some(format!("No policy test directory found at '{test_path}'")),
                        files: None,
                    };
                    crate::output::json::print(&result)?;
                } else {
                    crate::output::plain::blank();
                    crate::output::plain::warn(&format!(
                        "No policy test directory found at '{test_path}'"
                    ));
                    crate::output::plain::warn(
                        "Policy test execution is not yet implemented. \
                         Create Rego tests in .anvil/policies/ for future use.",
                    );
                }
                return Ok(());
            }

            let (file_count, test_files) = if *list_files {
                let files = collect_policy_test_files(test_path);
                (files.len(), Some(files))
            } else {
                (count_policy_test_files(test_path), None)
            };

            if file_count == 0 {
                if global.json {
                    let result = TestResult {
                        passed: 0,
                        failed: 0,
                        skipped: 0,
                        tests: vec![],
                        warning: Some(format!("No test files found in '{test_path}'")),
                        files: None,
                    };
                    crate::output::json::print(&result)?;
                } else {
                    crate::output::plain::blank();
                    crate::output::plain::warn("No test files found");
                }
                return Ok(());
            }

            if global.json {
                let result = TestResult {
                    passed: 0,
                    failed: 0,
                    skipped: u32::try_from(file_count).unwrap_or(u32::MAX),
                    tests: vec![],
                    warning: Some(format!(
                        "Found {file_count} test file(s) but policy test execution \
                         is not yet implemented. Use 'opa test' directly."
                    )),
                    files: test_files.clone(),
                };
                crate::output::json::print(&result)?;
            } else {
                crate::output::plain::blank();
                crate::output::plain::warn(&format!(
                    "Found {file_count} test file(s) in '{test_path}' but policy test execution is not yet implemented",
                ));
                if let Some(files) = &test_files {
                    for f in files {
                        crate::output::plain::info(f);
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::fs;
    use std::path::Path;

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
        let w = Wrapper::try_parse_from(["test", "explain", "ARCH-001"]).unwrap();
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

    #[test]
    fn args_parses_test_with_path() {
        let w = Wrapper::try_parse_from(["test", "test", "my/policies"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_test_list_files() {
        let w = Wrapper::try_parse_from(["test", "test", "--list-files"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_list_with_category() {
        let w = Wrapper::try_parse_from(["test", "list", "--category", "security"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_list_enabled() {
        let w = Wrapper::try_parse_from(["test", "list", "--enabled"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_validate_with_file() {
        let w = Wrapper::try_parse_from(["test", "validate", "my-policy.yaml"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn catalogue_includes_architecture_policies() {
        let policies = policy_catalogue();
        let arch_001 = policies.iter().find(|p| p.id == "ARCH-001");
        assert!(arch_001.is_some(), "should include ARCH-001");
        let arch_002 = policies.iter().find(|p| p.id == "ARCH-002");
        assert!(arch_002.is_some(), "should include ARCH-002");
    }

    #[test]
    fn catalogue_architecture_policies_are_enabled() {
        let policies = policy_catalogue();
        for p in policies.iter().filter(|p| p.category == "architecture") {
            assert!(p.enabled, "{} should be enabled", p.id);
        }
    }

    #[test]
    fn catalogue_is_non_empty() {
        let policies = policy_catalogue();
        assert!(
            policies.len() >= 2,
            "should have at least the ARCH policies"
        );
    }

    #[test]
    fn catalogue_entries_have_required_fields() {
        let policies = policy_catalogue();
        for p in &policies {
            assert!(!p.id.is_empty(), "id should not be empty");
            assert!(!p.name.is_empty(), "name should not be empty");
            assert!(!p.category.is_empty(), "category should not be empty");
            assert!(!p.description.is_empty(), "description should not be empty");
            assert!(!p.severity.is_empty(), "severity should not be empty");
        }
    }

    #[test]
    fn catalogue_category_filter() {
        let policies = policy_catalogue();
        let architecture: Vec<_> = policies
            .into_iter()
            .filter(|p| p.category == "architecture")
            .collect();

        assert!(
            architecture.iter().all(|p| p.category == "architecture"),
            "all filtered policies should have category 'architecture'"
        );
        assert!(
            architecture.iter().any(|p| p.id == "ARCH-001"),
            "ARCH-001 should be present in the architecture category"
        );
        assert!(
            architecture.iter().any(|p| p.id == "ARCH-002"),
            "ARCH-002 should be present in the architecture category"
        );
    }

    #[test]
    fn catalogue_enabled_filter() {
        let all = policy_catalogue();
        let mut enabled = policy_catalogue();
        enabled.retain(|p| p.enabled);
        assert!(enabled.len() >= 2);
        assert!(enabled.len() <= all.len());
    }

    #[test]
    fn diff_result_serialises() {
        let result = PolicyDiffResult {
            added: vec!["line-a".to_string()],
            removed: vec!["line-b".to_string()],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("line-a"));
        assert!(json.contains("line-b"));
    }

    #[test]
    fn validation_result_serialises() {
        let result = ValidationResult {
            valid: false,
            errors: vec!["err1".to_string()],
            warnings: vec!["warn1".to_string()],
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["valid"], false);
        assert_eq!(parsed["errors"][0], "err1");
    }

    #[test]
    fn collect_policy_test_files_uses_scan_walker_shape() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root = temp_dir.path();
        let hidden_dir = root.join(".hidden");
        fs::create_dir_all(&hidden_dir).unwrap();
        fs::write(root.join("b.rego"), "package b").unwrap();
        fs::write(hidden_dir.join("a.rego"), "package a").unwrap();

        let files = collect_policy_test_files(&root.to_string_lossy());

        assert_eq!(files.len(), 2);
        assert!(Path::new(&files[0]).ends_with(Path::new(".hidden").join("a.rego")));
        assert!(Path::new(&files[1]).ends_with("b.rego"));
        assert_eq!(count_policy_test_files(&root.to_string_lossy()), 2);
    }
}
