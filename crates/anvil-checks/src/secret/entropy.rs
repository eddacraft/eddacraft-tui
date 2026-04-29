use regex::Regex;

use crate::secret::patterns::PatternMatcher;
use crate::secret::types::{FindingType, SecretCheckConfig, SecretFinding};

pub fn calculate_entropy(value: &str) -> f64 {
    if value.is_empty() {
        return 0.0;
    }

    let mut frequencies = std::collections::BTreeMap::new();
    for character in value.chars() {
        *frequencies.entry(character).or_insert(0_usize) += 1;
    }

    let Ok(len_u32) = u32::try_from(value.chars().count()) else {
        return 0.0;
    };
    let len = f64::from(len_u32);
    frequencies.values().fold(0.0, |entropy, count| {
        let Ok(count_u32) = u32::try_from(*count) else {
            return entropy;
        };
        let frequency = f64::from(count_u32) / len;
        entropy - frequency * frequency.log2()
    })
}

pub fn detect_high_entropy_strings(
    content: &str,
    file: &str,
    config: &SecretCheckConfig,
) -> Vec<SecretFinding> {
    detect_high_entropy_strings_with_limit(content, file, config, usize::MAX)
}

pub fn detect_high_entropy_strings_with_limit(
    content: &str,
    file: &str,
    config: &SecretCheckConfig,
    limit: usize,
) -> Vec<SecretFinding> {
    detect_high_entropy_strings_with_line_filter_and_limit(content, file, config, limit, |_, _| {
        true
    })
}

pub(crate) fn detect_high_entropy_strings_with_line_filter_and_limit(
    content: &str,
    file: &str,
    config: &SecretCheckConfig,
    limit: usize,
    mut include_line: impl FnMut(usize, &str) -> bool,
) -> Vec<SecretFinding> {
    if limit == 0 {
        return Vec::new();
    }
    let matcher = PatternMatcher::new(&config.custom_allowlist);
    let Ok(quoted_pattern) = Regex::new(r#"['\"]([^'\"]{16,})['\"]"#) else {
        return Vec::new();
    };
    let Ok(assignment_pattern) = Regex::new(r#"[:=]\s*['\"]?([a-zA-Z0-9_/+=-]{16,})['\"]?"#) else {
        return Vec::new();
    };

    let mut findings = Vec::new();

    for (index, line) in content.lines().enumerate() {
        let line_number = index + 1;
        if !include_line(index, line) {
            continue;
        }

        for pattern in [&quoted_pattern, &assignment_pattern] {
            let Some(captures) = pattern.captures(line) else {
                continue;
            };
            let Some(candidate_match) = captures.get(1) else {
                continue;
            };
            let candidate = candidate_match.as_str();

            if candidate.len() < config.min_entropy_length {
                continue;
            }
            if matcher.is_allowlisted(candidate) {
                continue;
            }
            if matcher.looks_like_code(candidate) {
                continue;
            }

            let entropy = calculate_entropy(candidate);
            if entropy >= config.entropy_threshold {
                findings.push(SecretFinding {
                    file: file.to_string(),
                    line: line_number,
                    finding_type: FindingType::Entropy,
                    pattern_name: "High Entropy String".to_string(),
                    redacted_match: matcher.redact_secret(candidate),
                    redacted_line: matcher.redact_range_in_line(
                        line,
                        candidate_match.start(),
                        candidate_match.end(),
                    ),
                });
                if findings.len() == limit {
                    return findings;
                }
            }
        }
    }

    findings
}

pub fn rounded_entropy(value: &str) -> f64 {
    (calculate_entropy(value) * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use crate::secret::entropy::{calculate_entropy, detect_high_entropy_strings, rounded_entropy};
    use crate::secret::types::{FindingType, SecretCheckConfig};

    #[test]
    fn calculates_known_entropy_values() {
        assert!((calculate_entropy("") - 0.0).abs() < f64::EPSILON);
        assert!((calculate_entropy("aaaa") - 0.0).abs() < f64::EPSILON);
        assert!((calculate_entropy("abcd") - 2.0).abs() < f64::EPSILON);
        assert!((rounded_entropy("abcd") - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn detects_high_entropy_from_quoted_and_assignment_values() {
        let config = SecretCheckConfig {
            entropy_threshold: 3.5,
            ..SecretCheckConfig::default()
        };
        let content = "const token = '9xY7qW2vK8mN4pR6';\napi_key: 9xY7qW2vK8mN4pR6";

        let findings = detect_high_entropy_strings(content, "src/test.ts", &config);

        assert_eq!(findings.len(), 3);
        assert_eq!(findings[0].finding_type, FindingType::Entropy);
        assert_eq!(findings[0].pattern_name, "High Entropy String");
    }

    #[test]
    fn filters_allowlisted_and_code_like_candidates() {
        let config = SecretCheckConfig {
            custom_allowlist: vec!["safe-value".to_string()],
            entropy_threshold: 3.0,
            ..SecretCheckConfig::default()
        };
        let content = "const one = 'safe-value';\nconst two = 'fetch('";

        let findings = detect_high_entropy_strings(content, "src/test.ts", &config);

        assert!(findings.is_empty());
    }
}
