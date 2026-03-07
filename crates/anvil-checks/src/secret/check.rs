use std::fs;
use std::path::Path;

use crate::secret::git_scanner::scan_git_history;
use crate::secret::scanner::scan_content;
use crate::secret::types::{FindingType, SecretCheckConfig, SecretCheckResult, SecretFinding};

pub fn run_secret_check(
    files: &[&str],
    config: &SecretCheckConfig,
    workspace_root: Option<&str>,
) -> SecretCheckResult {
    let mut findings = Vec::new();

    for file in files {
        if should_skip_file(file, config) {
            continue;
        }

        let Ok(content) = fs::read_to_string(file) else {
            continue;
        };
        let display_path = normalise_file_path(file, workspace_root);
        let file_findings = scan_content(&content, &display_path, config);
        findings.extend(file_findings);
    }

    if config.scan_git_history {
        let root = workspace_root.unwrap_or(".");
        if let Ok(history_findings) = scan_git_history(root, config) {
            findings.extend(history_findings);
        }
    }

    let findings = deduplicate_findings(findings);
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
    }
}

fn should_skip_file(file: &str, config: &SecretCheckConfig) -> bool {
    config
        .skip_extensions
        .iter()
        .any(|extension| file.ends_with(extension))
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
                "{}:{}:{}:{}",
                finding.file, finding.line, finding_type, finding.pattern_name
            );
            seen.insert(key)
        })
        .collect()
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
    use crate::secret::types::SecretCheckConfig;

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
}
