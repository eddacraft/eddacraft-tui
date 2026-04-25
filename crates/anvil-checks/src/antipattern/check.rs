use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use rayon::prelude::*;

use crate::antipattern::scanner::{ScanOptions, scan_file};
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

#[must_use]
pub fn run_antipattern_check(
    files: &[&str],
    config: &AntipatternCheckConfig,
    workspace_root: Option<&str>,
) -> AntipatternCheckResult {
    let scan_options = ScanOptions {
        patterns: if config.patterns.is_empty() {
            None
        } else {
            Some(config.patterns.clone())
        },
        include_opt_in: config.include_opt_in,
    };

    // Scan files concurrently on the rayon thread pool. `filter_map` drops
    // non-scannable / unreadable files; `collect` materialises a Vec so we
    // can fold into aggregate state deterministically below.
    let per_file_results: Vec<_> = files
        .par_iter()
        .filter_map(|file_path| {
            if !is_scannable_file(file_path, config) {
                return None;
            }
            let content = fs::read_to_string(file_path).ok()?;
            let relative_path = normalise_file_path(file_path, workspace_root);
            Some(scan_file(&relative_path, &content, Some(&scan_options)))
        })
        .collect();

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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::antipattern::check::run_antipattern_check;
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
}
