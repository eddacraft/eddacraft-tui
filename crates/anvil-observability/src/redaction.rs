//! Redaction deny-list for span attributes and JSON-formatted log fields.
//!
//! # ⚠️ Advisory-only — NOT enforced
//!
//! This module is the **constant table** TRACE-003 will plug into a
//! `tracing-subscriber` redaction layer. Today nothing reads
//! [`SENSITIVE_FIELDS`] at runtime; emitting a span attribute named
//! `password` or `token` will appear in JSON output unredacted until
//! TRACE-003 lands. Callers that *want* a quick advisory check before
//! adding an attribute can call [`is_sensitive_field`], but the
//! subscriber does **not** strip values it returns true for. See
//! ADR-035's R1 risk acceptance for the launch-time gap rationale.
//!
//! The marker [`REDACTED`] is the canonical replacement string the
//! TRACE-003 layer will emit; tests pin it today so a contract change
//! is loud.

/// Canonical replacement value for redacted span attributes / log fields.
pub const REDACTED: &str = "<redacted>";

/// Field names whose **values** must never be forwarded into a span
/// attribute or formatted log line. Lower-case; comparison is
/// case-insensitive.
///
/// Sourced from the OWASP secret-name patterns Anvil's secret-detection
/// rule already recognises, plus the deny-list INTD-013 reviewers
/// flagged on `notification.context`.
pub const SENSITIVE_FIELDS: &[&str] = &[
    "api_key",
    "apikey",
    "access_key",
    "auth",
    "authorization",
    "bearer",
    "client_secret",
    "credential",
    "credentials",
    "password",
    "passwd",
    "pwd",
    "private_key",
    "secret",
    "session_token",
    "token",
];

/// Returns `true` if `field` exactly matches a known-sensitive field
/// name (case-insensitive). Substrings do **not** match: `token_type`
/// is allowed even though `token` is on the list. Callers that need
/// pattern matching must layer their own logic on top.
///
/// **Advisory-only** — the runtime subscriber installed by
/// [`init_tracing`](super::init_tracing) does NOT consult this. See
/// the module-level note.
#[must_use]
pub fn is_sensitive_field(field: &str) -> bool {
    SENSITIVE_FIELDS
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(field))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_lowercase_canonical_form() {
        assert!(is_sensitive_field("api_key"));
        assert!(is_sensitive_field("authorization"));
        assert!(is_sensitive_field("password"));
    }

    #[test]
    fn matches_case_insensitively() {
        assert!(is_sensitive_field("API_KEY"));
        assert!(is_sensitive_field("Authorization"));
        assert!(is_sensitive_field("Password"));
    }

    #[test]
    fn rejects_unrelated_names() {
        assert!(!is_sensitive_field("path"));
        assert!(!is_sensitive_field("trace_id"));
        assert!(!is_sensitive_field(""));
    }

    #[test]
    fn matches_exactly_not_substring() {
        // `token` is on the list; `token_type` and `pagination_token`
        // are common safe field names that must not be redacted.
        assert!(is_sensitive_field("token"));
        assert!(!is_sensitive_field("token_type"));
        assert!(!is_sensitive_field("pagination_token"));
        assert!(!is_sensitive_field("session_token_type"));
    }

    #[test]
    fn redacted_marker_is_stable() {
        // Pinned: TRACE-003 layer asserts on this exact string. Changing
        // it is a contract break across binary boundaries.
        assert_eq!(REDACTED, "<redacted>");
    }
}
