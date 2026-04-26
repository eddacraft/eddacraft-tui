use crate::secret::entropy::detect_high_entropy_strings;
use crate::secret::patterns::{
    CompiledPattern, DEFAULT_COMPILED_PATTERNS, PatternMatcher, compile_custom_patterns,
};
use crate::secret::types::{FindingType, SecretCheckConfig, SecretFinding};

/// SCAN-002: per-call stats returned alongside findings. Currently exposes
/// the count of lines that exceeded `SecretCheckConfig::max_line_bytes` and
/// were therefore skipped before regex evaluation.
#[derive(Debug, Default, Clone, Copy)]
pub struct ScanStats {
    /// Number of lines skipped because they exceeded the per-line length
    /// guard. A non-zero value means a pathological line was present and
    /// neither pattern matching nor entropy scanning ran for it.
    pub lines_skipped_oversize: usize,
}

/// SCAN-002: scan content and report any oversize-line skips. The
/// per-line length guard is the cheapest credible mitigation against
/// catastrophic backtracking on user-supplied custom regexes — see
/// `SecretCheckConfig::max_line_bytes` for the threshold rationale.
///
/// Use this when the caller wants to surface the skipped-line count
/// (e.g. discovery flows that need to tell users "we skipped a 4 MB
/// minified line"). For the legacy path, see `scan_content`, which
/// drops the stats and is a thin wrapper around this function.
pub fn scan_content_with_stats(
    content: &str,
    file_path: &str,
    config: &SecretCheckConfig,
) -> (Vec<SecretFinding>, ScanStats) {
    let matcher = PatternMatcher::new(&config.custom_allowlist);
    let (custom_patterns, _custom_errors) = compile_custom_patterns(&config.custom_patterns);
    let default_patterns: &[CompiledPattern] = &DEFAULT_COMPILED_PATTERNS;
    let mut findings = Vec::new();
    let mut stats = ScanStats::default();

    let patterns_iter = || default_patterns.iter().chain(custom_patterns.iter());

    // Tracks lines that were skipped by the length guard. The entropy pass
    // below honours the same set so a pathological line cannot route around
    // the guard via the entropy scanner.
    let mut oversize_line_indices: std::collections::HashSet<usize> =
        std::collections::HashSet::new();

    for (index, line) in content.lines().enumerate() {
        // SCAN-002: skip lines that exceed the configured byte cap. We use
        // `len()` (bytes) rather than `chars().count()` because the threat
        // is regex backtracking, which is bounded by the byte length the
        // regex engine actually walks — so byte-count is the correct
        // measure, and it is also O(1).
        if line.len() > config.max_line_bytes {
            stats.lines_skipped_oversize += 1;
            oversize_line_indices.insert(index);
            continue;
        }

        let line_number = index + 1;

        for pattern in patterns_iter() {
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
                // SCAN-002: respect the length-guard skip set — entropy
                // scanning over the original content already touches every
                // line, but we must not surface findings on lines the
                // pattern pass refused to inspect.
                if oversize_line_indices.contains(&line_index) {
                    return false;
                }
                lines.get(line_index).is_some_and(|line| {
                    patterns_iter().all(|pattern| !pattern.regex.is_match(line))
                })
            })
            .collect::<Vec<_>>();
        findings.extend(new_entropy_findings);
    }

    (findings, stats)
}

/// Legacy entry point that drops the SCAN-002 stats. Prefer
/// `scan_content_with_stats` for new callers that need the skipped-line
/// count.
pub fn scan_content(
    content: &str,
    file_path: &str,
    config: &SecretCheckConfig,
) -> Vec<SecretFinding> {
    scan_content_with_stats(content, file_path, config).0
}

#[cfg(test)]
mod tests {
    use crate::secret::scanner::{scan_content, scan_content_with_stats};
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

    // SCAN-002 — per-line length guard.
    //
    // The guard skips any line whose byte length exceeds
    // `max_line_bytes` so that a pathological minified/base64 line cannot
    // trigger worst-case backtracking on the 18 built-in patterns or any
    // user-supplied custom regex. The skip is reported via `ScanStats`.

    #[test]
    fn skips_oversize_line_and_counts_it() {
        // Embed a real secret pattern inside a 5 KB line. With the default
        // 4 KB guard the line is dropped wholesale, so the secret is
        // *intentionally* not surfaced — the guard's contract is "we
        // refuse to walk this line at all" and the counter tells the
        // caller that decision was made.
        let secret = "ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let padding = "x".repeat(5000);
        let content = format!("{padding}{secret}");

        let config = SecretCheckConfig::default();
        let (findings, stats) = scan_content_with_stats(&content, "src/huge.ts", &config);

        assert_eq!(
            findings.len(),
            0,
            "oversize line should be skipped wholesale"
        );
        assert_eq!(stats.lines_skipped_oversize, 1);
    }

    #[test]
    fn does_not_skip_normal_lines() {
        let content = "const a = 1;\nconst b = 2;\n";
        let config = SecretCheckConfig::default();
        let (_, stats) = scan_content_with_stats(content, "src/ok.ts", &config);
        assert_eq!(stats.lines_skipped_oversize, 0);
    }

    #[test]
    fn respects_custom_max_line_bytes_threshold() {
        // Drop the guard to 32 bytes and confirm a 100-byte line is skipped.
        let config = SecretCheckConfig {
            max_line_bytes: 32,
            ..SecretCheckConfig::default()
        };
        let content = "x".repeat(100);
        let (findings, stats) = scan_content_with_stats(&content, "src/x.ts", &config);
        assert!(findings.is_empty());
        assert_eq!(stats.lines_skipped_oversize, 1);
    }

    #[test]
    fn line_at_exact_threshold_is_kept() {
        // Boundary: `line.len() > threshold` — equal-length is *not*
        // skipped. Keep this test pinned so the boundary doesn't drift.
        let config = SecretCheckConfig {
            max_line_bytes: 40,
            ..SecretCheckConfig::default()
        };
        let content = "x".repeat(40);
        let (_, stats) = scan_content_with_stats(&content, "src/edge.ts", &config);
        assert_eq!(stats.lines_skipped_oversize, 0);
    }

    #[test]
    fn entropy_pass_skips_oversize_lines() {
        // High-entropy material on an oversize line must not surface
        // through the entropy pass either — the guard is global per line.
        let entropy_blob = "9xY7qW2vK8mN4pR69xY7qW2vK8mN4pR6";
        let padding = "y".repeat(5000);
        let content = format!("{padding}{entropy_blob}");

        let config = SecretCheckConfig {
            enable_entropy: true,
            entropy_threshold: 3.0,
            ..SecretCheckConfig::default()
        };
        let (findings, stats) = scan_content_with_stats(&content, "src/big.ts", &config);
        assert!(findings.is_empty());
        assert_eq!(stats.lines_skipped_oversize, 1);
    }
}
