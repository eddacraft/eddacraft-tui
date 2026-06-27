pub(crate) fn is_test_or_fixture_path(path: &str) -> bool {
    let norm = path.replace('\\', "/").to_ascii_lowercase();
    let file = norm.rsplit('/').next().unwrap_or(norm.as_str());
    file.ends_with(".test.ts")
        || file.ends_with(".test.tsx")
        || file.ends_with(".spec.ts")
        || file.ends_with(".spec.tsx")
        || norm.split('/').any(|segment| {
            matches!(
                segment,
                "__tests__" | "tests" | "fixtures" | "fixture" | "test-data"
            )
        })
}

pub(crate) fn context_window(lines: &[&str], index: usize, radius: usize) -> String {
    let start = index.saturating_sub(radius);
    let end = (index + radius + 1).min(lines.len());
    lines[start..end].join("\n").to_ascii_lowercase()
}

fn contains_word(haystack: &str, word: &str) -> bool {
    let mut start = 0;
    while let Some(rel) = haystack[start..].find(word) {
        let idx = start + rel;
        let before_ok = idx == 0 || !haystack.as_bytes()[idx - 1].is_ascii_alphanumeric();
        let after_idx = idx + word.len();
        let after_ok =
            after_idx >= haystack.len() || !haystack.as_bytes()[after_idx].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        start = idx + 1;
    }
    false
}

pub(crate) fn has_validator_fixture_context(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "safeparse",
        "expect(",
        "toequal(",
        "tobe(",
        "z.parse(",
        "z.safeparse(",
        "z.jwt",
        ".jwt(",
        "base64.parse",
        "base64url",
        "sha256base64",
        "connectionstring",
        "invalid_union",
        "invalid_type",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
        || contains_word(&lower, "validator")
        || (contains_word(&lower, "schema") && (lower.contains("z.") || lower.contains("expect(")))
}

/// Benign secret-scan context is limited to test/fixture paths. Validator
/// fixture tokens are consulted separately for known test vectors.
pub(crate) fn is_benign_context(path: &str, _window: &str) -> bool {
    is_test_or_fixture_path(path)
}

pub(crate) fn has_sensitive_binding_context(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "api_key",
        "apikey",
        "access_token",
        "refresh_token",
        "auth_token",
        "authorization",
        "bearer",
        "client_secret",
        "jwt_secret",
        "sessiontoken",
        "session_token",
        "password",
        "passwd",
        "secret",
        "credential",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::{has_validator_fixture_context, is_benign_context, is_test_or_fixture_path};

    #[test]
    fn benign_context_requires_test_or_fixture_path() {
        assert!(is_benign_context(
            "packages/zod/src/v4/classic/tests/string.test.ts",
            "expect(z.parse(x, token)).toEqual(token)"
        ));
        assert!(!is_benign_context(
            "src/api/validator.ts",
            "expect(z.parse(x, token)).toEqual(token)"
        ));
        assert!(!is_benign_context(
            "src/config.ts",
            "const url = 'https://example.com'; // valid config"
        ));
    }

    #[test]
    fn validator_fixture_context_ignores_bare_url_and_valid_substrings() {
        assert!(!has_validator_fixture_context(
            "const url = 'https://example.com'; // valid config"
        ));
        assert!(has_validator_fixture_context(
            "expect(z.parse(a, \"eyJ...\")).toEqual(\"eyJ...\")"
        ));
    }

    #[test]
    fn examples_directory_is_not_benign_by_path() {
        assert!(!is_test_or_fixture_path("docs/examples/demo.ts"));
    }
}
