//! Regression tests for the release-packaging public-key build gate
//! (Clawpatch fnd_sig-feat-cli-command-c2cc6bd208-_e6b2eeb4df / DISTRIB-001).
//!
//! These cover the pure validation logic shared with `build.rs`. The
//! workflow wiring that sets `ANVIL_REQUIRE_RELEASE_PUBLIC_KEY=1` is
//! asserted by `scripts/ci/release-public-key-build-gate.test.sh`.

#[path = "../build_support/release_public_key_gate.rs"]
mod release_public_key_gate;

use release_public_key_gate::{
    DEV_PUBLIC_KEY, is_acceptable_release_public_key, release_public_key_rejection,
    require_release_public_key_enabled,
};

#[test]
fn require_flag_only_accepts_exact_one() {
    assert!(!require_release_public_key_enabled(None));
    assert!(!require_release_public_key_enabled(Some("")));
    assert!(!require_release_public_key_enabled(Some("0")));
    assert!(!require_release_public_key_enabled(Some("true")));
    assert!(!require_release_public_key_enabled(Some("yes")));
    assert!(require_release_public_key_enabled(Some("1")));
}

#[test]
fn rejects_missing_empty_and_dev_fallback() {
    assert!(release_public_key_rejection(None).is_some());
    assert!(release_public_key_rejection(Some("")).is_some());
    assert!(release_public_key_rejection(Some("   ")).is_some());
    assert!(release_public_key_rejection(Some(DEV_PUBLIC_KEY)).is_some());
    assert!(release_public_key_rejection(Some(&format!("  {DEV_PUBLIC_KEY}  "))).is_some());
}

#[test]
fn accepts_non_dev_key() {
    // Shape of a real minisign public key (not the committed fixture).
    let productionish = "RWQaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    assert!(is_acceptable_release_public_key(productionish));
    assert!(release_public_key_rejection(Some(productionish)).is_none());
    assert!(release_public_key_rejection(Some(&format!("  {productionish}  "))).is_none());
}

#[test]
fn is_acceptable_mirrors_rejection() {
    assert!(!is_acceptable_release_public_key(""));
    assert!(!is_acceptable_release_public_key(DEV_PUBLIC_KEY));
    assert!(is_acceptable_release_public_key(
        "RWQbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    ));
}

#[test]
fn dev_public_key_matches_signature_module_constant() {
    // Keep the build-gate DEV_PUBLIC_KEY byte-identical to the constant
    // embedded by signature.rs. Drift would let packaging pass while the
    // runtime still thinks it holds a release key (or vice versa).
    //
    // The signature module constant is private; compare against the same
    // fixture file both sites must track.
    let fixture = include_str!("fixtures/minisign/anvil-test.pub.b64");
    assert_eq!(
        DEV_PUBLIC_KEY,
        fixture.trim(),
        "build-gate DEV_PUBLIC_KEY drifted from tests/fixtures/minisign/anvil-test.pub.b64"
    );
}
