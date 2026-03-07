use std::io;
use std::process::Command;

use crate::secret::patterns::{PatternMatcher, compile_secret_patterns};
use crate::secret::types::{FindingType, SecretCheckConfig, SecretFinding};

pub fn scan_git_history(
    workspace_root: &str,
    config: &SecretCheckConfig,
) -> Result<Vec<SecretFinding>, io::Error> {
    let matcher = PatternMatcher::new(&config.custom_allowlist);
    let patterns = compile_secret_patterns(&config.custom_patterns);
    let depth = config.git_history_depth.clamp(1, 1000);

    let git_check = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(workspace_root)
        .output()?;

    if !git_check.status.success() {
        return Ok(Vec::new());
    }

    let depth_flag = format!("-{depth}");
    let output = Command::new("git")
        .args([
            "log",
            "-p",
            &depth_flag,
            "--all",
            "--diff-filter=A",
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
        return Ok(Vec::new());
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
        for pattern in &patterns {
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

    Ok(findings)
}

#[cfg(test)]
mod tests {
    use crate::secret::types::SecretCheckConfig;

    #[test]
    fn clamps_depth() {
        let config = SecretCheckConfig {
            git_history_depth: 0,
            ..SecretCheckConfig::default()
        };

        let depth = config.git_history_depth.clamp(1, 1000);
        assert_eq!(depth, 1);
    }
}
