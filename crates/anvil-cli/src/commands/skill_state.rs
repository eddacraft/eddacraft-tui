//! Soft evaluation of managed skill install state (SKPKG freshness / doctor).
//!
//! Pure inspection only: never writes. Install refusal behaviour lives in
//! [`super::skill`]; this module owns the shared manifest schema, expected
//! bundle identity, and soft outcomes for `anvil doctor`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::activation::agent_registry::{AgentClientId, InstallScope};
use crate::activation::detect_agents::RealDetectionEnv;

pub const MANIFEST_NAME: &str = ".anvil-managed.json";
pub const DEFAULT_SKILL_NAME: &str = "anvil-developer-functions";

const SOURCE_COMMIT: &str = "ef5b34c5f424c9de4292406405e4bedfb603a65a";

pub(crate) const SKILL_MD: &str =
    include_str!("../../assets/skills/anvil-developer-functions/SKILL.md");
pub(crate) const TOOL_REFERENCE: &str =
    include_str!("../../assets/skills/anvil-developer-functions/references/tool-reference.md");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedManifest {
    pub schema_version: u32,
    pub skill: String,
    pub source_commit: String,
    pub anvil_version: String,
    pub bundle_digest: String,
    pub files: BTreeMap<String, String>,
}

/// Soft install outcome for doctor / inventory. Never blocks writes by itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillInstallOutcome {
    Fresh,
    Stale {
        installed_anvil: String,
        current_anvil: String,
    },
    Dirty,
    Unmanaged,
    Absent,
    Broken {
        reason: String,
    },
}

/// One evaluated install site for the default (or a known) skill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillInstallReport {
    pub path: PathBuf,
    pub clients: Vec<&'static str>,
    pub outcome: SkillInstallOutcome,
}

/// Build the managed manifest for the embedded `anvil-developer-functions` bundle.
#[must_use]
pub fn expected_developer_functions_manifest() -> ManagedManifest {
    let files = BTreeMap::from([
        ("SKILL.md".to_string(), sha256(SKILL_MD.as_bytes())),
        (
            "references/tool-reference.md".to_string(),
            sha256(TOOL_REFERENCE.as_bytes()),
        ),
    ]);
    ManagedManifest {
        schema_version: 1,
        skill: DEFAULT_SKILL_NAME.to_string(),
        source_commit: SOURCE_COMMIT.to_string(),
        anvil_version: env!("CARGO_PKG_VERSION").to_string(),
        bundle_digest: bundle_digest(&files),
        files,
    }
}

/// Pure evaluation of a skill destination against an expected managed manifest.
///
/// Never writes. Prefer [`SkillInstallOutcome::Dirty`] for content drift and
/// [`SkillInstallOutcome::Broken`] for a corrupt or unusable marker.
#[must_use]
pub fn evaluate_install(destination: &Path, expected: &ManagedManifest) -> SkillInstallOutcome {
    if !destination.exists() {
        return SkillInstallOutcome::Absent;
    }
    if !destination.is_dir() {
        return SkillInstallOutcome::Broken {
            // CIB-287: doctor interpolates this reason into the broken: row.
            reason: format!(
                "skill path {} exists but is not a directory",
                crate::display_path::shown(destination)
            ),
        };
    }

    let manifest_path = destination.join(MANIFEST_NAME);
    if !manifest_path.exists() {
        return SkillInstallOutcome::Unmanaged;
    }

    let raw = match fs::read_to_string(&manifest_path) {
        Ok(raw) => raw,
        Err(error) => {
            return SkillInstallOutcome::Broken {
                reason: format!(
                    "could not read {}: {error}",
                    crate::display_path::shown(&manifest_path)
                ),
            };
        }
    };

    let manifest: ManagedManifest = match serde_json::from_str(&raw) {
        Ok(manifest) => manifest,
        Err(error) => {
            return SkillInstallOutcome::Broken {
                reason: format!(
                    "invalid managed manifest {}: {error}",
                    crate::display_path::shown(&manifest_path)
                ),
            };
        }
    };

    if manifest.schema_version != 1 || manifest.skill != expected.skill {
        return SkillInstallOutcome::Broken {
            reason: format!(
                "managed manifest {} has unsupported schema or skill identity",
                crate::display_path::shown(&manifest_path)
            ),
        };
    }

    for relative in manifest.files.keys() {
        if !is_safe_relative_path(relative) {
            return SkillInstallOutcome::Broken {
                reason: format!("managed manifest contains unsafe relative path `{relative}`"),
            };
        }
    }

    if bundle_digest(&manifest.files) != manifest.bundle_digest {
        return SkillInstallOutcome::Broken {
            reason: format!(
                "managed manifest {} has an invalid bundle digest",
                crate::display_path::shown(&manifest_path)
            ),
        };
    }

    match files_match_manifest(destination, &manifest.files) {
        FileMatch::Ok => {}
        FileMatch::Drift => return SkillInstallOutcome::Dirty,
    }

    if has_unmanaged_entries(destination, &manifest.files) {
        return SkillInstallOutcome::Dirty;
    }

    if &manifest == expected {
        SkillInstallOutcome::Fresh
    } else {
        SkillInstallOutcome::Stale {
            installed_anvil: manifest.anvil_version,
            current_anvil: expected.anvil_version.clone(),
        }
    }
}

/// Discover candidate install paths for `skill_name` under home (Global) and
/// project (Project) roots using the agent registry skill roots.
///
/// Does not require client detection — every skill-capable registry root is
/// scanned when the corresponding root is provided. Paths are de-duplicated
/// with attached client labels.
#[must_use]
pub fn discover_skill_paths(
    home: Option<&Path>,
    project: Option<&Path>,
    skill_name: &str,
) -> Vec<(PathBuf, Vec<&'static str>)> {
    let mut destinations: BTreeMap<PathBuf, BTreeSet<&'static str>> = BTreeMap::new();

    for (scope, root) in [
        (InstallScope::Global, home),
        (InstallScope::Project, project),
    ] {
        let Some(root) = root else {
            continue;
        };
        for entry in AgentClientId::all() {
            let Some(skill_root) = entry.skill_root(scope, root) else {
                continue;
            };
            destinations
                .entry(skill_root.join(skill_name))
                .or_default()
                .insert(entry.label());
        }
    }

    destinations
        .into_iter()
        .map(|(path, clients)| (path, clients.into_iter().collect()))
        .collect()
}

/// Evaluate the default managed skill at discovered paths that exist, plus
/// paths for strongly detected skill-capable clients (so Absent is only
/// reported when a client in that scope would care).
#[must_use]
pub fn evaluate_known_skills(
    home: Option<&Path>,
    project: Option<&Path>,
) -> Vec<SkillInstallReport> {
    let env = RealDetectionEnv;
    let expected = expected_developer_functions_manifest();
    let mut interested: BTreeSet<PathBuf> = BTreeSet::new();

    for (scope, root) in [
        (InstallScope::Global, home),
        (InstallScope::Project, project),
    ] {
        let Some(root) = root else {
            continue;
        };
        for entry in AgentClientId::all() {
            if !entry.supports_skill(scope) || !entry.detected(&env) {
                continue;
            }
            if let Some(skill_root) = entry.skill_root(scope, root) {
                interested.insert(skill_root.join(DEFAULT_SKILL_NAME));
            }
        }
    }

    let mut reports = Vec::new();
    for (path, clients) in discover_skill_paths(home, project, DEFAULT_SKILL_NAME) {
        let exists = path.exists();
        if !exists && !interested.contains(&path) {
            continue;
        }
        let outcome = evaluate_install(&path, &expected);
        reports.push(SkillInstallReport {
            path,
            clients,
            outcome,
        });
    }
    reports
}

/// Whether any skill-capable agent client is strongly detected on this machine.
#[must_use]
pub fn any_skill_capable_client_detected() -> bool {
    let env = RealDetectionEnv;
    AgentClientId::all().iter().any(|entry| {
        (entry.supports_skill(InstallScope::Global) || entry.supports_skill(InstallScope::Project))
            && entry.detected(&env)
    })
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub(crate) fn bundle_digest(files: &BTreeMap<String, String>) -> String {
    let mut digest = Sha256::new();
    for (relative, file_digest) in files {
        digest.update(relative.as_bytes());
        digest.update([0]);
        digest.update(file_digest.as_bytes());
        digest.update([0]);
    }
    hex::encode(digest.finalize())
}

pub(crate) fn is_safe_relative_path(relative: &str) -> bool {
    let relative_path = Path::new(relative);
    !relative_path.as_os_str().is_empty()
        && relative_path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

enum FileMatch {
    Ok,
    Drift,
}

fn files_match_manifest(destination: &Path, files: &BTreeMap<String, String>) -> FileMatch {
    for (relative, expected_hash) in files {
        let path = crate::display_path::join_relative(destination, relative);
        // `symlink_metadata` (unlike `metadata`) does not follow a symlink,
        // so a symlink standing in for a managed file is flagged as drift
        // without ever reading whatever it points at.
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            return FileMatch::Drift;
        };
        if !metadata.is_file() {
            return FileMatch::Drift;
        }
        let Ok(bytes) = fs::read(&path) else {
            return FileMatch::Drift;
        };
        if sha256(&bytes) != *expected_hash {
            return FileMatch::Drift;
        }
    }
    FileMatch::Ok
}

fn has_unmanaged_entries(destination: &Path, managed_files: &BTreeMap<String, String>) -> bool {
    let mut allowed_files = managed_files
        .keys()
        .map(PathBuf::from)
        .collect::<BTreeSet<_>>();
    allowed_files.insert(PathBuf::from(MANIFEST_NAME));

    let mut allowed_directories = BTreeSet::new();
    for file in &allowed_files {
        let mut parent = file.parent();
        while let Some(path) = parent {
            if path.as_os_str().is_empty() {
                break;
            }
            allowed_directories.insert(path.to_path_buf());
            parent = path.parent();
        }
    }

    let mut directories = vec![destination.to_path_buf()];
    while let Some(directory) = directories.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            // Unreadable managed tree is treated as dirty drift rather than a
            // corrupt marker (the marker itself already parsed cleanly).
            return true;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(relative) = path.strip_prefix(destination) else {
                return true;
            };
            let Ok(metadata) = fs::symlink_metadata(&path) else {
                return true;
            };

            let allowed = if metadata.file_type().is_symlink() {
                false
            } else if metadata.is_dir() {
                if allowed_directories.contains(relative) {
                    directories.push(path);
                    true
                } else {
                    false
                }
            } else if metadata.is_file() {
                allowed_files.contains(relative)
            } else {
                false
            };

            if !allowed {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_managed_install(destination: &Path, manifest: &ManagedManifest) {
        fs::create_dir_all(destination.join("references")).unwrap();
        fs::write(destination.join("SKILL.md"), SKILL_MD).unwrap();
        fs::write(
            destination.join("references/tool-reference.md"),
            TOOL_REFERENCE,
        )
        .unwrap();
        let body = format!("{}\n", serde_json::to_string_pretty(manifest).unwrap());
        fs::write(destination.join(MANIFEST_NAME), body).unwrap();
    }

    #[test]
    fn evaluate_absent_when_path_missing() {
        let root = tempfile::tempdir().unwrap();
        let expected = expected_developer_functions_manifest();
        let outcome = evaluate_install(&root.path().join("missing"), &expected);
        assert_eq!(outcome, SkillInstallOutcome::Absent);
    }

    #[test]
    fn evaluate_unmanaged_when_directory_has_no_manifest() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join(DEFAULT_SKILL_NAME);
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("SKILL.md"), "user-owned").unwrap();
        let expected = expected_developer_functions_manifest();
        assert_eq!(
            evaluate_install(&destination, &expected),
            SkillInstallOutcome::Unmanaged
        );
    }

    #[test]
    fn evaluate_fresh_when_manifest_and_files_match_expected() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join(DEFAULT_SKILL_NAME);
        let expected = expected_developer_functions_manifest();
        write_managed_install(&destination, &expected);
        assert_eq!(
            evaluate_install(&destination, &expected),
            SkillInstallOutcome::Fresh
        );
    }

    #[test]
    fn evaluate_stale_when_hashes_match_but_manifest_differs_from_expected() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join(DEFAULT_SKILL_NAME);
        let expected = expected_developer_functions_manifest();
        // Files match the expected content hashes; only identity fields drift.
        let mut stale = expected.clone();
        stale.anvil_version = "0.0.0-stale".to_string();
        stale.source_commit = "0".repeat(40);
        // bundle_digest stays aligned with files so integrity still holds.
        write_managed_install(&destination, &stale);

        match evaluate_install(&destination, &expected) {
            SkillInstallOutcome::Stale {
                installed_anvil,
                current_anvil,
            } => {
                assert_eq!(installed_anvil, "0.0.0-stale");
                assert_eq!(current_anvil, expected.anvil_version);
            }
            other => panic!("expected Stale, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_dirty_when_managed_file_hash_mismatches() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join(DEFAULT_SKILL_NAME);
        let expected = expected_developer_functions_manifest();
        write_managed_install(&destination, &expected);
        fs::write(destination.join("SKILL.md"), "user modification").unwrap();
        assert_eq!(
            evaluate_install(&destination, &expected),
            SkillInstallOutcome::Dirty
        );
    }

    #[test]
    #[cfg(unix)]
    fn evaluate_dirty_when_managed_file_is_a_symlink() {
        // A symlink standing in for a managed file must be flagged as
        // drift without ever being followed and read.
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join(DEFAULT_SKILL_NAME);
        let expected = expected_developer_functions_manifest();
        write_managed_install(&destination, &expected);

        let outside = root.path().join("outside-secret.txt");
        fs::write(&outside, "not part of the skill bundle").unwrap();
        fs::remove_file(destination.join("SKILL.md")).unwrap();
        symlink(&outside, destination.join("SKILL.md")).unwrap();

        assert_eq!(
            evaluate_install(&destination, &expected),
            SkillInstallOutcome::Dirty
        );
    }

    #[test]
    fn evaluate_dirty_when_extra_user_file_present() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join(DEFAULT_SKILL_NAME);
        let expected = expected_developer_functions_manifest();
        write_managed_install(&destination, &expected);
        fs::write(destination.join("notes.md"), "user notes").unwrap();
        assert_eq!(
            evaluate_install(&destination, &expected),
            SkillInstallOutcome::Dirty
        );
    }

    #[test]
    fn evaluate_broken_when_skill_identity_mismatches() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join(DEFAULT_SKILL_NAME);
        let expected = expected_developer_functions_manifest();
        let mut broken = expected.clone();
        broken.skill = "another-skill".to_string();
        write_managed_install(&destination, &broken);
        match evaluate_install(&destination, &expected) {
            SkillInstallOutcome::Broken { reason } => {
                assert!(reason.contains("skill identity"), "{reason}");
            }
            other => panic!("expected Broken, got {other:?}"),
        }
    }

    /// CIB-287. `Broken { reason }` text is interpolated into doctor's
    /// `broken:` row, so a path that still carries `\\?\` leaks twice — once
    /// from the row's own path and once from the reason. Force a non-directory
    /// at a path whose string form starts with the verbatim prefix and assert
    /// on the **reason string**.
    ///
    /// Unix-only for the same reason as the CIB-285 install-error fixture: on
    /// Windows `\\?\C:\...` is a real verbatim path and would leave the temp
    /// root. The rendering is pure string logic, so Linux carries the signal.
    #[cfg(unix)]
    #[test]
    fn broken_reason_never_carries_a_windows_verbatim_prefix() {
        let root = tempfile::tempdir().unwrap();
        // Relative path whose *string* begins with the NT-extended prefix, so
        // `Path::display` would emit `\\?\...` and `shown` can strip it. Built
        // under a temp cwd so the fixture cannot escape the test root.
        crate::test_support::cwd::with_cwd_in(root.path(), || {
            let destination =
                PathBuf::from(r"\\?\C:\project\.agents\skills\anvil-developer-functions");
            fs::write(&destination, "not a directory").unwrap();
            let expected = expected_developer_functions_manifest();
            match evaluate_install(&destination, &expected) {
                SkillInstallOutcome::Broken { reason } => {
                    assert!(
                        !reason.contains(r"\\?\"),
                        "verbatim prefix leaked into Broken reason: {reason}"
                    );
                    assert!(
                        reason.contains(r"C:\project"),
                        "reason should still name the destination: {reason}"
                    );
                    assert!(
                        reason.contains("not a directory"),
                        "expected the non-directory branch, got: {reason}"
                    );
                }
                other => panic!("expected Broken, got {other:?}"),
            }
        });
    }

    /// CIB-287 tripwire for the five `Broken { reason }` sites. Production
    /// code only — the test module below may mention the forbidden spelling
    /// in assertion messages without being a leak.
    ///
    /// Cuts at the unique module header rather than the first `#[cfg(test)]`
    /// token, so a comment or string that happens to contain that token cannot
    /// silently shrink the scanned region (Copilot review on PR #3602).
    #[test]
    fn skill_state_renders_every_path_through_the_shared_helper() {
        // Item before any statement — clippy::items_after_statements.
        const PRODUCTION_END: &str = "#[cfg(test)]\nmod tests {";
        let source = include_str!("skill_state.rs").replace("\r\n", "\n");
        let Some((production, _)) = source.split_once(PRODUCTION_END) else {
            panic!(
                "expected skill_state.rs production code to end with {PRODUCTION_END:?}; \
                 the tripwire must fail loudly if the module layout moves"
            );
        };
        assert!(
            !production.contains(".display()"),
            "skill_state production code still formats a Path via Display, \
             which emits the Windows verbatim prefix into Broken reasons \
             that doctor prints (CIB-287). Use display_path::shown instead."
        );
        assert!(
            production.contains("display_path::shown("),
            "expected skill_state production code to render paths through the shared helper"
        );
    }

    #[test]
    fn discover_skill_paths_dedupes_shared_agents_root() {
        let home = tempfile::tempdir().unwrap();
        let paths = discover_skill_paths(Some(home.path()), None, DEFAULT_SKILL_NAME);
        let agents = home.path().join(".agents/skills").join(DEFAULT_SKILL_NAME);
        let entry = paths
            .iter()
            .find(|(path, _)| path == &agents)
            .expect("shared .agents/skills path present");
        // Codex, Gemini CLI, OpenClaw, and Copilot CLI all share this root.
        assert!(
            entry.1.len() >= 2,
            "expected shared client labels: {:?}",
            entry.1
        );
        assert!(entry.1.contains(&"codex"));
    }

    #[test]
    fn evaluate_known_skills_reports_existing_install_without_detection() {
        let project = tempfile::tempdir().unwrap();
        let destination = project
            .path()
            .join(".agents/skills")
            .join(DEFAULT_SKILL_NAME);
        let expected = expected_developer_functions_manifest();
        write_managed_install(&destination, &expected);

        let reports = evaluate_known_skills(None, Some(project.path()));
        assert!(
            reports.iter().any(|report| {
                report.path == destination && report.outcome == SkillInstallOutcome::Fresh
            }),
            "existing install must be reported even without client detection: {reports:?}"
        );
    }
}
