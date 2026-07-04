//! ATC-002 — loadable probe packs and the probe registry.
//!
//! A probe pack is a single YAML manifest ([`ProbePack`]) describing the pack
//! (id, name, version, description, owner) and its member [`Probe`]s (ATC-001
//! wire assets, embedded inline). [`load_probe_pack`] parses one manifest,
//! validates it, and returns members in declared order; [`discover_probe_packs`]
//! locates installed packs under `<workspace>/.anvil/probes/`; and
//! [`ProbeRegistry`] admits a set of packs and selects probes by
//! [`RiskProfile`].
//!
//! Constraints (mirroring the policy-engine pack module, POLVAL/OPAE):
//! - A missing manifest maps to [`ProbePackError::NotFound`]; no parse or I/O
//!   failure is ever folded into a default — every failure propagates as
//!   [`Err`] (fail-closed).
//! - Unknown fields on the *pack root* are rejected (`deny_unknown_fields`) so a
//!   mistyped authored key cannot silently drop a required value. Individual
//!   [`Probe`] entries stay forward-compatible (an unrecognised probe field is
//!   tolerated, per ATC-001): the pack schema is a small fixed authored set,
//!   whereas a probe is a wire asset that may gain fields across catalog
//!   revisions.
//! - Member ordering is the manifest's declared order (deterministic).
//! - Discovery canonicalises and contains every candidate within the
//!   workspace-scoped probes directory, rejecting a symlink that escapes it —
//!   the same containment lesson as
//!   [`crate::adversarial`]'s policy-engine sibling — while continuing the scan
//!   (one tampered entry cannot hide the rest).
//! - [`ProbeRegistry::load`] is the *admission* point and is all-or-nothing
//!   fail-closed: a rejected entry, a pack that fails to load, or a duplicate
//!   pack id fails the whole registry, because a registry that silently dropped
//!   a pack would under-select probes and quietly reduce coverage.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use anvil_kernel_types::{Probe, ProbeCategory};

/// Workspace-relative directory that holds installed probe packs.
const PROBES_SUBDIR: &str = ".anvil/probes";

/// Canonical manifest filename that marks a directory as a probe pack.
const MANIFEST_FILENAME: &str = "probes.yaml";

/// A parsed and validated probe pack manifest.
///
/// `probes` is kept in the manifest's declared order so a load is deterministic.
/// Unknown top-level fields are rejected so a newer manifest cannot be silently
/// under-read by an older build.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbePack {
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
    /// Member probes, in declared order.
    #[serde(default)]
    pub probes: Vec<Probe>,
}

/// A probe pack load or validation failure. User-facing text uses UK spelling.
#[derive(Debug, Error)]
pub enum ProbePackError {
    /// The manifest file does not exist.
    #[error("probe pack manifest not found: {0}")]
    NotFound(PathBuf),
    /// The manifest could not be read (other than not-found).
    #[error("could not read probe pack manifest {path}: {source}")]
    Io {
        /// The manifest path.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The manifest is not valid YAML for the pack schema (includes an unknown
    /// top-level field).
    #[error("could not parse probe pack manifest {path}: {message}")]
    Parse {
        /// The manifest path.
        path: PathBuf,
        /// The parser's message.
        message: String,
    },
    /// A required pack-level field is blank.
    #[error("probe pack field `{field}` is blank; set a non-blank `{field}` value")]
    MissingField {
        /// The name of the blank field.
        field: &'static str,
    },
    /// A pack declares no probes; an empty pack cannot assert anything.
    #[error("probe pack `{pack_id}` declares no probes; a pack must contain at least one probe")]
    EmptyPack {
        /// The `id` of the offending pack.
        pack_id: String,
    },
    /// A probe entry has no `id`, so nothing can be attributed to it.
    #[error("probe pack `{pack_id}` has a probe with no `id`; give every probe a non-blank id")]
    MissingProbeId {
        /// The `id` of the pack the probe belongs to.
        pack_id: String,
    },
    /// A required probe field is blank on an identified probe.
    #[error(
        "probe `{probe_id}` is missing required field `{field}`; set a non-blank `{field}` value"
    )]
    ProbeMissingField {
        /// The `id` of the offending probe.
        probe_id: String,
        /// The name of the blank field.
        field: &'static str,
    },
    /// Two probes in the same pack share an `id`.
    #[error("duplicate probe id `{0}` in pack; each probe id must be unique")]
    DuplicateProbeId(String),
    /// Two packs admitted to the same registry share an `id`.
    #[error("duplicate pack id `{0}` in registry; each pack id must be unique")]
    DuplicatePackId(String),
}

/// Load and validate a probe pack manifest from `path`.
///
/// Reads only `path`. A missing file is [`ProbePackError::NotFound`]; any other
/// read failure is [`ProbePackError::Io`]; a malformed manifest is
/// [`ProbePackError::Parse`]. On success the returned [`ProbePack`] has passed
/// [`ProbePack::validate`], and its `probes` preserve manifest order.
///
/// # Errors
///
/// Returns a [`ProbePackError`] on any read, parse, or validation failure.
pub fn load_probe_pack(path: &Path) -> Result<ProbePack, ProbePackError> {
    let content = std::fs::read_to_string(path).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            ProbePackError::NotFound(path.to_path_buf())
        } else {
            ProbePackError::Io {
                path: path.to_path_buf(),
                source,
            }
        }
    })?;

    let pack: ProbePack = serde_yaml::from_str(&content).map_err(|e| ProbePackError::Parse {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;

    pack.validate()?;
    Ok(pack)
}

impl ProbePack {
    /// Validate pack-level fields, every probe's required fields, and probe id
    /// uniqueness across the pack.
    ///
    /// Exposed so a manifest built in memory can be validated without a
    /// round-trip through the filesystem.
    ///
    /// # Errors
    ///
    /// Returns a [`ProbePackError`] describing the first field, probe, or
    /// uniqueness failure found.
    pub fn validate(&self) -> Result<(), ProbePackError> {
        for (field, value) in [
            ("id", self.id.as_str()),
            ("name", self.name.as_str()),
            ("version", self.version.as_str()),
            ("description", self.description.as_str()),
            ("owner", self.owner.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ProbePackError::MissingField { field });
            }
        }

        if self.probes.is_empty() {
            return Err(ProbePackError::EmptyPack {
                pack_id: self.id.trim().to_string(),
            });
        }

        let mut seen = BTreeSet::new();
        for probe in &self.probes {
            let probe_id = probe.id.trim();
            if probe_id.is_empty() {
                return Err(ProbePackError::MissingProbeId {
                    pack_id: self.id.trim().to_string(),
                });
            }
            if probe.version.trim().is_empty() {
                return Err(ProbePackError::ProbeMissingField {
                    probe_id: probe_id.to_string(),
                    field: "version",
                });
            }
            if probe.description.trim().is_empty() {
                return Err(ProbePackError::ProbeMissingField {
                    probe_id: probe_id.to_string(),
                    field: "description",
                });
            }
            if !seen.insert(probe_id) {
                return Err(ProbePackError::DuplicateProbeId(probe_id.to_string()));
            }
        }

        Ok(())
    }
}

/// A named selector over probe categories: which classes of adversarial
/// behaviour a caller wants exercised for a given risk profile and context.
///
/// Selection is category-based and deterministic; a profile that includes a
/// category admits every probe of that category from every admitted pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskProfile {
    /// Human-readable profile name (e.g. `"baseline"`, `"tool-heavy"`).
    pub name: String,
    /// The categories this profile selects.
    pub categories: Vec<ProbeCategory>,
}

impl RiskProfile {
    /// Build a profile from a name and the categories it selects.
    #[must_use]
    pub fn new(name: impl Into<String>, categories: Vec<ProbeCategory>) -> Self {
        Self {
            name: name.into(),
            categories,
        }
    }

    /// Whether this profile selects probes of `category`.
    #[must_use]
    pub fn includes(&self, category: ProbeCategory) -> bool {
        self.categories.contains(&category)
    }
}

/// An admitted set of probe packs, selectable by risk profile.
///
/// Packs are held sorted by id so iteration and selection are deterministic
/// regardless of insertion or discovery order.
#[derive(Debug, Clone)]
pub struct ProbeRegistry {
    packs: Vec<ProbePack>,
}

impl ProbeRegistry {
    /// Admit a set of already-loaded packs, rejecting a duplicate pack id.
    ///
    /// Packs are sorted by id for deterministic iteration. Each pack is assumed
    /// to have already passed [`ProbePack::validate`] (as [`load_probe_pack`]
    /// guarantees); this constructor adds only the cross-pack uniqueness check.
    ///
    /// # Errors
    ///
    /// Returns [`ProbePackError::DuplicatePackId`] if two packs share an `id`.
    pub fn from_packs(mut packs: Vec<ProbePack>) -> Result<Self, ProbePackError> {
        packs.sort_by(|a, b| a.id.cmp(&b.id));
        let mut seen = BTreeSet::new();
        for pack in &packs {
            if !seen.insert(pack.id.as_str()) {
                return Err(ProbePackError::DuplicatePackId(pack.id.clone()));
            }
        }
        Ok(Self { packs })
    }

    /// Discover, load, and admit every pack under `<workspace_root>/.anvil/probes/`.
    ///
    /// This is the fail-closed admission path: any rejected (containment-escaping
    /// or unresolvable) entry, any pack whose manifest fails to load, or a
    /// duplicate pack id fails the whole registry. A registry that silently
    /// dropped a pack would under-select probes and quietly reduce coverage,
    /// which is exactly what a security-probe catalog must not do. A missing
    /// probes directory yields an empty registry (nothing installed is normal).
    ///
    /// # Errors
    ///
    /// Returns a [`RegistryLoadError`] on a discovery failure, a rejected entry,
    /// a pack load failure, a pack directory name that disagrees with its
    /// manifest id, or a duplicate pack id.
    pub fn load(workspace_root: &Path) -> Result<Self, RegistryLoadError> {
        let discovery = discover_probe_packs(workspace_root)?;
        if let Some(rejected) = discovery.rejected.into_iter().next() {
            return Err(RegistryLoadError::Rejected(rejected));
        }
        let mut packs = Vec::with_capacity(discovery.packs.len());
        for pack_ref in discovery.packs {
            let pack = load_probe_pack(&pack_ref.manifest_path)?;
            if pack.id != pack_ref.id {
                return Err(RegistryLoadError::PackIdMismatch {
                    dir_id: pack_ref.id,
                    manifest_id: pack.id,
                });
            }
            packs.push(pack);
        }
        Ok(Self::from_packs(packs)?)
    }

    /// The admitted packs, sorted by id.
    #[must_use]
    pub fn packs(&self) -> &[ProbePack] {
        &self.packs
    }

    /// Every probe across every pack, in deterministic (pack id, declared)
    /// order.
    pub fn probes(&self) -> impl Iterator<Item = &Probe> {
        self.packs.iter().flat_map(|pack| pack.probes.iter())
    }

    /// Select every probe whose category the `profile` includes, in
    /// deterministic (pack id, declared) order.
    #[must_use]
    pub fn select(&self, profile: &RiskProfile) -> Vec<&Probe> {
        self.probes()
            .filter(|probe| profile.includes(probe.category))
            .collect()
    }
}

/// A discovered probe pack: its directory and where its manifest lives, without
/// having loaded or validated the manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbePackRef {
    /// The pack directory's own name, captured at discovery time before the
    /// manifest is parsed. [`ProbeRegistry::load`] verifies this agrees with
    /// the manifest-declared [`ProbePack::id`] and refuses admission
    /// ([`RegistryLoadError::PackIdMismatch`]) if they disagree, so once a pack
    /// is admitted this is the authoritative pack id.
    pub id: String,
    /// The pack directory, under `.anvil/probes/`.
    pub dir: PathBuf,
    /// The manifest path (`<dir>/probes.yaml`), known to exist as a file.
    pub manifest_path: PathBuf,
}

/// Why a directory under `.anvil/probes/` was rejected rather than discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectionReason {
    /// The entry canonicalises outside the probes directory — a symlink escaping
    /// the workspace-scoped root. Fail-closed: skipped so external content is
    /// never treated as an installed pack.
    ContainmentEscape,
    /// The entry could not be canonicalised for the containment check (removed
    /// mid-scan, or a broken symlink). Skipped fail-closed rather than trusted.
    Unresolvable,
}

/// A rejected entry, reported so a caller can see a tampered or unresolvable
/// entry was skipped rather than silently dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedEntry {
    /// The offending path, under `.anvil/probes/`.
    pub path: PathBuf,
    /// Why it was rejected.
    pub reason: RejectionReason,
}

/// The outcome of scanning `.anvil/probes/`: discovered packs and rejected
/// entries, each deterministically ordered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProbePackDiscovery {
    /// Discovered packs, sorted by id then directory.
    pub packs: Vec<ProbePackRef>,
    /// Entries skipped fail-closed (containment escape or unresolvable), sorted
    /// by path.
    pub rejected: Vec<RejectedEntry>,
}

impl ProbePackDiscovery {
    /// Whether nothing at all was found — no packs and no rejects.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.packs.is_empty() && self.rejected.is_empty()
    }
}

/// A whole-discovery failure. Per-entry problems are reported in
/// [`ProbePackDiscovery::rejected`], not here.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    /// The probes directory exists but could not be read, or the workspace root
    /// could not be canonicalised. Never folded into an empty result.
    #[error("could not read probes directory {path}: {source}")]
    Io {
        /// The path being read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The probes directory itself canonicalises outside the workspace root.
    /// Nothing beneath it can be trusted, so the whole scan fails.
    #[error(
        "probes directory {resolved} resolves outside the workspace root {root} \
         (path containment breach)"
    )]
    ProbesDirEscapesRoot {
        /// The canonical workspace root.
        root: PathBuf,
        /// The escaping canonical probes directory.
        resolved: PathBuf,
    },
}

/// A pack paired with the result of loading its manifest, as returned by
/// [`discover_and_load`]. `manifest` carries the per-pack load error verbatim;
/// there is no short-circuit, so a broken pack never hides a good one.
#[derive(Debug)]
pub struct LoadedProbePack {
    /// The discovered pack.
    pub pack: ProbePackRef,
    /// The loaded and validated manifest, or the load/validation error.
    pub manifest: Result<ProbePack, ProbePackError>,
}

/// A fail-closed [`ProbeRegistry::load`] failure.
#[derive(Debug, Error)]
pub enum RegistryLoadError {
    /// The scan itself failed (see [`DiscoveryError`]).
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
    /// A discovered entry was rejected fail-closed; the registry refuses to
    /// proceed with partial coverage.
    #[error("probe pack entry {path} was rejected ({reason:?}); refusing partial registry", path = .0.path.display(), reason = .0.reason)]
    Rejected(RejectedEntry),
    /// A discovered pack's manifest failed to load or validate.
    #[error(transparent)]
    Pack(#[from] ProbePackError),
    /// A pack's directory name does not match its manifest-declared `id` — the
    /// registry refuses to admit it rather than key it under a directory name
    /// that could silently diverge from the authoritative pack id.
    #[error(
        "probe pack directory `{dir_id}` does not match its manifest id `{manifest_id}`; \
         rename the directory or correct the manifest so they agree"
    )]
    PackIdMismatch {
        /// The pack directory's own name (the discovered [`ProbePackRef::id`]).
        dir_id: String,
        /// The manifest-declared [`ProbePack::id`].
        manifest_id: String,
    },
}

/// Discover probe packs under `<workspace_root>/.anvil/probes/`.
///
/// Scans exactly one directory level: an immediate subdirectory with a
/// `probes.yaml` is a pack; an entry escaping the workspace-scoped root is
/// rejected per-entry while the scan continues. A missing probes directory
/// returns an empty [`ProbePackDiscovery`] (not an error). Results are
/// deterministically sorted.
///
/// # Errors
///
/// Returns [`DiscoveryError::Io`] if the probes directory exists but cannot be
/// read, or [`DiscoveryError::ProbesDirEscapesRoot`] if it resolves outside the
/// workspace root.
pub fn discover_probe_packs(workspace_root: &Path) -> Result<ProbePackDiscovery, DiscoveryError> {
    let probes_dir = workspace_root.join(PROBES_SUBDIR);
    // Nothing installed yet is a normal state — but only true absence qualifies:
    // a broken `.anvil/probes` symlink is surfaced as an I/O error below
    // (fail-closed), never read as "missing".
    match std::fs::symlink_metadata(&probes_dir) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ProbePackDiscovery::default());
        }
        Err(source) => {
            return Err(DiscoveryError::Io {
                path: probes_dir,
                source,
            });
        }
        Ok(_) => {}
    }

    let canonical_root = workspace_root
        .canonicalize()
        .map_err(|source| DiscoveryError::Io {
            path: workspace_root.to_path_buf(),
            source,
        })?;
    let canonical_probes = probes_dir
        .canonicalize()
        .map_err(|source| DiscoveryError::Io {
            path: probes_dir.clone(),
            source,
        })?;
    if !canonical_probes.starts_with(&canonical_root) {
        return Err(DiscoveryError::ProbesDirEscapesRoot {
            root: canonical_root,
            resolved: canonical_probes,
        });
    }

    let mut packs = Vec::new();
    let mut rejected = Vec::new();

    let entries = std::fs::read_dir(&canonical_probes).map_err(|source| DiscoveryError::Io {
        path: canonical_probes.clone(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| DiscoveryError::Io {
            path: canonical_probes.clone(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            classify_dir(&canonical_probes, &path, &mut packs, &mut rejected);
        } else if entry.file_type().is_ok_and(|t| t.is_symlink()) && !path.exists() {
            // A broken symlink is reported, not silently dropped.
            rejected.push(RejectedEntry {
                path: path.clone(),
                reason: RejectionReason::Unresolvable,
            });
        }
        // A non-directory regular file directly under `.anvil/probes/` is
        // ignored: a pack owns its own subdirectory.
    }

    packs.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.dir.cmp(&b.dir)));
    rejected.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(ProbePackDiscovery { packs, rejected })
}

/// Discover packs and load each one's manifest, carrying per-pack errors.
///
/// There is no short-circuit — every discovered pack is loaded and its result
/// carried in [`LoadedProbePack::manifest`], so one broken pack does not hide
/// the packs that loaded cleanly. Rejected entries are not surfaced here; a
/// caller that needs them uses [`discover_probe_packs`] directly. The
/// fail-closed admission variant is [`ProbeRegistry::load`].
///
/// # Errors
///
/// Propagates the whole-discovery [`DiscoveryError`] from
/// [`discover_probe_packs`].
pub fn discover_and_load(workspace_root: &Path) -> Result<Vec<LoadedProbePack>, DiscoveryError> {
    let discovery = discover_probe_packs(workspace_root)?;
    Ok(discovery
        .packs
        .into_iter()
        .map(|pack| {
            let manifest = load_probe_pack(&pack.manifest_path);
            LoadedProbePack { pack, manifest }
        })
        .collect())
}

/// Classify an immediate subdirectory of the probes dir: a pack (if it holds a
/// `probes.yaml` and stays contained), a rejected escape, or ignored otherwise.
fn classify_dir(
    canonical_probes: &Path,
    dir: &Path,
    packs: &mut Vec<ProbePackRef>,
    rejected: &mut Vec<RejectedEntry>,
) {
    // Containment first: a directory symlinked out of the probes root is
    // rejected before its contents are inspected.
    match dir.canonicalize() {
        Ok(canonical) if canonical.starts_with(canonical_probes) => {}
        Ok(_) => {
            rejected.push(RejectedEntry {
                path: dir.to_path_buf(),
                reason: RejectionReason::ContainmentEscape,
            });
            return;
        }
        Err(_) => {
            rejected.push(RejectedEntry {
                path: dir.to_path_buf(),
                reason: RejectionReason::Unresolvable,
            });
            return;
        }
    }
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
    packs.push(ProbePackRef {
        id,
        dir: dir.to_path_buf(),
        manifest_path,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvil_kernel_types::{ExpectedOutcome, PayloadClass};
    use tempfile::TempDir;

    const VALID_MANIFEST: &str = r"
id: baseline-probes
name: Baseline Adversarial Probes
version: 1.0.0
description: Core adversarial regression probes shipped with Anvil.
owner: platform-security
probes:
  - id: pi-override-001
    category: prompt-injection
    payload_class: direct-instruction
    expected_outcome: refused
    version: 1.0.0
    description: Direct instruction-override attempt must be refused.
  - id: exfil-secret-001
    category: data-exfiltration
    payload_class: embedded-content
    expected_outcome: blocked
    version: 1.0.0
    description: Secret-exfiltration attempt embedded in a document must be blocked.
";

    fn write_manifest(body: &str) -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("probes.yaml");
        std::fs::write(&path, body).expect("write manifest");
        (dir, path)
    }

    /// Create `<root>/.anvil/probes/<id>/probes.yaml` and return the pack dir.
    fn write_pack(root: &Path, id: &str, body: &str) -> PathBuf {
        let dir = root.join(PROBES_SUBDIR).join(id);
        std::fs::create_dir_all(&dir).expect("create pack dir");
        std::fs::write(dir.join(MANIFEST_FILENAME), body).expect("write manifest");
        dir
    }

    fn manifest_with_id(id: &str) -> String {
        VALID_MANIFEST.replacen("id: baseline-probes", &format!("id: {id}"), 1)
    }

    #[test]
    fn probe_registry_valid_pack_loads() {
        let (_dir, path) = write_manifest(VALID_MANIFEST);
        let pack = load_probe_pack(&path).expect("valid manifest loads");
        assert_eq!(pack.id, "baseline-probes");
        assert_eq!(pack.probes.len(), 2);
        assert_eq!(pack.probes[0].category, ProbeCategory::PromptInjection);
        assert_eq!(
            pack.probes[0].payload_class,
            PayloadClass::DirectInstruction
        );
        assert_eq!(pack.probes[1].expected_outcome, ExpectedOutcome::Blocked);
    }

    #[test]
    fn probe_registry_preserves_probe_order() {
        let (_dir, path) = write_manifest(VALID_MANIFEST);
        let pack = load_probe_pack(&path).expect("loads");
        let ids: Vec<&str> = pack.probes.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["pi-override-001", "exfil-secret-001"]);
    }

    #[test]
    fn probe_registry_missing_file_is_not_found() {
        let dir = TempDir::new().expect("temp dir");
        let missing = dir.path().join("absent.yaml");
        match load_probe_pack(&missing) {
            Err(ProbePackError::NotFound(_)) => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn probe_registry_unknown_root_field_rejected() {
        let body = format!("{VALID_MANIFEST}surprise: value\n");
        let (_dir, path) = write_manifest(&body);
        match load_probe_pack(&path) {
            Err(ProbePackError::Parse { .. }) => {}
            other => panic!("expected Parse for unknown root field, got {other:?}"),
        }
    }

    #[test]
    fn probe_registry_probe_tolerates_unknown_field_and_category() {
        // A probe entry is a forward-compatible wire asset: an unknown field is
        // tolerated and an unrecognised category deserialises to Unknown — the
        // pack still loads (contrast with the fail-closed pack root above).
        let body = VALID_MANIFEST.replace(
            "    category: data-exfiltration",
            "    category: model-inversion\n    future_field: 1",
        );
        let (_dir, path) = write_manifest(&body);
        let pack = load_probe_pack(&path).expect("probe-level forward-compat still loads");
        assert_eq!(pack.probes[1].category, ProbeCategory::Unknown);
    }

    #[test]
    fn probe_registry_duplicate_probe_id_rejected() {
        let body = VALID_MANIFEST.replace("id: exfil-secret-001", "id: pi-override-001");
        let (_dir, path) = write_manifest(&body);
        match load_probe_pack(&path) {
            Err(ProbePackError::DuplicateProbeId(id)) => assert_eq!(id, "pi-override-001"),
            other => panic!("expected DuplicateProbeId, got {other:?}"),
        }
    }

    #[test]
    fn probe_registry_empty_pack_rejected() {
        let body = "\
id: empty-pack
name: Empty
version: 1.0.0
description: Has no probes.
owner: o
probes: []
";
        let (_dir, path) = write_manifest(body);
        match load_probe_pack(&path) {
            Err(ProbePackError::EmptyPack { pack_id }) => assert_eq!(pack_id, "empty-pack"),
            other => panic!("expected EmptyPack, got {other:?}"),
        }
    }

    #[test]
    fn probe_registry_missing_pack_field_reported() {
        let body = VALID_MANIFEST.replace("owner: platform-security", "owner: \"\"");
        let (_dir, path) = write_manifest(&body);
        match load_probe_pack(&path) {
            Err(ProbePackError::MissingField { field: "owner" }) => {}
            other => panic!("expected MissingField owner, got {other:?}"),
        }
    }

    #[test]
    fn probe_registry_blank_probe_version_reported() {
        let body = VALID_MANIFEST.replacen("    version: 1.0.0", "    version: \"\"", 1);
        let (_dir, path) = write_manifest(&body);
        match load_probe_pack(&path) {
            Err(ProbePackError::ProbeMissingField { probe_id, field }) => {
                assert_eq!(probe_id, "pi-override-001");
                assert_eq!(field, "version");
            }
            other => panic!("expected ProbeMissingField version, got {other:?}"),
        }
    }

    #[test]
    fn probe_registry_in_memory_validate_matches_loader() {
        let pack: ProbePack = serde_yaml::from_str(VALID_MANIFEST).expect("parse");
        assert!(pack.validate().is_ok());
    }

    #[test]
    fn probe_registry_selection_by_risk_profile() {
        let pack: ProbePack = serde_yaml::from_str(VALID_MANIFEST).expect("parse");
        let registry = ProbeRegistry::from_packs(vec![pack]).expect("admit");

        let profile = RiskProfile::new("injection-only", vec![ProbeCategory::PromptInjection]);
        let selected = registry.select(&profile);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "pi-override-001");

        // A profile spanning both categories selects both, in declared order.
        let both = RiskProfile::new(
            "baseline",
            vec![
                ProbeCategory::PromptInjection,
                ProbeCategory::DataExfiltration,
            ],
        );
        let all = registry.select(&both);
        assert_eq!(all.len(), 2);
        assert!(
            registry
                .select(&both)
                .iter()
                .any(|p| p.id == "exfil-secret-001")
        );

        // A profile matching nothing selects nothing.
        let empty = RiskProfile::new("none", vec![ProbeCategory::BoundaryEvasion]);
        assert!(registry.select(&empty).is_empty());
    }

    #[test]
    fn probe_registry_from_packs_sorts_by_id_deterministically() {
        let beta: ProbePack = serde_yaml::from_str(&manifest_with_id("beta-probes")).expect("beta");
        let alpha: ProbePack =
            serde_yaml::from_str(&manifest_with_id("alpha-probes")).expect("alpha");
        let registry = ProbeRegistry::from_packs(vec![beta, alpha]).expect("admit");
        let ids: Vec<&str> = registry.packs().iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["alpha-probes", "beta-probes"]);
        // `probes()` iterates in pack-id then declared order.
        assert_eq!(registry.probes().count(), 4);
    }

    #[test]
    fn probe_registry_from_packs_rejects_duplicate_pack_id() {
        let a: ProbePack = serde_yaml::from_str(VALID_MANIFEST).expect("a");
        let b: ProbePack = serde_yaml::from_str(VALID_MANIFEST).expect("b");
        match ProbeRegistry::from_packs(vec![a, b]) {
            Err(ProbePackError::DuplicatePackId(id)) => assert_eq!(id, "baseline-probes"),
            other => panic!("expected DuplicatePackId, got {other:?}"),
        }
    }

    #[test]
    fn probe_registry_discovery_missing_dir_is_empty_ok() {
        let ws = TempDir::new().expect("workspace");
        let discovery = discover_probe_packs(ws.path()).expect("missing dir is Ok");
        assert!(discovery.is_empty(), "{discovery:?}");
    }

    #[test]
    fn probe_registry_discovery_two_packs_sorted() {
        let ws = TempDir::new().expect("workspace");
        write_pack(ws.path(), "beta-probes", &manifest_with_id("beta-probes"));
        write_pack(ws.path(), "alpha-probes", &manifest_with_id("alpha-probes"));
        let discovery = discover_probe_packs(ws.path()).expect("discover");
        let ids: Vec<&str> = discovery.packs.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["alpha-probes", "beta-probes"]);
        assert!(discovery.rejected.is_empty());
    }

    #[test]
    fn probe_registry_load_admits_discovered_packs() {
        let ws = TempDir::new().expect("workspace");
        write_pack(ws.path(), "baseline-probes", VALID_MANIFEST);
        let registry = ProbeRegistry::load(ws.path()).expect("fail-closed admission");
        assert_eq!(registry.packs().len(), 1);
        assert_eq!(registry.probes().count(), 2);
    }

    #[test]
    fn probe_registry_load_is_empty_when_nothing_installed() {
        let ws = TempDir::new().expect("workspace");
        let registry = ProbeRegistry::load(ws.path()).expect("empty registry");
        assert_eq!(registry.packs().len(), 0);
    }

    #[test]
    fn probe_registry_load_fails_closed_on_broken_pack() {
        // A registry must not silently drop a pack whose manifest will not load:
        // that would under-select probes and quietly reduce coverage.
        let ws = TempDir::new().expect("workspace");
        write_pack(ws.path(), "good", &manifest_with_id("good"));
        write_pack(ws.path(), "busted", "id: busted\nname: n\n"); // missing fields
        match ProbeRegistry::load(ws.path()) {
            Err(RegistryLoadError::Pack(_)) => {}
            other => panic!("expected fail-closed Pack error, got {other:?}"),
        }
    }

    #[test]
    fn probe_registry_load_rejects_pack_id_directory_mismatch() {
        // The directory name (ProbePackRef::id) and the manifest-declared
        // ProbePack::id must agree — otherwise the "pack id" used for discovery
        // and sorting could silently diverge from the id actually keyed into the
        // registry and the eval store.
        let ws = TempDir::new().expect("workspace");
        write_pack(ws.path(), "dir-name", &manifest_with_id("manifest-name"));
        match ProbeRegistry::load(ws.path()) {
            Err(RegistryLoadError::PackIdMismatch {
                dir_id,
                manifest_id,
            }) => {
                assert_eq!(dir_id, "dir-name");
                assert_eq!(manifest_id, "manifest-name");
            }
            other => panic!("expected PackIdMismatch, got {other:?}"),
        }
    }

    #[test]
    fn probe_registry_discover_and_load_carries_broken_manifest() {
        // discover_and_load does NOT hide a good pack behind a broken one.
        let ws = TempDir::new().expect("workspace");
        write_pack(ws.path(), "good", &manifest_with_id("good"));
        write_pack(ws.path(), "busted", "id: busted\nname: n\n");
        let loaded = discover_and_load(ws.path()).expect("discover_and_load");
        assert_eq!(loaded.len(), 2);
        let good = loaded.iter().find(|l| l.pack.id == "good").unwrap();
        let busted = loaded.iter().find(|l| l.pack.id == "busted").unwrap();
        assert!(good.manifest.is_ok());
        assert!(busted.manifest.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn probe_registry_discovery_symlink_escape_rejected() {
        // A pack directory symlinked outside the probes root is rejected
        // per-entry while a legitimate sibling is still discovered.
        let outside = TempDir::new().expect("outside");
        std::fs::write(
            outside.path().join(MANIFEST_FILENAME),
            manifest_with_id("evil"),
        )
        .expect("outside manifest");

        let ws = TempDir::new().expect("workspace");
        write_pack(ws.path(), "legit", &manifest_with_id("legit"));
        let link = ws.path().join(PROBES_SUBDIR).join("escape");
        std::os::unix::fs::symlink(outside.path(), &link).expect("symlink");

        let discovery = discover_probe_packs(ws.path()).expect("discover continues past escape");
        let ids: Vec<&str> = discovery.packs.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["legit"]);
        assert_eq!(discovery.rejected.len(), 1);
        assert_eq!(
            discovery.rejected[0].reason,
            RejectionReason::ContainmentEscape
        );

        // And fail-closed admission refuses the whole registry on the escape.
        match ProbeRegistry::load(ws.path()) {
            Err(RegistryLoadError::Rejected(entry)) => {
                assert_eq!(entry.reason, RejectionReason::ContainmentEscape);
            }
            other => panic!("expected Rejected, got {other:?}"),
        }
    }
}
