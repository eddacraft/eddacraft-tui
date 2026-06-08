//! Capsule directory writer (GITGOV-004): assemble collected evidence
//! into the ADR-074 file-first layout with a digest-complete manifest.
//!
//! The writer owns the **placeholder discipline** for evidence whose
//! collectors have not landed yet: every ADR-074 file is written
//! (present-but-empty, never omitted), so a missing file is always a
//! tamper signal and never "not collected yet". `verification.json`
//! is the degraded no-checks placeholder — an unverified capsule
//! carries a machine-readable `degraded`, never silence.

use std::path::Path;

use crate::collect::CommitsDocument;
use crate::collect_digests::CollectedDigests;
use crate::errors::CapsuleError;
use crate::manifest::{CapsuleManifest, CapsuleRange, Producer};
use crate::verification::CapsuleVerification;

/// Minimal valid SARIF 2.1.0 document for the diagnostics placeholder
/// — a structured format's "present-but-empty" is an empty *document*,
/// not an empty byte stream (a 0-byte file would fail any consumer
/// that parses it before reading the degraded verdict). GITGOV-008
/// replaces this with real output from the shared ADR-058 emitter.
const EMPTY_SARIF: &str = r#"{"version":"2.1.0","runs":[]}"#;

/// Everything the writer needs to assemble a capsule directory.
#[derive(Debug, Clone)]
pub struct CapsuleContent {
    /// The collected commit range (GITGOV-005).
    pub commits: CommitsDocument,
    /// The collected digest documents (GITGOV-006).
    pub digests: CollectedDigests,
    /// Producer identity recorded in the manifest.
    pub producer: Producer,
}

/// Write a complete capsule directory at `out_dir` and return the
/// manifest that was written.
///
/// `out_dir` is created if missing; an existing **non-empty**
/// directory is refused so a capsule can never silently mix with (or
/// partially overwrite) prior content. Files are written directly
/// into the fresh directory — on a mid-write crash the directory is
/// simply incomplete and fails verification (`manifest.json` is
/// written last, so a manifest's presence implies every digest was
/// recorded).
///
/// Placeholders written by this step (their collectors land later):
///
/// - `witness.ndjson` — empty chain (GITGOV-007 collects the real
///   full chain; an empty file is valid NDJSON with no lines, and
///   verification reads it as missing witness evidence → `degraded`)
/// - `diagnostics.sarif` — minimal empty SARIF document (GITGOV-008
///   emits real output via the shared ADR-058 emitter)
/// - `exceptions.json` — `[]` (EXCEPT-009 collects applied records)
/// - `edda-context.json` — `{}` (reference-only, gated on EDDA-SEAL)
/// - `verification.json` — [`CapsuleVerification::from_checks`] with
///   no checks: the degraded placeholder GITGOV-009's verify step
///   overwrites
///
/// # Errors
///
/// [`CapsuleError::Collect`] for output-directory refusal or I/O
/// failure; [`CapsuleError::Serialise`] if a document cannot be
/// encoded.
pub fn write_capsule(
    out_dir: &Path,
    content: &CapsuleContent,
) -> Result<CapsuleManifest, CapsuleError> {
    prepare_out_dir(out_dir)?;

    let mut manifest = CapsuleManifest::new(
        CapsuleRange {
            base: content.commits.base.clone(),
            head: content.commits.head.clone(),
            // Pointers into the witness chain land with GITGOV-007.
            witness_seq_start: None,
            witness_seq_end: None,
        },
        content.producer.clone(),
    );

    let verification = CapsuleVerification::from_checks(vec![]);
    let readme = render_readme(content);

    let files: [(&str, Vec<u8>); 10] = [
        ("commits.json", content.commits.to_canonical_bytes()?),
        ("policy.json", content.digests.policy.to_canonical_bytes()?),
        (
            "baseline.json",
            content.digests.baseline.to_canonical_bytes()?,
        ),
        ("rules.json", content.digests.rules.to_canonical_bytes()?),
        ("witness.ndjson", Vec::new()),
        ("diagnostics.sarif", EMPTY_SARIF.as_bytes().to_vec()),
        ("exceptions.json", b"[]".to_vec()),
        ("edda-context.json", b"{}".to_vec()),
        ("verification.json", verification.to_canonical_bytes()?),
        ("README.md", readme.into_bytes()),
    ];

    for (name, bytes) in &files {
        write_file(out_dir, name, bytes)?;
        manifest.record_file(name, bytes);
    }

    // Layout invariant: every ADR-074 required file must be recorded
    // before the manifest is written. `files` is type-pinned to the
    // required count, but the names could still drift.
    let missing = manifest.missing_required();
    if !missing.is_empty() {
        return Err(CapsuleError::Serialise(format!(
            "capsule writer missed required files: {missing:?}"
        )));
    }

    let manifest_bytes = manifest.to_canonical_bytes()?;
    write_file(out_dir, "manifest.json", &manifest_bytes)?;

    Ok(manifest)
}

/// Create `out_dir` if missing; refuse a symlinked or non-empty
/// existing directory.
///
/// The symlink check (`lstat`, final component) keeps a
/// symlink-to-empty-dir from silently landing the capsule somewhere
/// other than the named path. The emptiness check is
/// check-then-write — a concurrent writer can still add *foreign*
/// files alongside ours (the per-file `create_new` writes refuse
/// collisions on capsule-owned names); the verifier treats files not
/// listed in the manifest as a finding, so mixed content is caught at
/// verification, not silently absorbed.
fn prepare_out_dir(out_dir: &Path) -> Result<(), CapsuleError> {
    if let Ok(md) = std::fs::symlink_metadata(out_dir)
        && md.file_type().is_symlink()
    {
        return Err(out_dir_error(
            out_dir,
            "is a symlink; name the real destination",
        ));
    }
    match std::fs::read_dir(out_dir) {
        Ok(mut entries) => {
            if entries.next().is_some() {
                return Err(out_dir_error(
                    out_dir,
                    "is not empty; refusing to write a capsule into existing content",
                ));
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => std::fs::create_dir_all(out_dir)
            .map_err(|e| out_dir_error(out_dir, &format!("could not be created: {e}"))),
        Err(e) => Err(out_dir_error(out_dir, &format!("could not be read: {e}"))),
    }
}

fn out_dir_error(out_dir: &Path, detail: &str) -> CapsuleError {
    CapsuleError::Collect {
        path: out_dir.display().to_string(),
        detail: format!("output directory {detail}"),
    }
}

/// Exclusive-create write: a concurrent file at the same name (racing
/// creator, leftover content past the emptiness check) is an error,
/// never an overwrite.
fn write_file(out_dir: &Path, name: &str, bytes: &[u8]) -> Result<(), CapsuleError> {
    use std::io::Write;
    let path = out_dir.join(name);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| CapsuleError::Collect {
            path: name.to_string(),
            detail: format!("creating: {e}"),
        })?;
    file.write_all(bytes).map_err(|e| CapsuleError::Collect {
        path: name.to_string(),
        detail: format!("writing: {e}"),
    })
}

/// Deterministic human-readable summary. No timestamps — the same
/// content must produce byte-identical capsules.
fn render_readme(content: &CapsuleContent) -> String {
    let policy = content
        .digests
        .policy
        .policy_file
        .as_ref()
        .map_or("absent", |file| file.path.as_str());
    let baseline = if content.digests.baseline.digest.is_some() {
        "present"
    } else {
        "absent"
    };
    let rules = content
        .digests
        .rules
        .rules_sha
        .as_deref()
        .unwrap_or("absent (no .anvil.* config)");
    format!(
        "# Anvil Review Capsule\n\n\
         Governance evidence for the commit range below, packaged per\n\
         ADR-074 (`anvil.capsule.v1`). Verify with `anvil capsule verify`;\n\
         every file's SHA-256 is recorded in `manifest.json`.\n\n\
         | Field | Value |\n\
         | ----- | ----- |\n\
         | Base | `{base}` |\n\
         | Head | `{head}` |\n\
         | Commits | {commits} |\n\
         | Policy | {policy} |\n\
         | Baseline | {baseline} |\n\
         | Rules identity | `{rules}` |\n\
         | Producer | anvil {version} |\n\n\
         `verification.json` starts as a degraded placeholder — an\n\
         unverified capsule never claims `pass`. `witness.ndjson` and\n\
         `diagnostics.sarif` are structural stubs until their\n\
         collectors land (GITGOV-007/-008); empty here means \"not yet\n\
         collected\", not \"no findings\".\n",
        base = content.commits.base,
        head = content.commits.head,
        commits = content.commits.commits.len(),
        version = content.producer.anvil_version,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::sha256_hex;
    use crate::collect::COMMITS_SCHEMA;
    use crate::collect_digests::{
        BASELINE_DIGEST_SCHEMA, BaselineDigest, POLICY_DIGEST_SCHEMA, PolicyDigest,
        RULES_DIGEST_SCHEMA, RulesDigest,
    };
    use crate::manifest::REQUIRED_FILES;
    use crate::verification::Verdict;

    fn sample_content() -> CapsuleContent {
        CapsuleContent {
            commits: CommitsDocument {
                schema: COMMITS_SCHEMA.to_string(),
                base: "1111111111111111111111111111111111111111".to_string(),
                head: "2222222222222222222222222222222222222222".to_string(),
                commits: vec![],
            },
            digests: CollectedDigests {
                policy: PolicyDigest {
                    schema: POLICY_DIGEST_SCHEMA.to_string(),
                    policy_file: None,
                    config_file: None,
                },
                rules: RulesDigest {
                    schema: RULES_DIGEST_SCHEMA.to_string(),
                    anvil_version: "0.0.0-test".to_string(),
                    opa_runtime_version: "0.10.0".to_string(),
                    rules: vec![],
                    config_sha: None,
                    rules_sha: None,
                },
                baseline: BaselineDigest {
                    schema: BASELINE_DIGEST_SCHEMA.to_string(),
                    cutoff_commit: None,
                    digest: None,
                },
            },
            producer: Producer {
                anvil_version: "0.0.0-test".to_string(),
            },
        }
    }

    #[test]
    fn write_capsule_writes_every_required_file_plus_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");

        let manifest = write_capsule(&out, &sample_content()).unwrap();

        for name in REQUIRED_FILES {
            assert!(out.join(name).exists(), "{name} missing");
        }
        assert!(out.join("manifest.json").exists());
        assert!(manifest.missing_required().is_empty());
    }

    /// Every manifest digest must match the bytes on disk — the byte
    /// contract a verifier will enforce.
    #[test]
    fn write_capsule_manifest_digests_match_disk_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");

        let manifest = write_capsule(&out, &sample_content()).unwrap();

        for (name, digest) in &manifest.files {
            let bytes = std::fs::read(out.join(name)).unwrap();
            assert_eq!(&sha256_hex(&bytes), digest, "digest mismatch for {name}");
        }
        // And the manifest file itself round-trips.
        let manifest_bytes = std::fs::read(out.join("manifest.json")).unwrap();
        let parsed = CapsuleManifest::from_json_bytes(&manifest_bytes).unwrap();
        assert_eq!(parsed, manifest);
    }

    /// The unverified capsule carries a machine-readable degraded
    /// verdict, never silence (ADR-072 §4).
    #[test]
    fn write_capsule_verification_placeholder_is_degraded() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");

        write_capsule(&out, &sample_content()).unwrap();

        let bytes = std::fs::read(out.join("verification.json")).unwrap();
        let verification = CapsuleVerification::from_json_bytes(&bytes).unwrap();
        assert_eq!(verification.verdict, Verdict::Degraded);
        assert!(verification.checks.is_empty());
    }

    /// The diagnostics placeholder is a valid (empty) SARIF document,
    /// not a 0-byte stream — consumers that parse it before reading
    /// the degraded verdict must not crash.
    #[test]
    fn write_capsule_diagnostics_placeholder_is_valid_sarif() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");

        write_capsule(&out, &sample_content()).unwrap();

        let bytes = std::fs::read(out.join("diagnostics.sarif")).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["version"], "2.1.0");
        assert!(value["runs"].as_array().unwrap().is_empty());
    }

    #[test]
    fn write_capsule_is_deterministic_for_same_content() {
        let dir = tempfile::tempdir().unwrap();
        let out_a = dir.path().join("a");
        let out_b = dir.path().join("b");

        let a = write_capsule(&out_a, &sample_content()).unwrap();
        let b = write_capsule(&out_b, &sample_content()).unwrap();

        assert_eq!(a, b);
        assert_eq!(
            std::fs::read(out_a.join("manifest.json")).unwrap(),
            std::fs::read(out_b.join("manifest.json")).unwrap()
        );
    }

    #[test]
    fn write_capsule_refuses_non_empty_out_dir() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("keep.txt"), "existing").unwrap();

        let err = write_capsule(&out, &sample_content()).unwrap_err();

        assert!(err.to_string().contains("not empty"), "{err}");
        assert_eq!(
            std::fs::read_to_string(out.join("keep.txt")).unwrap(),
            "existing",
            "existing content untouched"
        );
    }

    /// A symlinked `--out` must not silently land the capsule at the
    /// symlink's target.
    #[cfg(unix)]
    #[test]
    fn write_capsule_refuses_symlinked_out_dir() {
        let target = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();
        let out = staging.path().join("capsule");
        std::os::unix::fs::symlink(target.path(), &out).unwrap();

        let err = write_capsule(&out, &sample_content()).unwrap_err();

        assert!(err.to_string().contains("symlink"), "{err}");
        assert!(
            !target.path().join("manifest.json").exists(),
            "nothing written through the symlink"
        );
    }

    #[test]
    fn write_capsule_accepts_existing_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("capsule");
        std::fs::create_dir_all(&out).unwrap();

        write_capsule(&out, &sample_content()).unwrap();

        assert!(out.join("manifest.json").exists());
    }
}
