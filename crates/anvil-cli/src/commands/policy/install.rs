//! `anvil policy install` / `anvil policy show` — install bundled starter
//! policy packs into the local policy set with validated provenance, or preview
//! a bundled pack's manifest without installing it.
//!
//! Bundled packs are embedded in the binary at compile time (`include_str!`);
//! there is no remote fetch. Install writes a pack's files under
//! `<workspace>/.anvil/policies/<pack-id>/`, refusing to overwrite existing
//! files unless `--force`, then runs the full pack-admission pipeline (manifest
//! load, structural/metadata validation, test execution and enforcement) over
//! the installed copy. An install that fails validation is rolled back, so the
//! live gate directory never holds an invalid pack. A `provenance.yaml` records
//! the pack id, version, install source, and a sha256 for every installed file
//! (no timestamps — the version control history records when).
//!
//! ## Path containment
//!
//! The destination is resolved and canonicalised before anything is written:
//! the deepest existing ancestor of the pack directory must stay within the
//! canonical workspace root (so a `.anvil` symlinked outside the workspace is a
//! reported install failure with nothing written). The same guard is applied
//! per-write inside the [`Journal`] as defence in depth.
//!
//! ## Crash-safety
//!
//! The rollback [`Journal`] is in-memory only, not a crash-safe transaction: a
//! process killed mid-install can leave partially-written files on disk. The
//! recovery path is the existing-files pre-check — the next `install` without
//! `--force` detects and names those files and refuses, so a partial install is
//! visible rather than silently completed.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::Serialize;
use sha2::{Digest, Sha256};

use anvil_policy_engine::pack::{
    IssueCode, IssueSeverity, PackManifest, PolicySeverity, ValidationReport, enforce_tests,
    load_manifest, run_pack_tests, validate_pack,
};

use crate::GlobalArgs;
use crate::output;

/// Canonical manifest filename inside every pack directory.
const MANIFEST_FILENAME: &str = "pack.yaml";

/// Provenance record filename written beside the installed pack files.
const PROVENANCE_FILENAME: &str = "provenance.yaml";

/// One file inside a bundled pack: its pack-root-relative path (forward-slash)
/// and its verbatim compile-time contents.
struct BundledFile {
    rel: &'static str,
    contents: &'static str,
}

/// A starter pack embedded in the binary at compile time.
struct BundledPack {
    id: &'static str,
    files: &'static [BundledFile],
}

impl BundledPack {
    /// The pack's `pack.yaml` contents.
    fn manifest_contents(&self) -> &'static str {
        self.files
            .iter()
            .find(|f| f.rel == MANIFEST_FILENAME)
            .map(|f| f.contents)
            .expect("every bundled pack embeds a pack.yaml")
    }

    /// Parse and validate the embedded manifest. A bundled pack whose manifest
    /// does not validate is a build-time defect, surfaced loudly here.
    fn manifest(&self) -> Result<PackManifest> {
        let manifest: PackManifest = serde_yaml::from_str(self.manifest_contents())
            .with_context(|| format!("parsing embedded manifest for pack `{}`", self.id))?;
        manifest
            .validate()
            .with_context(|| format!("validating embedded manifest for pack `{}`", self.id))?;
        // Guard against a dual id source drifting: the enumeration id and the
        // manifest's own `id` must agree, or install/show would key off one
        // while the gate directory is named for the other.
        if manifest.id != self.id {
            bail!(
                "embedded pack id mismatch: registry id `{}` but manifest declares `{}`",
                self.id,
                manifest.id
            );
        }
        Ok(manifest)
    }

    /// The pack's files as owned write units, in embedded (declared) order.
    fn pack_files(&self) -> Vec<PackFile> {
        self.files
            .iter()
            .map(|f| PackFile {
                rel: f.rel.to_string(),
                contents: f.contents.to_string(),
            })
            .collect()
    }
}

/// The single bundled starter pack shipped in slice 1. Guardrails shaped over
/// the working-tree diff: change-set size and sensitive-path review.
const ANVIL_BASELINE: BundledPack = BundledPack {
    id: "anvil-baseline",
    files: &[
        BundledFile {
            rel: "pack.yaml",
            contents: include_str!("starter_packs/anvil-baseline/pack.yaml"),
        },
        BundledFile {
            rel: "policies/change_scope.rego",
            contents: include_str!("starter_packs/anvil-baseline/policies/change_scope.rego"),
        },
        BundledFile {
            rel: "policies/change_scope_test.rego",
            contents: include_str!("starter_packs/anvil-baseline/policies/change_scope_test.rego"),
        },
        BundledFile {
            rel: "policies/sensitive_paths.rego",
            contents: include_str!("starter_packs/anvil-baseline/policies/sensitive_paths.rego"),
        },
        BundledFile {
            rel: "policies/sensitive_paths_test.rego",
            contents: include_str!(
                "starter_packs/anvil-baseline/policies/sensitive_paths_test.rego"
            ),
        },
    ],
};

/// Every bundled starter pack, in enumeration order.
const BUNDLED_PACKS: &[BundledPack] = &[ANVIL_BASELINE];

/// Locate a bundled pack by id, or fail with the list of known ids.
fn find_pack(pack_id: &str) -> Result<&'static BundledPack> {
    BUNDLED_PACKS
        .iter()
        .find(|p| p.id == pack_id)
        .with_context(|| {
            let known: Vec<&str> = BUNDLED_PACKS.iter().map(|p| p.id).collect();
            format!(
                "unknown starter pack `{pack_id}`; available packs: {}",
                known.join(", ")
            )
        })
}

/// A file to write into a pack directory: pack-root-relative path and contents.
struct PackFile {
    rel: String,
    contents: String,
}

/// One provenance entry: an installed file and the sha256 of its bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ProvenanceFile {
    path: String,
    sha256: String,
}

/// The provenance record written beside an installed pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Provenance {
    pack: String,
    version: String,
    installed_from: String,
    files: Vec<ProvenanceFile>,
}

/// The result of an install attempt.
#[derive(Debug)]
enum InstallOutcome {
    /// The pack was installed and passed validation.
    Installed {
        dest_dir: PathBuf,
        written: Vec<String>,
        provenance: Provenance,
        report: ValidationReport,
    },
    /// The pack failed validation and was rolled back — nothing remains on disk.
    RolledBack {
        dest_dir: PathBuf,
        report: ValidationReport,
    },
}

/// Canonicalise `path`'s deepest existing ancestor and require it to stay
/// within `canonical_root`. Purely a containment check — it creates nothing.
///
/// `path` itself usually does not exist yet (it is about to be created), so the
/// check walks up to the first ancestor that does exist and canonicalises that:
/// a `.anvil` directory symlinked outside the workspace canonicalises outside
/// `canonical_root` and is rejected before any content is written. This mirrors
/// the gate's bundle containment guard (`gate.rs` `run_check_policy`) and the
/// pack loader's `resolve_member_path`.
fn ensure_within_root(canonical_root: &Path, path: &Path) -> io::Result<()> {
    let mut cursor = path;
    let existing = loop {
        if cursor.exists() {
            break cursor;
        }
        match cursor.parent() {
            Some(parent) => cursor = parent,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("no existing ancestor of {} to resolve", path.display()),
                ));
            }
        }
    };
    let canonical = existing.canonicalize()?;
    if canonical.starts_with(canonical_root) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "{} resolves outside the workspace root (path containment breach)",
                path.display()
            ),
        ))
    }
}

/// Journal of filesystem effects so an install can be fully undone.
///
/// Holds the canonical workspace root so every write and directory creation can
/// re-check containment as defence in depth (the caller has already checked the
/// destination once, fail-fast, before the journal is used).
struct Journal {
    root: PathBuf,
    created_files: Vec<PathBuf>,
    restored: Vec<(PathBuf, Vec<u8>)>,
    created_dirs: Vec<PathBuf>,
}

impl Journal {
    fn new(canonical_root: PathBuf) -> Self {
        Self {
            root: canonical_root,
            created_files: Vec::new(),
            restored: Vec::new(),
            created_dirs: Vec::new(),
        }
    }

    /// Undo every recorded effect: remove created files, restore overwritten
    /// files to their original bytes, then remove created directories deepest
    /// first. Best-effort — each step ignores its own error so one failure does
    /// not abort the rest of the unwind.
    fn rollback(self) {
        for path in &self.created_files {
            let _ = std::fs::remove_file(path);
        }
        for (path, original) in &self.restored {
            let _ = std::fs::write(path, original);
        }
        for dir in self.created_dirs.iter().rev() {
            let _ = std::fs::remove_dir(dir);
        }
    }

    /// Create `dir` and any missing ancestors, recording each newly-created
    /// directory so rollback can remove them. Containment is re-checked before
    /// each new directory is created so a race that swaps an ancestor for an
    /// escaping symlink is still caught.
    fn create_dirs(&mut self, dir: &Path) -> io::Result<()> {
        if dir.exists() {
            return Ok(());
        }
        let mut pending = Vec::new();
        let mut cursor = Some(dir);
        while let Some(candidate) = cursor {
            if candidate.exists() {
                break;
            }
            pending.push(candidate.to_path_buf());
            cursor = candidate.parent();
        }
        for candidate in pending.iter().rev() {
            ensure_within_root(&self.root, candidate)?;
            std::fs::create_dir(candidate)?;
            self.created_dirs.push(candidate.clone());
        }
        Ok(())
    }

    /// Write `contents` to `abs`, recording enough to undo it. An existing file
    /// is backed up before it is overwritten so rollback restores it exactly.
    /// Containment is re-checked before the write (defence in depth).
    fn write(&mut self, abs: &Path, contents: &[u8]) -> io::Result<()> {
        if let Some(parent) = abs.parent() {
            self.create_dirs(parent)?;
        }
        ensure_within_root(&self.root, abs)?;
        if abs.exists() {
            // Journal the backup BEFORE attempting the overwrite: a write
            // that fails part-way (disk full, permission flip) must still
            // leave the journal able to restore the pre-install bytes.
            let original = std::fs::read(abs)?;
            self.restored.push((abs.to_path_buf(), original));
            std::fs::write(abs, contents)?;
        } else {
            // Same ordering for creations: journal first, so a partial
            // write is still removed by rollback.
            self.created_files.push(abs.to_path_buf());
            std::fs::write(abs, contents)?;
        }
        Ok(())
    }
}

/// Install a set of pack files into `<workspace>/.anvil/policies/<pack_id>/`.
///
/// Refuses (before writing anything) to overwrite existing target files unless
/// `force` is set. After writing, runs the pack-admission pipeline over the
/// installed copy; an invalid pack is rolled back so the gate directory never
/// holds a partial or invalid pack. Returns [`InstallOutcome`] for the two
/// non-error results; only operational failures (existing files without
/// `--force`, I/O errors, a manifest that will not parse) return [`Err`].
fn install_pack_files(
    workspace: &Path,
    pack_id: &str,
    version: &str,
    files: &[PackFile],
    force: bool,
) -> Result<InstallOutcome> {
    let dest_dir = workspace.join(".anvil/policies").join(pack_id);

    // Path containment (fail-fast, before anything is written): canonicalise the
    // workspace root and require the destination's deepest existing ancestor to
    // resolve within it. A `.anvil` symlinked outside the workspace is a
    // reported install failure, not a write through the link.
    let canonical_root = workspace
        .canonicalize()
        .with_context(|| format!("resolving workspace root {}", workspace.display()))?;
    if let Err(e) = ensure_within_root(&canonical_root, &dest_dir) {
        bail!("refusing to install into {}: {e}", dest_dir.display());
    }

    // Provenance hashes every pack file's bytes, in declared order.
    let provenance = Provenance {
        pack: pack_id.to_string(),
        version: version.to_string(),
        installed_from: format!("bundled:{}", env!("CARGO_PKG_VERSION")),
        files: files
            .iter()
            .map(|f| ProvenanceFile {
                path: f.rel.clone(),
                sha256: sha256_hex(f.contents.as_bytes()),
            })
            .collect(),
    };
    let provenance_yaml =
        serde_yaml::to_string(&provenance).context("serialising pack provenance")?;

    // The full write set: pack files plus the provenance record.
    let mut write_set: Vec<(String, Vec<u8>)> = files
        .iter()
        .map(|f| (f.rel.clone(), f.contents.clone().into_bytes()))
        .collect();
    write_set.push((
        PROVENANCE_FILENAME.to_string(),
        provenance_yaml.into_bytes(),
    ));

    // Fail-closed pre-check: refuse to clobber existing files without --force.
    if !force {
        let existing: Vec<String> = write_set
            .iter()
            .filter(|(rel, _)| dest_dir.join(rel).exists())
            .map(|(rel, _)| rel.clone())
            .collect();
        if !existing.is_empty() {
            bail!(
                "refusing to overwrite existing files in {} (pass --force to overwrite): {}",
                dest_dir.display(),
                existing.join(", ")
            );
        }
    }

    // Write everything, journaling for rollback.
    let mut journal = Journal::new(canonical_root);
    let mut written = Vec::with_capacity(write_set.len());
    for (rel, bytes) in &write_set {
        let abs = dest_dir.join(rel);
        if let Err(e) = journal.write(&abs, bytes) {
            journal.rollback();
            return Err(e).with_context(|| format!("writing {}", abs.display()));
        }
        written.push(rel.clone());
    }

    // Admission over the installed copy. A manifest that will not load, or a
    // test run that cannot start, is an operational failure — roll back first.
    let manifest_path = dest_dir.join(MANIFEST_FILENAME);
    let report = match assemble_report(&manifest_path, &dest_dir) {
        Ok(report) => report,
        Err(e) => {
            journal.rollback();
            return Err(e);
        }
    };

    if report.is_valid() {
        Ok(InstallOutcome::Installed {
            dest_dir,
            written,
            provenance,
            report,
        })
    } else {
        // Never leave an invalid pack in the live gate directory.
        journal.rollback();
        Ok(InstallOutcome::RolledBack { dest_dir, report })
    }
}

/// Run the full pack-admission pipeline over an installed pack and fold every
/// issue into one report. Mirrors `anvil policy validate`'s assembly: the
/// structural validator's pre-enforcement missing-test warning is dropped in
/// favour of test enforcement's error, and a missing `.rego`'s load-error
/// restatement is de-duplicated against its `missing-policy-file`.
fn assemble_report(manifest_path: &Path, base_dir: &Path) -> Result<ValidationReport> {
    let manifest = load_manifest(manifest_path)
        .with_context(|| format!("loading pack manifest {}", manifest_path.display()))?;

    let mut report = validate_pack(&manifest, base_dir);
    report
        .issues
        .retain(|issue| issue.code != IssueCode::MissingTestFile);

    let test_report = run_pack_tests(&manifest, base_dir).context("running policy pack tests")?;
    report.issues.extend(enforce_tests(&test_report));

    let missing_file_ids: BTreeSet<String> = report
        .issues
        .iter()
        .filter(|i| i.code == IssueCode::MissingPolicyFile)
        .filter_map(|i| i.policy_id.clone())
        .collect();
    report.issues.retain(|i| {
        !(i.code == IssueCode::PolicyTestFailed
            && i.policy_id
                .as_ref()
                .is_some_and(|id| missing_file_ids.contains(id)))
    });

    Ok(report)
}

/// Lowercase hex sha256 of `bytes`.
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Resolve the workspace root to install into: the explicit `--workspace`
/// override, else the detected workspace root.
fn resolve_workspace(explicit: Option<&Path>) -> Result<PathBuf> {
    match explicit {
        Some(path) => Ok(path.to_path_buf()),
        None => crate::util::workspace_root(),
    }
}

// ── CLI wiring ──────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct InstallArgs {
    /// Identifier of the bundled pack to install (see `--list`).
    #[arg(required_unless_present = "list")]
    pack_id: Option<String>,
    /// List the bundled starter packs and exit.
    #[arg(long)]
    list: bool,
    /// Overwrite existing pack files instead of refusing.
    #[arg(long)]
    force: bool,
    /// Workspace root to install into (defaults to the current workspace).
    #[arg(long)]
    workspace: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    /// Identifier of the bundled pack to show (see `install --list`).
    pack_id: String,
}

pub fn run_install(args: &InstallArgs, global: &GlobalArgs) -> Result<()> {
    if args.list {
        return list_packs(global);
    }

    let pack_id = args
        .pack_id
        .as_deref()
        .expect("clap requires a pack id unless --list is set");
    let pack = find_pack(pack_id)?;
    let manifest = pack.manifest()?;
    let workspace = resolve_workspace(args.workspace.as_deref())?;

    let outcome = install_pack_files(
        &workspace,
        pack.id,
        &manifest.version,
        &pack.pack_files(),
        args.force,
    )?;

    report_install(&outcome, global)
}

pub fn run_show(args: &ShowArgs, global: &GlobalArgs) -> Result<()> {
    let pack = find_pack(&args.pack_id)?;
    let manifest = pack.manifest()?;
    render_show(&manifest, global)
}

/// Machine-readable summary of a bundled pack for `--list` / `show --json`.
#[derive(Debug, Serialize)]
struct PackSummary {
    id: String,
    name: String,
    version: String,
    description: String,
    policies: Vec<PolicySummary>,
}

#[derive(Debug, Serialize)]
struct PolicySummary {
    id: String,
    title: String,
    severity: String,
}

fn pack_summary(manifest: &PackManifest) -> PackSummary {
    PackSummary {
        id: manifest.id.clone(),
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        description: manifest.description.clone(),
        policies: manifest
            .policies
            .iter()
            .map(|entry| PolicySummary {
                id: entry.metadata.id.clone(),
                title: entry.metadata.title.clone(),
                severity: entry
                    .metadata
                    .severity
                    .map_or("unset", severity_label)
                    .to_string(),
            })
            .collect(),
    }
}

/// Stable lowercase label for a policy severity band.
fn severity_label(severity: PolicySeverity) -> &'static str {
    match severity {
        PolicySeverity::Low => "low",
        PolicySeverity::Medium => "medium",
        PolicySeverity::High => "high",
        PolicySeverity::Critical => "critical",
    }
}

fn list_packs(global: &GlobalArgs) -> Result<()> {
    let summaries: Vec<PackSummary> = BUNDLED_PACKS
        .iter()
        .map(|pack| pack.manifest().map(|m| pack_summary(&m)))
        .collect::<Result<_>>()?;

    if global.json {
        output::json::print(&summaries)?;
    } else {
        output::plain::blank();
        output::plain::section("Bundled starter packs");
        for summary in &summaries {
            println!(
                "  {id:<16} {name} (v{version})",
                id = summary.id,
                name = summary.name,
                version = summary.version
            );
            println!("    {}", summary.description);
            println!("    {} policy(ies)", summary.policies.len());
        }
        output::plain::blank();
        println!("{} pack(s)", summaries.len());
    }
    Ok(())
}

fn render_show(manifest: &PackManifest, global: &GlobalArgs) -> Result<()> {
    let summary = pack_summary(manifest);
    if global.json {
        output::json::print(&summary)?;
    } else {
        output::plain::blank();
        println!("Starter pack: {} (v{})", summary.id, summary.version);
        output::plain::blank();
        output::plain::label("Name", &summary.name);
        output::plain::label("Description", &summary.description);
        output::plain::blank();
        output::plain::section("Policies");
        for policy in &summary.policies {
            println!(
                "  {id:<18} [{severity}] {title}",
                id = policy.id,
                severity = policy.severity,
                title = policy.title
            );
        }
    }
    Ok(())
}

fn report_install(outcome: &InstallOutcome, global: &GlobalArgs) -> Result<()> {
    match outcome {
        InstallOutcome::Installed {
            dest_dir,
            written,
            provenance,
            report,
        } => {
            if global.json {
                #[derive(Serialize)]
                struct InstalledJson<'a> {
                    status: &'a str,
                    pack: &'a str,
                    dest: String,
                    installed_files: &'a [String],
                    provenance: &'a Provenance,
                    warnings: usize,
                }
                let warnings = report
                    .issues
                    .iter()
                    .filter(|i| i.severity == IssueSeverity::Warning)
                    .count();
                output::json::print(&InstalledJson {
                    status: "installed",
                    pack: &provenance.pack,
                    dest: dest_dir.display().to_string(),
                    installed_files: written,
                    provenance,
                    warnings,
                })?;
            } else {
                output::plain::blank();
                output::plain::success(&format!(
                    "Installed pack `{}` into {}",
                    provenance.pack,
                    dest_dir.display()
                ));
                for rel in written {
                    output::plain::info(rel);
                }
                let warnings: Vec<_> = report
                    .issues
                    .iter()
                    .filter(|i| i.severity == IssueSeverity::Warning)
                    .collect();
                if warnings.is_empty() {
                    output::plain::success("Validation passed — no issues.");
                } else {
                    output::plain::success(&format!(
                        "Validation passed — {} warning(s).",
                        warnings.len()
                    ));
                    for issue in warnings {
                        output::plain::warn(&issue.message);
                    }
                }
            }
            Ok(())
        }
        InstallOutcome::RolledBack { dest_dir, report } => {
            let errors: Vec<_> = report
                .issues
                .iter()
                .filter(|i| i.severity == IssueSeverity::Error)
                .collect();
            if global.json {
                #[derive(Serialize)]
                struct RolledBackJson<'a> {
                    status: &'a str,
                    dest: String,
                    report: &'a ValidationReport,
                }
                output::json::print(&RolledBackJson {
                    status: "rolled-back",
                    dest: dest_dir.display().to_string(),
                    report,
                })?;
            } else {
                output::plain::blank();
                output::plain::error(&format!(
                    "Install rolled back — pack failed validation ({} error(s)); \
                     nothing was left in {}",
                    errors.len(),
                    dest_dir.display()
                ));
                for issue in errors {
                    output::plain::error(&issue.message);
                    output::plain::info(&format!("fix: {}", issue.remediation));
                }
            }
            Err(output::AlreadyReported.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn baseline_files() -> Vec<PackFile> {
        ANVIL_BASELINE.pack_files()
    }

    /// The single bundled pack's own manifest must validate — a build guard.
    #[test]
    fn policy_install_bundled_manifest_validates() {
        let manifest = ANVIL_BASELINE
            .manifest()
            .expect("embedded manifest validates");
        assert_eq!(manifest.id, "anvil-baseline");
        assert_eq!(manifest.policies.len(), 2);
    }

    #[test]
    fn policy_install_writes_and_validates_pack() {
        let ws = TempDir::new().expect("workspace");
        let version = ANVIL_BASELINE.manifest().unwrap().version;
        let outcome = install_pack_files(
            ws.path(),
            "anvil-baseline",
            &version,
            &baseline_files(),
            false,
        )
        .expect("install");

        let InstallOutcome::Installed {
            dest_dir,
            written,
            provenance,
            report,
        } = outcome
        else {
            panic!("expected a clean install");
        };

        assert!(report.is_valid(), "unexpected issues: {:?}", report.issues);
        // Pack files + provenance.yaml are all present on disk.
        assert!(dest_dir.join("pack.yaml").is_file());
        assert!(dest_dir.join("policies/change_scope.rego").is_file());
        assert!(dest_dir.join("policies/sensitive_paths.rego").is_file());
        assert!(dest_dir.join(PROVENANCE_FILENAME).is_file());
        assert!(written.iter().any(|w| w == PROVENANCE_FILENAME));

        // Provenance source is the bundled build, with no timestamp field.
        assert!(provenance.installed_from.starts_with("bundled:"));
        let raw = std::fs::read_to_string(dest_dir.join(PROVENANCE_FILENAME)).unwrap();
        assert!(
            !raw.contains("timestamp") && !raw.contains("installed_at"),
            "{raw}"
        );
    }

    #[test]
    fn policy_install_provenance_hashes_verify() {
        let ws = TempDir::new().expect("workspace");
        let version = ANVIL_BASELINE.manifest().unwrap().version;
        let outcome = install_pack_files(
            ws.path(),
            "anvil-baseline",
            &version,
            &baseline_files(),
            false,
        )
        .expect("install");
        let InstallOutcome::Installed {
            dest_dir,
            provenance,
            ..
        } = outcome
        else {
            panic!("expected a clean install");
        };

        // Every provenance hash matches the bytes actually on disk.
        for entry in &provenance.files {
            let bytes = std::fs::read(dest_dir.join(&entry.path))
                .unwrap_or_else(|_| panic!("read {}", entry.path));
            assert_eq!(
                entry.sha256,
                sha256_hex(&bytes),
                "hash mismatch for {}",
                entry.path
            );
        }
        // Provenance covers every pack file (manifest + 4 rego), not itself.
        assert_eq!(provenance.files.len(), 5);
        assert!(
            !provenance
                .files
                .iter()
                .any(|f| f.path == PROVENANCE_FILENAME)
        );
    }

    #[test]
    fn policy_install_second_install_refuses_without_force() {
        let ws = TempDir::new().expect("workspace");
        let version = ANVIL_BASELINE.manifest().unwrap().version;
        install_pack_files(
            ws.path(),
            "anvil-baseline",
            &version,
            &baseline_files(),
            false,
        )
        .expect("first install");

        let err = install_pack_files(
            ws.path(),
            "anvil-baseline",
            &version,
            &baseline_files(),
            false,
        )
        .expect_err("second install must refuse");
        let msg = format!("{err:#}");
        assert!(msg.contains("refusing to overwrite"), "{msg}");
        // The refusal names existing files.
        assert!(msg.contains("pack.yaml"), "{msg}");
        // It is an operational error, not a reported validation failure.
        assert!(!err.is::<output::AlreadyReported>());
    }

    #[test]
    fn policy_install_force_overwrites() {
        let ws = TempDir::new().expect("workspace");
        let version = ANVIL_BASELINE.manifest().unwrap().version;
        install_pack_files(
            ws.path(),
            "anvil-baseline",
            &version,
            &baseline_files(),
            false,
        )
        .expect("first install");

        // A subsequent --force install succeeds and re-validates.
        let outcome = install_pack_files(
            ws.path(),
            "anvil-baseline",
            &version,
            &baseline_files(),
            true,
        )
        .expect("force install");
        assert!(matches!(outcome, InstallOutcome::Installed { .. }));
    }

    #[test]
    fn policy_install_invalid_pack_rolls_back_completely() {
        // Corrupt the change_scope policy so its tests fail, then install. The
        // pack must roll back, leaving no trace in the workspace.
        let ws = TempDir::new().expect("workspace");
        let version = ANVIL_BASELINE.manifest().unwrap().version;
        let mut files = baseline_files();
        for file in &mut files {
            if file.rel == "policies/change_scope.rego" {
                // Neuter the soft-threshold rule so its `test_soft_threshold_warns`
                // sibling can never pass — admission fails, forcing a rollback.
                file.contents = file.contents.replace("changed_count > soft_limit", "false");
            }
        }

        let outcome = install_pack_files(ws.path(), "anvil-baseline", &version, &files, false)
            .expect("install returns an outcome, not an error");
        let InstallOutcome::RolledBack { dest_dir, report } = outcome else {
            panic!("a failing pack must roll back");
        };
        assert!(!report.is_valid(), "expected validation errors");

        // Rollback leaves nothing: no pack dir, no partial files.
        assert!(!dest_dir.exists(), "pack dir must be gone after rollback");
        assert!(!dest_dir.join("pack.yaml").exists());
        assert!(!dest_dir.join(PROVENANCE_FILENAME).exists());
    }

    #[test]
    fn policy_install_rollback_preserves_pre_existing_files() {
        // A --force install over a corrupt pack must restore the file it
        // overwrote, not delete it (no data loss on rollback).
        let ws = TempDir::new().expect("workspace");
        let version = ANVIL_BASELINE.manifest().unwrap().version;
        install_pack_files(
            ws.path(),
            "anvil-baseline",
            &version,
            &baseline_files(),
            false,
        )
        .expect("seed a valid install");
        let policy_path = ws
            .path()
            .join(".anvil/policies/anvil-baseline/policies/change_scope.rego");
        let original = std::fs::read_to_string(&policy_path).unwrap();

        let mut broken = baseline_files();
        for file in &mut broken {
            if file.rel == "policies/change_scope.rego" {
                file.contents = file.contents.replace("changed_count > soft_limit", "false");
            }
        }
        let outcome = install_pack_files(ws.path(), "anvil-baseline", &version, &broken, true)
            .expect("install outcome");
        assert!(matches!(outcome, InstallOutcome::RolledBack { .. }));

        // The original valid policy is back, byte-for-byte.
        assert_eq!(std::fs::read_to_string(&policy_path).unwrap(), original);
    }

    #[test]
    fn policy_install_installed_pack_is_gate_discoverable() {
        // The installed .rego files live under .anvil/policies/ where the gate
        // discovers them; the test siblings and provenance.yaml are excluded by
        // the gate's own discovery filters (verified structurally here).
        let ws = TempDir::new().expect("workspace");
        let version = ANVIL_BASELINE.manifest().unwrap().version;
        install_pack_files(
            ws.path(),
            "anvil-baseline",
            &version,
            &baseline_files(),
            false,
        )
        .expect("install");
        let dir = ws.path().join(".anvil/policies/anvil-baseline/policies");
        // Non-test .rego present (gate-discovered); *_test.rego present but the
        // gate excludes it by suffix; provenance.yaml is not a .rego at all.
        assert!(dir.join("change_scope.rego").is_file());
        assert!(dir.join("change_scope_test.rego").is_file());
    }

    #[test]
    fn policy_install_pack_is_advisory_only() {
        // Advisory-first (slice 1): the starter pack defines no `violation` or
        // `deny` rule, so the gate only ever surfaces warnings — never a block —
        // from it. Blocking arrives later via posture-driven enforcement
        // routing, not via Rego severity.
        for file in baseline_files() {
            let is_rego = Path::new(&file.rel)
                .extension()
                .is_some_and(|e| e == "rego");
            if is_rego && !file.rel.ends_with("_test.rego") {
                assert!(
                    !file.contents.contains("violation contains"),
                    "{} must define no violation rule (advisory-first)",
                    file.rel
                );
                assert!(
                    !file.contents.contains("deny contains"),
                    "{} must define no deny rule (advisory-first)",
                    file.rel
                );
                // No dead references to a non-existent input config escape hatch.
                assert!(
                    !file.contents.contains("input.config"),
                    "{} must not read input.config (not on the PolicyInput contract)",
                    file.rel
                );
            }
        }
    }

    // A `.anvil` symlinked outside the workspace must be a reported install
    // failure with nothing written through the link. Unix-only (symlink API).
    #[cfg(unix)]
    #[test]
    fn policy_install_refuses_escaping_anvil_symlink() {
        let ws = TempDir::new().expect("workspace");
        let outside = TempDir::new().expect("outside dir");
        std::os::unix::fs::symlink(outside.path(), ws.path().join(".anvil")).expect("symlink");

        let version = ANVIL_BASELINE.manifest().unwrap().version;
        let err = install_pack_files(
            ws.path(),
            "anvil-baseline",
            &version,
            &baseline_files(),
            false,
        )
        .expect_err("an escaping .anvil symlink must be refused");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("containment") || msg.contains("outside"),
            "{msg}"
        );
        // The external directory the link points at is untouched — nothing was
        // written through the symlink.
        assert!(
            !outside.path().join("policies").exists(),
            "external dir must be untouched"
        );
    }

    #[test]
    fn policy_install_unknown_pack_lists_available() {
        let Err(err) = find_pack("does-not-exist") else {
            panic!("unknown pack must not resolve");
        };
        let msg = format!("{err:#}");
        assert!(msg.contains("anvil-baseline"), "{msg}");
    }

    #[test]
    fn policy_install_list_enumerates_bundled_packs() {
        let summaries: Vec<PackSummary> = BUNDLED_PACKS
            .iter()
            .map(|pack| pack.manifest().map(|m| pack_summary(&m)))
            .collect::<Result<_>>()
            .expect("summaries");
        assert!(summaries.iter().any(|s| s.id == "anvil-baseline"));
        let baseline = summaries.iter().find(|s| s.id == "anvil-baseline").unwrap();
        assert_eq!(baseline.policies.len(), 2);
    }

    #[test]
    fn policy_install_show_renders_manifest_summary() {
        let manifest = ANVIL_BASELINE.manifest().unwrap();
        let summary = pack_summary(&manifest);
        assert_eq!(summary.id, "anvil-baseline");
        assert!(summary.policies.iter().any(|p| p.id == "change-scope"));
        assert!(
            summary
                .policies
                .iter()
                .any(|p| p.id == "sensitive-paths" && p.severity == "high")
        );
        // JSON round-trips the summary shape.
        let json = serde_json::to_string(&summary).expect("serialise");
        assert!(json.contains("sensitive-paths"), "{json}");
    }
}
