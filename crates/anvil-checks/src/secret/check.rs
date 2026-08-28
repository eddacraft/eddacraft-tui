use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use rayon::prelude::*;

use crate::secret::git_scanner::scan_git_history;
use crate::secret::patterns::compile_custom_patterns;
use crate::secret::scanner::{ScanStats, scan_content_with_compiled_patterns};
use crate::secret::types::{
    FindingType, SecretCheckConfig, SecretCheckResult, SecretFinding, Suppression,
    partition_skip_extensions,
};

/// Maximum file size to scan (1 MiB). Files of this size or larger are
/// skipped to avoid excessive memory usage on binaries or generated artefacts.
pub const MAX_FILE_SIZE: u64 = 1024 * 1024;
const _: () = assert!(MAX_FILE_SIZE == 1024 * 1024);

/// At most this many paths are named inline in a coverage note. Beyond it the
/// note says "and N more" — the full list always stays on the result, but a
/// gate message that dumps 400 paths is not read by anyone.
const MAX_NAMED_PATHS: usize = 3;

/// SDT-006: what happened to one candidate file. Every arm except
/// [`FileOutcome::Scanned`] means "this file was not scanned", and the
/// distinction between the arms is the whole point: a configured
/// `skip_extensions` match is a deliberate exclusion, the other three are
/// coverage the check *wanted* and did not get.
enum FileOutcome {
    Scanned {
        findings: Vec<SecretFinding>,
        suppressions: Vec<Suppression>,
        lines_skipped_oversize: usize,
    },
    /// A configured `skip_extensions` entry matched. Advisory only.
    SkippedExtension,
    /// At or over [`MAX_FILE_SIZE`].
    Oversize(String),
    /// Exists, but `fs::read_to_string` failed (permissions, non-UTF-8, …).
    Unreadable(String),
    /// The scan panicked and SCAN-001's `catch_unwind` contained it.
    Panicked(String),
}

/// SDT-006: every file `run_secret_check` declined to scan, grouped by cause.
///
/// Three of the four block a clean pass; `extension` never does. Keeping them
/// in one value is what lets [`coverage_notes`] report all of them at once
/// instead of the first one it happens to notice.
#[derive(Debug, Default)]
struct UnscannedFiles {
    oversize: Vec<String>,
    unreadable: Vec<String>,
    panicked: Vec<String>,
    extension: usize,
}

impl UnscannedFiles {
    /// Did anything happen here that a clean pass must not paper over?
    ///
    /// `extension` is deliberately absent: an operator who configured
    /// `skip_extensions` asked for that skip, and the defaults (`.png`,
    /// `.jpg`, `.lock`) exist in every repository.
    fn blocks(&self) -> bool {
        !self.oversize.is_empty() || !self.unreadable.is_empty() || !self.panicked.is_empty()
    }

    /// Impose a stable order independent of the caller's file list. Anvil's
    /// first principle is same-input-same-output, and two callers handing the
    /// same set of files in different orders are the same input.
    ///
    /// Note this is *not* about rayon: `par_iter().map().collect()` is an
    /// indexed parallel iterator and preserves input order. The
    /// non-determinism this guards against comes from the walker above,
    /// which does not promise a stable enumeration order.
    fn sort(&mut self) {
        self.oversize.sort();
        self.unreadable.sort();
        self.panicked.sort();
    }
}

pub fn run_secret_check(
    files: &[&str],
    config: &SecretCheckConfig,
    workspace_root: Option<&str>,
) -> SecretCheckResult {
    // V050F-011: compile custom patterns ONCE up front. Previously
    // `scan_content_with_stats` recompiled them per file (per parallel
    // worker, per scan), and the per-pattern compile diagnostics were
    // silently dropped on every recompile. The compiled slice is
    // shared across rayon workers; `pattern_errors` is reported via
    // `SecretCheckResult.pattern_errors` so a misconfigured custom
    // pattern surfaces at the boundary that owns config.
    let (compiled_custom_patterns, pattern_errors) =
        compile_custom_patterns(&config.custom_patterns);

    run_secret_check_with_scanner(
        files,
        config,
        workspace_root,
        pattern_errors,
        |content, display_path| {
            scan_content_with_compiled_patterns(
                content,
                display_path,
                config,
                &compiled_custom_patterns,
                usize::MAX,
            )
        },
    )
}

/// [`run_secret_check`] with the per-file scan supplied by the caller.
///
/// The parameterisation exists for one reason: SCAN-001 wraps the scan in
/// `catch_unwind`, and before SDT-006 that arm had never been executed by a
/// test — a panicking scan discarded the file and the result still reported
/// "No secrets detected", passed, score 100. No input reachably panics the
/// real scanner today (probed across the credit-card, URL-path, drive-letter
/// and redaction slicing paths with adversarial multibyte content and custom
/// patterns named after the built-in rules), so a test cannot get at the arm
/// through the public entry point. Handing the scan in lets the guard test
/// drive the *real* accounting, assembly and message path with a scan that
/// does panic. It is not a production seam: `run_secret_check` is the only
/// non-test caller.
fn run_secret_check_with_scanner<F>(
    files: &[&str],
    config: &SecretCheckConfig,
    workspace_root: Option<&str>,
    mut pattern_errors: Vec<String>,
    scan: F,
) -> SecretCheckResult
where
    F: Fn(&str, &str) -> (Vec<SecretFinding>, ScanStats) + Sync,
{
    // A `skip_extensions` entry only narrows the scan if it is a well-formed
    // dotted suffix. An empty entry would match every path (`ends_with("")`)
    // and silently disable non-lockfile scanning while still reporting a
    // clean pass, so invalid entries are dropped and surfaced through the
    // same channel as a misconfigured custom pattern.
    let (skip_extensions, skip_extension_warnings) =
        partition_skip_extensions(&config.skip_extensions);
    pattern_errors.extend(skip_extension_warnings);

    // SCAN-001: read + scan each candidate file in parallel on the rayon
    // pool, mirroring the welcome-screen discovery shape. Per-file panics are
    // contained via `catch_unwind`. SDT-006: nothing drops out silently any
    // more — every non-scanned file returns the reason it was not scanned, so
    // the fold below can account for it. Deterministic ordering is restored
    // downstream after dedupe via `sort_findings` and `UnscannedFiles::sort`.
    let per_file: Vec<FileOutcome> = files
        .par_iter()
        .map(|file| scan_one_file(file, workspace_root, &skip_extensions, &scan))
        .collect();

    let mut findings: Vec<SecretFinding> = Vec::new();
    let mut suppressions: Vec<Suppression> = Vec::new();
    let mut lines_skipped_oversize = 0usize;
    let mut unscanned = UnscannedFiles::default();
    for outcome in per_file {
        match outcome {
            FileOutcome::Scanned {
                findings: file_findings,
                suppressions: file_suppressions,
                lines_skipped_oversize: file_lines_skipped,
            } => {
                findings.extend(file_findings);
                suppressions.extend(file_suppressions);
                lines_skipped_oversize += file_lines_skipped;
            }
            FileOutcome::SkippedExtension => unscanned.extension += 1,
            FileOutcome::Oversize(path) => unscanned.oversize.push(path),
            FileOutcome::Unreadable(path) => unscanned.unreadable.push(path),
            FileOutcome::Panicked(path) => unscanned.panicked.push(path),
        }
    }
    unscanned.sort();

    let history_scan_errors = merge_git_history_scan(
        config,
        workspace_root,
        &mut findings,
        &mut lines_skipped_oversize,
    );

    let findings = sort_findings(deduplicate_findings(findings));
    let suppressions = finalize_suppressions(suppressions);
    assemble_secret_check_result(
        findings,
        suppressions,
        pattern_errors,
        lines_skipped_oversize,
        history_scan_errors,
        unscanned,
    )
}

/// Select, read and scan one candidate file, reporting *why* when it is not
/// scanned. SDT-006: the four declining paths used to return `None` into a
/// `filter_map`, which is precisely how a whole file could vanish from a
/// result that still claimed a clean pass.
fn scan_one_file<F>(
    file: &str,
    workspace_root: Option<&str>,
    skip_extensions: &[&str],
    scan: &F,
) -> FileOutcome
where
    F: Fn(&str, &str) -> (Vec<SecretFinding>, ScanStats) + Sync,
{
    if should_skip_file(file, skip_extensions) {
        return FileOutcome::SkippedExtension;
    }
    // Report skipped files under the same path rendering as findings, so a
    // consumer never has to reconcile two path shapes from one result.
    let display_path = normalise_file_path(file, workspace_root);
    if file_exceeds_size_limit(file) {
        return FileOutcome::Oversize(display_path);
    }
    let Ok(content) = fs::read_to_string(file) else {
        return FileOutcome::Unreadable(display_path);
    };
    // SCAN-001: contain panics from custom user regexes so a single bad
    // pattern can't tear down the whole secret scan. SDT-006: containing it
    // is still right; returning nothing was the bug.
    match catch_unwind(AssertUnwindSafe(|| scan(&content, &display_path))) {
        Ok((findings, stats)) => FileOutcome::Scanned {
            findings,
            suppressions: stats.suppressions,
            lines_skipped_oversize: stats.lines_skipped_oversize,
        },
        Err(_) => FileOutcome::Panicked(display_path),
    }
}

/// Run the optional git-history pass, folding findings and skip counts into the
/// on-disk totals. Returns actionable errors when requested coverage cannot run.
fn merge_git_history_scan(
    config: &SecretCheckConfig,
    workspace_root: Option<&str>,
    findings: &mut Vec<SecretFinding>,
    lines_skipped_oversize: &mut usize,
) -> Vec<String> {
    if !config.scan_git_history {
        return Vec::new();
    }
    let root = workspace_root.unwrap_or(".");
    match scan_git_history(root, config) {
        Ok(history) => {
            findings.extend(history.findings);
            // Fold the history scan's oversize-line skips into the same total
            // so a "0 findings" result isn't silently hiding skipped content.
            *lines_skipped_oversize += history.lines_skipped_oversize;
            // history.pattern_errors are duplicates of the file-scan errors
            // (same custom_patterns input compiled twice) — already captured.
            Vec::new()
        }
        Err(err) => {
            // Fail closed: requested history coverage that cannot run must
            // not collapse into the ordinary clean-pass result.
            vec![format!(
                "Requested git history secret scan could not run: {err}"
            )]
        }
    }
}

fn assemble_secret_check_result(
    findings: Vec<SecretFinding>,
    suppressions: Vec<Suppression>,
    pattern_errors: Vec<String>,
    lines_skipped_oversize: usize,
    history_scan_errors: Vec<String>,
    unscanned: UnscannedFiles,
) -> SecretCheckResult {
    // SDT-001: everything that means "we did not scan all of it" collects
    // here. History-scan failures block a clean pass even when no secret
    // findings were produced — otherwise a broken scan looks identical to
    // "clean" — and an oversize-line skip is the same class of lie: the
    // SCAN-002 guard drops the line before *both* pattern and entropy
    // evaluation, so a secret inside it cannot have been seen.
    //
    // SDT-006 adds the file-level members of that same class. A configured
    // `skip_extensions` match is *not* one of them and never reaches here.
    let coverage_notes = coverage_notes(&history_scan_errors, lines_skipped_oversize, &unscanned);
    let passed = findings.is_empty() && coverage_notes.is_empty();
    let pattern_count = findings
        .iter()
        .filter(|finding| finding.finding_type == FindingType::Pattern)
        .count();
    let entropy_count = findings
        .iter()
        .filter(|finding| finding.finding_type == FindingType::Entropy)
        .count();
    let score_usize = if lines_skipped_oversize > 0 || unscanned.blocks() {
        // Unscanned surface makes the score meaningless, not merely lower:
        // there is no denominator for "how much of this file was checked".
        // SDT-006 follows SDT-001's rule unchanged — an unread file is a
        // larger hole than an unread line, not a smaller one.
        0
    } else if !history_scan_errors.is_empty() && findings.is_empty() {
        // Incomplete coverage is not a full score.
        0
    } else {
        100_usize.saturating_sub(findings.len().saturating_mul(10))
    };
    let score = u8::try_from(score_usize).unwrap_or(0);
    let message = secret_check_message(&findings, pattern_count, entropy_count, &coverage_notes);

    SecretCheckResult {
        passed,
        score,
        message,
        findings,
        pattern_errors,
        lines_skipped_oversize,
        suppressions,
        history_scan_errors,
        coverage_notes,
        files_skipped_oversize: unscanned.oversize,
        files_skipped_unreadable: unscanned.unreadable,
        files_skipped_panicked: unscanned.panicked,
        files_skipped_extension: unscanned.extension,
    }
}

/// SDT-001/SDT-006: every reason this scan could not see all of its input, in
/// a stable order. History failures first (they can mean *nothing* ran), then
/// the oversize-line skip, then the file-level skips sharpest-first — a panic
/// is a bug, an unreadable file is an environment problem, an oversize file is
/// a known bound. All of them are reported: a result can carry several at
/// once, and an operator who fixes one must not be surprised by the next.
fn coverage_notes(
    history_scan_errors: &[String],
    lines_skipped_oversize: usize,
    unscanned: &UnscannedFiles,
) -> Vec<String> {
    let mut notes = history_scan_errors.to_vec();
    if lines_skipped_oversize > 0 {
        notes.push(oversize_skip_note(lines_skipped_oversize));
    }
    if !unscanned.panicked.is_empty() {
        notes.push(panicked_file_note(&unscanned.panicked));
    }
    if !unscanned.unreadable.is_empty() {
        notes.push(unreadable_file_note(&unscanned.unreadable));
    }
    if !unscanned.oversize.is_empty() {
        notes.push(oversize_file_note(&unscanned.oversize));
    }
    notes
}

/// The operator-facing text for an oversize-line skip. It names the count
/// *and* both remedies deliberately: a repo with legitimately long lines in
/// scanned extensions flips from silent-green to red here, and a red with no
/// stated action is just a different kind of useless.
fn oversize_skip_note(lines_skipped_oversize: usize) -> String {
    format!(
        "{lines_skipped_oversize} line(s) too long to scan, so this result cannot prove them \
         clean: raise `max_line_bytes` to cover them, or suppress with a documented reason \
         (ADR-029)"
    )
}

/// Name the paths behind a coverage note, capped so one bad directory cannot
/// bury the remedy at the end of a wall of text.
fn named_paths(paths: &[String]) -> String {
    let shown = paths
        .iter()
        .take(MAX_NAMED_PATHS)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    match paths.len().checked_sub(MAX_NAMED_PATHS) {
        Some(rest) if rest > 0 => format!("{shown}, and {rest} more"),
        _ => shown,
    }
}

/// SDT-006, path 4 — the sharpest. Naming the file is the whole value here:
/// the panic is contained, so the file is the only evidence of which input
/// provoked it, and a custom regex is the documented cause SCAN-001 was
/// written for.
fn panicked_file_note(paths: &[String]) -> String {
    format!(
        "{} file(s) crashed the scanner and were not scanned, so this result cannot prove them \
         clean ({}): this is a bug — a `custom_patterns` regex is the usual cause, so remove or \
         narrow the pattern and report the crash",
        paths.len(),
        named_paths(paths)
    )
}

/// SDT-006, path 3. The remedy is an environment fix, not a config one, so
/// the note says which environment property to look at.
fn unreadable_file_note(paths: &[String]) -> String {
    format!(
        "{} file(s) could not be read, so this result cannot prove them clean ({}): the scanner \
         reads UTF-8 text — fix the file's permissions or encoding, or exclude it with \
         `skip_extensions` if it is binary",
        paths.len(),
        named_paths(paths)
    )
}

/// SDT-006, path 2. `MAX_FILE_SIZE` is a fixed resource bound and SDT-006 is
/// explicitly barred from raising it, so — unlike the `max_line_bytes` note —
/// "raise the limit" is not offered. The two remedies that do exist are.
///
/// Lockfiles get an extra sentence, because for them the first remedy is a
/// lie: [`should_skip_file`] returns early for a lockfile so that it reaches
/// the GH #2584 URL-credential scan, which means `skip_extensions` cannot
/// exclude one however it is configured. That is not a corner case here — the
/// file that motivated this item is this repository's own `pnpm-lock.yaml`.
/// Naming a remedy that provably does nothing would trade a false clean for a
/// false instruction.
fn oversize_file_note(paths: &[String]) -> String {
    // The caveat names the lockfiles it applies to. `named_paths` caps the
    // inline list at MAX_NAMED_PATHS, so a lockfile sorting past that cap would
    // otherwise tell the operator "a lockfile is the exception" without saying
    // which file — an exception arriving without its subject is worse than
    // either alone, and the generic remedy beside it is false for exactly that
    // file.
    let lockfiles: Vec<&str> = paths
        .iter()
        .filter(|path| crate::filter::is_lockfile(Path::new(path)))
        .map(String::as_str)
        .collect();
    let lockfile_caveat = if lockfiles.is_empty() {
        String::new()
    } else {
        format!(
            " ({} cannot be excluded that way: `skip_extensions` deliberately does not apply to \
             lockfiles, because they carry the URL-credential scan, so an oversize lockfile has \
             no exclusion remedy today)",
            lockfiles.join(", ")
        )
    };
    format!(
        "{} file(s) at or over the {} MiB scan limit were not scanned, so this result cannot \
         prove them clean ({}): exclude them with `skip_extensions` if they are generated \
         artefacts, or split them below the limit{lockfile_caveat}",
        paths.len(),
        MAX_FILE_SIZE / (1024 * 1024),
        named_paths(paths)
    )
}

fn secret_check_message(
    findings: &[SecretFinding],
    pattern_count: usize,
    entropy_count: usize,
    coverage_notes: &[String],
) -> String {
    if findings.is_empty() {
        if coverage_notes.is_empty() {
            return "No secrets detected".to_string();
        }
        return coverage_notes.join("; ");
    }
    let mut parts = Vec::new();
    if pattern_count > 0 {
        parts.push(format!("{pattern_count} pattern match(es)"));
    }
    if entropy_count > 0 {
        parts.push(format!("{entropy_count} high-entropy string(s)"));
    }
    let base = format!(
        "Found {} potential secret(s): {}",
        findings.len(),
        parts.join(", ")
    );
    if coverage_notes.is_empty() {
        base
    } else {
        format!("{base}; {}", coverage_notes.join("; "))
    }
}

/// Deterministically order and de-duplicate suppression records. Parallel
/// collection interleaves files, and the entropy detector's dual
/// quoted/assignment match records the same candidate twice (mirrors
/// `deduplicate_findings`).
fn finalize_suppressions(mut suppressions: Vec<Suppression>) -> Vec<Suppression> {
    // Sort key includes every field the dedup compares (provenance last) so
    // true duplicates are always adjacent regardless of which tier matched.
    suppressions.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.rule_name.cmp(&b.rule_name))
            .then(a.redacted_match.cmp(&b.redacted_match))
            .then(a.provenance.cmp(&b.provenance))
    });
    suppressions.dedup_by(|a, b| {
        a.file == b.file
            && a.line == b.line
            && a.rule_name == b.rule_name
            && a.redacted_match == b.redacted_match
            && a.provenance == b.provenance
    });
    suppressions
}

/// `skip_extensions` must already be validated by
/// [`partition_skip_extensions`] — an unvalidated empty entry would make
/// this return `true` for every path.
fn should_skip_file(file: &str, skip_extensions: &[&str]) -> bool {
    // Lockfiles are NOT skipped: they get a restricted URL-credential-only scan
    // in `scan_content_with_compiled_patterns` (GH #2584). Returning `false`
    // here forces them past the `.lock` entry in `skip_extensions`, so
    // `Cargo.lock`/`yarn.lock` reach that scan too — not just the non-`.lock`
    // lockfiles like `package-lock.json`.
    //
    // `.env*` files are likewise not skipped: `run_secret_check` backs `anvil
    // gate` (the `secret-detection` guardrail) and `anvil audit`, whose job is
    // to catch a *committed* secret, including one in a tracked `.env`. The
    // `.env` first-run noise that GH #2584 reports is suppressed in the
    // discovery scan only (`welcome::candidate_path`), not here.
    if crate::filter::is_lockfile(Path::new(file)) {
        return false;
    }
    skip_extensions
        .iter()
        .any(|extension| file.ends_with(extension))
}

fn file_exceeds_size_limit(file: &str) -> bool {
    fs::metadata(file).is_ok_and(|m| m.len() >= MAX_FILE_SIZE)
}

fn deduplicate_findings(findings: Vec<SecretFinding>) -> Vec<SecretFinding> {
    let mut seen = std::collections::BTreeSet::new();

    findings
        .into_iter()
        .filter(|finding| {
            let finding_type = match finding.finding_type {
                FindingType::Pattern => "pattern",
                FindingType::Entropy => "entropy",
            };
            let key = format!(
                "{}:{}:{}:{}:{}:{}",
                finding.file,
                finding.line,
                finding_type,
                finding.pattern_name,
                finding.redacted_match,
                finding.redacted_line
            );
            seen.insert(key)
        })
        .collect()
}

fn sort_findings(mut findings: Vec<SecretFinding>) -> Vec<SecretFinding> {
    findings.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| finding_type_key(a).cmp(&finding_type_key(b)))
            .then_with(|| a.pattern_name.cmp(&b.pattern_name))
    });
    findings
}

const fn finding_type_key(finding: &SecretFinding) -> u8 {
    match finding.finding_type {
        FindingType::Pattern => 0,
        FindingType::Entropy => 1,
    }
}

fn normalise_file_path(file: &str, workspace_root: Option<&str>) -> String {
    let Some(root) = workspace_root else {
        return file.to_string();
    };

    let file_path = Path::new(file);
    let root_path = Path::new(root);
    if let Ok(relative) = file_path.strip_prefix(root_path) {
        let relative_str = relative.to_string_lossy().replace('\\', "/");
        format!("/{relative_str}")
    } else {
        file.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::secret::check::run_secret_check;
    use crate::secret::types::{FindingType, SecretCheckConfig, SecretFinding};

    fn create_temp_dir(name: &str) -> PathBuf {
        let base = std::env::temp_dir();
        let unique = format!(
            "anvil-checks-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        );
        let path = base.join(unique);
        let _ = fs::create_dir_all(&path);
        path
    }

    /// The false-clean this guards against: `str::ends_with("")` is true
    /// for every path, so an empty `skip_extensions` entry made
    /// `should_skip_file` skip every non-lockfile file. The scan then
    /// reported "No secrets detected", score 100, passed — with nothing
    /// scanned at all.
    #[test]
    fn empty_skip_extension_does_not_disable_scanning() {
        let temp_dir = create_temp_dir("empty-skip-ext");
        let file = temp_dir.join("secret.ts");
        assert!(fs::write(&file, "api_key='abcdEFGH1234567890'\n").is_ok());

        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let config = SecretCheckConfig {
            skip_extensions: vec![String::new()],
            ..SecretCheckConfig::default()
        };
        let result = run_secret_check(&files, &config, None);

        assert_eq!(
            result.findings.len(),
            1,
            "an empty skip extension must not disable scanning; got {:?}",
            result.findings,
        );
        assert!(!result.passed);
        assert!(
            result
                .pattern_errors
                .iter()
                .any(|e| e.contains("skip_extensions")),
            "invalid config must be surfaced, got {:?}",
            result.pattern_errors,
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    /// Pathspec-magic and undotted values are equally unusable: they would
    /// narrow coverage on a malformed value.
    #[test]
    fn invalid_skip_extensions_are_ignored_and_reported() {
        let temp_dir = create_temp_dir("invalid-skip-ext");
        let file = temp_dir.join("secret.ts");
        assert!(fs::write(&file, "api_key='abcdEFGH1234567890'\n").is_ok());

        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        for bad in ["", ".", "ts", ":(exclude)*.ts", ".t s"] {
            let config = SecretCheckConfig {
                skip_extensions: vec![bad.to_string()],
                ..SecretCheckConfig::default()
            };
            let result = run_secret_check(&files, &config, None);
            assert_eq!(
                result.findings.len(),
                1,
                "skip_extensions {bad:?} must not narrow the scan",
            );
            assert!(
                result
                    .pattern_errors
                    .iter()
                    .any(|e| e.contains("skip_extensions")),
                "skip_extensions {bad:?} must be reported",
            );
        }

        let _ = fs::remove_dir_all(temp_dir);
    }

    /// The valid case must keep working — the validation must not turn the
    /// denylist off wholesale.
    #[test]
    fn valid_skip_extension_still_skips_and_is_not_reported() {
        let temp_dir = create_temp_dir("valid-skip-ext");
        let file = temp_dir.join("bundle.min.js");
        assert!(fs::write(&file, "api_key='abcdEFGH1234567890'\n").is_ok());

        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let result = run_secret_check(&files, &SecretCheckConfig::default(), None);

        assert!(
            result.findings.is_empty(),
            "`.min.js` must still be skipped, got {:?}",
            result.findings,
        );
        assert!(
            !result
                .pattern_errors
                .iter()
                .any(|e| e.contains("skip_extensions")),
            "the default denylist must not be reported as invalid, got {:?}",
            result.pattern_errors,
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn computes_pass_fail_and_score() {
        let temp_dir = create_temp_dir("score");
        let file = temp_dir.join("secret.ts");
        let write_result = fs::write(&file, "api_key='abcdEFGH1234567890'\npassword='hunter22'");
        assert!(write_result.is_ok());

        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let result = run_secret_check(&files, &SecretCheckConfig::default(), None);

        assert!(!result.passed);
        assert_eq!(result.score, 80);
        assert_eq!(result.findings.len(), 2);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn deduplicates_identical_findings() {
        let temp_dir = create_temp_dir("dedupe");
        let file = temp_dir.join("secret.ts");
        let write_result = fs::write(
            &file,
            "api_key='abcdEFGH1234567890'\napi_key='abcdEFGH1234567890'\n",
        );
        assert!(write_result.is_ok());

        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let result = run_secret_check(&files, &SecretCheckConfig::default(), None);

        assert_eq!(result.findings.len(), 2);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn dedupe_preserves_distinct_matches_on_same_line() {
        let findings = vec![
            SecretFinding {
                file: "src/config.ts".to_string(),
                line: 1,
                finding_type: FindingType::Pattern,
                pattern_name: "Generic Secret".to_string(),
                redacted_match: "abcd***7890".to_string(),
                redacted_line: "const a = 'abcd***7890'; const b = 'wxyz***4321';".to_string(),
                ..Default::default()
            },
            SecretFinding {
                file: "src/config.ts".to_string(),
                line: 1,
                finding_type: FindingType::Pattern,
                pattern_name: "Generic Secret".to_string(),
                redacted_match: "wxyz***4321".to_string(),
                redacted_line: "const a = 'abcd***7890'; const b = 'wxyz***4321';".to_string(),
                ..Default::default()
            },
        ];

        let deduped = super::deduplicate_findings(findings);

        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn skips_configured_extensions() {
        let temp_dir = create_temp_dir("skip");
        let file = temp_dir.join("app.min.js");
        let write_result = fs::write(&file, "api_key='abcdEFGH1234567890'");
        assert!(write_result.is_ok());

        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let result = run_secret_check(&files, &SecretCheckConfig::default(), None);

        // SDT-006: still a clean pass. A configured exclusion is an operator
        // decision, not a coverage failure — see
        // `tests/secret_file_coverage.rs` for why blocking here is rejected.
        assert!(result.passed);
        assert_eq!(result.findings.len(), 0);
        assert_eq!(
            result.files_skipped_extension, 1,
            "the exclusion is advisory, but it is not invisible: {result:?}"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn skips_files_exceeding_size_limit() {
        let temp_dir = create_temp_dir("size-limit");
        let file = temp_dir.join("big.ts");
        // Create a file just over 1 MiB with a secret on the first line.
        let secret_line = "api_key='abcdEFGH1234567890'\n";
        let padding = "a".repeat(1024 * 1024);
        let content = format!("{secret_line}{padding}");
        fs::write(&file, &content).unwrap();

        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let result = run_secret_check(&files, &SecretCheckConfig::default(), None);

        // The file is still skipped — SDT-006 does not raise `MAX_FILE_SIZE`.
        assert_eq!(result.findings.len(), 0);
        // SDT-006 inverts the old `assert!(result.passed)`. Skipping the file
        // is correct; reporting a clean pass over a file nobody read is not.
        assert!(
            !result.passed,
            "a file dropped by the size guard must not report a clean pass: {result:?}"
        );
        assert_eq!(result.score, 0, "{result:?}");
        assert_eq!(
            result.files_skipped_oversize.len(),
            1,
            "the skipped file must be named on the result: {result:?}"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    /// SDT-006 — the SCAN-001 `catch_unwind` arm.
    ///
    /// Before this item, a panicking scan discarded the whole file and the
    /// result read "No secrets detected", `passed = true`, `score = 100`. The
    /// panic must still be contained (a bad custom regex may not tear down
    /// the run) and must now also be reported.
    ///
    /// The scan is injected because no input reachably panics the real
    /// scanner — see `run_secret_check_with_scanner`. Everything downstream of
    /// the `catch_unwind` (accounting, `coverage_notes`, scoring, message) is
    /// the production path.
    #[test]
    fn panicking_scan_is_not_a_clean_pass() {
        let temp_dir = create_temp_dir("panic-arm");
        let file = temp_dir.join("boom.ts");
        fs::write(&file, "export const x = 1;\n").unwrap();

        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];

        // SCAN-001 contains the unwind, but the default hook still prints a
        // panic banner from the rayon worker that reads like a test failure.
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = super::run_secret_check_with_scanner(
            &files,
            &SecretCheckConfig::default(),
            None,
            Vec::new(),
            |_content, _display_path| panic!("custom pattern blew up mid-scan"),
        );
        std::panic::set_hook(previous);

        assert!(
            result.findings.is_empty(),
            "the scan never completed, so there is nothing to find: {result:?}"
        );
        assert!(
            !result.passed,
            "a file whose scan crashed must not report a clean pass: {result:?}"
        );
        assert_eq!(
            result.score, 0,
            "a crashed scan has no coverage denominator: {result:?}"
        );
        assert_ne!(
            result.message, "No secrets detected",
            "the clean-result message must not survive a crashed scan: {result:?}"
        );
        assert_eq!(
            result.files_skipped_panicked.len(),
            1,
            "the crashed file must be named on the result: {result:?}"
        );
        assert!(
            result.message.contains("boom.ts"),
            "the message must name the file that provoked the crash: {}",
            result.message
        );
        assert!(
            result.message.contains("custom_patterns"),
            "the message must name the usual cause so the operator can act: {}",
            result.message
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn scans_files_under_size_limit() {
        let temp_dir = create_temp_dir("under-limit");
        let file = temp_dir.join("small.ts");
        fs::write(&file, "api_key='abcdEFGH1234567890'").unwrap();

        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let result = run_secret_check(&files, &SecretCheckConfig::default(), None);

        assert!(!result.passed, "small files with secrets should be flagged");
        assert!(!result.findings.is_empty());

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn lockfile_url_credential_is_flagged_but_integrity_hash_is_not() {
        // GH #2584: a lockfile is scanned for URL-embedded credentials only —
        // its high-entropy integrity hash is a false positive and must not be
        // flagged, but a credential in a `resolved` URL must be.
        let temp_dir = create_temp_dir("lockfile-url");
        let lock = temp_dir.join("package-lock.json");
        fs::write(
            &lock,
            "{\n  \"integrity\": \"sha512-XI5MPzVNApjAyhQzphX8BkmKsKUxD4LdyK24iZeQGinB\",\n  \
             \"resolved\": \"https://deployer:s3cr3tT0ken@npm.private.example/left-pad/-/left-pad-1.3.0.tgz\"\n}",
        )
        .unwrap();

        let lock_string = lock.to_string_lossy().to_string();
        let files = [lock_string.as_str()];
        let result = run_secret_check(&files, &SecretCheckConfig::default(), None);

        assert_eq!(
            result.findings.len(),
            1,
            "exactly the URL credential is flagged, not the integrity hash: {:#?}",
            result.findings,
        );
        assert_eq!(result.findings[0].pattern_name, "Credential URL");
        assert!(
            !result.findings[0].redacted_line.contains("s3cr3tT0ken"),
            "the credential must be redacted in the finding",
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn committed_env_secret_is_still_scanned_by_run_secret_check() {
        // The `.env` noise exemption is discovery-only. `run_secret_check`
        // backs `anvil gate`/`anvil audit`, which must still catch a secret in
        // a tracked `.env`.
        let temp_dir = create_temp_dir("env-gate");
        let env = temp_dir.join(".env");
        fs::write(&env, format!("GITHUB_TOKEN=ghp_{}", "a".repeat(36))).unwrap();

        let env_string = env.to_string_lossy().to_string();
        let files = [env_string.as_str()];
        let result = run_secret_check(&files, &SecretCheckConfig::default(), None);

        assert!(
            !result.passed && !result.findings.is_empty(),
            "a committed .env secret must still be flagged by the gate/audit path",
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn returns_findings_in_deterministic_path_order() {
        let temp_dir = create_temp_dir("ordering");
        let first = temp_dir.join("a.ts");
        let second = temp_dir.join("b.ts");
        fs::write(&first, "api_key='abcdEFGH1234567890'").unwrap();
        fs::write(&second, "api_key='abcdEFGH1234567890'").unwrap();

        let second_string = second.to_string_lossy().to_string();
        let first_string = first.to_string_lossy().to_string();
        let files = [second_string.as_str(), first_string.as_str()];
        let result = run_secret_check(&files, &SecretCheckConfig::default(), None);

        assert_eq!(result.findings.len(), 2);
        assert_eq!(result.findings[0].file, first_string);
        assert_eq!(result.findings[1].file, second_string);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn skips_files_at_exact_size_boundary() {
        let temp_dir = create_temp_dir("boundary");
        let file = temp_dir.join("exact.ts");
        // Pad to exactly MAX_FILE_SIZE bytes — should be skipped (>= limit).
        let secret = "api_key='abcdEFGH1234567890'";
        let target_len = usize::try_from(super::MAX_FILE_SIZE).unwrap();
        let padding_len = target_len.saturating_sub(secret.len());
        let content = format!("{secret}{}", "x".repeat(padding_len));
        assert_eq!(content.len(), target_len);
        fs::write(&file, &content).unwrap();

        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let result = run_secret_check(&files, &SecretCheckConfig::default(), None);

        // The boundary itself is unchanged: `>= MAX_FILE_SIZE` is skipped.
        assert_eq!(result.findings.len(), 0);
        // SDT-006 inverts the old `assert!(result.passed)` for the same reason
        // as `skips_files_exceeding_size_limit`.
        assert!(
            !result.passed,
            "a file skipped at the exact size boundary is still an unscanned file: {result:?}"
        );
        assert_eq!(
            result.files_skipped_oversize.len(),
            1,
            "the boundary file must be reported, not merely dropped: {result:?}"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    fn missing_workspace_path(label: &str) -> String {
        // Cross-platform missing path: under temp_dir, never created, so
        // `Command::current_dir` fails on Unix and Windows runners alike.
        std::env::temp_dir()
            .join(format!(
                "anvil-history-scan-missing-{label}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_nanos())
            ))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn history_scan_io_failure_is_not_a_clean_pass() {
        // When history coverage is requested but the git scan cannot run
        // (I/O / process failure), the result must not claim a clean pass.
        let config = SecretCheckConfig {
            scan_git_history: true,
            ..SecretCheckConfig::default()
        };
        let missing = missing_workspace_path("io-fail");
        let result = run_secret_check(&[], &config, Some(&missing));

        assert!(
            !result.passed,
            "failed history coverage must not report passed=true: {result:?}"
        );
        assert_eq!(
            result.score, 0,
            "incomplete history coverage must not keep a full score: {result:?}"
        );
        assert!(
            !result.history_scan_errors.is_empty(),
            "history_scan_errors must surface the failure: {result:?}"
        );
        assert!(
            result.message.to_lowercase().contains("history")
                || result
                    .history_scan_errors
                    .iter()
                    .any(|e| e.to_lowercase().contains("history")),
            "operator-facing signal must mention history: {result:?}"
        );
        assert_ne!(
            result.message, "No secrets detected",
            "must not use the clean-result message when history coverage failed"
        );
    }

    // SDT-001 — unscanned surface must not report as clean.
    //
    // The SCAN-002 per-line guard skips any line over `max_line_bytes`
    // before *both* pattern and entropy evaluation, so a secret hiding in a
    // minified line is invisible to the scan. Before SDT-001 the gate
    // ignored the counter and reported "No secrets detected", passed,
    // score 100 — a false clean. These tests pin the fail-closed contract.

    fn git(dir: &std::path::Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("git is available");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Deterministic throwaway repo (identity pinned, signing and hooks off)
    /// mirroring the `git_scanner` test harness.
    fn temp_repo(label: &str) -> PathBuf {
        let tmp = create_temp_dir(label);
        git(&tmp, &["init", "-q"]);
        git(&tmp, &["config", "user.email", "test@example.com"]);
        git(&tmp, &["config", "user.name", "Test"]);
        git(&tmp, &["config", "commit.gpgsign", "false"]);
        let empty_hooks = tmp.join("empty-hooks");
        std::fs::create_dir_all(&empty_hooks).expect("create empty hooks dir");
        git(
            &tmp,
            &["config", "core.hooksPath", &empty_hooks.to_string_lossy()],
        );
        tmp
    }

    /// A single line long enough to trip the default `max_line_bytes` guard,
    /// with a real credential shape buried inside it.
    fn oversize_secret_line() -> String {
        format!("const k = '{}ghp_{}';\n", "x".repeat(5000), "a".repeat(36))
    }

    #[test]
    fn oversize_skipped_line_is_not_a_clean_pass() {
        let temp_dir = create_temp_dir("oversize-gate");
        let file = temp_dir.join("minified.ts");
        fs::write(&file, oversize_secret_line()).unwrap();

        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let result = run_secret_check(&files, &SecretCheckConfig::default(), None);

        assert_eq!(
            result.lines_skipped_oversize, 1,
            "the guard must have skipped the oversize line: {result:?}"
        );
        assert!(
            result.findings.is_empty(),
            "the guard refuses to walk the line, so there is nothing to find: {result:?}"
        );
        assert!(
            !result.passed,
            "an unscanned line must not report a clean pass: {result:?}"
        );
        assert_eq!(
            result.score, 0,
            "incomplete coverage must not keep a full score: {result:?}"
        );
        assert_ne!(
            result.message, "No secrets detected",
            "the clean-result message must not be used when lines went unscanned"
        );
        assert!(
            result.message.contains("1 line(s) too long to scan"),
            "the message must name the count: {}",
            result.message
        );
        assert!(
            result.message.contains("max_line_bytes"),
            "the message must name the raise-the-limit remedy: {}",
            result.message
        );
        assert!(
            result.message.contains("ADR-029"),
            "the message must name the suppression remedy: {}",
            result.message
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn oversize_skip_and_history_failure_are_both_named() {
        // A result can carry both conditions at once; neither may swallow
        // the other, or the operator fixes one and still sees a red.
        let temp_dir = create_temp_dir("oversize-plus-history");
        let file = temp_dir.join("minified.ts");
        fs::write(&file, oversize_secret_line()).unwrap();

        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let config = SecretCheckConfig {
            scan_git_history: true,
            ..SecretCheckConfig::default()
        };
        let missing = missing_workspace_path("oversize-plus-history");
        let result = run_secret_check(&files, &config, Some(&missing));

        assert!(!result.passed, "{result:?}");
        assert_eq!(result.score, 0, "{result:?}");
        assert!(
            result.message.to_lowercase().contains("history"),
            "the history failure must survive composition: {}",
            result.message
        );
        assert!(
            result.message.contains("1 line(s) too long to scan"),
            "the oversize skip must survive composition: {}",
            result.message
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn history_oversize_skip_blocks_a_clean_pass() {
        // `merge_git_history_scan` folds history skips into the same total,
        // so a secret buried in an oversize line in a *commit* is covered by
        // the same gate — nothing on disk needs to be scanned for it to fire.
        let repo = temp_repo("history-oversize");
        let name = "huge.py";
        std::fs::write(repo.join(name), oversize_secret_line()).expect("write fixture");
        git(&repo, &["add", name]);
        git(&repo, &["commit", "-q", "-m", "add fixture"]);

        let config = SecretCheckConfig {
            scan_git_history: true,
            ..SecretCheckConfig::default()
        };
        let root = repo.to_string_lossy().to_string();
        let result = run_secret_check(&[], &config, Some(&root));

        assert!(
            result.history_scan_errors.is_empty(),
            "the history scan itself must succeed: {result:?}"
        );
        assert_eq!(
            result.lines_skipped_oversize, 1,
            "the history skip must reach the result: {result:?}"
        );
        assert!(
            !result.passed,
            "an unscanned history line must not report a clean pass: {result:?}"
        );
        assert_eq!(result.score, 0, "{result:?}");
        assert!(
            result.message.contains("1 line(s) too long to scan"),
            "the message must name the count: {}",
            result.message
        );

        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn history_scan_disabled_does_not_error_on_bad_workspace() {
        // History errors only apply when coverage was requested.
        let config = SecretCheckConfig {
            scan_git_history: false,
            ..SecretCheckConfig::default()
        };
        let missing = missing_workspace_path("disabled");
        let result = run_secret_check(&[], &config, Some(&missing));
        assert!(result.passed);
        assert!(result.history_scan_errors.is_empty());
        assert_eq!(result.message, "No secrets detected");
    }

    #[test]
    fn non_git_workspace_with_history_enabled_is_still_a_clean_pass() {
        // Not being a git repo is a successful empty history scan, not a failure.
        let temp_dir = create_temp_dir("nongit-history");
        let config = SecretCheckConfig {
            scan_git_history: true,
            ..SecretCheckConfig::default()
        };
        let root = temp_dir.to_string_lossy().to_string();
        let result = run_secret_check(&[], &config, Some(&root));
        assert!(
            result.passed,
            "non-git workspace should not fail history coverage: {result:?}"
        );
        assert!(result.history_scan_errors.is_empty());
        let _ = fs::remove_dir_all(temp_dir);
    }
}
