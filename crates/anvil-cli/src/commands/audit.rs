use std::collections::BTreeMap;
use std::path::Path;

use anvil_kernel_types::{
    Notification, NotificationClass, NotificationContext, NotificationPriority,
};
use anvil_tui::surfaces::audit::{
    AuditData, AuditIssue, AuditState, HistoricalScore, IssueSeverity,
};
use clap::Args;
use serde::Serialize;

use crate::GlobalArgs;
use crate::services::interactive_fix::{
    FixOutcome, apply_fix_request, is_auto_fixable_console_statement,
};
use crate::util::is_ignored_dir_name;

#[derive(Debug, Args)]
pub struct AuditArgs {
    /// Output format: auto (default), tui, plain, json, or sarif. `json` is the
    /// `--json` alias; `sarif` emits SARIF 2.1.0 and is never auto-selected.
    #[arg(long, value_enum)]
    format: Option<crate::output::Format>,
}

impl AuditArgs {
    /// True when `--format json|sarif` requests structured output, so the
    /// pre-dispatch auth gate emits a JSON envelope rather than human text.
    pub(crate) fn wants_structured_output(&self) -> bool {
        self.format
            .is_some_and(crate::output::Format::is_structured)
    }
}

pub fn run(args: &AuditArgs, global: &GlobalArgs) -> anyhow::Result<()> {
    use crate::output::OutputMode;

    let mode = OutputMode::from_command_format(args.format, global);

    let data = run_audit(Path::new("."));

    match mode {
        OutputMode::Json => print_json(&data)?,
        OutputMode::Sarif => crate::output::json::print(&build_audit_sarif(&data))?,
        OutputMode::Tui => {
            let mut state = AuditState::new(data);
            loop {
                state = crate::tui::run_surface(state)?;
                if let Some(request) = state.pending_fix.take() {
                    let selected = state.selected_item;
                    if matches!(
                        apply_fix_request(&request, None),
                        FixOutcome::Applied { .. }
                    ) {
                        state.data = collect_audit_data();
                        state.selected_item =
                            selected.min(state.data.issues.len().saturating_sub(1));
                        state.expanded = false;
                    }
                    continue;
                }
                break;
            }
        }
        OutputMode::Plain => print_plain(&data),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Data gathering
// ---------------------------------------------------------------------------

/// Source file extensions we scan for issues.
const SOURCE_EXTS: &[&str] = &["ts", "js", "rs", "py"];

/// Maximum line count before a file is flagged.
const MAX_FILE_LINES: usize = 500;

/// Collect audit data for the current directory (convenience for sub-surface use).
pub fn collect_audit_data() -> AuditData {
    run_audit(Path::new("."))
}

/// Scan the repository at `root` and return audit data.
///
/// SCAN-001: file discovery uses `ignore::WalkBuilder` configured with
/// `.standard_filters(false)` plus the shared ignored-directory prune list.
/// `.gitignore` is intentionally NOT applied — a security scan must see
/// every file regardless of VCS state — but `target/`, `node_modules/`,
/// and similar noise dirs are skipped via the explicit prune.
/// Per-file scans run on the rayon thread pool with `catch_unwind`
/// panic containment. Findings are then sorted into the deterministic
/// `(severity, file, line)` order so concurrent collection cannot leak
/// scheduling into user-visible output.
pub fn run_audit(root: &Path) -> AuditData {
    use rayon::prelude::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let project_name = root
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "unknown".to_string());

    // Phase 1: discover candidate files via the noise-pruning walker (skips target/, node_modules/, etc; not .gitignore).
    // `standard_filters(true)` honours `.gitignore`; we still prune
    // known local/generated/tool-state directories explicitly to keep
    // audit independent of user VCS ignore rules without scanning noise.
    let walker = ignore::WalkBuilder::new(root)
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .filter_entry(|e| {
            if e.file_type().is_some_and(|ft| ft.is_dir())
                && let Some(name) = e.file_name().to_str()
                && is_ignored_dir_name(name)
            {
                return false;
            }
            true
        })
        .build();

    let candidates: Vec<(std::path::PathBuf, String)> = walker
        .filter_map(Result::ok)
        .filter_map(|entry| {
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                return None;
            }
            let path = entry.path();
            let rel = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .into_owned();
            Some((path.to_path_buf(), rel))
        })
        .collect();

    let total_files = candidates.len();

    // Phase 2: scan each file in parallel. Each closure produces its own
    // `Vec<AuditIssue>` so workers never contend on a shared mutable
    // collection. `catch_unwind` keeps a panic in `scan_source_file`
    // (e.g. on malformed UTF-8 we didn't anticipate) from poisoning the
    // whole audit run.
    let per_file: Vec<Vec<AuditIssue>> = candidates
        .par_iter()
        .map(|(path, rel)| {
            let result = catch_unwind(AssertUnwindSafe(|| {
                let mut local: Vec<AuditIssue> = Vec::new();
                check_env_file(path, rel, &mut local);
                scan_source_file(path, rel, &mut local);
                local
            }));
            result.unwrap_or_default()
        })
        .collect();

    let mut issues: Vec<AuditIssue> = per_file.into_iter().flatten().collect();

    // Issue #1798: `anvil audit` previously ran only its architecture-pass
    // (env files + quality/documentation) and reported "0 issues" on a
    // repo whose source files held hardcoded secrets, while `anvil gate`
    // failed `secret-detection` over the same tree. Audit now runs the
    // canonical secret-detection check from `anvil_checks::secret` over
    // the same candidate set so its summary cannot disagree with gate
    // on hardcoded secrets.
    scan_for_hardcoded_secrets(&candidates, &mut issues);

    // Deterministic order: severity descending, then file ascending, line
    // ascending, message ascending — without this the rayon collect order
    // would leak thread scheduling into the user-facing audit output.
    issues.sort_by(|a, b| {
        issue_severity_rank(b.severity)
            .cmp(&issue_severity_rank(a.severity))
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.message.cmp(&b.message))
    });

    let historical_scores = load_historical_scores(root);
    let next_steps = generate_next_steps(&issues);

    AuditData {
        project_name,
        total_files,
        issues,
        historical_scores,
        next_steps,
    }
}

/// Severity ordering helper used to sort the parallel-collected audit
/// issues into a deterministic, user-visible order.
const fn issue_severity_rank(severity: IssueSeverity) -> u8 {
    match severity {
        IssueSeverity::Critical => 5,
        IssueSeverity::High => 4,
        IssueSeverity::Medium => 3,
        IssueSeverity::Low => 2,
        IssueSeverity::Info => 1,
    }
}

/// Suffixes that mark a `.env` file as a committed *template* — its
/// presence alone is not a leak signal because by convention these
/// files contain placeholder values (e.g. `.env.example`,
/// `.env.local.example`, `.env.sample`, `.env.template`, `.env.dist`).
const ENV_TEMPLATE_SUFFIXES: &[&str] = &[".example", ".sample", ".template", ".dist"];

fn is_env_template_filename(name: &str) -> bool {
    // `check_env_file` only invokes this helper for filenames that
    // satisfy `is_env` (literal `.env` or `.env.*`), so `.envrc` is
    // already filtered out at the caller — no early-return needed.
    if name == ".env" {
        return false;
    }
    ENV_TEMPLATE_SUFFIXES
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

/// Flag `.env` files as potential secret leaks. Excludes committed
/// template files (`.env.example`, `.env.local.example`, etc.). Real
/// `.env` files are reported even under fixtures or runner directories:
/// audit is the broad, security-first surface and should not hide local
/// secret stores based on path alone.
fn check_env_file(path: &Path, rel: &str, issues: &mut Vec<AuditIssue>) {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    let is_env = name == ".env" || name.starts_with(".env.");
    if !is_env {
        return;
    }
    if is_env_template_filename(name) {
        return;
    }
    issues.push(AuditIssue {
        severity: IssueSeverity::High,
        category: "Security".to_string(),
        message: "Environment file may contain secrets".to_string(),
        file: rel.to_string(),
        line: 0,
        fixable: false,
    });
}

/// File extensions scanned for hardcoded secrets — kept in lock-step with the
/// `matches!` arm in `gate::run_check_secret` (`crates/anvil-cli/src/commands/gate.rs`).
/// Audit and gate must scan the same file set or they will disagree in the
/// opposite direction (audit flagging a file gate ignores), reintroducing the
/// confusion that issue #1798 is fixing. If gate's extension list changes,
/// update both lists together (or extract a shared helper).
const SECRET_SCAN_EXTS: &[&str] = &["ts", "js", "rs", "json", "yaml", "yml", "toml", "env"];

/// Map [`anvil_checks::secret`] findings discovered over `candidates` into
/// `Security`-category `AuditIssue` entries. Each finding becomes its own
/// `IssueSeverity::High` entry — matching the user-facing severity that
/// `anvil gate -p ai` shows for the same finding — so audit and gate cannot
/// disagree about the presence of hardcoded secrets.
///
/// Paths are reported using audit's own `rel` (the `strip_prefix(root)`
/// result already collected during file discovery) so that every issue in
/// `AuditData.issues` shares one path format. Reusing the candidates'
/// pre-computed `rel` also keeps the leading-slash / separator quirks of
/// `anvil_checks::secret::normalise_file_path` from leaking into audit
/// output across the secret/non-secret boundary.
fn scan_for_hardcoded_secrets(
    candidates: &[(std::path::PathBuf, String)],
    issues: &mut Vec<AuditIssue>,
) {
    if candidates.is_empty() {
        return;
    }

    // Restrict the secret scan to file types gate's `secret-detection` check
    // already covers; binary assets and lockfiles are also skipped by
    // `anvil_checks::secret::run_secret_check`, but filtering here keeps the
    // scanner call small on trees that still contain generated artefacts the
    // audit walker did not prune (e.g. SVGs).
    let scannable: Vec<(String, &str)> = candidates
        .iter()
        .filter(|(path, _)| {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // Dotfile-prefixed env files (`.env`, `.env.local`, …) match
            // by filename, source files match by extension.
            if name.starts_with(".env") {
                return true;
            }
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| SECRET_SCAN_EXTS.contains(&ext))
        })
        .map(|(path, rel)| (path.to_string_lossy().into_owned(), rel.as_str()))
        .collect();

    if scannable.is_empty() {
        return;
    }

    // Index abs-path → rel so each finding can be mapped back to the same
    // `rel` audit's other passes use. `run_secret_check` is invoked WITHOUT a
    // workspace root so `finding.file` comes back as the absolute path we
    // passed in (no leading `/`, no forced slash conversion).
    let rel_by_abs: std::collections::HashMap<&str, &str> = scannable
        .iter()
        .map(|(abs, rel)| (abs.as_str(), *rel))
        .collect();

    let file_refs: Vec<&str> = scannable.iter().map(|(abs, _)| abs.as_str()).collect();
    let config = anvil_checks::secret::SecretCheckConfig::default();
    let result = anvil_checks::secret::run_secret_check(&file_refs, &config, None);

    for finding in result.findings {
        // Fall back to the scanner's own path if the lookup misses (it
        // shouldn't — every scanned file came from `scannable` — but a
        // surprise upstream change should not drop a finding silently).
        let file = rel_by_abs
            .get(finding.file.as_str())
            .map_or_else(|| finding.file.clone(), |rel| (*rel).to_string());
        issues.push(AuditIssue {
            severity: IssueSeverity::High,
            category: "Security".to_string(),
            message: format!(
                "Potential hardcoded secret: {} (run `anvil gate` for the full secret-detection report)",
                finding.pattern_name,
            ),
            file,
            line: finding.line,
            fixable: false,
        });
    }
}

/// Scan a single source file for quality and documentation issues.
fn scan_source_file(path: &Path, rel: &str, issues: &mut Vec<AuditIssue>) {
    let is_source = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| SOURCE_EXTS.contains(&ext));

    if !is_source {
        return;
    }

    let Ok(contents) = std::fs::read_to_string(path) else {
        return;
    };

    let lines: Vec<&str> = contents.lines().collect();

    if lines.len() > MAX_FILE_LINES {
        issues.push(AuditIssue {
            severity: IssueSeverity::Medium,
            category: "Quality".to_string(),
            message: format!("File has {} lines (>{MAX_FILE_LINES})", lines.len()),
            file: rel.to_string(),
            line: lines.len(),
            fixable: false,
        });
    }

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        scan_line(ext, trimmed, line_num + 1, rel, issues);
    }
}

/// Check a single line for console statements and marker comments.
fn scan_line(ext: &str, trimmed: &str, line_num: usize, rel: &str, issues: &mut Vec<AuditIssue>) {
    if (ext == "ts" || ext == "js")
        && (trimmed.contains("console.log") || trimmed.contains("console.error"))
    {
        let fixable = is_auto_fixable_console_statement(trimmed);
        issues.push(AuditIssue {
            severity: IssueSeverity::Low,
            category: "Quality".to_string(),
            message: "console statement found".to_string(),
            file: rel.to_string(),
            line: line_num,
            fixable,
        });
    }

    if contains_marker(trimmed) {
        let marker = if trimmed.contains("TODO") {
            "TODO"
        } else if trimmed.contains("FIXME") {
            "FIXME"
        } else {
            "HACK"
        };
        issues.push(AuditIssue {
            severity: IssueSeverity::Info,
            category: "Documentation".to_string(),
            message: format!("{marker} comment"),
            file: rel.to_string(),
            line: line_num,
            fixable: false,
        });
    }
}

/// Check if a line contains a TODO, FIXME, or HACK marker in a comment context.
fn contains_marker(line: &str) -> bool {
    // Only match when the marker appears in a comment-like context.
    let has_marker = line.contains("TODO") || line.contains("FIXME") || line.contains("HACK");
    if !has_marker {
        return false;
    }
    // Simple heuristic: line contains a comment prefix.
    line.contains("//") || line.contains('#') || line.starts_with("/*") || line.contains("* ")
}

/// Load up to 4 historical score entries from the cache index.
fn load_historical_scores(root: &Path) -> Vec<HistoricalScore> {
    let index_path = root.join(".anvil/cache/index.json");

    let Ok(contents) = std::fs::read_to_string(&index_path) else {
        return vec![];
    };

    let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return vec![];
    };

    let Some(entries) = value.get("entries").and_then(serde_json::Value::as_object) else {
        return vec![];
    };

    let mut scores: Vec<HistoricalScore> = entries
        .iter()
        .filter_map(|(key, val)| {
            let ts_str = key.rsplit(':').next()?;
            let ts: i64 = ts_str.parse().ok()?;
            let timestamp = format_unix_timestamp(ts);
            let score = val.get("score").and_then(serde_json::Value::as_f64)?;
            #[allow(clippy::cast_possible_truncation)]
            let issue_count = val
                .get("issueCount")
                .or_else(|| val.get("checksRun"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as usize;
            Some(HistoricalScore {
                timestamp,
                score,
                issue_count,
            })
        })
        .collect();

    scores.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    scores.truncate(4);
    scores
}

/// Format a Unix timestamp as `YYYY-MM-DD HH:MM` (UTC, no external crate).
fn format_unix_timestamp(secs: i64) -> String {
    let days_since_epoch = secs.div_euclid(86400);
    let time_of_day = secs.rem_euclid(86400);

    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;

    let z = days_since_epoch + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02} {hours:02}:{minutes:02}")
}

/// Generate actionable next steps from the issue list.
fn generate_next_steps(issues: &[AuditIssue]) -> Vec<String> {
    let mut steps = Vec::new();

    let high_count = issues
        .iter()
        .filter(|i| matches!(i.severity, IssueSeverity::High | IssueSeverity::Critical))
        .count();
    if high_count > 0 {
        steps.push(format!(
            "Address {high_count} high/critical severity issue(s) first"
        ));
    }

    let console_count = issues
        .iter()
        .filter(|i| i.message.contains("console statement"))
        .count();
    if console_count > 0 {
        steps.push(format!(
            "Remove {console_count} console statement(s) from source files"
        ));
    }

    let large_count = issues
        .iter()
        .filter(|i| i.message.starts_with("File has"))
        .count();
    if large_count > 0 {
        steps.push(format!(
            "Consider splitting {large_count} large file(s) (>{MAX_FILE_LINES} lines)"
        ));
    }

    let todo_count = issues
        .iter()
        .filter(|i| i.category == "Documentation")
        .count();
    if todo_count > 0 {
        steps.push(format!("Review {todo_count} TODO/FIXME/HACK comment(s)"));
    }

    if steps.is_empty() {
        steps.push("No issues found — project looks clean!".to_string());
    }

    steps
}

// ---------------------------------------------------------------------------
// Output: plain text
// ---------------------------------------------------------------------------

fn print_plain(data: &AuditData) {
    println!("ANVIL AUDIT — {}\n", data.project_name);
    println!("Total files scanned: {}", data.total_files);
    println!("Issues found: {}\n", data.issues.len());

    if !data.issues.is_empty() {
        println!("ISSUES");
        for issue in &data.issues {
            let badge = match issue.severity {
                IssueSeverity::Critical => "[CRIT]",
                IssueSeverity::High => "[HIGH]",
                IssueSeverity::Medium => "[MED] ",
                IssueSeverity::Low => "[LOW] ",
                IssueSeverity::Info => "[INFO]",
            };
            let fix_tag = if issue.fixable { " (fixable)" } else { "" };
            println!(
                "  {badge} {:<15} {}:{}{fix_tag}",
                issue.category, issue.file, issue.line,
            );
            println!("        {}", issue.message);
        }
    }

    if !data.historical_scores.is_empty() {
        println!("\nHISTORICAL SCORES");
        for score in &data.historical_scores {
            println!(
                "  {}  score: {:.2}  issues: {}",
                score.timestamp, score.score, score.issue_count,
            );
        }
    }

    if !data.next_steps.is_empty() {
        println!("\nNEXT STEPS");
        for (i, step) in data.next_steps.iter().enumerate() {
            println!("  {}. {step}", i + 1);
        }
    }
}

// ---------------------------------------------------------------------------
// Output: JSON
// ---------------------------------------------------------------------------

/// `issues[]` is the canonical list of audit findings — always emitted in full.
/// `notifications[]` is a taxonomy-aligned envelope (`class`, `priority`,
/// `title`, `message`, `context`) for subscribers that consume the shared
/// notification model across `check`, `gate`, `doctor`, and `audit`. When the
/// two overlap, `issues[]` is authoritative: `notifications[]` mirrors the
/// highest-priority findings (capped — see `MAX_ISSUE_NOTIFICATIONS`) plus a
/// single summary. Consumers that want the full list should read `issues[]`.
#[derive(Serialize)]
struct AuditOutput {
    project_name: String,
    total_files: usize,
    issues: Vec<IssueOutput>,
    historical_scores: Vec<ScoreOutput>,
    next_steps: Vec<String>,
    notifications: Vec<Notification>,
}

#[derive(Serialize)]
struct IssueOutput {
    severity: String,
    category: String,
    message: String,
    file: String,
    line: usize,
    fixable: bool,
}

#[derive(Serialize)]
struct ScoreOutput {
    timestamp: String,
    score: f64,
    issue_count: usize,
}

/// Cap on per-issue notifications mirrored into `AuditOutput.notifications`.
///
/// Large monorepos produce thousands of `TODO`/console findings; without a cap
/// each `--json` invocation would allocate one `Notification` per issue plus
/// the full pretty-JSON buffer (OPS-002). `issues[]` remains the canonical
/// unbounded list; `notifications[]` is capped to the highest-priority entries
/// with a single overflow notification announcing truncation.
const MAX_ISSUE_NOTIFICATIONS: usize = 500;

fn notification_priority_for_severity(severity: IssueSeverity) -> NotificationPriority {
    // NOTE: `Critical` is reserved by the taxonomy for control-plane events
    // (block / interrupt / fence-state). Audit findings — including critical
    // severity — map to `High` so that a future control-lane `critical`
    // notification stays distinguishable from a high-severity finding.
    match severity {
        IssueSeverity::Critical | IssueSeverity::High => NotificationPriority::High,
        IssueSeverity::Medium => NotificationPriority::Normal,
        IssueSeverity::Low | IssueSeverity::Info => NotificationPriority::Low,
    }
}

fn notification_for_issue(issue: &AuditIssue) -> Notification {
    Notification::new(
        NotificationClass::Finding,
        notification_priority_for_severity(issue.severity),
        format!("[{}] {}", issue.severity.label_full(), issue.category),
        issue.message.clone(),
    )
    .with_context(NotificationContext {
        file: Some(issue.file.clone()),
        source: Some("audit".to_string()),
    })
}

fn severity_rank(severity: IssueSeverity) -> u8 {
    // Lower value = higher priority for the cap selector below.
    match severity {
        IssueSeverity::Critical => 0,
        IssueSeverity::High => 1,
        IssueSeverity::Medium => 2,
        IssueSeverity::Low => 3,
        IssueSeverity::Info => 4,
    }
}

fn notifications_for_audit(data: &AuditData) -> Vec<Notification> {
    let audit_context = NotificationContext {
        file: None,
        source: Some("audit".to_string()),
    };

    // Pick the top-N highest-priority issues, preserving original order within
    // each severity bucket so tests remain stable.
    let mut indexed: Vec<(usize, &AuditIssue)> = data.issues.iter().enumerate().collect();
    indexed.sort_by_key(|(idx, issue)| (severity_rank(issue.severity), *idx));
    let total_issues = indexed.len();
    let truncated = total_issues.saturating_sub(MAX_ISSUE_NOTIFICATIONS);

    let mut notifications: Vec<Notification> = indexed
        .into_iter()
        .take(MAX_ISSUE_NOTIFICATIONS)
        .map(|(_, issue)| notification_for_issue(issue))
        .collect();

    if truncated > 0 {
        notifications.push(
            Notification::new(
                NotificationClass::Info,
                NotificationPriority::Normal,
                "Audit notifications truncated",
                format!(
                    "Emitted {MAX_ISSUE_NOTIFICATIONS} of {total_issues} findings as notifications; see issues[] for the full list.",
                ),
            )
            .with_context(audit_context.clone()),
        );
    }

    let critical = data.issue_count_by_severity(IssueSeverity::Critical);
    let high = data.issue_count_by_severity(IssueSeverity::High);
    let medium = data.issue_count_by_severity(IssueSeverity::Medium);

    // Summary class follows finding severity. Priority is capped at `High` —
    // `Critical` is reserved for control-plane notifications.
    let (class, priority, message) = if critical > 0 {
        (
            NotificationClass::Failure,
            NotificationPriority::High,
            format!(
                "{critical} critical, {high} high, {medium} medium, {} total",
                data.issues.len()
            ),
        )
    } else if high > 0 {
        (
            NotificationClass::Warning,
            NotificationPriority::High,
            format!(
                "0 critical, {high} high, {medium} medium, {} total",
                data.issues.len()
            ),
        )
    } else if data.issues.is_empty() {
        (
            NotificationClass::Info,
            NotificationPriority::Low,
            format!("No issues across {} files", data.total_files),
        )
    } else {
        (
            NotificationClass::Warning,
            NotificationPriority::Normal,
            format!(
                "0 critical, 0 high, {medium} medium, {} total",
                data.issues.len()
            ),
        )
    };

    notifications.push(
        Notification::new(class, priority, "Audit summary", message).with_context(audit_context),
    );

    notifications
}

fn build_audit_output(data: &AuditData) -> AuditOutput {
    AuditOutput {
        project_name: data.project_name.clone(),
        total_files: data.total_files,
        issues: data
            .issues
            .iter()
            .map(|i| IssueOutput {
                severity: i.severity.label().to_string(),
                category: i.category.clone(),
                message: i.message.clone(),
                file: i.file.clone(),
                line: i.line,
                fixable: i.fixable,
            })
            .collect(),
        historical_scores: data
            .historical_scores
            .iter()
            .map(|s| ScoreOutput {
                timestamp: s.timestamp.clone(),
                score: s.score,
                issue_count: s.issue_count,
            })
            .collect(),
        next_steps: data.next_steps.clone(),
        notifications: notifications_for_audit(data),
    }
}

// ── SARIF adapter (SARIFOUT-004) ────────────────────────────────────

/// Map an audit `IssueSeverity` onto a SARIF `level`.
fn audit_sarif_level(severity: IssueSeverity) -> crate::output::sarif::Level {
    use crate::output::sarif::Level;
    match severity {
        IssueSeverity::Critical | IssueSeverity::High => Level::Error,
        IssueSeverity::Medium => Level::Warning,
        IssueSeverity::Low | IssueSeverity::Info => Level::Note,
    }
}

/// Build a SARIF document from audit findings. Each issue maps to one
/// `results[]` entry (`category` → `ruleId`, severity → `level`, `file`/`line`
/// → `locations[].physicalLocation.region`); the result set matches the JSON
/// output's `issues[]`. Audit has no suppression model, so no `suppressions[]`.
fn build_audit_sarif(data: &AuditData) -> crate::output::sarif::SarifLog {
    use crate::output::sarif;

    let mut rules: BTreeMap<String, sarif::ReportingDescriptor> = BTreeMap::new();
    let mut results = Vec::with_capacity(data.issues.len());
    for issue in &data.issues {
        rules
            .entry(issue.category.clone())
            .or_insert_with(|| sarif::ReportingDescriptor::new(issue.category.clone()));
        // Audit uses `line: 0` for whole-file findings (e.g. `.env` files);
        // SARIF `startLine` has `minimum: 1`, so omit the region in that case
        // and point at the artifact only.
        let line = (issue.line > 0).then(|| u32::try_from(issue.line).unwrap_or(u32::MAX));
        let region = line.map(sarif::Region::line);
        results.push(
            sarif::SarifResult::new(
                issue.category.clone(),
                audit_sarif_level(issue.severity),
                issue.message.clone(),
            )
            .location(sarif::Location::new(issue.file.clone(), region))
            .fingerprint(
                "anvilFingerprint/v1",
                sarif::stable_fingerprint(&issue.category, &issue.file, line, &issue.message),
            ),
        );
    }
    sarif::SarifLog::new(sarif::Run::new(rules.into_values().collect(), results))
}

fn print_json(data: &AuditData) -> anyhow::Result<()> {
    let output = build_audit_output(data);
    let json = serde_json::to_string_pretty(&output)?;
    println!("{json}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn make_temp_dir() -> std::path::PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("anvil-audit-test-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn empty_dir_produces_zero_issues() {
        let dir = make_temp_dir();
        let data = run_audit(&dir);
        assert_eq!(data.issues.len(), 0);
        assert_eq!(data.total_files, 0);
        cleanup(&dir);
    }

    #[test]
    fn detects_console_log_in_ts() {
        let dir = make_temp_dir();
        let ts_file = dir.join("example.ts");
        std::fs::write(&ts_file, "const x = 1;\nconsole.log(x);\n").unwrap();

        let data = run_audit(&dir);
        let console_issues: Vec<_> = data
            .issues
            .iter()
            .filter(|i| i.message.contains("console statement"))
            .collect();
        assert_eq!(console_issues.len(), 1);
        assert!(matches!(console_issues[0].severity, IssueSeverity::Low));
        assert_eq!(console_issues[0].line, 2);
        cleanup(&dir);
    }

    #[test]
    fn skips_git_and_node_modules() {
        let dir = make_temp_dir();
        std::fs::create_dir_all(dir.join(".git/objects")).unwrap();
        std::fs::write(dir.join(".git/objects/test.ts"), "console.log('hi');\n").unwrap();

        std::fs::create_dir_all(dir.join("node_modules/pkg")).unwrap();
        std::fs::write(
            dir.join("node_modules/pkg/index.js"),
            "console.log('dep');\n",
        )
        .unwrap();

        // A real source file that should be counted.
        std::fs::write(dir.join("app.ts"), "const y = 2;\n").unwrap();

        let data = run_audit(&dir);
        // Only app.ts should be counted.
        assert_eq!(data.total_files, 1);
        // No issues from skipped dirs.
        assert!(
            data.issues
                .iter()
                .all(|i| !i.file.contains(".git") && !i.file.contains("node_modules"))
        );
        cleanup(&dir);
    }

    #[test]
    fn skips_generated_and_agent_worktree_dirs() {
        let dir = make_temp_dir();
        std::fs::create_dir_all(dir.join("dist")).unwrap();
        std::fs::write(dir.join("dist/index.js"), "console.log('built');\n").unwrap();

        std::fs::create_dir_all(dir.join(".nx/cache")).unwrap();
        std::fs::write(dir.join(".nx/cache/prettify.js"), "console.log('cache');\n").unwrap();

        std::fs::create_dir_all(dir.join(".claude/worktrees/agent-a/apps/web")).unwrap();
        std::fs::write(
            dir.join(".claude/worktrees/agent-a/apps/web/.env.local"),
            "SECRET=abc123\n",
        )
        .unwrap();

        std::fs::write(dir.join("app.ts"), "const y = 2;\n").unwrap();

        let data = run_audit(&dir);
        assert_eq!(data.total_files, 1);
        assert!(data.issues.iter().all(|i| {
            !i.file.contains("dist")
                && !i.file.contains(".nx")
                && !i.file.contains(".claude/worktrees")
        }));
        cleanup(&dir);
    }

    #[test]
    fn detects_todo_comment() {
        let dir = make_temp_dir();
        std::fs::write(
            dir.join("lib.rs"),
            "// TODO: fix this later\nfn main() {}\n",
        )
        .unwrap();

        let data = run_audit(&dir);
        let todo_issues: Vec<_> = data
            .issues
            .iter()
            .filter(|i| i.message.contains("TODO"))
            .collect();
        assert_eq!(todo_issues.len(), 1);
        assert!(matches!(todo_issues[0].severity, IssueSeverity::Info));
        assert_eq!(todo_issues[0].category, "Documentation");
        cleanup(&dir);
    }

    #[test]
    fn detects_env_file() {
        let dir = make_temp_dir();
        std::fs::write(dir.join(".env"), "SECRET=abc123\n").unwrap();
        std::fs::write(dir.join(".env.example"), "SECRET=\n").unwrap();

        let data = run_audit(&dir);
        let env_issues: Vec<_> = data
            .issues
            .iter()
            .filter(|i| i.category == "Security")
            .collect();
        // .env should be flagged, .env.example should not.
        assert_eq!(env_issues.len(), 1);
        assert!(matches!(env_issues[0].severity, IssueSeverity::High));
        cleanup(&dir);
    }

    /// Issue #1798 — `anvil audit` previously reported "0 issues" on a
    /// repo whose source file held a hardcoded GitHub token, while
    /// `anvil gate` flagged the same file via `secret-detection`. Audit
    /// must surface hardcoded secrets in source files so its summary
    /// cannot disagree with gate on the canonical secret patterns.
    #[test]
    fn detects_hardcoded_secret_in_source_file() {
        let dir = make_temp_dir();
        let src = dir.join("src");
        std::fs::create_dir_all(&src).unwrap();
        // `ghp_…{40}` GitHub personal access token — matches the built-in
        // GitHub Token pattern and contains no allowlist tokens
        // (`example` / `test` / `sample`) that would otherwise be
        // filtered by the secret scanner.
        let token = format!("ghp_{}", "a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0");
        std::fs::write(
            src.join("smelly.ts"),
            format!("export const GH_TOKEN = \"{token}\";\n"),
        )
        .unwrap();

        let data = run_audit(&dir);
        let secret_issues: Vec<_> = data
            .issues
            .iter()
            .filter(|i| i.category == "Security" && i.file.ends_with("smelly.ts"))
            .collect();
        assert!(
            !secret_issues.is_empty(),
            "audit must surface hardcoded secrets in source files (got: {:?})",
            data.issues,
        );
        assert!(matches!(secret_issues[0].severity, IssueSeverity::High));
        // Secret-finding paths must share audit's existing `rel` format
        // (no leading `/`, same separator policy as the env/source passes)
        // so audit's print/sort logic does not mix two path styles.
        assert!(
            !secret_issues[0].file.starts_with('/'),
            "secret finding path must be repo-relative without a leading `/`, got `{}`",
            secret_issues[0].file,
        );
        cleanup(&dir);
    }

    #[test]
    fn detects_large_file() {
        let dir = make_temp_dir();
        let content = "fn noop() {}\n".repeat(501);
        std::fs::write(dir.join("big.rs"), content).unwrap();

        let data = run_audit(&dir);
        let large_issues: Vec<_> = data
            .issues
            .iter()
            .filter(|i| i.message.starts_with("File has"))
            .collect();
        assert_eq!(large_issues.len(), 1);
        assert!(matches!(large_issues[0].severity, IssueSeverity::Medium));
        cleanup(&dir);
    }

    #[test]
    fn next_steps_generated_from_issues() {
        let dir = make_temp_dir();
        std::fs::write(dir.join(".env"), "KEY=val\n").unwrap();
        std::fs::write(dir.join("app.ts"), "console.log('x');\n").unwrap();

        let data = run_audit(&dir);
        assert!(!data.next_steps.is_empty());
        assert!(data.next_steps.iter().any(|s| s.contains("high/critical")));
        assert!(data.next_steps.iter().any(|s| s.contains("console")));
        cleanup(&dir);
    }

    #[test]
    fn clean_project_gets_positive_next_step() {
        let dir = make_temp_dir();
        std::fs::write(dir.join("clean.rs"), "fn main() {}\n").unwrap();

        let data = run_audit(&dir);
        assert_eq!(data.next_steps.len(), 1);
        assert!(data.next_steps[0].contains("clean"));
        cleanup(&dir);
    }

    #[test]
    fn historical_scores_from_cache() {
        let dir = make_temp_dir();
        let cache_dir = dir.join(".anvil/cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(
            cache_dir.join("index.json"),
            r#"{
                "entries": {
                    "gate:f.md:1710000000": {"score": 0.9, "issueCount": 3},
                    "gate:f.md:1709990000": {"score": 0.8, "issueCount": 5}
                }
            }"#,
        )
        .unwrap();

        let data = run_audit(&dir);
        assert_eq!(data.historical_scores.len(), 2);
        cleanup(&dir);
    }

    #[test]
    fn no_cache_yields_empty_historical() {
        let dir = make_temp_dir();
        let data = run_audit(&dir);
        assert!(data.historical_scores.is_empty());
        cleanup(&dir);
    }

    // --- contains_marker ---

    #[test]
    fn contains_marker_todo_in_comment() {
        assert!(contains_marker("// TODO: fix later"));
    }

    #[test]
    fn contains_marker_fixme_in_comment() {
        assert!(contains_marker("// FIXME: broken"));
    }

    #[test]
    fn contains_marker_hack_in_comment() {
        assert!(contains_marker("// HACK: workaround"));
    }

    #[test]
    fn contains_marker_hash_comment() {
        assert!(contains_marker("# TODO: python style"));
    }

    #[test]
    fn contains_marker_block_comment() {
        assert!(contains_marker("/* TODO: block */"));
    }

    #[test]
    fn contains_marker_jsdoc_style() {
        assert!(contains_marker("* TODO: inside jsdoc"));
    }

    #[test]
    fn contains_marker_no_comment_context() {
        assert!(!contains_marker("const TODO = 'not a comment';"));
    }

    #[test]
    fn contains_marker_no_marker() {
        assert!(!contains_marker("// just a normal comment"));
    }

    #[test]
    fn contains_marker_todo_in_string_literal() {
        // The heuristic flags this as a false positive because it sees both
        // "TODO" and "//" in the line, even though the marker is inside a
        // string literal rather than an actual comment.
        assert!(contains_marker(r#"console.log("// TODO done")"#));
    }

    // --- scan_line ---

    #[test]
    fn scan_line_detects_console_log() {
        let mut issues = Vec::new();
        scan_line("ts", "console.log('hello');", 1, "app.ts", &mut issues);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("console statement"));
        assert!(issues[0].fixable);
    }

    #[test]
    fn scan_line_detects_console_error() {
        let mut issues = Vec::new();
        scan_line("js", "console.error('fail');", 5, "app.js", &mut issues);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("console statement"));
    }

    #[test]
    fn scan_line_ignores_console_in_rust() {
        let mut issues = Vec::new();
        scan_line("rs", "console.log('not js');", 1, "lib.rs", &mut issues);
        assert!(issues.is_empty());
    }

    #[test]
    fn scan_line_detects_todo_marker() {
        let mut issues = Vec::new();
        scan_line("rs", "// TODO: implement", 10, "lib.rs", &mut issues);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].category, "Documentation");
        assert!(issues[0].message.contains("TODO"));
    }

    #[test]
    fn scan_line_detects_fixme_marker() {
        let mut issues = Vec::new();
        scan_line("ts", "// FIXME: broken", 3, "app.ts", &mut issues);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("FIXME"));
    }

    #[test]
    fn scan_line_console_and_marker_same_line() {
        let mut issues = Vec::new();
        scan_line(
            "ts",
            "console.log('x'); // TODO: remove",
            1,
            "app.ts",
            &mut issues,
        );
        assert_eq!(issues.len(), 2);
    }

    #[test]
    fn scan_line_clean_line_no_issues() {
        let mut issues = Vec::new();
        scan_line("ts", "const x = 1;", 1, "app.ts", &mut issues);
        assert!(issues.is_empty());
    }

    // --- check_env_file ---

    #[test]
    fn check_env_flags_dotenv() {
        let dir = make_temp_dir();
        let path = dir.join(".env");
        std::fs::write(&path, "").unwrap();
        let mut issues = Vec::new();
        check_env_file(&path, ".env", &mut issues);
        assert_eq!(issues.len(), 1);
        assert!(matches!(issues[0].severity, IssueSeverity::High));
        cleanup(&dir);
    }

    #[test]
    fn check_env_flags_dotenv_local() {
        let dir = make_temp_dir();
        let path = dir.join(".env.local");
        std::fs::write(&path, "").unwrap();
        let mut issues = Vec::new();
        check_env_file(&path, ".env.local", &mut issues);
        assert_eq!(issues.len(), 1);
        cleanup(&dir);
    }

    #[test]
    fn check_env_flags_dotenv_production() {
        let dir = make_temp_dir();
        let path = dir.join(".env.production");
        std::fs::write(&path, "").unwrap();
        let mut issues = Vec::new();
        check_env_file(&path, ".env.production", &mut issues);
        assert_eq!(issues.len(), 1);
        cleanup(&dir);
    }

    #[test]
    fn check_env_skips_example() {
        let dir = make_temp_dir();
        let path = dir.join(".env.example");
        std::fs::write(&path, "").unwrap();
        let mut issues = Vec::new();
        check_env_file(&path, ".env.example", &mut issues);
        assert!(issues.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn check_env_skips_non_env() {
        let dir = make_temp_dir();
        let path = dir.join("config.toml");
        std::fs::write(&path, "").unwrap();
        let mut issues = Vec::new();
        check_env_file(&path, "config.toml", &mut issues);
        assert!(issues.is_empty());
        cleanup(&dir);
    }

    // v0.5.0 audit FPs — committed templates beyond `.env.example` and
    // test/vendored locations were flagged as "may contain secrets".

    #[test]
    fn check_env_skips_dotted_example_template() {
        let dir = make_temp_dir();
        let path = dir.join(".env.local.example");
        std::fs::write(&path, "").unwrap();
        let mut issues = Vec::new();
        check_env_file(&path, ".env.local.example", &mut issues);
        assert!(issues.is_empty(), "got: {issues:?}");
        cleanup(&dir);
    }

    #[test]
    fn check_env_skips_other_template_suffixes() {
        let dir = make_temp_dir();
        for name in [".env.sample", ".env.template", ".env.dist"] {
            let path = dir.join(name);
            std::fs::write(&path, "").unwrap();
            let mut issues = Vec::new();
            check_env_file(&path, name, &mut issues);
            assert!(
                issues.is_empty(),
                "{name} should be excluded, got: {issues:?}"
            );
        }
        cleanup(&dir);
    }

    #[test]
    fn check_env_flags_test_fixtures() {
        let dir = make_temp_dir();
        let path = dir.join(".env");
        std::fs::write(&path, "").unwrap();
        let mut issues = Vec::new();
        check_env_file(
            &path,
            "crates/anvil-checks/tests/fixtures/surfenv/aws-key.env",
            &mut issues,
        );
        assert_eq!(issues.len(), 1, "test fixture .env should still be audited");
        cleanup(&dir);
    }

    #[test]
    fn check_env_flags_actions_runner_dir() {
        let dir = make_temp_dir();
        let path = dir.join(".env");
        std::fs::write(&path, "").unwrap();
        let mut issues = Vec::new();
        check_env_file(&path, ".github/actions-runner/.env", &mut issues);
        assert_eq!(
            issues.len(),
            1,
            "actions-runner .env should still be audited"
        );
        cleanup(&dir);
    }

    #[test]
    fn check_env_still_flags_real_local() {
        // Regression guard: a normal `.env.local` outside excluded
        // paths must still fire — that's the original threat model.
        let dir = make_temp_dir();
        let path = dir.join(".env.local");
        std::fs::write(&path, "").unwrap();
        let mut issues = Vec::new();
        check_env_file(&path, "apps/website/.env.local", &mut issues);
        assert_eq!(issues.len(), 1, "real .env.local must still fire");
        cleanup(&dir);
    }

    // --- generate_next_steps ---

    #[test]
    fn next_steps_empty_issues() {
        let steps = generate_next_steps(&[]);
        assert_eq!(steps.len(), 1);
        assert!(steps[0].contains("clean"));
    }

    #[test]
    fn next_steps_high_severity_only() {
        let issues = vec![AuditIssue {
            severity: IssueSeverity::High,
            category: "Security".to_string(),
            message: "env file leak".to_string(),
            file: ".env".to_string(),
            line: 0,
            fixable: false,
        }];
        let steps = generate_next_steps(&issues);
        assert!(steps.iter().any(|s| s.contains("high/critical")));
        assert!(!steps.iter().any(|s| s.contains("console")));
    }

    #[test]
    fn next_steps_console_only() {
        let issues = vec![AuditIssue {
            severity: IssueSeverity::Low,
            category: "Quality".to_string(),
            message: "console statement found".to_string(),
            file: "app.ts".to_string(),
            line: 1,
            fixable: true,
        }];
        let steps = generate_next_steps(&issues);
        assert!(steps.iter().any(|s| s.contains("console")));
        assert!(!steps.iter().any(|s| s.contains("high/critical")));
    }

    #[test]
    fn next_steps_large_files_only() {
        let issues = vec![AuditIssue {
            severity: IssueSeverity::Medium,
            category: "Quality".to_string(),
            message: "File has 600 lines (>500)".to_string(),
            file: "big.rs".to_string(),
            line: 600,
            fixable: false,
        }];
        let steps = generate_next_steps(&issues);
        assert!(steps.iter().any(|s| s.contains("large file")));
    }

    #[test]
    fn next_steps_todo_only() {
        let issues = vec![AuditIssue {
            severity: IssueSeverity::Info,
            category: "Documentation".to_string(),
            message: "TODO comment".to_string(),
            file: "lib.rs".to_string(),
            line: 10,
            fixable: false,
        }];
        let steps = generate_next_steps(&issues);
        assert!(steps.iter().any(|s| s.contains("TODO/FIXME/HACK")));
    }

    #[test]
    fn next_steps_mixed_issues() {
        let issues = vec![
            AuditIssue {
                severity: IssueSeverity::High,
                category: "Security".to_string(),
                message: "env file".to_string(),
                file: ".env".to_string(),
                line: 0,
                fixable: false,
            },
            AuditIssue {
                severity: IssueSeverity::Low,
                category: "Quality".to_string(),
                message: "console statement found".to_string(),
                file: "a.ts".to_string(),
                line: 1,
                fixable: true,
            },
            AuditIssue {
                severity: IssueSeverity::Info,
                category: "Documentation".to_string(),
                message: "TODO comment".to_string(),
                file: "b.rs".to_string(),
                line: 5,
                fixable: false,
            },
        ];
        let steps = generate_next_steps(&issues);
        assert!(steps.len() >= 3);
        assert!(steps.iter().any(|s| s.contains("high/critical")));
        assert!(steps.iter().any(|s| s.contains("console")));
        assert!(steps.iter().any(|s| s.contains("TODO/FIXME/HACK")));
    }

    #[test]
    fn next_steps_counts_multiple_issues() {
        let issues = vec![
            AuditIssue {
                severity: IssueSeverity::Low,
                category: "Quality".to_string(),
                message: "console statement found".to_string(),
                file: "a.ts".to_string(),
                line: 1,
                fixable: true,
            },
            AuditIssue {
                severity: IssueSeverity::Low,
                category: "Quality".to_string(),
                message: "console statement found".to_string(),
                file: "b.ts".to_string(),
                line: 3,
                fixable: true,
            },
        ];
        let steps = generate_next_steps(&issues);
        assert!(steps.iter().any(|s| s.contains("2 console")));
    }

    // --- notification mapping ---

    fn issue_with(severity: IssueSeverity) -> AuditIssue {
        AuditIssue {
            severity,
            category: "Quality".to_string(),
            message: "sample".to_string(),
            file: "src/a.rs".to_string(),
            line: 1,
            fixable: false,
        }
    }

    #[test]
    fn issue_severity_maps_to_notification_priority() {
        // Taxonomy reserves `Critical` priority for control-plane events
        // (block / interrupt / fence-state); audit findings cap at `High`.
        let cases = [
            (IssueSeverity::Critical, NotificationPriority::High),
            (IssueSeverity::High, NotificationPriority::High),
            (IssueSeverity::Medium, NotificationPriority::Normal),
            (IssueSeverity::Low, NotificationPriority::Low),
            (IssueSeverity::Info, NotificationPriority::Low),
        ];
        for (severity, priority) in cases {
            let notification = notification_for_issue(&issue_with(severity));
            assert_eq!(notification.class, NotificationClass::Finding);
            assert_eq!(notification.priority, priority);
            assert_eq!(
                notification
                    .context
                    .as_ref()
                    .and_then(|c| c.source.as_deref()),
                Some("audit")
            );
            assert_eq!(
                notification
                    .context
                    .as_ref()
                    .and_then(|c| c.file.as_deref()),
                Some("src/a.rs")
            );
        }
    }

    #[test]
    fn notification_priority_never_uses_critical() {
        for severity in [
            IssueSeverity::Critical,
            IssueSeverity::High,
            IssueSeverity::Medium,
            IssueSeverity::Low,
            IssueSeverity::Info,
        ] {
            assert_ne!(
                notification_priority_for_severity(severity),
                NotificationPriority::Critical,
                "audit must not emit Critical priority for {severity:?}",
            );
        }
    }

    fn empty_audit_data() -> AuditData {
        AuditData {
            project_name: "p".to_string(),
            total_files: 10,
            issues: Vec::new(),
            historical_scores: Vec::new(),
            next_steps: Vec::new(),
        }
    }

    #[test]
    fn summary_notification_is_info_when_no_issues() {
        let notifications = notifications_for_audit(&empty_audit_data());
        assert_eq!(notifications.len(), 1);
        let summary = &notifications[0];
        assert_eq!(summary.class, NotificationClass::Info);
        assert_eq!(summary.priority, NotificationPriority::Low);
    }

    #[test]
    fn summary_notification_is_failure_when_critical_present() {
        let mut data = empty_audit_data();
        data.issues.push(issue_with(IssueSeverity::Critical));
        let notifications = notifications_for_audit(&data);
        let summary = notifications.last().unwrap();
        assert_eq!(summary.class, NotificationClass::Failure);
        // Priority is High (not Critical) — Critical is reserved for
        // control-plane events per the notification taxonomy.
        assert_eq!(summary.priority, NotificationPriority::High);
    }

    #[test]
    fn summary_notification_is_warning_when_high_present() {
        let mut data = empty_audit_data();
        data.issues.push(issue_with(IssueSeverity::High));
        let notifications = notifications_for_audit(&data);
        let summary = notifications.last().unwrap();
        assert_eq!(summary.class, NotificationClass::Warning);
        assert_eq!(summary.priority, NotificationPriority::High);
    }

    #[test]
    fn summary_notification_is_warning_when_only_medium_severity() {
        let mut data = empty_audit_data();
        data.issues.push(issue_with(IssueSeverity::Medium));
        let notifications = notifications_for_audit(&data);
        let summary = notifications.last().unwrap();
        // Previously `Info/Normal` — upgraded to `Warning/Normal` so a non-
        // empty medium-severity rollup is distinguishable from a clean run.
        assert_eq!(summary.class, NotificationClass::Warning);
        assert_eq!(summary.priority, NotificationPriority::Normal);
    }

    #[test]
    fn summary_notification_is_warning_when_only_low_or_info() {
        for severity in [IssueSeverity::Low, IssueSeverity::Info] {
            let mut data = empty_audit_data();
            data.issues.push(issue_with(severity));
            let notifications = notifications_for_audit(&data);
            let summary = notifications.last().unwrap();
            assert_eq!(
                summary.class,
                NotificationClass::Warning,
                "class for only-{severity:?}",
            );
            assert_eq!(summary.priority, NotificationPriority::Normal);
        }
    }

    #[test]
    fn build_audit_output_includes_notifications() {
        let mut data = empty_audit_data();
        data.issues.push(issue_with(IssueSeverity::Medium));
        data.issues.push(issue_with(IssueSeverity::Low));
        let output = build_audit_output(&data);
        // 2 per-issue notifications + 1 summary
        assert_eq!(output.notifications.len(), 3);
        let json = serde_json::to_value(&output).unwrap();
        assert!(json["notifications"].is_array());
        assert_eq!(json["notifications"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn notifications_are_capped_with_overflow_marker() {
        // OPS-002: unbounded allocation on large repos. Cap kicks in above
        // MAX_ISSUE_NOTIFICATIONS and emits a single truncation notification.
        let mut data = empty_audit_data();
        let overflow = 50;
        for _ in 0..(MAX_ISSUE_NOTIFICATIONS + overflow) {
            data.issues.push(issue_with(IssueSeverity::Low));
        }
        let notifications = notifications_for_audit(&data);

        // cap + 1 truncation + 1 summary
        assert_eq!(
            notifications.len(),
            MAX_ISSUE_NOTIFICATIONS + 2,
            "notifications must be capped at {MAX_ISSUE_NOTIFICATIONS}",
        );
        assert!(
            notifications
                .iter()
                .any(|n| n.title == "Audit notifications truncated"
                    && n.class == NotificationClass::Info),
            "expected truncation notification, got {notifications:?}",
        );
        assert!(
            notifications.iter().any(|n| n.title == "Audit summary"),
            "summary must still be present alongside the truncation marker",
        );
    }

    #[test]
    fn notifications_cap_prefers_highest_priority_findings() {
        // When truncated, the emitted per-issue notifications should be the
        // highest-severity ones so operators still see the signal.
        let mut data = empty_audit_data();
        // Fill most of the cap with Low, then add a handful of Critical.
        for _ in 0..MAX_ISSUE_NOTIFICATIONS {
            data.issues.push(issue_with(IssueSeverity::Low));
        }
        for _ in 0..3 {
            data.issues.push(issue_with(IssueSeverity::Critical));
        }
        let notifications = notifications_for_audit(&data);
        let critical_findings = notifications
            .iter()
            .filter(|n| n.class == NotificationClass::Finding && n.title.contains("Critical"))
            .count();
        assert_eq!(
            critical_findings, 3,
            "all Critical findings must survive truncation",
        );
    }

    // ── SARIF adapter (SARIFOUT-004) ────────────────────────────────

    fn issue(severity: IssueSeverity, category: &str, file: &str, line: usize) -> AuditIssue {
        AuditIssue {
            severity,
            category: category.to_string(),
            message: format!("{category} finding"),
            file: file.to_string(),
            line,
            fixable: false,
        }
    }

    #[test]
    fn audit_sarif_is_schema_valid_and_maps_severity() {
        let data = AuditData {
            project_name: "demo".to_string(),
            total_files: 2,
            issues: vec![
                issue(IssueSeverity::Critical, "hardcoded-secret", "src/a.ts", 4),
                issue(IssueSeverity::Medium, "large-file", "src/b.ts", 1),
                issue(IssueSeverity::Info, "large-file", "src/c.ts", 9),
                // Whole-file finding: line 0 must NOT emit `startLine: 0`
                // (schema `minimum: 1`) — the region is omitted instead.
                issue(IssueSeverity::High, "env-committed", ".env", 0),
            ],
            historical_scores: Vec::new(),
            next_steps: Vec::new(),
        };
        let value = serde_json::to_value(build_audit_sarif(&data)).expect("serialise");

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
        assert_eq!(results.len(), 4, "one result per audit issue");
        // category → ruleId, severity → level (Critical→error, Medium→warning,
        // Info→note).
        let crit = results
            .iter()
            .find(|r| r["ruleId"] == "hardcoded-secret")
            .unwrap();
        assert_eq!(crit["level"], "error");
        assert_eq!(
            crit["locations"][0]["physicalLocation"]["region"]["startLine"],
            4
        );
        assert!(
            results
                .iter()
                .any(|r| r["ruleId"] == "large-file" && r["level"] == "warning")
        );
        assert!(
            results
                .iter()
                .any(|r| r["ruleId"] == "large-file" && r["level"] == "note")
        );
        // `large-file` appears twice but is registered once in rules[].
        let rules = value["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .expect("rules");
        assert_eq!(
            rules.len(),
            3,
            "hardcoded-secret + large-file (deduped) + env-committed"
        );
        // Audit has no suppression model.
        assert!(results.iter().all(|r| r.get("suppressions").is_none()));

        // The whole-file finding (line 0) omits the region (no `startLine: 0`).
        let whole_file = results
            .iter()
            .find(|r| r["ruleId"] == "env-committed")
            .unwrap();
        assert!(
            whole_file["locations"][0]["physicalLocation"]
                .get("region")
                .is_none(),
            "line-0 finding must not emit a region"
        );
    }
}
