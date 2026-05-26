use std::io;
use std::process::Command;

use crate::secret::patterns::{DEFAULT_COMPILED_PATTERNS, PatternMatcher, compile_custom_patterns};
use crate::secret::types::{FindingType, SecretCheckConfig, SecretFinding};

/// Output of a git-history secret scan.
///
/// `pattern_errors` holds compile failures for `config.custom_patterns` so the
/// caller can surface them — silently dropped errors mean a misconfigured
/// custom pattern produces zero matches with no user-visible signal.
pub struct GitScanOutput {
    pub findings: Vec<SecretFinding>,
    pub pattern_errors: Vec<String>,
}

pub fn scan_git_history(
    workspace_root: &str,
    config: &SecretCheckConfig,
) -> Result<GitScanOutput, io::Error> {
    let matcher = PatternMatcher::new(&config.custom_allowlist);
    let (custom_patterns, pattern_errors) = compile_custom_patterns(&config.custom_patterns);
    let depth = config.git_history_depth.clamp(1, 1000);

    let git_check = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(workspace_root)
        .output()?;

    if !git_check.status.success() {
        return Ok(GitScanOutput {
            findings: Vec::new(),
            pattern_errors,
        });
    }

    let depth_flag = format!("-{depth}");
    // Match the on-disk scan's coverage model: scan every changed file in
    // history and exclude only the `skip_extensions` denylist (lockfiles,
    // minified bundles, binary assets), rather than the old narrow
    // JS/TS/JSON/YAML/env allowlist (EAMIG-004). The exclusions are derived
    // from the same `skip_extensions` the working-tree scan uses, so the two
    // surfaces stay in lockstep. `.` is the positive pathspec required when
    // every other pathspec is an exclusion.
    let mut args = vec![
        "log".to_string(),
        "-p".to_string(),
        depth_flag,
        "--all".to_string(),
        "--diff-filter=AM".to_string(),
        "--".to_string(),
        ".".to_string(),
    ];
    for extension in &config.skip_extensions {
        // Git pathspec globs match across `/`, so `*<ext>` excludes the
        // extension in any directory — the same suffix match the on-disk
        // `should_skip_file` (`file.ends_with(extension)`) applies.
        args.push(format!(":(exclude)*{extension}"));
    }
    let output = Command::new("git")
        .args(&args)
        .current_dir(workspace_root)
        .output()?;

    if !output.status.success() {
        return Ok(GitScanOutput {
            findings: Vec::new(),
            pattern_errors,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut findings = Vec::new();
    let mut commit_hash = String::from("unknown");
    let mut current_file = String::from("git-history:unknown");

    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("commit ") {
            commit_hash = rest.chars().take(8).collect::<String>();
            current_file = format!("git-history:{commit_hash}");
            continue;
        }

        if let Some(rest) = line.strip_prefix("diff --git ") {
            let parts = rest.split_whitespace().collect::<Vec<_>>();
            if parts.len() >= 2 {
                let raw_path = parts[1].strip_prefix("b/").unwrap_or(parts[1]);
                current_file = raw_path.to_string();
            }
            continue;
        }

        if !line.starts_with('+') || line.starts_with("+++") {
            continue;
        }

        let line_content = &line[1..];
        // SCAN-002 per-line guard, mirrored from the on-disk scan: skip
        // pathologically long lines (minified/base64/concatenated) before any
        // regex runs, so the now-broader history coverage can't open a ReDoS
        // backtracking window across the built-in and custom patterns.
        if line_content.len() > config.max_line_bytes {
            continue;
        }
        for pattern in DEFAULT_COMPILED_PATTERNS
            .iter()
            .chain(custom_patterns.iter())
        {
            let maybe_match = pattern.regex.find(line_content);
            let matched_value = match maybe_match {
                Some(match_result) => match_result.as_str(),
                None => continue,
            };

            if matcher.is_allowlisted(matched_value) {
                continue;
            }

            findings.push(SecretFinding {
                file: if current_file == "git-history:unknown" {
                    format!("git-history:{commit_hash}")
                } else {
                    current_file.clone()
                },
                line: 0,
                finding_type: FindingType::Pattern,
                pattern_name: format!("{} (in git history)", pattern.name),
                redacted_match: matcher.redact_secret(matched_value),
                redacted_line: matcher.redact_line(line_content.trim()),
            });
        }
    }

    Ok(GitScanOutput {
        findings,
        pattern_errors,
    })
}

#[cfg(test)]
mod tests {
    use super::scan_git_history;
    use crate::secret::types::{SecretCheckConfig, SecretPatternDef};

    #[test]
    fn clamps_depth() {
        let config = SecretCheckConfig {
            git_history_depth: 0,
            ..SecretCheckConfig::default()
        };

        let depth = config.git_history_depth.clamp(1, 1000);
        assert_eq!(depth, 1);
    }

    #[test]
    fn surfaces_invalid_custom_pattern_errors_in_git_scan() {
        // Run against a non-git directory so the early-return path triggers
        // — but pattern_errors must still be populated so callers can see
        // the misconfiguration even when no commits are scanned.
        let tmp = std::env::temp_dir().join(format!(
            "anvil-git-scan-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&tmp).expect("should create temporary test directory");
        let config = SecretCheckConfig {
            scan_git_history: true,
            custom_patterns: vec![SecretPatternDef {
                name: "broken-rule".to_string(),
                pattern: "(unclosed".to_string(),
            }],
            ..SecretCheckConfig::default()
        };

        let output = scan_git_history(&tmp.to_string_lossy(), &config).expect("scan returns Ok");

        assert!(output.findings.is_empty(), "non-git dir yields no findings");
        assert_eq!(
            output.pattern_errors.len(),
            1,
            "pattern_errors is populated even on early-return path"
        );
        assert!(
            output.pattern_errors[0].contains("'broken-rule'"),
            "error names the offending pattern: {}",
            output.pattern_errors[0]
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // A valid-shaped AWS access key id (matches the built-in `AKIA[0-9A-Z]{16}`
    // pattern) used as the secret needle across the git-coverage tests. Avoids
    // the `EXAMPLE`/`test`/`sample` keyword-allowlist markers so the finding is
    // not suppressed — the tests exercise extension coverage, not allowlisting.
    const AWS_KEY: &str = "AKIA1B2C3D4E5F6G7H8J";

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

    fn temp_repo() -> std::path::PathBuf {
        let tmp = std::env::temp_dir().join(format!(
            "anvil-gitscan-repo-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&tmp).expect("create temp repo dir");
        git(&tmp, &["init", "-q"]);
        // Pin identity + disable signing/hooks so the commit is deterministic
        // regardless of the host's global git config.
        git(&tmp, &["config", "user.email", "test@example.com"]);
        git(&tmp, &["config", "user.name", "Test"]);
        git(&tmp, &["config", "commit.gpgsign", "false"]);
        git(&tmp, &["config", "core.hooksPath", "/dev/null"]);
        tmp
    }

    fn commit_file(dir: &std::path::Path, name: &str, content: &str) {
        std::fs::write(dir.join(name), content).expect("write file");
        git(dir, &["add", name]);
        git(dir, &["commit", "-q", "-m", "add fixture"]);
    }

    fn scan(dir: &std::path::Path) -> Vec<crate::secret::types::SecretFinding> {
        let config = SecretCheckConfig {
            scan_git_history: true,
            ..SecretCheckConfig::default()
        };
        scan_git_history(&dir.to_string_lossy(), &config)
            .expect("scan returns Ok")
            .findings
    }

    #[test]
    fn scans_extensions_outside_the_old_allowlist() {
        // `.py` and `.rs` were not in the old JS/TS/JSON/YAML/env allowlist;
        // EAMIG-004 brings git-history coverage to parity with on-disk scanning.
        let repo = temp_repo();
        commit_file(&repo, "config.py", &format!("API_KEY = \"{AWS_KEY}\"\n"));
        commit_file(
            &repo,
            "main.rs",
            &format!("const K: &str = \"{AWS_KEY}\";\n"),
        );

        let findings = scan(&repo);
        assert!(
            findings.iter().any(|f| f.file.ends_with("config.py")),
            "python file should be scanned: {findings:?}"
        );
        assert!(
            findings.iter().any(|f| f.file.ends_with("main.rs")),
            "rust file should be scanned: {findings:?}"
        );

        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn excludes_skip_extension_files() {
        // A secret committed only into a skip-list extension (`.lock`) must not
        // surface — the git pathspec exclusion mirrors on-disk `should_skip_file`.
        let repo = temp_repo();
        commit_file(&repo, "deps.lock", &format!("token = {AWS_KEY}\n"));

        let findings = scan(&repo);
        assert!(
            findings.is_empty(),
            "skip-extension file must be excluded from git history scan: {findings:?}"
        );

        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn skips_oversize_lines() {
        // A secret buried in a line past the per-line byte guard is skipped
        // before any regex runs (mirrors the on-disk SCAN-002 guard).
        let repo = temp_repo();
        let long_line = format!("x = \"{}{AWS_KEY}\"\n", "a".repeat(5000));
        commit_file(&repo, "huge.py", &long_line);

        let findings = scan(&repo);
        assert!(
            findings.is_empty(),
            "oversize line should be guarded out of the regex path: {findings:?}"
        );

        let _ = std::fs::remove_dir_all(&repo);
    }
}
