//! Pack manifest format and loader (POLVAL-002).
//!
//! A pack manifest is a single YAML file describing the pack (id, name,
//! version, description, ownership) and its member policies (a `.rego` path
//! plus that policy's [`PolicyMetadata`]). [`load_manifest`] parses one
//! manifest file, validates it, and returns members in manifest order.
//!
//! Constraints:
//! - A missing manifest maps to [`ManifestError::NotFound`]; no parse or I/O
//!   failure is ever folded into a default — every failure propagates as
//!   [`Err`].
//! - Loading reads only the manifest file. Member `.rego` files are not opened
//!   and the filesystem is not walked beyond the manifest's own directory;
//!   member paths are checked lexically to stay within it.
//! - Member ordering is the manifest's declared order (deterministic).
//! - Unknown fields on the manifest root and on member entries are rejected
//!   (`deny_unknown_fields`) so an older engine reading a newer manifest fails
//!   closed and loudly, rather than silently ignoring policy entries it does
//!   not understand.

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::pack::metadata::{MetadataError, PolicyMetadata, ensure_unique_ids};

/// One policy member of a pack: its Rego source path plus its metadata.
///
/// `path` is relative to the manifest's own directory. It is not opened during
/// loading; only its shape is checked (see [`ManifestError::PathEscapesManifest`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyEntry {
    /// Path to the `.rego` source, relative to the manifest directory.
    pub path: PathBuf,
    /// Inline metadata describing this policy.
    pub metadata: PolicyMetadata,
}

/// A parsed and validated policy pack manifest.
///
/// `policies` is kept in the manifest's declared order so a load is
/// deterministic. Unknown top-level fields are rejected so a newer manifest
/// cannot be silently under-read by an older engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackManifest {
    /// Unique pack identifier.
    pub id: String,
    /// Human-readable pack name.
    pub name: String,
    /// Pack version string (opaque to the loader).
    pub version: String,
    /// What the pack is for — its intent.
    pub description: String,
    /// Accountable owner for the pack as a whole.
    pub owner: String,
    /// Member policies, in declared order.
    #[serde(default)]
    pub policies: Vec<PolicyEntry>,
}

/// A manifest load or validation failure. User-facing text uses UK spelling.
#[derive(Debug, Error)]
pub enum ManifestError {
    /// The manifest file does not exist.
    #[error("pack manifest not found: {0}")]
    NotFound(PathBuf),
    /// The manifest file could not be read (other than not-found).
    #[error("could not read pack manifest {path}: {source}")]
    Io {
        /// The manifest path.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The manifest file is not valid YAML for the manifest schema (includes an
    /// unknown field or an unrecognised severity band).
    #[error("could not parse pack manifest {path}: {message}")]
    Parse {
        /// The manifest path.
        path: PathBuf,
        /// The parser's message.
        message: String,
    },
    /// A required pack-level field is absent or blank.
    #[error("pack manifest is missing required field `{field}`; set a non-blank `{field}` value")]
    MissingField {
        /// The name of the missing field.
        field: &'static str,
    },
    /// A member's metadata failed validation.
    #[error(transparent)]
    Metadata(#[from] MetadataError),
    /// A member path is absolute or escapes the manifest directory.
    #[error(
        "policy `{policy_id}` path `{path}` escapes the manifest directory; \
         members must be relative paths beside the manifest with no `..` segments"
    )]
    PathEscapesManifest {
        /// The `id` of the offending policy.
        policy_id: String,
        /// The offending path.
        path: PathBuf,
    },
}

/// Load and validate a pack manifest from `path`.
///
/// Reads only `path`. A missing file is [`ManifestError::NotFound`]; any other
/// read failure is [`ManifestError::Io`]; a malformed manifest is
/// [`ManifestError::Parse`]. On success the returned [`PackManifest`] has
/// passed [`PackManifest::validate`], and its `policies` preserve manifest
/// order.
pub fn load_manifest(path: &Path) -> Result<PackManifest, ManifestError> {
    let content = std::fs::read_to_string(path).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            ManifestError::NotFound(path.to_path_buf())
        } else {
            ManifestError::Io {
                path: path.to_path_buf(),
                source,
            }
        }
    })?;

    let manifest: PackManifest =
        serde_yaml::from_str(&content).map_err(|e| ManifestError::Parse {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;

    manifest.validate()?;
    Ok(manifest)
}

impl PackManifest {
    /// Validate pack-level fields, every member's metadata, member id
    /// uniqueness across the pack, and that member paths stay within the
    /// manifest directory.
    ///
    /// This is called by [`load_manifest`]; it is exposed so a manifest built
    /// in memory can be validated without a round-trip through the filesystem.
    pub fn validate(&self) -> Result<(), ManifestError> {
        for (field, value) in [
            ("id", self.id.as_str()),
            ("name", self.name.as_str()),
            ("version", self.version.as_str()),
            ("description", self.description.as_str()),
            ("owner", self.owner.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ManifestError::MissingField { field });
            }
        }

        for entry in &self.policies {
            entry.metadata.validate()?;
            confirm_within_manifest_dir(entry)?;
        }

        let metadata: Vec<PolicyMetadata> =
            self.policies.iter().map(|e| e.metadata.clone()).collect();
        ensure_unique_ids(&metadata)?;

        Ok(())
    }
}

/// Confirm a member path is relative and contains no `..` segment, so it can
/// only refer to a file within the manifest's own directory tree. This is a
/// purely lexical check — the filesystem is never touched.
fn confirm_within_manifest_dir(entry: &PolicyEntry) -> Result<(), ManifestError> {
    let escapes = entry.path.is_absolute()
        || entry
            .path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)));
    if escapes {
        return Err(ManifestError::PathEscapesManifest {
            policy_id: entry.metadata.id.clone(),
            path: entry.path.clone(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const VALID_MANIFEST: &str = r"
id: baseline-pack
name: Baseline Security Pack
version: 1.0.0
description: Core architectural guardrails shipped with Anvil.
owner: platform-security
policies:
  - path: policies/no-network-imports.rego
    metadata:
      id: no-network-imports
      title: Disallow new network imports
      severity: high
      owner: platform-security
      rationale: New network edges widen the blast radius of a breach.
      scope: src/**/*.rs
      tags: [security, imports]
  - path: policies/require-tests.rego
    metadata:
      id: require-tests
      title: Require tests for new modules
      severity: medium
      owner: dx
      rationale: Untested modules regress silently.
      scope: crates/**
      tags: [quality]
";

    /// Write `body` to a `pack.yaml` inside a fresh temp dir and return both so
    /// the dir lives for the test's duration.
    fn write_manifest(body: &str) -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("pack.yaml");
        std::fs::write(&path, body).expect("write manifest");
        (dir, path)
    }

    #[test]
    fn policy_pack_manifest_valid_fixture_loads() {
        let (_dir, path) = write_manifest(VALID_MANIFEST);
        let manifest = load_manifest(&path).expect("valid manifest loads");
        assert_eq!(manifest.id, "baseline-pack");
        assert_eq!(manifest.policies.len(), 2);
    }

    #[test]
    fn policy_pack_manifest_preserves_member_order() {
        let (_dir, path) = write_manifest(VALID_MANIFEST);
        let manifest = load_manifest(&path).expect("loads");
        let ids: Vec<&str> = manifest
            .policies
            .iter()
            .map(|p| p.metadata.id.as_str())
            .collect();
        assert_eq!(ids, ["no-network-imports", "require-tests"]);
    }

    #[test]
    fn policy_pack_manifest_missing_file_is_not_found() {
        let dir = TempDir::new().expect("temp dir");
        let missing = dir.path().join("absent.yaml");
        match load_manifest(&missing) {
            Err(ManifestError::NotFound(_)) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn policy_pack_manifest_unknown_root_field_is_rejected() {
        // A future/older skew: an unknown top-level key must fail closed, not be
        // silently ignored, so no policy entry is dropped unnoticed.
        let body = format!("{VALID_MANIFEST}surprise: value\n");
        let (_dir, path) = write_manifest(&body);
        match load_manifest(&path) {
            Err(ManifestError::Parse { .. }) => {}
            other => panic!("expected Parse for unknown field, got {other:?}"),
        }
    }

    #[test]
    fn policy_pack_manifest_unknown_member_field_is_rejected() {
        let body = r"
id: p
name: n
version: 1.0.0
description: d
owner: o
policies:
  - path: a.rego
    surprise: value
    metadata:
      id: a
      title: t
      severity: low
      owner: o
      rationale: r
      scope: s
      tags: [x]
";
        let (_dir, path) = write_manifest(body);
        match load_manifest(&path) {
            Err(ManifestError::Parse { .. }) => {}
            other => panic!("expected Parse for unknown member field, got {other:?}"),
        }
    }

    #[test]
    fn policy_pack_manifest_bad_severity_is_parse_error() {
        let body = VALID_MANIFEST.replace("severity: high", "severity: apocalyptic");
        let (_dir, path) = write_manifest(&body);
        match load_manifest(&path) {
            Err(ManifestError::Parse { .. }) => {}
            other => panic!("expected Parse for bad severity, got {other:?}"),
        }
    }

    #[test]
    fn policy_pack_manifest_duplicate_policy_ids_rejected() {
        let body = VALID_MANIFEST.replace("id: require-tests", "id: no-network-imports");
        let (_dir, path) = write_manifest(&body);
        match load_manifest(&path) {
            Err(ManifestError::Metadata(MetadataError::DuplicateId(id))) => {
                assert_eq!(id, "no-network-imports");
            }
            other => panic!("expected DuplicateId, got {other:?}"),
        }
    }

    #[test]
    fn policy_pack_manifest_missing_pack_field_reported() {
        let body = VALID_MANIFEST.replace("owner: platform-security\n", "owner: \"\"\n");
        let (_dir, path) = write_manifest(&body);
        match load_manifest(&path) {
            Err(ManifestError::MissingField { field: "owner" }) => {}
            other => panic!("expected MissingField owner, got {other:?}"),
        }
    }

    #[test]
    fn policy_pack_manifest_member_metadata_is_validated() {
        // A member with blank rationale must be rejected via metadata validation.
        let body = VALID_MANIFEST.replace(
            "rationale: New network edges widen the blast radius of a breach.",
            "rationale: \"\"",
        );
        let (_dir, path) = write_manifest(&body);
        match load_manifest(&path) {
            Err(ManifestError::Metadata(MetadataError::MissingField { policy_id, field })) => {
                assert_eq!(policy_id, "no-network-imports");
                assert_eq!(field, "rationale");
            }
            other => panic!("expected member metadata MissingField, got {other:?}"),
        }
    }

    #[test]
    fn policy_pack_manifest_absolute_member_path_rejected() {
        let body = VALID_MANIFEST.replace(
            "path: policies/no-network-imports.rego",
            "path: /etc/passwd.rego",
        );
        let (_dir, path) = write_manifest(&body);
        match load_manifest(&path) {
            Err(ManifestError::PathEscapesManifest { policy_id, .. }) => {
                assert_eq!(policy_id, "no-network-imports");
            }
            other => panic!("expected PathEscapesManifest, got {other:?}"),
        }
    }

    #[test]
    fn policy_pack_manifest_parent_dir_member_path_rejected() {
        let body = VALID_MANIFEST.replace(
            "path: policies/no-network-imports.rego",
            "path: ../../secrets/leak.rego",
        );
        let (_dir, path) = write_manifest(&body);
        match load_manifest(&path) {
            Err(ManifestError::PathEscapesManifest { .. }) => {}
            other => panic!("expected PathEscapesManifest for `..`, got {other:?}"),
        }
    }

    #[test]
    fn policy_pack_manifest_in_memory_validate_matches_loader() {
        let manifest: PackManifest =
            serde_yaml::from_str(VALID_MANIFEST).expect("parse in-memory manifest");
        assert!(manifest.validate().is_ok());
    }
}
