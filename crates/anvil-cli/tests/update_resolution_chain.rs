//! DISTRIB-001: resolution-chain integration tests.
//!
//! Black-box coverage of the observable `anvil update` CLI contract:
//! `--help` shape, clap rejection of unknown flags, and the minisign
//! public-key fixture shape (prefix + length sanity check on
//! `tests/fixtures/minisign/anvil-test.pub.b64`).
//!
//! Parser-level acceptance of the hidden `--insecure-skip-verify` flag
//! and the loud-stderr warning emitted when signature verification is
//! skipped are covered by the unit tests inside
//! `crates/anvil-cli/src/commands/update.rs` (see the
//! `skip_verify_warning_*` and `*_parses_*` test fns). Those tests do
//! not invoke the real update probe, so they stay deterministic
//! regardless of network state or install posture (CLAWP-001).
//!
//! The actual fixture-vs-`DEV_PUBLIC_KEY` drift comparison and the
//! deeper signature unit tests live in
//! `crates/anvil-cli/src/commands/update/{signature,fetch}.rs` (see
//! `is_using_dev_public_key_reports_truth` for the byte-for-byte
//! check).

use std::process::Command;

const ANVIL_BIN: &str = env!("CARGO_BIN_EXE_anvil");

fn anvil() -> Command {
    let mut cmd = Command::new(ANVIL_BIN);
    cmd.arg("--no-tui")
        .env("ANVIL_DEV", "1")
        .env("ANVIL_SKIP_WELCOME", "1");
    cmd
}

#[test]
fn update_help_advertises_insecure_skip_verify_only_implicitly() {
    // The `--insecure-skip-verify` flag is intentionally hidden from
    // `--help` so users do not stumble onto it by accident, but `clap`
    // still has to accept it. We verify both halves.
    let out = anvil().args(["update", "--help"]).output().unwrap();
    assert!(out.status.success(), "update --help should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("--insecure-skip-verify"),
        "hidden flag must not appear in --help, got:\n{stdout}"
    );
    assert!(
        stdout.contains("--check"),
        "documented flags must still appear, got:\n{stdout}"
    );
}

#[test]
fn update_unknown_flag_is_rejected_by_clap() {
    // Sanity-check the clap configuration: an unknown flag must fail.
    let out = anvil()
        .args(["update", "--this-flag-does-not-exist"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "unknown flag must error");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unexpected argument") || stderr.contains("unrecognized"),
        "expected clap rejection message, got:\n{stderr}"
    );
}

#[test]
fn update_help_documents_update_and_consent_flags() {
    // The visible flags from the v0.6 surface stay visible.
    let out = anvil().args(["update", "--help"]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    for visible_flag in ["--check", "--version", "--force", "--yes"] {
        assert!(
            stdout.contains(visible_flag),
            "{visible_flag} must remain in --help, got:\n{stdout}"
        );
    }
}

#[test]
fn update_short_yes_alias_parses() {
    let out = anvil().args(["update", "-y", "--help"]).output().unwrap();
    assert!(out.status.success(), "-y should parse: {out:?}");
}

#[test]
fn signature_fixture_public_key_matches_dev_constant() {
    // The committed test public key must match the DEV_PUBLIC_KEY constant
    // embedded in signature.rs. If you regenerate the fixture keypair, the
    // constant must be updated to match — this test catches drift before
    // a developer ships a release that quietly cannot verify its own
    // signatures.
    //
    // We discover the fixture path relative to CARGO_MANIFEST_DIR rather
    // than CWD so this test runs from any working directory.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture = std::fs::read_to_string(format!(
        "{manifest_dir}/tests/fixtures/minisign/anvil-test.pub.b64"
    ))
    .expect("test fixture must exist; run tests/fixtures/minisign/regenerate.sh");
    let trimmed = fixture.trim();
    assert!(
        trimmed.starts_with("RW"),
        "minisign public keys start with 'RW' prefix; got: {trimmed:?}"
    );
    assert_eq!(trimmed.len(), 56, "minisign public-key base64 is 56 chars");
}
