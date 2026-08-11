//! Release-packaging build gate for DISTRIB-001 / ADR-045.
//!
//! When `ANVIL_REQUIRE_RELEASE_PUBLIC_KEY=1`, refuse to compile unless
//! `ANVIL_RELEASE_PUBLIC_KEY` is set to a non-empty value that is not the
//! committed development fallback. Mirrors the dashboard bundle gate
//! (`ANVIL_DASHBOARD_REQUIRE_BUNDLE`) so a misconfigured release job fails
//! at compile time rather than shipping a binary that silently trusts the
//! development minisign key.
//!
//! Clawpatch: fnd_sig-feat-cli-command-c2cc6bd208-_e6b2eeb4df.

#[path = "build_support/release_public_key_gate.rs"]
mod release_public_key_gate;

use release_public_key_gate::{
    is_acceptable_release_public_key, release_public_key_rejection,
    require_release_public_key_enabled,
};

fn main() {
    println!("cargo:rerun-if-env-changed=ANVIL_RELEASE_PUBLIC_KEY");
    println!("cargo:rerun-if-env-changed=ANVIL_REQUIRE_RELEASE_PUBLIC_KEY");
    // Pure gate module is included via `#[path]`; declare the dependency so
    // iterative edits to the support file re-run this script.
    println!("cargo:rerun-if-changed=build_support/release_public_key_gate.rs");

    let require = std::env::var("ANVIL_REQUIRE_RELEASE_PUBLIC_KEY").ok();
    if !require_release_public_key_enabled(require.as_deref()) {
        return;
    }

    let key = std::env::var("ANVIL_RELEASE_PUBLIC_KEY").ok();
    if let Some(reason) = release_public_key_rejection(key.as_deref()) {
        panic!("{reason}");
    }
    // Defence in depth: rejection and acceptance predicates stay inverted.
    let accepted = key.as_deref().is_some_and(is_acceptable_release_public_key);
    assert!(
        accepted,
        "release_public_key_rejection and is_acceptable_release_public_key disagree"
    );
}
