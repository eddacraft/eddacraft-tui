use regex::Regex;

use crate::secret::context::{
    context_window, has_sensitive_binding_context, has_validator_fixture_context, is_benign_context,
};
use crate::secret::patterns::PatternMatcher;
use crate::secret::types::{
    AllowlistProvenance, FindingType, SecretCheckConfig, SecretFinding, Suppression,
};

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
    let lines = content.lines().collect::<Vec<_>>();

    for (index, line) in lines.iter().copied().enumerate() {
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
            let match_start = candidate_match.start();
            let match_end = candidate_match.end();

            if candidate.len() < config.min_entropy_length {
                continue;
            }
            // Path-shaped tokens (drive-prefixed document paths, absolute
            // filesystem paths ending in known document extensions) trip
            // the generic entropy heuristic because path separators and
            // long English filenames raise the score past threshold.
            // Vendor-prefixed secret rules still fire on those strings —
            // this exemption is entropy-only (Dave SEC-FP-1).
            if is_path_shaped_document_token(candidate, line, match_start, match_end) {
                continue;
            }
            if matcher.looks_like_code(candidate) {
                continue;
            }

            let entropy = calculate_entropy(candidate);
            if entropy < config.entropy_threshold {
                continue;
            }

            if is_benign_entropy_fixture(file, &lines, index, candidate) {
                suppressions.push(Suppression {
                    file: file.to_string(),
                    line: line_number,
                    rule_name: "High Entropy String".to_string(),
                    redacted_match: matcher.redact_secret(candidate),
                    provenance: AllowlistProvenance::BuiltinBenignFixture,
                });
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
                redacted_line: matcher.redact_range_in_line(line, match_start, match_end),
                match_start: Some(match_start),
                match_end: Some(match_end),
                token_shape: Some(crate::secret::types::TokenShape::Opaque),
            });
            if findings.len() == limit {
                return findings;
            }
        }
    }

    findings
}

/// True when the matched token is a filesystem / document path rather than a
/// credential. Used only by the generic high-entropy pass — vendor-prefixed
/// rules (`SECRET-STRIPE-KEY`, etc.) still fire when a secret sits inside a
/// path-shaped string.
///
/// Heuristic (Dave SEC-FP-1, pack 05 / 05a):
/// - token contains a path separator (`/` or `\`), **and**
/// - either the token itself or the characters immediately after the match
///   end in a known document extension, **or**
/// - the match is introduced by a Windows drive-letter colon (`D:…`) which
///   the assignment regex otherwise treats as a secret assignment.
pub(crate) fn is_path_shaped_document_token(
    candidate: &str,
    line: &str,
    match_start: usize,
    match_end: usize,
) -> bool {
    let has_sep = candidate.contains('/') || candidate.contains('\\');
    if !has_sep {
        return false;
    }

    // Characters after the regex capture often hold the document extension
    // (the capture class excludes `.`).
    let after = line.get(match_end..).unwrap_or("");
    if document_extension_prefix(after).is_some() {
        return true;
    }
    // Capture may already include a dotted extension when the pattern
    // changes; keep this branch for future char-class expansion.
    if candidate
        .rsplit_once('.')
        .is_some_and(|(_, ext)| is_document_extension(ext))
    {
        return true;
    }

    // Windows drive assignment false positive: `D:/DOCS/…` — the `:` after a
    // single letter is the assignment-pattern trigger, not a secret binding.
    if match_start >= 2 {
        let prefix = &line[..match_start];
        if let Some(drive_idx) = prefix.rfind(':') {
            let before_colon = &prefix[..drive_idx];
            let letter = before_colon.chars().last();
            if letter.is_some_and(|c| c.is_ascii_alphabetic()) {
                let rest_ok = before_colon
                    .chars()
                    .rev()
                    .nth(1)
                    .is_none_or(|c| !c.is_ascii_alphanumeric());
                if rest_ok {
                    return true;
                }
            }
        }
    }

    // Multi-segment paths with no extension still look like paths when they
    // carry several separators (e.g. nested project directories).
    let seps = candidate
        .chars()
        .filter(|c| *c == '/' || *c == '\\')
        .count();
    seps >= 2
}

fn document_extension_prefix(after: &str) -> Option<&str> {
    let trimmed = after.trim_start_matches(['"', '\'']);
    let ext = trimmed.strip_prefix('.')?;
    let end = ext
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(ext.len());
    let token = &ext[..end];
    if is_document_extension(token) {
        Some(token)
    } else {
        None
    }
}

fn is_document_extension(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "md" | "markdown"
            | "txt"
            | "rst"
            | "adoc"
            | "html"
            | "htm"
            | "css"
            | "scss"
            | "less"
            | "json"
            | "yaml"
            | "yml"
            | "toml"
            | "xml"
            | "csv"
            | "tsv"
            | "pdf"
            | "doc"
            | "docx"
            | "xls"
            | "xlsx"
            | "ppt"
            | "pptx"
            | "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "svg"
            | "webp"
            | "ico"
            | "mp4"
            | "mp3"
            | "wav"
            | "zip"
            | "gz"
            | "tgz"
            | "tar"
            | "7z"
            | "rar"
            | "log"
            | "ini"
            | "cfg"
            | "conf"
            | "properties"
            | "env"
            | "gitignore"
            | "editorconfig"
            | "lock"
            | "sum"
            | "mod"
            | "rs"
            | "py"
            | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "mjs"
            | "cjs"
            | "go"
            | "java"
            | "kt"
            | "rb"
            | "php"
            | "cs"
            | "cpp"
            | "c"
            | "h"
            | "hpp"
            | "swift"
            | "sh"
            | "bash"
            | "zsh"
            | "ps1"
            | "bat"
            | "cmd"
    )
}

fn is_benign_entropy_fixture(file: &str, lines: &[&str], index: usize, candidate: &str) -> bool {
    let window = context_window(lines, index, 2);
    if has_sensitive_binding_context(&window) {
        return false;
    }
    let lower_line = lines
        .get(index)
        .copied()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let context_is_benign = is_benign_context(file, &window)
        || lower_line.contains("alphabet")
        || lower_line.contains("charset")
        || (is_public_alphabet(candidate)
            && (window.contains("chars")
                || window.contains("digits")
                || window.contains("alphabet")));
    if !context_is_benign {
        return false;
    }

    is_public_alphabet(candidate)
        || is_known_text_base64_vector(candidate)
        || is_known_non_secret_identifier_vector(candidate, &window)
        || (looks_base64ish(candidate) && has_validator_fixture_context(&window))
}

fn is_public_alphabet(candidate: &str) -> bool {
    matches!(
        candidate,
        "abcdefghijklmnopqrstuvwxyz"
            | "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
            | "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
            | "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
            | "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
    )
}

fn is_known_text_base64_vector(candidate: &str) -> bool {
    matches!(
        candidate,
        "TWFueSBoYW5kcyBtYWtlIGxpZ2h0IHdvcms="
            | "TWFueSBoYW5kcyBtYWtlIGxpZ2h0IHdvcms"
            | "UGF0aWVuY2UgaXMgdGhlIGtleSB0byBzdWNjZXNz"
            | "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXo="
            | "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXo"
            | "QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVo"
    )
}

fn looks_base64ish(candidate: &str) -> bool {
    candidate.len() >= 24
        && candidate
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '_' | '-' | '='))
        && candidate
            .chars()
            .any(|c| matches!(c, '+' | '/' | '_' | '-' | '='))
}

fn is_known_non_secret_identifier_vector(candidate: &str, window: &str) -> bool {
    candidate == "2naeRjTrrHJAkfd3tOuEjw90WCA" && window.contains("ksuid")
}

pub fn rounded_entropy(value: &str) -> f64 {
    (calculate_entropy(value) * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use crate::secret::entropy::{calculate_entropy, detect_high_entropy_strings, rounded_entropy};
    use crate::secret::types::{AllowlistProvenance, FindingType, SecretCheckConfig};

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
        let content = concat!(
            "\"nudge\": \"This TO",
            "DO has no tracking reference. Without a ticket number, issue\\n",
            "link, or project reference, nobody will be reminded to do this work.\\n\\n",
            "Add a reference: `// TO",
            "DO(PROJ-123): description` or\\n`// TO",
            "DO(#456): description`. If the work doesn't warrant a TO",
            "DO — either do it now or decide it's\\nnot important enough to track.\""
        );

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
    fn does_not_flag_drive_prefixed_document_paths() {
        // Dave SEC-FP-1: ordinary Windows document paths trip the generic
        // entropy rule because the drive colon matches the assignment
        // pattern and the long path scores above threshold.
        let config = SecretCheckConfig::default();
        let content = "See D:/DOCS/PROPOSAL_quarterly_revenue_forecast_summary_20260704.md\n";
        let findings = detect_high_entropy_strings(content, "notes.md", &config);
        assert!(
            findings.is_empty(),
            "drive-prefixed document path must not trip entropy, got: {:?}",
            findings
                .iter()
                .map(|f| &f.redacted_match)
                .collect::<Vec<_>>()
        );

        // Bare filename of the same shape still passes (control).
        let bare = "PROPOSAL_quarterly_revenue_forecast_summary_20260704.md\n";
        assert!(detect_high_entropy_strings(bare, "notes.md", &config).is_empty());
    }

    #[test]
    fn path_shaped_exemption_does_not_mask_opaque_secrets() {
        // Entropy still flags a high-entropy opaque token that is not path-shaped.
        let config = SecretCheckConfig {
            entropy_threshold: 3.5,
            ..SecretCheckConfig::default()
        };
        let content = "const apiToken = 'Qm9kR3p4VnNNdkxaWlhTamtCdQ==';\n";
        let findings = detect_high_entropy_strings(content, "src/auth.ts", &config);
        assert!(
            findings
                .iter()
                .any(|f| f.pattern_name == "High Entropy String"),
            "opaque high-entropy secret must still flag: {findings:?}"
        );
        assert!(
            findings
                .iter()
                .all(|f| f.match_start.is_some() && f.match_end.is_some())
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
    fn production_path_still_flags_secret_near_url_and_valid_tokens() {
        let config = SecretCheckConfig {
            entropy_threshold: 3.5,
            ..SecretCheckConfig::default()
        };
        let content = r"
// valid url for the webhook endpoint
const callbackUrl = 'https://example.com/hooks';
const apiToken = 'Qm9kR3p4VnNNdkxaWlhTamtCdQ==';
";

        let findings = detect_high_entropy_strings(content, "src/webhook.ts", &config);

        assert!(
            findings
                .iter()
                .any(|finding| finding.pattern_name == "High Entropy String"),
            "production secret near url/valid context must still flag: {findings:?}"
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

    #[test]
    fn suppresses_base64_and_alphabet_validation_vectors_with_provenance() {
        let config = SecretCheckConfig::default();
        let content = "test('base64 vectors', () => {\n  expect(base64.parse(\"TWFueSBoYW5kcyBtYWtlIGxpZ2h0IHdvcms=\")).toBe('ok');\n  const alphabet = \"abcdefghijklmnopqrstuvwxyz\";\n});";
        let mut suppressions = Vec::new();
        let findings = super::detect_high_entropy_strings_with_line_filter_and_limit(
            content,
            "packages/zod/src/v4/classic/tests/string.test.ts",
            &config,
            usize::MAX,
            |_, _| true,
            &mut suppressions,
        );

        assert!(
            findings.is_empty(),
            "benign vectors should be suppressed, got: {findings:?}"
        );
        assert!(
            suppressions
                .iter()
                .any(|s| s.rule_name == "High Entropy String"
                    && s.provenance == AllowlistProvenance::BuiltinBenignFixture),
            "benign fixture suppression should be observable: {suppressions:?}"
        );
    }

    #[test]
    fn credential_named_base64_like_value_still_flags_in_tests() {
        let config = SecretCheckConfig::default();
        let content = "test('session token', () => {\n  const sessionToken = \"TWFueSBoYW5kcyBtYWtlIGxpZ2h0IHdvcms=\";\n});";
        let findings = detect_high_entropy_strings(
            content,
            "packages/foo/src/__tests__/auth.fixture.ts",
            &config,
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.pattern_name == "High Entropy String"),
            "credential-bound base64-looking value must still flag: {findings:?}"
        );
    }

    #[test]
    fn suppresses_public_alphabet_constants_but_not_password_bindings() {
        let config = SecretCheckConfig::default();
        let content = "const chars = \"abcdefghijklmnopqrstuvwxyz\";\nexport const BASE_62_DIGITS =\n  \"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz\";";
        let mut suppressions = Vec::new();
        let findings = super::detect_high_entropy_strings_with_line_filter_and_limit(
            content,
            "packages/fractional-indexing/src/index.ts",
            &config,
            usize::MAX,
            |_, _| true,
            &mut suppressions,
        );

        assert!(
            findings.is_empty(),
            "public alphabet constants should be suppressed: {findings:?}"
        );
        assert!(
            suppressions
                .iter()
                .any(|s| s.provenance == AllowlistProvenance::BuiltinBenignFixture),
            "public alphabet suppression should be observable: {suppressions:?}"
        );

        let credential_findings = detect_high_entropy_strings(
            "const password = \"abcdefghijklmnopqrstuvwxyz\";",
            "src/config.ts",
            &config,
        );
        assert!(
            credential_findings
                .iter()
                .any(|finding| finding.pattern_name == "High Entropy String"),
            "credential-bound alphabet must still flag: {credential_findings:?}"
        );
    }

    #[test]
    fn suppresses_known_ksuid_validator_vector_only_in_context() {
        let config = SecretCheckConfig::default();
        let mut suppressions = Vec::new();
        let findings = super::detect_high_entropy_strings_with_line_filter_and_limit(
            "test('z.ksuid', () => {\n  expect(z.parse(a, \"2naeRjTrrHJAkfd3tOuEjw90WCA\")).toEqual(\"2naeRjTrrHJAkfd3tOuEjw90WCA\");\n});",
            "packages/zod/src/v4/mini/tests/string.test.ts",
            &config,
            usize::MAX,
            |_, _| true,
            &mut suppressions,
        );

        assert!(
            findings.is_empty(),
            "known zod KSUID validator vector should be suppressed: {findings:?}"
        );
        assert!(
            suppressions
                .iter()
                .any(|s| s.provenance == AllowlistProvenance::BuiltinBenignFixture),
            "KSUID vector suppression should be observable: {suppressions:?}"
        );

        let credential_findings = detect_high_entropy_strings(
            "const token = \"2naeRjTrrHJAkfd3tOuEjw90WCA\";",
            "src/auth.ts",
            &config,
        );
        assert!(
            credential_findings
                .iter()
                .any(|finding| finding.pattern_name == "High Entropy String"),
            "same KSUID-looking value outside validator context must still flag: {credential_findings:?}"
        );
    }
}
