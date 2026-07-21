use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use rayon::prelude::*;

use crate::antipattern::scanner::{ScanOptions, ScanResult, scan_file};
use crate::antipattern::types::{
    AntipatternCheckConfig, AntipatternCheckResult, WarningResult, WarningSeverity,
    create_warning_result,
};

const ERROR_PENALTY: usize = 15;
const WARNING_PENALTY: usize = 5;
const INFO_PENALTY: usize = 1;

fn severity_level(severity: WarningSeverity) -> usize {
    match severity {
        WarningSeverity::Error => 3,
        WarningSeverity::Warning => 2,
        WarningSeverity::Info => 1,
    }
}

fn severity_penalty(severity: WarningSeverity) -> usize {
    match severity {
        WarningSeverity::Error => ERROR_PENALTY,
        WarningSeverity::Warning => WARNING_PENALTY,
        WarningSeverity::Info => INFO_PENALTY,
    }
}

fn build_message(result: &WarningResult, files_scanned: usize, passed: bool) -> String {
    if result.warnings.is_empty() {
        return format!(
            "Anti-pattern check passed: {files_scanned} files scanned, no issues found"
        );
    }

    let mut parts = Vec::new();
    if result.summary.errors > 0 {
        parts.push(format!(
            "{} error{}",
            result.summary.errors,
            if result.summary.errors > 1 { "s" } else { "" }
        ));
    }
    if result.summary.warnings > 0 {
        parts.push(format!(
            "{} warning{}",
            result.summary.warnings,
            if result.summary.warnings > 1 { "s" } else { "" }
        ));
    }
    if result.summary.info > 0 {
        parts.push(format!("{} info", result.summary.info));
    }
    if result.summary.suppressed > 0 {
        parts.push(format!("{} suppressed", result.summary.suppressed));
    }

    let status = if passed {
        "passed with issues"
    } else {
        "failed"
    };
    format!(
        "Anti-pattern check {status}: {} ({files_scanned} files scanned)",
        parts.join(", ")
    )
}

fn is_scannable_file(file_path: &str, config: &AntipatternCheckConfig) -> bool {
    config
        .extensions
        .iter()
        .any(|extension| file_path.ends_with(extension))
}

fn normalise_file_path(file_path: &str, workspace_root: Option<&str>) -> String {
    let Some(root) = workspace_root else {
        return file_path.to_string();
    };

    let file = Path::new(file_path);
    let root_path = Path::new(root);
    if let Ok(relative) = file.strip_prefix(root_path) {
        relative.to_string_lossy().replace('\\', "/")
    } else {
        file_path.to_string()
    }
}

fn scan_options_from_config(config: &AntipatternCheckConfig) -> ScanOptions {
    ScanOptions {
        patterns: if config.patterns.is_empty() {
            None
        } else {
            Some(config.patterns.clone())
        },
        include_opt_in: config.include_opt_in,
    }
}

/// Fold per-file scan results into the aggregate check verdict (warnings,
/// score, pass/fail, message). Shared by both the disk-reading wrapper and the
/// bytes core so the two cannot diverge.
fn aggregate_scan_results(
    per_file_results: Vec<ScanResult>,
    config: &AntipatternCheckConfig,
) -> AntipatternCheckResult {
    let files_scanned = per_file_results.len();
    let mut all_warnings = Vec::new();
    let mut all_patterns_checked = BTreeSet::new();
    for result in per_file_results {
        all_warnings.extend(result.warnings);
        all_patterns_checked.extend(result.patterns_checked);
    }

    let pattern_ids = all_patterns_checked.into_iter().collect::<Vec<_>>();
    let warning_result = create_warning_result(all_warnings, pattern_ids.clone());

    let threshold = severity_level(config.severity_threshold);
    let mut total_penalty = 0_usize;
    let mut has_blocking_warning = false;

    for warning in &warning_result.warnings {
        if warning.suppressed.is_some() {
            continue;
        }

        if severity_level(warning.severity) >= threshold {
            has_blocking_warning = true;
        }
        total_penalty = total_penalty.saturating_add(severity_penalty(warning.severity));
    }

    let score_usize = 100_usize.saturating_sub(total_penalty);
    let score = u8::try_from(score_usize).unwrap_or(0);
    let passed = !has_blocking_warning;
    let message = build_message(&warning_result, files_scanned, passed);

    AntipatternCheckResult {
        passed,
        score,
        message,
        warnings: warning_result,
        files_scanned,
        patterns_checked: pattern_ids,
    }
}

/// Lazily-built default rayon pool for the disk-reading [`run_antipattern_check`]
/// wrapper. The daemon (DSV-005) supplies its own interactive pool to
/// [`run_antipattern_check_bytes`]; everyone else shares this one.
fn default_pool() -> &'static rayon::ThreadPool {
    static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .build()
            .expect("build default antipattern rayon pool")
    })
}

/// Run the anti-pattern check over already-read file bytes, on an injected
/// rayon pool. This is the core the daemon's save-time `validate_paths` path
/// (DSV-005 Task 8) calls with the openat2-guarded bytes it already read
/// (Task 3) and its interactive pool (DSV-006/Task 10): it never re-opens a
/// file (closing the TOCTOU window a re-read would reopen) and never bleeds
/// onto the global rayon pool. `files` pairs each path with its content bytes.
///
/// Bytes are decoded with [`String::from_utf8_lossy`]; the scanner is a
/// text-pattern matcher, and the disk wrapper only ever supplies valid UTF-8
/// (it reads via `read_to_string`), so the lossy path is exercised only by
/// callers that knowingly hand non-UTF-8 bytes.
#[must_use]
pub fn run_antipattern_check_bytes(
    files: &[(&str, &[u8])],
    config: &AntipatternCheckConfig,
    workspace_root: Option<&str>,
    pool: &rayon::ThreadPool,
) -> AntipatternCheckResult {
    let scan_options = scan_options_from_config(config);

    // Scan supplied bytes concurrently on the *injected* pool. `filter_map`
    // drops non-scannable files; the bytes are already in hand, so no file is
    // re-opened here.
    let per_file_results: Vec<ScanResult> = pool.install(|| {
        files
            .par_iter()
            .filter_map(|(file_path, bytes)| {
                if !is_scannable_file(file_path, config) {
                    return None;
                }
                // CIB-199: machine-generated files (TanStack Router's
                // `routeTree.gen.ts`, protobuf/GraphQL codegen output) are
                // overwritten on regeneration and ship blanket suppression
                // headers by construction. Anti-pattern findings on them are
                // unactionable, so skip them. Secret detection and other gate
                // engines walk independently and are unaffected. The cheap
                // path check runs first so a path-identified generated file is
                // never UTF-8 decoded.
                if crate::antipattern::generated::is_generated_path(file_path) {
                    return None;
                }
                let content = String::from_utf8_lossy(bytes);
                if crate::antipattern::generated::has_generated_banner(&content) {
                    return None;
                }
                let relative_path = normalise_file_path(file_path, workspace_root);
                Some(scan_file(&relative_path, &content, Some(&scan_options)))
            })
            .collect()
    });

    aggregate_scan_results(per_file_results, config)
}

/// Run the anti-pattern check by reading each path from disk.
///
/// Thin wrapper over [`run_antipattern_check_bytes`]: it reads each scannable
/// file via `fs::read_to_string` (skipping unreadable / non-UTF-8 files, as
/// before) on the [`default_pool`], then delegates. The disk-reading CLI
/// surfaces have no openat2 guard and legitimately read from cwd, so they keep
/// this entry point; only the save-time daemon uses the bytes core.
#[must_use]
pub fn run_antipattern_check(
    files: &[&str],
    config: &AntipatternCheckConfig,
    workspace_root: Option<&str>,
) -> AntipatternCheckResult {
    let owned: Vec<(&str, Vec<u8>)> = files
        .iter()
        .filter(|file_path| is_scannable_file(file_path, config))
        .filter_map(|&file_path| {
            fs::read_to_string(file_path)
                .ok()
                .map(|content| (file_path, content.into_bytes()))
        })
        .collect();
    let borrowed: Vec<(&str, &[u8])> = owned
        .iter()
        .map(|(path, bytes)| (*path, bytes.as_slice()))
        .collect();

    run_antipattern_check_bytes(&borrowed, config, workspace_root, default_pool())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use crate::antipattern::check::{run_antipattern_check, run_antipattern_check_bytes};
    use crate::antipattern::types::{AntipatternCheckConfig, WarningSeverity};

    fn create_temp_dir(name: &str) -> PathBuf {
        let base = std::env::temp_dir();
        let unique = format!(
            "anvil-antipattern-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        );
        let path = base.join(unique);
        let _ = fs::create_dir_all(&path);
        path
    }

    #[test]
    fn passes_with_no_issues_message() {
        let temp_dir = create_temp_dir("clean");
        let file = temp_dir.join("clean.ts");
        let write_result = fs::write(&file, "const value = 1;");
        assert!(write_result.is_ok());

        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let result = run_antipattern_check(&files, &AntipatternCheckConfig::default(), None);

        assert!(result.passed);
        assert_eq!(result.score, 100);
        assert_eq!(
            result.message,
            "Anti-pattern check passed: 1 files scanned, no issues found"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn warning_threshold_fails_on_warning() {
        let temp_dir = create_temp_dir("threshold");
        let file = temp_dir.join("warn.ts");
        let write_result = fs::write(&file, "const value: any = source;");
        assert!(write_result.is_ok());

        let config = AntipatternCheckConfig {
            severity_threshold: WarningSeverity::Warning,
            ..AntipatternCheckConfig::default()
        };
        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let result = run_antipattern_check(&files, &config, None);

        assert!(!result.passed);
        assert_eq!(result.score, 95);
        assert_eq!(result.warnings.summary.warnings, 1);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn generated_file_by_path_is_skipped_under_warning_threshold() {
        // CIB-199: a committed generated file (`*.gen.ts`) carrying the exact
        // blanket `eslint-disable` that trips AP-001 must not fail the gate,
        // even with warnings promoted to blocking.
        let temp_dir = create_temp_dir("generated-path");
        let file = temp_dir.join("routeTree.gen.ts");
        let write_result = fs::write(&file, "/* eslint-disable */\nconst value = 1;\n");
        assert!(write_result.is_ok());

        let config = AntipatternCheckConfig {
            severity_threshold: WarningSeverity::Warning,
            ..AntipatternCheckConfig::default()
        };
        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let result = run_antipattern_check(&files, &config, None);

        assert!(result.passed, "generated file must not fail the gate");
        assert_eq!(result.warnings.summary.warnings, 0);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn same_content_at_hand_written_path_still_warns() {
        // Control for `generated_file_by_path_is_skipped...`: identical content
        // at a normal path must still be flagged, proving the exclusion — not a
        // broken trigger — is what suppresses the finding.
        let temp_dir = create_temp_dir("generated-control");
        let file = temp_dir.join("routes.ts");
        let write_result = fs::write(&file, "/* eslint-disable */\nconst value = 1;\n");
        assert!(write_result.is_ok());

        let config = AntipatternCheckConfig {
            severity_threshold: WarningSeverity::Warning,
            ..AntipatternCheckConfig::default()
        };
        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let result = run_antipattern_check(&files, &config, None);

        assert!(!result.passed);
        assert_eq!(result.warnings.summary.warnings, 1);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn generated_file_by_banner_is_skipped() {
        // Plain path, but a generator attribution banner in the header.
        let temp_dir = create_temp_dir("generated-banner");
        let file = temp_dir.join("client.ts");
        let write_result = fs::write(
            &file,
            "// Code generated by openapi-typescript. DO NOT EDIT.\n/* eslint-disable */\nconst value = 1;\n",
        );
        assert!(write_result.is_ok());

        let config = AntipatternCheckConfig {
            severity_threshold: WarningSeverity::Warning,
            ..AntipatternCheckConfig::default()
        };
        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let result = run_antipattern_check(&files, &config, None);

        assert!(result.passed);
        assert_eq!(result.warnings.summary.warnings, 0);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn default_error_threshold_passes_with_warning_issues() {
        let temp_dir = create_temp_dir("pass-with-issues");
        let file = temp_dir.join("warn.ts");
        let write_result = fs::write(&file, "const value: any = source;");
        assert!(write_result.is_ok());

        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let result = run_antipattern_check(&files, &AntipatternCheckConfig::default(), None);

        assert!(result.passed);
        assert_eq!(result.score, 95);
        assert_eq!(
            result.message,
            "Anti-pattern check passed with issues: 1 warning (1 files scanned)"
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn suppressed_warnings_do_not_affect_score_or_fail() {
        let temp_dir = create_temp_dir("suppressed");
        let file = temp_dir.join("suppress.ts");
        let write_result = fs::write(
            &file,
            "// @anvil-ignore AP-003 -- legacy bridge\nconst value: any = source;",
        );
        assert!(write_result.is_ok());

        let config = AntipatternCheckConfig {
            severity_threshold: WarningSeverity::Warning,
            ..AntipatternCheckConfig::default()
        };
        let file_string = file.to_string_lossy().to_string();
        let files = [file_string.as_str()];
        let result = run_antipattern_check(&files, &config, None);

        assert!(result.passed);
        assert_eq!(result.score, 100);
        assert_eq!(result.warnings.summary.suppressed, 1);
        assert_eq!(result.warnings.summary.warnings, 0);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn extension_filtering_and_pattern_tracking_work() {
        let temp_dir = create_temp_dir("extensions");
        let ts_file = temp_dir.join("a.ts");
        let html_file = temp_dir.join("b.html");
        let txt_file = temp_dir.join("c.txt");

        assert!(fs::write(&ts_file, "const x: any = source;").is_ok());
        assert!(fs::write(&html_file, "<div style=\"color:red\"></div>").is_ok());
        assert!(fs::write(&txt_file, "const y: any = source;").is_ok());

        let config = AntipatternCheckConfig {
            include_opt_in: true,
            ..AntipatternCheckConfig::default()
        };
        let ts_string = ts_file.to_string_lossy().to_string();
        let html_string = html_file.to_string_lossy().to_string();
        let txt_string = txt_file.to_string_lossy().to_string();
        let files = [
            ts_string.as_str(),
            html_string.as_str(),
            txt_string.as_str(),
        ];
        let result = run_antipattern_check(&files, &config, None);

        // .ts and .html are scannable extensions; .txt is not. HTML/CSS rules
        // were retired in RSCAN-004 so only the .ts file produces warnings.
        assert_eq!(result.files_scanned, 2);
        assert!(result.patterns_checked.contains(&"AP-001".to_string()));
        assert!(result.patterns_checked.contains(&"AP-003".to_string()));
        assert!(
            result
                .warnings
                .warnings
                .iter()
                .any(|warning| warning.id == "AP-003")
        );

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn run_antipattern_check_bytes_scans_supplied_bytes_not_disk() {
        // `phantom.ts` does not exist on disk: if the core re-read the path it
        // would find nothing (files_scanned == 0). It must scan the supplied
        // bytes instead, so the `any` anti-pattern is found in a 1-file scan.
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .expect("test pool");
        let bytes: &[u8] = b"const value: any = source;";
        let files: [(&str, &[u8]); 1] = [("phantom.ts", bytes)];

        let config = AntipatternCheckConfig {
            severity_threshold: WarningSeverity::Warning,
            ..AntipatternCheckConfig::default()
        };
        let result = run_antipattern_check_bytes(&files, &config, None, &pool);

        assert_eq!(
            result.files_scanned, 1,
            "supplied bytes were scanned even though the path does not exist on disk"
        );
        assert_eq!(
            result.warnings.summary.warnings, 1,
            "the anti-pattern in the supplied bytes was detected"
        );
    }

    #[test]
    fn run_antipattern_check_bytes_runs_on_supplied_pool() {
        // A pool whose workers flip a flag on start. After a scan that does real
        // per-file work, the flag proves the *supplied* pool's threads ran the
        // work (rather than the global pool).
        let used = Arc::new(AtomicBool::new(false));
        let used_in_handler = Arc::clone(&used);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .start_handler(move |_| used_in_handler.store(true, Ordering::SeqCst))
            .build()
            .expect("test pool");

        let bytes: &[u8] = b"const value: any = source;";
        let files: [(&str, &[u8]); 1] = [("a.ts", bytes)];
        let _ =
            run_antipattern_check_bytes(&files, &AntipatternCheckConfig::default(), None, &pool);

        assert!(
            used.load(Ordering::SeqCst),
            "the scan must execute on the injected pool, not the global one"
        );
    }
}
