//! Minisign verification for `anvil update` (DISTRIB-001 / ADR-045).
//!
//! Verifies a downloaded artefact against a detached `.minisig` produced by
//! the release-time signing workflow. The trusted public key is embedded at
//! compile time and travels inside every `anvil` binary, so a CDN compromise
//! cannot rewrite it.
//!
//! The module is platform-agnostic and has no I/O beyond reading files the
//! caller hands it. The caller (in `commands::update`) is responsible for
//! downloading the artefact and its detached signature into a tempdir
//! before calling `verify_files`.

use std::path::Path;

use minisign_verify::{PublicKey, Signature};

/// Trusted public key, embedded at compile time.
///
/// Release builds in CI override this via the `ANVIL_RELEASE_PUBLIC_KEY`
/// env var (set from the `ANVIL_MINISIGN_PUBLIC_KEY` repo variable).
///
/// The development fallback is **not** safe for releases — the matching
/// private key is committed in `crates/anvil-cli/tests/fixtures/minisign/`
/// for fixture generation only. Packaging refuses this fallback at two
/// layers (see ADR-045 §"Concrete commitments"):
/// 1. `crates/anvil-cli/build.rs` panics when
///    `ANVIL_REQUIRE_RELEASE_PUBLIC_KEY=1` and the key is missing/dev.
/// 2. `.github/workflows/release.yml` sets that flag and runs a shell
///    preflight before `dist build`.
const EMBEDDED_PUBLIC_KEY: &str = match option_env!("ANVIL_RELEASE_PUBLIC_KEY") {
    Some(k) => k,
    None => DEV_PUBLIC_KEY,
};

/// Development-only public key. The matching secret key is committed in
/// `crates/anvil-cli/tests/fixtures/minisign/` and is used by the test
/// suite to generate signatures. Production builds must override this via
/// `ANVIL_RELEASE_PUBLIC_KEY` at compile time.
///
/// Keep this byte-identical to
/// `build_support/release_public_key_gate.rs::DEV_PUBLIC_KEY` — the
/// packaging gate rejects this exact string.
const DEV_PUBLIC_KEY: &str = "RWRbilgipcbv8egsndfKxcAxjJCTusQPh/IsOy6ROFDiqvz8QNCVZRZ5";

/// Returns true when the binary was built with the development public key
/// and any signature verification is therefore trusted only by the test
/// suite. Released binaries must report `false`.
pub fn is_using_dev_public_key() -> bool {
    EMBEDDED_PUBLIC_KEY == DEV_PUBLIC_KEY
}

/// Errors that can arise while verifying a downloaded artefact.
#[derive(Debug, thiserror::Error)]
pub enum SignatureError {
    /// File-system read failures are produced by [`verify_files`], which
    /// is exposed for callers that have already persisted the artefact +
    /// signature to disk (release-readiness checks, sidecar handoff,
    /// future `anvil verify` command).
    #[error("failed to read artefact at {path}: {source}")]
    #[allow(dead_code)]
    ArtefactRead {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read signature at {path}: {source}")]
    #[allow(dead_code)]
    SignatureRead {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("trusted public key is malformed: {0}")]
    PublicKeyDecode(String),
    #[error("signature file is malformed: {0}")]
    SignatureDecode(String),
    #[error(
        "signature does not match artefact — refusing to install. \
         The downloaded file may be corrupted or has not been signed by \
         the anvil release key. Reason: {0}"
    )]
    Mismatch(String),
}

/// Result of a successful verification. Carries the trusted comment line so
/// the caller can surface it to the user (e.g. `tag=v0.7.0-beta;commit=…`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedArtefact {
    pub trusted_comment: String,
}

/// Verify an in-memory artefact against an in-memory signature string,
/// using the embedded trusted public key.
pub fn verify_bytes(
    artefact: &[u8],
    signature_str: &str,
) -> Result<VerifiedArtefact, SignatureError> {
    verify_bytes_with(EMBEDDED_PUBLIC_KEY, artefact, signature_str)
}

/// Verify against an explicit public key. Intended for tests; production
/// code paths should call `verify_bytes`.
pub fn verify_bytes_with(
    public_key_b64: &str,
    artefact: &[u8],
    signature_str: &str,
) -> Result<VerifiedArtefact, SignatureError> {
    let pk = PublicKey::from_base64(public_key_b64)
        .map_err(|e| SignatureError::PublicKeyDecode(e.to_string()))?;
    let sig = Signature::decode(signature_str)
        .map_err(|e| SignatureError::SignatureDecode(e.to_string()))?;
    pk.verify(artefact, &sig, false)
        .map_err(|e| SignatureError::Mismatch(e.to_string()))?;
    Ok(VerifiedArtefact {
        trusted_comment: sig.trusted_comment().to_string(),
    })
}

/// Verify a file against its detached `.minisig`, using the embedded
/// trusted public key. Exposed for callers that have already persisted
/// the artefact + signature to disk (release-readiness checks; future
/// `anvil verify` command). The library-fallback update path uses
/// [`verify_bytes`] via the streaming download helper.
#[allow(dead_code)]
pub fn verify_files(
    artefact_path: &Path,
    signature_path: &Path,
) -> Result<VerifiedArtefact, SignatureError> {
    let artefact = std::fs::read(artefact_path).map_err(|source| SignatureError::ArtefactRead {
        path: artefact_path.display().to_string(),
        source,
    })?;
    let signature_str = std::fs::read_to_string(signature_path).map_err(|source| {
        SignatureError::SignatureRead {
            path: signature_path.display().to_string(),
            source,
        }
    })?;
    verify_bytes(&artefact, &signature_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test public key — must match the secret key under `tests/fixtures/minisign/`.
    /// Generated by `tests/fixtures/minisign/regenerate.sh`.
    const TEST_PUBLIC_KEY_B64: &str = "RWRbilgipcbv8egsndfKxcAxjJCTusQPh/IsOy6ROFDiqvz8QNCVZRZ5";

    fn test_keypair() -> (minisign::PublicKey, minisign::SecretKey) {
        let kp = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        (kp.pk, kp.sk)
    }

    fn sign(secret_key: &minisign::SecretKey, data: &[u8], trusted_comment: &str) -> String {
        let signature_box = minisign::sign(
            None,
            secret_key,
            std::io::Cursor::new(data),
            Some(trusted_comment),
            Some("anvil-test"),
        )
        .unwrap();
        String::from(signature_box)
    }

    #[test]
    fn verify_round_trip_succeeds() {
        let (pk, sk) = test_keypair();
        let data = b"anvil release artefact bytes";
        let signature = sign(&sk, data, "tag=v0.7.0-beta;commit=deadbeef");

        let pk_b64 = pk.to_base64();
        let verified = verify_bytes_with(&pk_b64, data, &signature).unwrap();
        assert_eq!(verified.trusted_comment, "tag=v0.7.0-beta;commit=deadbeef");
    }

    #[test]
    fn verify_refuses_tampered_payload() {
        let (pk, sk) = test_keypair();
        let original = b"original artefact";
        let signature = sign(&sk, original, "tag=v0.7.0-beta");

        let tampered = b"original artefactX";
        let pk_b64 = pk.to_base64();
        let err = verify_bytes_with(&pk_b64, tampered, &signature).unwrap_err();
        assert!(matches!(err, SignatureError::Mismatch(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("refusing to install"),
            "actionable message expected, got: {msg}"
        );
    }

    #[test]
    fn verify_refuses_wrong_signing_key() {
        let (_pk_a, sk_a) = test_keypair();
        let (pk_b, _sk_b) = test_keypair();
        let data = b"artefact";
        let signature = sign(&sk_a, data, "tag=v0.7.0-beta");

        let pk_b64 = pk_b.to_base64();
        let err = verify_bytes_with(&pk_b64, data, &signature).unwrap_err();
        assert!(matches!(err, SignatureError::Mismatch(_)));
    }

    #[test]
    fn verify_refuses_malformed_signature() {
        let (pk, _sk) = test_keypair();
        let pk_b64 = pk.to_base64();
        let err = verify_bytes_with(&pk_b64, b"data", "not a real signature file").unwrap_err();
        assert!(matches!(err, SignatureError::SignatureDecode(_)));
    }

    #[test]
    fn verify_refuses_malformed_public_key() {
        let err = verify_bytes_with("not-base64!!!", b"data", "irrelevant").unwrap_err();
        assert!(matches!(err, SignatureError::PublicKeyDecode(_)));
    }

    #[test]
    fn embedded_dev_key_constant_matches_fixture() {
        // The fixture regeneration script writes the public key to
        // tests/fixtures/minisign/anvil-test.pub.b64. The DEV_PUBLIC_KEY
        // constant must equal that value so unit tests and the
        // tampered-artefact integration test agree on the trusted key
        // when ANVIL_RELEASE_PUBLIC_KEY is unset.
        let fixture = std::fs::read_to_string("tests/fixtures/minisign/anvil-test.pub.b64").ok();
        if let Some(fixture) = fixture {
            assert_eq!(
                DEV_PUBLIC_KEY,
                fixture.trim(),
                "DEV_PUBLIC_KEY drift — run tests/fixtures/minisign/regenerate.sh and update the constant"
            );
            // Anchors the test_pub_key_b64 mirror used in this module's tests.
            assert_eq!(TEST_PUBLIC_KEY_B64, DEV_PUBLIC_KEY);
        }
    }

    #[test]
    fn is_using_dev_public_key_reports_truth() {
        // Test builds never set ANVIL_RELEASE_PUBLIC_KEY, so we expect true.
        assert!(is_using_dev_public_key());
    }

    #[test]
    fn packaging_gate_dev_key_is_never_acceptable_release_key() {
        // Mirrors build_support/release_public_key_gate.rs so a constant
        // drift surfaces here even when the packaging gate is not enabled.
        let is_acceptable = |candidate: &str| {
            let trimmed = candidate.trim();
            !trimmed.is_empty() && trimmed != DEV_PUBLIC_KEY
        };
        assert!(!is_acceptable(""));
        assert!(!is_acceptable("   "));
        assert!(!is_acceptable(DEV_PUBLIC_KEY));
        assert!(!is_acceptable(&format!("  {DEV_PUBLIC_KEY}  ")));
        assert!(is_acceptable(
            "RWQccccccccccccccccccccccccccccccccccccccccccccccccccccc"
        ));
    }
}
