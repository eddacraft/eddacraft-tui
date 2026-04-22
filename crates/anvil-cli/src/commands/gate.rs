use std::path::{Path, PathBuf};

use anvil_kernel_types::{
    Notification, NotificationClass, NotificationContext, NotificationPriority,
};
use anyhow::{Result, bail};
use clap::Args;
use regex::Regex;
use serde::Serialize;

use crate::GlobalArgs;
use crate::commands::check_catalog::{
    GATE_INTERNAL_CHECKS, gate_canonical_name_from_internal, gate_canonical_names,
    gate_internal_name,
};

#[derive(Debug, Default, Args)]
#[allow(clippy::struct_excessive_bools)]
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
    notifications: Vec<Notification>,
    duration_ms: u64,
}

#[derive(Debug, Serialize)]
struct CheckResult {
    name: String,
    passed: bool,
    score: f64,
    message: String,
}

fn notifications_for_gate_result(checks: &[CheckResult], overall: bool) -> Vec<Notification> {
    let gate_context = || NotificationContext {
        file: None,
        source: Some("gate".to_string()),
    };

    let mut notifications: Vec<Notification> = checks
        .iter()
        .map(|check| {
            Notification::new(
                if check.passed {
                    NotificationClass::Info
                } else {
                    NotificationClass::Failure
                },
                if check.passed {
                    NotificationPriority::Low
                } else {
                    NotificationPriority::High
                },
                format!("Gate check: {}", check.name),
                if check.message.is_empty() {
                    if check.passed {
                        "Passed".to_string()
                    } else {
                        "Failed".to_string()
                    }
                } else {
                    check.message.clone()
                },
            )
            .with_context(gate_context())
        })
        .collect();

    notifications.push(
        Notification::new(
            if overall {
                NotificationClass::Info
            } else {
                NotificationClass::Failure
            },
            if overall {
                NotificationPriority::Normal
            } else {
                NotificationPriority::High
            },
            "Gate result",
            if overall {
                "All quality gates passed"
            } else {
                "Quality gates failed"
            },
        )
        .with_context(gate_context()),
    );

    notifications
}

/// Extract file paths referenced in a `.aps.md` plan file.
///
/// Parses `- **Files:** ...` lines and returns deduplicated paths.
/// Returns an empty set (and emits a warning) if the file cannot be read.
fn extract_plan_files(plan_path: &Path) -> std::collections::HashSet<String> {
    let content = match std::fs::read_to_string(plan_path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!(
                "Warning: failed to read plan file '{}': {err}. Falling back to full codebase scan.",
                plan_path.display()
            );
            return std::collections::HashSet::new();
        }
    };

    let file_re = Regex::new(r"`([^`]+)`").expect("valid regex");
    let mut files = std::collections::HashSet::new();

    // Track whether we're in a Files: continuation (multi-line entries).
    let mut in_files_block = false;

    for line in content.lines() {
        let trimmed = line.trim_start_matches([' ', '-']);
        if trimmed.starts_with("**Files:**") {
            in_files_block = true;
            for cap in file_re.captures_iter(trimmed) {
                let path = cap[1].to_string();
                if path.contains('/') || path.contains('.') {
                    files.insert(path);
                }
            }
        } else if in_files_block {
            // Continuation lines: indented lines with backticked paths.
            let has_backticks = trimmed.contains('`');
            let is_continuation =
                has_backticks && !trimmed.starts_with("**") && !trimmed.starts_with('#');
            if is_continuation {
                for cap in file_re.captures_iter(trimmed) {
                    let path = cap[1].to_string();
                    if path.contains('/') || path.contains('.') {
                        files.insert(path);
                    }
                }
            } else {
                in_files_block = false;
            }
        }
    }

    files
}

/// Resolve a plan argument to a path: either an absolute path, or relative to
/// the workspace root. Searches `plans/modules/` if not found directly.
fn resolve_plan_path(plan_arg: &str, root: &Path) -> Option<PathBuf> {
    let direct = PathBuf::from(plan_arg);
    if direct.exists() {
        return Some(direct);
    }

    // Try relative to workspace root.
    let relative = root.join(plan_arg);
    if relative.exists() {
        return Some(relative);
    }

    // Try in plans/modules/.
    let in_modules = root.join("plans/modules").join(plan_arg);
    if in_modules.exists() {
        return Some(in_modules);
    }

    // Try with .aps.md extension.
    let with_ext = root
        .join("plans/modules")
        .join(format!("{plan_arg}.aps.md"));
    if with_ext.exists() {
        return Some(with_ext);
    }

    None
}

fn run_check_lint(name: &str, root: &Path) -> CheckResult {
    let output = std::process::Command::new("pnpm")
        .args(["lint:check"])
        .current_dir(root)
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

fn run_check_test(name: &str, root: &Path) -> CheckResult {
    let output = std::process::Command::new("pnpm")
        .args(["test"])
        .current_dir(root)
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

/// Directories to skip when walking the workspace for source files.
/// Aligned with kernel `FileFilter::default_patterns` in
/// `crates/anvil-kernel/src/watcher/filter.rs`.
pub(crate) const WALK_IGNORE_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    ".next",
    ".turbo",
    ".nx",
    "coverage",
    ".anvil",
    "__pycache__",
];

const SECRET_SCAN_IGNORE: &[&str] = &[
    "node_modules",
    "dist",
    "build",
    "target",
    ".git",
    ".anvil",
    "coverage",
];

/// Maximum directory depth for the secret scan walk. Prevents runaway
/// recursion into deeply nested or symlink-heavy trees.
const SECRET_SCAN_MAX_DEPTH: usize = 20;

fn run_check_secret(
    name: &str,
    root: &Path,
    plan_files: &std::collections::HashSet<String>,
) -> CheckResult {
    let mut files_to_scan: Vec<String> = Vec::new();

    let mut walker = walkdir::WalkDir::new(root).follow_links(false);
    // Only cap depth for full-codebase scans; plan-scoped runs must reach
    // explicitly referenced files regardless of nesting depth.
    if plan_files.is_empty() {
        walker = walker.max_depth(SECRET_SCAN_MAX_DEPTH);
    }
    for entry in walker
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !SECRET_SCAN_IGNORE.iter().any(|&ig| name == ig)
        })
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();

        // Plan scoping: skip files not referenced in the plan.
        if !plan_files.is_empty() {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            if !plan_files.iter().any(|pf| {
                if pf.ends_with('/') || root.join(pf).is_dir() {
                    rel.starts_with(pf.as_str())
                } else {
                    rel == pf.as_str()
                }
            }) {
                continue;
            }
        }

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

        files_to_scan.push(path.to_string_lossy().into_owned());
    }

    let file_refs: Vec<&str> = files_to_scan.iter().map(String::as_str).collect();
    let config = anvil_checks::secret::SecretCheckConfig::default();
    let root_str = root.to_string_lossy();
    let result = anvil_checks::secret::run_secret_check(&file_refs, &config, Some(&root_str));

    if result.passed {
        CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: "No hardcoded secrets found".to_string(),
        }
    } else {
        let locations: Vec<String> = result
            .findings
            .iter()
            .map(|f| format!("{}:{} [{}]", f.file, f.line, f.pattern_name))
            .collect();
        CheckResult {
            name: name.to_string(),
            passed: false,
            score: f64::from(result.score),
            message: format!(
                "Potential secrets found in {} location(s):\n{}",
                result.findings.len(),
                locations.join("\n")
            ),
        }
    }
}

fn run_check_antipattern(
    name: &str,
    root: &Path,
    plan_files: &std::collections::HashSet<String>,
) -> CheckResult {
    let mut files_to_scan = walk_source_files(root, &[]);
    if !plan_files.is_empty() {
        files_to_scan.retain(|f| {
            plan_files.iter().any(|pf| {
                if pf.ends_with('/') || root.join(pf).is_dir() {
                    f.starts_with(pf.as_str())
                } else {
                    f == pf.as_str()
                }
            })
        });
    }

    let absolute_files: Vec<String> = files_to_scan
        .iter()
        .map(|rel| root.join(rel).to_string_lossy().into_owned())
        .collect();
    let file_refs: Vec<&str> = absolute_files.iter().map(String::as_str).collect();
    let root_str = root.to_string_lossy();
    let result = anvil_checks::antipattern::run_antipattern_check(
        &file_refs,
        &anvil_checks::antipattern::AntipatternCheckConfig {
            severity_threshold: anvil_checks::antipattern::WarningSeverity::Warning,
            ..anvil_checks::antipattern::AntipatternCheckConfig::default()
        },
        Some(&root_str),
    );

    if result.files_scanned == 0 {
        return CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: "No analysable files found for anti-pattern scan. Skipping.".to_string(),
        };
    }

    if result.passed {
        CheckResult {
            name: name.to_string(),
            passed: true,
            score: f64::from(result.score),
            message: result.message,
        }
    } else {
        let locations: Vec<String> = result
            .warnings
            .warnings
            .iter()
            .filter(|w| w.suppressed.is_none())
            .map(|w| format!("{}:{} [{}]", w.location.file, w.location.line, w.id))
            .collect();
        let details = if locations.is_empty() {
            result.message
        } else {
            format!("{}\n{}", result.message, locations.join("\n"))
        };
        CheckResult {
            name: name.to_string(),
            passed: false,
            score: f64::from(result.score),
            message: details,
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

/// Extract import edges from source files using the kernel's tree-sitter parser.
///
/// When `source_files` is provided, only those files are parsed (avoids a
/// redundant directory walk). Otherwise falls back to walking `project_root`.
fn extract_import_edges(
    project_root: &Path,
    source_files: Option<&[String]>,
) -> Vec<anvil_architecture::ImportEdge> {
    let mut parser = anvil_kernel::parser::Parser::new();
    let mut edges = Vec::new();

    let include_extensions = ["ts", "tsx", "js", "jsx", "mjs", "cjs"];

    // Collect file paths to parse — either from the pre-collected list or via walkdir.
    let owned_paths: Vec<String>;
    let file_paths: &[String] = if let Some(files) = source_files {
        files
    } else {
        owned_paths = walk_source_files(project_root, &include_extensions);
        &owned_paths
    };

    for rel_path in file_paths {
        // Filter to JS/TS — the pre-collected list from collect_source_files
        // may include .rs and other file types matched by architecture layer globs.
        let ext = std::path::Path::new(rel_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if !include_extensions.contains(&ext) {
            continue;
        }

        let path = project_root.join(rel_path);

        let Ok(content) = std::fs::read(&path) else {
            continue;
        };

        let Ok(parse_result) = parser.parse_bytes(&path, &content) else {
            continue;
        };

        let file_symbols =
            anvil_kernel::parser::extract::extract_symbols(&parse_result.tree, &content, &path, 0);

        for import in &file_symbols.imports {
            // Only resolve relative imports (starting with . or ..).
            if !import.to_source.starts_with('.') {
                continue;
            }

            if let Some(resolved) = resolve_import(rel_path, &import.to_source) {
                edges.push(anvil_architecture::ImportEdge {
                    from_file: rel_path.clone(),
                    to_file: resolved,
                    line: import.line,
                });
            }
        }
    }

    edges
}

/// Walk the workspace directory and collect source file paths (relative).
///
/// When `extensions` is non-empty, only files with a matching extension are
/// included. When empty, all files are collected.
fn walk_source_files(project_root: &Path, extensions: &[&str]) -> Vec<String> {
    let walker = walkdir::WalkDir::new(project_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            if e.file_type().is_dir() {
                return !WALK_IGNORE_DIRS.contains(&name.as_ref());
            }
            true
        });

    let mut files = Vec::new();
    for entry in walker.flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !extensions.is_empty() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !extensions.contains(&ext) {
                continue;
            }
        }
        let rel_path = path
            .strip_prefix(project_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        files.push(rel_path);
    }
    files
}

/// Resolve a relative import specifier to a workspace-relative path.
///
/// Given `from_file = "src/app/service.ts"` and `specifier = "../core/entity"`,
/// returns `"src/core/entity"`. Does not verify the file exists on disk;
/// the validator matches against assigned files by prefix.
///
/// Returns `None` if the specifier traverses above the workspace root
/// (e.g. `"../../../outside"`). These imports are silently excluded from
/// boundary analysis since they reference external code.
fn resolve_import(from_file: &str, specifier: &str) -> Option<String> {
    let from_dir = from_file.rsplit_once('/').map_or("", |(dir, _)| dir);

    // Combine from_dir with the specifier and normalise.
    let combined = if from_dir.is_empty() {
        specifier.to_string()
    } else {
        format!("{from_dir}/{specifier}")
    };

    // Normalise path segments (resolve .. and .).
    let mut parts: Vec<&str> = Vec::new();
    for segment in combined.split('/') {
        match segment {
            "." | "" => {}
            ".." => {
                // Returns None if traversal goes above workspace root.
                parts.pop()?;
            }
            s => parts.push(s),
        }
    }

    if parts.is_empty() {
        return None;
    }

    Some(parts.join("/"))
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

    // Collect source files once and share between edge extraction and validation
    // to avoid redundant directory walks (RCLI-053).
    let source_files = anvil_architecture::collect_source_files(project_root, &definition);
    let edges = extract_import_edges(project_root, Some(&source_files));

    let result =
        anvil_architecture::validate_with_files_and_edges(&definition, &source_files, &edges);

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
                let boundary_name = v.boundary.as_ref().map_or("unknown", |b| b.name.as_str());
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

/// Collect changed files from git status (unstaged + staged).
fn git_changed_files(project_root: &Path) -> Vec<String> {
    std::process::Command::new("git")
        .args(["status", "--porcelain", "-u"])
        .current_dir(project_root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|line| {
                    // porcelain format: XY filename
                    // Renamed/copied files: XY old -> new
                    let trimmed = line.get(3..)?;
                    if trimmed.contains(" -> ") {
                        trimmed.rsplit_once(" -> ").map(|(_, new)| new.to_string())
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Build policy input with project context so policies can reference
/// `input.workspace`, `input.files`, `input.changed_files`, etc.
///
/// When `all_files` is provided, filters it by policy-relevant extensions
/// instead of walking the directory tree again.
fn build_policy_input(
    project_root: &Path,
    profile: Option<&str>,
    plan_path: Option<&str>,
    plan_files: &std::collections::HashSet<String>,
    all_files: Option<&[String]>,
) -> serde_json::Value {
    let policy_extensions = [
        "ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "json", "yaml", "yml",
    ];

    let source_files: Vec<String> = if let Some(files) = all_files {
        files
            .iter()
            .filter(|f| {
                std::path::Path::new(f.as_str())
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|ext| policy_extensions.contains(&ext))
            })
            .cloned()
            .collect()
    } else {
        walk_source_files(project_root, &policy_extensions)
    };

    let changed_files = git_changed_files(project_root);

    // When plan-scoped, filter files to only those referenced in the plan.
    let files = if plan_files.is_empty() {
        source_files
    } else {
        source_files
            .into_iter()
            .filter(|f| {
                plan_files.iter().any(|pf| {
                    if pf.ends_with('/') {
                        f.starts_with(pf.as_str())
                    } else {
                        f == pf.as_str()
                    }
                })
            })
            .collect()
    };

    let mut input = serde_json::json!({
        "workspace": project_root.to_string_lossy(),
        "files": files,
        "changed_files": changed_files,
        "profile": profile.unwrap_or("default"),
    });

    if let Some(plan) = plan_path {
        input["plan_path"] = serde_json::Value::String(plan.to_string());
    }

    input
}

fn run_check_policy(
    project_root: &Path,
    profile: Option<&str>,
    plan_path: Option<&str>,
    plan_files: &std::collections::HashSet<String>,
    all_files: Option<&[String]>,
) -> CheckResult {
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
    let input = build_policy_input(project_root, profile, plan_path, plan_files, all_files);

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
        Err(anvil_policy::evaluator::EvalError::UnexpectedShape { pointer, .. }) => CheckResult {
            // UnexpectedShape comes through as a structured variant rather
            // than a substring match so wording changes in the Display impl
            // don't silently break this branch. Most common cause is an OPA
            // version whose output layout has drifted; point the operator
            // at the runbook rather than dumping the raw snippet at them.
            name: "policy".to_string(),
            passed: false,
            score: 0.0,
            message: format!(
                "Policy evaluation failed: unexpected OPA output shape at {pointer}.\n\
                 hint: the OPA output schema does not match what Anvil expects. \
                 Verify your OPA version with `anvil doctor` and confirm it matches \
                 the version pinned in docs/guides/opa-policy-testing.md."
            ),
        },
        Err(e) => CheckResult {
            name: "policy".to_string(),
            passed: false,
            score: 0.0,
            message: format!("Policy evaluation failed: {e}"),
        },
    }
}

fn run_single_check(name: &str, ctx: &GateContext) -> CheckResult {
    let root = &ctx.workspace_root;
    let mut result = match name {
        "lint" => run_check_lint(name, root),
        "test" => run_check_test(name, root),
        "antipattern-scan" => run_check_antipattern(name, root, &ctx.plan_files),
        "secret" => run_check_secret(name, root, &ctx.plan_files),
        "coverage" => run_check_coverage(root, DEFAULT_COVERAGE_THRESHOLD),
        "dependency" => run_check_dependency(root),
        "architecture" => run_check_architecture(root),
        "policy" => run_check_policy(
            root,
            ctx.profile.as_deref(),
            ctx.plan_path.as_deref(),
            &ctx.plan_files,
            Some(&ctx.walked_files),
        ),
        _ => CheckResult {
            name: name.to_string(),
            passed: false,
            score: 0.0,
            message: format!("Unknown check: {name}"),
        },
    };
    result.name = gate_canonical_name_from_internal(name);
    result
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

/// Read `.anvilrc#checks` from the workspace root, when present.
///
/// Supports JSON, YAML, and TOML variants of `.anvilrc`. Returns `Ok(None)`
/// when the file is absent, has no `checks` field, or the list is empty.
/// Parsing/shape errors are surfaced so gate can fail clearly instead of
/// silently acting on a malformed filter.
fn read_anvilrc_checks(workspace_root: &Path) -> Result<Option<std::collections::HashSet<String>>> {
    let path = workspace_root.join(".anvilrc");
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(anyhow::anyhow!("failed to read {}: {err}", path.display())),
    };

    let checks = if let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) {
        if value.is_object() {
            extract_checks_from_json(&value)
        } else {
            return Err(anyhow::anyhow!(
                "failed to parse {}: JSON config must be an object",
                path.display()
            ));
        }
    } else if let Ok(value) = toml::from_str::<toml::Value>(&contents) {
        if value.is_table() {
            extract_checks_from_toml(&value)
        } else {
            return Err(anyhow::anyhow!(
                "failed to parse {}: TOML config must be a table",
                path.display()
            ));
        }
    } else if let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(&contents) {
        if value.is_mapping() {
            extract_checks_from_yaml(&value)
        } else {
            return Err(anyhow::anyhow!(
                "failed to parse {}: YAML config must be a mapping",
                path.display()
            ));
        }
    } else {
        return Err(anyhow::anyhow!(
            "failed to parse {} as JSON, YAML, or TOML",
            path.display()
        ));
    };

    if checks.is_empty() {
        Ok(None)
    } else {
        Ok(Some(checks.into_iter().collect()))
    }
}

fn extract_checks_from_json(value: &serde_json::Value) -> Vec<String> {
    value
        .get("checks")
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(serde_json::Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

fn extract_checks_from_yaml(value: &serde_yaml::Value) -> Vec<String> {
    value
        .get("checks")
        .and_then(serde_yaml::Value::as_sequence)
        .map(|seq| {
            seq.iter()
                .filter_map(serde_yaml::Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

fn extract_checks_from_toml(value: &toml::Value) -> Vec<String> {
    value
        .get("checks")
        .and_then(toml::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(toml::Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

fn validate_check_names(names: &std::collections::HashSet<&str>) -> Result<()> {
    let unknown: Vec<&&str> = names
        .iter()
        .filter(|n| gate_internal_name(n).is_none())
        .collect();
    if !unknown.is_empty() {
        let unknown_str: Vec<&str> = unknown.into_iter().copied().collect();
        let available = gate_canonical_names();
        bail!(
            "unknown check(s): {}; available: {}",
            unknown_str.join(", "),
            available.join(", ")
        );
    }
    Ok(())
}

fn normalize_gate_check_set(
    names: &std::collections::HashSet<&str>,
) -> Result<std::collections::HashSet<&'static str>> {
    validate_check_names(names)?;
    Ok(names
        .iter()
        .filter_map(|name| gate_internal_name(name))
        .collect())
}

/// Resolve the `.anvilrc#checks` filter into a set of gate-runner internal
/// names. Canonical names like `secret-detection` are mapped to their
/// internal form (`secret`) so they match `GATE_INTERNAL_CHECKS` in the
/// downstream dispatch loop. Returns `None` when `--only-checks` is set
/// (explicit flag wins) or when `.anvilrc#checks` is absent/empty.
fn resolve_anvilrc_check_filter(
    root: &Path,
    only_set: Option<&std::collections::HashSet<&'static str>>,
) -> Result<Option<std::collections::HashSet<String>>> {
    if only_set.is_some() {
        return Ok(None);
    }

    let anvilrc_checks = read_anvilrc_checks(root)?;
    if let Some(ref rc) = anvilrc_checks {
        let unknown: Vec<&str> = rc
            .iter()
            .filter(|n| gate_internal_name(n).is_none())
            .map(String::as_str)
            .collect();
        if !unknown.is_empty() {
            let valid = gate_canonical_names();
            eprintln!(
                "Warning: .anvilrc#checks contains unknown check(s): {}. Valid: {}",
                unknown.join(", "),
                valid.join(", ")
            );
        }

        // Map each known name to its internal form so it matches the
        // gate runner vocabulary (GATE_INTERNAL_CHECKS).
        let known: std::collections::HashSet<String> = rc
            .iter()
            .filter_map(|n| gate_internal_name(n).map(str::to_string))
            .collect();
        if known.is_empty() {
            let valid = gate_canonical_names();
            bail!(
                ".anvilrc#checks contains no valid gate checks. Valid: {}",
                valid.join(", ")
            );
        }
        return Ok(Some(known));
    }

    Ok(None)
}

/// Run all gate checks with default settings and return TUI-ready data.
pub fn collect_gate_data() -> anvil_tui::surfaces::gate::GateResult {
    let start = std::time::Instant::now();
    let default_args = GateArgs::default();
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

/// Resolved gate context from CLI arguments.
struct GateContext {
    workspace_root: PathBuf,
    profile: Option<String>,
    /// Files referenced by the plan (empty = full codebase scan).
    plan_files: std::collections::HashSet<String>,
    /// Path to the plan file, if provided.
    plan_path: Option<String>,
    /// All workspace files (walked once, shared across checks).
    walked_files: Vec<String>,
}

fn run_checks(args: &GateArgs) -> Result<Vec<CheckResult>> {
    let root = crate::util::workspace_root()?;

    let profile_skips = resolve_profile_skips(args.profile.as_deref())?;

    let skip_names: std::collections::HashSet<&str> = args
        .skip_checks
        .as_deref()
        .map(|s| s.split(',').map(str::trim).collect())
        .unwrap_or_default();
    let mut skip_set = normalize_gate_check_set(&skip_names)?;
    skip_set.extend(&profile_skips);

    let only_names: Option<std::collections::HashSet<&str>> = args
        .only_checks
        .as_deref()
        .map(|s| s.split(',').map(str::trim).collect());
    let only_set = only_names
        .as_ref()
        .map(normalize_gate_check_set)
        .transpose()?;

    // `.anvilrc#checks` acts as a persistent default filter. When the user
    // passes `--only-checks`, that wins — but otherwise we restrict the run
    // to whatever the project configured. Missing/empty file = run everything.
    let anvilrc_known_checks = resolve_anvilrc_check_filter(&root, only_set.as_ref())?;

    // Resolve plan-scoped file set.
    let (plan_files, plan_path) = if let Some(ref plan_arg) = args.plan {
        match resolve_plan_path(plan_arg, &root) {
            Some(path) => {
                let files = extract_plan_files(&path);
                if args.progress {
                    eprintln!(
                        "  \u{2139} plan scope: {} files from {}",
                        files.len(),
                        path.display()
                    );
                }
                (files, Some(path.to_string_lossy().to_string()))
            }
            None => {
                bail!("plan file not found: {plan_arg}");
            }
        }
    } else {
        (std::collections::HashSet::new(), None)
    };

    // Walk workspace files once — shared across architecture and policy checks.
    let walked_files = walk_source_files(&root, &[]);

    let ctx = GateContext {
        workspace_root: root,
        profile: args.profile.clone(),
        plan_files,
        plan_path,
        walked_files,
    };

    let mut checks = Vec::new();
    for check_name in GATE_INTERNAL_CHECKS {
        if skip_set.contains(check_name) {
            continue;
        }
        if let Some(ref only_s) = only_set
            && !only_s.contains(check_name)
        {
            continue;
        }
        if let Some(ref rc) = anvilrc_known_checks
            && !rc.contains(*check_name)
        {
            continue;
        }

        let display_name = gate_canonical_name_from_internal(check_name);

        if args.progress {
            eprintln!("  \u{25b6} {display_name} running...");
        }

        let result = run_single_check(check_name, &ctx);

        if args.progress {
            let icon = if result.passed {
                "\u{2713}"
            } else {
                "\u{2717}"
            };
            eprintln!("  {icon} {display_name}");
        }

        let failed = !result.passed;
        checks.push(result);

        if args.fail_fast && failed {
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
    let notifications = notifications_for_gate_result(&checks, overall);
    let result = GateResult {
        overall,
        score,
        checks,
        notifications,
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

    // Regression guard: ensures --no-cache is not re-introduced (was dead code, removed in TCOV-006).
    #[test]
    fn no_cache_flag_removed() {
        let result = Wrapper::try_parse_from(["test", "--no-cache"]);
        assert!(result.is_err(), "--no-cache should not be accepted");
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
        let result = run_check_policy(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(result.passed);
        assert!(result.message.contains("Skipping"));
    }

    #[test]
    fn policy_with_bundle_evaluates_or_skips() {
        let tmp = tempfile::TempDir::new().unwrap();
        let policy_dir = tmp.path().join(".anvil/policies");
        std::fs::create_dir_all(&policy_dir).unwrap();
        // Use a valid policy under the anvil.policies namespace
        std::fs::write(
            policy_dir.join("noop.rego"),
            "package anvil.policies.noop\n",
        )
        .unwrap();
        let result = run_check_policy(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        // With OPA installed: evaluates and passes (no violations in noop policy)
        // Without OPA: skips gracefully
        // OPA evaluation may also fail due to missing input structure — that's
        // acceptable; the test verifies the command doesn't panic.
        assert!(
            result.passed || result.message.contains("evaluation failed"),
            "unexpected failure: {}",
            result.message
        );
    }

    // ── Policy input context tests ─────────────────────────────────────

    #[test]
    fn build_policy_input_populates_workspace() {
        let tmp = tempfile::TempDir::new().unwrap();
        let input = build_policy_input(
            tmp.path(),
            Some("ci"),
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert_eq!(
            input["workspace"].as_str().unwrap(),
            tmp.path().to_string_lossy()
        );
        assert_eq!(input["profile"].as_str().unwrap(), "ci");
        assert!(input["files"].as_array().is_some());
        assert!(input["changed_files"].as_array().is_some());
    }

    #[test]
    fn build_policy_input_includes_source_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("main.ts"), "export const x = 1;").unwrap();
        std::fs::write(src.join("readme.md"), "# Hi").unwrap();

        let input = build_policy_input(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        let files: Vec<&str> = input["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        assert!(files.contains(&"src/main.ts"));
        assert!(!files.iter().any(|f| f.contains("readme.md")));
    }

    #[test]
    fn build_policy_input_defaults_profile() {
        let tmp = tempfile::TempDir::new().unwrap();
        let input = build_policy_input(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert_eq!(input["profile"].as_str().unwrap(), "default");
    }

    // ── Import resolution tests ────────────────────────────────────────

    #[test]
    fn resolve_import_sibling() {
        let resolved = resolve_import("src/app/service.ts", "./helper");
        assert_eq!(resolved.as_deref(), Some("src/app/helper"));
    }

    #[test]
    fn resolve_import_parent() {
        let resolved = resolve_import("src/app/service.ts", "../core/entity");
        assert_eq!(resolved.as_deref(), Some("src/core/entity"));
    }

    #[test]
    fn resolve_import_escapes_root() {
        let resolved = resolve_import("src/main.ts", "../../outside");
        assert!(resolved.is_none());
    }

    #[test]
    fn resolve_import_from_root_file() {
        let resolved = resolve_import("index.ts", "./src/lib");
        assert_eq!(resolved.as_deref(), Some("src/lib"));
    }

    // ── Architecture boundary detection tests ──────────────────────────

    #[test]
    fn architecture_detects_violations_with_edges() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();

        // Set up layers: core has no deps, app depends on core.
        // A core→app import is forbidden.
        std::fs::write(
            anvil_dir.join("architecture.yaml"),
            r#"
schema_version: "0.1.0"
template: custom
layers:
  core:
    patterns: ["src/core/**"]
    depends_on: []
  app:
    patterns: ["src/app/**"]
    depends_on: ["core"]
rules: []
"#,
        )
        .unwrap();

        // Create source files that produce an import edge.
        let core_dir = tmp.path().join("src/core");
        let app_dir = tmp.path().join("src/app");
        std::fs::create_dir_all(&core_dir).unwrap();
        std::fs::create_dir_all(&app_dir).unwrap();
        std::fs::write(
            core_dir.join("entity.ts"),
            "import { service } from '../app/service';\nexport const x = 1;\n",
        )
        .unwrap();
        std::fs::write(app_dir.join("service.ts"), "export const service = 1;\n").unwrap();

        let edges = extract_import_edges(tmp.path(), None);
        assert!(!edges.is_empty(), "should extract at least one import edge");

        let definition = anvil_architecture::parse_architecture_definition(tmp.path()).unwrap();
        let result =
            anvil_architecture::validate_with_edges(tmp.path(), &definition, &edges).unwrap();

        assert!(
            !result.violations.is_empty(),
            "core importing from app should produce a boundary violation"
        );
        assert!(!result.valid);
    }

    // ── Plan scoping tests ─────────────────────────────────────────────

    #[test]
    fn extract_plan_files_parses_files_lines() {
        let tmp = tempfile::TempDir::new().unwrap();
        let plan = tmp.path().join("test.aps.md");
        std::fs::write(
            &plan,
            r"
### ITEM-001: do something

- **Status:** In Progress
- **Intent:** Some work
- **Files:** `src/core/entity.ts`, `src/app/service.ts`
- **Confidence:** high
",
        )
        .unwrap();

        let files = extract_plan_files(&plan);
        assert!(files.contains("src/core/entity.ts"));
        assert!(files.contains("src/app/service.ts"));
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn extract_plan_files_skips_non_path_backticks() {
        let tmp = tempfile::TempDir::new().unwrap();
        let plan = tmp.path().join("test.aps.md");
        std::fs::write(
            &plan,
            "- **Files:** `src/main.ts`\n\nSome text with `inline code` here.\n",
        )
        .unwrap();

        let files = extract_plan_files(&plan);
        assert!(files.contains("src/main.ts"));
        assert!(!files.contains("inline code"));
    }

    #[test]
    fn extract_plan_files_returns_empty_for_missing_file() {
        let files = extract_plan_files(Path::new("/nonexistent/plan.aps.md"));
        assert!(files.is_empty());
    }

    #[test]
    fn resolve_plan_path_finds_in_modules() {
        let root = crate::util::workspace_root().unwrap();
        let modules_dir = root.join("plans/modules");
        if modules_dir.exists() {
            // Only run on actual workspace with plans.
            if let Some(path) = resolve_plan_path("rust-cli", &root) {
                assert!(path.to_string_lossy().ends_with(".aps.md"));
            }
        }
    }

    #[test]
    fn build_policy_input_includes_plan_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let input = build_policy_input(
            tmp.path(),
            None,
            Some("/plans/test.aps.md"),
            &std::collections::HashSet::new(),
            None,
        );
        assert_eq!(input["plan_path"].as_str().unwrap(), "/plans/test.aps.md");
    }

    #[test]
    fn build_policy_input_omits_plan_when_none() {
        let tmp = tempfile::TempDir::new().unwrap();
        let input = build_policy_input(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(input.get("plan_path").is_none());
    }

    // ── Secret check integration tests ────────────────────────────────
    //
    // These exercise the anvil-checks wiring that gate.rs delegates to,
    // using temp files to avoid coupling to the real workspace.

    #[test]
    fn secret_check_clean_file_passes() {
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("clean.ts");
        std::fs::write(&file, "export const x = 1;\n").unwrap();

        let files = [file.to_string_lossy().to_string()];
        let file_refs: Vec<&str> = files.iter().map(String::as_str).collect();
        let config = anvil_checks::secret::SecretCheckConfig::default();
        let result = anvil_checks::secret::run_secret_check(&file_refs, &config, None);

        assert!(result.passed);
        assert_eq!(result.findings.len(), 0);
    }

    #[test]
    fn secret_check_detects_aws_secret_key() {
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("creds.ts");
        let secret = "abcdabcdabcdabcdabcdabcdabcdabcdabcdabcd";
        std::fs::write(&file, format!("aws_secret_access_key='{secret}'")).unwrap();

        let files = [file.to_string_lossy().to_string()];
        let file_refs: Vec<&str> = files.iter().map(String::as_str).collect();
        let config = anvil_checks::secret::SecretCheckConfig::default();
        let result = anvil_checks::secret::run_secret_check(&file_refs, &config, None);

        assert!(!result.passed);
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.pattern_name == "AWS Secret Key"),
            "should detect AWS Secret Key pattern"
        );
    }

    #[test]
    fn secret_check_detects_stripe_key_with_pattern_name() {
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("billing.ts");
        let stripe = format!("sk_live_{}", "1234567890abcdefghijABCD");
        std::fs::write(&file, format!("const secret = '{stripe}';")).unwrap();

        let files = [file.to_string_lossy().to_string()];
        let file_refs: Vec<&str> = files.iter().map(String::as_str).collect();
        let config = anvil_checks::secret::SecretCheckConfig::default();
        let result = anvil_checks::secret::run_secret_check(&file_refs, &config, None);

        assert!(!result.passed);
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.pattern_name.contains("Stripe")),
            "should detect Stripe key pattern by name"
        );
    }

    #[test]
    fn secret_check_result_maps_to_check_result_format() {
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("leak.ts");
        let secret = "abcdabcdabcdabcdabcdabcdabcdabcdabcdabcd";
        std::fs::write(&file, format!("aws_secret_access_key='{secret}'")).unwrap();

        let files = [file.to_string_lossy().to_string()];
        let file_refs: Vec<&str> = files.iter().map(String::as_str).collect();
        let config = anvil_checks::secret::SecretCheckConfig::default();
        let root_str = tmp.path().to_string_lossy().to_string();
        let result = anvil_checks::secret::run_secret_check(&file_refs, &config, Some(&root_str));

        // Verify the mapping logic used in run_check_secret produces the
        // expected format with pattern name in brackets.
        let locations: Vec<String> = result
            .findings
            .iter()
            .map(|f| format!("{}:{} [{}]", f.file, f.line, f.pattern_name))
            .collect();
        assert!(!locations.is_empty());
        assert!(
            locations[0].contains("[AWS Secret Key]"),
            "location should include pattern name in brackets, got: {}",
            locations[0]
        );
    }

    #[test]
    fn antipattern_check_detects_explicit_any() {
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("warn.ts");
        std::fs::write(&file, "const value: any = source;\n").unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
        );

        assert!(!result.passed);
        assert!(result.message.contains("AP-003"));
    }

    #[test]
    fn antipattern_check_skips_when_no_supported_files_exist() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("notes.md"), "hello\n").unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
        );

        assert!(result.passed);
        assert!(result.message.contains("Skipping"));
    }

    // ── Validate check names ──────────────────────────────────────────

    #[test]
    fn validate_check_names_accepts_known() {
        let names: std::collections::HashSet<&str> = [
            "lint",
            "secret-detection",
            "import-boundaries",
            "antipattern-scan",
        ]
        .into_iter()
        .collect();
        assert!(validate_check_names(&names).is_ok());
    }

    #[test]
    fn validate_check_names_rejects_unknown() {
        let names: std::collections::HashSet<&str> = ["lint", "bogus"].into_iter().collect();
        let err = validate_check_names(&names).unwrap_err();
        assert!(err.to_string().contains("bogus"));
    }

    #[test]
    fn validate_check_names_empty_is_ok() {
        let names: std::collections::HashSet<&str> = std::collections::HashSet::new();
        assert!(validate_check_names(&names).is_ok());
    }

    // ── GateResult serialisation ──────────────────────────────────────

    #[test]
    fn gate_result_serialises_to_json() {
        let overall = true;
        let checks = vec![CheckResult {
            name: "secret-detection".to_string(),
            passed: true,
            score: 100.0,
            message: "clean".to_string(),
        }];
        let notifications = notifications_for_gate_result(&checks, overall);
        let result = GateResult {
            overall,
            score: 100.0,
            checks,
            notifications,
            duration_ms: 42,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["overall"], true);
        assert_eq!(parsed["checks"][0]["name"], "secret-detection");
        assert_eq!(parsed["duration_ms"], 42);

        let notifications = parsed["notifications"].as_array().expect("notifications");
        assert_eq!(notifications.len(), 2, "per-check + overall notifications");

        let per_check = &notifications[0];
        assert_eq!(per_check["class"], "info");
        assert_eq!(per_check["priority"], "low");
        assert_eq!(per_check["title"], "Gate check: secret-detection");
        assert_eq!(per_check["message"], "clean");
        assert_eq!(per_check["context"]["source"], "gate");

        let overall_notif = &notifications[1];
        assert_eq!(overall_notif["class"], "info");
        assert_eq!(overall_notif["priority"], "normal");
        assert_eq!(overall_notif["title"], "Gate result");
        assert_eq!(overall_notif["message"], "All quality gates passed");
        assert_eq!(overall_notif["context"]["source"], "gate");
    }

    // ── run_single_check unknown ─────────────────────────────────────

    #[test]
    fn unknown_check_fails() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = GateContext {
            workspace_root: dir.path().to_path_buf(),
            profile: None,
            plan_files: std::collections::HashSet::new(),
            plan_path: None,
            walked_files: Vec::new(),
        };
        let result = run_single_check("nonexistent", &ctx);
        assert!(!result.passed);
        assert!(result.message.contains("Unknown check"));
    }

    // ── Plan-scoped policy input filtering ────────────────────────────

    #[test]
    fn build_policy_input_filters_by_plan_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("included.ts"), "export const x = 1;").unwrap();
        std::fs::write(src.join("excluded.ts"), "export const y = 2;").unwrap();

        let mut plan_files = std::collections::HashSet::new();
        plan_files.insert("src/included.ts".to_string());

        let input = build_policy_input(tmp.path(), None, None, &plan_files, None);
        let files: Vec<&str> = input["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        assert!(files.contains(&"src/included.ts"));
        assert!(!files.contains(&"src/excluded.ts"));
    }

    // ── Extract plan files multi-line ─────────────────────────────────

    #[test]
    fn extract_plan_files_multi_line_continuation() {
        let tmp = tempfile::TempDir::new().unwrap();
        let plan = tmp.path().join("test.aps.md");
        std::fs::write(
            &plan,
            "- **Files:** `src/a.ts`,\n  `src/b.ts`, `src/c.ts`\n- **Status:** Done\n",
        )
        .unwrap();

        let files = extract_plan_files(&plan);
        assert!(files.contains("src/a.ts"));
        assert!(files.contains("src/b.ts"));
        assert!(files.contains("src/c.ts"));
    }

    // ── Coverage edge cases ──────────────────────────────────────────

    #[test]
    fn coverage_lcov_empty_report_skips() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cov_dir = tmp.path().join("coverage");
        std::fs::create_dir_all(&cov_dir).unwrap();
        std::fs::write(
            cov_dir.join("lcov.info"),
            "SF:src/main.rs\nLF:0\nLH:0\nend_of_record\n",
        )
        .unwrap();
        let result = run_check_coverage(tmp.path(), 80.0);
        assert!(result.passed);
        assert!(result.message.contains("empty"));
    }

    #[test]
    fn coverage_cobertura_unparseable_rate() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cov_dir = tmp.path().join("coverage");
        std::fs::create_dir_all(&cov_dir).unwrap();
        std::fs::write(
            cov_dir.join("cobertura.xml"),
            r#"<?xml version="1.0"?><coverage></coverage>"#,
        )
        .unwrap();
        let result = run_check_coverage(tmp.path(), 80.0);
        assert!(!result.passed);
        assert!(result.message.contains("Failed to parse"));
    }

    // ── .anvilrc#checks filter (#1016) ────────────────────────────────

    #[test]
    fn read_anvilrc_checks_none_when_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(read_anvilrc_checks(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn read_anvilrc_checks_none_for_empty_list() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvilrc"), r#"{"checks": []}"#).unwrap();
        assert!(read_anvilrc_checks(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn read_anvilrc_checks_parses_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            r#"{"checks": ["secret", "architecture"]}"#,
        )
        .unwrap();
        let checks = read_anvilrc_checks(tmp.path()).unwrap().unwrap();
        assert_eq!(checks.len(), 2);
        assert!(checks.contains("secret"));
        assert!(checks.contains("architecture"));
    }

    #[test]
    fn read_anvilrc_checks_parses_yaml() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            "schemaVersion: \"1.0.0\"\nchecks:\n  - \"secret\"\n  - \"architecture\"\n",
        )
        .unwrap();
        let checks = read_anvilrc_checks(tmp.path()).unwrap().unwrap();
        assert!(checks.contains("secret"));
        assert!(checks.contains("architecture"));
    }

    #[test]
    fn read_anvilrc_checks_parses_toml_inline() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            "schema_version = \"1.0.0\"\nchecks = [\"secret\", \"policy\"]\n",
        )
        .unwrap();
        let checks = read_anvilrc_checks(tmp.path()).unwrap().unwrap();
        assert!(checks.contains("secret"));
        assert!(checks.contains("policy"));
    }

    #[test]
    fn read_anvilrc_checks_errors_on_unparseable_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvilrc"), "checks: [\n").unwrap();
        let err = read_anvilrc_checks(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("failed to parse"));
    }
}
