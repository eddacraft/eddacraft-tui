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

    let AuditRun {
        data,
        mut coverage_notes,
        mut unread_files,
    } = run_audit(Path::new("."));

    match mode {
        OutputMode::Json => print_json(&data, &coverage_notes)?,
        // SARIF 2.1.0 carries tool-execution problems in
        // `invocations[].toolExecutionNotifications`, which `anvil-sarif` does
        // not model yet. Until it does, a `--format sarif` run reports a
        // file-level coverage failure through its exit code alone; the
        // document is unchanged, and a line-level skip is invisible to a SARIF
        // consumer entirely. Emitting a `results[]` entry instead would put
        // "nobody read this file" into the finding vocabulary, which is the
        // one thing SDT-008 is barred from doing. Documented as a known gap in
        // `docs/runbooks/sarif-code-scanning-upload.md`.
        OutputMode::Sarif => crate::output::json::print(&build_audit_sarif(&data))?,
        OutputMode::Tui => {
            // The TUI takes the alternate screen, so a coverage block drawn
            // before it would be wiped and one drawn into it needs a panel
            // this surface does not have. Report on stderr after the screen is
            // restored: stdout stays clean, and the operator still sees what
            // the exit code is about.
            let mut state = AuditState::new(data);
            loop {
                state = crate::tui::run_surface(state)?;
                if let Some(request) = state.pending_fix.take() {
                    let selected = state.selected_item;
                    if matches!(
                        apply_fix_request(&request, None),
                        FixOutcome::Applied { .. }
                    ) {
                        // Re-scan through `run_audit`, not `collect_audit_data`:
                        // the refresh is what the exit code and the stderr
                        // block below are computed from, so dropping the
                        // coverage half here would report the pre-fix state.
                        // The TUI itself still shows only `data` — rendering
                        // the notes inside the surface needs a panel it does
                        // not have (see the comment above).
                        let refreshed = run_audit(Path::new("."));
                        state.data = refreshed.data;
                        coverage_notes = refreshed.coverage_notes;
                        unread_files = refreshed.unread_files;
                        state.selected_item =
                            selected.min(state.data.issues.len().saturating_sub(1));
                        state.expanded = false;
                    }
                    continue;
                }
                break;
            }
            eprint!("{}", crate::util::secret_coverage_suffix(&coverage_notes));
        }
        OutputMode::Plain => print_plain(&data, &coverage_notes),
    }

    // SDT-008: audit stays advisory about its *findings* — a repository full
    // of TODOs and hardcoded secrets still exits 0, because audit is a project
    // overview and warnings-over-blocks is the product posture. Coverage is a
    // different claim: "I read these files and here is what I found" is false
    // when some of them were never read, and a surface that cannot tell the
    // difference is the false-clean SDT-001 and SDT-006 exist to remove.
    //
    // ── The asymmetry, and why it is not an inconsistency ──────────────
    //
    // Only *file*-level coverage failures fail this command. A line-level
    // skip (`lines_skipped_oversize`) is reported in full — in the human
    // block, in `coverageNotes`, in the notifications and in the next steps —
    // but contributes nothing to the exit code. This is deliberate, and it
    // differs from what `gate` and planless `check` do with the same note:
    //
    //   * `anvil gate` is diff-scoped. A long line in the change under review
    //     is the author's own line, and asking them about it is actionable.
    //   * planless `anvil check <files>` is argument-scoped. The operator
    //     named the file, so a line inside it that the scan could not read is
    //     an answer to a question they actually asked.
    //   * `anvil audit` is neither. It walks the whole tree un-scoped, and
    //     every real checkout carries long lines nobody wrote and nobody can
    //     fix: vendored bundles, `coverage-final.json`, `node_modules` inside
    //     nested worktrees. Measured on the anvil repository itself, all of
    //     the long lines came from exactly those. An exit code that is 1 on
    //     every checkout tells an operator nothing, and a red with no
    //     available action is the unactionable-red failure SDT-001 recorded
    //     against itself — the same defect class as the false clean, pointing
    //     the other way.
    //
    // File-level failures are not in that population: an unreadable file, a
    // scan that panicked, or a file over the size limit is a bounded, named,
    // fixable set on any surface. So they still fail here.
    //
    // Do NOT "tidy" this into uniformity in either direction: dropping the
    // note would restore the silence, and failing on it would restore the
    // noise. The split is tested both ways in
    // `crates/anvil-cli/tests/secret_coverage_surfaces.rs`
    // (operator decision, 2026-08-29).
    if unread_files {
        // Already rendered above for every mode; `AlreadyReported` exits
        // non-zero without printing it a second time.
        Err(crate::output::AlreadyReported.into())
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Data gathering
// ---------------------------------------------------------------------------

/// Source file extensions we scan for issues.
const SOURCE_EXTS: &[&str] = &["ts", "js", "rs", "py"];

/// Maximum line count before a file is flagged.
const MAX_FILE_LINES: usize = 500;

/// One audit run: the surface data, plus what the run could not cover.
///
/// SDT-008: the coverage notes live here rather than inside [`AuditData`]
/// because they are not audit findings and must not be counted, sorted or
/// severity-ranked as if they were. `AuditData` is the issue model every
/// renderer consumes; a coverage failure is a statement about the run that
/// produced it — closer to `total_files` than to `issues[]` — and unread
/// *files* among them are the only thing that decides this command's exit
/// code.
pub struct AuditRun {
    pub data: AuditData,
    /// Every reason the secret scan could not see all of its input, in the
    /// scanner's own words. Empty means audit read everything in its domain.
    ///
    /// Reported in full on every output mode. **Not** the exit-code
    /// predicate — see `unread_files`.
    pub coverage_notes: Vec<String>,
    /// True when at least one whole *file* in audit's domain went unread: it
    /// could not be opened as text, its scan panicked, or it was over the size
    /// limit. This — not `coverage_notes` — decides audit's exit code.
    ///
    /// The two are separate because they answer different questions.
    /// `coverage_notes` answers "what could this run not see?", which the
    /// operator always wants. `unread_files` answers "is that a failure *on
    /// this surface*?", which is a judgement about audit's un-scoped walk and
    /// is made once, in [`run`]. A line-level skip sets the first and not the
    /// second; every file-level cause sets both.
    pub unread_files: bool,
}

/// Collect audit data for the current directory (convenience for sub-surface use).
///
/// Drops the coverage notes. The remaining caller is `commands::welcome`'s
/// embedded audit surface, which has no exit code to carry and no place to
/// draw them: `anvil_tui::surfaces::audit::AuditState` models only
/// [`AuditData`], and giving it the notes means adding a field in the
/// `anvil-tui` crate plus a panel — the Project panel is a fixed
/// `Constraint::Length(10)` — which is TUI work this repair deliberately
/// leaves. Consequence, stated rather than hidden: coverage gaps are invisible
/// inside `anvil` → Run audit. `anvil audit` itself uses [`run_audit`]
/// directly on both the first scan and the post-fix refresh, so the notes and
/// the exit code always reach the operator there.
pub fn collect_audit_data() -> AuditData {
    run_audit(Path::new(".")).data
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
pub fn run_audit(root: &Path) -> AuditRun {
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
            // CIB-237: render through the shared helper so audit agrees with
            // `check` and `gate` on separators instead of emitting native ones.
            let rel = crate::display_path::render(&path.to_string_lossy(), Some(root));
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
    // SDT-008: the same call now also reports what it could not read. Those
    // are not issues and never enter `issues[]`.
    let SecretCoverage {
        notes: coverage_notes,
        unread_files,
    } = scan_for_hardcoded_secrets(root, &candidates, &mut issues);

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
    let next_steps = generate_next_steps(&issues, &coverage_notes);

    AuditRun {
        data: AuditData {
            project_name,
            total_files,
            security_scope: SECURITY_SCOPE.to_string(),
            issues,
            historical_scores,
            next_steps,
        },
        coverage_notes,
        unread_files,
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

// Secret allow-list extensions: `crate::util::SECRET_SCAN_EXTS` (shared with
// gate — single definition for issue #1798 lock-step).

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
///
/// What audit's secret pass could not see: the operator-facing prose, and
/// whether any of it was a whole unread file.
///
/// The classification is read off the scanner's own structured accounting
/// (`files_skipped_unreadable` / `_panicked` / `_oversize`, plus
/// `history_scan_errors`) rather than by matching text in the notes. Prose is
/// for operators; a predicate that parses it would break the first time a note
/// is reworded, and silently — in the direction of exiting 0.
#[derive(Default)]
struct SecretCoverage {
    /// Every reason the scan could not see all of its input, in the scanner's
    /// own words. Always reported in full.
    notes: Vec<String>,
    /// True when at least one whole file went unread. See [`AuditRun`] for why
    /// this is tracked apart from `notes`.
    unread_files: bool,
}

/// Returns the scan's coverage notes — every reason it could not see all of
/// its input (SDT-008). These are deliberately *not* `AuditIssue` entries: an
/// issue has a file, a line, a severity and a category, and a file nobody read
/// has none of those. Forcing one into that shape would report a fabricated
/// location for a fabricated finding — trading the false-clean this fixes for
/// a false-positive, which is the trade the item's non-scope forbids. They
/// travel beside the issues instead.
fn scan_for_hardcoded_secrets(
    root: &Path,
    candidates: &[(std::path::PathBuf, String)],
    issues: &mut Vec<AuditIssue>,
) -> SecretCoverage {
    if candidates.is_empty() {
        return SecretCoverage::default();
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
                .is_some_and(crate::util::secret_scan_ext_allowed)
        })
        .map(|(path, rel)| (path.to_string_lossy().into_owned(), rel.as_str()))
        .collect();

    if scannable.is_empty() {
        return SecretCoverage::default();
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
    let config = crate::util::secret_check_config(root);
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

    // Every cause that means a whole file went unread, as opposed to a line
    // inside a file audit did read. Taken from the scanner's per-cause vectors
    // so the two halves of the exit-code decision cannot drift from the prose:
    // if `anvil-checks` grows a fifth blocking cause, this line stops
    // compiling to a lie only if someone extends it — so the destructure is
    // spelled out rather than folded into a helper, to put the choice in front
    // of whoever adds the field.
    //
    // `files_skipped_extension` is absent on purpose: it is a configured
    // operator exclusion, never a failure, and it never reaches
    // `coverage_notes` either.
    let unread_files = !result.files_skipped_unreadable.is_empty()
        || !result.files_skipped_panicked.is_empty()
        || !result.files_skipped_oversize.is_empty()
        // A git-history scan that failed did not read the revisions it was
        // asked for. Off by default (`scan_git_history: false`), but it is a
        // failure to do the work, not a bound on it, so it belongs here.
        || !result.history_scan_errors.is_empty();

    SecretCoverage {
        // Passed through verbatim so all three surfaces describe the same
        // unread file in the same words. The paths inside them are the ones
        // audit handed the scanner (`./src/foo.ts` for the usual `root = "."`)
        // rather than the `rel` form used by `issues[]`: the notes are prose
        // built by `anvil-checks`, and rewriting paths inside a sentence to
        // gain a two-character cosmetic match would risk corrupting the remedy
        // the note exists to deliver.
        notes: result.coverage_notes,
        unread_files,
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

/// Load up to 4 historical score entries from `.anvil/gate-history.ndjson`.
fn load_historical_scores(root: &Path) -> Vec<HistoricalScore> {
    crate::services::gate_history::load_recent(root, 4)
        .into_iter()
        .map(|point| HistoricalScore {
            timestamp: point.timestamp_display(),
            score: point.score,
            issue_count: point.warning_count,
        })
        .collect()
}

/// Generate actionable next steps from the issue list.
///
/// SDT-008: `coverage_notes` is not an issue list, but it is the most
/// actionable thing a run can produce — the report above it is incomplete
/// until it is cleared — so it leads, and it suppresses the "nothing to do
/// here" fallback that would otherwise sit under a failing run.
fn generate_next_steps(issues: &[AuditIssue], coverage_notes: &[String]) -> Vec<String> {
    let mut steps = Vec::new();

    if !coverage_notes.is_empty() {
        steps.push(
            "Restore secret-scan coverage first — this report is not a clean result \
             while part of audit's scope was never read"
                .to_string(),
        );
    }

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
        // "Looks clean" over-claims: audit runs a strict subset of the
        // checks, so an empty audit is a statement about audit's scope,
        // not about the tree (CIB-234). Route to `gate` — it is the
        // full suite; planless `check` runs only two of the checks.
        steps.push(
            "No issues in audit's scope — run `anvil gate` for the full check suite".to_string(),
        );
    }

    steps
}

// ---------------------------------------------------------------------------
// Output: plain text
// ---------------------------------------------------------------------------

/// What audit's security pass actually covers, in terms a reader can
/// act on.
///
/// CIB-234: a reader compared `anvil audit`'s secret count against
/// another surface's and read the gap as "the tree is cleaner than that
/// says" or "that surface is wrong". Both counts were right for their
/// surface; audit simply never said what its own covers.
///
/// Every clause here is load-bearing and checked against the code —
/// vague copy was the original defect, so wrong-but-specific copy would
/// be a worse one:
///
/// - `.env` handling is **not** one finding per file: `check_env_file`
///   adds a file-level flag, and `scan_for_hardcoded_secrets` then
///   scans the same file and adds one entry per pattern match.
/// - Findings are **not** summarised — each secret match is its own
///   `High` entry (see `scan_for_hardcoded_secrets`).
/// - Audit and gate select the **same file types** — the extension list
///   and the `.env*` filename rule are kept in lock-step deliberately
///   (see `crate::util::SECRET_SCAN_EXTS`), and claiming they differ would licence
///   the drift that issue #1798 fixed. Still say "types", not "set":
///   CIB-280 removed gate's planless depth cap, so the two walks now
///   agree on depth, but `anvil gate <plan>` still narrows to the plan's
///   `Files:` entries and audit never does. One narrowing mode remains,
///   so asserting set equality would be the same over-claim in the
///   opposite direction. The depth half of that agreement is pinned by
///   `gate_and_audit_secret_walks_reach_the_same_deep_file` in `gate.rs`.
/// - The full check suite is `anvil gate`, not `anvil check --all` —
///   the planless `check` path runs only `PLANLESS_ELIGIBLE_CHECKS`
///   (`secret-detection`, `antipattern-scan`), so pointing a reader
///   there "for the full report" would hand them a narrower surface
///   wearing the word "full".
///
/// So the honest difference is which *checks* run, not which files are
/// read. Stating that is the fix; forcing counts to match across
/// surfaces stays out of scope — that needs a product decision.
const SECURITY_SCOPE: &str = "audit is a project overview. Its security pass flags `.env` files \
     and runs the same secret-detection patterns as `anvil gate` over the same \
     file types — one entry per match, plus one file-level flag per `.env`. It \
     does not run gate's architecture, policy, dependency or lint checks, so \
     finding counts are not comparable between surfaces and an audit with \
     nothing to report is not a passing `anvil gate`.";

fn print_plain(data: &AuditData, coverage_notes: &[String]) {
    print!("{}", render_plain(data, coverage_notes));
}

fn render_plain(data: &AuditData, coverage_notes: &[String]) -> String {
    use std::fmt::Write as _;

    // Writing into a String is infallible; `let _ =` keeps the render
    // path free of unwrap noise. One buffer for the whole report —
    // a second buffer for the body would copy every finding twice on
    // trees large enough for that to matter (cf. the OPS-002 cap on
    // `notifications[]`).
    let mut out = String::new();
    let _ = writeln!(out, "ANVIL AUDIT — {}\n", data.project_name);
    let _ = writeln!(out, "Total files scanned: {}", data.total_files);
    let _ = writeln!(out, "Issues found: {}", data.issues.len());
    // SDT-008: the coverage block rides with the count for the same reason
    // the scope note does. "Issues found: 0" is the line a reader skims, and
    // it is not a statement about the files this run never read — putting the
    // correction in a footer would leave the skim wrong.
    if !coverage_notes.is_empty() {
        // The shared renderer is built to be *appended* to a gate message, so
        // it carries its own leading separator. Here it is a standalone
        // paragraph: strip that separator and set the spacing locally.
        let block = crate::util::secret_coverage_suffix(coverage_notes);
        let _ = writeln!(out, "\n{}\n", block.trim_start_matches('\n'));
    }
    // Sits with the count, not in a footer: the count is what gets
    // skimmed, so the qualifier has to travel with it. Wrapped rather
    // than emitted as one long line — an unbroken paragraph is the
    // shape readers skip, which would defeat the point.
    let _ = writeln!(out, "Security scope:");
    for line in wrap_scope_note(&data.security_scope, 76) {
        let _ = writeln!(out, "  {line}");
    }
    let _ = writeln!(out);

    render_body_into(&mut out, data);
    out
}

/// Greedy word wrap for the scope note. Deliberately tiny — pulling a
/// wrapping crate in for one paragraph is not worth the dependency.
fn wrap_scope_note(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.len() + 1 + word.len() > width {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn render_body_into(out: &mut String, data: &AuditData) {
    use std::fmt::Write as _;

    if !data.issues.is_empty() {
        let _ = writeln!(out, "ISSUES");
        for issue in &data.issues {
            let badge = match issue.severity {
                IssueSeverity::Critical => "[CRIT]",
                IssueSeverity::High => "[HIGH]",
                IssueSeverity::Medium => "[MED] ",
                IssueSeverity::Low => "[LOW] ",
                IssueSeverity::Info => "[INFO]",
            };
            let fix_tag = if issue.fixable { " (fixable)" } else { "" };
            // `line: 0` is the whole-file sentinel (a committed `.env`, say),
            // not a zero-based line. Rendering it as `.env:0` implied a
            // zero-based numbering no anvil surface uses (CIB-237); the SARIF
            // adapter below already omitted the region for it.
            let location = crate::display_path::format_location(&issue.file, issue.line);
            let _ = writeln!(out, "  {badge} {:<15} {location}{fix_tag}", issue.category);
            let _ = writeln!(out, "        {}", issue.message);
        }
    }

    if !data.historical_scores.is_empty() {
        let _ = writeln!(out, "\nHISTORICAL SCORES");
        for score in &data.historical_scores {
            let _ = writeln!(
                out,
                "  {}  score: {:.2}  issues: {}",
                score.timestamp, score.score, score.issue_count,
            );
        }
    }

    if !data.next_steps.is_empty() {
        let _ = writeln!(out, "\nNEXT STEPS");
        for (i, step) in data.next_steps.iter().enumerate() {
            let _ = writeln!(out, "  {}. {step}", i + 1);
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
    /// What audit's security pass covers, so a consumer comparing
    /// `issues.len()` against another surface's count can see the two
    /// run different checks rather than infer a discrepancy. Carries
    /// the same text the human output shows. Additive field;
    /// `issues[]` remains canonical.
    security_scope: String,
    issues: Vec<IssueOutput>,
    /// SDT-008: every reason audit's secret pass could not see all of its
    /// input, in the scanner's own words (cause plus remedy). Kept out of
    /// `issues[]` on purpose — an issue carries a file, a line and a severity,
    /// and a file nobody read has none of them, so counting one as an issue
    /// would be a fabricated finding. Absent when audit read everything in its
    /// domain.
    ///
    /// Non-empty does **not** imply a non-zero exit: on `audit` only unread
    /// *files* fail the command, while an unscanned long *line* is reported
    /// here on a passing run (operator decision, 2026-08-29 — see [`run`]).
    /// Consumers deciding whether coverage is complete must read this field
    /// rather than infer it from the exit code.
    #[serde(rename = "coverageNotes", skip_serializing_if = "Vec::is_empty")]
    coverage_notes: Vec<String>,
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
    /// `null` for a whole-file finding.
    ///
    /// CIB-237: this was `0` for findings about a file's existence rather than
    /// a line in it, which read as a zero-based line number next to the
    /// 1-based numbers everywhere else. The key is retained (rather than
    /// skipped) so consumers reading `issue.line` still find it.
    line: Option<usize>,
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

fn notifications_for_audit(data: &AuditData, coverage_notes: &[String]) -> Vec<Notification> {
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

    // SDT-008: one notification per coverage gap, uncapped. There is at most
    // one per cause (history, oversize line, panicked/unreadable/oversize
    // file), so the `MAX_ISSUE_NOTIFICATIONS` cap that findings need does not
    // apply. `Warning`, not `Finding`: nothing was found here — something was
    // not looked at.
    notifications.extend(coverage_notes.iter().map(|note| {
        Notification::new(
            NotificationClass::Warning,
            NotificationPriority::High,
            "Secret scan coverage gap",
            note.clone(),
        )
        .with_context(audit_context.clone())
    }));

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
        // SDT-008: "No issues in audit's scope across N files" is a claim
        // about N files, and it is false when part of them was never read.
        // A subscriber that only reads the summary must not be told the run
        // was clean when it was not — and that holds for a line-level skip
        // too, which no longer fails the command but still leaves lines this
        // run cannot prove clean.
        if coverage_notes.is_empty() {
            (
                NotificationClass::Info,
                NotificationPriority::Low,
                format!(
                    "No issues in audit's scope across {} files; `anvil gate` runs the full check suite",
                    data.total_files
                ),
            )
        } else {
            (
                NotificationClass::Warning,
                NotificationPriority::High,
                // No count: the notes carry per-cause counts and this
                // function has no honest number of its own to quote.
                "0 issues found, but part of audit's scope was never read — \
                 this is not a clean result; see coverageNotes"
                    .to_string(),
            )
        }
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

fn build_audit_output(data: &AuditData, coverage_notes: &[String]) -> AuditOutput {
    AuditOutput {
        project_name: data.project_name.clone(),
        total_files: data.total_files,
        security_scope: data.security_scope.clone(),
        issues: data
            .issues
            .iter()
            .map(|i| IssueOutput {
                severity: i.severity.label().to_string(),
                category: i.category.clone(),
                message: i.message.clone(),
                file: i.file.clone(),
                line: (i.line > 0).then_some(i.line),
                fixable: i.fixable,
            })
            .collect(),
        coverage_notes: coverage_notes.to_vec(),
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
        notifications: notifications_for_audit(data, coverage_notes),
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
        rules.entry(issue.category.clone()).or_insert_with(|| {
            sarif::ReportingDescriptor::new(issue.category.clone())
                .full_description(data.security_scope.clone())
        });
        // Audit uses `line: 0` for whole-file findings (e.g. `.env` files);
        // SARIF `startLine` has `minimum: 1`, so omit the region in that case
        // and point at the artifact only.
        let line = (issue.line > 0).then(|| u32::try_from(issue.line).unwrap_or(u32::MAX));
        let region = line.and_then(sarif::Region::try_line);
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
    sarif::SarifLog::new(
        sarif::Run::new(rules.into_values().collect(), results)
            .security_scope(data.security_scope.clone()),
    )
}

fn print_json(data: &AuditData, coverage_notes: &[String]) -> anyhow::Result<()> {
    let output = build_audit_output(data, coverage_notes);
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
        let data = run_audit(&dir).data;
        assert_eq!(data.issues.len(), 0);
        assert_eq!(data.total_files, 0);
        cleanup(&dir);
    }

    #[test]
    fn detects_console_log_in_ts() {
        let dir = make_temp_dir();
        let ts_file = dir.join("example.ts");
        std::fs::write(&ts_file, "const x = 1;\nconsole.log(x);\n").unwrap();

        let data = run_audit(&dir).data;
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

    /// CIB-237: a committed `.env` is a whole-file finding, carried as the
    /// `line: 0` sentinel. It must never surface as the zero-based-looking
    /// `.env:0`.
    #[test]
    fn env_file_finding_uses_the_whole_file_sentinel() {
        let dir = make_temp_dir();
        std::fs::write(dir.join(".env"), "API_KEY=abc\n").unwrap();

        let data = run_audit(&dir).data;
        let env_issue = data
            .issues
            .iter()
            .find(|i| i.message.contains("Environment file"))
            .expect("env issue");
        assert_eq!(env_issue.line, 0, "sentinel is preserved internally");
        assert_eq!(
            crate::display_path::format_location(&env_issue.file, env_issue.line),
            ".env",
            "plain output must not render the sentinel as a line number"
        );
        cleanup(&dir);
    }

    /// The JSON surface reports the sentinel as `null`, never `0`, so every
    /// rendered line number stays 1-based (CIB-237).
    #[test]
    fn json_output_nulls_the_whole_file_sentinel_and_keeps_real_lines() {
        let dir = make_temp_dir();
        std::fs::write(dir.join(".env"), "API_KEY=abc\n").unwrap();
        std::fs::write(dir.join("example.ts"), "const x = 1;\nconsole.log(x);\n").unwrap();

        let data = run_audit(&dir).data;
        let output = build_audit_output(&data, &[]);

        let env = output
            .issues
            .iter()
            .find(|i| i.message.contains("Environment file"))
            .expect("env issue");
        assert_eq!(env.line, None);

        let console = output
            .issues
            .iter()
            .find(|i| i.message.contains("console statement"))
            .expect("console issue");
        assert_eq!(console.line, Some(2), "real lines stay 1-based");

        let value = serde_json::to_value(&output).expect("serialise");
        let env_json = value["issues"]
            .as_array()
            .expect("issues array")
            .iter()
            .find(|i| {
                i["message"]
                    .as_str()
                    .is_some_and(|m| m.contains("Environment file"))
            })
            .expect("env issue json");
        assert!(
            env_json.get("line").is_some_and(serde_json::Value::is_null),
            "the `line` key is retained as null, not dropped: {env_json}"
        );
        cleanup(&dir);
    }

    /// Audit paths must use the shared forward-slash style, not native
    /// separators (CIB-237).
    #[test]
    fn nested_paths_render_with_forward_slashes() {
        let dir = make_temp_dir();
        std::fs::create_dir_all(dir.join("src/nested")).unwrap();
        std::fs::write(
            dir.join("src/nested/app.ts"),
            "const x = 1;\nconsole.log(x);\n",
        )
        .unwrap();

        let data = run_audit(&dir).data;
        let issue = data
            .issues
            .iter()
            .find(|i| i.message.contains("console statement"))
            .expect("console issue");
        assert_eq!(issue.file, "src/nested/app.ts");
        assert!(!issue.file.contains('\\'), "got: {}", issue.file);
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

        let data = run_audit(&dir).data;
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

        let data = run_audit(&dir).data;
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

        let data = run_audit(&dir).data;
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

        let data = run_audit(&dir).data;
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

        let data = run_audit(&dir).data;
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

        let data = run_audit(&dir).data;
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

        let data = run_audit(&dir).data;
        assert!(!data.next_steps.is_empty());
        assert!(data.next_steps.iter().any(|s| s.contains("high/critical")));
        assert!(data.next_steps.iter().any(|s| s.contains("console")));
        cleanup(&dir);
    }

    /// A tree that audit finds nothing in must not be reported as clean
    /// outright — audit is an overview surface that runs a subset of
    /// the checks, and `anvil gate` applies rules audit never does. The
    /// "nothing here" step has to scope its claim and name the fuller
    /// surface.
    #[test]
    fn clean_project_next_step_scopes_its_claim() {
        let dir = make_temp_dir();
        std::fs::write(dir.join("clean.rs"), "fn main() {}\n").unwrap();

        let data = run_audit(&dir).data;
        assert_eq!(data.next_steps.len(), 1);
        let step = &data.next_steps[0];
        assert!(
            step.contains("audit's scope"),
            "a clean audit must scope its claim, got: {step}"
        );
        assert!(
            step.contains("anvil gate"),
            "a clean audit must point at the full check suite, got: {step}"
        );
        cleanup(&dir);
    }

    /// CIB-234: on the reported tree `check --all` surfaced 4 secret
    /// findings and `audit` 2. Both are correct for their surface —
    /// the failure is that audit never says so, letting a lower count
    /// read as a cleaner tree. The plain summary must name the domain.
    #[test]
    fn plain_output_discloses_security_scope() {
        let dir = make_temp_dir();
        std::fs::write(dir.join(".env"), "API_KEY=sk-live-abcdefghijklmnop\n").unwrap();
        let data = run_audit(&dir).data;
        let out = render_plain(&data, &[]);

        assert!(
            out.contains("Security scope"),
            "audit must name its security domain:\n{out}"
        );
        assert!(
            out.contains("anvil gate"),
            "audit must point at the full check suite:\n{out}"
        );
        assert!(
            out.contains("not comparable between surfaces"),
            "audit must refuse the 'count comparison = safety verdict' reading:\n{out}"
        );
        assert!(
            out.contains("not a passing `anvil gate`"),
            "an empty audit must not be readable as a passing gate:\n{out}"
        );

        // Placement is load-bearing, not decoration: the qualifier has
        // to travel with the count a reader skims. Footer placement
        // would satisfy every substring assertion above and still lose
        // the argument, so pin the ordering.
        let scope_at = out.find("Security scope").expect("scope line present");
        let count_at = out.find("Issues found").expect("count line present");
        let issues_at = out.find("ISSUES").expect("issue list present");
        assert!(
            count_at < scope_at && scope_at < issues_at,
            "scope note must sit between the count and the findings, not in a footer:\n{out}"
        );
        cleanup(&dir);
    }

    /// The disclosure is a property of the surface, not of a populated
    /// result — a zero-issue audit is exactly when the reader is most
    /// likely to over-read it.
    #[test]
    fn plain_output_discloses_scope_even_with_no_issues() {
        let dir = make_temp_dir();
        std::fs::write(dir.join("clean.rs"), "fn main() {}\n").unwrap();
        let data = run_audit(&dir).data;
        let out = render_plain(&data, &[]);
        assert!(
            out.contains("Security scope"),
            "scope disclosure must survive an empty finding list:\n{out}"
        );
        cleanup(&dir);
    }

    /// JSON consumers need the same disclosure the human reader gets.
    #[test]
    fn json_output_carries_the_security_scope() {
        let dir = make_temp_dir();
        std::fs::write(dir.join(".env"), "API_KEY=sk-live-abcdefghijklmnop\n").unwrap();
        let data = run_audit(&dir).data;
        let value = serde_json::to_value(build_audit_output(&data, &[])).unwrap();

        let scope = value["security_scope"]
            .as_str()
            .expect("JSON output must carry security_scope");
        assert!(
            scope.contains("anvil gate"),
            "security_scope must name the full check suite, got: {scope}"
        );
        assert_eq!(
            scope, SECURITY_SCOPE,
            "JSON and human output must carry the same disclosure verbatim"
        );
        // The canonical finding list stays exactly where it was.
        assert!(value["issues"].is_array(), "issues[] must remain canonical");
        cleanup(&dir);
    }

    /// Non-scope guard: this item disclosed the domain, it did NOT
    /// change aggregation. `.env` stays one file-level finding rather
    /// than being fanned out to match `check --all`'s per-rule count.
    #[test]
    fn scope_disclosure_does_not_change_env_aggregation() {
        let dir = make_temp_dir();
        std::fs::write(
            dir.join(".env"),
            "API_KEY=sk-live-abcdefghijklmnop\nDB_PASSWORD=hunter2hunter2hunter2\n",
        )
        .unwrap();
        let data = run_audit(&dir).data;

        let env_issues: Vec<_> = data.issues.iter().filter(|i| i.file == ".env").collect();
        let file_level = env_issues
            .iter()
            .filter(|i| i.message.contains("Environment file"))
            .count();
        let per_pattern = env_issues
            .iter()
            .filter(|i| i.message.contains("Potential hardcoded secret"))
            .count();

        assert_eq!(
            file_level, 1,
            "exactly one file-level flag per `.env`, got: {env_issues:?}"
        );
        assert_eq!(
            per_pattern, 2,
            "one entry per pattern match — no fan-out, no collapsing; forcing count \
             parity with another surface is out of scope, got: {env_issues:?}"
        );
        assert_eq!(
            env_issues.len(),
            file_level + per_pattern,
            "no other `.env` findings should appear: {env_issues:?}"
        );
        cleanup(&dir);
    }

    /// The scope note describes `.env` handling, so that description
    /// must track the code. If aggregation changes, this fails right
    /// next to the copy that would silently become a lie.
    #[test]
    fn security_scope_matches_actual_env_handling() {
        let dir = make_temp_dir();
        std::fs::write(dir.join(".env"), "API_KEY=sk-live-abcdefghijklmnop\n").unwrap();
        let data = run_audit(&dir).data;
        let env_issues: Vec<_> = data.issues.iter().filter(|i| i.file == ".env").collect();

        // Copy promises: one file-level flag PLUS one entry per match.
        assert!(
            SECURITY_SCOPE.contains("one entry per match")
                && SECURITY_SCOPE.contains("one file-level flag per `.env`"),
            "scope copy must state the actual `.env` shape: {SECURITY_SCOPE}"
        );
        assert_eq!(
            env_issues.len(),
            2,
            "one `.env` with one secret must yield flag + match, matching the copy: {env_issues:?}"
        );

        // Guard against resurrecting claims that were checked and found
        // false: `.env` reported "once", findings "summarised", a file
        // set differing from gate's (the *types* match by invariant), or
        // the mirror-image over-claim that the two traversals are
        // identical (gate depth-caps and plan-scopes; audit does not).
        for overclaim in [
            "reported once",
            "summarised",
            "different file set",
            "same file set",
        ] {
            assert!(
                !SECURITY_SCOPE.contains(overclaim),
                "scope copy must not claim {overclaim:?}: {SECURITY_SCOPE}"
            );
        }
        // Planless `check` runs only PLANLESS_ELIGIBLE_CHECKS, so it
        // must never be sold to the reader as the full surface.
        assert!(
            !SECURITY_SCOPE.contains("check --all"),
            "the full suite is `anvil gate`, not planless `check`: {SECURITY_SCOPE}"
        );
        cleanup(&dir);
    }

    #[test]
    fn historical_scores_from_gate_history() {
        let dir = make_temp_dir();
        let anvil = dir.join(".anvil");
        std::fs::create_dir_all(&anvil).unwrap();
        std::fs::write(
            anvil.join("gate-history.ndjson"),
            concat!(
                r#"{"recorded_at":"2026-08-17T19:41:45Z","score":90.0,"status":"pass","status_label":"PASSED","warning_count":3,"duration_seconds":"0.5","checks_run":"8"}"#,
                "\n",
                r#"{"recorded_at":"2026-08-16T10:00:00Z","score":80.0,"status":"fail","status_label":"FAILED","warning_count":5,"duration_seconds":"1","checks_run":"8"}"#,
                "\n",
            ),
        )
        .unwrap();

        let data = run_audit(&dir).data;
        assert_eq!(data.historical_scores.len(), 2);
        assert_eq!(data.historical_scores[0].timestamp, "2026-08-17 19:41");
        assert_eq!(data.historical_scores[0].issue_count, 3);
        cleanup(&dir);
    }

    #[test]
    fn typescript_cache_index_is_not_a_score_source() {
        let dir = make_temp_dir();
        let cache_dir = dir.join(".anvil/cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(
            cache_dir.join("index.json"),
            r#"{
                "entries": {
                    "gate:f.md:1710000000": {"score": 0.9, "issueCount": 3}
                }
            }"#,
        )
        .unwrap();

        let data = run_audit(&dir).data;
        assert!(data.historical_scores.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn no_cache_yields_empty_historical() {
        let dir = make_temp_dir();
        let data = run_audit(&dir).data;
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

    /// An empty finding list is a statement about audit's scope, not a
    /// clean bill of health for the tree (CIB-234), so the single step
    /// must qualify the claim and route to the fuller surface.
    #[test]
    fn next_steps_empty_issues() {
        let steps = generate_next_steps(&[], &[]);
        assert_eq!(steps.len(), 1);
        assert!(
            steps[0].contains("audit's scope"),
            "empty result must scope its claim, got: {}",
            steps[0]
        );
        assert!(
            steps[0].contains("anvil gate"),
            "empty result must name the full check suite, got: {}",
            steps[0]
        );
        assert!(
            !steps[0].contains("check --all"),
            "planless `check` runs only two checks; it must not be sold as the \
             full surface, got: {}",
            steps[0]
        );
    }

    /// SDT-008: the "nothing to do here" fallback is a clean-result claim, and
    /// it must not appear under a run that never read part of its scope.
    #[test]
    fn next_steps_lead_with_coverage_and_drop_the_clean_fallback() {
        let notes = vec!["1 file(s) could not be read (src/broken.ts)".to_string()];
        let steps = generate_next_steps(&[], &notes);
        assert!(
            steps[0].contains("Restore secret-scan coverage"),
            "the coverage failure is the first thing to act on, got: {steps:?}"
        );
        assert!(
            !steps
                .iter()
                .any(|s| s.contains("No issues in audit's scope")),
            "an unread file makes the clean fallback false, got: {steps:?}"
        );
    }

    /// The JSON summary notification is the single line a subscriber reads.
    /// With zero issues and a coverage failure it must not be the `Info`
    /// "no issues in scope" line — part of the scope was never read, whether
    /// or not that failed the command.
    #[test]
    fn audit_json_summary_is_not_clean_when_coverage_failed() {
        let data = empty_audit_data();
        let notes = vec!["1 file(s) could not be read (src/broken.ts)".to_string()];
        let output = build_audit_output(&data, &notes);
        assert_eq!(output.coverage_notes, notes);

        let summary = output
            .notifications
            .last()
            .expect("summary notification is always last");
        assert_eq!(summary.class, NotificationClass::Warning);
        assert!(
            !summary.message.contains("No issues in audit's scope"),
            "clean-summary copy must not survive a coverage failure, got: {}",
            summary.message
        );
        assert!(
            output
                .notifications
                .iter()
                .any(|n| n.message.contains("src/broken.ts")),
            "the unread file must reach notification subscribers"
        );
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
        let steps = generate_next_steps(&issues, &[]);
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
        let steps = generate_next_steps(&issues, &[]);
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
        let steps = generate_next_steps(&issues, &[]);
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
        let steps = generate_next_steps(&issues, &[]);
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
        let steps = generate_next_steps(&issues, &[]);
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
        let steps = generate_next_steps(&issues, &[]);
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
            security_scope: SECURITY_SCOPE.to_string(),
            issues: Vec::new(),
            historical_scores: Vec::new(),
            next_steps: Vec::new(),
        }
    }

    #[test]
    fn summary_notification_is_info_when_no_issues() {
        let notifications = notifications_for_audit(&empty_audit_data(), &[]);
        assert_eq!(notifications.len(), 1);
        let summary = &notifications[0];
        assert_eq!(summary.class, NotificationClass::Info);
        assert_eq!(summary.priority, NotificationPriority::Low);
    }

    #[test]
    fn summary_notification_is_failure_when_critical_present() {
        let mut data = empty_audit_data();
        data.issues.push(issue_with(IssueSeverity::Critical));
        let notifications = notifications_for_audit(&data, &[]);
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
        let notifications = notifications_for_audit(&data, &[]);
        let summary = notifications.last().unwrap();
        assert_eq!(summary.class, NotificationClass::Warning);
        assert_eq!(summary.priority, NotificationPriority::High);
    }

    #[test]
    fn summary_notification_is_warning_when_only_medium_severity() {
        let mut data = empty_audit_data();
        data.issues.push(issue_with(IssueSeverity::Medium));
        let notifications = notifications_for_audit(&data, &[]);
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
            let notifications = notifications_for_audit(&data, &[]);
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
        let output = build_audit_output(&data, &[]);
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
        let notifications = notifications_for_audit(&data, &[]);

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
        let notifications = notifications_for_audit(&data, &[]);
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
            security_scope: SECURITY_SCOPE.to_string(),
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

        assert_eq!(
            value["runs"][0]["properties"]["securityScope"], SECURITY_SCOPE,
            "SARIF run metadata must carry the same security scope verbatim"
        );

        let rules = value["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .expect("rules");
        assert!(!rules.is_empty(), "fixture must emit audit rules");
        for rule in rules {
            assert_eq!(
                rule["fullDescription"]["text"], SECURITY_SCOPE,
                "every audit rule must expose the security scope in a visible description"
            );
        }

        let schema: serde_json::Value =
            serde_json::from_str(anvil_sarif::SARIF_SCHEMA_JSON).expect("schema json");
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
