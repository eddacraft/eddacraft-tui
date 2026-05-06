use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anvil_checks::antipattern::{
    AntipatternCheckConfig, Warning, WarningSeverity, WarningSummary, run_antipattern_check,
};
use anvil_checks::secret::{SecretCheckConfig, SecretFinding, scan_content};
use walkdir::WalkDir;

/// Maximum number of files we will scan in the post-init analysis.
/// Matches the IFR-003 budget.
pub const SAMPLE_SIZE_LIMIT: usize = 50;

/// How far back to look in git history for changed files.
/// Matches the IFR-003 30-day window.
pub const SAMPLE_HISTORY_DAYS: u32 = 30;

/// Soft time budget for the post-init scan. We do not abort if exceeded —
/// we just surface the cost so a slow scan is visible to the user.
/// Matches the IFR-003 5-second target.
pub const ANALYSIS_TIME_BUDGET: Duration = Duration::from_secs(5);

/// Directories we skip during sample selection. Same set the `check`
/// command uses for `--all`.
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
];

/// Outcome of the post-init sample analysis. Holds the data the init
/// command needs to render a compact human summary.
#[derive(Debug, Clone)]
pub struct AnalysisOutcome {
    pub files_scanned: usize,
    pub source: SampleSource,
    pub summary: WarningSummary,
    pub elapsed: Duration,
    pub exceeded_budget: bool,
    pub top_warnings: Vec<TopWarning>,
    /// LAUNCH-016: tally of files belonging to `Unsupported` languages
    /// that were excluded from language-specific antipattern checks.
    /// Surfaces the skip honestly so users do not silently miss the
    /// gap; cross-language checks (secrets) still run on these files
    /// when invoked via separate code paths.
    pub skipped_unsupported_languages: crate::activation::language_profile::LanguageSkipLedger,
}

/// A trimmed-down warning view used for the inline summary. We do not
/// surface every warning at init time — just enough to demonstrate value
/// without overwhelming. Matches IFR-003's "show real results" intent
/// without becoming a full dashboard (that was IFR-005, out of scope).
#[derive(Debug, Clone)]
pub struct TopWarning {
    pub id: String,
    pub title: String,
    pub file: String,
    pub line: usize,
    pub severity: WarningSeverity,
}

/// How the sample was selected — used by the renderer to phrase the
/// "scanned X files" line accurately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleSource {
    GitHistory,
    RepoWalk,
    Empty,
}

/// Run the post-init sample analysis. Selects up to `SAMPLE_SIZE_LIMIT`
/// recently-changed source files (or falls back to a repo walk if git
/// history is unavailable) and runs the antipattern check against them.
///
/// Returns `None` if no analysable files were found — the caller should
/// skip the analysis section in that case rather than printing an empty
/// "0 files scanned" block.
#[must_use]
pub fn run_post_init_analysis(root: &Path) -> Option<AnalysisOutcome> {
    let started = Instant::now();
    let config = AntipatternCheckConfig::default();

    let (sample, source) = select_sample(root, SAMPLE_SIZE_LIMIT, SAMPLE_HISTORY_DAYS, &config);

    if sample.is_empty() {
        return None;
    }

    let file_strs: Vec<String> = sample
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    let file_refs: Vec<&str> = file_strs.iter().map(String::as_str).collect();

    let workspace_root = root
        .canonicalize()
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    // LAUNCH-016: the skip ledger reflects what was excluded from
    // THIS scan. `select_sample` already pre-filters by the
    // antipattern extension allowlist, so the ledger is necessarily
    // empty for the post-init path — there are no `Unsupported`-
    // language files in `file_refs` to skip. Emitting an empty
    // ledger preserves the contract on `AnalysisOutcome` without
    // walking the working tree (a full `profile_repo(root)` call
    // here would defeat `SAMPLE_SIZE_LIMIT` on large repos for no
    // observable benefit; the broader composition is surfaced via
    // `anvil status --verify`'s language profile, not init).
    //
    // Downstream PRs that broaden `select_sample` to include
    // unsupported-language files MUST call
    // `partition_for_language_specific_checks(&file_refs,
    // &profile_repo(root))` here and propagate the resulting
    // ledger; the partition helper is the seam.
    let skipped = crate::activation::language_profile::LanguageSkipLedger::default();

    let result = run_antipattern_check(&file_refs, &config, workspace_root.as_deref());
    let elapsed = started.elapsed();

    let mut top_warnings: Vec<TopWarning> = result
        .warnings
        .warnings
        .iter()
        .filter(|w| w.suppressed.is_none())
        .map(|w| TopWarning {
            id: w.id.clone(),
            title: w.title.clone(),
            file: w.location.file.clone(),
            line: w.location.line,
            severity: w.severity,
        })
        .collect();
    // Most severe first, then by file for stable ordering.
    top_warnings.sort_by(|a, b| {
        severity_rank(b.severity)
            .cmp(&severity_rank(a.severity))
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });
    top_warnings.truncate(5);

    Some(AnalysisOutcome {
        files_scanned: result.files_scanned,
        skipped_unsupported_languages: skipped,
        source,
        summary: result.warnings.summary,
        elapsed,
        exceeded_budget: elapsed > ANALYSIS_TIME_BUDGET,
        top_warnings,
    })
}

/// Outcome of a baseline-targeted scan: every finding the activation
/// flow can use to seed `.anvil/baseline.json`. Distinct from
/// [`AnalysisOutcome`] (which is shaped for the inline summary
/// renderer) — this one is shaped for the baseline writer.
///
/// LAUNCH-010: emitted by [`run_baseline_scan`]. Empty when the repo
/// has no analysable files.
#[derive(Debug, Clone, Default)]
pub(crate) struct BaselineScanOutcome {
    pub warnings: Vec<Warning>,
    pub secrets: Vec<SecretFinding>,
}

/// Run the same sample selection as [`run_post_init_analysis`] but
/// return raw `Warning` / `SecretFinding` lists for baseline
/// construction. Returns `None` when no analysable files are present.
///
/// Antipattern + secret scanners both run on the same sample so the
/// baseline reflects what a single first-activation scan saw, not two
/// drift-prone snapshots. Secret scanning is content-only — no git
/// history walk — to keep this bounded by the analyser's existing
/// time budget.
#[must_use]
pub(crate) fn run_baseline_scan(root: &Path) -> Option<BaselineScanOutcome> {
    let antipattern_config = AntipatternCheckConfig::default();
    let (sample, _source) = select_sample(
        root,
        SAMPLE_SIZE_LIMIT,
        SAMPLE_HISTORY_DAYS,
        &antipattern_config,
    );
    if sample.is_empty() {
        return None;
    }

    let file_strs: Vec<String> = sample
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    let file_refs: Vec<&str> = file_strs.iter().map(String::as_str).collect();

    let workspace_root = root
        .canonicalize()
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    let antipattern_result =
        run_antipattern_check(&file_refs, &antipattern_config, workspace_root.as_deref());

    // Secret scan: content-based, per file. Skip files we can't read or
    // that the antipattern scanner already counted as scanned but whose
    // bytes we can't get (e.g. transient I/O error). A file we can't
    // read can't have a finding here, so the silent skip is correct.
    let secret_config = SecretCheckConfig::default();
    let mut secrets: Vec<SecretFinding> = Vec::new();
    for path in &sample {
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let path_str = path.to_string_lossy();
        secrets.extend(scan_content(&content, &path_str, &secret_config));
    }

    Some(BaselineScanOutcome {
        warnings: antipattern_result.warnings.warnings,
        secrets,
    })
}

/// Pick a representative sample of source files for the post-init scan.
///
/// Strategy, in order:
/// 1. Files changed in git history over the last `days` (matches IFR-003).
/// 2. Fallback: walk the repo, taking the first `limit` matching files.
///
/// Returns the sample plus the selection source so the renderer can
/// phrase the message accurately.
fn select_sample(
    root: &Path,
    limit: usize,
    days: u32,
    config: &AntipatternCheckConfig,
) -> (Vec<PathBuf>, SampleSource) {
    if let Some(files) = git_recent_files(root, days, limit, config)
        && !files.is_empty()
    {
        return (files, SampleSource::GitHistory);
    }

    let walked = walk_repo_for_sample(root, limit, config);
    let source = if walked.is_empty() {
        SampleSource::Empty
    } else {
        SampleSource::RepoWalk
    };
    (walked, source)
}

/// Hard cap on the git subprocess used to gather recent files. The
/// `ANALYSIS_TIME_BUDGET` is measured *after* the subprocess returns and
/// cannot bound a hung child — on NFS or a stalled remote, that budget is
/// useless. This timeout prevents `anvil init` from blocking indefinitely
/// when git itself never returns.
const GIT_RECENT_FILES_TIMEOUT: Duration = Duration::from_secs(3);

/// Poll interval while waiting for the git subprocess to exit. Small
/// enough that the timeout boundary is tight; large enough that we don't
/// burn CPU spinning on `try_wait`.
const GIT_RECENT_FILES_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Ask git for files touched in the last `days`. Returns `None` if the
/// directory is not a git repo, git is unavailable, or the git subprocess
/// exceeded `GIT_RECENT_FILES_TIMEOUT` (in which case the child is killed
/// before returning so a hung filesystem call can't leak a stuck process).
/// An empty Vec means the repo exists but no in-window changes match our
/// extensions.
fn git_recent_files(
    root: &Path,
    days: u32,
    limit: usize,
    config: &AntipatternCheckConfig,
) -> Option<Vec<PathBuf>> {
    // `--diff-filter=d` excludes deletions, so we don't try to scan files
    // git knows about but no longer exist on disk.
    let since = format!("--since={days}.days");
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "log",
            &since,
            "--name-only",
            "--pretty=format:",
            "--diff-filter=d",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;

    let started = Instant::now();
    let output = loop {
        match child.try_wait() {
            Ok(Some(_)) => match child.wait_with_output() {
                Ok(output) => break output,
                Err(_) => return None,
            },
            Ok(None) => {
                if started.elapsed() >= GIT_RECENT_FILES_TIMEOUT {
                    // Timeout: the git call is taking too long (NFS, stalled
                    // remote, network filesystem). Terminate the child so we
                    // do not leak a stuck subprocess, then fall through to
                    // the walk-based sample so init still surfaces something.
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(GIT_RECENT_FILES_POLL_INTERVAL);
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut seen = std::collections::BTreeSet::new();
    let mut files: Vec<PathBuf> = Vec::new();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !has_matching_extension(trimmed, &config.extensions) {
            continue;
        }
        let path = root.join(trimmed);
        if !path.is_file() {
            continue;
        }
        let canonical = path.canonicalize().unwrap_or(path);
        if seen.insert(canonical.clone()) {
            files.push(canonical);
            if files.len() >= limit {
                break;
            }
        }
    }

    Some(files)
}

/// Walk the repo and collect up to `limit` matching files. Used when
/// git history is unavailable (no git repo, or `init` running in a fresh
/// non-git tree).
fn walk_repo_for_sample(
    root: &Path,
    limit: usize,
    config: &AntipatternCheckConfig,
) -> Vec<PathBuf> {
    let mut files = Vec::with_capacity(limit);

    for entry in WalkDir::new(root)
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
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let path_str = path.to_string_lossy();
        if !has_matching_extension(&path_str, &config.extensions) {
            continue;
        }
        // Canonicalise so `normalise_file_path` can strip the canonical
        // workspace root prefix consistently with the git-history branch
        // (which also canonicalises). Without this, a relative caller-supplied
        // root (e.g. `.`) produces walked paths like `./src/foo.ts` that won't
        // strip against the absolute canonical workspace root, leaving
        // un-normalised entries in `TopWarning.file`.
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        files.push(canonical);
        if files.len() >= limit {
            break;
        }
    }

    files
}

fn has_matching_extension(path: &str, extensions: &[String]) -> bool {
    extensions.iter().any(|ext| path.ends_with(ext.as_str()))
}

const fn severity_rank(s: WarningSeverity) -> u8 {
    match s {
        WarningSeverity::Error => 3,
        WarningSeverity::Warning => 2,
        WarningSeverity::Info => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(root: &Path, rel: &str, content: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn walk_repo_picks_supported_extensions() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "src/foo.ts", "export const x = 1;\n");
        write(dir.path(), "src/bar.tsx", "export const y = 2;\n");
        write(dir.path(), "README.md", "# nope\n");

        let config = AntipatternCheckConfig::default();
        let files = walk_repo_for_sample(dir.path(), 50, &config);

        assert_eq!(files.len(), 2, "expected 2 source files, got {files:?}");
        assert!(files.iter().any(|p| p.ends_with("foo.ts")));
        assert!(files.iter().any(|p| p.ends_with("bar.tsx")));
    }

    #[test]
    fn walk_repo_skips_ignored_dirs() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "src/foo.ts", "export const x = 1;\n");
        write(
            dir.path(),
            "node_modules/pkg/index.ts",
            "export const z = 3;\n",
        );
        write(dir.path(), "dist/bundle.js", "console.log(1);\n");

        let config = AntipatternCheckConfig::default();
        let files = walk_repo_for_sample(dir.path(), 50, &config);

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("foo.ts"));
    }

    #[test]
    fn walk_repo_respects_limit() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..10 {
            write(dir.path(), &format!("src/f{i}.ts"), "export const x = 1;\n");
        }

        let config = AntipatternCheckConfig::default();
        let files = walk_repo_for_sample(dir.path(), 3, &config);
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn select_sample_falls_back_to_walk_when_no_git() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "src/foo.ts", "export const x = 1;\n");

        let config = AntipatternCheckConfig::default();
        let (files, source) = select_sample(dir.path(), 50, 30, &config);

        assert_eq!(files.len(), 1);
        assert_eq!(source, SampleSource::RepoWalk);
    }

    #[test]
    fn select_sample_returns_empty_for_empty_tree() {
        let dir = tempfile::tempdir().unwrap();
        let config = AntipatternCheckConfig::default();
        let (files, source) = select_sample(dir.path(), 50, 30, &config);
        assert!(files.is_empty());
        assert_eq!(source, SampleSource::Empty);
    }

    #[test]
    fn run_post_init_analysis_returns_none_for_empty_tree() {
        let dir = tempfile::tempdir().unwrap();
        let outcome = run_post_init_analysis(dir.path());
        assert!(outcome.is_none());
    }

    #[test]
    fn run_post_init_analysis_scans_a_real_file() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "src/foo.ts", "export const x = 1;\n");

        let outcome = run_post_init_analysis(dir.path()).expect("should produce outcome");
        assert!(outcome.files_scanned >= 1);
        assert_eq!(outcome.source, SampleSource::RepoWalk);
    }
}
