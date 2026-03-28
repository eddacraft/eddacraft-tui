use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Metadata describing a policy bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub policies: Vec<BundlePolicyRef>,
}

/// A reference to a policy within a bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundlePolicyRef {
    pub id: String,
    pub file: String,
    #[serde(default)]
    pub enabled: Option<bool>,
}

/// A loaded bundle with its resolved path and manifest.
#[derive(Debug, Clone, Serialize)]
pub struct Bundle {
    pub manifest: BundleManifest,
    pub path: PathBuf,
    /// Policy files that were found on disc.
    pub resolved_files: Vec<PathBuf>,
    /// Policy IDs referenced in the manifest but missing on disc.
    pub missing_files: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("bundle directory not found: {0}")]
    DirNotFound(PathBuf),
    #[error("manifest not found in bundle: {0}")]
    ManifestNotFound(PathBuf),
    #[error("I/O error reading bundle: {0}")]
    Io(#[from] std::io::Error),
    #[error("manifest parse error: {0}")]
    Parse(String),
    #[error("duplicate policy ID {id} in bundle {bundle}")]
    DuplicatePolicy { id: String, bundle: String },
}

const MANIFEST_NAME: &str = "manifest.json";
const BUNDLES_DIR: &str = ".anvil/bundles";

/// Loads a single bundle from a directory containing `manifest.json`.
pub fn load_bundle(path: &Path) -> Result<Bundle, BundleError> {
    if !path.is_dir() {
        return Err(BundleError::DirNotFound(path.to_path_buf()));
    }

    let manifest_path = path.join(MANIFEST_NAME);
    if !manifest_path.exists() {
        return Err(BundleError::ManifestNotFound(path.to_path_buf()));
    }

    let content = std::fs::read_to_string(&manifest_path)?;
    let manifest: BundleManifest =
        serde_json::from_str(&content).map_err(|e| BundleError::Parse(e.to_string()))?;

    validate_no_duplicate_ids(&manifest)?;

    let mut resolved_files = Vec::new();
    let mut missing_files = Vec::new();

    for policy_ref in &manifest.policies {
        let file_path = path.join(&policy_ref.file);
        if file_path.exists() {
            resolved_files.push(file_path);
        } else {
            missing_files.push(policy_ref.id.clone());
        }
    }

    Ok(Bundle {
        manifest,
        path: path.to_path_buf(),
        resolved_files,
        missing_files,
    })
}

/// Discovers and loads all bundles under `{workspace_root}/.anvil/bundles/`.
///
/// Each immediate subdirectory containing a `manifest.json` is treated as a bundle.
/// Directories without a manifest are silently skipped.
pub fn list_bundles(workspace_root: &Path) -> Result<Vec<Bundle>, BundleError> {
    let bundles_dir = workspace_root.join(BUNDLES_DIR);
    if !bundles_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut bundles = Vec::new();

    let entries = std::fs::read_dir(&bundles_dir)?;
    for entry in entries {
        let entry = entry?;
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }
        if entry_path.join(MANIFEST_NAME).exists() {
            match load_bundle(&entry_path) {
                Ok(bundle) => bundles.push(bundle),
                Err(BundleError::Parse(_)) => {},
                Err(e) => return Err(e),
            }
        }
    }

    bundles.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
    Ok(bundles)
}

/// Validates a bundle: all referenced files exist and no duplicate IDs.
pub fn validate_bundle(bundle: &Bundle) -> Vec<String> {
    let mut issues = Vec::new();

    if !bundle.missing_files.is_empty() {
        for id in &bundle.missing_files {
            issues.push(format!("policy {id} referenced in manifest but file not found"));
        }
    }

    issues
}

fn validate_no_duplicate_ids(manifest: &BundleManifest) -> Result<(), BundleError> {
    let mut seen = HashSet::new();
    for policy_ref in &manifest.policies {
        if !seen.insert(&policy_ref.id) {
            return Err(BundleError::DuplicatePolicy {
                id: policy_ref.id.clone(),
                bundle: manifest.name.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_manifest(dir: &Path, manifest: &BundleManifest) {
        let content = serde_json::to_string_pretty(manifest).unwrap();
        fs::write(dir.join(MANIFEST_NAME), content).unwrap();
    }

    fn sample_manifest() -> BundleManifest {
        BundleManifest {
            name: "test-bundle".into(),
            version: "1.0.0".into(),
            description: "A test bundle".into(),
            policies: vec![
                BundlePolicyRef {
                    id: "BP-001".into(),
                    file: "security.rego".into(),
                    enabled: Some(true),
                },
                BundlePolicyRef {
                    id: "BP-002".into(),
                    file: "quality.rego".into(),
                    enabled: None,
                },
            ],
        }
    }

    #[test]
    fn load_bundle_succeeds_with_valid_manifest() {
        let tmp = tempfile::TempDir::new().unwrap();
        let bundle_dir = tmp.path().join("my-bundle");
        fs::create_dir_all(&bundle_dir).unwrap();

        let manifest = sample_manifest();
        write_manifest(&bundle_dir, &manifest);

        // Create the referenced policy files
        fs::write(
            bundle_dir.join("security.rego"),
            "package test.security\n",
        )
        .unwrap();
        fs::write(bundle_dir.join("quality.rego"), "package test.quality\n").unwrap();

        let bundle = load_bundle(&bundle_dir).unwrap();
        assert_eq!(bundle.manifest.name, "test-bundle");
        assert_eq!(bundle.resolved_files.len(), 2);
        assert!(bundle.missing_files.is_empty());
    }

    #[test]
    fn load_bundle_reports_missing_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let bundle_dir = tmp.path().join("my-bundle");
        fs::create_dir_all(&bundle_dir).unwrap();

        let manifest = sample_manifest();
        write_manifest(&bundle_dir, &manifest);
        // Don't create the .rego files

        let bundle = load_bundle(&bundle_dir).unwrap();
        assert_eq!(bundle.missing_files.len(), 2);
        assert_eq!(bundle.resolved_files.len(), 0);
    }

    #[test]
    fn load_bundle_fails_for_missing_dir() {
        let result = load_bundle(Path::new("/nonexistent/bundle"));
        assert!(matches!(result, Err(BundleError::DirNotFound(_))));
    }

    #[test]
    fn load_bundle_fails_for_missing_manifest() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = load_bundle(tmp.path());
        assert!(matches!(result, Err(BundleError::ManifestNotFound(_))));
    }

    #[test]
    fn load_bundle_rejects_duplicate_ids() {
        let tmp = tempfile::TempDir::new().unwrap();
        let bundle_dir = tmp.path().join("dup-bundle");
        fs::create_dir_all(&bundle_dir).unwrap();

        let manifest = BundleManifest {
            name: "dup-bundle".into(),
            version: "1.0.0".into(),
            description: String::new(),
            policies: vec![
                BundlePolicyRef {
                    id: "BP-001".into(),
                    file: "a.rego".into(),
                    enabled: None,
                },
                BundlePolicyRef {
                    id: "BP-001".into(),
                    file: "b.rego".into(),
                    enabled: None,
                },
            ],
        };
        write_manifest(&bundle_dir, &manifest);

        let result = load_bundle(&bundle_dir);
        assert!(matches!(result, Err(BundleError::DuplicatePolicy { .. })));
    }

    #[test]
    fn list_bundles_discovers_subdirectories() {
        let tmp = tempfile::TempDir::new().unwrap();
        let bundles_root = tmp.path().join(BUNDLES_DIR);
        fs::create_dir_all(&bundles_root).unwrap();

        // Bundle A
        let a = bundles_root.join("alpha");
        fs::create_dir_all(&a).unwrap();
        write_manifest(
            &a,
            &BundleManifest {
                name: "alpha".into(),
                version: "1.0.0".into(),
                description: String::new(),
                policies: vec![],
            },
        );

        // Bundle B
        let b = bundles_root.join("beta");
        fs::create_dir_all(&b).unwrap();
        write_manifest(
            &b,
            &BundleManifest {
                name: "beta".into(),
                version: "0.1.0".into(),
                description: String::new(),
                policies: vec![],
            },
        );

        // Not a bundle (no manifest)
        let c = bundles_root.join("no-manifest");
        fs::create_dir_all(&c).unwrap();

        let bundles = list_bundles(tmp.path()).unwrap();
        assert_eq!(bundles.len(), 2);
        assert_eq!(bundles[0].manifest.name, "alpha");
        assert_eq!(bundles[1].manifest.name, "beta");
    }

    #[test]
    fn list_bundles_returns_empty_when_no_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let bundles = list_bundles(tmp.path()).unwrap();
        assert!(bundles.is_empty());
    }

    #[test]
    fn validate_bundle_reports_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let bundle_dir = tmp.path().join("val-bundle");
        fs::create_dir_all(&bundle_dir).unwrap();

        write_manifest(&bundle_dir, &sample_manifest());

        let bundle = load_bundle(&bundle_dir).unwrap();
        let issues = validate_bundle(&bundle);
        assert_eq!(issues.len(), 2);
    }

    #[test]
    fn validate_bundle_clean_when_all_present() {
        let tmp = tempfile::TempDir::new().unwrap();
        let bundle_dir = tmp.path().join("ok-bundle");
        fs::create_dir_all(&bundle_dir).unwrap();

        write_manifest(&bundle_dir, &sample_manifest());
        fs::write(bundle_dir.join("security.rego"), "package s\n").unwrap();
        fs::write(bundle_dir.join("quality.rego"), "package q\n").unwrap();

        let bundle = load_bundle(&bundle_dir).unwrap();
        let issues = validate_bundle(&bundle);
        assert!(issues.is_empty());
    }
}
