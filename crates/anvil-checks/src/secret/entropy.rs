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
            // Structural only (calls, URLs, extensions, SCREAMING_SNAKE).
            // CamelCase / PascalCase used to drop ~half of mixed-case
            // opaque tokens (Dave B16 / CIB-340). Named-pattern scanning
            // still uses the full `looks_like_code` set.
            if crate::secret::patterns::looks_like_structural_code(candidate) {
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
/// Heuristic (Dave SEC-FP-1, pack 05 / 05a; Git Bash drive, CIB-339;
/// bare filename, CIB-348):
/// - the token itself or the characters immediately after the match end in
///   a known document extension (a separator is not required — a bare
///   `quarterly-revenue-forecast-summary.md` is still a document path), **or**
/// - token contains a path separator (`/` or `\`) and either:
///   - the match is introduced by a Windows drive-letter colon (`D:…`) which
///     the assignment regex otherwise treats as a secret assignment, **or**
///   - the token itself starts with a POSIX / Git Bash drive prefix (`/c/…`).
///
/// CIB-323 also treats a match that sits in an `http(s)://` URL path as
/// path-shaped, but only when the authority has a syntactically valid host.
/// Empty-host (`https:///…`) and delimiter-broken authorities are not
/// path-shaped (#3917). Named vendor-prefixed rules do not use this helper.
pub(crate) fn is_path_shaped_document_token(
    candidate: &str,
    line: &str,
    match_start: usize,
    match_end: usize,
) -> bool {
    if is_inside_http_url_path(line, match_start) {
        return true;
    }

    let has_sep = candidate.contains('/') || candidate.contains('\\');

    // Characters after the regex capture often hold the document extension
    // (the capture class excludes `.`). A separator is not required: a bare
    // `name.md` is still a document path (CIB-348).
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

    if !has_sep {
        return false;
    }

    // Git Bash / MSYS / Cygwin: `/c/Users/…` is `C:\Users\…`. The first
    // path component is a single ASCII letter. `/usr/…` is not a drive.
    // Separator-alone is still not enough (Copilot on #3724).
    if is_posix_drive_prefixed(candidate) {
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

    // Do not exempt on separator count alone: opaque base64-ish tokens can
    // contain multiple `/` characters. Extension / drive-letter evidence above
    // is required (Copilot review on #3724).
    false
}

/// True when `candidate` starts with `/X/` or `\X\` for an ASCII letter `X`.
/// That is the Git Bash / MSYS / Cygwin spelling of a Windows drive.
fn is_posix_drive_prefixed(candidate: &str) -> bool {
    let Some(rest) = candidate.strip_prefix(['/', '\\']) else {
        return false;
    };
    let mut chars = rest.chars();
    let Some(letter) = chars.next() else {
        return false;
    };
    letter.is_ascii_alphabetic() && matches!(chars.next(), Some('/' | '\\'))
}

/// True when `match_start` sits in the path of an `http://` or `https://`
/// URL on this line. Query and fragment matches are left to the card rule
/// — a PAN in `?card=` is still a finding.
/// Only a syntactically valid, host-bearing authority earns the exemption
/// (#3917): `https:///accounts/…` has no host. The URL token ends at
/// whitespace, query/fragment, or a quote / markup / list delimiter so a
/// same-line standalone PAN is not swallowed.
fn is_inside_http_url_path(line: &str, match_start: usize) -> bool {
    let Some(prefix) = line.get(..match_start) else {
        return false;
    };

    let scheme_at = prefix
        .rmatch_indices("https://")
        .map(|(i, _)| i)
        .chain(prefix.rmatch_indices("http://").map(|(i, _)| i))
        .max();
    let Some(scheme_at) = scheme_at else {
        return false;
    };

    let after_scheme = if prefix[scheme_at..].starts_with("https://") {
        scheme_at + "https://".len()
    } else {
        scheme_at + "http://".len()
    };
    if after_scheme > prefix.len() {
        return false;
    }

    let host_and_maybe_path = &prefix[after_scheme..];
    let Some((authority, path)) = split_http_authority_and_path(host_and_maybe_path) else {
        return false;
    };
    if !http_authority_has_valid_host(authority) {
        return false;
    }
    if path.chars().any(url_path_ended) {
        return false;
    }
    true
}

/// Split `host[:port]/path` or `[ipv6][:port]/path` after `http(s)://`.
fn split_http_authority_and_path(after_scheme: &str) -> Option<(&str, &str)> {
    if after_scheme.starts_with('[') {
        let close = after_scheme.find(']')?;
        let after_bracket = &after_scheme[close + 1..];
        let path_rel = after_bracket.find('/')?;
        return Some((
            &after_scheme[..close + 1 + path_rel],
            &after_scheme[close + 1 + path_rel..],
        ));
    }
    let slash = after_scheme.find('/')?;
    Some((&after_scheme[..slash], &after_scheme[slash..]))
}

/// HTTP(S) special-scheme host: non-empty reg-name / IPv4, or bracketed IPv6.
/// Empty authority (`https:///…`), userinfo with no host, port-only, and
/// empty/unclosed IPv6 brackets do not count.
fn http_authority_has_valid_host(authority: &str) -> bool {
    let Some(hostport) = authority_hostport(authority) else {
        return false;
    };
    if hostport.starts_with('[') {
        return bracketed_ipv6_authority_valid(hostport);
    }
    if hostport.chars().any(url_path_ended) {
        return false;
    }
    let Some(host) = strip_http_port(hostport) else {
        return false;
    };
    is_http_reg_name(host)
}

fn authority_hostport(authority: &str) -> Option<&str> {
    if let Some(at) = last_at_outside_brackets(authority) {
        let hostport = &authority[at + 1..];
        if hostport.is_empty() {
            return None;
        }
        return Some(hostport);
    }
    Some(authority)
}

fn last_at_outside_brackets(value: &str) -> Option<usize> {
    let mut depth = 0_i32;
    let mut last = None;
    for (index, character) in value.char_indices() {
        match character {
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            '@' if depth == 0 => last = Some(index),
            _ => {}
        }
    }
    last
}

fn bracketed_ipv6_authority_valid(hostport: &str) -> bool {
    let Some(rest) = hostport.strip_prefix('[') else {
        return false;
    };
    let Some((addr, after)) = rest.split_once(']') else {
        return false;
    };
    if addr.is_empty()
        || !addr
            .chars()
            .all(|c| c.is_ascii_hexdigit() || matches!(c, ':' | '.' | '%'))
    {
        return false;
    }
    if after.is_empty() {
        return true;
    }
    after.strip_prefix(':').is_some_and(is_http_port)
}

fn strip_http_port(hostport: &str) -> Option<&str> {
    match hostport.rsplit_once(':') {
        Some((host, port)) if is_http_port(port) => {
            if host.is_empty() {
                None
            } else {
                Some(host)
            }
        }
        Some((_, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => None,
        _ => Some(hostport),
    }
}

fn is_http_port(port: &str) -> bool {
    if port.is_empty() || port.len() > 5 {
        return false;
    }
    port.parse::<u32>().is_ok_and(|n| n <= 65_535)
}

fn is_http_reg_name(host: &str) -> bool {
    if host.is_empty() {
        return false;
    }
    let mut saw_alnum = false;
    for character in host.chars() {
        if character.is_ascii_alphanumeric() {
            saw_alnum = true;
        } else if !matches!(character, '.' | '-' | '_') {
            return false;
        }
    }
    saw_alnum
}

fn url_path_ended(c: char) -> bool {
    c.is_ascii_whitespace()
        || matches!(
            c,
            '?' | '#'
                | '"'
                | '\''
                | '`'
                | '<'
                | '>'
                | ','
                | '{'
                | '}'
                | '['
                | ']'
                | '('
                | ')'
                | ';'
        )
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

    fn high_entropy_hyphenated_stem() -> String {
        // Assembled so the fixture source is not itself one entropy token.
        [
            "abcd",
            "efgh",
            "ijkl",
            "mnop",
            "qrst",
            "uvwx",
            "yz",
            "-",
            "quarterly",
            "-",
            "review",
        ]
        .concat()
    }

    #[test]
    fn does_not_flag_hyphenated_document_filenames() {
        // CIB-348: SEC-FP-1 required a `/` or `\`. A bare hyphenated
        // dictionary-word `.md` name is still a document path.
        let config = SecretCheckConfig::default();
        let filename = "quarterly-revenue-forecast-summary.md";
        let stem = "quarterly-revenue-forecast-summary";

        assert!(
            path_shaped(filename, stem),
            "bare document filename must be path-shaped: {filename}"
        );
        let assigned = format!("file = {filename}");
        assert!(
            path_shaped(&assigned, stem),
            "file = document filename must be path-shaped: {assigned}"
        );

        assert!(
            detect_high_entropy_strings(filename, "notes.md", &config).is_empty(),
            "bare hyphenated .md filename must not trip entropy"
        );
        assert!(
            detect_high_entropy_strings(&assigned, "notes.md", &config).is_empty(),
            "file = hyphenated .md filename must not trip entropy"
        );

        // Same exemption must hold when the stem itself is above threshold.
        let high_stem = high_entropy_hyphenated_stem();
        let high = format!("{high_stem}.md");
        assert!(
            path_shaped(&high, &high_stem),
            "high-entropy document filename must be path-shaped: {high}"
        );
        let assigned_high = format!("file = {high}");
        assert!(
            detect_high_entropy_strings(&assigned_high, "notes.md", &config).is_empty(),
            "file = high-entropy hyphenated .md filename must not trip entropy"
        );
    }

    #[test]
    fn hyphen_count_alone_does_not_exempt_opaque_tokens() {
        // CIB-348 non-scope: do not exempt on hyphen count alone.
        let config = SecretCheckConfig::default();
        let token = high_entropy_hyphenated_stem();
        let content = format!("const apiToken = '{token}';\n");
        let findings = detect_high_entropy_strings(&content, "src/auth.ts", &config);
        assert!(
            findings
                .iter()
                .any(|f| f.pattern_name == "High Entropy String"),
            "hyphenated opaque token without a document extension must still flag: {findings:?}"
        );
    }

    fn git_bash_temp_path() -> String {
        // Assembled so the fixture source is not itself one entropy token.
        [
            "/", "c", "/", "Users", "/", "dave", "/", "AppData", "/", "Local", "/", "Temp", "/",
            "anvil-", "abc123", "def456", "7890ab", "cdef",
        ]
        .concat()
    }

    #[test]
    fn git_bash_drive_prefix_is_path_shaped() {
        // Dave B17 / CIB-339: SEC-FP-1 keys off `C:`. Git Bash writes `/c/`.
        let path = git_bash_temp_path();
        let line = format!("path = \"{path}\"");
        assert!(
            path_shaped(&line, &path),
            "Git Bash /c/Users path must be path-shaped: {line}"
        );
        let d_drive = [
            "/d",
            "/",
            "Projects",
            "/",
            "workspace",
            "/",
            "long-token-name-here",
        ]
        .concat();
        assert!(
            path_shaped(&format!("see {d_drive}"), &d_drive),
            "/d/ drive prefix must also be path-shaped"
        );
        let unix = [
            "/", "usr", "/", "local", "/", "share", "/", "anvil-", "abc123", "def456", "7890ab",
            "cdef",
        ]
        .concat();
        assert!(
            !path_shaped(&format!("path = \"{unix}\""), &unix),
            "Unix path with a multi-letter first component is not a Git Bash drive"
        );
    }

    #[test]
    fn does_not_flag_git_bash_drive_prefixed_paths() {
        let config = SecretCheckConfig {
            entropy_threshold: 3.0,
            ..SecretCheckConfig::default()
        };
        let content = format!("path = \"{}\"\n", git_bash_temp_path());
        let findings = detect_high_entropy_strings(&content, "notes.md", &config);
        assert!(
            findings.is_empty(),
            "Git Bash drive-prefixed path must not trip entropy, got: {:?}",
            findings
                .iter()
                .map(|f| &f.redacted_match)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn mixed_case_opaque_token_still_flags_on_entropy() {
        // Dave B16 / CIB-340: camelCase looks_like_code dropped mixed-case
        // secrets. Entropy must not treat that shape as code.
        let config = SecretCheckConfig {
            entropy_threshold: 3.5,
            ..SecretCheckConfig::default()
        };
        let token = ["aB", "3d", "E7", "fG", "9h", "J2", "kL", "4m"].concat();
        let content = format!("const apiToken = '{token}';\n");
        let findings = detect_high_entropy_strings(&content, "src/auth.ts", &config);
        assert!(
            findings
                .iter()
                .any(|f| f.pattern_name == "High Entropy String"),
            "mixed-case opaque token must still flag: {findings:?}"
        );
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

    fn visa_test_pan() -> String {
        ["4111", "1111", "1111", "1111"].concat()
    }

    fn path_shaped(line: &str, digits: &str) -> bool {
        let start = line.find(digits).expect("digits in fixture");
        super::is_path_shaped_document_token(digits, line, start, start + digits.len())
    }

    #[test]
    fn empty_or_malformed_http_host_is_not_path_shaped() {
        let digits = visa_test_pan();
        let cases = [
            format!("https:///accounts/{digits}/events"),
            format!("http:///accounts/{digits}/events"),
            format!("https:////accounts/{digits}"),
            format!("https:/accounts/{digits}"),
            format!("https://user@/accounts/{digits}"),
            format!("https://[]/accounts/{digits}"),
            format!("https://[::1/accounts/{digits}"),
            format!("https://:443/accounts/{digits}"),
        ];
        for line in cases {
            assert!(
                !path_shaped(&line, &digits),
                "malformed host must not earn the URL-path exemption: {line}"
            );
        }
    }

    #[test]
    fn valid_http_hosts_remain_path_shaped() {
        let digits = visa_test_pan();
        let cases = [
            format!("https://www.facebook.com/reel/{digits}"),
            format!("http://www.facebook.com/reel/{digits}"),
            format!("https://pay.example.com:443/accounts/{digits}"),
            format!("https://127.0.0.1/accounts/{digits}"),
            format!("https://[::1]/accounts/{digits}"),
            format!("https://[::1]:8443/accounts/{digits}"),
            format!("https://user:pass@pay.example.com/accounts/{digits}"),
            format!("https://localhost/accounts/{digits}"),
        ];
        for line in cases {
            assert!(
                path_shaped(&line, &digits),
                "valid host-bearing URL path must stay path-shaped: {line}"
            );
        }
    }
}
