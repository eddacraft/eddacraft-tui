//! Pure validation for the release-packaging public-key gate.
//!
//! Shared between `build.rs` (compile-time refuse for packaging) and the
//! integration test under `tests/release_public_key_gate.rs` so the
//! regression surface does not drift from the build script.
//!
//! Closes Clawpatch fnd_sig-feat-cli-command-c2cc6bd208-_e6b2eeb4df:
//! release packaging must fail when `ANVIL_RELEASE_PUBLIC_KEY` is absent
//! or still the committed development fallback.

/// Committed development minisign public key. Matching private key lives in
/// `tests/fixtures/minisign/` for fixture generation only — never ship a
/// binary that trusts this key for release verification.
pub const DEV_PUBLIC_KEY: &str = "RWRbilgipcbv8egsndfKxcAxjJCTusQPh/IsOy6ROFDiqvz8QNCVZRZ5";

/// Whether packaging should refuse to compile without a real release key.
///
/// True only when the env value is exactly `"1"`, matching the dashboard
/// `ANVIL_DASHBOARD_REQUIRE_BUNDLE` convention.
pub fn require_release_public_key_enabled(env_value: Option<&str>) -> bool {
    env_value == Some("1")
}

/// Accept only a non-empty key that is not the committed development fallback.
pub fn is_acceptable_release_public_key(candidate: &str) -> bool {
    let trimmed = candidate.trim();
    !trimmed.is_empty() && trimmed != DEV_PUBLIC_KEY
}

/// Human-readable reason a candidate is rejected, if any.
pub fn release_public_key_rejection(candidate: Option<&str>) -> Option<&'static str> {
    match candidate {
        None => Some(concat!(
            "ANVIL_REQUIRE_RELEASE_PUBLIC_KEY=1 but ANVIL_RELEASE_PUBLIC_KEY is unset. ",
            "Set vars.ANVIL_MINISIGN_PUBLIC_KEY before packaging — see docs/runbooks/release-signing.md.",
        )),
        Some(key) => {
            let trimmed = key.trim();
            if trimmed.is_empty() {
                Some(concat!(
                    "ANVIL_REQUIRE_RELEASE_PUBLIC_KEY=1 but ANVIL_RELEASE_PUBLIC_KEY is empty. ",
                    "Set vars.ANVIL_MINISIGN_PUBLIC_KEY before packaging — see docs/runbooks/release-signing.md.",
                ))
            } else if trimmed == DEV_PUBLIC_KEY {
                Some(concat!(
                    "ANVIL_REQUIRE_RELEASE_PUBLIC_KEY=1 but ANVIL_RELEASE_PUBLIC_KEY is still the ",
                    "committed development fallback. Generate a real keypair before releasing — ",
                    "see docs/runbooks/release-signing.md.",
                ))
            } else {
                None
            }
        }
    }
}
