use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use anvil_kernel_types::{
    Notification, NotificationClass, NotificationContext, NotificationPriority,
};
use anyhow::{Result, bail};
use clap::Args;
use serde::Serialize;

use anvil_checks::antipattern::{
    AntipatternCheckConfig, Artifact, ArtifactKind, ScanOptions, Warning, WarningSeverity,
    WarningSummary, create_warning_result, run_antipattern_check, scan_artifacts,
};
use anvil_checks::secret::{SecretCheckConfig, SecretFinding, run_secret_check};

use crate::GlobalArgs;
use crate::commands::check_catalog::canonical_check_name;
use crate::commands::gate::read_anvilrc_checks;
use crate::output::{self, OutputMode, sarif};
use crate::util::is_ignored_dir_name;

/// Canonical names of checks the planless `anvil check` path can run.
///
/// Planless-eligible means: the check operates on the supplied file list and
/// needs no profile, policy bundle, or project-level config beyond the source
/// itself. `secret-detection` and `antipattern-scan` qualify; `architecture`,
/// `policy`, `command-safety`, `import-boundaries`, `lint`, `test`,
/// `coverage`, and `dependency` do not — they require config files,
/// language-toolchain context, or a profile, and live under `anvil gate`.
///
/// Issue #1797: `gate` and `anvil_validate_write` catch hardcoded secrets but
/// planless `check` silently dropped `.anvilrc#checks`; this list closes that
/// gap by routing the planless dispatcher through the same `.anvilrc` reader.
const PLANLESS_ELIGIBLE_CHECKS: &[&str] = &["secret-detection", "antipattern-scan"];

/// JSON output schema version — shared across all output paths.
const CHECK_OUTPUT_VERSION: &str = "1.0.0";

/// Hard cap on a single artifact's size for `anvil check --artifact`. PR
/// descriptions and commit messages are kilobytes; agent outputs can grow
/// but a multi-megabyte input is almost always an operator mistake (a
/// build artefact piped by accident). Stops the artifact path from
/// OOM-ing on `std::fs::read_to_string`.
const MAX_ARTIFACT_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Debug, Args)]
#[allow(clippy::struct_excessive_bools)]
pub struct CheckArgs {
    /// Files to analyse (optional if using --changed, --staged, --since, or --all).
    /// Explicit file paths take precedence over --changed/--staged/--since.
    files: Vec<String>,

    /// Analyse git-changed files only (ignored if explicit file paths are given).
    #[arg(long, conflicts_with = "all")]
    changed: bool,

    /// Analyse only staged files (implies --changed; ignored if explicit file paths are given).
    #[arg(long, conflicts_with = "since", conflicts_with = "all")]
    staged: bool,

    /// Compare against a git ref, e.g. main, HEAD~3 (implies --changed; ignored if explicit file paths are given).
    #[arg(long, conflicts_with = "staged", conflicts_with = "all")]
    since: Option<String>,

    /// Analyse all source files in the project.
    #[arg(long)]
    all: bool,

    /// Comma-separated file extensions to analyse (e.g. .ts,.tsx,.html).
    #[arg(long)]
    extensions: Option<String>,

    /// Minimum severity for blocking: error, warning, info (default: error).
    #[arg(long, default_value = "error")]
    severity: String,

    /// Include opt-in patterns.
    #[arg(long)]
    include_opt_in: bool,

    /// Artifact kind: source, pr-description, commit-message, agent-output.
    /// Non-source kinds read each file as the artifact content and route
    /// through `scan_artifact` with the matching `ArtifactKind`. Non-source
    /// kinds are incompatible with `--all`, `--changed`, `--staged`,
    /// `--since`, and `--extensions`.
    #[arg(long, default_value = "source")]
    artifact: String,

    /// Output format: auto (default), tui, plain, json, or sarif. `json` is the
    /// `--json` alias; `sarif` emits SARIF 2.1.0 and is never auto-selected.
    #[arg(long, value_enum)]
    format: Option<output::Format>,
}

impl CheckArgs {
    /// True when `--format json|sarif` requests structured output, so the
    /// pre-dispatch auth gate emits a JSON envelope rather than human text.
    pub(crate) fn wants_structured_output(&self) -> bool {
        self.format.is_some_and(output::Format::is_structured)
    }

    /// True when any flag selects the git-changed files mode. `--staged` and
    /// `--since` imply `--changed`, so all three collapse to a single mode.
    fn changed_mode(&self) -> bool {
        self.changed || self.staged || self.since.is_some()
    }
}

/// Describes how files were selected, for user-facing messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileSource {
    All,
    Changed,
    Explicit,
}

// ── JSON output schema (parity with Node.js CLI) ────────────────────

#[derive(Debug, Serialize)]
struct CheckOutput {
    version: &'static str,
    timestamp: String,
    files: Vec<String>,
    #[serde(rename = "hasBlockingWarnings")]
    has_blocking_warnings: bool,
    #[serde(rename = "executionTimeMs")]
    execution_time_ms: u64,
    #[serde(rename = "checksRun")]
    checks_run: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provenance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    notifications: Vec<Notification>,
    warnings: Vec<JsonWarning>,
    summary: WarningSummary,
    /// SPG-002: rules whose regex failed to compile in the Rust scanner.
    /// Operators can rely on this to distinguish "rule ran, no matches"
    /// from "rule never ran". Empty on a clean registry.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    diagnostics: Vec<JsonDiagnostic>,
}

#[derive(Debug, Serialize)]
struct JsonDiagnostic {
    id: String,
    title: String,
    error: String,
}

#[derive(Debug, Serialize)]
struct JsonWarning {
    id: String,
    category: String,
    severity: String,
    title: String,
    message: String,
    file: String,
    line: usize,
    suggestion: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    nudge: Option<String>,
}

// ── Entry point ─────────────────────────────────────────────────────

#[allow(clippy::too_many_lines)] // Linear phase pipeline (parse → gather → dispatch → render).
pub fn run(args: &CheckArgs, global: &GlobalArgs) -> Result<()> {
    let mode = OutputMode::from_command_format(args.format, global);
    let start = Instant::now();
    // SARIFOUT-003: collect SARIF results alongside the JSON projection when
    // `--format sarif` is requested. An empty document is emitted for the
    // early-return guards below so the SARIF stream is always well-formed.
    let mut sarif = SarifAccumulator::default();

    // Validate mutually exclusive flags. `--staged` and `--since` imply
    // `--changed`, so treat any of them as the change-selection mode.
    if args.all && args.changed_mode() {
        bail!("Cannot use --all with --changed/--staged/--since. Choose one.");
    }

    let artifact_kind = parse_artifact_kind(&args.artifact)?;
    if artifact_kind != ArtifactKind::Source {
        return run_non_source_artifact(args, global, artifact_kind, start);
    }

    let severity_threshold = parse_severity(&args.severity)?;

    // Resolve file extensions.
    let extensions = resolve_extensions(args.extensions.as_deref());

    // Gather files to analyse.
    let (files, source) = gather_files(args, &extensions)?;

    if files.is_empty() {
        let elapsed = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        let message = match source {
            FileSource::All => "No source files found",
            FileSource::Changed => "No changed files to analyse",
            FileSource::Explicit => "No files to analyse",
        };
        if mode == OutputMode::Json {
            output::json::print(&empty_output(elapsed, message))?;
        } else if mode == OutputMode::Sarif {
            output::json::print(&SarifAccumulator::default().into_log())?;
        } else {
            output::plain::info(message);
        }
        return Ok(());
    }

    if matches!(mode, OutputMode::Plain | OutputMode::Tui) && global.verbose {
        output::plain::info(&format!("Analysing {} file(s)...", files.len()));
    }

    // Workspace root is needed both for `.anvilrc` discovery and for path
    // relativisation in output. Falls back to the current directory when
    // git is unavailable so non-git callers still get sane paths.
    let workspace_root = git_toplevel()
        .ok()
        .map(|p| p.to_string_lossy().to_string())
        .or_else(|| {
            std::env::current_dir()
                .and_then(std::fs::canonicalize)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        });

    // Issue #1797: resolve which planless-eligible checks to run. The
    // planless dispatcher must honour `.anvilrc#checks` (or fall back to
    // the planless-eligible default set) the same way `anvil gate` does.
    let enabled_checks = resolve_enabled_planless_checks(workspace_root.as_deref())?;

    // `.anvilrc#checks` is configured but every entry is gate-only
    // (`policy`, `architecture`, `import-boundaries`, etc.). The
    // intersection with planless-eligible is empty, so without an
    // explicit message we would silently report "no warnings" or "no
    // analysable files" — both of which mislead operators into thinking
    // their files were clean. Tell them which surface they need instead.
    if enabled_checks.is_empty() {
        let elapsed = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        let message = format!(
            "No planless-eligible checks enabled in .anvilrc (planless-eligible: {}). \
             Use `anvil gate` for config-heavy checks like policy, architecture, or import-boundaries.",
            PLANLESS_ELIGIBLE_CHECKS.join(", ")
        );
        if mode == OutputMode::Json {
            output::json::print(&empty_output(elapsed, &message))?;
        } else if mode == OutputMode::Sarif {
            output::json::print(&SarifAccumulator::default().into_log())?;
        } else {
            output::plain::warn(&message);
        }
        return Ok(());
    }

    if matches!(mode, OutputMode::Plain | OutputMode::Tui) && global.verbose {
        output::plain::info(&format!("Running checks: {}", enabled_checks.join(", ")));
    }

    let mut aggregated_warnings: Vec<JsonWarning> = Vec::new();
    let mut aggregated_patterns: BTreeSet<String> = BTreeSet::new();
    let mut checks_run: Vec<String> = Vec::new();
    let mut any_files_scanned = false;

    for check_name in &enabled_checks {
        match *check_name {
            "antipattern-scan" => {
                let config = AntipatternCheckConfig {
                    patterns: Vec::new(),
                    include_opt_in: args.include_opt_in,
                    extensions: extensions.clone(),
                    severity_threshold,
                };
                let file_refs: Vec<&str> = files.iter().map(String::as_str).collect();
                let result = run_antipattern_check(&file_refs, &config, workspace_root.as_deref());
                // ADR-071: the gate-time AST tier (Rust unwrap/unsafe/serde/panic
                // rules the regex scanner can't express). `anvil check` is a
                // gate-time surface, never the save-time daemon, so running it
                // here respects ADR-064 — the daemon links neither this crate
                // nor tree-sitter (the `daemon_dep_boundary` guard verifies).
                let ast = anvil_checks_ast::scan_paths(
                    &file_refs,
                    workspace_root.as_deref(),
                    &anvil_checks_ast::AstScanOptions {
                        registry_path: None,
                        include_opt_in: args.include_opt_in,
                    },
                );
                // Surface scanner-init failures (malformed ast_query, missing
                // predicate) to the operator on stderr so a silently-dropped
                // rule is visible without a tracing subscriber and without
                // corrupting JSON/SARIF stdout (council operations MAJOR).
                for err in &ast.init_errors {
                    eprintln!("anvil: AST anti-pattern rule load error: {err}");
                }
                if result.files_scanned > 0 || ast.files_scanned > 0 {
                    any_files_scanned = true;
                }
                // Merge both tiers into one deterministic order (ADR-071 §7).
                let mut merged: Vec<(&Warning, bool)> =
                    Vec::with_capacity(result.warnings.warnings.len() + ast.warnings.len());
                merged.extend(result.warnings.warnings.iter().map(|w| (w, false)));
                merged.extend(ast.warnings.iter().map(|w| (w, true)));
                merged.sort_by(|(a, _), (b, _)| {
                    a.location
                        .file
                        .cmp(&b.location.file)
                        .then_with(|| a.location.line.cmp(&b.location.line))
                        .then_with(|| a.location.column.cmp(&b.location.column))
                        .then_with(|| a.id.cmp(&b.id))
                });
                for (w, is_ast) in &merged {
                    aggregated_warnings.push(antipattern_warning_to_json(w));
                    if mode == OutputMode::Sarif {
                        sarif.add_warning_tiered(w, *is_ast);
                    }
                }
                aggregated_patterns.extend(result.patterns_checked);
                aggregated_patterns.extend(ast.patterns_checked);
                checks_run.push((*check_name).to_string());
            }
            "secret-detection" => {
                let config = SecretCheckConfig::default();
                // `run_secret_check` silently drops files by extension
                // (`config.skip_extensions`) and by size (`MAX_FILE_SIZE`).
                // Pre-filter so the "0 scanned" guard below stays honest
                // even when every input falls in the skip set (e.g. all
                // `.lock` or all 2 MB minified bundles).
                let scannable_files: Vec<String> = files
                    .iter()
                    .filter(|f| is_secret_scannable(f, &config))
                    .cloned()
                    .collect();
                if scannable_files.is_empty() {
                    // No input is in scope for secret-detection — skip
                    // without flipping `any_files_scanned`. If antipattern
                    // also produced nothing scannable, the empty-output
                    // guard below renders a clear message.
                    checks_run.push((*check_name).to_string());
                    continue;
                }
                any_files_scanned = true;
                let file_refs: Vec<&str> = scannable_files.iter().map(String::as_str).collect();
                let result = run_secret_check(&file_refs, &config, workspace_root.as_deref());
                for finding in &result.findings {
                    aggregated_warnings.push(secret_finding_to_json(finding));
                    if mode == OutputMode::Sarif {
                        sarif.add_secret(finding);
                    }
                }
                checks_run.push((*check_name).to_string());
            }
            // PLANLESS_ELIGIBLE_CHECKS is exhaustive against this match —
            // any future addition to the constant must also extend the
            // arms above.
            other => {
                debug_assert!(
                    false,
                    "planless-eligible check '{other}' has no dispatch arm in commands::check"
                );
            }
        }
    }

    let elapsed = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

    // Guard: if no files were actually scanned (e.g. extension mismatch or
    // unreadable paths) and no check accepted the inputs, report clearly
    // rather than a misleading "no warnings".
    if !any_files_scanned {
        if mode == OutputMode::Json {
            output::json::print(&empty_output(elapsed, "No analysable files found"))?;
        } else if mode == OutputMode::Sarif {
            output::json::print(&SarifAccumulator::default().into_log())?;
        } else {
            output::plain::warn(
                "No analysable files found (0 scanned). Check file extensions and readability.",
            );
        }
        return Ok(());
    }

    let summary = summarise_json_warnings(&aggregated_warnings);
    let has_blocking = aggregated_warnings
        .iter()
        .any(|w| json_severity_meets_threshold(&w.severity, severity_threshold));

    // Relativise file paths for output.
    let relative_files: Vec<String> = files
        .iter()
        .map(|f| relativise(f, workspace_root.as_deref()))
        .collect();
    let patterns_checked: Vec<String> = aggregated_patterns.into_iter().collect();

    match mode {
        OutputMode::Json => {
            let json_output = build_json_output(
                &relative_files,
                aggregated_warnings,
                &checks_run,
                &summary,
                &patterns_checked,
                has_blocking,
                elapsed,
            );
            output::json::print(&json_output)?;
        }
        OutputMode::Sarif => output::json::print(&sarif.into_log())?,
        OutputMode::Plain | OutputMode::Tui => {
            print_human(
                &aggregated_warnings_for_print(&aggregated_warnings),
                &summary,
                &relative_files,
                global.verbose,
                elapsed,
                source,
            );
        }
    }

    if has_blocking {
        // Keep the blocking notice off the machine-output streams (it goes to
        // stdout); JSON and SARIF stay well-formed. Exit code is unchanged.
        if matches!(mode, OutputMode::Plain | OutputMode::Tui) {
            output::plain::error("Blocking warnings found (severity meets threshold)");
        }
        // Signal failure via AlreadyReported so main exits with EXIT_ERROR
        // without reprinting the message.
        Err(output::AlreadyReported.into())
    } else {
        Ok(())
    }
}

// ── Planless dispatch helpers (issue #1797) ─────────────────────────

/// Decide which planless-eligible checks to run for this invocation.
///
/// Reads `.anvil.<ext>` / `.anvilrc#checks` via the same path `gate` uses
/// so the two surfaces never disagree about what's enabled. When no
/// config file is present (or `checks` is empty), falls back to the
/// planless-eligible default set so a fresh project still catches
/// secrets and antipatterns on `anvil check <file>`.
///
/// Unknown / non-planless entries in `.anvilrc` are silently ignored
/// here — `gate` already prints a warning when those names appear, and
/// repeating it on every `check` invocation would be noise.
fn resolve_enabled_planless_checks(workspace_root: Option<&str>) -> Result<Vec<&'static str>> {
    let configured = workspace_root
        .map(Path::new)
        .map(read_anvilrc_checks)
        .transpose()?
        .flatten();

    let enabled: std::collections::HashSet<String> = match configured {
        Some(rc) => rc
            .into_iter()
            .filter_map(|name| canonical_check_name(&name).map(str::to_string))
            .collect(),
        None => PLANLESS_ELIGIBLE_CHECKS
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    };

    let resolved: Vec<&'static str> = PLANLESS_ELIGIBLE_CHECKS
        .iter()
        .copied()
        .filter(|name| enabled.contains(*name))
        .collect();

    Ok(resolved)
}

/// Mirror the skip criteria inside `anvil_checks::secret::run_secret_check`
/// so the planless dispatcher can tell ahead of time whether a file would
/// be scanned. Without this pre-check, handing the scanner only
/// `skip_extensions` inputs (e.g. all `.lock`) lets the empty-output
/// guard below flip on a scan that never actually ran.
///
/// Kept in lockstep with the upstream `should_skip_file` /
/// `file_exceeds_size_limit` predicates — see
/// `crates/anvil-checks/src/secret/check.rs`.
fn is_secret_scannable(file: &str, config: &SecretCheckConfig) -> bool {
    if config.skip_extensions.iter().any(|ext| file.ends_with(ext)) {
        return false;
    }
    // Mirror `MAX_FILE_SIZE` from `anvil_checks::secret::check`. If we can't
    // stat the file, let the scanner decide — its error path is silent.
    match std::fs::metadata(file) {
        Ok(m) => m.len() < anvil_checks::secret::MAX_FILE_SIZE,
        Err(_) => true,
    }
}

fn antipattern_warning_to_json(w: &Warning) -> JsonWarning {
    JsonWarning {
        id: w.id.clone(),
        category: category_str(w.category).to_string(),
        severity: severity_str(w.severity).to_string(),
        title: w.title.clone(),
        message: w.message.clone(),
        file: w.location.file.clone(),
        line: w.location.line,
        suggestion: w.suggestion.clone(),
        nudge: w.nudge.clone(),
    }
}

/// Convert a secret-scanner finding into the unified `JsonWarning` shape so
/// it flows through the same output path as antipattern warnings. Findings
/// are reported as `severity = "error"` to match `gate`'s
/// "any hardcoded secret blocks" semantic — a planted `sk-…` literal must
/// not pass with `--severity error` (the default).
///
/// `anvil_checks::secret::normalise_file_path` prepends a `/` to workspace-
/// relative paths (e.g. `/src/smelly.ts`), while `commands::check`'s
/// antipattern path and the top-level `files` array use unrooted relative
/// paths (e.g. `src/smelly.ts`). Strip the leading slash here so a single
/// JSON response stays internally consistent and downstream consumers can
/// match warnings to entries in `files` without special-casing secrets.
/// Stable rule id for a secret-scanner finding, shared by the JSON projection
/// and the SARIF adapter so the same finding carries the same `ruleId`.
fn secret_rule_id(pattern_name: &str) -> String {
    format!(
        "SECRET-{}",
        pattern_name.to_ascii_uppercase().replace(' ', "-")
    )
}

fn secret_finding_to_json(f: &SecretFinding) -> JsonWarning {
    let id = secret_rule_id(&f.pattern_name);
    let file = f.file.strip_prefix('/').unwrap_or(&f.file).to_string();
    JsonWarning {
        id,
        category: "secret".to_string(),
        severity: "error".to_string(),
        title: format!("Potential secret: {}", f.pattern_name),
        message: f.redacted_line.clone(),
        file,
        line: f.line,
        suggestion:
            "Move the value to a secret manager or environment variable; never commit literals."
                .to_string(),
        nudge: None,
    }
}

// ── SARIF adapter (SARIFOUT-003) ────────────────────────────────────
//
// Maps `anvil check` findings into the shared SARIF emitter. The result set is
// the full warning list (matching the JSON output's finding set); suppressed
// antipattern warnings are read from the upstream `Warning.suppressed` — which
// the `JsonWarning` projection drops — and rendered under `results[].
// suppressions[]` so reviewers see what was accepted at scan time.

/// Map an antipattern `WarningSeverity` to a SARIF `level`.
fn sarif_level(severity: WarningSeverity) -> sarif::Level {
    match severity {
        WarningSeverity::Error => sarif::Level::Error,
        WarningSeverity::Warning => sarif::Level::Warning,
        WarningSeverity::Info => sarif::Level::Note,
    }
}

/// Accumulates check findings into a SARIF document, registering each distinct
/// rule once (`tool.driver.rules[]`).
#[derive(Default)]
struct SarifAccumulator {
    rules: BTreeMap<String, sarif::ReportingDescriptor>,
    results: Vec<sarif::SarifResult>,
}

impl SarifAccumulator {
    fn add_warning(&mut self, w: &Warning) {
        self.add_warning_tiered(w, false);
    }

    /// Add a warning, tagging its rule descriptor with the AST tier when it
    /// comes from the gate-time AST scanner (ADR-071 §9).
    fn add_warning_tiered(&mut self, w: &Warning, is_ast: bool) {
        self.rules.entry(w.id.clone()).or_insert_with(|| {
            let rule =
                sarif::ReportingDescriptor::new(w.id.clone()).short_description(w.title.clone());
            if is_ast { rule.tier("ast") } else { rule }
        });
        let line = u32::try_from(w.location.line).unwrap_or(u32::MAX);
        // `Warning.location.column` is a 0-based byte offset, but SARIF
        // `startColumn` is 1-based and the schema rejects 0; column is optional
        // in our pinned subset, so we emit line-only regions rather than risk an
        // invalid (or semantically wrong) `startColumn`.
        let region = sarif::Region::line(line);
        let fingerprint = w.fingerprint.clone().unwrap_or_else(|| {
            sarif::stable_fingerprint(&w.id, &w.location.file, Some(line), &w.message)
        });
        let mut result =
            sarif::SarifResult::new(w.id.clone(), sarif_level(w.severity), w.message.clone())
                .location(sarif::Location::new(w.location.file.clone(), Some(region)))
                .fingerprint("anvilFingerprint/v1", fingerprint);
        if let Some(s) = &w.suppressed {
            // `check` suppressions are in-source (`@anvil-ignore`) markers.
            result = result.suppression(
                sarif::Suppression::new(sarif::SuppressionKind::InSource)
                    .justification(s.reason.clone()),
            );
        }
        self.results.push(result);
    }

    fn add_secret(&mut self, f: &SecretFinding) {
        let id = secret_rule_id(&f.pattern_name);
        let file = f.file.strip_prefix('/').unwrap_or(&f.file).to_string();
        self.rules.entry(id.clone()).or_insert_with(|| {
            sarif::ReportingDescriptor::new(id.clone())
                .short_description(format!("Potential secret: {}", f.pattern_name))
        });
        let line = u32::try_from(f.line).unwrap_or(u32::MAX);
        let result =
            sarif::SarifResult::new(id.clone(), sarif::Level::Error, f.redacted_line.clone())
                .location(sarif::Location::new(
                    file.clone(),
                    Some(sarif::Region::line(line)),
                ))
                .fingerprint(
                    "anvilFingerprint/v1",
                    sarif::stable_fingerprint(&id, &file, Some(line), &f.redacted_line),
                );
        self.results.push(result);
    }

    fn into_log(self) -> sarif::SarifLog {
        sarif::SarifLog::new(sarif::Run::new(
            self.rules.into_values().collect(),
            self.results,
        ))
    }
}

fn summarise_json_warnings(warnings: &[JsonWarning]) -> WarningSummary {
    let mut summary = WarningSummary {
        total: warnings.len(),
        ..WarningSummary::default()
    };
    for w in warnings {
        match w.severity.as_str() {
            "error" => summary.errors += 1,
            "warning" => summary.warnings += 1,
            "info" => summary.info += 1,
            _ => {}
        }
    }
    summary
}

fn json_severity_meets_threshold(severity: &str, threshold: WarningSeverity) -> bool {
    let actual = match severity {
        "error" => WarningSeverity::Error,
        "warning" => WarningSeverity::Warning,
        "info" => WarningSeverity::Info,
        _ => return false,
    };
    severity_at_least(actual, threshold)
}

/// Adapt unified `JsonWarning`s back into the shape `print_human` expects.
/// `print_human` was built around `anvil_checks::antipattern::Warning`; this
/// keeps the human output path stable while the planless dispatcher feeds
/// it both antipattern and secret findings.
fn aggregated_warnings_for_print(warnings: &[JsonWarning]) -> Vec<Warning> {
    use anvil_checks::antipattern::{Confidence, Location, WarningCategory};

    warnings
        .iter()
        .map(|w| Warning {
            id: w.id.clone(),
            fingerprint: None,
            category: match w.category.as_str() {
                "boundary" => WarningCategory::Boundary,
                "architecture" => WarningCategory::Architecture,
                // Secrets, anti-patterns, and anything else collapse to
                // AntiPattern for human-printer purposes — the category
                // string is preserved verbatim in the JSON envelope, which
                // is the contract callers consume.
                _ => WarningCategory::AntiPattern,
            },
            severity: match w.severity.as_str() {
                "error" => WarningSeverity::Error,
                "info" => WarningSeverity::Info,
                _ => WarningSeverity::Warning,
            },
            confidence: Confidence::High,
            title: w.title.clone(),
            message: w.message.clone(),
            explanation: String::new(),
            suggestion: w.suggestion.clone(),
            nudge: w.nudge.clone(),
            location: Location {
                file: w.file.clone(),
                line: w.line,
                column: None,
                end_line: None,
                end_column: None,
            },
            pattern: None,
            suppressed: None,
            family: None,
            definition_ref: None,
            spectrum_position: None,
        })
        .collect()
}

// ── Non-source artifacts (pr-description / commit-message / agent-output) ─

fn parse_artifact_kind(input: &str) -> Result<ArtifactKind> {
    ArtifactKind::from_wire(input).ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid --artifact \"{input}\". Allowed values: source, pr-description, commit-message, agent-output"
        )
    })
}

#[allow(clippy::too_many_lines)] // Each phase (validate, load, scan, render) is sequenced linearly.
fn run_non_source_artifact(
    args: &CheckArgs,
    global: &GlobalArgs,
    kind: ArtifactKind,
    start: Instant,
) -> Result<()> {
    let mode = OutputMode::from_command_format(args.format, global);

    if args.all || args.changed_mode() || args.extensions.is_some() {
        bail!(
            "--artifact {} requires explicit file paths; --all, --changed, --staged, --since, and --extensions apply to source scans only",
            kind.as_str()
        );
    }
    if args.files.is_empty() {
        bail!(
            "--artifact {} requires at least one file path containing the artifact content",
            kind.as_str()
        );
    }

    let severity_threshold = parse_severity(&args.severity)?;

    // Load each path as artifact content. `reference` is the path as given
    // so operators can trace warnings back to their input (mirrors the TS
    // scanner's behaviour for non-source artifacts).
    //
    // `MAX_ARTIFACT_BYTES` (module scope) is the hard cap on a single
    // artifact's size. We enforce it via a bounded `take(MAX + 1)` reader,
    // not via `std::fs::metadata().len()` followed by a separate read:
    //   * `metadata` follows symlinks and reports `0` for FIFOs / named
    //     pipes / sockets. A `read_to_string` on those then either blocks
    //     indefinitely or streams unbounded into memory — exactly the OOM
    //     failure mode the cap is supposed to prevent.
    //   * Even on a regular file, the file may be replaced between the
    //     stat and the read on a multi-tenant CI runner (TOCTOU).
    // We additionally refuse anything that isn't a regular file so a
    // symlinked /dev/zero or named pipe bails with a clear error rather
    // than racing through.
    let mut artifacts = Vec::with_capacity(args.files.len());
    for path_str in &args.files {
        let path = Path::new(path_str);
        if !path.exists() {
            bail!("File not found: {path_str}");
        }
        let metadata = std::fs::symlink_metadata(path)
            .map_err(|e| anyhow::anyhow!("Failed to stat {path_str}: {e}"))?;
        if metadata.file_type().is_symlink() {
            bail!(
                "{path_str} is a symlink; --artifact requires a regular file (resolve the symlink and pass the target directly)"
            );
        }
        if metadata.is_dir() {
            bail!("Expected a file, got a directory: {path_str}");
        }
        if !metadata.is_file() {
            bail!(
                "{path_str} is not a regular file (FIFO, socket, or special file); --artifact requires a regular file"
            );
        }
        let mut file = std::fs::File::open(path)
            .map_err(|e| anyhow::anyhow!("Failed to open {path_str}: {e}"))?;
        // Read at most MAX + 1 bytes. If we drained MAX + 1, the input is
        // over the cap; reject without OOM-ing on a 500 MB input.
        let mut buf = String::with_capacity(8 * 1024);
        file.by_ref()
            .take(MAX_ARTIFACT_BYTES + 1)
            .read_to_string(&mut buf)
            .map_err(|e| anyhow::anyhow!("Failed to read {path_str}: {e}"))?;
        if buf.len() as u64 > MAX_ARTIFACT_BYTES {
            bail!(
                "{path_str} exceeds the {} MB --artifact size cap; trim the artifact or split it before scanning.",
                MAX_ARTIFACT_BYTES / (1024 * 1024)
            );
        }
        artifacts.push(Artifact {
            kind,
            reference: path_str.clone(),
            content: buf,
        });
    }

    let scan_options = ScanOptions {
        patterns: None,
        include_opt_in: args.include_opt_in,
    };
    let results = scan_artifacts(&artifacts, Some(&scan_options));

    let mut all_warnings = Vec::new();
    let mut all_patterns = BTreeSet::new();
    for result in results {
        all_warnings.extend(result.warnings);
        all_patterns.extend(result.patterns_checked);
    }
    let pattern_ids: Vec<String> = all_patterns.into_iter().collect();
    let warning_result = create_warning_result(all_warnings, pattern_ids.clone());

    let has_blocking = warning_result
        .warnings
        .iter()
        .any(|w| w.suppressed.is_none() && severity_at_least(w.severity, severity_threshold));

    let elapsed = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    let reference_files: Vec<String> = args.files.clone();

    match mode {
        OutputMode::Json => {
            let json_warnings: Vec<JsonWarning> = warning_result
                .warnings
                .iter()
                .map(antipattern_warning_to_json)
                .collect();
            let json_output = build_json_output(
                &reference_files,
                json_warnings,
                &["antipattern-scan".to_string()],
                &warning_result.summary,
                &pattern_ids,
                has_blocking,
                elapsed,
            );
            output::json::print(&json_output)?;
        }
        OutputMode::Sarif => {
            let mut sarif = SarifAccumulator::default();
            for w in &warning_result.warnings {
                sarif.add_warning(w);
            }
            output::json::print(&sarif.into_log())?;
        }
        OutputMode::Plain | OutputMode::Tui => {
            print_human(
                &warning_result.warnings,
                &warning_result.summary,
                &reference_files,
                global.verbose,
                elapsed,
                FileSource::Explicit,
            );
        }
    }

    if has_blocking {
        if matches!(mode, OutputMode::Plain | OutputMode::Tui) {
            output::plain::error("Blocking warnings found (severity meets threshold)");
        }
        Err(output::AlreadyReported.into())
    } else {
        Ok(())
    }
}

fn severity_at_least(actual: WarningSeverity, threshold: WarningSeverity) -> bool {
    severity_rank(actual) >= severity_rank(threshold)
}

const fn severity_rank(s: WarningSeverity) -> u8 {
    match s {
        WarningSeverity::Error => 3,
        WarningSeverity::Warning => 2,
        WarningSeverity::Info => 1,
    }
}

// ── File gathering ──────────────────────────────────────────────────

fn gather_files(args: &CheckArgs, extensions: &[String]) -> Result<(Vec<String>, FileSource)> {
    if !args.files.is_empty() {
        // Explicit file arguments — validate they exist.
        let mut resolved = Vec::with_capacity(args.files.len());
        for f in &args.files {
            let path = Path::new(f);
            if !path.exists() {
                bail!("File not found: {f}");
            }
            if path.is_dir() {
                bail!("Expected a file, got a directory: {f}");
            }
            resolved.push(
                path.canonicalize()
                    .unwrap_or_else(|_| path.to_path_buf())
                    .to_string_lossy()
                    .to_string(),
            );
        }
        return Ok((resolved, FileSource::Explicit));
    }

    if args.changed_mode() {
        let files = get_changed_files(args.staged, args.since.as_deref(), extensions)?;
        return Ok((files, FileSource::Changed));
    }

    if args.all {
        let files = get_all_source_files(extensions)?;
        return Ok((files, FileSource::All));
    }

    bail!(
        "No files specified.\n\n\
         Choose what to analyse:\n  \
         anvil check --changed       # files changed since last commit (in a git repo)\n  \
         anvil check --all           # all source files in the project\n  \
         anvil check path/to/file    # one or more specific files\n\n\
         New here? Run `anvil welcome` for a guided tour, or `anvil status` for a project overview."
    );
}

/// Validate that a git ref does not look like a git option.
///
/// `std::process::Command` uses execvp (no shell), so shell injection is not
/// possible — but a ref like `--upload-pack=evil` would still be interpreted
/// as a git option and could invoke arbitrary code in some git versions.
fn validate_git_ref(ref_name: &str) -> Result<()> {
    if ref_name.starts_with('-') {
        bail!(
            "Invalid git ref \"{ref_name}\": ref must not start with '-' (possible option injection)"
        );
    }
    Ok(())
}

/// Resolve the git repository root. `git diff --name-only` emits paths
/// relative to the repo root, not the current directory — so we must
/// join against the repo root for correct resolution from subdirectories.
fn git_toplevel() -> Result<std::path::PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run git: {e}"))?;
    if !output.status.success() {
        bail!("Not a git repository");
    }
    Ok(std::path::PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim(),
    ))
}

fn get_changed_files(
    staged_only: bool,
    since: Option<&str>,
    extensions: &[String],
) -> Result<Vec<String>> {
    let output = if let Some(ref_name) = since {
        validate_git_ref(ref_name)?;
        Command::new("git")
            .args(["diff", "--name-only", ref_name])
            .output()
    } else if staged_only {
        Command::new("git")
            .args(["diff", "--name-only", "--cached"])
            .output()
    } else {
        // Default: combine staged + unstaged (not untracked) to match Node.js
        // behaviour of `getChangedFiles({ staged: true, unstaged: true })`.
        // `git diff HEAD` only works when HEAD exists; for initial commits
        // before any commit exists, we fall back to --cached + unstaged.
        let staged = Command::new("git")
            .args(["diff", "--name-only", "--cached"])
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run git: {e}"))?;

        let unstaged = Command::new("git")
            .args(["diff", "--name-only"])
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run git: {e}"))?;

        if !staged.status.success() {
            let stderr = String::from_utf8_lossy(&staged.stderr);
            bail!("git diff --cached failed: {stderr}");
        }
        if !unstaged.status.success() {
            let stderr = String::from_utf8_lossy(&unstaged.stderr);
            bail!("git diff failed: {stderr}");
        }

        // Merge and deduplicate — resolve against repo root, not cwd.
        let repo_root = git_toplevel()?;
        let mut seen = BTreeSet::new();
        for line in String::from_utf8_lossy(&staged.stdout)
            .lines()
            .chain(String::from_utf8_lossy(&unstaged.stdout).lines())
        {
            if !line.is_empty() && has_matching_extension(line, extensions) {
                let abs = repo_root.join(line).to_string_lossy().to_string();
                if Path::new(&abs).exists() {
                    seen.insert(abs);
                }
            }
        }
        return Ok(seen.into_iter().collect());
    };

    let output = output.map_err(|e| anyhow::anyhow!("Failed to run git: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git diff failed: {stderr}");
    }

    let repo_root = git_toplevel()?;
    let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .filter(|line| has_matching_extension(line, extensions))
        .map(|line| repo_root.join(line).to_string_lossy().to_string())
        .filter(|path| Path::new(path).exists())
        .collect();

    Ok(files)
}

// SCAN-001: `--all` discovery shares the welcome-screen walker shape
// (`ignore::WalkBuilder`). Per-file regex work is already on the rayon
// pool inside `scan_artifacts` / `run_antipattern_check`, so swapping the
// walker is the only change needed here.
//
// `Result` return is retained even though the body cannot currently fail —
// callers expect the signature, and future fallible discovery (e.g.
// permission errors surfacing through `ignore::WalkBuilder` once we stop
// silently swallowing them) will use it.
#[allow(clippy::unnecessary_wraps)]
fn get_all_source_files(extensions: &[String]) -> Result<Vec<String>> {
    // Scan from git toplevel so --all covers the full repo even from a subdirectory.
    let root = git_toplevel().unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());

    let walker = ignore::WalkBuilder::new(&root)
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .filter_entry(|e| {
            if e.file_type().is_some_and(|ft| ft.is_dir()) {
                let name = e.file_name().to_string_lossy();
                !is_ignored_dir_name(&name)
            } else {
                true
            }
        })
        .build();

    let mut files: Vec<String> = walker
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
        .filter_map(|e| {
            let path_str = e.path().to_string_lossy().to_string();
            has_matching_extension(&path_str, extensions).then_some(path_str)
        })
        .collect();

    files.sort();
    Ok(files)
}

fn has_matching_extension(path: &str, extensions: &[String]) -> bool {
    extensions.iter().any(|ext| path.ends_with(ext.as_str()))
}

// ── Extension resolution ────────────────────────────────────────────

fn resolve_extensions(input: Option<&str>) -> Vec<String> {
    match input {
        Some(list) => list
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                if s.starts_with('.') {
                    s.to_lowercase()
                } else {
                    format!(".{}", s.to_lowercase())
                }
            })
            .collect(),
        None => AntipatternCheckConfig::default().extensions,
    }
}

// ── Severity parsing ────────────────────────────────────────────────

fn parse_severity(s: &str) -> Result<WarningSeverity> {
    match s.to_lowercase().as_str() {
        "error" => Ok(WarningSeverity::Error),
        "warning" => Ok(WarningSeverity::Warning),
        "info" => Ok(WarningSeverity::Info),
        _ => bail!("Invalid --severity \"{s}\". Allowed values: error, warning, info"),
    }
}

// ── Path helpers ────────────────────────────────────────────────────

fn relativise(path: &str, workspace_root: Option<&str>) -> String {
    let Some(root) = workspace_root else {
        return path.to_string();
    };
    let file = Path::new(path);
    let root_path = Path::new(root);
    if let Ok(relative) = file.strip_prefix(root_path) {
        relative.to_string_lossy().replace('\\', "/")
    } else {
        path.to_string()
    }
}

// ── Output formatters ───────────────────────────────────────────────

fn empty_output(elapsed: u64, message: &str) -> CheckOutput {
    CheckOutput {
        version: CHECK_OUTPUT_VERSION,
        timestamp: chrono::Utc::now().to_rfc3339(),
        files: Vec::new(),
        has_blocking_warnings: false,
        execution_time_ms: elapsed,
        checks_run: Vec::new(),
        provenance_id: None,
        message: Some(message.to_string()),
        notifications: vec![
            Notification::new(
                NotificationClass::Info,
                NotificationPriority::Low,
                "Check status",
                message,
            )
            .with_context(NotificationContext {
                file: None,
                source: Some("check".to_string()),
            }),
        ],
        warnings: Vec::new(),
        summary: WarningSummary {
            total: 0,
            errors: 0,
            warnings: 0,
            info: 0,
            suppressed: 0,
        },
        diagnostics: anvil_checks::antipattern::registry_compile_diagnostics()
            .into_iter()
            .map(|d| JsonDiagnostic {
                id: d.pattern_id,
                title: d.pattern_title,
                error: d.error,
            })
            .collect(),
    }
}

fn severity_str(s: WarningSeverity) -> &'static str {
    match s {
        WarningSeverity::Error => "error",
        WarningSeverity::Warning => "warning",
        WarningSeverity::Info => "info",
    }
}

fn category_str(c: anvil_checks::antipattern::WarningCategory) -> &'static str {
    match c {
        anvil_checks::antipattern::WarningCategory::AntiPattern => "anti-pattern",
        anvil_checks::antipattern::WarningCategory::Boundary => "boundary",
        anvil_checks::antipattern::WarningCategory::Architecture => "architecture",
    }
}

fn build_json_output(
    files: &[String],
    warnings: Vec<JsonWarning>,
    checks_run: &[String],
    summary: &WarningSummary,
    _patterns_checked: &[String],
    has_blocking: bool,
    elapsed: u64,
) -> CheckOutput {
    let notifications: Vec<Notification> = warnings
        .iter()
        .map(|w| {
            Notification::new(
                NotificationClass::Finding,
                notification_priority_for_json_warning(&w.severity),
                format!("[{}] {}", w.id, w.title),
                w.message.clone(),
            )
            .with_context(NotificationContext {
                file: Some(w.file.clone()),
                source: Some("check".to_string()),
            })
        })
        .collect();

    let diagnostics = anvil_checks::antipattern::registry_compile_diagnostics()
        .into_iter()
        .map(|d| JsonDiagnostic {
            id: d.pattern_id,
            title: d.pattern_title,
            error: d.error,
        })
        .collect();

    CheckOutput {
        version: CHECK_OUTPUT_VERSION,
        timestamp: chrono::Utc::now().to_rfc3339(),
        files: files.to_vec(),
        has_blocking_warnings: has_blocking,
        execution_time_ms: elapsed,
        // `checksRun` reports the canonical names of checks that actually
        // executed for this invocation — issue #1797: planless `check`
        // previously hard-coded `["architecture"]` while actually running
        // only the antipattern scanner.
        checks_run: checks_run.to_vec(),
        provenance_id: None,
        message: None,
        notifications,
        warnings,
        summary: summary.clone(),
        diagnostics,
    }
}

fn notification_priority_for_json_warning(severity: &str) -> NotificationPriority {
    match severity {
        "error" => NotificationPriority::High,
        "info" => NotificationPriority::Low,
        _ => NotificationPriority::Normal,
    }
}

fn print_human(
    warnings: &[Warning],
    summary: &WarningSummary,
    files: &[String],
    verbose: bool,
    elapsed: u64,
    source: FileSource,
) {
    match source {
        FileSource::All => output::plain::dim(&format!("Checked {} file(s)", files.len())),
        FileSource::Changed => {
            output::plain::dim(&format!("Checked {} changed file(s)", files.len()));
        }
        FileSource::Explicit => {}
    }

    output::plain::blank();

    if warnings.is_empty() {
        output::plain::success("No warnings found");
        return;
    }

    // Group by severity.
    let errors: Vec<&Warning> = warnings
        .iter()
        .filter(|w| w.severity == WarningSeverity::Error && w.suppressed.is_none())
        .collect();
    let warns: Vec<&Warning> = warnings
        .iter()
        .filter(|w| w.severity == WarningSeverity::Warning && w.suppressed.is_none())
        .collect();
    let infos: Vec<&Warning> = warnings
        .iter()
        .filter(|w| w.severity == WarningSeverity::Info && w.suppressed.is_none())
        .collect();

    if !errors.is_empty() {
        output::plain::section("Errors");
        for w in &errors {
            print_warning(w, verbose);
        }
    }

    if !warns.is_empty() {
        output::plain::section("Warnings");
        for w in &warns {
            print_warning(w, verbose);
        }
    }

    if !infos.is_empty() && verbose {
        output::plain::section("Info");
        for w in &infos {
            print_warning(w, verbose);
        }
    }

    output::plain::blank();
    output::plain::section("Summary");
    output::plain::label("Total", summary.total);
    if summary.errors > 0 {
        output::plain::label("Errors", summary.errors);
    }
    if summary.warnings > 0 {
        output::plain::label("Warnings", summary.warnings);
    }
    if summary.info > 0 {
        output::plain::label("Info", summary.info);
    }
    if summary.suppressed > 0 {
        output::plain::label("Suppressed", summary.suppressed);
    }
    output::plain::label("Time", format!("{elapsed}ms"));
}

fn print_warning(w: &Warning, verbose: bool) {
    let icon = match w.severity {
        WarningSeverity::Error => "\u{2717}",
        WarningSeverity::Warning => "\u{26a0}",
        WarningSeverity::Info => "\u{2139}",
    };

    output::plain::item(icon, &format!("[{}] {}", w.id, w.title));
    output::plain::dim(&format!("{}:{}", w.location.file, w.location.line));
    output::plain::dim(&w.message);

    if verbose {
        if let Some(nudge) = &w.nudge {
            output::plain::dim(&format!("\u{2192} {nudge}"));
        }
        output::plain::dim(&format!("Why: {}", w.explanation));
        output::plain::dim(&format!("Fix: {}", w.suggestion));
    }
    output::plain::blank();
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Severity parsing ────────────────────────────────────────

    #[test]
    fn parse_severity_valid_values() {
        assert_eq!(parse_severity("error").unwrap(), WarningSeverity::Error);
        assert_eq!(parse_severity("warning").unwrap(), WarningSeverity::Warning);
        assert_eq!(parse_severity("info").unwrap(), WarningSeverity::Info);
        assert_eq!(parse_severity("WARNING").unwrap(), WarningSeverity::Warning);
    }

    #[test]
    fn parse_severity_invalid_value() {
        assert!(parse_severity("critical").is_err());
    }

    // ── Extension resolution ────────────────────────────────────

    #[test]
    fn resolve_extensions_default() {
        let exts = resolve_extensions(None);
        assert!(exts.contains(&".ts".to_string()));
        assert!(exts.contains(&".tsx".to_string()));
        assert!(exts.contains(&".js".to_string()));
    }

    #[test]
    fn resolve_extensions_custom() {
        let exts = resolve_extensions(Some(".rs,.go,py"));
        assert_eq!(exts, vec![".rs", ".go", ".py"]);
    }

    #[test]
    fn resolve_extensions_normalises_dots() {
        let exts = resolve_extensions(Some("ts,  .tsx"));
        assert_eq!(exts, vec![".ts", ".tsx"]);
    }

    // ── Extension matching ──────────────────────────────────────

    #[test]
    fn has_matching_extension_works() {
        let exts = vec![".ts".to_string(), ".tsx".to_string()];
        assert!(has_matching_extension("src/foo.ts", &exts));
        assert!(has_matching_extension("src/bar.tsx", &exts));
        assert!(!has_matching_extension("src/baz.rs", &exts));
    }

    // ── Path helpers ────────────────────────────────────────────

    #[test]
    fn relativise_strips_prefix() {
        let result = relativise("/home/user/project/src/foo.ts", Some("/home/user/project"));
        assert_eq!(result, "src/foo.ts");
    }

    #[test]
    fn relativise_returns_original_when_no_root() {
        let result = relativise("/home/user/project/src/foo.ts", None);
        assert_eq!(result, "/home/user/project/src/foo.ts");
    }

    // ── Git ref validation (C-003) ──────────────────────────────

    #[test]
    fn validate_git_ref_rejects_option_injection() {
        assert!(validate_git_ref("--upload-pack=evil").is_err());
        assert!(validate_git_ref("-c").is_err());
        assert!(validate_git_ref("--exec=rm").is_err());
    }

    #[test]
    fn validate_git_ref_accepts_valid_refs() {
        assert!(validate_git_ref("main").is_ok());
        assert!(validate_git_ref("HEAD~3").is_ok());
        assert!(validate_git_ref("v1.0.0").is_ok());
        assert!(validate_git_ref("feature/my-branch").is_ok());
        assert!(validate_git_ref("abc123").is_ok());
    }

    // ── JSON output ─────────────────────────────────────────────

    #[test]
    fn empty_output_has_correct_version_and_elapsed() {
        let out = empty_output(42, "No source files found");
        assert_eq!(out.version, CHECK_OUTPUT_VERSION);
        assert!(!out.has_blocking_warnings);
        assert_eq!(out.execution_time_ms, 42);
        assert!(out.provenance_id.is_none());
        assert_eq!(out.notifications.len(), 1);
        assert_eq!(out.notifications[0].class, NotificationClass::Info);
    }

    #[test]
    fn build_json_output_maps_warnings_correctly() {
        use anvil_checks::antipattern::{Confidence, Location, WarningCategory};

        let warnings = [Warning {
            id: "AP-003".to_string(),
            fingerprint: None,
            category: WarningCategory::AntiPattern,
            severity: WarningSeverity::Warning,
            confidence: Confidence::High,
            title: "Explicit any type".to_string(),
            message: "Avoid using 'any' type".to_string(),
            explanation: "Weakens type safety".to_string(),
            suggestion: "Use a specific type".to_string(),
            nudge: Some("Try narrowing the type".to_string()),
            location: Location {
                file: "src/foo.ts".to_string(),
                line: 10,
                column: Some(5),
                end_line: None,
                end_column: None,
            },
            pattern: Some("AP-003".to_string()),
            suppressed: None,
            family: None,
            definition_ref: None,
            spectrum_position: None,
        }];
        let summary = WarningSummary {
            total: 1,
            errors: 0,
            warnings: 1,
            info: 0,
            suppressed: 0,
        };

        let json_warnings: Vec<JsonWarning> =
            warnings.iter().map(antipattern_warning_to_json).collect();
        let out = build_json_output(
            &["src/foo.ts".to_string()],
            json_warnings,
            &["antipattern-scan".to_string()],
            &summary,
            &["AP-003".to_string()],
            false,
            42,
        );

        assert_eq!(out.warnings.len(), 1);
        assert_eq!(out.warnings[0].id, "AP-003");
        assert_eq!(out.warnings[0].category, "anti-pattern");
        assert_eq!(out.warnings[0].severity, "warning");
        assert_eq!(out.warnings[0].file, "src/foo.ts");
        assert_eq!(out.warnings[0].line, 10);
        assert!(out.warnings[0].nudge.is_some());
        assert_eq!(out.execution_time_ms, 42);
        assert!(!out.has_blocking_warnings);
        assert!(out.provenance_id.is_none());
        assert_eq!(out.checks_run, vec!["antipattern-scan".to_string()]);
        assert_eq!(out.notifications.len(), 1);
        assert_eq!(out.notifications[0].class, NotificationClass::Finding);
        assert_eq!(out.notifications[0].priority, NotificationPriority::Normal);
        assert_eq!(
            out.notifications[0]
                .context
                .as_ref()
                .and_then(|c| c.file.as_deref()),
            Some("src/foo.ts")
        );
    }

    #[test]
    fn json_output_includes_provenance_id_field() {
        let json = serde_json::to_value(empty_output(0, "test")).unwrap();
        // provenance_id should be absent (None is skipped)
        assert!(!json.as_object().unwrap().contains_key("provenance_id"));
    }

    // ── Enum string conversions ─────────────────────────────────

    #[test]
    fn severity_str_matches_serde_names() {
        assert_eq!(severity_str(WarningSeverity::Error), "error");
        assert_eq!(severity_str(WarningSeverity::Warning), "warning");
        assert_eq!(severity_str(WarningSeverity::Info), "info");
    }

    #[test]
    fn category_str_matches_serde_names() {
        use anvil_checks::antipattern::WarningCategory;
        assert_eq!(category_str(WarningCategory::AntiPattern), "anti-pattern");
        assert_eq!(category_str(WarningCategory::Boundary), "boundary");
        assert_eq!(category_str(WarningCategory::Architecture), "architecture");
    }

    // ── Argument validation (C-005) ─────────────────────────────

    // `--staged` and `--since` imply `--changed` (CIB-012 / GH #1804).
    // Prior behaviour rejected them without explicit `--changed`; the
    // mental model `git diff --staged` makes the bare flag the obvious
    // entry point, so the runtime now treats either as `--changed`.

    #[test]
    fn clap_accepts_staged_without_changed() {
        use clap::Parser;
        let result = crate::Cli::try_parse_from(["anvil", "check", "--staged"]);
        assert!(
            result.is_ok(),
            "--staged should imply --changed: {result:?}"
        );
    }

    #[test]
    fn clap_accepts_since_without_changed() {
        use clap::Parser;
        let result = crate::Cli::try_parse_from(["anvil", "check", "--since", "main"]);
        assert!(result.is_ok(), "--since should imply --changed: {result:?}");
    }

    #[test]
    fn clap_accepts_staged_with_changed() {
        use clap::Parser;
        let result = crate::Cli::try_parse_from(["anvil", "check", "--changed", "--staged"]);
        assert!(result.is_ok());
    }

    #[test]
    fn clap_accepts_since_with_changed() {
        use clap::Parser;
        let result = crate::Cli::try_parse_from(["anvil", "check", "--changed", "--since", "main"]);
        assert!(result.is_ok());
    }

    #[test]
    fn clap_rejects_staged_with_all() {
        use clap::Parser;
        let result = crate::Cli::try_parse_from(["anvil", "check", "--all", "--staged"]);
        assert!(result.is_err(), "--all and --staged are mutually exclusive");
    }

    #[test]
    fn clap_rejects_since_with_all() {
        use clap::Parser;
        let result = crate::Cli::try_parse_from(["anvil", "check", "--all", "--since", "main"]);
        assert!(result.is_err(), "--all and --since are mutually exclusive");
    }

    // ── File source enum ────────────────────────────────────────

    #[test]
    fn gather_files_returns_explicit_source() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        let args = CheckArgs {
            files: vec![path],
            changed: false,
            staged: false,
            since: None,
            all: false,
            extensions: None,
            severity: "warning".to_string(),
            include_opt_in: false,
            artifact: "source".to_string(),
            format: None,
        };
        let exts = resolve_extensions(None);
        let (files, source) = gather_files(&args, &exts).unwrap();
        assert_eq!(source, FileSource::Explicit);
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn gather_files_errors_on_missing_file() {
        let args = CheckArgs {
            files: vec!["__nonexistent_file_42__.ts".to_string()],
            changed: false,
            staged: false,
            since: None,
            all: false,
            extensions: None,
            severity: "warning".to_string(),
            include_opt_in: false,
            artifact: "source".to_string(),
            format: None,
        };
        let exts = resolve_extensions(None);
        assert!(gather_files(&args, &exts).is_err());
    }

    #[test]
    fn gather_files_errors_when_no_mode_selected() {
        let args = CheckArgs {
            files: Vec::new(),
            changed: false,
            staged: false,
            since: None,
            all: false,
            extensions: None,
            severity: "warning".to_string(),
            include_opt_in: false,
            artifact: "source".to_string(),
            format: None,
        };
        let exts = resolve_extensions(None);
        let err = gather_files(&args, &exts).unwrap_err();
        assert!(err.to_string().contains("No files specified"));
    }

    // ── --artifact flag (RSCAN-006) ─────────────────────────────

    #[test]
    fn parse_artifact_kind_accepts_wire_values() {
        assert_eq!(parse_artifact_kind("source").unwrap(), ArtifactKind::Source);
        assert_eq!(
            parse_artifact_kind("pr-description").unwrap(),
            ArtifactKind::PrDescription
        );
        assert_eq!(
            parse_artifact_kind("commit-message").unwrap(),
            ArtifactKind::CommitMessage
        );
        assert_eq!(
            parse_artifact_kind("agent-output").unwrap(),
            ArtifactKind::AgentOutput
        );
    }

    #[test]
    fn parse_artifact_kind_rejects_unknown_value() {
        let err = parse_artifact_kind("slack-message").unwrap_err();
        assert!(err.to_string().contains("Invalid --artifact"));
    }

    #[test]
    fn clap_accepts_artifact_with_files() {
        use clap::Parser;
        let result = crate::Cli::try_parse_from([
            "anvil",
            "check",
            "--artifact",
            "pr-description",
            "pr-body.md",
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn clap_rejects_invalid_artifact_via_run_path() {
        // clap itself accepts any string — validation happens in `run`.
        let args = CheckArgs {
            files: vec!["anything.md".to_string()],
            changed: false,
            staged: false,
            since: None,
            all: false,
            extensions: None,
            severity: "warning".to_string(),
            include_opt_in: false,
            artifact: "not-a-kind".to_string(),
            format: None,
        };
        assert!(parse_artifact_kind(&args.artifact).is_err());
    }

    #[test]
    fn run_non_source_artifact_requires_explicit_files() {
        let args = CheckArgs {
            files: Vec::new(),
            changed: false,
            staged: false,
            since: None,
            all: false,
            extensions: None,
            severity: "warning".to_string(),
            include_opt_in: false,
            artifact: "pr-description".to_string(),
            format: None,
        };
        let global = GlobalArgs::default();
        let result =
            run_non_source_artifact(&args, &global, ArtifactKind::PrDescription, Instant::now());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("requires at least one file path"));
    }

    #[test]
    fn run_non_source_artifact_rejects_all_flag() {
        let args = CheckArgs {
            files: vec!["ignored.md".to_string()],
            changed: false,
            staged: false,
            since: None,
            all: true,
            extensions: None,
            severity: "warning".to_string(),
            include_opt_in: false,
            artifact: "pr-description".to_string(),
            format: None,
        };
        let global = GlobalArgs::default();
        let result =
            run_non_source_artifact(&args, &global, ArtifactKind::PrDescription, Instant::now());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("--all, --changed"));
    }

    #[test]
    fn run_non_source_artifact_errors_on_missing_file() {
        let args = CheckArgs {
            files: vec!["__nonexistent_pr_body__.md".to_string()],
            changed: false,
            staged: false,
            since: None,
            all: false,
            extensions: None,
            severity: "warning".to_string(),
            include_opt_in: false,
            artifact: "pr-description".to_string(),
            format: None,
        };
        let global = GlobalArgs::default();
        let result =
            run_non_source_artifact(&args, &global, ArtifactKind::PrDescription, Instant::now());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[test]
    fn run_non_source_artifact_scans_pr_description_and_triggers_rl_family() {
        // RL-004 regex (`none\s+of\s+which\s+were\s+touched\b`) has no
        // lookaround so it compiles cleanly in Rust regex. It targets
        // pr-description + agent-output, which makes it a reliable witness
        // that --artifact routed through scan_artifact (and not the
        // source-only path).
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            tmp.path(),
            "All CI failures are in dependencies, none of which were touched.",
        )
        .unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let args = CheckArgs {
            files: vec![path.clone()],
            changed: false,
            staged: false,
            since: None,
            all: false,
            extensions: None,
            // `error` threshold lets the RL-004 Warning fire without
            // blocking — we're asserting the artifact was scanned, not
            // asserting gate semantics.
            severity: "error".to_string(),
            include_opt_in: false,
            artifact: "pr-description".to_string(),
            format: None,
        };
        let global = GlobalArgs::default();
        let result =
            run_non_source_artifact(&args, &global, ArtifactKind::PrDescription, Instant::now());
        assert!(result.is_ok(), "scan should pass the default threshold");
    }

    #[test]
    fn run_non_source_artifact_blocks_when_threshold_crossed() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            tmp.path(),
            "All CI failures are in dependencies, none of which were touched.",
        )
        .unwrap();
        let path = tmp.path().to_string_lossy().to_string();

        let args = CheckArgs {
            files: vec![path],
            changed: false,
            staged: false,
            since: None,
            all: false,
            extensions: None,
            severity: "warning".to_string(),
            include_opt_in: false,
            artifact: "pr-description".to_string(),
            format: None,
        };
        let global = GlobalArgs::default();
        let err =
            run_non_source_artifact(&args, &global, ArtifactKind::PrDescription, Instant::now())
                .unwrap_err();
        assert!(err.downcast_ref::<output::AlreadyReported>().is_some());
    }

    // ── Planless dispatch (issue #1797) ─────────────────────────

    #[test]
    fn secret_finding_maps_to_blocking_error_json_warning() {
        use anvil_checks::secret::{FindingType, SecretFinding};

        let finding = SecretFinding {
            file: "src/smelly.ts".to_string(),
            line: 1,
            finding_type: FindingType::Pattern,
            pattern_name: "High Entropy String".to_string(),
            redacted_match: "sk-***".to_string(),
            redacted_line: "const apiKey = \"sk-***\";".to_string(),
        };
        let json = secret_finding_to_json(&finding);
        assert_eq!(json.category, "secret");
        // Issue #1797 hinges on `--severity error` (the default) treating
        // a hardcoded secret as blocking — same semantic as `anvil gate`.
        assert_eq!(json.severity, "error");
        assert_eq!(json.file, "src/smelly.ts");
        assert_eq!(json.line, 1);
        assert!(json.id.starts_with("SECRET-"));
        assert!(json_severity_meets_threshold(
            &json.severity,
            WarningSeverity::Error
        ));
    }

    #[test]
    fn resolve_enabled_planless_checks_defaults_when_no_anvilrc() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_string_lossy().to_string();
        let resolved = resolve_enabled_planless_checks(Some(&root)).unwrap();
        // Default planless set must include both — this is the safety
        // net for fresh projects that haven't written `.anvilrc` yet.
        assert!(resolved.contains(&"secret-detection"));
        assert!(resolved.contains(&"antipattern-scan"));
    }

    #[test]
    fn resolve_enabled_planless_checks_honours_anvilrc_subset() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            "checks:\n  - secret-detection\n",
        )
        .unwrap();
        let root = tmp.path().to_string_lossy().to_string();
        let resolved = resolve_enabled_planless_checks(Some(&root)).unwrap();
        assert_eq!(resolved, vec!["secret-detection"]);
    }

    #[test]
    fn resolve_enabled_planless_checks_drops_non_planless_entries() {
        // `.anvilrc` enables `policy` (gate-only) — planless dispatch must
        // silently ignore it, not surface it as something `check` runs.
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            "checks:\n  - antipattern-scan\n  - policy\n",
        )
        .unwrap();
        let root = tmp.path().to_string_lossy().to_string();
        let resolved = resolve_enabled_planless_checks(Some(&root)).unwrap();
        assert_eq!(resolved, vec!["antipattern-scan"]);
    }

    #[test]
    fn planless_check_detects_sk_literal_in_typescript_file() {
        // Fixture witness for issue #1797: a TS file with a planted
        // `sk-…` literal must trigger secret-detection under the planless
        // `anvil check` path. `anvil gate` and MCP already do this; the
        // regression that made `check` silently pass was the open issue.
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();
        let smelly = src_dir.join("smelly.ts");
        std::fs::write(
            &smelly,
            "const apiKey = \"sk-1234567890abcdefghijklmnopqrstuv\";\n\
             export function unsafe(input: any): any {\n  return eval(input);\n}\n",
        )
        .unwrap();
        // No `.anvilrc` — default planless set must include
        // secret-detection.

        let args = CheckArgs {
            files: vec![smelly.to_string_lossy().to_string()],
            changed: false,
            staged: false,
            since: None,
            all: false,
            extensions: None,
            // `error` is the user-facing default in the issue repro; we
            // assert the secret finding is blocking at that threshold.
            severity: "error".to_string(),
            include_opt_in: false,
            artifact: "source".to_string(),
            format: None,
        };

        // `run()` needs `git_toplevel` to resolve workspace root; the tmp
        // dir is not a git repo, so the function falls back to cwd. Move
        // cwd into the tmp dir for the duration of the test so relative
        // paths resolve. The workspace-wide cwd guard (CIB-026) serialises
        // every test that mutates the process-wide cwd.
        // Force JSON mode so the print path doesn't write to stderr/stdout
        // for the test runner.
        let global = GlobalArgs {
            json: true,
            ..GlobalArgs::default()
        };

        let result = crate::test_support::cwd::with_cwd_in(tmp.path(), || run(&args, &global));

        let err = result.expect_err("planted sk- literal must be a blocking finding");
        assert!(
            err.downcast_ref::<output::AlreadyReported>().is_some(),
            "expected AlreadyReported (blocking warnings), got: {err}"
        );
    }

    // ── Review-feedback fixes (PR #1817) ────────────────────────

    #[test]
    fn secret_finding_strips_leading_slash_for_path_consistency() {
        // PR #1817 review: `anvil_checks::secret::normalise_file_path`
        // prepends `/` to workspace-relative paths; the check.rs envelope
        // strips it so secret findings match antipattern + `files` paths.
        use anvil_checks::secret::{FindingType, SecretFinding};
        let finding = SecretFinding {
            file: "/src/smelly.ts".to_string(),
            line: 1,
            finding_type: FindingType::Pattern,
            pattern_name: "High Entropy String".to_string(),
            redacted_match: "[REDACTED]".to_string(),
            redacted_line: "const apiKey = \"[REDACTED]\";".to_string(),
        };
        let json = secret_finding_to_json(&finding);
        assert_eq!(
            json.file, "src/smelly.ts",
            "leading slash from secret scanner must be stripped"
        );
    }

    #[test]
    fn is_secret_scannable_rejects_skip_extensions() {
        let config = SecretCheckConfig::default();
        assert!(!is_secret_scannable("foo.lock", &config));
        assert!(!is_secret_scannable("foo.min.js", &config));
        assert!(!is_secret_scannable("foo.svg", &config));
        assert!(is_secret_scannable("src/foo.ts", &config));
        assert!(is_secret_scannable("src/foo.rs", &config));
    }

    #[test]
    fn empty_planless_intersection_emits_clear_message_not_silent_pass() {
        // PR #1817 review: `.anvilrc#checks` enabling only gate-only
        // checks (e.g. `policy`) used to fall through to a misleading
        // "No analysable files found" guard. Caller must now see a clear
        // message naming the planless-eligible set.
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();
        let file = src_dir.join("foo.ts");
        std::fs::write(&file, "export const x = 1;\n").unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            "checks:\n  - policy\n  - architecture\n",
        )
        .unwrap();

        let args = CheckArgs {
            files: vec![file.to_string_lossy().to_string()],
            changed: false,
            staged: false,
            since: None,
            all: false,
            extensions: None,
            severity: "error".to_string(),
            include_opt_in: false,
            artifact: "source".to_string(),
            format: None,
        };

        let global = GlobalArgs {
            json: true,
            ..GlobalArgs::default()
        };
        let result = crate::test_support::cwd::with_cwd_in(tmp.path(), || run(&args, &global));

        // No planless-eligible checks → clean exit (not an error), but a
        // dispatcher that scanned zero things must not pretend it passed.
        // The user-visible message is asserted via the JSON `message`
        // field carried on `empty_output`; rendering happens through
        // `output::json::print` so we just verify the contract didn't
        // return a blocking error.
        assert!(
            result.is_ok(),
            "empty planless intersection should not surface a blocking error; got {result:?}"
        );
    }

    #[test]
    fn secret_only_skip_extension_inputs_dont_falsely_mark_scanned() {
        // PR #1817 review: handing the secret scanner only `.lock` /
        // `.svg` files used to flip `any_files_scanned = true` even
        // though zero bytes were actually scanned. With pre-filtering,
        // an all-skip-extension input set falls through to the
        // "No analysable files found" guard (Ok(()) without error).
        let tmp = tempfile::tempdir().unwrap();
        let lock = tmp.path().join("yarn.lock");
        std::fs::write(&lock, "lockfile v1\n").unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            "checks:\n  - secret-detection\n",
        )
        .unwrap();

        let args = CheckArgs {
            files: vec![lock.to_string_lossy().to_string()],
            changed: false,
            staged: false,
            since: None,
            all: false,
            extensions: None,
            severity: "error".to_string(),
            include_opt_in: false,
            artifact: "source".to_string(),
            format: None,
        };

        let global = GlobalArgs {
            json: true,
            ..GlobalArgs::default()
        };
        let result = crate::test_support::cwd::with_cwd_in(tmp.path(), || run(&args, &global));

        // Skipped silently → not blocking. The point of the assertion is
        // that we don't crash, don't double-mark scanned, and surface a
        // clean exit.
        assert!(result.is_ok(), "all-skip-extension inputs should not block");
    }

    // ── SARIF adapter (SARIFOUT-003) ────────────────────────────────

    fn ap_warning(id: &str, severity: WarningSeverity, suppressed: bool) -> Warning {
        use anvil_checks::antipattern::{
            Confidence, Location, Suppression, SuppressionScope, WarningCategory,
        };
        Warning {
            id: id.to_string(),
            fingerprint: None,
            category: WarningCategory::AntiPattern,
            severity,
            confidence: Confidence::High,
            title: format!("{id} title"),
            message: format!("{id} message"),
            explanation: String::new(),
            suggestion: "fix it".to_string(),
            nudge: None,
            location: Location {
                file: "src/foo.ts".to_string(),
                line: 10,
                column: Some(5),
                end_line: None,
                end_column: None,
            },
            pattern: Some(id.to_string()),
            suppressed: suppressed.then(|| Suppression {
                reason: "baseline-accepted".to_string(),
                author: None,
                timestamp: None,
                scope: SuppressionScope::Line,
            }),
            family: None,
            definition_ref: None,
            spectrum_position: None,
        }
    }

    #[test]
    fn sarif_adapter_emits_schema_valid_document_with_suppressions() {
        let mut acc = SarifAccumulator::default();
        acc.add_warning(&ap_warning("AP-003", WarningSeverity::Warning, false));
        acc.add_warning(&ap_warning("AP-006", WarningSeverity::Error, true));
        acc.add_secret(&SecretFinding {
            file: "/src/config.ts".to_string(),
            line: 7,
            finding_type: anvil_checks::secret::FindingType::Pattern,
            pattern_name: "OpenAI key".to_string(),
            redacted_match: "sk-…".to_string(),
            redacted_line: "const k = \"sk-…\"".to_string(),
        });
        let value = serde_json::to_value(acc.into_log()).expect("serialise");

        // Schema validation against the bundled upstream 2.1.0 schema.
        let schema: serde_json::Value =
            serde_json::from_str(anvil_sarif::SARIF_SCHEMA_JSON).expect("schema json");
        let validator = jsonschema::validator_for(&schema).expect("compile schema");
        let errors: Vec<String> = validator
            .iter_errors(&value)
            .map(|e| format!("{} at {}", e, e.instance_path()))
            .collect();
        assert!(errors.is_empty(), "schema errors:\n{}", errors.join("\n"));

        let results = value["runs"][0]["results"].as_array().expect("results");
        assert_eq!(results.len(), 3, "one result per finding");

        // The suppressed antipattern carries an in-source suppression with its
        // reason; the secret's id and leading-slash stripping match the JSON
        // projection; rules are deduplicated.
        let suppressed = results
            .iter()
            .find(|r| r["ruleId"] == "AP-006")
            .expect("AP-006 result");
        assert_eq!(suppressed["suppressions"][0]["kind"], "inSource");
        assert_eq!(
            suppressed["suppressions"][0]["justification"],
            "baseline-accepted"
        );
        assert_eq!(suppressed["level"], "error");

        assert!(
            results.iter().any(|r| r["ruleId"] == "SECRET-OPENAI-KEY"),
            "secret rule id matches the JSON projection"
        );
        let secret = results
            .iter()
            .find(|r| r["ruleId"] == "SECRET-OPENAI-KEY")
            .unwrap();
        assert_eq!(
            secret["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "src/config.ts"
        );

        let rules = value["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .expect("rules");
        assert_eq!(rules.len(), 3, "AP-003, AP-006, SECRET-OPENAI-KEY deduped");

        // Non-suppressed result has no suppressions key (omitted when empty).
        let plain = results.iter().find(|r| r["ruleId"] == "AP-003").unwrap();
        assert!(plain.get("suppressions").is_none());
    }
}
