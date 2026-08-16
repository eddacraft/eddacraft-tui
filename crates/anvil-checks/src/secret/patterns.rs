use std::sync::LazyLock;

use regex::Regex;

use crate::secret::types::{AllowlistProvenance, SecretPatternDef};

pub struct SecretPattern {
    pub name: &'static str,
    pub pattern: &'static str,
    /// `true` for structurally-unambiguous shapes (`AKIA…`, `ghp_…`,
    /// `sk_live_…`, etc.) whose match is itself the credential.
    ///
    /// High-confidence matches bypass the `looks_like_code` filter and the
    /// keyword allowlist (`example`, `test`, `dummy`, …) — both of which
    /// silently suppressed textbook AWS access keys before this flag was
    /// introduced (issue #1800). They still honour shape-anchored
    /// allowlist entries (hex hashes, `0x…` addresses, `data:image/…;base64`)
    /// and any user-supplied `custom_allowlist` entries so opt-outs remain
    /// possible.
    ///
    /// Leave `false` for keyword-driven patterns (`API Key`, `Generic
    /// Secret`) and patterns with their own structural false-positive
    /// guard (`Credit Card`).
    pub high_confidence: bool,
}

pub const SECRET_PATTERNS: [SecretPattern; 21] = [
    SecretPattern {
        name: "API Key",
        pattern: r#"(?i)(?:api[_-]?key|apikey)\s*[:=]\s*['\"]?[a-zA-Z0-9_-]{16,}['\"]?"#,
        high_confidence: false,
    },
    SecretPattern {
        name: "JWT Token",
        pattern: r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*",
        high_confidence: true,
    },
    SecretPattern {
        name: "AWS Key",
        pattern: r"AKIA[0-9A-Z]{16}",
        high_confidence: true,
    },
    SecretPattern {
        // STS temporary access keys (`ASIA…`) share the AKIA shape but
        // are issued by AWS Security Token Service; treated identically
        // for detection purposes.
        name: "AWS STS Key",
        pattern: r"ASIA[0-9A-Z]{16}",
        high_confidence: true,
    },
    SecretPattern {
        name: "AWS Secret Key",
        pattern: r#"(?i)(?:aws_secret|aws_secret_access_key)\s*[:=]\s*['\"]?[A-Za-z0-9/+=]{40}['\"]?"#,
        high_confidence: true,
    },
    SecretPattern {
        name: "Private Key",
        pattern: r"-----BEGIN\s+(?:RSA\s+)?PRIVATE\s+KEY-----",
        high_confidence: true,
    },
    SecretPattern {
        name: "PGP Private Key",
        pattern: r"-----BEGIN PGP PRIVATE KEY BLOCK-----",
        high_confidence: true,
    },
    SecretPattern {
        name: "Database URL",
        pattern: r"(?:postgres|mysql|mongodb|redis):\/\/[^:\s]+:[^@\s]+@",
        high_confidence: true,
    },
    SecretPattern {
        name: "Generic Secret",
        pattern: r#"(?i)(?:secret|password|passwd|pwd)\s*[:=]\s*['\"]?[^\s'\"]{8,}['\"]?"#,
        high_confidence: false,
    },
    SecretPattern {
        name: "Credit Card",
        pattern: r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b",
        high_confidence: false,
    },
    SecretPattern {
        name: "GitHub Token",
        pattern: r"gh[pousr]_[A-Za-z0-9_]{36,}",
        high_confidence: true,
    },
    SecretPattern {
        name: "Slack Token",
        pattern: r"xox[baprs]-[0-9]{10,13}-[0-9]{10,13}-[a-zA-Z0-9]{24}",
        high_confidence: true,
    },
    SecretPattern {
        name: "Stripe Key",
        pattern: r"sk_live_[0-9a-zA-Z]{24}",
        high_confidence: true,
    },
    SecretPattern {
        name: "Stripe Test Key",
        pattern: r"sk_test_[0-9a-zA-Z]{24}",
        high_confidence: true,
    },
    SecretPattern {
        name: "Google API Key",
        pattern: r"AIza[0-9A-Za-z_-]{35}",
        high_confidence: true,
    },
    SecretPattern {
        name: "Heroku API Key",
        pattern: r#"[hH]eroku[a-zA-Z0-9_-]*[:=]\s*['\"]?[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}['\"]?"#,
        high_confidence: true,
    },
    SecretPattern {
        name: "SendGrid API Key",
        pattern: r"SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43}",
        high_confidence: true,
    },
    SecretPattern {
        name: "Twilio API Key",
        pattern: r"SK[a-f0-9]{32}",
        high_confidence: true,
    },
    SecretPattern {
        name: "NPM Token",
        pattern: r"npm_[A-Za-z0-9]{36}",
        high_confidence: true,
    },
    SecretPattern {
        // Anthropic keys are prefixed `sk-ant-` followed by an opaque
        // alphanumeric / `-_` body. Listed before the OpenAI pattern so
        // a textbook Anthropic key reports under the correct provider.
        name: "Anthropic API Key",
        pattern: r"sk-ant-[A-Za-z0-9_-]{32,}",
        high_confidence: true,
    },
    SecretPattern {
        // Modern OpenAI keys carry one of the documented account-scope
        // prefixes (`proj`, `svcacct`, `admin`); the older bare `sk-…`
        // shape is left to the entropy heuristic so we don't false-positive
        // every CSS class string that begins `sk-`.
        name: "OpenAI API Key",
        pattern: r"sk-(?:proj|svcacct|admin)-[A-Za-z0-9_-]{20,}",
        high_confidence: true,
    },
];

/// Shape-anchored allowlist entries: the matched value must structurally
/// look like the listed shape end-to-end (`^…$`) or carry a recognisable
/// header (`data:image/…;base64,`). Applies to *all* patterns including
/// high-confidence ones, because a 40-char hex string really is just an
/// SHA-1 hash regardless of where it appears.
pub const DEFAULT_SHAPE_ALLOWLIST: [&str; 6] = [
    r"^[a-f0-9]{32}$",
    r"^[a-f0-9]{40}$",
    r"^[a-f0-9]{64}$",
    r"^0x[a-f0-9]+$",
    r"^data:image\/[a-z]+;base64,",
    // ULID: 26-char Crockford base32 (excludes I, L, O, U). A maximally
    // diverse ULID reaches entropy ≈ 4.70 and trips the detector; it is a
    // public record identifier, never a secret. UUIDs need no entry — their
    // 16-symbol hex alphabet caps entropy at 4.0, below the 4.5 threshold.
    //
    // Anchored to the ULID-spec timestamp constraint — the first character
    // encodes the high bits of the 48-bit millisecond timestamp and is `0`–`7`
    // for any ULID minted before the year ~10889 — so an arbitrary 26-char
    // Crockford run starting `8`–`Z` is *not* allowlisted, keeping the
    // suppression as narrow as possible.
    r"^[0-7][0-9A-HJKMNP-TV-Z]{25}$",
];

/// Keyword allowlist: suppresses fuzzy detections (entropy + low-confidence
/// patterns) when the matched value mentions a documentation/test marker.
/// **Does not** apply to high-confidence shape-specific patterns — the
/// canonical AWS textbook key `AKIAIOSFODNN7EXAMPLE` is still a real-shape
/// access key and must surface despite the `EXAMPLE` suffix (issue #1800).
pub const DEFAULT_KEYWORD_ALLOWLIST: [&str; 6] = [
    r"placeholder",
    r"example",
    r"test",
    r"dummy",
    r"sample",
    r"lorem ipsum",
];

/// Combined default allowlist — preserved for back-compat with the
/// pre-#1800 single-list contract. New code should prefer the
/// shape / keyword split above.
pub const DEFAULT_ALLOWLIST: [&str; 12] = [
    r"^[a-f0-9]{32}$",
    r"^[a-f0-9]{40}$",
    r"^[a-f0-9]{64}$",
    r"^0x[a-f0-9]+$",
    r"^data:image\/[a-z]+;base64,",
    r"^[0-7][0-9A-HJKMNP-TV-Z]{25}$",
    r"placeholder",
    r"example",
    r"test",
    r"dummy",
    r"sample",
    r"lorem ipsum",
];

pub struct CompiledPattern {
    pub name: String,
    pub regex: Regex,
    /// Mirrors [`SecretPattern::high_confidence`]. User-supplied custom
    /// patterns default to `false` — the scanner cannot know whether a
    /// hand-written regex is structurally unambiguous, so it keeps the
    /// safer fuzzy filters in place.
    pub high_confidence: bool,
}

static QUOTED_REDACTION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(['"])([^'"\r\n]{8,})(['"])"#).expect("quoted redaction pattern is valid")
});
static ASSIGNMENT_REDACTION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"([:=]\s*)(['"]?)([^\s'"]{8,})(['"]?)"#)
        .expect("assignment redaction pattern is valid")
});
static BARE_REDACTION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z0-9_./+=:@%-]{16,}").expect("bare redaction pattern is valid")
});

/// Built-in secret patterns compiled once per process. Sharing the compiled
/// regex set across parallel scan workers avoids recompiling 18 patterns per
/// file, which was a measurable waste of CPU in the discovery scan.
///
/// Built-in patterns are repo-owned constants — if one fails to compile, that
/// is a developer bug, not a runtime condition to recover from. We panic on
/// access (caught by every test that exercises the secret scanner) so a bad
/// regex is a loud CI failure instead of a silent reduction in detection
/// coverage.
pub static DEFAULT_COMPILED_PATTERNS: LazyLock<Vec<CompiledPattern>> = LazyLock::new(|| {
    SECRET_PATTERNS
        .iter()
        .map(|pattern| {
            let regex = Regex::new(pattern.pattern).unwrap_or_else(|err| {
                panic!(
                    "built-in secret pattern `{}` failed to compile: {err}",
                    pattern.name
                )
            });
            CompiledPattern {
                name: pattern.name.to_string(),
                regex,
                high_confidence: pattern.high_confidence,
            }
        })
        .collect()
});

/// Compile the combined built-in + user-supplied secret patterns.
///
/// Returns `(compiled, errors)` — callers must surface `errors` so a
/// misconfigured custom pattern doesn't silently produce zero matches.
/// Prefer iterating `DEFAULT_COMPILED_PATTERNS` chained with
/// `compile_custom_patterns` on hot paths rather than rebuilding the full
/// `Vec` here.
pub fn compile_secret_patterns(
    custom_patterns: &[SecretPatternDef],
) -> (Vec<CompiledPattern>, Vec<String>) {
    // Built-ins go through the fail-fast path; custom patterns tolerate
    // compile failures because they come from user config.
    let mut compiled: Vec<CompiledPattern> = DEFAULT_COMPILED_PATTERNS
        .iter()
        .map(|p| CompiledPattern {
            name: p.name.clone(),
            regex: p.regex.clone(),
            high_confidence: p.high_confidence,
        })
        .collect();
    let (custom, errors) = compile_custom_patterns(custom_patterns);
    compiled.extend(custom);
    (compiled, errors)
}

/// Compile only the user-supplied custom patterns.
///
/// Returns the patterns that compiled successfully and a list of errors for
/// patterns that did not. Callers must surface the errors to the user — a
/// silently dropped pattern is a misconfiguration the scanner cannot detect on
/// its own. Pair the result with `DEFAULT_COMPILED_PATTERNS` when scanning;
/// callers on the hot path should chain the two iterators rather than calling
/// `compile_secret_patterns`, which rebuilds the combined `Vec` and clones
/// the pre-compiled built-ins on every call.
pub fn compile_custom_patterns(
    custom_patterns: &[SecretPatternDef],
) -> (Vec<CompiledPattern>, Vec<String>) {
    let mut compiled = Vec::with_capacity(custom_patterns.len());
    let mut errors = Vec::new();
    for pattern in custom_patterns {
        match Regex::new(&pattern.pattern) {
            Ok(regex) => compiled.push(CompiledPattern {
                name: pattern.name.clone(),
                regex,
                // User-supplied patterns are treated as fuzzy by default —
                // the scanner cannot know whether a hand-written regex is
                // structurally unambiguous, so the full FP filter stack
                // (keyword allowlist, `looks_like_code`) keeps running.
                high_confidence: false,
            }),
            Err(err) => errors.push(format!(
                "custom secret pattern '{}' failed to compile: {err}",
                pattern.name
            )),
        }
    }
    (compiled, errors)
}

#[allow(clippy::struct_field_names)] // all three are distinct allowlist tiers
pub struct PatternMatcher {
    default_shape_allowlist: Vec<Regex>,
    default_keyword_allowlist: Vec<Regex>,
    /// Source pattern paired with its compiled regex so a suppression can be
    /// traced back to the exact operator-configured opt-out
    /// ([`AllowlistProvenance::Custom`]).
    custom_allowlist: Vec<(String, Regex)>,
}

impl PatternMatcher {
    pub fn new(custom_allowlist: &[String]) -> Self {
        let default_shape_allowlist = DEFAULT_SHAPE_ALLOWLIST
            .iter()
            .filter_map(|pattern| Regex::new(&format!("(?i){pattern}")).ok())
            .collect();
        let default_keyword_allowlist = DEFAULT_KEYWORD_ALLOWLIST
            .iter()
            .filter_map(|pattern| Regex::new(&format!("(?i){pattern}")).ok())
            .collect();
        let custom_allowlist = custom_allowlist
            .iter()
            .filter_map(|pattern| {
                Regex::new(&format!("(?i){pattern}"))
                    .ok()
                    .map(|re| (pattern.clone(), re))
            })
            .collect();

        Self {
            default_shape_allowlist,
            default_keyword_allowlist,
            custom_allowlist,
        }
    }

    /// Full allowlist check — used for entropy findings and low-confidence
    /// patterns where fuzzy keyword suppression is desirable.
    pub fn is_allowlisted(&self, value: &str) -> bool {
        self.matched_allowlist(value).is_some()
    }

    /// Like [`PatternMatcher::is_allowlisted`] but returns *which* allowlist
    /// tier matched, so the caller can record the suppression's provenance.
    /// Tiers are checked shape → keyword → custom; the first match wins.
    #[must_use]
    pub fn matched_allowlist(&self, value: &str) -> Option<AllowlistProvenance> {
        if self
            .default_shape_allowlist
            .iter()
            .any(|pattern| pattern.is_match(value))
        {
            return Some(AllowlistProvenance::BuiltinShape);
        }
        if self
            .default_keyword_allowlist
            .iter()
            .any(|pattern| pattern.is_match(value))
        {
            return Some(AllowlistProvenance::BuiltinKeyword);
        }
        self.matched_custom(value)
    }

    fn matched_custom(&self, value: &str) -> Option<AllowlistProvenance> {
        self.custom_allowlist
            .iter()
            .find(|(_, re)| re.is_match(value))
            .map(|(pattern, _)| AllowlistProvenance::Custom {
                pattern: pattern.clone(),
            })
    }

    /// Allowlist check for high-confidence shape-specific patterns
    /// (`AKIA…`, `ghp_…`, `sk_live_…`, …). Bypasses the keyword filter
    /// — `EXAMPLE` in the AWS textbook key `AKIAIOSFODNN7EXAMPLE` is not
    /// grounds for suppression because the prefix-anchored shape is the
    /// credential itself (issue #1800). Still honours shape-anchored
    /// defaults (hex hashes, `0x…`, data URIs) and any user-supplied
    /// `custom_allowlist` entries so legitimate opt-outs keep working.
    pub fn is_shape_or_custom_allowlisted(&self, value: &str) -> bool {
        self.matched_shape_or_custom(value).is_some()
    }

    /// Provenance-returning form of [`PatternMatcher::is_shape_or_custom_allowlisted`].
    #[must_use]
    pub fn matched_shape_or_custom(&self, value: &str) -> Option<AllowlistProvenance> {
        if self
            .default_shape_allowlist
            .iter()
            .any(|pattern| pattern.is_match(value))
        {
            return Some(AllowlistProvenance::BuiltinShape);
        }
        self.matched_custom(value)
    }

    pub fn looks_like_code(&self, value: &str) -> bool {
        self.looks_like_structural_code(value) || looks_like_mixed_case_identifier(value)
    }

    /// Calls, URLs, file extensions, and SCREAMING_SNAKE constants.
    /// Entropy uses this set and not mixed-case identifiers (CIB-340).
    pub fn looks_like_structural_code(&self, value: &str) -> bool {
        static STRUCTURAL: LazyLock<Vec<Regex>> = LazyLock::new(|| {
            [
                r"^[a-z][a-zA-Z0-9]*\(",
                r"^[a-z][a-zA-Z0-9]*\.[a-z]",
                r"^https?:\/\/",
                r"^[a-z]+:\/\/",
                r"\.(js|ts|css|html|json|md|txt)$",
                r"^[A-Z][A-Z0-9_]+$",
            ]
            .iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect()
        });

        STRUCTURAL.iter().any(|pattern| pattern.is_match(value))
    }

    pub fn redact_secret(&self, value: &str) -> String {
        let char_count = value.chars().count();
        if char_count <= 8 {
            "***".to_string()
        } else {
            let prefix: String = value.chars().take(4).collect();
            let suffix: String = value.chars().skip(char_count - 4).collect();
            format!("{prefix}...{suffix}")
        }
    }

    pub fn redact_line(&self, line: &str) -> String {
        let quoted_redacted =
            QUOTED_REDACTION_PATTERN.replace_all(line, |captures: &regex::Captures<'_>| {
                let opening = captures.get(1).map_or("\"", |value| value.as_str());
                let closing = captures.get(3).map_or(opening, |value| value.as_str());
                format!("{opening}[REDACTED]{closing}")
            });
        let assignment_redacted = ASSIGNMENT_REDACTION_PATTERN.replace_all(
            quoted_redacted.as_ref(),
            |captures: &regex::Captures<'_>| {
                let prefix = captures.get(1).map_or("", |value| value.as_str());
                let opening = captures.get(2).map_or("", |value| value.as_str());
                let closing = captures.get(4).map_or("", |value| value.as_str());
                format!("{prefix}{opening}[REDACTED]{closing}")
            },
        );

        BARE_REDACTION_PATTERN
            .replace_all(assignment_redacted.as_ref(), "[REDACTED]")
            .into_owned()
    }

    pub fn redact_range_in_line(&self, line: &str, start: usize, end: usize) -> String {
        let (Some(prefix), Some(segment), Some(suffix)) =
            (line.get(..start), line.get(start..end), line.get(end..))
        else {
            return self.redact_line(line);
        };

        format!("{prefix}{}{suffix}", self.redact_line(segment))
    }
}

fn looks_like_mixed_case_identifier(value: &str) -> bool {
    static MIXED_CASE: LazyLock<Vec<Regex>> = LazyLock::new(|| {
        [r"^[a-z][a-z0-9]*[A-Z]", r"^[A-Z][a-z]+[A-Z]"]
            .iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect()
    });

    MIXED_CASE.iter().any(|pattern| pattern.is_match(value))
}

#[cfg(test)]
mod tests {
    use crate::secret::types::SecretPatternDef;

    use super::{
        DEFAULT_ALLOWLIST, DEFAULT_COMPILED_PATTERNS, DEFAULT_KEYWORD_ALLOWLIST,
        DEFAULT_SHAPE_ALLOWLIST, PatternMatcher, SECRET_PATTERNS, compile_custom_patterns,
        compile_secret_patterns,
    };

    #[test]
    fn every_builtin_pattern_compiles() {
        // Force LazyLock initialisation. Any invalid built-in pattern panics
        // with the offending pattern name; running this in CI guarantees a
        // bad regex is caught loudly, not silently dropped from the scan set.
        assert_eq!(DEFAULT_COMPILED_PATTERNS.len(), SECRET_PATTERNS.len());
    }

    #[test]
    fn matches_all_default_patterns_with_examples() {
        // Build secret-shaped test values at runtime so the literal strings
        // never appear in source — avoids GitHub push-protection false positives.
        let stripe_live = format!("sk_live_{}", "1234567890abcdefghijABCD");
        let stripe_test = format!("sk_test_{}", "1234567890abcdefghijABCD");
        let slack_token = format!(
            "xoxb-{}-{}-{}",
            "1234567890", "1234567890", "a1b2c3d4e5f6g7h8i9j0k1l2"
        );
        let twilio_key = format!("SK{}", "abcdef0123456789abcdef0123456789");
        let anthropic_key = format!("sk-ant-{}", "abcdefghijklmnopqrstuvwxyz012345");
        let openai_key = format!("sk-proj-{}", "abcdefghijklmnopqrst");

        let examples: Vec<(&str, String)> = vec![
            ("API Key", "api_key='abcdEFGH1234567890'".into()),
            (
                "JWT Token",
                "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.sig".into(),
            ),
            ("AWS Key", "AKIAABCDEFGHIJKLMNOP".into()),
            ("AWS STS Key", "ASIAABCDEFGHIJKLMNOP".into()),
            (
                "AWS Secret Key",
                "aws_secret_access_key='abcdabcdabcdabcdabcdabcdabcdabcdabcdabcd'".into(),
            ),
            ("Private Key", "-----BEGIN RSA PRIVATE KEY-----".into()),
            (
                "PGP Private Key",
                "-----BEGIN PGP PRIVATE KEY BLOCK-----".into(),
            ),
            (
                "Database URL",
                "postgres://user:pass@localhost:5432/db".into(),
            ),
            ("Generic Secret", "password='hunter22'".into()),
            ("Credit Card", "4242 4242 4242 4242".into()),
            (
                "GitHub Token",
                "ghp_abcdefghijklmnopqrstuvwxyz1234567890abc".into(),
            ),
            ("Slack Token", slack_token),
            ("Stripe Key", stripe_live),
            ("Stripe Test Key", stripe_test),
            (
                "Google API Key",
                "AIzaSyA12345678901234567890123456789012".into(),
            ),
            (
                "Heroku API Key",
                "HerokuKey=123e4567-e89b-12d3-a456-426614174000".into(),
            ),
            (
                "SendGrid API Key",
                "SG.1234567890123456789012.1234567890123456789012345678901234567890123".into(),
            ),
            ("Twilio API Key", twilio_key),
            (
                "NPM Token",
                "npm_abcdefghijklmnopqrstuvwxyz1234567890".into(),
            ),
            ("Anthropic API Key", anthropic_key),
            ("OpenAI API Key", openai_key),
        ];

        let (compiled, errors) = compile_secret_patterns(&[]);
        assert!(
            errors.is_empty(),
            "no custom patterns should yield no errors"
        );

        for (name, sample) in &examples {
            let maybe_pattern = compiled.iter().find(|pattern| pattern.name == *name);
            if let Some(pattern) = maybe_pattern {
                assert!(pattern.regex.is_match(sample), "{name} should match");
            } else {
                panic!("pattern not found: {name}");
            }
        }

        assert_eq!(SECRET_PATTERNS.len(), 21);
    }

    #[test]
    fn generic_secret_matches_compound_names() {
        let (compiled, _errors) = compile_secret_patterns(&[]);
        let pattern = compiled
            .iter()
            .find(|p| p.name == "Generic Secret")
            .expect("Generic Secret pattern");
        // Construct test inputs at runtime to avoid tripping secret-scanning
        // push protection (consistent with other tests in this file).
        let db_password = format!("DB_PASSWORD={}{}", "super", "secret99");
        let my_secret = format!("my_secret = {}{}", "long", "value123");
        let admin_pwd = format!("ADMIN_PWD='{}{}{}'", "f3k8", "q9m2", "x7");
        assert!(pattern.regex.is_match(&db_password));
        assert!(pattern.regex.is_match(&my_secret));
        assert!(pattern.regex.is_match(&admin_pwd));
    }

    #[test]
    fn compile_custom_patterns_separates_invalid_from_valid() {
        let inputs = vec![
            SecretPatternDef {
                name: "ok".to_string(),
                pattern: r"valid_[A-Z]+".to_string(),
            },
            SecretPatternDef {
                name: "broken".to_string(),
                pattern: "(unclosed".to_string(),
            },
            SecretPatternDef {
                name: "lookahead-not-supported".to_string(),
                pattern: r"foo(?=bar)".to_string(),
            },
        ];

        let (compiled, errors) = compile_custom_patterns(&inputs);

        assert_eq!(compiled.len(), 1, "only the valid pattern compiles");
        assert_eq!(compiled[0].name, "ok");
        assert_eq!(errors.len(), 2, "both invalid patterns reported");
        assert!(
            errors[0].contains("'broken'"),
            "error names the offending pattern, got: {}",
            errors[0]
        );
        assert!(
            errors[1].contains("'lookahead-not-supported'"),
            "error names the offending pattern, got: {}",
            errors[1]
        );
    }

    #[test]
    fn allows_default_and_custom_allowlist_patterns() {
        let matcher = PatternMatcher::new(&["my-safe-value".to_string()]);

        assert!(matcher.is_allowlisted("placeholder"));
        assert!(matcher.is_allowlisted("my-safe-value"));
        assert!(!matcher.is_allowlisted("ghp_abcdefghijklmnopqrstuvwxyz1234567890abc"));
        assert_eq!(DEFAULT_ALLOWLIST.len(), 12);

        // Pin the back-compat contract structurally, not just by length:
        // `DEFAULT_ALLOWLIST` MUST be the in-order concatenation of the
        // shape and keyword splits. A length-only assertion lets a
        // future edit swap entries without anyone noticing, which would
        // silently flip the meaning of values exported from this module.
        let expected: Vec<&str> = DEFAULT_SHAPE_ALLOWLIST
            .iter()
            .chain(DEFAULT_KEYWORD_ALLOWLIST.iter())
            .copied()
            .collect();
        let actual: Vec<&str> = DEFAULT_ALLOWLIST.to_vec();
        assert_eq!(
            actual, expected,
            "DEFAULT_ALLOWLIST must be the in-order concat of the \
             shape + keyword splits"
        );
    }

    #[test]
    fn shape_or_custom_allowlist_excludes_keyword_filter() {
        // Issue #1800: a textbook AWS access key (`AKIAIOSFODNN7EXAMPLE`)
        // is structurally a real AWS key — the `EXAMPLE` suffix is part of
        // the matched value but must NOT suppress the finding under the
        // high-confidence path.
        let matcher = PatternMatcher::new(&[]);
        let aws_textbook_key = "AKIAIOSFODNN7EXAMPLE";

        // The legacy `is_allowlisted` still returns true (it sees the
        // substring "EXAMPLE") — that path is now reserved for fuzzy
        // entropy + low-confidence patterns.
        assert!(matcher.is_allowlisted(aws_textbook_key));
        // The high-confidence path must NOT allowlist it.
        assert!(!matcher.is_shape_or_custom_allowlisted(aws_textbook_key));
    }

    #[test]
    fn shape_allowlist_still_applies_to_high_confidence_path() {
        // A 40-char hex string is an SHA-1 hash, not a secret — even
        // for a hypothetical high-confidence pattern that matched it,
        // the shape-anchored allowlist must still suppress it.
        let matcher = PatternMatcher::new(&[]);
        let sha1_hash = "abcdef0123456789abcdef0123456789abcdef01";
        assert!(matcher.is_shape_or_custom_allowlisted(sha1_hash));
    }

    #[test]
    fn custom_allowlist_applies_to_high_confidence_path() {
        // Users opting an AWS key out via `custom_allowlist` must still
        // suppress high-confidence findings — this is the operator
        // escape hatch.
        let matcher = PatternMatcher::new(&["AKIAIOSFODNN7EXAMPLE".to_string()]);
        assert!(matcher.is_shape_or_custom_allowlisted("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn recognises_code_like_values() {
        let matcher = PatternMatcher::new(&[]);

        assert!(matcher.looks_like_code("fetch("));
        assert!(matcher.looks_like_code("service.call"));
        assert!(matcher.looks_like_code("https://eddacraft.dev"));
        assert!(matcher.looks_like_code("aB3dE7fG9hJ2kL4m"));
        assert!(!matcher.looks_like_structural_code("aB3dE7fG9hJ2kL4m"));
        assert!(matcher.looks_like_structural_code("fetch("));
        assert!(!matcher.looks_like_code("9xY7qW2vK8mN4pR6"));
    }

    #[test]
    fn redacts_secrets_and_lines() {
        let matcher = PatternMatcher::new(&[]);

        assert_eq!(matcher.redact_secret("short"), "***");
        assert_eq!(matcher.redact_secret("abcdefghijklmnop"), "abcd...mnop");
        assert_eq!(
            matcher.redact_line("token = 'abcdefghijklmnop'"),
            "token = '[REDACTED]'"
        );

        let github_token = format!("ghp_{}{}", "a".repeat(20), "b".repeat(20));
        let line = format!("token = {github_token}");
        let range_start = line.find(&github_token).unwrap_or(0);
        let range_end = range_start + github_token.len();

        assert_eq!(matcher.redact_line(&line), "token = [REDACTED]");
        assert_eq!(
            matcher.redact_range_in_line(&line, range_start, range_end),
            "token = [REDACTED]"
        );
    }

    #[test]
    fn redact_secret_handles_multibyte_utf8() {
        let matcher = PatternMatcher::new(&[]);
        // Each CJK char is 3 bytes — byte-slicing at index 4 would land inside
        // the second character on a naive &value[..4] slice and panic.
        let value = "\u{4e16}\u{754c}\u{4f60}\u{597d}\u{5f00}\u{59cb}\u{7ed3}\u{675f}\u{5b8c}";
        let redacted = matcher.redact_secret(value);
        assert!(
            redacted.contains("..."),
            "should contain ellipsis: {redacted}"
        );
        assert!(
            !redacted.contains("***"),
            "9-char input should not be fully redacted"
        );
    }
}
