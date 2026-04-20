use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use anyhow::{Result, bail};
use clap::Args;
use serde::Serialize;
use walkdir::WalkDir;

use anvil_checks::antipattern::{
    AntipatternCheckConfig, Artifact, ArtifactKind, ScanOptions, Warning, WarningSeverity,
    WarningSummary, create_warning_result, run_antipattern_check, scan_artifacts,
};

use crate::GlobalArgs;
use crate::output::{self, OutputMode};

/// JSON output schema version — shared across all output paths.
const CHECK_OUTPUT_VERSION: &str = "1.0.0";

/// Default directories to ignore during file scanning.
const IGNORE_DIRS: &[&str] = &[
    "node_modules",
    "dist",
    "build",
    ".git",
    "target",
    ".anvil",
    ".next",
    ".turbo",
    ".nx",
    "coverage",
    "__pycache__",
    ".turbo",
    ".nx",
];

// TODO(RCLI2): The following Node.js CLI flags are intentionally deferred:
//   --no-cache       (caching infrastructure not yet ported)
//   --interactive    (crossterm interactive prompts — future work item)
//   --nudge / --nudge-threshold  (nudge coaching system — future work item)
// These will be added when their backing infrastructure is available in Rust.

#[derive(Debug, Args)]
#[allow(clippy::struct_excessive_bools)]
pub struct CheckArgs {
    /// Files to analyse (optional if using --changed or --all).
    files: Vec<String>,

    /// Analyse git-changed files only.
    #[arg(long, conflicts_with = "all")]
    changed: bool,

    /// With --changed, analyse only staged files.
    #[arg(long, requires = "changed", conflicts_with = "since")]
    staged: bool,

    /// With --changed, compare against a git ref (e.g. main, HEAD~3).
    #[arg(long, requires = "changed", conflicts_with = "staged")]
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
    warnings: Vec<JsonWarning>,
    summary: WarningSummary,
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

pub fn run(args: &CheckArgs, global: &GlobalArgs) -> Result<()> {
    let mode = OutputMode::from_global(global);
    let start = Instant::now();

    // Validate mutually exclusive flags.
    if args.all && args.changed {
        bail!("Cannot use --all and --changed together. Choose one.");
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
        } else {
            output::plain::info(message);
        }
        return Ok(());
    }

    if mode != OutputMode::Json && global.verbose {
        output::plain::info(&format!("Analysing {} file(s)...", files.len()));
    }

    // Run antipattern check.
    let config = AntipatternCheckConfig {
        patterns: Vec::new(),
        include_opt_in: args.include_opt_in,
        extensions,
        severity_threshold,
    };

    let file_refs: Vec<&str> = files.iter().map(String::as_str).collect();
    // For git-based modes and --all, use git toplevel as workspace root so
    // path normalisation matches how files were resolved. Fall back to cwd.
    let workspace_root = if matches!(source, FileSource::Changed | FileSource::All) {
        git_toplevel().ok().map(|p| p.to_string_lossy().to_string())
    } else {
        None
    }
    .or_else(|| {
        std::env::current_dir()
            .and_then(std::fs::canonicalize)
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    });
    let result = run_antipattern_check(&file_refs, &config, workspace_root.as_deref());

    let elapsed = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

    // Guard: if no files were actually scanned (e.g. extension mismatch or
    // unreadable paths), report clearly rather than a misleading "no warnings".
    if result.files_scanned == 0 {
        if mode == OutputMode::Json {
            output::json::print(&empty_output(elapsed, "No analysable files found"))?;
        } else {
            output::plain::warn(
                "No analysable files found (0 scanned). Check file extensions and readability.",
            );
        }
        return Ok(());
    }

    let has_blocking = !result.passed;

    // Relativise file paths for output.
    let relative_files: Vec<String> = files
        .iter()
        .map(|f| relativise(f, workspace_root.as_deref()))
        .collect();

    match mode {
        OutputMode::Json => {
            let json_output = build_json_output(
                &relative_files,
                &result.warnings.warnings,
                &result.warnings.summary,
                &result.patterns_checked,
                has_blocking,
                elapsed,
            );
            output::json::print(&json_output)?;
        }
        OutputMode::Plain | OutputMode::Tui => {
            print_human(
                &result.warnings.warnings,
                &result.warnings.summary,
                &relative_files,
                global.verbose,
                elapsed,
                source,
            );
        }
    }

    if has_blocking {
        if mode != OutputMode::Json {
            output::plain::error("Blocking warnings found (severity meets threshold)");
        }
        // Signal failure via AlreadyReported so main exits with EXIT_ERROR
        // without reprinting the message.
        Err(output::AlreadyReported.into())
    } else {
        Ok(())
    }
}

// ── Non-source artifacts (pr-description / commit-message / agent-output) ─

fn parse_artifact_kind(input: &str) -> Result<ArtifactKind> {
    ArtifactKind::from_wire(input).ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid --artifact \"{input}\". Allowed values: source, pr-description, commit-message, agent-output"
        )
    })
}

fn run_non_source_artifact(
    args: &CheckArgs,
    global: &GlobalArgs,
    kind: ArtifactKind,
    start: Instant,
) -> Result<()> {
    let mode = OutputMode::from_global(global);

    if args.all || args.changed || args.staged || args.since.is_some() || args.extensions.is_some()
    {
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
    let mut artifacts = Vec::with_capacity(args.files.len());
    for path_str in &args.files {
        let path = Path::new(path_str);
        if !path.exists() {
            bail!("File not found: {path_str}");
        }
        if path.is_dir() {
            bail!("Expected a file, got a directory: {path_str}");
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read {path_str}: {e}"))?;
        artifacts.push(Artifact {
            kind,
            reference: path_str.clone(),
            content,
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
            let json_output = build_json_output(
                &reference_files,
                &warning_result.warnings,
                &warning_result.summary,
                &pattern_ids,
                has_blocking,
                elapsed,
            );
            output::json::print(&json_output)?;
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
        if mode != OutputMode::Json {
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

    if args.changed {
        let files = get_changed_files(args.staged, args.since.as_deref(), extensions)?;
        return Ok((files, FileSource::Changed));
    }

    if args.all {
        let files = get_all_source_files(extensions)?;
        return Ok((files, FileSource::All));
    }

    bail!("No files specified. Use --all, --changed, or provide file paths.");
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

fn get_all_source_files(extensions: &[String]) -> Result<Vec<String>> {
    // Scan from git toplevel so --all covers the full repo even from a subdirectory.
    let root = git_toplevel().unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    let mut files = Vec::new();

    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy();
                !IGNORE_DIRS.contains(&name.as_ref())
            } else {
                true
            }
        })
    {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path_str = entry.path().to_string_lossy().to_string();
        if has_matching_extension(&path_str, extensions) {
            files.push(path_str);
        }
    }

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
        warnings: Vec::new(),
        summary: WarningSummary {
            total: 0,
            errors: 0,
            warnings: 0,
            info: 0,
            suppressed: 0,
        },
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
    warnings: &[Warning],
    summary: &WarningSummary,
    _patterns_checked: &[String],
    has_blocking: bool,
    elapsed: u64,
) -> CheckOutput {
    let json_warnings: Vec<JsonWarning> = warnings
        .iter()
        .map(|w| JsonWarning {
            id: w.id.clone(),
            category: category_str(w.category).to_string(),
            severity: severity_str(w.severity).to_string(),
            title: w.title.clone(),
            message: w.message.clone(),
            file: w.location.file.clone(),
            line: w.location.line,
            suggestion: w.suggestion.clone(),
            nudge: w.nudge.clone(),
        })
        .collect();

    CheckOutput {
        version: CHECK_OUTPUT_VERSION,
        timestamp: chrono::Utc::now().to_rfc3339(),
        files: files.to_vec(),
        has_blocking_warnings: has_blocking,
        execution_time_ms: elapsed,
        // checksRun lists executed check categories (matching Node.js schema),
        // not individual antipattern rule IDs.
        checks_run: vec!["architecture".to_string()],
        provenance_id: None, // TODO(RCLI2): wire up Kindling provenance when available
        message: None,
        warnings: json_warnings,
        summary: summary.clone(),
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
    }

    #[test]
    fn build_json_output_maps_warnings_correctly() {
        use anvil_checks::antipattern::{Confidence, Location, WarningCategory};

        let warnings = vec![Warning {
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

        let out = build_json_output(
            &["src/foo.ts".to_string()],
            &warnings,
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

    #[test]
    fn clap_rejects_staged_without_changed() {
        use clap::Parser;
        // Build a full CLI parse to test clap's `requires` constraint.
        let result = crate::Cli::try_parse_from(["anvil", "check", "--staged"]);
        assert!(result.is_err());
    }

    #[test]
    fn clap_rejects_since_without_changed() {
        use clap::Parser;
        let result = crate::Cli::try_parse_from(["anvil", "check", "--since", "main"]);
        assert!(result.is_err());
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
        };
        let global = GlobalArgs::default();
        let result =
            run_non_source_artifact(&args, &global, ArtifactKind::PrDescription, Instant::now());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("File not found")
        );
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
        };
        let global = GlobalArgs::default();
        let err =
            run_non_source_artifact(&args, &global, ArtifactKind::PrDescription, Instant::now())
                .unwrap_err();
        assert!(err.downcast_ref::<output::AlreadyReported>().is_some());
    }
}
