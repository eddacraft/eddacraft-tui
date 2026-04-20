use std::sync::LazyLock;

use regex::Regex;

use crate::secret::types::SecretPatternDef;

pub struct SecretPattern {
    pub name: &'static str,
    pub pattern: &'static str,
}

pub const SECRET_PATTERNS: [SecretPattern; 18] = [
    SecretPattern {
        name: "API Key",
        pattern: r#"(?i)(?:api[_-]?key|apikey)\s*[:=]\s*['\"]?[a-zA-Z0-9_-]{16,}['\"]?"#,
    },
    SecretPattern {
        name: "JWT Token",
        pattern: r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*",
    },
    SecretPattern {
        name: "AWS Key",
        pattern: r"AKIA[0-9A-Z]{16}",
    },
    SecretPattern {
        name: "AWS Secret Key",
        pattern: r#"(?i)(?:aws_secret|aws_secret_access_key)\s*[:=]\s*['\"]?[A-Za-z0-9/+=]{40}['\"]?"#,
    },
    SecretPattern {
        name: "Private Key",
        pattern: r"-----BEGIN\s+(?:RSA\s+)?PRIVATE\s+KEY-----",
    },
    SecretPattern {
        name: "PGP Private Key",
        pattern: r"-----BEGIN PGP PRIVATE KEY BLOCK-----",
    },
    SecretPattern {
        name: "Database URL",
        pattern: r"(?:postgres|mysql|mongodb|redis):\/\/[^:\s]+:[^@\s]+@",
    },
    SecretPattern {
        name: "Generic Secret",
        pattern: r#"(?i)(?:secret|password|passwd|pwd)\s*[:=]\s*['\"]?[^\s'\"]{8,}['\"]?"#,
    },
    SecretPattern {
        name: "Credit Card",
        pattern: r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b",
    },
    SecretPattern {
        name: "GitHub Token",
        pattern: r"gh[pousr]_[A-Za-z0-9_]{36,}",
    },
    SecretPattern {
        name: "Slack Token",
        pattern: r"xox[baprs]-[0-9]{10,13}-[0-9]{10,13}-[a-zA-Z0-9]{24}",
    },
    SecretPattern {
        name: "Stripe Key",
        pattern: r"sk_live_[0-9a-zA-Z]{24}",
    },
    SecretPattern {
        name: "Stripe Test Key",
        pattern: r"sk_test_[0-9a-zA-Z]{24}",
    },
    SecretPattern {
        name: "Google API Key",
        pattern: r"AIza[0-9A-Za-z_-]{35}",
    },
    SecretPattern {
        name: "Heroku API Key",
        pattern: r#"[hH]eroku[a-zA-Z0-9_-]*[:=]\s*['\"]?[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}['\"]?"#,
    },
    SecretPattern {
        name: "SendGrid API Key",
        pattern: r"SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43}",
    },
    SecretPattern {
        name: "Twilio API Key",
        pattern: r"SK[a-f0-9]{32}",
    },
    SecretPattern {
        name: "NPM Token",
        pattern: r"npm_[A-Za-z0-9]{36}",
    },
];

pub const DEFAULT_ALLOWLIST: [&str; 11] = [
    r"^[a-f0-9]{32}$",
    r"^[a-f0-9]{40}$",
    r"^[a-f0-9]{64}$",
    r"^0x[a-f0-9]+$",
    r"^data:image\/[a-z]+;base64,",
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
            }
        })
        .collect()
});

pub fn compile_secret_patterns(custom_patterns: &[SecretPatternDef]) -> Vec<CompiledPattern> {
    // Built-ins go through the fail-fast path; custom patterns tolerate
    // compile failures because they come from user config.
    let mut compiled: Vec<CompiledPattern> = DEFAULT_COMPILED_PATTERNS
        .iter()
        .map(|p| CompiledPattern {
            name: p.name.clone(),
            regex: p.regex.clone(),
        })
        .collect();
    compiled.extend(compile_custom_patterns(custom_patterns));
    compiled
}

/// Compile only the user-supplied custom patterns.
///
/// Pair the result with `DEFAULT_COMPILED_PATTERNS` when scanning — callers on
/// the hot path should chain the two iterators rather than calling
/// `compile_secret_patterns`, which recompiles the 18 built-ins every time.
pub fn compile_custom_patterns(custom_patterns: &[SecretPatternDef]) -> Vec<CompiledPattern> {
    custom_patterns
        .iter()
        .filter_map(|pattern| {
            Regex::new(&pattern.pattern)
                .ok()
                .map(|regex| CompiledPattern {
                    name: pattern.name.clone(),
                    regex,
                })
        })
        .collect()
}

pub struct PatternMatcher {
    default_allowlist: Vec<Regex>,
    custom_allowlist: Vec<Regex>,
}

impl PatternMatcher {
    pub fn new(custom_allowlist: &[String]) -> Self {
        let default_allowlist = DEFAULT_ALLOWLIST
            .iter()
            .filter_map(|pattern| Regex::new(&format!("(?i){pattern}")).ok())
            .collect();
        let custom_allowlist = custom_allowlist
            .iter()
            .filter_map(|pattern| Regex::new(&format!("(?i){pattern}")).ok())
            .collect();

        Self {
            default_allowlist,
            custom_allowlist,
        }
    }

    pub fn is_allowlisted(&self, value: &str) -> bool {
        self.default_allowlist
            .iter()
            .any(|pattern| pattern.is_match(value))
            || self
                .custom_allowlist
                .iter()
                .any(|pattern| pattern.is_match(value))
    }

    pub fn looks_like_code(&self, value: &str) -> bool {
        static CODE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
            [
                r"^[a-z][a-zA-Z0-9]*\(",
                r"^[a-z][a-zA-Z0-9]*\.[a-z]",
                r"^https?:\/\/",
                r"^[a-z]+:\/\/",
                r"\.(js|ts|css|html|json|md|txt)$",
                r"^[A-Z][A-Z0-9_]+$",
                r"^[a-z][a-z0-9]*[A-Z]",
                r"^[A-Z][a-z]+[A-Z]",
            ]
            .iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect()
        });

        CODE_PATTERNS.iter().any(|pattern| pattern.is_match(value))
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

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_ALLOWLIST, DEFAULT_COMPILED_PATTERNS, PatternMatcher, SECRET_PATTERNS,
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

        let examples: Vec<(&str, String)> = vec![
            ("API Key", "api_key='abcdEFGH1234567890'".into()),
            (
                "JWT Token",
                "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.sig".into(),
            ),
            ("AWS Key", "AKIAABCDEFGHIJKLMNOP".into()),
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
        ];

        let compiled = compile_secret_patterns(&[]);

        for (name, sample) in &examples {
            let maybe_pattern = compiled.iter().find(|pattern| pattern.name == *name);
            if let Some(pattern) = maybe_pattern {
                assert!(pattern.regex.is_match(sample), "{name} should match");
            } else {
                panic!("pattern not found: {name}");
            }
        }

        assert_eq!(SECRET_PATTERNS.len(), 18);
    }

    #[test]
    fn generic_secret_matches_compound_names() {
        let compiled = compile_secret_patterns(&[]);
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
    fn allows_default_and_custom_allowlist_patterns() {
        let matcher = PatternMatcher::new(&["my-safe-value".to_string()]);

        assert!(matcher.is_allowlisted("placeholder"));
        assert!(matcher.is_allowlisted("my-safe-value"));
        assert!(!matcher.is_allowlisted("ghp_abcdefghijklmnopqrstuvwxyz1234567890abc"));
        assert_eq!(DEFAULT_ALLOWLIST.len(), 11);
    }

    #[test]
    fn recognises_code_like_values() {
        let matcher = PatternMatcher::new(&[]);

        assert!(matcher.looks_like_code("fetch("));
        assert!(matcher.looks_like_code("service.call"));
        assert!(matcher.looks_like_code("https://eddacraft.dev"));
        assert!(matcher.looks_like_code("aB3dE7fG9hJ2kL4m"));
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
