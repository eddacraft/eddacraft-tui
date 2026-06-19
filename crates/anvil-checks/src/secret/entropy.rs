use regex::Regex;

use crate::secret::patterns::PatternMatcher;
use crate::secret::types::{FindingType, SecretCheckConfig, SecretFinding, Suppression};

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
    detect_high_entropy_strings_with_line_filter_and_limit(
        content,
        file,
        config,
        limit,
        |_, _| true,
        &mut Vec::new(),
    )
}

pub(crate) fn detect_high_entropy_strings_with_line_filter_and_limit(
    content: &str,
    file: &str,
    config: &SecretCheckConfig,
    limit: usize,
    mut include_line: impl FnMut(usize, &str) -> bool,
    suppressions: &mut Vec<Suppression>,
) -> Vec<SecretFinding> {
    if limit == 0 {
        return Vec::new();
    }
    let matcher = PatternMatcher::new(&config.custom_allowlist);
    // Real secret tokens are dense alphanumeric runs with at most a few
    // structure characters (`_/+=-`). Capturing any quoted run of 16+
    // chars produced thousands of false positives in the v0.5.0 discovery
    // scan: long Tailwind className strings, JSDoc/`nudge` prose, regex
    // pattern definitions, and pnpm-lock.yaml peer-dep keys all crossed
    // the entropy threshold. Mirroring `assignment_pattern`'s char class
    // here rejects values containing spaces, parentheses, brackets,
    // commas, dots, etc — none of which appear in actual secret tokens.
    let Ok(quoted_pattern) = Regex::new(r#"['\"]([a-zA-Z0-9_/+=\-]{16,})['\"]"#) else {
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
            if matcher.looks_like_code(candidate) {
                continue;
            }

            let entropy = calculate_entropy(candidate);
            if entropy < config.entropy_threshold {
                continue;
            }

            // The candidate is high-entropy enough to flag. If an allowlist
            // entry covers it, withhold it but record the suppression with
            // provenance rather than dropping it silently — the allowlist
            // check runs *after* the threshold test so only genuine would-be
            // findings are reported as suppressed.
            if let Some(provenance) = matcher.matched_allowlist(candidate) {
                suppressions.push(Suppression {
                    file: file.to_string(),
                    line: line_number,
                    rule_name: "High Entropy String".to_string(),
                    redacted_match: matcher.redact_secret(candidate),
                    provenance,
                });
                continue;
            }

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

    // Public record identifiers — ULIDs — are dense Crockford-base32 runs
    // that appear all over real codebases (databases, logs, diagnostics).
    // A maximally-diverse 26-char ULID reaches entropy log2(26) ≈ 4.70,
    // above the 4.5 default threshold, so the heuristic flags it as a
    // secret. A real secret is never *exactly* a 26-char Crockford ULID, so
    // allowlisting the anchored shape is safe (same rationale as the
    // existing hex-hash entries in `DEFAULT_SHAPE_ALLOWLIST`).

    #[test]
    fn does_not_flag_bare_ulid() {
        // High-diversity 26-char Crockford base32 ULID (entropy ≈ 4.70).
        let config = SecretCheckConfig::default();
        let content = "const id = '0123456789ABCDEFGHJKMNPQRS';";
        let findings = detect_high_entropy_strings(content, "src/record.ts", &config);
        assert!(
            findings.is_empty(),
            "bare ULID should not be flagged as a secret, got: {:?}",
            findings
                .iter()
                .map(|f| &f.redacted_match)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn uuid_is_inherently_below_threshold() {
        // Characterisation guard: a hex UUID draws from a 16-symbol
        // alphabet (max entropy log2(16) = 4.0 < 4.5), so it can never trip
        // the entropy detector and needs no allowlist entry. If the default
        // threshold ever drops below 4.0 this test fails loudly, flagging
        // that a UUID shape rule becomes necessary.
        let config = SecretCheckConfig::default();
        let content = "const id = 'f47ac10b-58cc-4372-a567-0e02b2c3d479';";
        let findings = detect_high_entropy_strings(content, "src/record.ts", &config);
        assert!(
            findings.is_empty(),
            "UUID unexpectedly flagged, got: {:?}",
            findings
                .iter()
                .map(|f| &f.redacted_match)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn ulid_is_suppressed_but_a_real_secret_still_flags() {
        // Exercise the allowlist path directly: a ULID (line 1) must be
        // withheld while a genuine high-entropy secret (line 2) still fires —
        // the allowlist must suppress only the ULID, nothing else. Uses the
        // default threshold so it reflects production behaviour.
        let config = SecretCheckConfig::default();
        let content = "const id = '0123456789ABCDEFGHJKMNPQRS';\n\
                       const token = '7kQ2mZ9pV4xL8nB3rW6tC1yH5jD0sF';";
        let findings = detect_high_entropy_strings(content, "src/auth.ts", &config);
        assert!(
            findings.iter().any(|f| f.line == 2),
            "real secret on line 2 must still flag, got: {:?}",
            findings
                .iter()
                .map(|f| (f.line, &f.redacted_match))
                .collect::<Vec<_>>()
        );
        assert!(
            !findings.iter().any(|f| f.line == 1),
            "ULID on line 1 must be suppressed by the allowlist"
        );
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

    // v0.5.0 Discovery scan false positives — real strings that the
    // entropy detector flagged in the anvil-001 codebase. None of these
    // are secrets; they are prose, regex source, or CSS class lists.
    // The fix is to require the captured value to consist of secret-
    // shaped characters (no spaces, parens, brackets, commas, etc).

    #[test]
    fn does_not_flag_long_natural_prose_in_quoted_string() {
        // From patterns/compiled/registry.json:210 — a "nudge" field
        // (367 chars, entropy 4.70) the discovery scan flagged as
        // High Entropy String. Spaces and punctuation make it not a
        // secret-shaped value.
        let config = SecretCheckConfig::default();
        let content = "\"nudge\": \"This TODO has no tracking reference. Without a ticket number, issue\\nlink, or project reference, nobody will be reminded to do this work.\\n\\nAdd a reference: `// TODO(PROJ-123): description` or\\n`// TODO(#456): description`. If the work doesn't warrant a ticket,\\nconsider whether it warrants a TODO — either do it now or decide it's\\nnot important enough to track.\"";

        let findings =
            detect_high_entropy_strings(content, "patterns/compiled/registry.json", &config);

        assert!(
            findings.is_empty(),
            "natural-prose string should not be flagged as a secret, got: {:?}",
            findings
                .iter()
                .map(|f| &f.redacted_match)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn does_not_flag_regex_pattern_source() {
        // From patterns/deferred-debt/DD-003.anvil:18 (121 chars,
        // entropy 4.74). Backslash-escapes, parens, brackets, pipes
        // make this a regex source, not a secret value.
        let config = SecretCheckConfig::default();
        let content = r"  pattern: '//\s*(temporary|workaround|compat|shim|stopgap|interim)\b(?!.*(until|before|after|when|remove|drop|deadline|\d{4}-\d{2}))'";

        let findings =
            detect_high_entropy_strings(content, "patterns/deferred-debt/DD-003.anvil", &config);

        assert!(
            findings.is_empty(),
            "regex pattern source should not be flagged as a secret, got: {:?}",
            findings
                .iter()
                .map(|f| &f.redacted_match)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn does_not_flag_tailwind_classname_lists() {
        // The CSS-class case other users hit: a long Tailwind className
        // string. Spaces disqualify it as a secret value.
        let config = SecretCheckConfig::default();
        let content = "<button className=\"rounded-md bg-indigo-500 px-3.5 py-2.5 text-sm font-semibold text-white shadow-sm hover:bg-indigo-400 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-500\">";

        let findings = detect_high_entropy_strings(content, "components/Button.tsx", &config);

        assert!(
            findings.is_empty(),
            "Tailwind className list should not be flagged as a secret, got: {:?}",
            findings
                .iter()
                .map(|f| &f.redacted_match)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn still_flags_real_secret_shaped_quoted_value() {
        // Regression guard: tightening quoted_pattern must not disable
        // detection of high-entropy secret-shaped values. The same line
        // matches both quoted and assignment patterns, so two findings
        // is the contract of the existing detector.
        let config = SecretCheckConfig {
            entropy_threshold: 3.5,
            ..SecretCheckConfig::default()
        };
        let content = "const token = '9xY7qW2vK8mN4pR6';";

        let findings = detect_high_entropy_strings(content, "src/auth.ts", &config);

        assert!(!findings.is_empty(), "secret-shaped value must still fire");
        assert!(
            findings
                .iter()
                .all(|f| f.finding_type == FindingType::Entropy)
        );
    }
}
