//! Per-pack member overlay (CPACKS-011).
//!
//! Selection of which pack members evaluate lives **beside** the pack directory,
//! not inside it, so `anvil policy install --force` can rewrite pack files
//! without clobbering the operator's on/off choices.
//!
//! File: `.anvil/policies/<pack-id>.overlay.yaml`
//!
//! An absent overlay means every member is enabled. Unknown `disabled` ids are
//! ignored at evaluation time (fail-open). Parse failures also fail open so a
//! malformed overlay cannot take the policy pass down.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::pack::manifest::{PackManifest, PolicyEntry, load_manifest};

/// Overlay schema version written by this crate.
const SCHEMA_VERSION: u32 = 1;

/// Member-selection overlay for one installed pack.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackOverlay {
    /// Schema version. Absent on older files deserialises as 1.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    /// Member metadata ids that must not evaluate. Empty means all on.
    #[serde(default)]
    pub disabled: Vec<String>,
}

fn default_schema_version() -> u32 {
    SCHEMA_VERSION
}

/// Why an overlay could not be loaded or saved. Evaluation callers treat load
/// errors as "all members enabled" rather than failing the pack.
#[derive(Debug, Error)]
pub enum OverlayError {
    /// The overlay file could not be read (other than not-found).
    #[error("could not read pack overlay {path}: {source}")]
    Io {
        /// Overlay path.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The overlay file is not valid YAML for this schema.
    #[error("could not parse pack overlay {path}: {message}")]
    Parse {
        /// Overlay path.
        path: PathBuf,
        /// Parser message.
        message: String,
    },
    /// `pack_id` is not a single directory name (path separators, `..`, or
    /// absolute). Overlay files must stay beside `.anvil/policies/<id>/`.
    #[error(
        "pack id `{pack_id}` is not a safe directory name; use a single path component with no `/` or `..`"
    )]
    InvalidPackId {
        /// The rejected pack id.
        pack_id: String,
    },
}

/// Whether `pack_id` is a single, non-escaping directory name.
#[must_use]
pub fn is_safe_pack_id(pack_id: &str) -> bool {
    if pack_id.is_empty()
        || pack_id == "."
        || pack_id == ".."
        || pack_id.contains('/')
        || pack_id.contains('\\')
        || pack_id.contains('\0')
    {
        return false;
    }
    let path = Path::new(pack_id);
    let mut components = path.components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

/// Path of the overlay file that sits beside `<policies_dir>/<pack_id>/`.
pub fn overlay_path(policies_dir: &Path, pack_id: &str) -> Result<PathBuf, OverlayError> {
    if !is_safe_pack_id(pack_id) {
        return Err(OverlayError::InvalidPackId {
            pack_id: pack_id.to_string(),
        });
    }
    Ok(policies_dir.join(format!("{pack_id}.overlay.yaml")))
}

/// Load the overlay for `pack_id`. A missing file is an empty overlay (all
/// members enabled), not an error.
pub fn load_overlay(policies_dir: &Path, pack_id: &str) -> Result<PackOverlay, OverlayError> {
    let path = overlay_path(policies_dir, pack_id)?;
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PackOverlay::default());
        }
        Err(source) => {
            return Err(OverlayError::Io { path, source });
        }
    };
    let mut overlay: PackOverlay =
        serde_yaml::from_str(&content).map_err(|e| OverlayError::Parse {
            path,
            message: e.to_string(),
        })?;
    overlay.normalise();
    Ok(overlay)
}

/// Best-effort load: parse errors and I/O failures other than not-found become
/// an empty overlay so evaluation stays fail-open.
#[must_use]
pub fn load_overlay_fail_open(policies_dir: &Path, pack_id: &str) -> PackOverlay {
    load_overlay(policies_dir, pack_id).unwrap_or_default()
}

/// Persist `overlay` beside the pack. Creates the policies directory if needed.
pub fn save_overlay(
    policies_dir: &Path,
    pack_id: &str,
    overlay: &PackOverlay,
) -> Result<(), OverlayError> {
    let path = overlay_path(policies_dir, pack_id)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| OverlayError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let mut to_write = overlay.clone();
    to_write.normalise();
    if to_write.schema_version == 0 {
        to_write.schema_version = SCHEMA_VERSION;
    }
    let body = serde_yaml::to_string(&to_write).map_err(|e| OverlayError::Parse {
        path: path.clone(),
        message: e.to_string(),
    })?;
    std::fs::write(&path, body).map_err(|source| OverlayError::Io { path, source })
}

impl PackOverlay {
    /// Deduplicate and sort disabled ids; drop blanks.
    pub fn normalise(&mut self) {
        let set: BTreeSet<String> = self
            .disabled
            .drain(..)
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty())
            .collect();
        self.disabled = set.into_iter().collect();
        if self.schema_version == 0 {
            self.schema_version = SCHEMA_VERSION;
        }
    }

    /// Whether `member_id` should evaluate. Unknown ids are treated as enabled.
    #[must_use]
    pub fn is_enabled(&self, member_id: &str) -> bool {
        !self.disabled.iter().any(|id| id == member_id)
    }

    /// Disable `member_id` (idempotent).
    pub fn disable(&mut self, member_id: &str) {
        let id = member_id.trim();
        if id.is_empty() {
            return;
        }
        if !self.disabled.iter().any(|existing| existing == id) {
            self.disabled.push(id.to_string());
        }
        self.normalise();
    }

    /// Re-enable `member_id` (idempotent).
    pub fn enable(&mut self, member_id: &str) {
        self.disabled.retain(|id| id != member_id);
    }
}

/// Manifest members that the overlay leaves enabled, in manifest order.
#[must_use]
pub fn enabled_entries<'a>(
    manifest: &'a PackManifest,
    overlay: &PackOverlay,
) -> Vec<&'a PolicyEntry> {
    manifest
        .policies
        .iter()
        .filter(|entry| overlay.is_enabled(&entry.metadata.id))
        .collect()
}

/// Map a `data.anvil.policies` JSON key (Rego package last segment) onto the
/// pack member metadata id. Falls back to the key itself when nothing matches.
#[must_use]
pub fn resolve_member_id(manifest: &PackManifest, json_key: &str) -> String {
    for entry in &manifest.policies {
        if entry.metadata.id == json_key {
            return entry.metadata.id.clone();
        }
        if entry.metadata.id.replace('-', "_") == json_key {
            return entry.metadata.id.clone();
        }
        if entry
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|stem| stem == json_key)
        {
            return entry.metadata.id.clone();
        }
    }
    json_key.to_string()
}

/// Whether `policy_file` should evaluate given overlays under `policies_dir`.
/// Loose `.rego` files (not inside a pack directory) always evaluate. Load
/// failures fail open (evaluate).
#[must_use]
pub fn policy_file_is_enabled(policies_dir: &Path, policy_file: &Path) -> bool {
    let Ok(relative) = policy_file.strip_prefix(policies_dir) else {
        return true;
    };
    let mut components = relative.components();
    let Some(first) = components.next() else {
        return true;
    };
    // A loose file is a single component (`foo.rego`). A pack member has at
    // least `pack-id/policies/member.rego`.
    if components.next().is_none() {
        return true;
    }
    let pack_id = first.as_os_str().to_string_lossy();
    let pack_dir = policies_dir.join(pack_id.as_ref());
    let overlay = load_overlay_fail_open(policies_dir, pack_id.as_ref());
    let Ok(manifest) = load_manifest(&pack_dir.join("pack.yaml")) else {
        return true;
    };
    let Ok(member_rel) = policy_file.strip_prefix(&pack_dir) else {
        return true;
    };
    match manifest
        .policies
        .iter()
        .find(|entry| entry.path == member_rel)
    {
        Some(entry) => overlay.is_enabled(&entry.metadata.id),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn policies_dir() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().expect("tmp");
        let dir = tmp.path().join(".anvil/policies");
        std::fs::create_dir_all(&dir).expect("policies dir");
        (tmp, dir)
    }

    #[test]
    fn overlay_missing_file_is_all_enabled() {
        let (_tmp, dir) = policies_dir();
        let overlay = load_overlay(&dir, "anvil-control-examples").expect("load");
        assert!(overlay.disabled.is_empty());
        assert!(overlay.is_enabled("crypto-human-signoff"));
    }

    #[test]
    fn overlay_round_trip_disables_named_member() {
        let (_tmp, dir) = policies_dir();
        let mut overlay = PackOverlay::default();
        overlay.disable("personal-data-paths");
        overlay.disable("personal-data-paths");
        save_overlay(&dir, "anvil-control-examples", &overlay).expect("save");
        let loaded = load_overlay(&dir, "anvil-control-examples").expect("reload");
        assert!(!loaded.is_enabled("personal-data-paths"));
        assert!(loaded.is_enabled("crypto-human-signoff"));
        // Reinstall-shaped write: overlay sits beside the pack, not inside it.
        assert!(dir.join("anvil-control-examples.overlay.yaml").is_file());
        assert!(
            !dir.join("anvil-control-examples")
                .join("overlay.yaml")
                .exists()
        );
    }

    #[test]
    fn overlay_enable_clears_disabled_id() {
        let mut overlay = PackOverlay::default();
        overlay.disable("crypto-human-signoff");
        overlay.enable("crypto-human-signoff");
        assert!(overlay.is_enabled("crypto-human-signoff"));
        assert!(overlay.disabled.is_empty());
    }

    #[test]
    fn overlay_malformed_yaml_is_an_error_on_strict_load() {
        let (_tmp, dir) = policies_dir();
        let path = overlay_path(&dir, "p").expect("safe id");
        std::fs::write(&path, "disabled: [").expect("write");
        assert!(matches!(
            load_overlay(&dir, "p"),
            Err(OverlayError::Parse { .. })
        ));
        let open = load_overlay_fail_open(&dir, "p");
        assert!(open.disabled.is_empty());
    }

    #[test]
    fn resolve_member_id_maps_package_key_to_metadata_id() {
        let manifest: PackManifest = serde_yaml::from_str(
            "id: demo\n\
             name: Demo\n\
             version: 1.0.0\n\
             description: d\n\
             owner: o\n\
             policies:\n\
             \x20 - path: policies/crypto_human_signoff.rego\n\
             \x20   metadata:\n\
             \x20     id: crypto-human-signoff\n\
             \x20     title: t\n\
             \x20     severity: high\n\
             \x20     owner: o\n\
             \x20     rationale: r\n\
             \x20     scope: diff.changed_files\n\
             \x20     tags: [t]\n",
        )
        .expect("manifest");
        assert_eq!(
            resolve_member_id(&manifest, "crypto_human_signoff"),
            "crypto-human-signoff"
        );
        assert_eq!(
            resolve_member_id(&manifest, "crypto-human-signoff"),
            "crypto-human-signoff"
        );
    }

    #[test]
    fn overlay_rejects_escaping_pack_ids() {
        let (_tmp, dir) = policies_dir();
        for id in ["../escape", "/tmp/x", "a/b", "..", "", r"..\win"] {
            assert!(!is_safe_pack_id(id), "{id} must not be a safe pack id");
            assert!(matches!(
                overlay_path(&dir, id),
                Err(OverlayError::InvalidPackId { .. })
            ));
            assert!(load_overlay(&dir, id).is_err());
        }
        assert!(is_safe_pack_id("anvil-control-examples"));
        assert!(overlay_path(&dir, "anvil-control-examples").is_ok());
    }
}
