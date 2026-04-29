//! Production-shaped values in non-production `.env` files (SURFENV-003).
//!
//! Catches the classic foot-gun: someone copy-pastes a real production
//! value into `.env.local` / `.env.development` "just to test something"
//! and never reverts it. The file is meant for local secrets so its
//! values escape the production-credential blast radius — until they
//! don't.
//!
//! The heuristic is intentionally conservative. False-positive noise
//! erodes the rule's credibility faster than a missed catch. We fire
//! only on indicators that are very unlikely in a development context:
//!
//! - The value mentions `production` / `prod-` / `_PROD` markers
//!   (case-insensitive substring on the relevant boundary).
//! - The value's hostname has a `prod`-prefixed segment (e.g.
//!   `prod-db.example.com`, `db.production.example.com`) — but we skip
//!   localhost / loopback / `*.local` / `*.test` / `*.example`.
//! - The key name itself ends in `_PROD` and the value is non-empty.
//!
//! Anything tagged `staging`, `dev`, `test`, or `local` short-circuits
//! the check on a per-line basis — the user has already announced the
//! environment intent.
//!
//! Suppression follows [ADR-029](../../../../plans/decisions/029-suppression-parser-authority.md):
//! a `# @anvil-ignore SURFENV-003 -- <reason>` directive on the line
//! immediately preceding the offending entry suppresses it.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::surface::env::parser::{EnvEntry, parse_env};
use crate::surface::env::suppression::resolve_line_suppression;

/// Rule ID for the SURFENV-003 production-value check.
pub const SURFENV_003_RULE_ID: &str = "SURFENV-003";

/// One production-shaped value found in a non-prod `.env` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProdValueFinding {
    pub file: String,
    pub line: usize,
    pub key: String,
    /// The indicator that fired — e.g. "value mentions `production`",
    /// "host segment `prod-db`", "key suffix `_PROD`". Surfaces in the
    /// CLI message so operators can tell why a line tripped the rule.
    pub indicator: ProdIndicator,
    /// Redacted excerpt of the matched value (no raw secret material —
    /// the caller already knows the line, and the redaction protects
    /// against the value containing a credential as well).
    pub redacted_excerpt: String,
    pub suppressed: bool,
    pub suppression_reason: Option<String>,
}

/// Why a production-shaped value tripped the rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProdIndicator {
    /// The literal substring `production` (case-insensitive) appears in
    /// the value at a word boundary.
    ValueMentionsProduction,
    /// A hostname segment in the value starts with `prod-` or contains
    /// `.production.`.
    ProdHostSegment,
    /// The key name itself ends in `_PROD` and the value is non-empty.
    KeySuffixProd,
}

/// Filenames considered explicitly non-production. Production-shaped
/// values inside these files trigger SURFENV-003.
fn is_non_prod_env_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    matches!(
        name,
        ".env.local"
            | ".env.development"
            | ".env.development.local"
            | ".env.dev"
            | ".env.dev.local"
            | ".env.test"
            | ".env.test.local"
    )
}

/// Scan a non-production `.env` file for production-shaped values.
///
/// Returns at most one finding per `EnvEntry` (multiple indicators on
/// the same line collapse to the highest-precedence one). Returns an
/// empty vec for files that aren't classified as non-prod — callers
/// can route every `.env*` file here without filtering first.
#[must_use]
pub fn scan_prod_values(file_path: &str, content: &str) -> Vec<ProdValueFinding> {
    if !is_non_prod_env_file(Path::new(file_path)) {
        return Vec::new();
    }
    let (entries, _errors) = parse_env(content);
    let lines: Vec<&str> = content.lines().collect();
    let mut findings = Vec::new();

    for entry in &entries {
        let Some(indicator) = classify_entry(entry) else {
            continue;
        };
        let (suppressed, reason) =
            resolve_line_suppression(&lines, entry.line, SURFENV_003_RULE_ID);
        findings.push(ProdValueFinding {
            file: file_path.to_string(),
            line: entry.line,
            key: entry.key.clone(),
            indicator,
            redacted_excerpt: redact_excerpt(&entry.value),
            suppressed,
            suppression_reason: reason,
        });
    }
    findings
}

fn classify_entry(entry: &EnvEntry) -> Option<ProdIndicator> {
    if value_announces_non_prod(&entry.value) {
        return None;
    }
    if value_has_prod_host_segment(&entry.value) {
        return Some(ProdIndicator::ProdHostSegment);
    }
    if value_mentions_production(&entry.value) {
        return Some(ProdIndicator::ValueMentionsProduction);
    }
    if key_has_prod_suffix(&entry.key) && !entry.value.trim().is_empty() {
        return Some(ProdIndicator::KeySuffixProd);
    }
    None
}

const NON_PROD_TOKENS: &[&str] = &[
    "staging",
    "stage.",
    ".stage",
    "localhost",
    "127.0.0.1",
    ".local",
    ".test",
    ".example",
    "example.com",
    // Development markers — both bounded forms (so a value like
    // `developer-tools-prod-key` doesn't short-circuit on a bare
    // `dev` substring). The module-level doc lists these as
    // short-circuits; council review caught the doc/code mismatch.
    "dev-",
    ".dev",
    "dev.",
    "development",
];

/// True when the value contains a marker like `staging`, `local`,
/// `dev`, or `test` — short-circuits the check because the user has
/// already announced this isn't a prod copy.
fn value_announces_non_prod(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    NON_PROD_TOKENS.iter().any(|tok| lower.contains(tok))
}

fn value_mentions_production(value: &str) -> bool {
    // Word-boundary check, with an extra carve-out: when the value is
    // a URL, occurrences of `production` after the path-start `/` are
    // almost always documentation/CDN paths rather than production-host
    // signals. Adversarial review caught
    // `https://docs.acme.io/production/quickstart` tripping the rule;
    // the path-skip below preserves the expected hits on bare values
    // (`FEATURE_FLAGS_ENV=production`) and on prod hostnames
    // (`https://production.acme.io/...` — match before the path).
    let lower = value.to_ascii_lowercase();
    let needle = "production";
    let bytes = lower.as_bytes();
    let n = needle.len();
    let url_path_start = url_path_start_index(&lower);
    let mut i = 0;
    while i + n <= bytes.len() {
        if &bytes[i..i + n] == needle.as_bytes() {
            let before_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            let after = i + n;
            let after_ok = after == bytes.len() || !bytes[after].is_ascii_alphanumeric();
            let in_url_path = url_path_start.is_some_and(|start| i >= start);
            if before_ok && after_ok && !in_url_path {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Returns the byte offset of the first `/` *after* a `scheme://host`
/// prefix, or `None` for non-URL values. Used by
/// [`value_mentions_production`] to skip path-segment matches.
fn url_path_start_index(value: &str) -> Option<usize> {
    let scheme_end = value.find("://")?;
    let after_scheme = scheme_end + 3;
    value[after_scheme..]
        .find('/')
        .map(|offset| after_scheme + offset)
}

fn value_has_prod_host_segment(value: &str) -> bool {
    // Look for hostname-shaped substrings (two-or-more `.`-joined
    // segments) and ask whether any segment is a prod marker. The
    // multi-segment requirement is what distinguishes "this looks
    // like a hostname" from "the value is the literal word
    // `production`" — the latter is reported under the
    // `ValueMentionsProduction` indicator instead.
    for segment in value.split(['/', '@', ':', '?', '&']) {
        let parts: Vec<&str> = segment.split('.').collect();
        if parts.len() < 2 {
            continue;
        }
        for part in &parts {
            let lower = part.to_ascii_lowercase();
            if lower == "prod" || lower == "production" || lower.starts_with("prod-") {
                return true;
            }
        }
    }
    false
}

fn key_has_prod_suffix(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    upper.ends_with("_PROD") || upper.ends_with("_PRODUCTION")
}

fn redact_excerpt(value: &str) -> String {
    // Keep enough context for the operator to recognise the value
    // (first 4 + last 4 chars) but never echo it whole — matches the
    // SURFENV-001 redaction style.
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 12 {
        return "[redacted]".to_string();
    }
    let head: String = chars.iter().take(4).collect();
    let tail: String = chars
        .iter()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{head}…{tail}")
}

#[cfg(test)]
mod tests {
    use super::{ProdIndicator, SURFENV_003_RULE_ID, is_non_prod_env_file, scan_prod_values};
    use std::path::Path;

    #[test]
    fn non_prod_filenames_are_recognised() {
        for name in [
            ".env.local",
            ".env.development",
            ".env.development.local",
            ".env.test",
            ".env.test.local",
        ] {
            assert!(is_non_prod_env_file(Path::new(name)), "{name}");
        }
        // `.env` and `.env.production` are NOT non-prod files.
        assert!(!is_non_prod_env_file(Path::new(".env")));
        assert!(!is_non_prod_env_file(Path::new(".env.production")));
        // `.env.staging` is ambiguous — staging != prod, but a value
        // that says `production` there is still wrong. Out of scope for
        // 003 today; revisit if a real-world report comes in.
    }

    #[test]
    fn ignores_files_outside_non_prod_set() {
        let findings = scan_prod_values(".env.production", "DB=production-east-1.aws\n");
        assert!(findings.is_empty());
    }

    #[test]
    fn flags_value_mentioning_production_word() {
        let findings = scan_prod_values(".env.local", "FEATURE_FLAGS_ENV=production\n");
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].indicator,
            ProdIndicator::ValueMentionsProduction
        );
        assert_eq!(findings[0].key, "FEATURE_FLAGS_ENV");
    }

    #[test]
    fn url_path_segment_named_production_does_not_fire() {
        // Adversarial-review carve-out: a docs URL whose path
        // contains `/production/` is documentation, not a production
        // host pointer. Compare against the host-name positive below.
        let findings = scan_prod_values(
            ".env.local",
            "API_DOC_URL=https://docs.acme.io/production/quickstart\n",
        );
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn production_host_before_path_still_fires() {
        // Sanity: the URL-path carve-out must not silence a real
        // production hostname. Host-segment indicator wins here
        // (multi-part hostname is enough).
        let findings = scan_prod_values(
            ".env.local",
            "API_HOST=https://production.acme.io/quickstart\n",
        );
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn dev_host_short_circuits() {
        // Council + adversarial both flagged: doc claimed `dev` was a
        // short-circuit token but the array didn't include it. With
        // the bounded `dev-` / `.dev` / `dev.` tokens, a dev-host URL
        // must short-circuit even when prod-shaped values appear.
        let findings = scan_prod_values(
            ".env.local",
            "DATABASE_URL=postgres://dev-db.internal/app\n",
        );
        assert!(findings.is_empty(), "got {findings:?}");

        let findings = scan_prod_values(".env.local", "API_HOST=https://api.dev.acme.io/v1\n");
        assert!(findings.is_empty(), "got {findings:?}");
    }

    #[test]
    fn dev_substring_inside_word_does_not_short_circuit() {
        // The bounded `dev-`/`.dev`/`dev.` tokens must NOT match a
        // bare `dev` substring like `developer-tools` — otherwise the
        // dev-token carve-out becomes a false-negative vector.
        let findings = scan_prod_values(
            ".env.local",
            "DATABASE_URL=postgres://prod-db.acme.io/developer-tools\n",
        );
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn does_not_match_reproduction() {
        // Word boundary check: `reproduction` must NOT trip the rule.
        let findings = scan_prod_values(".env.local", "DOC_TITLE=reproduction-of-bug\n");
        assert!(findings.is_empty());
    }

    #[test]
    fn flags_prod_dash_host_segment() {
        let findings = scan_prod_values(
            ".env.local",
            "DATABASE_URL=postgres://prod-db.internal/app\n",
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].indicator, ProdIndicator::ProdHostSegment);
    }

    #[test]
    fn flags_dot_production_dot_host() {
        // Use a realistic production-domain placeholder rather than
        // `example.com` — `.example` is an explicit non-prod token, so
        // an `example.com` host short-circuits the rule even when a
        // `production` segment is present (intentional carve-out for
        // IETF-reserved docs domains).
        let findings = scan_prod_values(".env.local", "API_HOST=api.production.acme.io\n");
        assert_eq!(findings.len(), 1);
        // Host-segment check fires first (higher precedence than the
        // word check) — but both should agree on "this is prod".
        assert_eq!(findings[0].indicator, ProdIndicator::ProdHostSegment);
    }

    #[test]
    fn flags_key_suffix_prod_with_value() {
        let findings =
            scan_prod_values(".env.local", "DATABASE_URL_PROD=postgres://elsewhere/db\n");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].indicator, ProdIndicator::KeySuffixProd);
    }

    #[test]
    fn key_suffix_with_empty_value_does_not_fire() {
        let findings = scan_prod_values(".env.local", "DATABASE_URL_PROD=\n");
        assert!(findings.is_empty());
    }

    #[test]
    fn staging_short_circuit() {
        let findings = scan_prod_values(
            ".env.local",
            "DATABASE_URL=postgres://prod-db.staging.example.com/app\n",
        );
        // `staging` token in the value short-circuits — the operator
        // already announced non-prod intent.
        assert!(
            findings.is_empty(),
            "got {findings:?}; expected staging short-circuit"
        );
    }

    #[test]
    fn localhost_short_circuit() {
        let findings = scan_prod_values(
            ".env.local",
            "DATABASE_URL=postgres://localhost:5432/prod-db\n",
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn directive_on_previous_line_suppresses() {
        let content = format!(
            "# @anvil-ignore {SURFENV_003_RULE_ID} -- intentional prod replay\n\
             DATABASE_URL=postgres://prod-db.internal/app\n"
        );
        let findings = scan_prod_values(".env.local", &content);
        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert!(f.suppressed);
        assert_eq!(
            f.suppression_reason.as_deref(),
            Some("intentional prod replay")
        );
    }

    #[test]
    fn directive_for_other_rule_does_not_suppress() {
        let content = "\
# @anvil-ignore SURFENV-001 -- unrelated
DATABASE_URL=postgres://prod-db.internal/app
";
        let findings = scan_prod_values(".env.local", content);
        assert_eq!(findings.len(), 1);
        assert!(!findings[0].suppressed);
    }

    #[test]
    fn redaction_does_not_leak_full_value() {
        let findings = scan_prod_values(".env.local", "SECRET_PROD=ABCDEFGHIJKLMNOPQRSTUVWXYZ\n");
        assert_eq!(findings.len(), 1);
        let excerpt = &findings[0].redacted_excerpt;
        assert!(!excerpt.contains("ABCDEFGHIJKLMNOPQRSTUVWXYZ"));
        assert!(excerpt.contains("…") || excerpt.contains("[redacted]"));
    }
}
