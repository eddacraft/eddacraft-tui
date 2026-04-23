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
    let output = Command::new("git")
        .args([
            "log",
            "-p",
            &depth_flag,
            "--all",
            "--diff-filter=AM",
            "--",
            "*.ts",
            "*.js",
            "*.json",
            "*.env*",
            "*.yaml",
            "*.yml",
        ])
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
}
