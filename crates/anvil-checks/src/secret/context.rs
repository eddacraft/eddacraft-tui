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
                "__tests__"
                    | "tests"
                    | "fixtures"
                    | "fixture"
                    | "test-data"
                    | "examples"
                    | "bench"
                    | "benchmarks"
            )
        })
}

pub(crate) fn context_window(lines: &[&str], index: usize, radius: usize) -> String {
    let start = index.saturating_sub(radius);
    let end = (index + radius + 1).min(lines.len());
    lines[start..end].join("\n").to_ascii_lowercase()
}

pub(crate) fn has_validator_fixture_context(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "parse",
        "safeparse",
        "expect",
        "valid",
        "invalid",
        "validator",
        "schema",
        "connectionstring",
        "base64",
        "base64url",
        "sha256base64",
        ".jwt",
        "z.jwt",
        "jwt",
        "url",
        "uri",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(crate) fn is_benign_context(path: &str, window: &str) -> bool {
    is_test_or_fixture_path(path) || has_validator_fixture_context(window)
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
