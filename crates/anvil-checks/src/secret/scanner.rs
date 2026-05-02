use crate::secret::entropy::detect_high_entropy_strings_with_line_filter_and_limit;
use crate::secret::patterns::{
    CompiledPattern, DEFAULT_COMPILED_PATTERNS, PatternMatcher, compile_custom_patterns,
};
use crate::secret::types::{FindingType, SecretCheckConfig, SecretFinding};

/// Reject Credit Card matches that are actually a fragment of a
/// UUID (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`). The Credit Card
/// regex `\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b` happily matches
/// the first 18 digits of a UUID like `11111111-2222-0000-...`. If
/// the byte immediately before or after the matched range is `-` or
/// another digit, we are inside a longer dashed/numeric token and
/// the match is not a real card number.
fn is_credit_card_false_positive(line: &str, match_start: usize, match_end: usize) -> bool {
    let bytes = line.as_bytes();
    if match_start > 0 {
        let prev = bytes[match_start - 1];
        if prev == b'-' || prev.is_ascii_digit() {
            return true;
        }
    }
    if match_end < bytes.len() {
        let next = bytes[match_end];
        if next == b'-' || next.is_ascii_digit() {
            return true;
        }
    }
    false
}

/// Reject the "Generic Secret" pattern when its right-hand side is
/// clearly a code expression rather than a secret literal. Real
/// secrets are quoted strings or unquoted env-style values that
/// contain digits / mixed case; variable references, type annotations,
/// `process.env.X` accesses, and `${...}` template substitutions all
/// fail one of these tests.
///
/// Returns `true` when the value should be SKIPPED (i.e. the match is
/// a false positive).
fn is_generic_secret_false_positive(matched_value: &str) -> bool {
    // Split on the first `=` or `:` so the RHS is the candidate value.
    let Some(rhs_start) = matched_value.find(['=', ':']) else {
        return false;
    };
    let rhs_raw = &matched_value[rhs_start + 1..];
    let rhs = rhs_raw.trim();
    if rhs.is_empty() {
        return false;
    }

    // Strip outer matched quotes — `"hunter2"` should be evaluated as
    // `hunter2`, not as starting with `"`. Trailing terminators like
    // `;`, `,` are noise from how the regex captures up to the next
    // whitespace; strip them too.
    let unquoted = rhs
        .strip_prefix(['"', '\''])
        .and_then(|s| s.strip_suffix(['"', '\'']))
        .unwrap_or(rhs)
        .trim_end_matches([';', ',']);

    // Code-structural characters that never appear in a real secret
    // literal: TS type closures (`string)`), property access
    // (`config.password`), bracket env access (`process.env['X']`),
    // template substitutions (`${dbPassword}`), function calls
    // (`requireSecret(...)`), and similar. Real passwords with
    // `!@#$%^&*` survive — only structural code shape is rejected.
    let has_code_shape = unquoted
        .chars()
        .any(|c| matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | '`' | '.' | ',' | ';'));
    if has_code_shape {
        return true;
    }

    // Bare identifier (alphabetic + underscore only, no digits, no
    // special chars). Catches variable references like `passwordEnv`,
    // `configuredPassword` and TS types like `string`, `Buffer`,
    // `ArrayBuffer`. Real secret literals carry at least one digit
    // or non-alpha character.
    let pure_identifier = unquoted
        .chars()
        .all(|c| c.is_ascii_alphabetic() || c == '_');
    if pure_identifier {
        return true;
    }

    false
}

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
    scan_content_with_limit_and_stats(content, file_path, config, usize::MAX)
}

pub fn scan_content_with_limit_and_stats(
    content: &str,
    file_path: &str,
    config: &SecretCheckConfig,
    limit: usize,
) -> (Vec<SecretFinding>, ScanStats) {
    let matcher = PatternMatcher::new(&config.custom_allowlist);
    let (custom_patterns, _custom_errors) = compile_custom_patterns(&config.custom_patterns);
    let default_patterns: &[CompiledPattern] = &DEFAULT_COMPILED_PATTERNS;
    let mut findings = Vec::new();
    let mut stats = ScanStats::default();

    if limit == 0 {
        return (findings, stats);
    }

    let patterns_iter = || default_patterns.iter().chain(custom_patterns.iter());

    // Tracks lines that were skipped by the length guard. The entropy pass
    // below honours the same set so a pathological line cannot route around
    // the guard via the entropy scanner. Lazily allocated — the common case
    // is no oversize lines, and a missing set means "no skipped lines".
    let mut oversize_line_indices: Option<std::collections::HashSet<usize>> = None;

    for (index, line) in content.lines().enumerate() {
        // SCAN-002: skip lines that exceed the configured byte cap. We use
        // `len()` (bytes) rather than `chars().count()` because the threat
        // is regex backtracking, which is bounded by the byte length the
        // regex engine actually walks — so byte-count is the correct
        // measure, and it is also O(1).
        if line.len() > config.max_line_bytes {
            stats.lines_skipped_oversize += 1;
            oversize_line_indices
                .get_or_insert_with(std::collections::HashSet::new)
                .insert(index);
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
            // Generic Secret is keyword-anchored and accepts any non-quote
            // run as a value, which mis-flags TS type annotations,
            // env-var accesses, and identifiers. Apply the RHS-shape
            // filter so only real-secret-shaped values escape.
            if pattern.name == "Generic Secret" && is_generic_secret_false_positive(matched_value) {
                continue;
            }
            // Credit Card matches a UUID fragment if the preceding or
            // following char is `-` or another digit — we're inside a
            // longer dashed/numeric token, not a real 16-digit card.
            if pattern.name == "Credit Card"
                && is_credit_card_false_positive(line, matched_range.start(), matched_range.end())
            {
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
            if findings.len() == limit {
                return (findings, stats);
            }
        }
    }

    if config.enable_entropy {
        let remaining = limit.saturating_sub(findings.len());
        let entropy_findings = detect_high_entropy_strings_with_line_filter_and_limit(
            content,
            file_path,
            config,
            remaining,
            |line_index, line| {
                // SCAN-002: respect the length-guard skip set — entropy
                // scanning over the original content already touches every
                // line, but we must not surface findings on lines the
                // pattern pass refused to inspect.
                if oversize_line_indices
                    .as_ref()
                    .is_some_and(|set| set.contains(&line_index))
                {
                    return false;
                }
                patterns_iter().all(|pattern| !pattern.regex.is_match(line))
            },
        );
        findings.extend(entropy_findings);
    }

    (findings, stats)
}

pub fn scan_content_with_limit(
    content: &str,
    file_path: &str,
    config: &SecretCheckConfig,
    limit: usize,
) -> Vec<SecretFinding> {
    scan_content_with_limit_and_stats(content, file_path, config, limit).0
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
    fn scan_content_with_limit_stops_after_requested_findings() {
        let config = SecretCheckConfig::default();
        let content = "api_key='abcdEFGH1234567890'\npassword='abcdEFGH1234567890'";

        let findings = super::scan_content_with_limit(content, "src/test.ts", &config, 1);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 1);
    }

    #[test]
    fn does_not_flag_uuid_as_credit_card() {
        // From entx apps/admin/src/lib/authorization.spec.ts — UUIDs
        // shaped `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` were caught
        // by the Credit Card regex which only inspects the first 16
        // digits.
        let config = SecretCheckConfig::default();
        let content = "const VENUE_1 = '11111111-2222-0000-000000000001' as Id;";
        let findings = scan_content(content, "src/spec.ts", &config);
        let card = findings.iter().find(|f| f.pattern_name == "Credit Card");
        assert!(
            card.is_none(),
            "UUID fragment should not be flagged as Credit Card, got: {:?}",
            card.map(|f| &f.redacted_line)
        );
    }

    #[test]
    fn still_flags_real_credit_card_number() {
        // Regression guard: a genuine 16-digit card (with or without
        // hyphens) must still fire.
        let config = SecretCheckConfig::default();
        let content = "// see card 4242 4242 4242 4242 in fixtures";
        let findings = scan_content(content, "src/payments.ts", &config);
        assert!(
            findings.iter().any(|f| f.pattern_name == "Credit Card"),
            "real Visa-shaped card must still fire"
        );
    }

    // v0.5.0 Generic Secret false positives — actual repo lines that
    // were flagged but are not secret leaks. They share a shape: the
    // RHS after the keyword is a code expression (variable reference,
    // env access, type annotation, template substitution), not a
    // secret literal.

    #[test]
    fn does_not_flag_typescript_type_annotation_as_secret() {
        // From apps/anvil-api/src/middleware/admin-auth.ts:54
        let config = SecretCheckConfig::default();
        let content = "function hashBearer(bearer: string, secret: string): string {";
        let findings = scan_content(content, "src/auth.ts", &config);
        let generic = findings.iter().find(|f| f.pattern_name == "Generic Secret");
        assert!(
            generic.is_none(),
            "TS type annotation `secret: string` should not be flagged, got: {:?}",
            generic.map(|f| &f.redacted_line)
        );
    }

    #[test]
    fn does_not_flag_process_env_access_as_secret() {
        // From apps/anvil-api/src/routes/cron.ts:28 and similar.
        let config = SecretCheckConfig::default();
        let content = "  const cronSecret = process.env.CRON_SECRET || '';";
        let findings = scan_content(content, "src/cron.ts", &config);
        let generic = findings.iter().find(|f| f.pattern_name == "Generic Secret");
        assert!(
            generic.is_none(),
            "process.env access should not be flagged, got: {:?}",
            generic.map(|f| &f.redacted_line)
        );
    }

    #[test]
    fn does_not_flag_bracket_env_access_as_secret() {
        // From apps/anvil-api/src/__tests__/auth-github.test.ts:45
        let config = SecretCheckConfig::default();
        let content = "const ORIGINAL_CLIENT_SECRET = process.env['GITHUB_CLIENT_SECRET'];";
        let findings = scan_content(content, "src/test.ts", &config);
        let generic = findings.iter().find(|f| f.pattern_name == "Generic Secret");
        assert!(
            generic.is_none(),
            "process.env[...] access should not be flagged, got: {:?}",
            generic.map(|f| &f.redacted_line)
        );
    }

    #[test]
    fn does_not_flag_template_substitution_as_secret() {
        // From .codex/skills/pulumi-esc/SKILL.md:92,96
        let config = SecretCheckConfig::default();
        let content = "    DB_PASSWORD: ${dbPassword}";
        let findings = scan_content(content, "docs/SKILL.md", &config);
        let generic = findings.iter().find(|f| f.pattern_name == "Generic Secret");
        assert!(
            generic.is_none(),
            "template substitution should not be flagged, got: {:?}",
            generic.map(|f| &f.redacted_line)
        );
    }

    #[test]
    fn does_not_flag_bare_identifier_as_secret() {
        // From archive/anvil-cli-node/src/commands/policy/bundle.ts:296
        let config = SecretCheckConfig::default();
        let content = "    auth.password = configuredPassword;";
        let findings = scan_content(content, "src/bundle.ts", &config);
        let generic = findings.iter().find(|f| f.pattern_name == "Generic Secret");
        assert!(
            generic.is_none(),
            "variable reference should not be flagged, got: {:?}",
            generic.map(|f| &f.redacted_line)
        );
    }

    #[test]
    fn still_flags_real_quoted_secret_literal() {
        // Regression guard: the RHS-shape filter must still let real
        // password literals through.
        let config = SecretCheckConfig::default();
        let content = "password='hunter22hunter22'";
        let findings = scan_content(content, "src/.env", &config);
        assert!(
            findings.iter().any(|f| f.pattern_name == "Generic Secret"),
            "real quoted secret should still fire, got: {:?}",
            findings.iter().map(|f| &f.pattern_name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn still_flags_real_unquoted_secret_with_digits() {
        // Regression guard: unquoted env-style values with digits remain detectable.
        let config = SecretCheckConfig::default();
        let content = "PASSWORD=ab1cd2ef3gh4ij5";
        let findings = scan_content(content, ".env", &config);
        assert!(
            findings.iter().any(|f| f.pattern_name == "Generic Secret"),
            "real env-style secret should still fire, got: {:?}",
            findings.iter().map(|f| &f.pattern_name).collect::<Vec<_>>()
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
