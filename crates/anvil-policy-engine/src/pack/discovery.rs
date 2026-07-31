//! Policy pack discovery (OPAE-002).
//!
//! Locates user and bundled packs and enumerates available policies for load.

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::pack::manifest::{ManifestError, PackManifest, load_manifest};

/// Workspace-relative directory that holds installed policy packs.
const POLICIES_SUBDIR: &str = ".anvil/policies";

/// Canonical manifest filename that marks a directory as a pack.
const MANIFEST_FILENAME: &str = "pack.yaml";

/// Provenance record written beside an installed pack (OPAE-004).
const PROVENANCE_FILENAME: &str = "provenance.yaml";

/// A discovered policy pack: its directory and where its manifest lives, without
/// having loaded or validated the manifest.
///
/// A [`PackRef`] means "a directory under `.anvil/policies/` carries a
/// `pack.yaml`", nothing more — discovery is not admission. A pack whose
/// `pack.yaml` fails to load is still returned in this form by
/// [`discover_packs`]; the load error is only surfaced by [`discover_and_load`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackRef {
    /// The pack id: the pack directory's own name. (The manifest may declare its
    /// own `id`; reconciling the two is admission's job, not discovery's.)
    pub id: String,
    /// The pack directory, under `.anvil/policies/`.
    pub dir: PathBuf,
    /// The manifest path (`<dir>/pack.yaml`), known to exist as a file.
    pub manifest_path: PathBuf,
    /// Whether an OPAE-004 `provenance.yaml` sits beside the manifest.
    pub has_provenance: bool,
}

/// Why a directory or loose file under `.anvil/policies/` was rejected rather
/// than discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectionReason {
    /// The entry canonicalises outside the policies directory — a symlink
    /// escaping the workspace-scoped root. Fail-closed: the entry is skipped so
    /// external content is never treated as an installed pack or policy.
    ContainmentEscape,
    /// The entry could not be canonicalised for the containment check (it was
    /// removed mid-scan, or is a broken symlink). Skipped fail-closed rather
    /// than trusted.
    Unresolvable,
}

/// A rejected entry, reported so a caller can see (and surface) that a tampered
/// or unresolvable entry was skipped rather than silently dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedEntry {
    /// The offending path, under `.anvil/policies/`.
    pub path: PathBuf,
    /// Why it was rejected.
    pub reason: RejectionReason,
}

/// The outcome of scanning `.anvil/policies/`: discovered packs, loose legacy
/// policies, and rejected entries — each list deterministically ordered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackDiscovery {
    /// Discovered packs, sorted by id then directory.
    pub packs: Vec<PackRef>,
    /// Loose `*.rego` files directly under `.anvil/policies/` (pre-pack flat
    /// layout), sorted by path. Reported for caller distinction only; never
    /// opened or evaluated here.
    pub loose_policies: Vec<PathBuf>,
    /// Entries skipped fail-closed (containment escape or unresolvable), sorted
    /// by path.
    pub rejected: Vec<RejectedEntry>,
}

impl PackDiscovery {
    /// Whether nothing at all was found — no packs, no loose policies, no
    /// rejects. True for a missing or empty policies directory.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.packs.is_empty() && self.loose_policies.is_empty() && self.rejected.is_empty()
    }
}

/// A whole-discovery failure. Per-entry problems are reported in
/// [`PackDiscovery::rejected`], not here — this covers only failures that make
/// the entire scan untrustworthy.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    /// The policies directory exists but could not be read (permission denied,
    /// or an entry could not be enumerated), or the workspace root could not be
    /// canonicalised. Never folded into an empty result — a scan that cannot see
    /// the directory must fail loudly, not report "no packs".
    #[error("could not read policies directory {path}: {source}")]
    Io {
        /// The path being read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The policies directory itself canonicalises outside the workspace root (a
    /// `.anvil` symlinked out of the workspace). Nothing beneath it can be
    /// trusted, so the whole scan fails rather than reporting per-entry rejects.
    #[error(
        "policies directory {resolved} resolves outside the workspace root {root} \
         (path containment breach)"
    )]
    PoliciesDirEscapesRoot {
        /// The canonical workspace root.
        root: PathBuf,
        /// The escaping canonical policies directory.
        resolved: PathBuf,
    },
}

/// A pack paired with the result of loading its manifest, as returned by
/// [`discover_and_load`]. `manifest` carries the per-pack load error verbatim;
/// there is no short-circuit, so a broken pack never hides a good one.
#[derive(Debug)]
pub struct LoadedPack {
    /// The discovered pack.
    pub pack: PackRef,
    /// The loaded and validated manifest, or the load/validation error.
    pub manifest: Result<PackManifest, ManifestError>,
}

/// Discover policy packs under `<workspace_root>/.anvil/policies/`.
///
/// Scans exactly one directory level (see the module docs): an immediate
/// subdirectory with a `pack.yaml` is a pack; a loose `*.rego` file is a legacy
/// flat-layout policy; an entry escaping the workspace-scoped root is rejected
/// per-entry while the scan continues. A missing policies directory returns an
/// empty [`PackDiscovery`] (not an error). Results are deterministically sorted.
///
/// # Errors
///
/// Returns [`DiscoveryError::Io`] if the policies directory exists but cannot be
/// read, or [`DiscoveryError::PoliciesDirEscapesRoot`] if it resolves outside
/// the workspace root.
pub fn discover_packs(workspace_root: &Path) -> Result<PackDiscovery, DiscoveryError> {
    let policies_dir = workspace_root.join(POLICIES_SUBDIR);
    // Nothing installed yet is a normal state, not an error — but only true
    // absence qualifies: a broken `.anvil/policies` symlink is surfaced as an
    // I/O error below (fail-closed; likely tampering or misconfiguration),
    // never read as "missing".
    match std::fs::symlink_metadata(&policies_dir) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PackDiscovery::default());
        }
        Err(source) => {
            return Err(DiscoveryError::Io {
                path: policies_dir,
                source,
            });
        }
        Ok(_) => {}
    }

    // Workspace-scoped: the policies directory must resolve within the canonical
    // workspace root before anything beneath it is trusted.
    let canonical_root = workspace_root
        .canonicalize()
        .map_err(|source| DiscoveryError::Io {
            path: workspace_root.to_path_buf(),
            source,
        })?;
    let canonical_policies = policies_dir
        .canonicalize()
        .map_err(|source| DiscoveryError::Io {
            path: policies_dir.clone(),
            source,
        })?;
    if !canonical_policies.starts_with(&canonical_root) {
        return Err(DiscoveryError::PoliciesDirEscapesRoot {
            root: canonical_root,
            resolved: canonical_policies,
        });
    }

    let mut packs = Vec::new();
    let mut loose_policies = Vec::new();
    let mut rejected = Vec::new();

    let entries = std::fs::read_dir(&canonical_policies).map_err(|source| DiscoveryError::Io {
        path: canonical_policies.clone(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| DiscoveryError::Io {
            path: canonical_policies.clone(),
            source,
        })?;
        let path = entry.path();
        // `is_dir`/`is_file` follow symlinks; the containment check on the
        // canonicalised target then catches a link escaping the policies dir.
        if path.is_dir() {
            classify_dir(&canonical_policies, &path, &mut packs, &mut rejected);
        } else if path.is_file() && is_rego(&path) {
            classify_loose(
                &canonical_policies,
                &path,
                &mut loose_policies,
                &mut rejected,
            );
        } else if entry.file_type().is_ok_and(|t| t.is_symlink()) && !path.exists() {
            // A broken symlink is reported, not silently dropped — per-entry
            // fail-closed, matching the module's Unresolvable contract.
            rejected.push(RejectedEntry {
                path: path.clone(),
                reason: RejectionReason::Unresolvable,
            });
        }
        // Anything else — a non-`.rego` regular file — is ignored.
    }

    packs.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.dir.cmp(&b.dir)));
    loose_policies.sort();
    rejected.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(PackDiscovery {
        packs,
        loose_policies,
        rejected,
    })
}

/// Discover packs and load each one's manifest, deterministically keyed by pack.
///
/// Convenience over [`discover_packs`] + [`load_manifest`]: it chains the two so
/// callers get POLVAL-manifest compatibility without wiring the pipeline
/// themselves. There is no short-circuit — every discovered pack is loaded and
/// its result carried in [`LoadedPack::manifest`], so one pack whose manifest
/// fails to load does not hide the packs that loaded cleanly. The returned list
/// preserves [`discover_packs`]'s deterministic pack order.
///
/// Loose policies and rejected entries are not surfaced here; a caller that
/// needs them uses [`discover_packs`] directly.
///
/// # Errors
///
/// Propagates the whole-discovery [`DiscoveryError`] from [`discover_packs`].
/// Per-pack manifest failures are carried in each [`LoadedPack`], not returned
/// as an error.
pub fn discover_and_load(workspace_root: &Path) -> Result<Vec<LoadedPack>, DiscoveryError> {
    let discovery = discover_packs(workspace_root)?;
    Ok(discovery
        .packs
        .into_iter()
        .map(|pack| {
            let manifest = load_manifest(&pack.manifest_path);
            LoadedPack { pack, manifest }
        })
        .collect())
}

/// Classify an immediate subdirectory of the policies dir: a pack (if it holds a
/// `pack.yaml` and stays contained), a rejected escape, or ignored otherwise.
fn classify_dir(
    canonical_policies: &Path,
    dir: &Path,
    packs: &mut Vec<PackRef>,
    rejected: &mut Vec<RejectedEntry>,
) {
    // Containment first: a directory symlinked out of the policies root is
    // rejected before its contents are inspected, so an external `pack.yaml` is
    // never treated as an installed pack.
    if canonicalise_or_reject(canonical_policies, dir, rejected).is_none() {
        return;
    }
    // A pack owns its subtree; without a manifest at this level it is not a pack
    // and is not recursed into (nested packs are not a thing).
    let manifest_path = dir.join(MANIFEST_FILENAME);
    if !manifest_path.is_file() {
        return;
    }
    let Some(id) = dir.file_name().and_then(|n| n.to_str()).map(str::to_owned) else {
        rejected.push(RejectedEntry {
            path: dir.to_path_buf(),
            reason: RejectionReason::Unresolvable,
        });
        return;
    };
    let has_provenance = dir.join(PROVENANCE_FILENAME).is_file();
    packs.push(PackRef {
        id,
        dir: dir.to_path_buf(),
        manifest_path,
        has_provenance,
    });
}

/// Classify a loose `*.rego` file directly under the policies dir: reported as a
/// legacy flat policy if contained, rejected if it escapes.
fn classify_loose(
    canonical_policies: &Path,
    file: &Path,
    loose_policies: &mut Vec<PathBuf>,
    rejected: &mut Vec<RejectedEntry>,
) {
    if canonicalise_or_reject(canonical_policies, file, rejected).is_some() {
        loose_policies.push(file.to_path_buf());
    }
}

/// Canonicalise `entry` and require it to stay within `canonical_policies`,
/// pushing a [`RejectedEntry`] and returning `None` on failure or escape.
fn canonicalise_or_reject(
    canonical_policies: &Path,
    entry: &Path,
    rejected: &mut Vec<RejectedEntry>,
) -> Option<PathBuf> {
    match entry.canonicalize() {
        Ok(canonical) if canonical.starts_with(canonical_policies) => Some(canonical),
        Ok(_) => {
            rejected.push(RejectedEntry {
                path: entry.to_path_buf(),
                reason: RejectionReason::ContainmentEscape,
            });
            None
        }
        Err(_) => {
            rejected.push(RejectedEntry {
                path: entry.to_path_buf(),
                reason: RejectionReason::Unresolvable,
            });
            None
        }
    }
}

/// Whether a path names a `.rego` file (by extension).
fn is_rego(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "rego")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A minimal valid pack manifest for pack `id` (no members needed to be a
    /// discoverable, loadable pack).
    fn valid_manifest(id: &str) -> String {
        format!(
            "id: {id}\n\
             name: Pack {id}\n\
             version: 1.0.0\n\
             description: Discoverable pack {id}.\n\
             owner: platform-security\n\
             policies: []\n"
        )
    }

    /// Create `<root>/.anvil/policies/<id>/pack.yaml` with the given body and
    /// return the pack directory.
    fn write_pack(root: &Path, id: &str, manifest_body: &str) -> PathBuf {
        let dir = root.join(POLICIES_SUBDIR).join(id);
        std::fs::create_dir_all(&dir).expect("create pack dir");
        std::fs::write(dir.join(MANIFEST_FILENAME), manifest_body).expect("write manifest");
        dir
    }

    #[test]
    fn policy_pack_discovery_missing_dir_is_empty_ok() {
        // No `.anvil/` at all: nothing installed yet is a normal state.
        let ws = TempDir::new().expect("workspace");
        let discovery = discover_packs(ws.path()).expect("missing dir is Ok");
        assert!(discovery.is_empty(), "{discovery:?}");
        assert!(discovery.packs.is_empty());
    }

    #[test]
    fn policy_pack_discovery_empty_policies_dir_is_empty() {
        // The directory exists but holds nothing.
        let ws = TempDir::new().expect("workspace");
        std::fs::create_dir_all(ws.path().join(POLICIES_SUBDIR)).expect("mkdir");
        let discovery = discover_packs(ws.path()).expect("empty dir is Ok");
        assert!(discovery.is_empty(), "{discovery:?}");
    }

    #[test]
    fn policy_pack_discovery_two_packs_sorted() {
        // Write in reverse order; discovery must return them id-sorted.
        let ws = TempDir::new().expect("workspace");
        write_pack(ws.path(), "beta-pack", &valid_manifest("beta-pack"));
        write_pack(ws.path(), "alpha-pack", &valid_manifest("alpha-pack"));
        let discovery = discover_packs(ws.path()).expect("discover");
        let ids: Vec<&str> = discovery.packs.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["alpha-pack", "beta-pack"]);
        assert!(discovery.packs.iter().all(|p| p.manifest_path.is_file()));
        assert!(discovery.loose_policies.is_empty());
        assert!(discovery.rejected.is_empty());
    }

    #[test]
    fn policy_pack_discovery_reports_loose_rego() {
        // A pre-pack flat-layout policy sits directly under `.anvil/policies/`.
        let ws = TempDir::new().expect("workspace");
        write_pack(ws.path(), "a-pack", &valid_manifest("a-pack"));
        let policies = ws.path().join(POLICIES_SUBDIR);
        std::fs::write(policies.join("legacy.rego"), "package legacy").expect("write loose");
        // A non-`.rego` file is ignored entirely.
        std::fs::write(policies.join("README.md"), "notes").expect("write readme");

        let discovery = discover_packs(ws.path()).expect("discover");
        assert_eq!(discovery.packs.len(), 1);
        assert_eq!(discovery.loose_policies.len(), 1);
        assert!(
            discovery.loose_policies[0].ends_with("legacy.rego"),
            "{:?}",
            discovery.loose_policies
        );
    }

    #[test]
    fn policy_pack_discovery_nested_dirs_not_packs() {
        // A pack owns a subtree with its own nested `pack.yaml`; discovery must
        // NOT recurse and report the nested manifest as a second pack. A sibling
        // directory with no manifest is not a pack at all.
        let ws = TempDir::new().expect("workspace");
        let pack = write_pack(ws.path(), "outer", &valid_manifest("outer"));
        // A nested directory inside the pack that itself carries a pack.yaml.
        let nested = pack.join("policies").join("inner");
        std::fs::create_dir_all(&nested).expect("nested dir");
        std::fs::write(nested.join(MANIFEST_FILENAME), valid_manifest("inner"))
            .expect("nested manifest");
        // A manifest-less sibling directory directly under policies.
        std::fs::create_dir_all(ws.path().join(POLICIES_SUBDIR).join("not-a-pack"))
            .expect("bare dir");

        let discovery = discover_packs(ws.path()).expect("discover");
        let ids: Vec<&str> = discovery.packs.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["outer"], "only the top-level pack is discovered");
        assert!(discovery.rejected.is_empty());
    }

    #[test]
    fn policy_pack_discovery_provenance_flag_tracks_sibling() {
        let ws = TempDir::new().expect("workspace");
        let with = write_pack(ws.path(), "with-prov", &valid_manifest("with-prov"));
        std::fs::write(with.join(PROVENANCE_FILENAME), "pack: with-prov").expect("provenance");
        write_pack(ws.path(), "without-prov", &valid_manifest("without-prov"));

        let discovery = discover_packs(ws.path()).expect("discover");
        let with_ref = discovery
            .packs
            .iter()
            .find(|p| p.id == "with-prov")
            .unwrap();
        let without_ref = discovery
            .packs
            .iter()
            .find(|p| p.id == "without-prov")
            .unwrap();
        assert!(with_ref.has_provenance);
        assert!(!without_ref.has_provenance);
    }

    #[test]
    fn policy_pack_discovery_broken_manifest_still_listed() {
        // discovery != admission: a pack whose pack.yaml will not parse is still
        // returned as a PackRef by discover_packs.
        let ws = TempDir::new().expect("workspace");
        write_pack(ws.path(), "broken", "{ this is not valid pack yaml");
        let discovery = discover_packs(ws.path()).expect("discover");
        assert_eq!(discovery.packs.len(), 1);
        assert_eq!(discovery.packs[0].id, "broken");
    }

    #[test]
    fn policy_pack_discovery_and_load_carries_broken_manifest() {
        // Two packs, one good and one with a broken manifest. discover_and_load
        // must carry the broken pack's Err WITHOUT hiding the good pack.
        let ws = TempDir::new().expect("workspace");
        write_pack(ws.path(), "good", &valid_manifest("good"));
        write_pack(ws.path(), "busted", "id: busted\nname: n\n"); // missing required fields

        let loaded = discover_and_load(ws.path()).expect("discover_and_load");
        assert_eq!(loaded.len(), 2, "both packs are carried");

        let good = loaded.iter().find(|l| l.pack.id == "good").unwrap();
        let busted = loaded.iter().find(|l| l.pack.id == "busted").unwrap();
        assert!(
            good.manifest.is_ok(),
            "good pack loads: {:?}",
            good.manifest
        );
        assert!(busted.manifest.is_err(), "busted pack carries its error");
    }

    #[cfg(unix)]
    #[test]
    fn policy_pack_discovery_broken_policies_dir_symlink_is_io_error() {
        let root = TempDir::new().expect("root");
        std::fs::create_dir_all(root.path().join(".anvil")).expect("mkdir .anvil");
        std::os::unix::fs::symlink(
            root.path().join("nonexistent-target"),
            root.path().join(".anvil/policies"),
        )
        .expect("symlink");
        let err = discover_packs(root.path()).expect_err("broken dir symlink fails closed");
        assert!(matches!(err, DiscoveryError::Io { .. }));
    }

    #[cfg(unix)]
    #[test]
    fn policy_pack_discovery_broken_entry_symlink_is_rejected() {
        let root = TempDir::new().expect("root");
        let dir = root.path().join(".anvil/policies");
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::os::unix::fs::symlink(dir.join("gone.rego"), dir.join("dangling.rego"))
            .expect("symlink");
        let discovery = discover_packs(root.path()).expect("scan succeeds");
        assert_eq!(discovery.rejected.len(), 1);
        assert_eq!(discovery.rejected[0].reason, RejectionReason::Unresolvable);
        assert!(discovery.packs.is_empty());
    }

    // A symlinked pack directory escaping the policies root must be rejected
    // per-entry while a legitimate sibling is still discovered — one tampered
    // entry cannot hide the rest. Unix-only (symlink API): this cfg gate had
    // drifted onto the neighbouring test (which carried it twice) — caught by
    // the Cross (x86_64-pc-windows-msvc) smoke leg once the anvil-cli compile
    // fixes un-masked the test build.
    #[cfg(unix)]
    #[test]
    fn policy_pack_discovery_symlink_escape_rejected() {
        // An external directory holding a would-be pack manifest.
        let outside = TempDir::new().expect("outside");
        std::fs::write(
            outside.path().join(MANIFEST_FILENAME),
            valid_manifest("evil"),
        )
        .expect("outside manifest");

        let ws = TempDir::new().expect("workspace");
        // A legitimate contained pack.
        write_pack(ws.path(), "legit", &valid_manifest("legit"));
        // A symlink under .anvil/policies/ pointing at the external pack dir.
        let link = ws.path().join(POLICIES_SUBDIR).join("escape");
        std::os::unix::fs::symlink(outside.path(), &link).expect("symlink");

        let discovery = discover_packs(ws.path()).expect("discover continues past the escape");
        let ids: Vec<&str> = discovery.packs.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["legit"], "the contained sibling is still discovered");
        assert_eq!(discovery.rejected.len(), 1, "{:?}", discovery.rejected);
        assert_eq!(
            discovery.rejected[0].reason,
            RejectionReason::ContainmentEscape
        );
        assert!(discovery.rejected[0].path.ends_with("escape"));
    }

    // A loose `.rego` symlink escaping the policies root is rejected too.
    #[cfg(unix)]
    #[test]
    fn policy_pack_discovery_loose_symlink_escape_rejected() {
        let outside = TempDir::new().expect("outside");
        let secret = outside.path().join("secret.rego");
        std::fs::write(&secret, "package secret").expect("secret");

        let ws = TempDir::new().expect("workspace");
        std::fs::create_dir_all(ws.path().join(POLICIES_SUBDIR)).expect("mkdir");
        let link = ws.path().join(POLICIES_SUBDIR).join("linked.rego");
        std::os::unix::fs::symlink(&secret, &link).expect("symlink");

        let discovery = discover_packs(ws.path()).expect("discover");
        assert!(
            discovery.loose_policies.is_empty(),
            "escape is not reported as loose"
        );
        assert_eq!(discovery.rejected.len(), 1);
        assert_eq!(
            discovery.rejected[0].reason,
            RejectionReason::ContainmentEscape
        );
    }
}
