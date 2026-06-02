use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anvil_kernel_types::{
    Category, Diagnostic, DiagnosticSource, Location, Mode, Notification, NotificationClass,
    NotificationContext, NotificationPriority, Severity, diagnostics::KnownMode,
};
use anyhow::{Context, Result, bail};
use clap::Args;
use regex::Regex;
use serde::Serialize;

use crate::GlobalArgs;
use crate::commands::check_catalog::{
    GATE_INTERNAL_CHECKS, canonical_check_name, definition_by_internal,
    gate_canonical_name_from_internal, gate_canonical_names, gate_internal_name,
};
use crate::commands::check_guards::{WallTimeGuard, evaluate_file_presence, evaluate_wall_time};
use crate::util::is_ignored_dir_name;

#[derive(Debug, Default, Args)]
#[allow(clippy::struct_excessive_bools)]
pub struct GateArgs {
    /// Plan file to run gates against (omit for full codebase scan)
    plan: Option<String>,

    /// Gate profile: dev, ci, production, ai
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

    /// Output format: auto (default), tui, plain, json, or sarif. `json` is the
    /// `--json` alias; `sarif` emits SARIF 2.1.0 and is never auto-selected.
    #[arg(long, value_enum)]
    format: Option<crate::output::Format>,
}

impl GateArgs {
    /// True when an explicit `--format json|sarif` requests structured
    /// output, so the pre-dispatch auth gate emits a JSON envelope rather
    /// than human text. (The AI-guardrail profile's implicit JSON default
    /// is resolved later in `resolve_gate_output_mode` and is intentionally
    /// out of scope for the pre-auth check.)
    pub(crate) fn wants_structured_output(&self) -> bool {
        self.format
            .is_some_and(crate::output::Format::is_structured)
    }
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
    (
        "ai",
        "AI guardrail mode \u{2014} curated checks for AI-generated code",
        &["lint", "test", "coverage", "dependency"],
    ),
];

/// AI guardrail profile (AIGUARD-001 + AIGUARD-003).
///
/// Declares the curated rule set that the AI guardrail runs to validate
/// AI-generated changes. The profile bundles structural-governance checks
/// (architecture, policy, antipattern, secret detection, command safety)
/// into a single coherent set so external AI tools have a predictable
/// safety harness.
///
/// `--profile ai` is wired end-to-end as of AIGUARD-003: the gate
/// runner selects from [`AI_GUARDRAIL_CHECKS`] as an allow-list (not
/// the inverse skip list), `strict_config = true` converts the
/// "missing config, skipping" path into a blocking diagnostic for
/// architecture/policy/command-safety, `json_output_default = true`
/// pins JSON output for AI consumers unless the caller passes a
/// non-JSON output mode explicitly, and the JSON envelope uses the
/// canonical `anvil.diagnostic.v1` shape published by AIGUARD-002.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AiGuardrailProfile {
    /// Canonical check names included in the profile.
    pub checks: &'static [&'static str],
    /// When true, missing or invalid configuration is treated as a
    /// blocking diagnostic rather than a soft warning.
    pub strict_config: bool,
    /// When true, output defaults to structured JSON for AI consumption.
    pub json_output_default: bool,
}

/// Canonical check names included in the AI guardrail profile.
///
/// Selection rationale: every check here flags a structural concern that
/// AI-generated changes regularly trip over — secret leakage, antipatterns,
/// import-boundary violations, OPA policy breaches, and command-safety
/// rules. Lint/test/coverage/dependency are intentionally excluded:
/// they're language-toolchain concerns the host project already enforces
/// and they would push the profile past the 5s budget set out in the
/// AIGUARD acceptance criteria.
pub(crate) const AI_GUARDRAIL_CHECKS: &[&str] = &[
    "secret-detection",
    "import-boundaries",
    "antipattern-scan",
    "policy",
    "command-safety",
];

impl AiGuardrailProfile {
    /// Default AI guardrail profile.
    pub(crate) const DEFAULT: Self = Self {
        checks: AI_GUARDRAIL_CHECKS,
        strict_config: true,
        json_output_default: true,
    };

    /// Profile name as used on the CLI (`--profile ai`).
    pub(crate) const NAME: &'static str = "ai";
}

/// Return the canonical check names that make up the AI guardrail
/// profile. Used by the gate runner to filter checks when
/// `--profile ai` is selected.
pub(crate) fn ai_guardrail_profile_checks() -> &'static [&'static str] {
    AiGuardrailProfile::DEFAULT.checks
}

#[derive(Debug, Serialize)]
struct GateResult {
    overall: bool,
    score: f64,
    checks: Vec<CheckResult>,
    notifications: Vec<Notification>,
    duration_ms: u64,
}

#[derive(Debug, Default, Serialize)]
struct CheckResult {
    name: String,
    passed: bool,
    score: f64,
    message: String,
    /// CIB-011 / #1803 — true when the check is unavailable on this
    /// repo because its configuration is missing. Excluded from the
    /// gate score denominator and rendered as `CONFIG NEEDED` with a
    /// `next:` hint, rather than as a `FAIL`. Skipped from JSON output
    /// when false so the schema stays additive.
    // serde idiom: skip the field in JSON when false. Equivalent to
    // `if !*x` — the `std::ops::Not::not` path lets serde call the
    // free function without needing a custom `is_false` helper.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    requires_config: bool,
}

/// CIB-011 / #1803 — aggregation result for the gate render + envelope.
///
/// Config-gap checks (where the check could not run because its config
/// is absent under `--profile ai` strict mode) are excluded from the
/// score denominator: a fresh repo with three missing configs and no
/// actual failures must read as `2/2 available passed (100%)`, not
/// `1/5 passed (20%)`. The pre-CIB-011 grading was the most-cited
/// reason new users believed Anvil was broken on first contact.
#[derive(Debug, Clone, Copy)]
struct GateAggregate {
    passed_count: usize,
    available_total: usize,
    config_gaps: usize,
    overall: bool,
    score: f64,
}

fn aggregate_gate_outcome(checks: &[CheckResult]) -> GateAggregate {
    let available: Vec<&CheckResult> = checks.iter().filter(|c| !c.requires_config).collect();
    let available_total = available.len();
    let passed_count = available.iter().filter(|c| c.passed).count();
    let config_gaps = checks.len() - available_total;
    let overall = available.iter().all(|c| c.passed);
    #[allow(clippy::cast_precision_loss)]
    let score = if available_total > 0 {
        (passed_count as f64 / available_total as f64) * 100.0
    } else {
        // No real checks ran — there is nothing to fail, so the gate
        // is vacuously green. The render layer surfaces the config
        // gaps alongside this so the user is not misled into thinking
        // a fully-passing 100% means "everything is checked".
        100.0
    };
    GateAggregate {
        passed_count,
        available_total,
        config_gaps,
        overall,
        score,
    }
}

/// Filename of the persisted last-gate-run snapshot under `.anvil/`.
const GATE_SNAPSHOT_FILE: &str = "gates.json";

/// A display-ready view of the last gate run, persisted to `.anvil/gates.json`
/// for the `gate-summary` TUI dashboard to bind against (#2242).
///
/// This is intentionally **not** the internal [`GateResult`]: the json-render
/// dashboard components read *string* props (`MetricCard.value`,
/// `StatusBadge.status`/`label`) and `Table.rows` as an array-of-arrays, so the
/// snapshot pre-formats values into the exact shapes a spec's `$data` paths bind
/// to (`gates.status`, `gates.checkRows`, …). camelCase to match json-render
/// prop conventions.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GateSnapshot {
    /// `"pass"`, `"warn"` (passed with config gaps), or `"fail"` — drives
    /// `StatusBadge.status`.
    status: &'static str,
    /// e.g. `"PASSED — score 92/100"` — `StatusBadge.label`.
    status_label: String,
    /// Raw 0–100 score, for machine consumers.
    score: f64,
    /// Available checks run, as a display string (`MetricCard.value`).
    checks_run: String,
    /// Count of attention items (failed + config-needed), display string.
    warnings: String,
    /// Duration in seconds to one decimal place (e.g. `"4.2"`), display string.
    duration_seconds: String,
    /// Per-check rows `[name, status, score, message]` for `Table.rows`.
    check_rows: Vec<Vec<String>>,
    /// Attention items for a `WarningList`.
    warning_list: Vec<SnapshotWarning>,
}

/// One attention item (failed or config-needed check) in [`GateSnapshot`].
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotWarning {
    /// `"error"` (failed) or `"warn"` (config needed).
    severity: &'static str,
    /// `"<check>: <message>"`.
    message: String,
}

impl GateSnapshot {
    fn from_result(result: &GateResult, aggregate: &GateAggregate) -> Self {
        let check_rows = result
            .checks
            .iter()
            .map(|c| {
                let status = if c.requires_config {
                    "config"
                } else if c.passed {
                    "passed"
                } else {
                    "failed"
                };
                vec![
                    c.name.clone(),
                    status.to_owned(),
                    format!("{:.0}", c.score),
                    c.message.clone(),
                ]
            })
            .collect();

        let warning_list: Vec<SnapshotWarning> = result
            .checks
            .iter()
            .filter_map(|c| {
                // Attention items: a real failure, or a check that could not run
                // for want of config. Passing checks are not warnings.
                let severity = if c.requires_config {
                    "warn"
                } else if !c.passed {
                    "error"
                } else {
                    return None;
                };
                let message = if c.message.is_empty() {
                    c.name.clone()
                } else {
                    format!("{}: {}", c.name, c.message)
                };
                Some(SnapshotWarning { severity, message })
            })
            .collect();

        // Tri-state: a failure is "fail"; an overall pass that still has
        // attention items (config gaps) is "warn"; a clean pass is "pass".
        let (status, status_word) = if !result.overall {
            ("fail", "FAILED")
        } else if warning_list.is_empty() {
            ("pass", "PASSED")
        } else {
            ("warn", "PASSED")
        };
        let status_label = format!("{status_word} — score {:.0}/100", result.score);

        Self {
            status,
            status_label,
            score: result.score,
            checks_run: aggregate.available_total.to_string(),
            warnings: warning_list.len().to_string(),
            // Tenths of a second via integer math (avoids a lossy f64 cast): a
            // sub-second run shows e.g. "0.4", not a misleading "0".
            duration_seconds: format!(
                "{}.{}",
                result.duration_ms / 1000,
                (result.duration_ms % 1000) / 100
            ),
            check_rows,
            warning_list,
        }
    }
}

/// Persist the last gate run to `.anvil/gates.json` for the dashboard.
///
/// Best-effort: a write failure is logged at debug and otherwise ignored, so it
/// can never change the gate's exit code — persistence is a side effect, and the
/// gate stays "warnings over blocks, exit 0 by default".
fn persist_gate_snapshot(result: &GateResult, aggregate: &GateAggregate) {
    let Ok(root) = crate::util::workspace_root() else {
        tracing::debug!("gate snapshot: workspace root unresolved; skipping persist");
        return;
    };
    let dir = root.join(".anvil");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::debug!(error = %e, "gate snapshot: could not create .anvil/; skipping persist");
        return;
    }
    let snapshot = GateSnapshot::from_result(result, aggregate);
    let json = match serde_json::to_vec_pretty(&snapshot) {
        Ok(json) => json,
        Err(e) => {
            tracing::debug!(error = %e, "gate snapshot: serialize failed; skipping persist");
            return;
        }
    };
    if let Err(e) = crate::util::atomic_write(&dir.join(GATE_SNAPSHOT_FILE), &json) {
        tracing::debug!(error = %e, "gate snapshot: write to .anvil/gates.json failed");
    }
}

/// CIB-011 / #1803 — actionable next-step hint shown beneath a
/// config-gap check. Names match the internal dispatch keys in
/// `run_single_check`; the hints point at the canonical onboarding
/// docs so the user can move from "Anvil is broken" to a working
/// configuration without guessing.
fn config_gap_next_hint(name: &str) -> &'static str {
    match name {
        "architecture" => {
            "Create .anvil/architecture.yaml — see docs/public/anvil/tutorials/architecture.md"
        }
        "policy" => {
            "Create a .rego rule under .anvil/policies/ — see docs/public/anvil/tutorials/policies.md"
        }
        "command-safety" => {
            "Pass --plan <path/to/plan.aps.md> to anvil gate so command-safety has commands to analyse"
        }
        _ => "See `anvil gate --help` and the public docs for setup steps",
    }
}

fn notifications_for_gate_result(checks: &[CheckResult], overall: bool) -> Vec<Notification> {
    let gate_context = || NotificationContext {
        file: None,
        source: Some("gate".to_string()),
    };

    let mut notifications: Vec<Notification> = checks
        .iter()
        .map(|check| {
            // CIB-011 / #1803 — config-gap checks emit a Normal-priority
            // info notification carrying the `next:` hint rather than a
            // high-priority Failure (the check could not run, but the
            // user is not in a failing state until they configure).
            let class = if check.requires_config || check.passed {
                NotificationClass::Info
            } else {
                NotificationClass::Failure
            };
            let priority = if check.requires_config {
                NotificationPriority::Normal
            } else if check.passed {
                NotificationPriority::Low
            } else {
                NotificationPriority::High
            };
            Notification::new(
                class,
                priority,
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
            requires_config: false,
        },
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            CheckResult {
                name: name.to_string(),
                passed: false,
                score: 0.0,
                message: format!("Lint errors found\n{stdout}\n{stderr}"),
                requires_config: false,
            }
        }
        Err(e) => CheckResult {
            name: name.to_string(),
            passed: false,
            score: 0.0,
            message: format!("Failed to run lint: {e}"),
            requires_config: false,
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
            requires_config: false,
        },
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            CheckResult {
                name: name.to_string(),
                passed: false,
                score: 0.0,
                message: format!("Tests failed\n{stdout}\n{stderr}"),
                requires_config: false,
            }
        }
        Err(e) => CheckResult {
            name: name.to_string(),
            passed: false,
            score: 0.0,
            message: format!("Failed to run tests: {e}"),
            requires_config: false,
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

/// Maximum directory depth for the secret scan walk. Prevents runaway
/// recursion into deeply nested or symlink-heavy trees.
const SECRET_SCAN_MAX_DEPTH: usize = 20;

fn run_check_secret(
    name: &str,
    root: &Path,
    plan_files: &std::collections::HashSet<String>,
) -> CheckResult {
    let mut files_to_scan: Vec<String> = Vec::new();

    // SCAN-001: gate-secret discovery uses `ignore::WalkBuilder`. Per-file
    // scans run on the rayon pool inside `run_secret_check` (rolled out as
    // part of this slice). The depth cap is preserved for full-codebase
    // scans only; plan-scoped runs must reach explicitly referenced files
    // regardless of nesting depth.
    let mut walker_builder = ignore::WalkBuilder::new(root);
    walker_builder
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !SECRET_SCAN_IGNORE.iter().any(|&ig| name == ig)
        });
    if plan_files.is_empty() {
        walker_builder.max_depth(Some(SECRET_SCAN_MAX_DEPTH));
    }
    let walker = walker_builder.build();

    for entry in walker
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
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

    let pattern_errors_suffix = if result.pattern_errors.is_empty() {
        String::new()
    } else {
        format!(
            "\n\n⚠ {} custom secret pattern(s) failed to compile and were skipped:\n{}",
            result.pattern_errors.len(),
            result
                .pattern_errors
                .iter()
                .map(|err| format!("  - {err}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    if result.passed {
        CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: format!("No hardcoded secrets found{pattern_errors_suffix}"),
            requires_config: false,
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
                "Potential secrets found in {} location(s):\n{}{pattern_errors_suffix}",
                result.findings.len(),
                locations.join("\n")
            ),
            requires_config: false,
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
            requires_config: false,
        };
    }

    if result.passed {
        CheckResult {
            name: name.to_string(),
            passed: true,
            score: f64::from(result.score),
            message: result.message,
            requires_config: false,
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
            requires_config: false,
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
                        requires_config: false,
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
                    requires_config: false,
                }
            }
            Err(e) => CheckResult {
                name: "coverage".to_string(),
                passed: false,
                score: 0.0,
                message: format!("Failed to read lcov.info: {e}"),
                requires_config: false,
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
                            requires_config: false,
                        }
                    }
                    None => CheckResult {
                        name: "coverage".to_string(),
                        passed: false,
                        score: 0.0,
                        message: "Failed to parse line-rate from cobertura.xml".to_string(),
                        requires_config: false,
                    },
                }
            }
            Err(e) => CheckResult {
                name: "coverage".to_string(),
                passed: false,
                score: 0.0,
                message: format!("Failed to read cobertura.xml: {e}"),
                requires_config: false,
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
            requires_config: false,
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
            requires_config: false,
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
                    requires_config: false,
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
            requires_config: false,
        }
    } else {
        CheckResult {
            name: "dependency".to_string(),
            passed: false,
            score: 0.0,
            message: format!("Blocked dependencies found: {}", blocked_found.join(", ")),
            requires_config: false,
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
///
/// SCAN-001: discovery routed through `ignore::WalkBuilder` to share the
/// welcome-screen walker shape. The per-file boundary scan downstream
/// already parallelises on rayon.
fn walk_source_files(project_root: &Path, extensions: &[&str]) -> Vec<String> {
    let walker = ignore::WalkBuilder::new(project_root)
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            if e.file_type().is_some_and(|ft| ft.is_dir()) {
                return !is_ignored_dir_name(&name);
            }
            true
        })
        .build();

    let mut files = Vec::new();
    for entry in walker.filter_map(std::result::Result::ok) {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
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
            requires_config: false,
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
                requires_config: false,
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
            requires_config: false,
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
            requires_config: false,
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
            requires_config: false,
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
                    requires_config: false,
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
                    requires_config: false,
                }
            }
        }
        Err(anvil_policy::evaluator::EvalError::OpaNotAvailable) => CheckResult {
            name: "policy".to_string(),
            passed: true,
            score: 100.0,
            message: "OPA not installed. Skipping policy evaluation.".to_string(),
            requires_config: false,
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
            requires_config: false,
        },
        Err(e) => CheckResult {
            name: "policy".to_string(),
            passed: false,
            score: 0.0,
            message: format!("Policy evaluation failed: {e}"),
            requires_config: false,
        },
    }
}

fn run_single_check(name: &str, ctx: &GateContext) -> CheckResult {
    let root = &ctx.workspace_root;

    // OPSUP-006 — file-presence guard. A check that declares file-shape
    // patterns short-circuits when none of the walked workspace files
    // match. All current core checks declare none and therefore always
    // run (Unguarded). Surface/pack checks added under Track 3 and Track
    // 4 will opt in by populating `file_shape_globs` on their
    // CheckDefinition.
    let definition = definition_by_internal(name);
    if let Some(def) = definition {
        let presence = evaluate_file_presence(def.file_shape_globs, &ctx.walked_files);
        if !presence.should_run() {
            // Use a "No files in scope" prefix rather than "Skipping" so
            // the file-presence short-circuit cannot be mistaken for the
            // missing-config skip pattern that `is_skipped_for_missing_config`
            // matches on. The two are semantically different (no work to
            // do vs. no config to evaluate) and the AI strict-config mode
            // must only elevate the latter.
            return CheckResult {
                name: gate_canonical_name_from_internal(name),
                passed: true,
                score: 100.0,
                message: format!(
                    "No files in scope for {}: no workspace files match declared shapes ({})",
                    def.canonical_name,
                    def.file_shape_globs.join(", "),
                ),
                requires_config: false,
            };
        }
    }

    // OPSUP-006 — measure elapsed wall-time so the post-flight guard can
    // surface a precise overrun reason. Side-effect-free if no budget is
    // declared (every current core check).
    let started = std::time::Instant::now();

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
        "command-safety" => run_check_command_safety(name, root, ctx.plan_path.as_deref()),
        _ => CheckResult {
            name: name.to_string(),
            passed: false,
            score: 0.0,
            message: format!("Unknown check: {name}"),
            requires_config: false,
        },
    };

    if let Some(def) = definition {
        let wall_time = evaluate_wall_time(def.wall_time_soft_budget_secs, started.elapsed());
        if let WallTimeGuard::Exceeded { .. } = &wall_time
            && let Some(reason) = wall_time.timeout_reason()
        {
            // Append the timeout reason so the overrun is surfaced
            // without losing the check's own message. The check itself is
            // not cancelled — Rust threads cannot be safely pre-empted —
            // but the reason makes the budget breach actionable.
            if result.message.is_empty() {
                result.message = format!("{}: {}", def.canonical_name, reason);
            } else {
                result.message = format!("{} ({reason})", result.message);
            }
        }
    }

    // AI guardrail strict-config (CIB-011 / #1803): missing/invalid
    // config marks the check as a config-gap with an actionable
    // `next:` hint, rather than flipping the soft skip into a hard
    // FAIL. Score is graded against **available** checks — a fresh
    // repo with no project config and no actual violations reads as
    // a green run with three config-needed notifications, not a 20%
    // score that screams "Anvil is broken".
    //
    // Architecture and policy checks return passed=true with a
    // "Skipping" message when their config files are absent — that's
    // the precise signal we mark as config-gap.
    if ctx.strict_config && result.passed && is_skipped_for_missing_config(name, &result.message) {
        result.requires_config = true;
        result.message = format!("{}\n  next: {}", result.message, config_gap_next_hint(name));
    }

    result.name = gate_canonical_name_from_internal(name);
    result
}

/// Detect the canonical "no project config found, skipping" signal
/// emitted by architecture, policy, and command-safety checks. Used by
/// the AI guardrail's strict-config flag (CIB-011 / #1803) to mark
/// the check as a **config-gap** (rendered as `CONFIG NEEDED` with a
/// `next:` hint, excluded from the score denominator) rather than to
/// flip the soft skip into a hard FAIL. The pre-CIB-011 behaviour was
/// to elevate to a blocking diagnostic; that produced a "1/5 passed,
/// score: 20%" UX on fresh repos and is no longer the contract.
///
/// This intentionally distinguishes **missing project config** (which
/// strict mode marks as a config-gap) from **missing host tooling**
/// like a missing OPA binary (which is an environment problem, not a
/// project posture problem — left as a normal soft skip). The two
/// were previously conflated via a substring match on "Skipping",
/// with the result that any developer or CI runner without OPA in
/// PATH would get a blocked AI-guardrail run.
fn is_skipped_for_missing_config(name: &str, message: &str) -> bool {
    match name {
        "architecture" => message.contains("Skipping"),
        "policy" => {
            // OPA-not-installed is host tooling, not project config — do
            // not mark it as a config-gap under strict mode.
            message.contains("Skipping") && !message.contains("OPA not installed")
        }
        "command-safety" => {
            // Two project-config gaps map to a strict-mode config-gap:
            //   * "Skipping" — the check is disabled via config.
            //   * "No commands to analyse" — the gate ran without a plan
            //     file at all, so the command-safety guarantee is empty.
            message.contains("Skipping") || message.contains("No commands to analyse")
        }
        _ => false,
    }
}

/// Dispatch the command-safety check from `anvil-checks`.
///
/// The plan file (if any) is parsed for fenced shell-script blocks; the
/// commands extracted are evaluated against the default rule set. When
/// no plan is provided the check has nothing to evaluate and reports as
/// skipped (passed with a clear message).
fn run_check_command_safety(name: &str, root: &Path, plan_path: Option<&str>) -> CheckResult {
    use anvil_checks::command_safety::{
        CommandSafetyCheckContext, run_command_safety_check,
        types::{ScriptChange, ScriptChangeType, ScriptPlan},
    };

    let plan = match plan_path {
        Some(raw) => {
            let path = Path::new(raw);
            match std::fs::read_to_string(path) {
                Ok(content) => Some(ScriptPlan {
                    proposed_changes: vec![ScriptChange {
                        change_type: ScriptChangeType::ScriptExecute,
                        description: Some(content),
                        path: Some(raw.to_string()),
                    }],
                }),
                Err(e) => {
                    // Treat unreadable plans as a check failure rather than
                    // silently passing as "no commands to analyse" — under
                    // --profile ai (strict mode) this would otherwise mask
                    // permission errors and CI-only IO failures behind a
                    // green gate.
                    return CheckResult {
                        name: name.to_string(),
                        passed: false,
                        score: 0.0,
                        message: format!("failed to read plan file '{}': {e}", path.display()),
                        requires_config: false,
                    };
                }
            }
        }
        None => None,
    };

    let context = CommandSafetyCheckContext {
        plan,
        check_config: None,
        workspace_root: Some(root.to_string_lossy().into_owned()),
    };

    let result = run_command_safety_check(&context);

    if result.skipped {
        return CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: "Command-safety check disabled. Skipping.".to_string(),
            requires_config: false,
        };
    }

    if result.passed {
        let message = if result.message.is_empty() {
            "No unsafe commands detected".to_string()
        } else {
            result.message
        };
        CheckResult {
            name: name.to_string(),
            passed: true,
            score: f64::from(result.score),
            message,
            requires_config: false,
        }
    } else {
        let mut details: Vec<String> = result
            .blocked
            .iter()
            .map(|f| {
                format!(
                    "[blocked:{}] {} \u{2014} {}",
                    f.rule_id, f.command, f.reason
                )
            })
            .collect();
        details.extend(
            result
                .warnings
                .iter()
                .map(|f| format!("[warn:{}] {} \u{2014} {}", f.rule_id, f.command, f.reason)),
        );

        let header = if result.message.is_empty() {
            format!(
                "{} blocked, {} warning(s)",
                result.blocked.len(),
                result.warnings.len()
            )
        } else {
            result.message
        };

        let message = if details.is_empty() {
            header
        } else {
            format!("{header}\n{}", details.join("\n"))
        };

        CheckResult {
            name: name.to_string(),
            passed: false,
            score: f64::from(result.score),
            message,
            requires_config: false,
        }
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
    println!("Usage: anvil gate [plan] --profile <name>");
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

/// Resolve a profile's skip list to canonical gate-runner internal names.
///
/// Profile skip-list entries can use either canonical names like
/// `secret-detection` or internal names like `secret`. Routing them
/// through [`gate_internal_name`] guarantees `--profile <name>` and
/// `--skip-checks <name>` use the same vocabulary downstream.
///
/// Any entry that does not resolve through the catalog is treated as a
/// hard error rather than silently dropped — a typo in `PROFILES` (or
/// any future profile definition) used to fail open, letting the
/// supposedly-skipped check run anyway.
fn resolve_profile_skip_set(
    profile: Option<&str>,
) -> Result<std::collections::HashSet<&'static str>> {
    let raw = resolve_profile_skips(profile)?;
    let invalid: Vec<&str> = raw
        .iter()
        .copied()
        .filter(|name| gate_internal_name(name).is_none())
        .collect();
    if !invalid.is_empty() {
        let mut sorted = invalid;
        sorted.sort_unstable();
        bail!(
            "profile '{}' references unknown check name(s): {}",
            profile.unwrap_or("<none>"),
            sorted.join(", ")
        );
    }
    Ok(raw.iter().filter_map(|n| gate_internal_name(n)).collect())
}

/// Canonical check names included in the AI guardrail profile, expressed
/// as gate-runner internal names. Acts as an allow-list when
/// `--profile ai` is selected so the runner never executes a check
/// outside the curated set, even if the project's `.anvilrc` would
/// otherwise enable it.
fn ai_guardrail_only_set() -> Result<std::collections::HashSet<&'static str>> {
    let names: std::collections::HashSet<&str> =
        ai_guardrail_profile_checks().iter().copied().collect();
    normalize_gate_check_set(&names)
}

/// Read the project's `checks` filter, preferring MLP-011's multi-format
/// `.anvil.<ext>` (yaml/yml/json/toml) discovery and falling back to the
/// legacy `.anvilrc` for projects that have not migrated yet.
///
/// Returns `Ok(None)` when no config file is found, no `checks` field is
/// present, or the list is empty. Parsing or shape errors are surfaced so
/// gate can fail clearly instead of silently acting on a malformed filter.
///
/// `pub(crate)` so the planless `anvil check` dispatcher in `commands/check.rs`
/// can share the same discovery + parsing path as gate (issue #1797).
pub(crate) fn read_anvilrc_checks(
    workspace_root: &Path,
) -> Result<Option<std::collections::HashSet<String>>> {
    // MLP2-040 — prefer `.anvil.<ext>` via MLP-011's `discover` precedence
    // (yaml → yml → json → toml). When discover finds nothing, we fall
    // back to the legacy `.anvilrc` reader below.
    if let Some(discovered) = anvil_config::discover(workspace_root, ".anvil")
        .with_context(|| format!("scanning {} for .anvil.<ext>", workspace_root.display()))?
    {
        let value = anvil_config::parse_file(&discovered.path)
            .with_context(|| format!("failed to parse {}", discovered.path.display()))?;
        return finalise_checks_from_value(&value);
    }

    // Legacy `.anvilrc` fallback. Format detection mirrors the pre-MLP2-040
    // behaviour: try JSON, TOML, then YAML in order. The first parser that
    // produces an object wins. This path is the deprecation tail; new
    // projects land via `.anvil.<ext>` instead.
    let path = workspace_root.join(".anvilrc");
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(anyhow::anyhow!("failed to read {}: {err}", path.display())),
    };

    let value = parse_anvilrc_contents(&contents, &path)?;
    finalise_checks_from_value(&value)
}

fn finalise_checks_from_value(
    value: &serde_json::Value,
) -> Result<Option<std::collections::HashSet<String>>> {
    let view = crate::config_view::GateConfigView::from_value(value)
        .map_err(|e| anyhow::anyhow!("invalid config: {e}"))?;
    if view.checks.is_empty() {
        return Ok(None);
    }
    let canonical: std::collections::HashSet<String> = view
        .checks
        .into_iter()
        .map(|name| canonical_check_name(&name).unwrap_or(&name).to_string())
        .collect();
    Ok(Some(canonical))
}

fn parse_anvilrc_contents(contents: &str, path: &Path) -> Result<serde_json::Value> {
    for format in [
        anvil_config::ConfigFormat::Json,
        anvil_config::ConfigFormat::Toml,
        anvil_config::ConfigFormat::Yaml,
    ] {
        if let Ok(value) = anvil_config::parse_str(contents, format, path)
            && value.is_object()
        {
            return Ok(value);
        }
    }
    Err(anyhow::anyhow!(
        "failed to parse {} as JSON, YAML, or TOML",
        path.display()
    ))
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
    /// When true (set by `--profile ai`), missing or invalid config is
    /// treated as a blocking diagnostic rather than a soft warning.
    strict_config: bool,
}

fn run_checks(args: &GateArgs) -> Result<Vec<CheckResult>> {
    let root = crate::util::workspace_root()?;

    // Profile skip lists are canonicalised through `gate_internal_name`
    // so entries declared as canonical names (`secret-detection`) and
    // internal names (`secret`) both resolve consistently against
    // `GATE_INTERNAL_CHECKS`. AIGUARD-003 lifted the literal-skip-list
    // constraint that PR #1097 deferred.
    let profile_skip_set = resolve_profile_skip_set(args.profile.as_deref())?;

    let skip_names: std::collections::HashSet<&str> = args
        .skip_checks
        .as_deref()
        .map(|s| s.split(',').map(str::trim).collect())
        .unwrap_or_default();
    let mut skip_set = normalize_gate_check_set(&skip_names)?;
    skip_set.extend(profile_skip_set.iter().copied());

    let only_names: Option<std::collections::HashSet<&str>> = args
        .only_checks
        .as_deref()
        .map(|s| s.split(',').map(str::trim).collect());
    let mut only_set = only_names
        .as_ref()
        .map(normalize_gate_check_set)
        .transpose()?;

    // `--profile ai` selects from `AI_GUARDRAIL_CHECKS` as an allow-list
    // (intersected with any explicit `--only-checks` if both were
    // supplied) so the curated rule set is the floor, not just an
    // inverse skip list. Mirrors the path `--only-checks` already takes.
    if args.profile.as_deref() == Some(AiGuardrailProfile::NAME) {
        let ai_only = ai_guardrail_only_set()?;
        only_set = Some(match only_set.take() {
            Some(existing) => existing.intersection(&ai_only).copied().collect(),
            None => ai_only,
        });
    }

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

    let strict_config = args.profile.as_deref() == Some(AiGuardrailProfile::NAME)
        && AiGuardrailProfile::DEFAULT.strict_config;

    let ctx = GateContext {
        workspace_root: root,
        profile: args.profile.clone(),
        plan_files,
        plan_path,
        walked_files,
        strict_config,
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

/// AI guardrail return-value envelope (`anvil.gate-result.v1`).
///
/// Wraps a list of canonical [`Diagnostic`] payloads — the inner shape
/// pinned by AIGUARD-002 / `anvil.diagnostic.v1` — with a summary and
/// exit code so external AI consumers can branch without re-deriving
/// counts from `diagnostics[]`. Per the diagnostic-envelope spec at
/// `plans/specs/2026-04-26-diagnostic-envelope-coordination.md`.
///
/// **v1 extension policy:** the schema string stays `v1` for
/// backwards-compatible additive fields (fields skipped from output
/// when at their default value). CIB-011 added `summary.config_gaps`
/// under this policy — existing strict consumers on a
/// fully-configured repo see no shape change, and consumers using
/// `#[serde(deny_unknown_fields)]` who hit a partial-config repo
/// will need to update to v1.1+ semantics. Breaking changes (renames,
/// removals, type changes) require a v2 schema string.
#[derive(Debug, Serialize)]
struct AiGateResultEnvelope {
    schema: &'static str,
    exit_code: u8,
    summary: AiGateResultSummary,
    diagnostics: Vec<Diagnostic>,
    duration_ms: u64,
}

#[derive(Debug, Serialize)]
struct AiGateResultSummary {
    total: usize,
    by_severity: AiGateBySeverity,
    by_category: std::collections::BTreeMap<String, usize>,
    overall_passed: bool,
    score: f64,
    /// CIB-011 / #1803 — number of checks the gate could not run
    /// because their project config is missing under strict mode
    /// (e.g. no `.anvil/architecture.yaml`). These do not count
    /// toward `total` (they are not failures), but consumers may
    /// want to surface them to the user as "configure these next".
    /// Skipped from JSON output when zero so the schema stays
    /// additive — existing v1 consumers reading the envelope on a
    /// fully-configured repo see no shape change.
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    config_gaps: usize,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero_usize(n: &usize) -> bool {
    *n == 0
}

#[derive(Debug, Serialize)]
struct AiGateBySeverity {
    error: usize,
    warning: usize,
    info: usize,
}

fn build_ai_gate_result_envelope(result: &GateResult) -> AiGateResultEnvelope {
    // CIB-011 / #1803 — diagnostics are real failures only. Config-gap
    // checks stay passed=true so they are filtered out here as well as
    // by the `!c.passed` clause; surfacing them via `summary.config_gaps`
    // lets consumers count them without re-deriving from `result.checks`.
    let diagnostics: Vec<Diagnostic> = result
        .checks
        .iter()
        .filter(|c| !c.passed && !c.requires_config)
        .map(check_result_to_diagnostic)
        .collect();

    let config_gaps = result.checks.iter().filter(|c| c.requires_config).count();

    let mut by_category: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut error_count: usize = 0;
    for diag in &diagnostics {
        let cat_key = serde_json::to_value(diag.category)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "other".to_string());
        *by_category.entry(cat_key).or_insert(0) += 1;
        if matches!(diag.severity, Severity::Error) {
            error_count += 1;
        }
    }

    AiGateResultEnvelope {
        schema: "anvil.gate-result.v1",
        exit_code: if result.overall { 0 } else { 2 },
        summary: AiGateResultSummary {
            total: diagnostics.len(),
            by_severity: AiGateBySeverity {
                error: error_count,
                warning: 0,
                info: 0,
            },
            by_category,
            overall_passed: result.overall,
            score: result.score,
            config_gaps,
        },
        diagnostics,
        duration_ms: result.duration_ms,
    }
}

/// Map a failed [`CheckResult`] to a canonical `anvil.diagnostic.v1`
/// payload for the AI guardrail envelope. The outer envelope sets
/// `mode = "gate"` and the file anchor is the workspace root when no
/// per-finding location is available — diagnostics that need
/// finer-grained location data are emitted by the underlying check
/// itself in future work.
fn check_result_to_diagnostic(check: &CheckResult) -> Diagnostic {
    let category = check_name_to_category(&check.name);
    let rule_id = format!("gate-{}", check.name);
    let id = format!("diag_gate_{}", check.name);

    Diagnostic::new(
        id,
        Severity::Error,
        check.message.lines().next().unwrap_or("").to_string(),
        Location {
            file: "<workspace>".to_string(),
            line: None,
            column: None,
            end_line: None,
            end_column: None,
        },
        category,
        DiagnosticSource {
            rule_id,
            source_module: format!("anvil-cli::gate::{}", check.name),
        },
        Mode::known(KnownMode::Gate),
    )
    .with_remediation_hint(check.message.clone())
}

fn check_name_to_category(name: &str) -> Category {
    match name {
        "secret-detection" => Category::Secret,
        "antipattern-scan" => Category::Antipattern,
        "import-boundaries" => Category::Boundary,
        "architecture" => Category::Architecture,
        "policy" => Category::Policy,
        "command-safety" => Category::CommandSafety,
        _ => Category::Other,
    }
}

/// Build a SARIF document from gate results (SARIFOUT-005).
///
/// Gate findings are per-check aggregates, not per-location warnings, so each
/// emitted `result` is repo-level (no `locations[]`). Failed checks map to
/// `error`-level results; config-gap checks (`requires_config`) map to
/// `note`-level results so they surface without inflating the failure set;
/// passed checks are not findings and are omitted. `ruleId` is the check name.
/// SARIF emission does not affect the gate exit code.
fn build_gate_sarif(result: &GateResult) -> crate::output::sarif::SarifLog {
    use crate::output::sarif;

    let mut rules: BTreeMap<String, sarif::ReportingDescriptor> = BTreeMap::new();
    let mut results = Vec::new();
    for check in &result.checks {
        // A passing, fully-configured check is not a finding.
        if check.passed && !check.requires_config {
            continue;
        }
        let level = if check.requires_config {
            sarif::Level::Note
        } else {
            sarif::Level::Error
        };
        let message = if check.message.is_empty() {
            if check.requires_config {
                format!("{} requires configuration to run", check.name)
            } else {
                format!("{} did not pass", check.name)
            }
        } else {
            check.message.clone()
        };
        rules
            .entry(check.name.clone())
            .or_insert_with(|| sarif::ReportingDescriptor::new(check.name.clone()));
        results.push(
            sarif::SarifResult::new(check.name.clone(), level, message.clone()).fingerprint(
                "anvilFingerprint/v1",
                sarif::stable_fingerprint(&check.name, "", None, &message),
            ),
        );
    }
    sarif::SarifLog::new(sarif::Run::new(rules.into_values().collect(), results))
}

/// Resolve the gate command's output mode.
///
/// An explicit, non-`auto` `--format` wins outright (including over the
/// AI-guardrail JSON default). `--format auto` and an absent `--format` are
/// equivalent "use the defaults" requests: with the AI guardrail profile they
/// keep the JSON default (unless `--no-tui`), otherwise the legacy `--json` /
/// `--no-tui` / TTY resolver applies.
fn resolve_gate_output_mode(
    format: Option<crate::output::Format>,
    profile_is_ai: bool,
    ai_json_default: bool,
    global: &GlobalArgs,
    is_tty: bool,
) -> crate::output::OutputMode {
    use crate::output::{Format, OutputMode};
    match format {
        Some(f) if f != Format::Auto => {
            OutputMode::resolve_format(Some(f), global.json, global.no_tui, is_tty)
        }
        _ if profile_is_ai && ai_json_default && !global.no_tui => OutputMode::Json,
        _ => OutputMode::resolve(global.json, global.no_tui, is_tty),
    }
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

    // The AI guardrail profile pins JSON output by default so AI
    // consumers reading the gate result get the documented schema
    // without a flag. Callers can still opt out with `--no-tui` (which
    // resolves to plain text) when they pass `--profile ai`.
    let mode = resolve_gate_output_mode(
        args.format,
        args.profile.as_deref() == Some(AiGuardrailProfile::NAME),
        AiGuardrailProfile::DEFAULT.json_output_default,
        global,
        std::io::stdout().is_terminal(),
    );

    let start = std::time::Instant::now();
    let checks = run_checks(args)?;

    // CIB-011 / #1803 — score and overall computed against available
    // checks only; config-gap checks (set under strict mode when a
    // required project config is missing) surface as info, not FAIL,
    // and do not bring the score down.
    let aggregate = aggregate_gate_outcome(&checks);
    let passed_count = aggregate.passed_count;
    let total = aggregate.available_total;
    let overall = aggregate.overall;
    let score = aggregate.score;

    let elapsed = start.elapsed().as_millis();
    let notifications = notifications_for_gate_result(&checks, overall);
    let result = GateResult {
        overall,
        score,
        checks,
        notifications,
        duration_ms: u64::try_from(elapsed).unwrap_or(u64::MAX),
    };

    // Persist the run for the `gate-summary` dashboard (#2242). Best-effort:
    // never affects the gate's exit code.
    persist_gate_snapshot(&result, &aggregate);

    match mode {
        OutputMode::Json => {
            if args.profile.as_deref() == Some(AiGuardrailProfile::NAME) {
                let envelope = build_ai_gate_result_envelope(&result);
                crate::output::json::print(&envelope)?;
            } else {
                crate::output::json::print(&result)?;
            }
        }
        OutputMode::Sarif => crate::output::json::print(&build_gate_sarif(&result))?,
        OutputMode::Plain | OutputMode::Tui => {
            // TUI surface for gate is not yet implemented; fall back to plain.
            use crate::output::plain;

            plain::header("Gate Results");
            plain::section("Checks");
            for check in &result.checks {
                if check.requires_config {
                    // CIB-011 — render config-gaps as INFO with their
                    // full message (which carries the `next:` hint).
                    plain::info(&format!("{:<20} CONFIG NEEDED", check.name));
                } else if check.passed {
                    plain::success(&format!("{:<20} PASS", check.name));
                } else {
                    plain::error(&format!("{:<20} FAIL", check.name));
                }
                let show_message = global.verbose || !check.passed || check.requires_config;
                if !check.message.is_empty() && show_message {
                    for line in check.message.lines() {
                        plain::dim(&format!("  {line}"));
                    }
                }
            }
            plain::blank();
            if overall {
                if aggregate.config_gaps > 0 {
                    plain::success(&format!(
                        "All available gates passed! ({passed_count}/{total} available, {} config-needed, score: {:.0}%)",
                        aggregate.config_gaps, result.score,
                    ));
                } else {
                    plain::success(&format!(
                        "All quality gates passed! (score: {:.0}%)",
                        result.score,
                    ));
                }
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

    // ── AI guardrail profile (AIGUARD-001) ────────────────────────────

    #[test]
    fn ai_guardrail_profile_is_registered() {
        // The "ai" profile must be discoverable via --list-profiles so users
        // (and AI tools reading the help output) can find it.
        let names: Vec<&str> = PROFILES.iter().map(|(n, _, _)| *n).collect();
        assert!(
            names.contains(&AiGuardrailProfile::NAME),
            "ai profile missing from PROFILES table: {names:?}"
        );
    }

    #[test]
    fn ai_guardrail_profile_bundles_expected_rule_set() {
        // The profile must declare the structural-governance rule families
        // documented in plans/modules/ai-guardrail-profile.aps.md.
        let checks = ai_guardrail_profile_checks();
        assert!(checks.contains(&"secret-detection"));
        assert!(checks.contains(&"import-boundaries"));
        assert!(checks.contains(&"antipattern-scan"));
        assert!(checks.contains(&"policy"));
        assert!(checks.contains(&"command-safety"));
    }

    #[test]
    fn ai_guardrail_profile_excludes_toolchain_checks() {
        // Lint/test/coverage/dependency are project-toolchain concerns
        // outside the AI guardrail's structural focus and would push the
        // profile past its <5s budget. Guard against accidental inclusion.
        let checks = ai_guardrail_profile_checks();
        for excluded in ["lint", "test", "coverage", "dependency"] {
            assert!(
                !checks.contains(&excluded),
                "ai profile must not include {excluded}: got {checks:?}"
            );
        }
    }

    #[test]
    fn ai_guardrail_profile_defaults_are_strict() {
        // Per AIGUARD acceptance criteria: missing/invalid config blocks,
        // and JSON output is the documented default for AI consumers.
        let profile = AiGuardrailProfile::DEFAULT;
        assert!(profile.strict_config);
        assert!(profile.json_output_default);
    }

    #[test]
    fn gate_output_mode_honours_ai_default_and_explicit_format() {
        use crate::output::{Format, OutputMode};
        let global = GlobalArgs::default();

        // `--format auto` and an absent `--format` both keep the AI-profile
        // JSON default — auto must NOT be treated as an explicit override.
        assert_eq!(
            resolve_gate_output_mode(Some(Format::Auto), true, true, &global, true),
            OutputMode::Json,
        );
        assert_eq!(
            resolve_gate_output_mode(None, true, true, &global, true),
            OutputMode::Json,
        );

        // An explicit, non-auto `--format` overrides the AI JSON default.
        assert_eq!(
            resolve_gate_output_mode(Some(Format::Plain), true, true, &global, true),
            OutputMode::Plain,
        );
        assert_eq!(
            resolve_gate_output_mode(Some(Format::Sarif), true, true, &global, true),
            OutputMode::Sarif,
        );

        // `--no-tui` opts out of the AI JSON default to plain text.
        let no_tui = GlobalArgs {
            no_tui: true,
            ..GlobalArgs::default()
        };
        assert_eq!(
            resolve_gate_output_mode(None, true, true, &no_tui, true),
            OutputMode::Plain,
        );

        // Without the AI profile, auto/absent falls through to the legacy
        // resolver (TTY → Tui).
        assert_eq!(
            resolve_gate_output_mode(None, false, true, &global, true),
            OutputMode::Tui,
        );
    }

    #[test]
    fn ai_guardrail_profile_skips_match_inverse_of_rule_set() {
        // The PROFILES skip list and AI_GUARDRAIL_CHECKS allow list must
        // stay in sync — every gate-supported check is either in the
        // profile's rule set or in its skip list (modulo command-safety,
        // which is wired in by AIGUARD-003).
        let skips = resolve_profile_skips(Some(AiGuardrailProfile::NAME)).unwrap();
        assert!(skips.contains("lint"));
        assert!(skips.contains("test"));
        assert!(skips.contains("coverage"));
        assert!(skips.contains("dependency"));

        // Checks that should run under the profile must NOT appear in skips.
        assert!(!skips.contains("antipattern-scan"));
        assert!(!skips.contains("secret"));
        assert!(!skips.contains("architecture"));
        assert!(!skips.contains("policy"));
    }

    #[test]
    fn ai_guardrail_profile_check_names_are_canonical() {
        // The profile exposes canonical names so it composes with the
        // public check catalog and the `--profile ai` flag wired in
        // AIGUARD-003. Each entry must round-trip through the catalog
        // — command-safety was registered in AIGUARD-003 so the earlier
        // skip is no longer required.
        for name in ai_guardrail_profile_checks() {
            assert!(
                canonical_check_name(name).is_some(),
                "ai profile references unknown check: {name}"
            );
        }
    }

    #[test]
    fn ai_guardrail_only_set_resolves_to_internal_names() {
        // Allow-list path used by `run_checks` when --profile ai is
        // selected — every entry must resolve to a gate-runner internal
        // name, otherwise the dispatcher loop in `run_checks` silently
        // drops it.
        let resolved = ai_guardrail_only_set().expect("should resolve");
        for internal in &resolved {
            assert!(GATE_INTERNAL_CHECKS.contains(internal));
        }
        // And every canonical entry in the rule set has a corresponding
        // internal name in the resolved set.
        assert_eq!(resolved.len(), ai_guardrail_profile_checks().len());
    }

    #[test]
    fn resolve_profile_skip_set_canonicalises_through_internal_names() {
        // The dev profile's skip list (`coverage`, `dependency`) must
        // resolve through the catalog so it stays in lock-step with
        // user-supplied --skip-checks vocabulary.
        let skips = resolve_profile_skip_set(Some("dev")).unwrap();
        assert!(skips.contains("coverage"));
        assert!(skips.contains("dependency"));
        // Round-trip: every entry must be a real gate-internal name.
        for entry in &skips {
            assert!(GATE_INTERNAL_CHECKS.contains(entry));
        }
    }

    #[test]
    fn check_result_to_diagnostic_emits_canonical_envelope_fields() {
        let check = CheckResult {
            name: "secret-detection".to_string(),
            passed: false,
            score: 0.0,
            message: "Potential secret on src/leak.ts:12".to_string(),
            requires_config: false,
        };
        let diag = check_result_to_diagnostic(&check);
        assert_eq!(diag.schema_version, "anvil.diagnostic.v1");
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.category, Category::Secret);
        assert_eq!(diag.mode, Mode::known(KnownMode::Gate));
        assert!(diag.source.rule_id.contains("secret-detection"));
    }

    #[test]
    fn build_ai_gate_result_envelope_pins_schema_and_summary() {
        let checks = vec![
            CheckResult {
                name: "secret-detection".to_string(),
                passed: false,
                score: 0.0,
                message: "leak".to_string(),
                requires_config: false,
            },
            CheckResult {
                name: "policy".to_string(),
                passed: true,
                score: 100.0,
                message: "ok".to_string(),
                requires_config: false,
            },
        ];
        let notifications = notifications_for_gate_result(&checks, false);
        let result = GateResult {
            overall: false,
            score: 50.0,
            checks,
            notifications,
            duration_ms: 17,
        };
        let envelope = build_ai_gate_result_envelope(&result);
        let value = serde_json::to_value(&envelope).unwrap();
        assert_eq!(value["schema"], "anvil.gate-result.v1");
        assert_eq!(value["exit_code"], 2);
        assert_eq!(value["summary"]["total"], 1);
        assert_eq!(value["summary"]["overall_passed"], false);
        assert_eq!(value["diagnostics"][0]["mode"], "gate");
        assert_eq!(value["diagnostics"][0]["category"], "secret");
    }

    #[test]
    fn gate_snapshot_maps_status_rows_and_warnings() {
        let checks = vec![
            CheckResult {
                name: "lint".into(),
                passed: true,
                score: 100.0,
                message: "clean".into(),
                requires_config: false,
            },
            CheckResult {
                name: "secret".into(),
                passed: false,
                score: 0.0,
                message: "leak found".into(),
                requires_config: false,
            },
            CheckResult {
                name: "architecture".into(),
                passed: false,
                score: 0.0,
                message: "no config".into(),
                requires_config: true,
            },
        ];
        let aggregate = aggregate_gate_outcome(&checks);
        let result = GateResult {
            overall: aggregate.overall,
            score: aggregate.score,
            notifications: vec![],
            duration_ms: 4200,
            checks,
        };
        let snap = GateSnapshot::from_result(&result, &aggregate);

        // One available check (secret) failed -> fail; the config-gap is excluded.
        assert_eq!(snap.status, "fail");
        assert!(
            snap.status_label.starts_with("FAILED"),
            "{}",
            snap.status_label
        );
        assert_eq!(snap.checks_run, "2", "config-gap excluded from checks run");
        assert_eq!(snap.duration_seconds, "4.2", "4200ms -> 4.2s");

        assert_eq!(snap.check_rows.len(), 3);
        assert_eq!(snap.check_rows[0], ["lint", "passed", "100", "clean"]);
        assert_eq!(snap.check_rows[1][1], "failed");
        assert_eq!(snap.check_rows[2][1], "config");

        // Warnings: secret (error) + architecture (warn); passing lint excluded.
        assert_eq!(snap.warnings, "2");
        assert_eq!(snap.warning_list.len(), 2);
        assert_eq!(snap.warning_list[0].severity, "error");
        assert!(snap.warning_list[0].message.contains("secret: leak found"));
        assert_eq!(snap.warning_list[1].severity, "warn");

        // The persisted JSON uses the camelCase keys the dashboard `$data` paths
        // bind to.
        let v = serde_json::to_value(&snap).unwrap();
        for key in [
            "status",
            "statusLabel",
            "checksRun",
            "warnings",
            "durationSeconds",
            "checkRows",
            "warningList",
        ] {
            assert!(v.get(key).is_some(), "missing key {key}");
        }
        assert_eq!(v["checkRows"][1][1], "failed");
    }

    #[test]
    fn gate_snapshot_status_is_warn_when_passing_with_config_gaps() {
        let checks = vec![
            CheckResult {
                name: "lint".into(),
                passed: true,
                score: 100.0,
                message: "ok".into(),
                requires_config: false,
            },
            CheckResult {
                name: "architecture".into(),
                passed: false,
                score: 0.0,
                message: "no config".into(),
                requires_config: true,
            },
        ];
        let aggregate = aggregate_gate_outcome(&checks);
        let result = GateResult {
            overall: aggregate.overall,
            score: aggregate.score,
            notifications: vec![],
            duration_ms: 500,
            checks,
        };
        assert!(result.overall, "no available check failed -> overall pass");
        let snap = GateSnapshot::from_result(&result, &aggregate);
        assert_eq!(
            snap.status, "warn",
            "passing-with-config-gaps is warn, not pass"
        );
        assert!(snap.status_label.starts_with("PASSED"));
        assert_eq!(
            snap.duration_seconds, "0.5",
            "sub-second run shows tenths, not 0"
        );
    }

    #[test]
    fn is_skipped_for_missing_config_only_fires_for_config_dependent_checks() {
        assert!(is_skipped_for_missing_config(
            "architecture",
            "No architecture config found. Skipping."
        ));
        assert!(is_skipped_for_missing_config(
            "policy",
            "No policy bundle found. Skipping."
        ));
        assert!(is_skipped_for_missing_config(
            "command-safety",
            "Command-safety check disabled. Skipping."
        ));
        // Command-safety with no plan supplied — also a project-config gap.
        assert!(is_skipped_for_missing_config(
            "command-safety",
            "No commands to analyse"
        ));
        // OPA-not-installed is a host-tooling gap, not a project-config
        // gap; strict mode must NOT block on it.
        assert!(!is_skipped_for_missing_config(
            "policy",
            "OPA not installed. Skipping policy evaluation."
        ));
        // Secret detection skipping is content-driven, not config-driven —
        // strict_config must not elevate it to a blocking diagnostic.
        assert!(!is_skipped_for_missing_config("secret", "Skipping."));
        assert!(!is_skipped_for_missing_config("architecture", "All good"));
    }

    #[test]
    fn check_name_to_category_covers_every_ai_guardrail_check() {
        // Every check listed in AI_GUARDRAIL_CHECKS must map to a
        // dedicated Category — Other is a routing failure that hides the
        // signal from `summary.by_category` in the AI envelope.
        for canonical in [
            "secret-detection",
            "antipattern-scan",
            "import-boundaries",
            "architecture",
            "policy",
            "command-safety",
        ] {
            let cat = check_name_to_category(canonical);
            assert!(
                !matches!(cat, Category::Other),
                "{canonical} must map to a non-Other category"
            );
        }
    }

    #[test]
    fn resolve_profile_skip_set_rejects_unknown_check_names() {
        // Mock a profile whose skip list contains a typo. Currently
        // PROFILES are static so we can only assert the present-day
        // contents always resolve. This guard prevents future profile
        // edits from silently failing open.
        for (name, _, skips) in PROFILES {
            let result = resolve_profile_skip_set(Some(*name));
            assert!(
                result.is_ok(),
                "profile '{name}' has unresolvable skip entries: {skips:?}"
            );
        }
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

    // ── LANGTS-006 / #1801: dynamic-execution rules ──────────────────

    #[test]
    fn antipattern_check_detects_dynamic_eval() {
        // AP-008 — eval(<identifier>) must fire under default profile.
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("smelly.ts");
        std::fs::write(
            &file,
            "export function unsafe(input: any): unknown {\n    return eval(input);\n}\n",
        )
        .unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
        );

        assert!(!result.passed, "AP-008 must trip on eval(<identifier>)");
        assert!(
            result.message.contains("AP-008"),
            "expected AP-008 in message, got: {}",
            result.message
        );
    }

    #[test]
    fn antipattern_check_detects_new_function() {
        // AP-009 — `new Function(...)` always fires (the dynamic-string
        // ergonomics are never worth the audit cost).
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("smelly.ts");
        std::fs::write(
            &file,
            "export const compiled = new Function('a', 'b', 'return a + b');\n",
        )
        .unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
        );

        assert!(!result.passed, "AP-009 must trip on `new Function(...)`");
        assert!(
            result.message.contains("AP-009"),
            "expected AP-009 in message, got: {}",
            result.message
        );
    }

    #[test]
    fn antipattern_check_detects_template_literal_eval() {
        // Council follow-up: `eval(`${userInput}`)` is the most ergonomic
        // way to build a dynamic-string eval call in modern TS; the
        // regex extends to backtick so the template-literal shape
        // does not slip through.
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("smelly.ts");
        std::fs::write(
            &file,
            "export function unsafe(input: string): unknown {\n    return eval(`run(${input})`);\n}\n",
        )
        .unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
        );

        assert!(
            !result.passed,
            "AP-008 must trip on eval(`...`) template-literal arg; got: {}",
            result.message
        );
        assert!(result.message.contains("AP-008"));
    }

    #[test]
    fn antipattern_check_skips_static_eval() {
        // AP-008 is intentionally narrow: a literal-string `eval("1+1")`
        // is rare but benign, and false positives here would erode
        // trust in the rule. The detector requires an identifier char
        // (A-Za-z_$) immediately after the opening paren — a quote
        // skips the rule.
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("benign.ts");
        std::fs::write(
            &file,
            "export const two = eval(\"1 + 1\");\nexport function evalQueue() {}\n",
        )
        .unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
        );

        assert!(
            result.passed,
            "AP-008 must not fire on static-string eval or on `evalQueue` word boundary; got: {}",
            result.message
        );
        assert!(!result.message.contains("AP-008"));
    }

    // ── LANGTS-004 / TS-G5: Zod-creep rules ──────────────────────────

    #[test]
    fn antipattern_check_detects_zod_creep() {
        // AP-015 — the on-by-default Zod escape hatches (`z.any()` and a
        // Zod `.passthrough()`) must trip the type-system-evasion gate the
        // same way `: any` (AP-003) does. Each fixture runs on its own so a
        // regex that only caught one alternative would fail here.
        for snippet in [
            "export const S = z.any();\n",
            "export const S = z.object({ id: z.string() }).passthrough();\n",
        ] {
            let tmp = tempfile::TempDir::new().unwrap();
            std::fs::write(tmp.path().join("schema.ts"), snippet).unwrap();

            let result = run_check_antipattern(
                "antipattern-scan",
                tmp.path(),
                &std::collections::HashSet::new(),
            );

            assert!(
                !result.passed,
                "AP-015 must trip on Zod escape hatch `{snippet}`"
            );
            assert!(
                result.message.contains("AP-015"),
                "expected AP-015 for `{snippet}`, got: {}",
                result.message
            );
        }
    }

    #[test]
    fn antipattern_check_skips_zod_unknown_by_default() {
        // `z.unknown()` is the opt-in rule AP-016 (idiomatic as a typed-record
        // leaf), so the default gate must stay quiet on it.
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("schema.ts"),
            "export const Meta = z.record(z.string(), z.unknown());\n",
        )
        .unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
        );

        assert!(
            result.passed,
            "z.unknown() (AP-016 opt-in) must not trip the default gate; got: {}",
            result.message
        );
        assert!(!result.message.contains("AP-016"));
    }

    #[test]
    fn antipattern_check_skips_typed_zod_schema() {
        // A fully-typed Zod schema must NOT fire AP-015 — the rule targets
        // the escape hatches, not Zod itself (Zod is the recommended fix
        // for `: any`, per the type-system-evasion family doc).
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("schema.ts"),
            "export const User = z.object({ id: z.string(), age: z.number() });\n",
        )
        .unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
        );

        assert!(
            result.passed,
            "typed Zod schema must not trip the gate; got: {}",
            result.message
        );
        assert!(!result.message.contains("AP-015"));
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
    fn normalize_gate_check_set_accepts_stable_ids_and_aliases() {
        let names: std::collections::HashSet<&str> =
            ["ANV-CORE-001", "architecture"].into_iter().collect();

        let normalised = normalize_gate_check_set(&names).unwrap();

        assert!(normalised.contains("secret"));
        assert!(normalised.contains("architecture"));
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
            requires_config: false,
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

    // ── CIB-011 / #1803: strict-config produces config-gap, not FAIL ──

    fn strict_ai_ctx(root: &Path) -> GateContext {
        GateContext {
            workspace_root: root.to_path_buf(),
            profile: Some(AiGuardrailProfile::NAME.to_string()),
            plan_files: std::collections::HashSet::new(),
            plan_path: None,
            walked_files: Vec::new(),
            strict_config: true,
        }
    }

    #[test]
    fn strict_config_missing_arch_becomes_config_gap_not_fail() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = strict_ai_ctx(dir.path());
        let result = run_single_check("architecture", &ctx);
        assert!(
            result.passed,
            "missing-config must NOT flip to fail under strict mode; got message: {}",
            result.message
        );
        assert!(
            result.requires_config,
            "missing-config must set requires_config=true; got: passed={}, message={}",
            result.passed, result.message
        );
        assert!(
            result.message.contains("next:"),
            "config-gap message must carry an actionable `next:` hint; got: {}",
            result.message
        );
        assert!(
            !result.message.starts_with("Strict mode"),
            "pre-CIB-011 FAIL prefix must be removed; got: {}",
            result.message
        );
    }

    #[test]
    fn strict_config_missing_policy_becomes_config_gap_not_fail() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = strict_ai_ctx(dir.path());
        let result = run_single_check("policy", &ctx);
        assert!(
            result.passed,
            "missing policy bundle must not fail under strict mode"
        );
        assert!(result.requires_config);
        assert!(result.message.contains("next:"));
        assert!(result.message.contains(".anvil/policies"));
    }

    #[test]
    fn strict_config_missing_command_safety_becomes_config_gap_not_fail() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = strict_ai_ctx(dir.path());
        let result = run_single_check("command-safety", &ctx);
        assert!(
            result.passed,
            "no plan supplied must not fail under strict mode"
        );
        assert!(result.requires_config);
        assert!(result.message.contains("next:"));
        assert!(result.message.contains("--plan"));
    }

    #[test]
    fn non_strict_skip_does_not_set_requires_config() {
        // Outside strict mode, the same architecture-missing scenario
        // returns a soft skip with no config-gap marker. Regression
        // pin against accidentally treating every soft skip as a
        // config-gap.
        let dir = tempfile::tempdir().unwrap();
        let ctx = GateContext {
            workspace_root: dir.path().to_path_buf(),
            profile: None,
            plan_files: std::collections::HashSet::new(),
            plan_path: None,
            walked_files: Vec::new(),
            strict_config: false,
        };
        let result = run_single_check("architecture", &ctx);
        assert!(result.passed);
        assert!(
            !result.requires_config,
            "non-strict skip must not mark config-gap"
        );
        assert!(
            !result.message.contains("next:"),
            "non-strict path stays clean"
        );
    }

    #[test]
    fn aggregate_excludes_config_gap_from_score_denominator() {
        let checks = vec![
            CheckResult {
                name: "antipattern-scan".into(),
                passed: true,
                score: 100.0,
                message: "clean".into(),
                requires_config: false,
            },
            CheckResult {
                name: "secret-detection".into(),
                passed: false,
                score: 0.0,
                message: "secret found".into(),
                requires_config: false,
            },
            CheckResult {
                name: "import-boundaries".into(),
                passed: true,
                score: 100.0,
                message: "...Skipping. next: ...".into(),
                requires_config: true,
            },
            CheckResult {
                name: "policy".into(),
                passed: true,
                score: 100.0,
                message: "...Skipping. next: ...".into(),
                requires_config: true,
            },
            CheckResult {
                name: "command-safety".into(),
                passed: true,
                score: 100.0,
                message: "...No commands... next: ...".into(),
                requires_config: true,
            },
        ];
        let agg = aggregate_gate_outcome(&checks);
        assert_eq!(agg.available_total, 2, "3 config-gap checks excluded");
        assert_eq!(agg.passed_count, 1);
        assert_eq!(agg.config_gaps, 3);
        assert!((agg.score - 50.0).abs() < f64::EPSILON, "1/2 = 50%");
        assert!(!agg.overall);
    }

    #[test]
    fn aggregate_overall_true_when_all_available_pass() {
        let checks = vec![
            CheckResult {
                name: "antipattern-scan".into(),
                passed: true,
                score: 100.0,
                message: "clean".into(),
                requires_config: false,
            },
            CheckResult {
                name: "secret-detection".into(),
                passed: true,
                score: 100.0,
                message: "clean".into(),
                requires_config: false,
            },
            CheckResult {
                name: "import-boundaries".into(),
                passed: true,
                score: 100.0,
                message: "...next: ...".into(),
                requires_config: true,
            },
        ];
        let agg = aggregate_gate_outcome(&checks);
        assert!(agg.overall);
        assert!((agg.score - 100.0).abs() < f64::EPSILON);
        assert_eq!(agg.available_total, 2);
        assert_eq!(agg.config_gaps, 1);
    }

    #[test]
    fn aggregate_score_100_when_only_config_gaps() {
        // Edge case: every check is a config-gap. Nothing actually ran,
        // so nothing failed; the gate is vacuously green and the
        // render layer surfaces the config gaps so the user is not
        // misled into thinking "100%" means "fully covered".
        let checks = vec![CheckResult {
            name: "import-boundaries".into(),
            passed: true,
            score: 100.0,
            message: "...next: ...".into(),
            requires_config: true,
        }];
        let agg = aggregate_gate_outcome(&checks);
        assert!(agg.overall);
        assert!((agg.score - 100.0).abs() < f64::EPSILON);
        assert_eq!(agg.available_total, 0);
        assert_eq!(agg.config_gaps, 1);
    }

    #[test]
    fn ai_envelope_excludes_config_gaps_from_diagnostics_and_counts_them() {
        let checks = vec![
            CheckResult {
                name: "secret-detection".into(),
                passed: false,
                score: 0.0,
                message: "secret found".into(),
                requires_config: false,
            },
            CheckResult {
                name: "import-boundaries".into(),
                passed: true,
                score: 100.0,
                message: "...next: ...".into(),
                requires_config: true,
            },
        ];
        let result = GateResult {
            overall: false,
            score: 0.0,
            checks,
            notifications: vec![],
            duration_ms: 1,
        };
        let envelope = build_ai_gate_result_envelope(&result);
        assert_eq!(
            envelope.summary.total, 1,
            "only the real failure surfaces as a diagnostic"
        );
        assert_eq!(
            envelope.summary.config_gaps, 1,
            "config-gap is counted separately"
        );
        assert_eq!(envelope.diagnostics.len(), 1);
    }

    #[test]
    fn config_gap_next_hint_covers_strict_mode_check_names() {
        // All three checks that the strict-mode flip touches must have
        // a real hint (not the generic fallback).
        let generic = config_gap_next_hint("__no_such_check__");
        for name in ["architecture", "policy", "command-safety"] {
            let hint = config_gap_next_hint(name);
            assert_ne!(hint, generic, "{name} must have a dedicated hint");
            assert!(!hint.is_empty());
        }
    }

    #[test]
    fn config_gap_check_keeps_passed_true_for_fail_fast_continuity() {
        // The fail_fast path in `run_checks` derives `failed =
        // !result.passed` to decide whether to short-circuit. Config-gap
        // checks must keep `passed: true` so they do NOT trip fail_fast
        // (otherwise a config-gap in the first check would silently drop
        // every subsequent real failure). Pin the invariant at its
        // source — run_single_check under strict mode for a missing
        // architecture config.
        let dir = tempfile::tempdir().unwrap();
        let ctx = strict_ai_ctx(dir.path());
        let result = run_single_check("architecture", &ctx);
        assert!(result.requires_config, "test pre-condition");
        assert!(
            result.passed,
            "config-gap must keep passed=true so fail_fast does not trip on it"
        );
        // Confirm the derived `failed` flag matches the expectation.
        let failed = !result.passed;
        assert!(!failed, "config-gap must derive failed=false for fail_fast");
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
            strict_config: false,
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
        assert!(checks.contains("secret-detection"));
        assert!(checks.contains("import-boundaries"));
    }

    #[test]
    fn read_anvilrc_checks_parses_stable_ids() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            r#"{"checks": ["ANV-CORE-001", "ANV-CORE-002"]}"#,
        )
        .unwrap();

        let checks = read_anvilrc_checks(tmp.path()).unwrap().unwrap();

        assert_eq!(checks.len(), 2);
        assert!(checks.contains("secret-detection"));
        assert!(checks.contains("import-boundaries"));
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
        assert!(checks.contains("secret-detection"));
        assert!(checks.contains("import-boundaries"));
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
        assert!(checks.contains("secret-detection"));
        assert!(checks.contains("policy"));
    }

    #[test]
    fn read_anvilrc_checks_errors_on_unparseable_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvilrc"), "checks: [\n").unwrap();
        let err = read_anvilrc_checks(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("failed to parse"));
    }

    // MLP2-040 — `.anvil.<ext>` discovery via MLP-011 takes precedence over
    // the legacy `.anvilrc`. The fallback only triggers when no
    // `.anvil.<ext>` is present.

    #[test]
    fn read_anvilrc_checks_prefers_anvil_yaml_when_present() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "checks: [\"secret-detection\"]\n",
        )
        .unwrap();
        // Legacy `.anvilrc` exists too with a different value to prove
        // precedence — discover should win.
        std::fs::write(
            tmp.path().join(".anvilrc"),
            r#"{"checks":["import-boundaries"]}"#,
        )
        .unwrap();

        let checks = read_anvilrc_checks(tmp.path()).unwrap().unwrap();
        assert!(checks.contains("secret-detection"));
        assert!(!checks.contains("import-boundaries"));
    }

    #[test]
    fn read_anvilrc_checks_reads_anvil_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.json"),
            r#"{"checks":["secret-detection","import-boundaries"]}"#,
        )
        .unwrap();
        let checks = read_anvilrc_checks(tmp.path()).unwrap().unwrap();
        assert_eq!(checks.len(), 2);
        assert!(checks.contains("secret-detection"));
    }

    #[test]
    fn read_anvilrc_checks_reads_anvil_toml() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.toml"),
            "checks = [\"secret-detection\"]\n",
        )
        .unwrap();
        let checks = read_anvilrc_checks(tmp.path()).unwrap().unwrap();
        assert!(checks.contains("secret-detection"));
    }

    #[test]
    fn read_anvilrc_checks_falls_back_to_anvilrc_when_no_anvil_ext() {
        // Sanity guard against accidentally inverting the precedence: when
        // there is no `.anvil.<ext>`, the legacy reader must still pick
        // up `.anvilrc`. This test catches a regression where the
        // fallback was lost entirely.
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            "checks: [\"secret-detection\"]\n",
        )
        .unwrap();
        let checks = read_anvilrc_checks(tmp.path()).unwrap().unwrap();
        assert!(checks.contains("secret-detection"));
    }

    // ── SARIF adapter (SARIFOUT-005) ────────────────────────────────

    fn check_result(name: &str, passed: bool, requires_config: bool, message: &str) -> CheckResult {
        CheckResult {
            name: name.to_string(),
            passed,
            score: if passed { 100.0 } else { 0.0 },
            message: message.to_string(),
            requires_config,
        }
    }

    #[test]
    fn gate_sarif_maps_failed_and_config_gap_checks_only() {
        let result = GateResult {
            overall: false,
            score: 50.0,
            checks: vec![
                check_result("secret-detection", false, false, "2 hardcoded secrets"),
                check_result("policy", false, true, "needs .anvil/policy"),
                check_result("antipattern-scan", true, false, ""),
            ],
            notifications: Vec::new(),
            duration_ms: 1,
        };
        let value = serde_json::to_value(build_gate_sarif(&result)).expect("serialise");

        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../output/sarif-schema-2.1.0.json"))
                .expect("schema json");
        let validator = jsonschema::validator_for(&schema).expect("compile schema");
        let errors: Vec<String> = validator
            .iter_errors(&value)
            .map(|e| format!("{} at {}", e, e.instance_path()))
            .collect();
        assert!(errors.is_empty(), "schema errors:\n{}", errors.join("\n"));

        let results = value["runs"][0]["results"].as_array().expect("results");
        // Passing, fully-configured checks are not findings.
        assert_eq!(results.len(), 2, "failed + config-gap only");

        let failed = results
            .iter()
            .find(|r| r["ruleId"] == "secret-detection")
            .unwrap();
        assert_eq!(failed["level"], "error");
        // Repo-level aggregate: no physical location.
        assert!(failed.get("locations").is_none());

        let config_gap = results.iter().find(|r| r["ruleId"] == "policy").unwrap();
        assert_eq!(
            config_gap["level"], "note",
            "config-gap does not inflate failures"
        );

        assert!(
            !results.iter().any(|r| r["ruleId"] == "antipattern-scan"),
            "passing check omitted"
        );
    }
}
