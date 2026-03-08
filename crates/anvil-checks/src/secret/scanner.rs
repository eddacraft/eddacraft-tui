use crate::secret::entropy::detect_high_entropy_strings;
use crate::secret::patterns::{PatternMatcher, compile_secret_patterns};
use crate::secret::types::{FindingType, SecretCheckConfig, SecretFinding};

pub fn scan_content(
    content: &str,
    file_path: &str,
    config: &SecretCheckConfig,
) -> Vec<SecretFinding> {
    let matcher = PatternMatcher::new(&config.custom_allowlist);
    let patterns = compile_secret_patterns(&config.custom_patterns);
    let mut findings = Vec::new();

    for (index, line) in content.lines().enumerate() {
        let line_number = index + 1;

        for pattern in &patterns {
            let maybe_match = pattern.regex.find(line);
            let Some(matched_range) = maybe_match else {
                continue;
            };
            let matched_value = matched_range.as_str();

            if matcher.is_allowlisted(matched_value) {
                continue;
            }
            if matcher.looks_like_code(matched_value) {
                continue;
            }

            findings.push(SecretFinding {
                file: file_path.to_string(),
                line: line_number,
                finding_type: FindingType::Pattern,
                pattern_name: pattern.name.clone(),
                redacted_match: matcher.redact_secret(matched_value),
                redacted_line: matcher.redact_range_in_line(
                    line,
                    matched_range.start(),
                    matched_range.end(),
                ),
            });
        }
    }

    if config.enable_entropy {
        let lines: Vec<&str> = content.lines().collect();
        let entropy_findings = detect_high_entropy_strings(content, file_path, config);
        let new_entropy_findings = entropy_findings
            .into_iter()
            .filter(|finding| {
                let line_index = finding.line.saturating_sub(1);
                lines.get(line_index).is_some_and(|line| {
                    patterns.iter().all(|pattern| !pattern.regex.is_match(line))
                })
            })
            .collect::<Vec<_>>();
        findings.extend(new_entropy_findings);
    }

    findings
}

#[cfg(test)]
mod tests {
    use crate::secret::scanner::scan_content;
    use crate::secret::types::{FindingType, SecretCheckConfig};

    #[test]
    fn scans_patterns_and_entropy_together() {
        let config = SecretCheckConfig {
            entropy_threshold: 3.5,
            ..SecretCheckConfig::default()
        };
        let content = "api_key='abcdEFGH1234567890'\nconst token = '9xY7qW2vK8mN4pR6'";

        let findings = scan_content(content, "src/test.ts", &config);

        assert_eq!(findings.len(), 3);
        assert!(
            findings
                .iter()
                .any(|f| f.finding_type == FindingType::Pattern)
        );
        assert!(
            findings
                .iter()
                .any(|f| f.finding_type == FindingType::Entropy)
        );
    }

    #[test]
    fn filters_entropy_when_pattern_already_detected_on_line() {
        let config = SecretCheckConfig {
            entropy_threshold: 2.0,
            ..SecretCheckConfig::default()
        };
        let content = "password='9xY7qW2vK8mN4pR6'";

        let findings = scan_content(content, "src/test.ts", &config);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].finding_type, FindingType::Pattern);
    }

    #[test]
    fn redacts_unquoted_pattern_match_in_output_line() {
        let config = SecretCheckConfig::default();
        let github_token = format!("ghp_{}{}", "a".repeat(20), "b".repeat(20));
        let content = format!("const token = {github_token};");

        let findings = scan_content(&content, "src/test.ts", &config);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].finding_type, FindingType::Pattern);
        assert!(findings[0].redacted_line.contains("[REDACTED]"));
        assert!(!findings[0].redacted_line.contains(&github_token));
    }
}
