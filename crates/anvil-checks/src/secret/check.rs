use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::secret::git_scanner::scan_git_history;
use crate::secret::patterns::compile_custom_patterns;
use crate::secret::scanner::scan_content_with_compiled_patterns;
use crate::secret::types::{FindingType, SecretCheckConfig, SecretCheckResult, SecretFinding};

/// Maximum file size to scan (1 MiB). Files of this size or larger are
/// skipped to avoid excessive memory usage on binaries or generated artefacts.
pub const MAX_FILE_SIZE: u64 = 1024 * 1024;
const _: () = assert!(MAX_FILE_SIZE == 1024 * 1024);

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

    // SCAN-001: read + scan each candidate file in parallel on the rayon
    // pool, mirroring the welcome-screen discovery shape. Per-file panics
    // are contained via `catch_unwind`; read failures and skipped files
    // simply drop out of the collected findings stream. Deterministic
    // ordering is restored downstream after dedupe via `sort_findings`.
    let lines_skipped_atomic = AtomicUsize::new(0);
    let per_file: Vec<Vec<SecretFinding>> = files
        .par_iter()
        .filter_map(|file| {
            if should_skip_file(file, config) {
                return None;
            }
            if file_exceeds_size_limit(file) {
                return None;
            }
            let content = fs::read_to_string(file).ok()?;
            let display_path = normalise_file_path(file, workspace_root);
            // SCAN-001: contain panics from custom user regexes so a
            // single bad pattern can't tear down the whole secret scan.
            let scan_result = catch_unwind(AssertUnwindSafe(|| {
                scan_content_with_compiled_patterns(
                    &content,
                    &display_path,
                    config,
                    &compiled_custom_patterns,
                    usize::MAX,
                )
            }));
            match scan_result {
                Ok((file_findings, stats)) => {
                    lines_skipped_atomic.fetch_add(stats.lines_skipped_oversize, Ordering::Relaxed);
                    Some(file_findings)
                }
                Err(_) => None,
            }
        })
        .collect();

    let mut findings: Vec<SecretFinding> = per_file.into_iter().flatten().collect();
    let mut lines_skipped_oversize = lines_skipped_atomic.load(Ordering::Relaxed);

    if config.scan_git_history {
        let root = workspace_root.unwrap_or(".");
        if let Ok(history) = scan_git_history(root, config) {
            findings.extend(history.findings);
            // Fold the history scan's oversize-line skips into the same total
            // so a "0 findings" result isn't silently hiding skipped content.
            lines_skipped_oversize += history.lines_skipped_oversize;
            // history.pattern_errors are duplicates of the file-scan errors
            // (same custom_patterns input compiled twice) — already captured.
        }
    }

    let findings = sort_findings(deduplicate_findings(findings));
    let passed = findings.is_empty();
    let pattern_count = findings
        .iter()
        .filter(|finding| finding.finding_type == FindingType::Pattern)
        .count();
    let entropy_count = findings
        .iter()
        .filter(|finding| finding.finding_type == FindingType::Entropy)
        .count();
    let score_usize = 100_usize.saturating_sub(findings.len().saturating_mul(10));
    let score = u8::try_from(score_usize).unwrap_or(0);

    let message = if passed {
        "No secrets detected".to_string()
    } else {
        let mut parts = Vec::new();
        if pattern_count > 0 {
            parts.push(format!("{pattern_count} pattern match(es)"));
        }
        if entropy_count > 0 {
            parts.push(format!("{entropy_count} high-entropy string(s)"));
        }
        format!(
            "Found {} potential secret(s): {}",
            findings.len(),
            parts.join(", ")
        )
    };

    SecretCheckResult {
        passed,
        score,
        message,
        findings,
        pattern_errors,
        lines_skipped_oversize,
    }
}

fn should_skip_file(file: &str, config: &SecretCheckConfig) -> bool {
    // Dependency lockfiles carry high-entropy integrity hashes that are false
    // positives for secret detection (GH #2584). Most are not caught by the
    // `.lock` suffix in `skip_extensions` (e.g. `package-lock.json`,
    // `pnpm-lock.yaml`, `go.sum`), so match them by basename too.
    //
    // `.env*` files are deliberately NOT skipped here. `run_secret_check` also
    // backs `anvil gate` (the `secret-detection` guardrail) and `anvil audit`,
    // whose job is to catch a *committed* secret — including one in a tracked
    // `.env`. The `.env` noise that GH #2584 reports is from the first-run
    // discovery scan, so that exemption lives there (`welcome::candidate_path`)
    // and must not blind the gate/audit secret check.
    if crate::filter::is_lockfile(Path::new(file)) {
        return true;
    }
    config
        .skip_extensions
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
            },
            SecretFinding {
                file: "src/config.ts".to_string(),
                line: 1,
                finding_type: FindingType::Pattern,
                pattern_name: "Generic Secret".to_string(),
                redacted_match: "wxyz***4321".to_string(),
                redacted_line: "const a = 'abcd***7890'; const b = 'wxyz***4321';".to_string(),
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

        assert!(result.passed);
        assert_eq!(result.findings.len(), 0);

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

        assert!(result.passed, "large files should be skipped entirely");
        assert_eq!(result.findings.len(), 0);

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
    fn skips_lockfiles_but_still_scans_committed_env() {
        // GH #2584: a lockfile's integrity hashes are false positives, so
        // `run_secret_check` skips lockfiles. A `.env` is NOT skipped here —
        // `run_secret_check` backs `anvil gate`/`anvil audit`, whose job is to
        // catch a committed secret (incl. one in a tracked `.env`). The
        // discovery-scan `.env` noise exemption lives in welcome, not here.
        let temp_dir = create_temp_dir("exempt");
        let lock = temp_dir.join("package-lock.json");
        let env = temp_dir.join(".env");
        fs::write(
            &lock,
            "{\"integrity\":\"sha512-XI5MPzVNApjAyhQzphX8BkmKsKUxD4LdyK24iZeQGinB\"}",
        )
        .unwrap();
        fs::write(&env, format!("GITHUB_TOKEN=ghp_{}", "a".repeat(36))).unwrap();

        let lock_string = lock.to_string_lossy().to_string();
        let env_string = env.to_string_lossy().to_string();
        let files = [lock_string.as_str(), env_string.as_str()];
        let result = run_secret_check(&files, &SecretCheckConfig::default(), None);

        // The lockfile contributes nothing; the committed .env secret is flagged.
        assert_eq!(
            result.findings.len(),
            1,
            "lockfile skipped, committed .env still scanned: {:#?}",
            result.findings,
        );
        assert_eq!(
            std::path::Path::new(&result.findings[0].file).file_name(),
            Some(std::ffi::OsStr::new(".env")),
            "the surviving finding is the committed .env secret, not the lockfile",
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

        assert!(
            result.passed,
            "file at exact size boundary should be skipped"
        );
        assert_eq!(result.findings.len(), 0);

        let _ = fs::remove_dir_all(temp_dir);
    }
}
