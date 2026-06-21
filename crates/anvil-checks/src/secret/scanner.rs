use std::sync::LazyLock;

use regex::Regex;

use crate::secret::entropy::detect_high_entropy_strings_with_line_filter_and_limit;
use crate::secret::patterns::{
    CompiledPattern, DEFAULT_COMPILED_PATTERNS, PatternMatcher, compile_custom_patterns,
};
use crate::secret::types::{
    AllowlistProvenance, FindingType, SecretCheckConfig, SecretFinding, Suppression,
};

/// Matches a credential embedded in a URL's userinfo: `scheme://user:secret@host`.
///
/// The colon between `user` and `secret` is the load-bearing signal — it
/// matches basic-auth credentials (`https://user:token@registry`) without
/// matching a bare username like `git@github.com` (no colon), which appears
/// throughout lockfiles and would otherwise be a false positive. A token-only
/// userinfo (`https://TOKEN@host`, no colon) is intentionally not matched for
/// the same reason; npm keeps such tokens in `.npmrc`, which is scanned in full.
static CREDENTIAL_URL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[a-zA-Z][a-zA-Z0-9+.\-]*://[^/\s:@]+:[^/\s@]+@")
        .expect("credential URL pattern is valid")
});

/// Scan a dependency lockfile for credentials embedded in URLs, and nothing
/// else (GH #2584).
///
/// Lockfiles pin resolved dependencies and carry integrity hashes that are
/// high-entropy by construction; running them through the full secret scan —
/// especially the entropy detector — produces only false positives. The one
/// real secret class that legitimately appears in a lockfile is a credential in
/// a `resolved`/source URL (`https://user:token@registry/…`), so this scans for
/// exactly that: no entropy pass, no other patterns.
#[must_use]
pub fn scan_lockfile_url_credentials(
    content: &str,
    file_path: &str,
    limit: usize,
) -> Vec<SecretFinding> {
    if limit == 0 {
        return Vec::new();
    }
    // No custom allowlist: the pattern only fires on `user:pass@` userinfo,
    // which has no benign lockfile form that would need allowlisting.
    let matcher = PatternMatcher::new(&[]);
    let mut findings = Vec::new();
    for (index, line) in content.lines().enumerate() {
        for matched in CREDENTIAL_URL_PATTERN.find_iter(line) {
            let range = matched.range();
            findings.push(SecretFinding {
                file: file_path.to_string(),
                line: index + 1,
                finding_type: FindingType::Pattern,
                pattern_name: "Credential URL".to_string(),
                redacted_match: matcher.redact_secret(&line[range.clone()]),
                redacted_line: matcher.redact_range_in_line(line, range.start, range.end),
            });
            if findings.len() == limit {
                return findings;
            }
        }
    }
    findings
}

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
        // `-`/digit: inside a longer dashed/numeric token (UUID). `.`: the
        // matched digits are the fractional part of a decimal literal (the
        // external-FP dogfood hit float coordinate arrays like
        // `[3.5475921483286084, …]`).
        if prev == b'-' || prev == b'.' || prev.is_ascii_digit() {
            return true;
        }
    }
    if match_end < bytes.len() {
        let next = bytes[match_end];
        if next == b'-' || next.is_ascii_digit() {
            return true;
        }
    }
    // A real card number satisfies the Luhn checksum. Arbitrary 16-digit runs
    // (coordinates, ids, hashes) almost never do, so a Luhn failure marks the
    // match as a false positive (external-FP dogfood: 119 coordinate FPs).
    let matched = &line[match_start..match_end];
    !luhn_valid(matched)
}

/// Luhn (mod-10) checksum over the digits in `s`. Any non-digit character
/// (grouping separators such as `-` or spaces, and anything else) is skipped.
/// Input with no digits is not valid.
fn luhn_valid(s: &str) -> bool {
    let mut sum = 0u32;
    let mut count = 0u32;
    for d in s.bytes().rev().filter(u8::is_ascii_digit) {
        let mut v = u32::from(d - b'0');
        if count % 2 == 1 {
            v *= 2;
            if v > 9 {
                v -= 9;
            }
        }
        sum += v;
        count += 1;
    }
    count > 0 && sum.is_multiple_of(10)
}

fn is_js_identifier_path(value: &str) -> bool {
    value.split('.').all(|part| {
        let mut chars = part.chars();
        chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

fn is_likely_code_identifier_path(value: &str) -> bool {
    is_js_identifier_path(value)
        && (value.starts_with("config.")
            || value.starts_with("env.")
            || value.starts_with("options.")
            || value.starts_with("process.")
            || value.starts_with("secrets.")
            || value.split('.').any(|part| {
                part.chars()
                    .any(|c| c.is_ascii_digit() || c == '_' || c.is_ascii_uppercase())
            }))
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

    let quoted = (rhs.starts_with('"') && rhs.ends_with('"'))
        || (rhs.starts_with('\'') && rhs.ends_with('\''));

    // Strip outer matched quotes — `"hunter2"` should be evaluated as
    // `hunter2`, not as starting with `"`. Trailing terminators like
    // `;`, `,` are noise from how the regex captures up to the next
    // whitespace; strip them too.
    let unquoted = rhs
        .strip_prefix(['"', '\''])
        .and_then(|s| s.strip_suffix(['"', '\'']))
        .unwrap_or(rhs)
        .trim_end_matches([';', ',']);

    if unquoted.starts_with("process.env.") {
        return true;
    }

    // Code-structural characters that rarely appear in a real secret
    // literal: TS type closures (`string)`), bracket env access
    // (`process.env['X']`), template substitutions (`${dbPassword}`),
    // function calls (`requireSecret(...)`), and similar. Dots are only
    // rejected for unquoted identifier paths (`config.password`) so
    // generated dotted passwords remain detectable.
    let has_code_shape = unquoted
        .chars()
        .any(|c| matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | '`'));
    if has_code_shape {
        return true;
    }

    let dotted_identifier_path =
        !quoted && unquoted.contains('.') && is_likely_code_identifier_path(unquoted);
    if dotted_identifier_path {
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

/// Why a pattern match was withheld. Distinguishes a deliberate *allowlist*
/// suppression (recorded with provenance so it can be surfaced) from a
/// heuristic non-match (a value the scanner judged not to be a real secret —
/// `looks_like_code`, generic-secret / credit-card false-positive filters),
/// which is not a suppression worth reporting.
enum SkipReason {
    Allowlisted(AllowlistProvenance),
    Heuristic,
}

fn pattern_skip_reason(
    pattern: &CompiledPattern,
    matcher: &PatternMatcher,
    line: &str,
    match_start: usize,
    match_end: usize,
) -> Option<SkipReason> {
    let matched_value = &line[match_start..match_end];

    if pattern.high_confidence {
        // The pattern is itself the credential shape — fuzzy keyword
        // suppression (`example`, `test`, `dummy`) and the all-caps
        // `looks_like_code` heuristic both defeat textbook credentials
        // like the canonical AWS `AKIAIOSFODNN7EXAMPLE` access key
        // (issue #1800). Only shape-anchored allowlist entries and
        // user-supplied `custom_allowlist` opt-outs apply here.
        return matcher
            .matched_shape_or_custom(matched_value)
            .map(SkipReason::Allowlisted);
    }

    if let Some(provenance) = matcher.matched_allowlist(matched_value) {
        return Some(SkipReason::Allowlisted(provenance));
    }

    let heuristic_skip = matcher.looks_like_code(matched_value)
        || (pattern.name == "Generic Secret" && is_generic_secret_false_positive(matched_value))
        || (pattern.name == "Credit Card"
            && is_credit_card_false_positive(line, match_start, match_end));
    heuristic_skip.then_some(SkipReason::Heuristic)
}

/// Decide whether a pattern match should be skipped, recording an allowlist
/// suppression (with provenance) into `stats` when it is. Returns `true` when
/// the match must not become a finding (allowlisted *or* heuristic non-match).
#[allow(clippy::too_many_arguments)] // a focused skip-and-record seam
fn skip_and_record(
    stats: &mut ScanStats,
    matcher: &PatternMatcher,
    pattern: &CompiledPattern,
    file_path: &str,
    line: &str,
    line_number: usize,
    range: &std::ops::Range<usize>,
) -> bool {
    match pattern_skip_reason(pattern, matcher, line, range.start, range.end) {
        Some(SkipReason::Allowlisted(provenance)) => {
            // Don't drop it silently — record what was suppressed and which
            // allowlist tier did it.
            stats.suppressions.push(Suppression {
                file: file_path.to_string(),
                line: line_number,
                rule_name: pattern.name.clone(),
                redacted_match: matcher.redact_secret(&line[range.clone()]),
                provenance,
            });
            true
        }
        Some(SkipReason::Heuristic) => true,
        None => false,
    }
}

/// Boolean form for callers that only need "would this match be skipped?"
/// (e.g. the entropy line-filter), without recording a suppression.
fn should_skip_pattern_match(
    pattern: &CompiledPattern,
    matcher: &PatternMatcher,
    line: &str,
    match_start: usize,
    match_end: usize,
) -> bool {
    pattern_skip_reason(pattern, matcher, line, match_start, match_end).is_some()
}

/// CIB-063: one credential, one finding. A low-confidence keyword pattern
/// often wraps the same value a high-confidence shape pattern identifies
/// precisely (`OPENAI_API_KEY = "sk-proj-…"` matches both `API Key` and
/// `OpenAI API Key`). A low-confidence match overlapping a high-confidence
/// one on the same line is the same credential seen twice — report it once,
/// under the provider-specific pattern. Custom patterns always compile with
/// `high_confidence: false`, so they too yield to an overlapping built-in
/// shape match; the credential still reports, just under the precise rule.
pub(crate) fn suppressed_by_high_confidence_overlap(
    pattern: &CompiledPattern,
    range: &std::ops::Range<usize>,
    line_matches: &[(&CompiledPattern, std::ops::Range<usize>)],
) -> bool {
    !pattern.high_confidence
        && line_matches.iter().any(|(other, other_range)| {
            other.high_confidence && other_range.start < range.end && range.start < other_range.end
        })
}

/// SCAN-002: per-call stats returned alongside findings. Currently exposes
/// the count of lines that exceeded `SecretCheckConfig::max_line_bytes` and
/// were therefore skipped before regex evaluation.
#[derive(Debug, Default, Clone)]
pub struct ScanStats {
    /// Number of lines skipped because they exceeded the per-line length
    /// guard. A non-zero value means a pathological line was present and
    /// neither pattern matching nor entropy scanning ran for it.
    pub lines_skipped_oversize: usize,
    /// Candidates that matched a secret rule but were withheld because they
    /// also matched an allowlist entry, with the provenance of the entry that
    /// suppressed them. Lets callers surface "we suppressed N would-be
    /// secrets via the allowlist" instead of dropping them silently.
    pub suppressions: Vec<Suppression>,
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
///
/// V050F-011: this entry point recompiles `config.custom_patterns`
/// every call. The errors are dropped from the return value for
/// back-compat (the signature is fixed) but they are NOT silent —
/// any dropped errors are surfaced via a single aggregated
/// `tracing::warn!` per call so the silent-loss path is observable
/// without flooding logs across N files × M errors.
///
/// Hot-path callers (e.g. `run_secret_check`) should compile once
/// via [`compile_custom_patterns`] and route through
/// [`scan_content_with_compiled_patterns`] instead so the per-file
/// recompile cost goes away AND the errors are returned to the
/// caller verbatim. New callers that need the errors in-signature
/// should use [`scan_content_with_pattern_errors_and_stats`].
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
    let (custom_patterns, custom_errors) = compile_custom_patterns(&config.custom_patterns);
    if !custom_errors.is_empty() {
        // V050F-011: legacy entry points cannot return the errors in
        // their signature without a breaking change. Surface via a
        // single aggregated tracing event per call so the silent-loss
        // path is observable but a caller scanning K files with M
        // broken patterns logs K events, not K×M (council finding:
        // copilot, log-spam concern). New callers should use
        // `scan_content_with_compiled_patterns` (no recompile, no
        // logging) or `scan_content_with_pattern_errors_and_stats`
        // (errors in the return tuple) so the contract is enforced
        // by the function signature.
        tracing::warn!(
            file = %file_path,
            error_count = custom_errors.len(),
            errors = ?custom_errors,
            "scan_content dropped {} custom-pattern compile error(s); use \
             `scan_content_with_pattern_errors_and_stats` to receive them",
            custom_errors.len(),
        );
    }
    scan_content_with_compiled_patterns(content, file_path, config, &custom_patterns, limit)
}

/// V050F-011: hot-path scan that takes pre-compiled custom patterns
/// so the per-call recompile cost is paid once at check setup, not
/// once per file. Use this from any flow that calls the scanner in a
/// loop or in parallel.
///
/// Compile errors are caller-owned: get them from
/// [`compile_custom_patterns`] (or [`compile_secret_patterns`])
/// once upfront and surface them at the boundary that owns
/// configuration. Built-in patterns are sourced from the per-process
/// [`DEFAULT_COMPILED_PATTERNS`] cache.
pub fn scan_content_with_compiled_patterns(
    content: &str,
    file_path: &str,
    config: &SecretCheckConfig,
    custom_patterns: &[CompiledPattern],
    limit: usize,
) -> (Vec<SecretFinding>, ScanStats) {
    let matcher = PatternMatcher::new(&config.custom_allowlist);
    let default_patterns: &[CompiledPattern] = &DEFAULT_COMPILED_PATTERNS;
    let mut findings = Vec::new();
    let mut stats = ScanStats::default();

    if limit == 0 {
        return (findings, stats);
    }

    // Lockfiles get a restricted scan: only credentials embedded in URLs, never
    // the entropy detector (their integrity hashes are entropy false positives,
    // GH #2584). Centralised here so every scan surface — discovery, `anvil
    // check`/`gate`/`audit`, and the save-time intercept — treats lockfiles
    // identically without each having to special-case them.
    if crate::filter::is_lockfile(std::path::Path::new(file_path)) {
        return (
            scan_lockfile_url_credentials(content, file_path, limit),
            stats,
        );
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

        let mut line_matches: Vec<(&CompiledPattern, std::ops::Range<usize>)> = Vec::new();
        for pattern in patterns_iter() {
            for matched_range in pattern.regex.find_iter(line) {
                let range = matched_range.range();
                if skip_and_record(
                    &mut stats,
                    &matcher,
                    pattern,
                    file_path,
                    line,
                    line_number,
                    &range,
                ) {
                    continue;
                }
                line_matches.push((pattern, range));
            }
        }

        for (pattern, range) in &line_matches {
            if suppressed_by_high_confidence_overlap(pattern, range, &line_matches) {
                continue;
            }

            let matched_value = &line[range.clone()];
            findings.push(SecretFinding {
                file: file_path.to_string(),
                line: line_number,
                finding_type: FindingType::Pattern,
                pattern_name: pattern.name.clone(),
                redacted_match: matcher.redact_secret(matched_value),
                redacted_line: matcher.redact_range_in_line(line, range.start, range.end),
            });
            if findings.len() == limit {
                return (findings, stats);
            }
        }
    }

    if config.enable_entropy {
        let remaining = limit.saturating_sub(findings.len());
        let mut entropy_suppressions = Vec::new();
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
                patterns_iter().all(|pattern| {
                    pattern.regex.find_iter(line).all(|matched_range| {
                        should_skip_pattern_match(
                            pattern,
                            &matcher,
                            line,
                            matched_range.start(),
                            matched_range.end(),
                        )
                    })
                })
            },
            &mut entropy_suppressions,
        );
        findings.extend(entropy_findings);
        stats.suppressions.append(&mut entropy_suppressions);
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
/// count, or `scan_content_with_pattern_errors_and_stats` if the
/// caller wants to surface custom-pattern compile errors directly
/// (V050F-011) instead of relying on the `tracing::warn!` log path.
pub fn scan_content(
    content: &str,
    file_path: &str,
    config: &SecretCheckConfig,
) -> Vec<SecretFinding> {
    scan_content_with_stats(content, file_path, config).0
}

/// V050F-011: scan and surface custom-pattern compile errors in the
/// return tuple so third-party callers can route them into their
/// own diagnostic / config-validation surface instead of relying on
/// the `tracing::warn!` log emitted by [`scan_content_with_stats`].
/// Compiles the patterns once per call; for repeated scans, reuse the
/// compiled slice via [`scan_content_with_compiled_patterns`].
pub fn scan_content_with_pattern_errors_and_stats(
    content: &str,
    file_path: &str,
    config: &SecretCheckConfig,
) -> (Vec<SecretFinding>, ScanStats, Vec<String>) {
    let (custom_patterns, errors) = compile_custom_patterns(&config.custom_patterns);
    let (findings, stats) = scan_content_with_compiled_patterns(
        content,
        file_path,
        config,
        &custom_patterns,
        usize::MAX,
    );
    (findings, stats, errors)
}

#[cfg(test)]
mod tests {
    use crate::secret::scanner::{
        luhn_valid, scan_content, scan_content_with_compiled_patterns,
        scan_content_with_pattern_errors_and_stats, scan_content_with_stats,
    };
    use crate::secret::types::{AllowlistProvenance, FindingType, SecretCheckConfig};

    // ── Suppression provenance: allowlisted candidates are recorded, not
    //    silently dropped (so a `.anvilrc` opt-out can never hide a real
    //    credential without the operator seeing it called out). ───────────

    #[test]
    fn custom_allowlist_records_a_suppression_with_provenance() {
        // A high-entropy token covered by a user `custom_allowlist` entry is
        // withheld from findings but surfaced as a `Custom` suppression that
        // names the exact opt-out pattern.
        let config = SecretCheckConfig {
            entropy_threshold: 3.5,
            custom_allowlist: vec!["9xY7qW2vK8mN4pR6".to_string()],
            ..SecretCheckConfig::default()
        };
        let content = "const token = '9xY7qW2vK8mN4pR6sT1uV3wX';";
        let (findings, stats) = scan_content_with_stats(content, "src/auth.ts", &config);

        assert!(
            findings.is_empty(),
            "allowlisted value must not be a finding, got: {findings:?}"
        );
        // The entropy detector matches both its quoted and assignment forms,
        // so the low-level scan records the candidate once per form (the same
        // pre-dedup duplication the findings path has). Assert on properties,
        // not an exact count.
        assert!(
            !stats.suppressions.is_empty(),
            "suppression must be recorded"
        );
        let s = &stats.suppressions[0];
        assert_eq!(s.rule_name, "High Entropy String");
        assert_eq!(s.file, "src/auth.ts");
        assert_eq!(
            s.provenance,
            AllowlistProvenance::Custom {
                pattern: "9xY7qW2vK8mN4pR6".to_string()
            }
        );
        assert!(s.provenance.is_operator_configured());
        assert!(
            stats
                .suppressions
                .iter()
                .all(|s| !s.redacted_match.contains("9xY7qW2vK8mN4pR6sT1uV3wX")),
            "raw value must not leak into the suppression record"
        );
    }

    #[test]
    fn high_confidence_pattern_suppressed_by_custom_allowlist_is_recorded() {
        // The canonical AWS key is a high-confidence shape; a user opt-out
        // still withholds it, but the suppression is recorded so it is never
        // silent — under the precise rule name.
        let config = SecretCheckConfig {
            custom_allowlist: vec!["AKIAIOSFODNN7EXAMPLE".to_string()],
            ..SecretCheckConfig::default()
        };
        let content = "const k = \"AKIAIOSFODNN7EXAMPLE\";";
        let (findings, stats) = scan_content_with_stats(content, "src/aws.ts", &config);

        assert!(
            !findings.iter().any(|f| f.pattern_name == "AWS Key"),
            "allowlisted AWS key must be withheld"
        );
        assert!(
            stats
                .suppressions
                .iter()
                .any(|s| s.rule_name == "AWS Key" && s.provenance.is_operator_configured()),
            "AWS-key suppression must be recorded with Custom provenance, got: {:?}",
            stats.suppressions
        );
    }

    #[test]
    fn unmatched_secret_produces_no_suppression() {
        // A genuine secret with no allowlist coverage flags normally and
        // records nothing as suppressed.
        let config = SecretCheckConfig {
            entropy_threshold: 3.5,
            ..SecretCheckConfig::default()
        };
        let content = "const token = '9xY7qW2vK8mN4pR6sT1uV3wX';";
        let (findings, stats) = scan_content_with_stats(content, "src/auth.ts", &config);
        assert!(!findings.is_empty(), "unallowlisted secret must flag");
        assert!(
            stats.suppressions.is_empty(),
            "nothing should be recorded as suppressed, got: {:?}",
            stats.suppressions
        );
    }

    // Issue #1800 — textbook AWS access keys (`AKIA…`) used to be
    // suppressed by both `looks_like_code` (`^[A-Z][A-Z0-9_]+$`) and the
    // keyword allowlist (`example` substring inside `AKIAIOSFODNN7EXAMPLE`).
    // High-confidence patterns now bypass both filters.

    #[test]
    fn flags_textbook_aws_access_key_id_from_aws_docs() {
        let config = SecretCheckConfig::default();
        // Canonical AWS-documentation literal — flag despite the `EXAMPLE`
        // suffix in the matched value.
        let content = "const k = \"AKIAIOSFODNN7EXAMPLE\";";
        let findings = scan_content(content, "src/aws.ts", &config);
        assert!(
            findings.iter().any(|f| f.pattern_name == "AWS Key"),
            "AKIAIOSFODNN7EXAMPLE must flag as AWS Key, got: {:?}",
            findings.iter().map(|f| &f.pattern_name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn flags_textbook_aws_pair_in_typescript_module() {
        // Reproduces the fixture from issue #1800: the canonical AWS
        // documentation key pair embedded in an ES module.
        let config = SecretCheckConfig::default();
        let content = "\
const k = \"AKIAIOSFODNN7EXAMPLE\";
const s = \"wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY\";
export function go(){return [k,s];}";
        let findings = scan_content(content, "src/aws.ts", &config);
        assert!(
            findings.iter().any(|f| f.pattern_name == "AWS Key"),
            "AWS access key id must flag, got: {:?}",
            findings.iter().map(|f| &f.pattern_name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn flags_aws_sts_temporary_access_key() {
        // STS temporary access keys share the AKIA-style shape but with
        // the `ASIA` prefix — they're still real AWS credentials.
        let config = SecretCheckConfig::default();
        let content = "AWS_ACCESS_KEY_ID=ASIAIOSFODNN7EXAMPLE";
        let findings = scan_content(content, ".env", &config);
        assert!(
            findings.iter().any(|f| f.pattern_name == "AWS STS Key"),
            "ASIA… key must flag as AWS STS Key, got: {:?}",
            findings.iter().map(|f| &f.pattern_name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn flags_stripe_test_key_despite_test_keyword() {
        // The keyword allowlist used to suppress `sk_test_…` matches
        // because the substring "test" hits the allowlist. Stripe test
        // keys are still real Stripe API credentials and must surface.
        let config = SecretCheckConfig::default();
        let stripe_test = format!("sk_test_{}", "1234567890abcdefghijABCD");
        let content = format!("const key = '{stripe_test}';");
        let findings = scan_content(&content, "src/stripe.ts", &config);
        assert!(
            findings.iter().any(|f| f.pattern_name == "Stripe Test Key"),
            "sk_test_… must flag as Stripe Test Key, got: {:?}",
            findings.iter().map(|f| &f.pattern_name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn flags_anthropic_api_key() {
        let config = SecretCheckConfig::default();
        let key = format!("sk-ant-{}", "abcdefghijklmnopqrstuvwxyz012345");
        let content = format!("export const ANTHROPIC_KEY = '{key}';");
        let findings = scan_content(&content, "src/claude.ts", &config);
        assert!(
            findings
                .iter()
                .any(|f| f.pattern_name == "Anthropic API Key"),
            "sk-ant-… key must flag, got: {:?}",
            findings.iter().map(|f| &f.pattern_name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn flags_openai_project_api_key() {
        let config = SecretCheckConfig::default();
        let key = format!("sk-proj-{}", "abcdefghijklmnopqrst");
        let content = format!("export const OPENAI_KEY = '{key}';");
        let findings = scan_content(&content, "src/openai.ts", &config);
        assert!(
            findings.iter().any(|f| f.pattern_name == "OpenAI API Key"),
            "sk-proj-… key must flag, got: {:?}",
            findings.iter().map(|f| &f.pattern_name).collect::<Vec<_>>()
        );
    }

    // CIB-063 — one credential, one finding. `OPENAI_API_KEY = "sk-proj-…"`
    // matched both the low-confidence `API Key` keyword pattern and the
    // high-confidence `OpenAI API Key` shape pattern, double-reporting the
    // same planted string (SECRET-API-KEY + SECRET-OPENAI-API-KEY) on the
    // beta golden path.

    #[test]
    fn overlapping_keyword_match_dedups_to_the_provider_pattern() {
        let config = SecretCheckConfig::default();
        let key = format!("sk-proj-{}", "abcdefghijklmnopqrst");
        let content = format!("const OPENAI_API_KEY = \"{key}\";");
        let findings = scan_content(&content, "src/config.ts", &config);
        let names: Vec<_> = findings.iter().map(|f| f.pattern_name.as_str()).collect();
        assert_eq!(
            names,
            vec!["OpenAI API Key"],
            "one credential must report once, under the provider pattern"
        );
    }

    #[test]
    fn non_overlapping_matches_on_one_line_both_report() {
        let config = SecretCheckConfig {
            enable_entropy: false,
            ..SecretCheckConfig::default()
        };
        let key = format!("sk-proj-{}", "abcdefghijklmnopqrst");
        let content = format!("api_key='abcdEFGH1234567890'; const OTHER = '{key}';");
        let findings = scan_content(&content, "src/config.ts", &config);
        let names: Vec<_> = findings.iter().map(|f| f.pattern_name.as_str()).collect();
        assert!(
            names.contains(&"API Key") && names.contains(&"OpenAI API Key"),
            "distinct values on one line keep their own findings, got: {names:?}"
        );
    }

    #[test]
    fn custom_allowlist_still_suppresses_high_confidence_pattern() {
        // The operator escape hatch: `custom_allowlist` opt-out must
        // continue to suppress findings even after the high-confidence
        // bypass — otherwise legitimate documentation/test fixtures
        // can't be excluded.
        let config = SecretCheckConfig {
            custom_allowlist: vec!["AKIAIOSFODNN7EXAMPLE".to_string()],
            ..SecretCheckConfig::default()
        };
        let content = "const k = \"AKIAIOSFODNN7EXAMPLE\";";
        let findings = scan_content(content, "src/aws_docs.ts", &config);
        assert!(
            !findings.iter().any(|f| f.pattern_name == "AWS Key"),
            "custom_allowlist must still suppress, got: {:?}",
            findings.iter().map(|f| &f.pattern_name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn keyword_allowlist_still_suppresses_low_confidence_pattern() {
        // Regression guard: the keyword allowlist must continue to
        // suppress fuzzy patterns like `API Key` that catch generic
        // documentation values.
        let config = SecretCheckConfig {
            enable_entropy: false,
            ..SecretCheckConfig::default()
        };
        let content = "apiKey: 'placeholder-api-key-for-testing'";
        let findings = scan_content(content, "src/config.ts", &config);
        assert!(
            findings.is_empty(),
            "placeholder API key must still be suppressed, got: {:?}",
            findings.iter().map(|f| &f.pattern_name).collect::<Vec<_>>()
        );
    }

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

    #[test]
    fn does_not_flag_float_coordinate_array_as_credit_card() {
        // External-FP dogfood (excalidraw initialData): float coordinate arrays
        // produced 119 Credit Card false positives — the 16-digit fractional
        // part of a decimal is preceded by `.` and is not Luhn-valid.
        let config = SecretCheckConfig::default();
        let content = "const points = [3.5475921483286084, -47.099726468136254];";
        let findings = scan_content(content, "src/initialData.js", &config);
        assert!(
            !findings.iter().any(|f| f.pattern_name == "Credit Card"),
            "float coordinates must not be flagged as Credit Card, got: {:?}",
            findings
                .iter()
                .find(|f| f.pattern_name == "Credit Card")
                .map(|f| &f.redacted_line)
        );
    }

    #[test]
    fn does_not_flag_luhn_invalid_16_digit_run() {
        // A standalone 16-digit number that fails the Luhn checksum is an id /
        // nonce, not a card.
        let config = SecretCheckConfig::default();
        let content = "const id = 1234567812345678;";
        let findings = scan_content(content, "src/ids.ts", &config);
        assert!(
            !findings.iter().any(|f| f.pattern_name == "Credit Card"),
            "Luhn-invalid 16-digit run must not be flagged as Credit Card"
        );
    }

    #[test]
    fn luhn_valid_accepts_test_cards_and_rejects_noise() {
        assert!(luhn_valid("4242 4242 4242 4242"));
        assert!(luhn_valid("4111-1111-1111-1111"));
        assert!(!luhn_valid("3547592148328608"));
        assert!(!luhn_valid(""));
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
        // From anvil-archive/anvil-cli-node/src/commands/policy/bundle.ts:296
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
    fn still_flags_real_dotted_secret_literal() {
        let config = SecretCheckConfig::default();
        let content = "PASSWORD=correct.horse.battery.staple";
        let findings = scan_content(content, ".env", &config);
        assert!(
            findings.iter().any(|f| f.pattern_name == "Generic Secret"),
            "dotted env-style secret should still fire, got: {:?}",
            findings.iter().map(|f| &f.pattern_name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn does_not_flag_dotted_code_path_with_digits_as_secret() {
        let config = SecretCheckConfig::default();
        let content = "const clientSecret = config.oauth2Secret;";
        let findings = scan_content(content, "src/config.ts", &config);
        let generic = findings.iter().find(|f| f.pattern_name == "Generic Secret");
        assert!(
            generic.is_none(),
            "dotted code path should not be flagged, got: {:?}",
            generic.map(|f| &f.redacted_line)
        );
    }

    #[test]
    fn generic_false_positive_does_not_hide_later_pattern_match_on_same_line() {
        let config = SecretCheckConfig::default();
        let content = "const authSecret = process.env.CRON_SECRET; password='realSecret123456';";
        let findings = scan_content(content, "src/test.ts", &config);
        assert!(
            findings.iter().any(|f| f.pattern_name == "Generic Secret"),
            "later same-line generic secret should still fire, got: {findings:?}"
        );
    }

    #[test]
    fn generic_false_positive_does_not_block_entropy_scan() {
        let config = SecretCheckConfig {
            entropy_threshold: 3.0,
            ..SecretCheckConfig::default()
        };
        let content = "const authSecret = process.env.CRON_SECRET; token='9xY7qW2vK8mN4pR6'";
        let findings = scan_content(content, "src/test.ts", &config);
        assert!(
            findings
                .iter()
                .any(|f| f.finding_type == FindingType::Entropy),
            "skipped generic-secret matches should not suppress entropy findings, got: {findings:?}"
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

    // V050F-011 — surface custom-pattern compile errors and remove
    // the per-call recompile from the hot path. Pin the new API
    // contract so a future refactor cannot silently regress it.

    #[test]
    fn pattern_errors_surfaced_in_return_tuple() {
        // A custom pattern with an invalid regex must produce an
        // error string that the caller can route into its diagnostic
        // surface, NOT a silently-dropped error.
        let config = SecretCheckConfig {
            custom_patterns: vec![crate::secret::types::SecretPatternDef {
                name: "Broken".to_string(),
                pattern: "(unclosed".to_string(),
            }],
            ..SecretCheckConfig::default()
        };
        let (_findings, _stats, errors) =
            scan_content_with_pattern_errors_and_stats("hello", "src/x.ts", &config);
        assert_eq!(
            errors.len(),
            1,
            "the broken custom pattern must produce exactly one error, got {errors:?}",
        );
        assert!(
            errors[0].contains("Broken") && errors[0].contains("failed to compile"),
            "error must name the pattern and the failure mode, got {errors:?}",
        );
    }

    #[test]
    fn pattern_errors_returned_empty_on_clean_compile() {
        let config = SecretCheckConfig::default();
        let (_findings, _stats, errors) =
            scan_content_with_pattern_errors_and_stats("hello", "src/x.ts", &config);
        assert!(
            errors.is_empty(),
            "default config has no custom patterns, so errors must be empty, got {errors:?}",
        );
    }

    #[test]
    fn compiled_patterns_path_does_not_recompile() {
        // The hot-path API takes a pre-compiled slice — invoking it
        // twice with the same slice exercises the no-recompile
        // contract. Directly observable via `Regex::as_str()`
        // identity is fragile; instead pin the structural contract:
        // the API takes `&[CompiledPattern]` so callers can share the
        // slice across rayon workers without rebuilding it.
        let config = SecretCheckConfig::default();
        let (compiled, _errors) =
            crate::secret::patterns::compile_custom_patterns(&config.custom_patterns);
        let (findings_a, _stats_a) = scan_content_with_compiled_patterns(
            "api_key='abcdEFGH1234567890'",
            "src/a.ts",
            &config,
            &compiled,
            usize::MAX,
        );
        let (findings_b, _stats_b) = scan_content_with_compiled_patterns(
            "api_key='abcdEFGH1234567890'",
            "src/b.ts",
            &config,
            &compiled,
            usize::MAX,
        );
        assert!(!findings_a.is_empty());
        assert!(!findings_b.is_empty());
    }

    #[test]
    fn legacy_scan_content_still_returns_findings_with_broken_pattern() {
        // Legacy callers don't see the error in the return value, but
        // a broken custom pattern must NOT prevent the built-in
        // patterns from firing. The skipped-broken-pattern path is
        // observable via tracing::warn!; the findings array must
        // still carry built-in matches.
        let config = SecretCheckConfig {
            custom_patterns: vec![crate::secret::types::SecretPatternDef {
                name: "Broken".to_string(),
                pattern: "(unclosed".to_string(),
            }],
            ..SecretCheckConfig::default()
        };
        let findings = scan_content("api_key='abcdEFGH1234567890'", "src/test.ts", &config);
        assert!(
            !findings.is_empty(),
            "broken custom pattern must not suppress built-in matches"
        );
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
