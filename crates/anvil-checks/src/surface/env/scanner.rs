//! `.env` file secret scan.
//!
//! Parses the file into key/value pairs and applies the existing built-in
//! secret patterns to each value. Findings carry the variable name plus
//! the source line so operators can jump straight to the offending entry.
//!
//! Suppressions follow [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md):
//! a `# @anvil-ignore SURFENV-001 -- <reason>` comment on the line
//! immediately preceding the offending entry suppresses it. The directive
//! is parsed by `crate::antipattern::parse_suppression` (the authoritative
//! Rust suppression parser).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::secret::patterns::{DEFAULT_COMPILED_PATTERNS, PatternMatcher};
use crate::secret::types::FindingType;
use crate::secret::{SecretCheckConfig, SecretFinding};
use crate::surface::env::parser::parse_env;
use crate::surface::env::suppression::resolve_line_suppression;

/// Rule ID for the SURFENV-001 secret-detected-in-`.env`-file check.
///
/// Matches the `[A-Z]+-NNN` shape recognised by `parse_suppression` so
/// `# @anvil-ignore SURFENV-001 -- <reason>` works without further
/// parser changes.
pub const SURFENV_001_RULE_ID: &str = "SURFENV-001";

/// One secret finding sourced from a `.env` entry.
///
/// Wraps the underlying `SecretFinding` so consumers can render the env
/// variable name (e.g. `AWS_ACCESS_KEY_ID`) alongside the redacted match.
/// Suppression status mirrors the antipattern scanner's `Suppression` —
/// suppressed findings still appear in the result so the TUI / CLI can
/// show "1 suppressed" counts rather than silently dropping them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvFinding {
    pub finding: SecretFinding,
    pub key: String,
    pub suppressed: bool,
    pub suppression_reason: Option<String>,
}

/// Filenames that route through the `.env` scanner.
///
/// Recognises `.env`, `.envrc`, and the open-ended `.env.<environment>` case
/// (`.env.local`, `.env.production.local`, team-specific suffixes, …).
/// Discovery callers (anvil-cli / anvil-architecture) should consult this
/// before falling back to extension-based routing.
///
/// Note: `.env*` files are no longer in `crate::filter::ALWAYS_SCAN_FILENAMES`
/// (GH #2584) — a gitignored `.env` is the user's local secret store and is
/// not force-scanned. This predicate classifies `.env` files for callers that
/// scan them deliberately; it does not itself force a gitignored file to be
/// read.
#[must_use]
pub fn is_env_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };

    if name == ".env" || name == ".envrc" {
        return true;
    }

    // `.env.<anything>` — covers `.env.local`, `.env.production.local`,
    // `.env.staging`, plus team-specific suffixes. `.env.example` is
    // intentionally included: SURFENV-001 only does *content* scanning,
    // and an example file with a real-looking key is just as much of a
    // leak as one in `.env`. The structural rule that treats
    // `.env.example` differently (committed-template handling) is
    // SURFENV-004's job.
    name.starts_with(".env.")
}

/// Scan a `.env` file's content for secrets in values.
///
/// Returns one `EnvFinding` per `(entry, secret pattern)` match, sorted
/// by source line. Values are checked with the existing secret patterns
/// and `PatternMatcher` allowlist, but this scanner intentionally does
/// **not** apply the standalone secret scanner's `looks_like_code`
/// filter — see the inline note in the scan loop for the rationale
/// (a parsed `.env` value position is exactly where AWS-shaped keys
/// belong; the all-caps heuristic that drops them in source-line
/// scanning hides real findings here).
#[must_use]
pub fn scan_env_file(
    file_path: &str,
    content: &str,
    config: &SecretCheckConfig,
) -> Vec<EnvFinding> {
    let (entries, _parse_errors) = parse_env(content);
    let matcher = PatternMatcher::new(&config.custom_allowlist);
    let lines: Vec<&str> = content.lines().collect();
    let mut findings = Vec::new();

    for entry in &entries {
        for pattern in DEFAULT_COMPILED_PATTERNS.iter() {
            // Match against the raw value rather than the source line so
            // we don't double-trigger on comments or the key name.
            let Some(matched_range) = pattern.regex.find(&entry.value) else {
                continue;
            };
            let matched_value = matched_range.as_str();
            // High-confidence shape patterns (AWS, GitHub, Slack, …)
            // bypass the keyword allowlist — the textbook AWS docs key
            // `AKIAIOSFODNN7EXAMPLE` is still a real-shape credential
            // even with `EXAMPLE` in the value (issue #1800). Shape-
            // anchored allowlist entries (hex hashes, `0x…`, data URIs)
            // and user `custom_allowlist` opt-outs still apply for both
            // paths.
            let allowlisted = if pattern.high_confidence {
                matcher.is_shape_or_custom_allowlisted(matched_value)
            } else {
                matcher.is_allowlisted(matched_value)
            };
            if allowlisted {
                continue;
            }
            // The standalone secret scanner runs `looks_like_code` to drop
            // identifiers and method calls (`fetch(`, `service.call`,
            // `MAX_RETRIES`). Inside a parsed `.env` *value* those shapes
            // are vanishingly rare and the same heuristic actively hides
            // real AWS Access Keys (`AKIA…` matches the all-caps rule).
            // Skip the filter here — the parser already isolated the
            // value, so we don't need a second-line "is this code?" guard.

            let (suppressed, reason) =
                resolve_line_suppression(&lines, entry.line, SURFENV_001_RULE_ID);

            // Compute the redaction range against the *raw source line*,
            // not the decoded value. `matched_range` indexes into
            // `entry.value` (post-decode: quotes stripped, `\\n`-style
            // escapes resolved); adding it to `entry.value_span.start`
            // would misalign with the raw line for any quoted value or
            // any escape sequence. Searching for the matched substring
            // inside the raw value span keeps the column accurate, and
            // we fall back to redacting the whole value span if the
            // matched value isn't found verbatim (e.g. the match crosses
            // an escape boundary in a double-quoted value).
            let line_text = lines
                .get(entry.line.saturating_sub(1))
                .copied()
                .unwrap_or("");
            let raw_value = line_text.get(entry.value_span.clone()).unwrap_or("");
            let (redact_start, redact_end) = match raw_value.find(matched_value) {
                Some(offset) => {
                    let start = entry.value_span.start + offset;
                    (start, start + matched_value.len())
                }
                None => (entry.value_span.start, entry.value_span.end),
            };

            let secret = SecretFinding {
                file: file_path.to_string(),
                line: entry.line,
                finding_type: FindingType::Pattern,
                pattern_name: pattern.name.clone(),
                redacted_match: matcher.redact_secret(matched_value),
                redacted_line: matcher.redact_range_in_line(line_text, redact_start, redact_end),
                match_start: Some(redact_start),
                match_end: Some(redact_end),
                token_shape: Some(crate::secret::types::TokenShape::Opaque),
            };

            findings.push(EnvFinding {
                finding: secret,
                key: entry.key.clone(),
                suppressed,
                suppression_reason: reason,
            });
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{SURFENV_001_RULE_ID, SecretCheckConfig, is_env_file, scan_env_file};

    fn config_no_entropy() -> SecretCheckConfig {
        SecretCheckConfig {
            enable_entropy: false,
            ..SecretCheckConfig::default()
        }
    }

    #[test]
    fn is_env_file_matches_canonical_names() {
        assert!(is_env_file(Path::new(".env")));
        assert!(is_env_file(Path::new(".envrc")));
        assert!(is_env_file(Path::new(".env.local")));
        assert!(is_env_file(Path::new(".env.production.local")));
        assert!(is_env_file(Path::new("packages/api/.env.staging")));
        // `.env.example` deliberately matches — content scan still applies.
        assert!(is_env_file(Path::new(".env.example")));
    }

    #[test]
    fn is_env_file_rejects_lookalikes() {
        assert!(!is_env_file(Path::new("env.ts")));
        assert!(!is_env_file(Path::new("config.env.json")));
        assert!(!is_env_file(Path::new("README.md")));
    }

    #[test]
    fn detects_aws_access_key_in_value() {
        let content = "\
# Production AWS credentials — do NOT commit
AWS_ACCESS_KEY_ID=AKIAABCDEFGHIJKLMNOP
AWS_REGION=eu-west-2
";
        let findings = scan_env_file(".env", content, &config_no_entropy());
        assert_eq!(findings.len(), 1, "expected one AWS key finding");
        let f = &findings[0];
        assert_eq!(f.finding.pattern_name, "AWS Key");
        assert_eq!(f.key, "AWS_ACCESS_KEY_ID");
        assert_eq!(f.finding.line, 2);
        assert!(!f.suppressed);
    }

    #[test]
    fn does_not_match_secret_in_a_comment() {
        let content = "\
# Example: AWS_ACCESS_KEY_ID=AKIAABCDEFGHIJKLMNOP
SAFE=value
";
        let findings = scan_env_file(".env", content, &config_no_entropy());
        assert!(
            findings.is_empty(),
            "comment-only AWS key must not produce a finding"
        );
    }

    #[test]
    fn honours_anvil_ignore_directive_per_adr_029() {
        let content = format!(
            "# @anvil-ignore {SURFENV_001_RULE_ID} -- legacy fixture, rotated\n\
AWS_ACCESS_KEY_ID=AKIAABCDEFGHIJKLMNOP\n"
        );
        let findings = scan_env_file(".env", &content, &config_no_entropy());
        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert!(f.suppressed, "directive on previous line should suppress");
        assert_eq!(
            f.suppression_reason.as_deref(),
            Some("legacy fixture, rotated")
        );
    }

    #[test]
    fn ignore_directive_for_different_rule_does_not_suppress() {
        let content = "\
# @anvil-ignore AP-003 -- unrelated rule
AWS_ACCESS_KEY_ID=AKIAABCDEFGHIJKLMNOP
";
        let findings = scan_env_file(".env", content, &config_no_entropy());
        assert_eq!(findings.len(), 1);
        assert!(!findings[0].suppressed);
    }

    #[test]
    fn detects_quoted_value_with_secret() {
        // Double-quoted GitHub token — the parser strips quotes before
        // handing the value to the secret scanner.
        let token = format!("ghp_{}", "a".repeat(36));
        let content = format!("GITHUB_TOKEN=\"{token}\"\n");
        let findings = scan_env_file(".env", &content, &config_no_entropy());
        assert!(
            findings
                .iter()
                .any(|f| f.finding.pattern_name == "GitHub Token"),
            "GitHub token inside double quotes should still match"
        );
    }

    #[test]
    fn redacts_match_in_line_output() {
        let content = "AWS_ACCESS_KEY_ID=AKIAABCDEFGHIJKLMNOP\n";
        let findings = scan_env_file(".env", content, &config_no_entropy());
        assert_eq!(findings.len(), 1);
        let redacted = &findings[0].finding.redacted_line;
        assert!(
            redacted.contains("[REDACTED]"),
            "expected [REDACTED] in {redacted}"
        );
        assert!(!redacted.contains("AKIAABCDEFGHIJKLMNOP"));
    }
}
