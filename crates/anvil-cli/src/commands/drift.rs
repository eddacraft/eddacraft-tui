use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::{Deserialize, Serialize};

use anvil_architecture::{load_baseline, read_to_string_capped};
use anvil_checks::antipattern::{AntipatternCheckConfig, run_antipattern_check};

use crate::GlobalArgs;
use crate::output::{self, OutputMode};
use crate::util::is_ignored_dir_name;

const SNAPSHOTS_DIR: &str = "snapshots";
const ANVIL_DIR: &str = ".anvil";
const SNAPSHOT_PREFIX: &str = "snapshot-";
/// Maximum size of a single drift snapshot JSON the readers load into memory
/// (CIB-084). An over-cap file is skipped/errored rather than committing
/// unbounded memory.
const MAX_SNAPSHOT_BYTES: u64 = 16 * 1024 * 1024;
/// Maximum number of snapshot files [`list_snapshot_files`] scans before sorting
/// (CIB-084). Bounds the per-file read+parse work a pathological
/// `.anvil/snapshots/` directory can force; excess files are dropped with a
/// logged warning.
const MAX_SNAPSHOTS_SCANNED: usize = 1000;

// ── Drift baseline schema versioning (OPSUP-003) ────────────────────
//
// The baseline schema version is no longer a hand-edited string constant.
// It is *derived* from a registry of per-surface field declarations: each
// Track 3/4 surface declares the baseline fields it contributes and the
// schema version that introduced them. The current schema version is the
// highest `introduced_in` across the registry, so shipping a new surface
// advances the version additively rather than mutating a literal in place.
//
// On load, a baseline whose version is newer than this binary understands
// is rejected with an "upgrade anvil" message instead of silently dropping
// the fields the newer schema added. Migrating an older baseline forward is
// owned by OPSUP-004 and is out of scope here.

/// A `major.minor.patch` drift baseline schema version. Ordering is
/// lexicographic over the three components (derived), so comparisons answer
/// "is this baseline newer than what we support?".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SchemaVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

impl SchemaVersion {
    const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parse a `major.minor.patch` string. Every component must be a
    /// non-negative integer; anything else is a hard error so an
    /// unrecognised baseline is never read as though it were understood.
    fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        let [major, minor, patch] = parts.as_slice() else {
            bail!("schema version '{s}' must be major.minor.patch");
        };
        let component = |p: &str| -> Result<u32> {
            p.parse::<u32>().map_err(|_| {
                anyhow::anyhow!(
                    "schema version '{s}' has a component that is not a u32 \
                     (non-numeric or out of range)"
                )
            })
        };
        Ok(Self::new(
            component(major)?,
            component(minor)?,
            component(patch)?,
        ))
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A baseline-field contribution from one surface, tagged with the schema
/// version that introduced it. Adding a new declaration with a higher
/// `introduced_in` is how the schema advances additively.
struct FieldDeclaration {
    /// The surface or pack contributing these fields (documentation only).
    #[allow(dead_code)]
    surface: &'static str,
    /// The baseline fields this surface contributes (documentation only).
    #[allow(dead_code)]
    fields: &'static [&'static str],
    introduced_in: SchemaVersion,
}

/// The per-surface baseline field registry. Concrete future surface fields
/// are owned by their surface modules (OPSUP-003 non-scope); this is the
/// versioning mechanism plus the v1.0.0 fields that ship today.
const FIELD_DECLARATIONS: &[FieldDeclaration] = &[
    FieldDeclaration {
        surface: "core",
        fields: &["schema_version", "created_at", "name", "git_ref"],
        introduced_in: SchemaVersion::new(1, 0, 0),
    },
    FieldDeclaration {
        // Nested under the top-level `metrics` object; paths are dotted so
        // the registry mirrors the actual snapshot JSON shape.
        surface: "metrics",
        fields: &[
            "metrics.boundary_violations",
            "metrics.antipattern_count",
            "metrics.suppression_count",
            "metrics.expired_suppressions",
            "metrics.files_analysed",
        ],
        introduced_in: SchemaVersion::new(1, 0, 0),
    },
    FieldDeclaration {
        surface: "architecture",
        fields: &["violations", "antipattern_breakdown"],
        introduced_in: SchemaVersion::new(1, 0, 0),
    },
    FieldDeclaration {
        surface: "antipattern",
        fields: &["antipatterns"],
        introduced_in: SchemaVersion::new(1, 0, 0),
    },
    FieldDeclaration {
        surface: "suppressions",
        fields: &["suppressions"],
        introduced_in: SchemaVersion::new(1, 0, 0),
    },
    FieldDeclaration {
        // SURFSQL-006: the SQL governance surface contributes its findings to
        // the drift baseline so the gate can warn only on *new* edges. Added
        // additively, so a v1.0.0 baseline still reads (the field defaults to
        // empty) and shipping this surface advances the schema to v1.1.0.
        surface: "sql-migrations",
        fields: &["sql_findings"],
        introduced_in: SchemaVersion::new(1, 1, 0),
    },
];

/// The schema version a registry describes: the highest `introduced_in`
/// across its declarations. Pure over its input so an additive declaration
/// can be shown to advance the version in tests.
fn schema_version_for(declarations: &[FieldDeclaration]) -> SchemaVersion {
    declarations
        .iter()
        .map(|d| d.introduced_in)
        .max()
        .unwrap_or_else(|| SchemaVersion::new(1, 0, 0))
}

/// The schema version this binary writes and is the newest it can read.
fn current_schema() -> SchemaVersion {
    schema_version_for(FIELD_DECLARATIONS)
}

/// Reject a baseline written by a newer schema than this binary understands.
/// Equal or older baselines are accepted (forward migration of older
/// baselines is OPSUP-004's concern); a newer one fails loudly rather than
/// silently dropping the fields the newer schema added.
fn ensure_readable(baseline: SchemaVersion, current: SchemaVersion) -> Result<()> {
    if baseline > current {
        bail!(
            "baseline schema {baseline} is newer than this anvil understands \
             (supported up to {current}); upgrade anvil to read it"
        );
    }
    Ok(())
}

// ── Drift baseline migration (OPSUP-004) ────────────────────────────

/// Outcome counts from a `anvil drift migrate` run.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
struct MigrateReport {
    /// Baselines upgraded from an older schema to the current one.
    migrated: usize,
    /// Baselines already on the current schema — left untouched.
    already_current: usize,
    /// Compatibility alias: baselines written by a newer anvil.
    newer: usize,
    skipped: usize,
    partial: bool,
    skipped_by_reason: BTreeMap<MigrateSkipReason, usize>,
    backups: BackupPruneReport,
}

impl MigrateReport {
    fn add_skip(&mut self, reason: MigrateSkipReason, count: usize) {
        if count == 0 {
            return;
        }
        *self.skipped_by_reason.entry(reason).or_insert(0) += count;
        self.skipped += count;
        self.partial = true;
        if reason == MigrateSkipReason::NewerSchema {
            self.newer += count;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum MigrateSkipReason {
    Unreadable,
    InvalidJson,
    InvalidSchemaVersion,
    NewerSchema,
    ScanLimitExceeded,
    WriteFailed,
}

impl MigrateSkipReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Unreadable => "unreadable",
            Self::InvalidJson => "invalid_json",
            Self::InvalidSchemaVersion => "invalid_schema_version",
            Self::NewerSchema => "newer_schema",
            Self::ScanLimitExceeded => "scan_limit_exceeded",
            Self::WriteFailed => "write_failed",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
struct BackupPruneReport {
    pruned: usize,
    retained: usize,
    skipped: usize,
    retention: BackupRetention,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct BackupRetention {
    mode: &'static str,
    keep_per_baseline: usize,
}

impl Default for BackupRetention {
    fn default() -> Self {
        Self {
            mode: "count",
            keep_per_baseline: 1,
        }
    }
}

/// The base backup path for a baseline: a `<file>.bak` sibling so it sorts
/// next to the original and is obviously a backup. Retained for one release as
/// a manual rollback escape hatch. Extension is `bak` (not `json`) so a backup
/// is never re-discovered as a snapshot by [`list_snapshot_files`].
fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".bak");
    path.with_file_name(name)
}

/// Write `content` to the next backup generation —
/// `<file>.bak`, then `<file>.bak.1`, `<file>.bak.2`, … — creating it
/// **exclusively** (`O_EXCL`). Returns the path written.
///
/// Migration *always* writes the pre-migration content to a fresh file (never
/// skipping the backup) and *never* overwrites an existing backup (a prior
/// migration's rollback copy, or an unrelated stale `.bak`). The never-clobber
/// guarantee is enforced atomically by the OS via exclusive create, not a racy
/// `exists()` pre-check, and an exhausted candidate space is a hard error
/// rather than a silent fall back to clobbering. The original file is untouched
/// until after this returns, so a crash mid-backup loses nothing — a re-run
/// simply writes the next free candidate.
fn write_fresh_backup(path: &Path, content: &[u8]) -> Result<PathBuf> {
    use std::io::Write as _;

    let base = backup_path(path);
    let base_name = base
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let start = max_backup_generation(path)?.map_or(0, |n| n + 1);
    // Bounded so a directory already full of `.bak.N` files can't spin forever.
    for n in start..=10_000 {
        let candidate = if n == 0 {
            base.clone()
        } else {
            base.with_file_name(format!("{base_name}.{n}"))
        };
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                file.write_all(content)
                    .with_context(|| format!("writing backup {}", candidate.display()))?;
                return Ok(candidate);
            }
            // Candidate taken — try the next suffix.
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => {
                return Err(anyhow::Error::new(e)
                    .context(format!("creating backup {}", candidate.display())));
            }
        }
    }
    bail!(
        "could not create a backup for {}: too many existing .bak files",
        path.display()
    );
}

fn max_backup_generation(path: &Path) -> Result<Option<usize>> {
    let base = backup_path(path);
    let Some(dir) = base.parent() else {
        return Ok(None);
    };
    if !dir.exists() {
        return Ok(None);
    }
    let Some(base_name) = base.file_name().map(|n| n.to_string_lossy().into_owned()) else {
        return Ok(None);
    };
    let mut max = None;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(generation) = backup_generation_for_name(&name, &base_name) {
            max = Some(max.map_or(generation, |m: usize| m.max(generation)));
        }
    }
    Ok(max)
}

fn backup_generation_for_name(name: &str, base_name: &str) -> Option<usize> {
    if name == base_name {
        return Some(0);
    }
    let suffix = name.strip_prefix(base_name)?.strip_prefix('.')?;
    suffix.parse::<usize>().ok()
}

/// One-line hint emitted when a loaded baseline is on an older schema than this
/// binary writes, pointing the user at `anvil drift migrate` rather than
/// leaving the staleness implicit (OPSUP-004). Returns `None` for a baseline
/// that is current (or newer, which is handled by [`ensure_readable`]).
fn outdated_schema_hint(snapshot: &DriftSnapshot) -> Option<String> {
    let baseline = SchemaVersion::parse(&snapshot.schema_version).ok()?;
    (baseline < current_schema()).then(|| {
        format!(
            "note: drift baseline schema {baseline} is older than this anvil \
             ({}); run `anvil drift migrate` to upgrade it",
            current_schema()
        )
    })
}

/// Emit the OPSUP-004 one-line migrate hint to stderr if any of the loaded
/// `snapshots` is on an older schema. Stderr so it never corrupts `--json`
/// stdout; once per command (a single line, per the spec). Shared by the
/// `report` and `compare` read paths — both load baselines that may be stale.
fn emit_stale_baseline_hint(snapshots: &[&DriftSnapshot]) {
    if let Some(hint) = snapshots.iter().copied().find_map(outdated_schema_hint) {
        eprintln!("{hint}");
    }
}

/// Migrate every drift baseline in `workspace` that is on an older schema
/// version to the current one, backing up the original first. Pure of the
/// write-gate and output concerns so it is unit-testable; `run_migrate` wraps
/// it with the ADR-060 write guard and rendering.
fn migrate_snapshots(workspace: &Path, prune_backups: bool) -> Result<MigrateReport> {
    migrate_snapshots_capped(workspace, prune_backups, MAX_SNAPSHOTS_SCANNED)
}

fn migrate_snapshots_capped(
    workspace: &Path,
    prune_backups: bool,
    snapshot_cap: usize,
) -> Result<MigrateReport> {
    let mut report = MigrateReport::default();
    let current = current_schema();
    if prune_backups {
        report.backups = prune_drift_backups(workspace)?;
    }

    let listed = list_snapshot_files_capped_report(workspace, snapshot_cap)?;
    report.add_skip(MigrateSkipReason::ScanLimitExceeded, listed.ignored);
    for path in listed.files {
        // Read + parse directly rather than via `load_snapshot_file`: that
        // helper bails on a future schema, but migrate must *skip* such a
        // baseline (it cannot be downgraded) without aborting the whole run.
        let content = match read_to_string_capped(&path, MAX_SNAPSHOT_BYTES) {
            Ok(c) => c,
            Err(e) => {
                let _ = e;
                report.add_skip(MigrateSkipReason::Unreadable, 1);
                continue;
            }
        };
        let snapshot: DriftSnapshot = match serde_json::from_str(&content) {
            Ok(s) => s,
            Err(e) => {
                let _ = e;
                report.add_skip(MigrateSkipReason::InvalidJson, 1);
                continue;
            }
        };
        let Ok(version) = SchemaVersion::parse(&snapshot.schema_version) else {
            report.add_skip(MigrateSkipReason::InvalidSchemaVersion, 1);
            continue;
        };

        // The struct parse above validated the baseline and read its version;
        // the in-place rewrite works on the raw JSON so unknown/custom fields
        // are preserved rather than silently dropped by a struct round-trip.
        match version.cmp(&current) {
            std::cmp::Ordering::Equal => report.already_current += 1,
            std::cmp::Ordering::Greater => report.add_skip(MigrateSkipReason::NewerSchema, 1),
            std::cmp::Ordering::Less => match migrate_one(&path, &content, current) {
                Ok(()) => report.migrated += 1,
                Err(_) => report.add_skip(MigrateSkipReason::WriteFailed, 1),
            },
        }
    }

    Ok(report)
}

fn prune_drift_backups(workspace: &Path) -> Result<BackupPruneReport> {
    let mut report = BackupPruneReport::default();
    let dir = snapshots_dir(workspace);
    if !dir.exists() {
        return Ok(report);
    }
    let mut live: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(SNAPSHOT_PREFIX)
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
            && entry.file_type().is_ok_and(|ft| ft.is_file())
        {
            live.push(path);
        }
    }
    for snapshot in live {
        let base = backup_path(&snapshot);
        let Some(base_name) = base.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        let mut candidates: Vec<(usize, PathBuf)> = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(generation) = backup_generation_for_name(&name, &base_name) else {
                continue;
            };
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path)?;
            if meta.file_type().is_symlink() || !meta.is_file() {
                report.skipped += 1;
                continue;
            }
            candidates.push((generation, path));
        }
        if candidates.is_empty() {
            continue;
        }
        candidates.sort_by_key(|(generation, _)| *generation);
        let keep_generation = candidates.last().map(|(generation, _)| *generation);
        for (generation, path) in candidates {
            if Some(generation) == keep_generation {
                report.retained += 1;
                continue;
            }
            std::fs::remove_file(&path)
                .with_context(|| format!("pruning drift backup {}", path.display()))?;
            report.pruned += 1;
        }
    }
    Ok(report)
}

/// Back up `original_content`, then write it back re-stamped at `current`
/// schema in place.
///
/// The rewrite is done on the raw JSON value (only `schema_version` is
/// changed) so additive schema evolution stays lossless — any field not in the
/// current `DriftSnapshot` struct (a user annotation, a field from a future
/// patch) is preserved rather than dropped.
///
/// The pre-migration content is **always** written to a fresh backup path
/// ([`write_fresh_backup`], exclusive-create) before the in-place write — so
/// the original is never lost — and an existing backup is **never** clobbered,
/// so a prior migration's rollback copy survives. `serde_json` is built with
/// `preserve_order` workspace-wide, so the `Value` round-trip keeps field
/// order: the migrated file differs from the original only in `schema_version`.
fn migrate_one(path: &Path, original_content: &str, current: SchemaVersion) -> Result<()> {
    write_fresh_backup(path, original_content.as_bytes())
        .with_context(|| format!("backing up {} before migration", path.display()))?;

    let mut value: serde_json::Value = serde_json::from_str(original_content)
        .with_context(|| format!("re-parsing {} for migration", path.display()))?;
    value["schema_version"] = serde_json::Value::String(current.to_string());
    let json = serde_json::to_string_pretty(&value)?;
    crate::util::atomic_write(path, json.as_bytes())
        .with_context(|| format!("writing migrated baseline {}", path.display()))?;
    Ok(())
}

fn run_migrate(global: &GlobalArgs, prune_backups: bool) -> Result<()> {
    // DISTRIB-006 (ADR-060): migration rewrites durable `.anvil/snapshots/*.json`
    // in place. Refuse under a gated ANVIL_HOME without `--touch-project-state`.
    crate::install_root::ensure_project_write_allowed("drift migrate")?;

    let mode = OutputMode::from_global(global);
    let cwd = std::env::current_dir()?;
    let report = migrate_snapshots(&cwd, prune_backups)?;

    match mode {
        OutputMode::Json => output::json::print(&report)?,
        OutputMode::Plain | OutputMode::Tui | OutputMode::Sarif => {
            let total = report.migrated + report.already_current + report.skipped;
            if total == 0 && report.backups.pruned == 0 && report.backups.retained == 0 {
                output::plain::info("No drift baselines found — nothing to migrate");
            } else if report.migrated == 0
                && !report.partial
                && report.backups.pruned == 0
                && report.backups.retained == 0
            {
                output::plain::success("Drift baselines already current — nothing to migrate");
            } else {
                if report.migrated > 0 {
                    output::plain::success(&format!(
                        "Migrated {} drift baseline(s) to schema {} (originals backed up to .bak)",
                        report.migrated,
                        current_schema()
                    ));
                }
                if report.already_current > 0 {
                    output::plain::info(&format!(
                        "{} baseline(s) already current",
                        report.already_current
                    ));
                }
                if report.backups.pruned > 0 || report.backups.retained > 0 {
                    output::plain::success(&format!(
                        "Pruned {} drift backup(s); retained {} rollback backup(s)",
                        report.backups.pruned, report.backups.retained
                    ));
                }
                if report.partial {
                    output::plain::warn(&format!(
                        "Partial migration: {} baseline(s) skipped",
                        report.skipped
                    ));
                    for (reason, count) in &report.skipped_by_reason {
                        output::plain::dim(&format!("  - {}: {count}", reason.as_str()));
                    }
                }
            }
        }
    }
    if report.partial {
        return Err(output::AlreadyReported.into());
    }
    Ok(())
}

#[derive(Debug, Args)]
pub struct DriftArgs {
    #[command(subcommand)]
    command: DriftCommand,
}

#[derive(Debug, clap::Subcommand)]
enum DriftCommand {
    /// Capture current state as a snapshot.
    Snapshot {
        /// Give the snapshot a name (e.g. release-1.0).
        #[arg(long)]
        name: Option<String>,
    },
    /// Compare two snapshots.
    Compare {
        /// First snapshot (name or filename).
        snapshot1: String,
        /// Second snapshot (name or filename).
        snapshot2: String,
    },
    /// Generate a drift report (compares latest two or --since).
    Report {
        /// Compare against a specific snapshot.
        #[arg(long)]
        since: Option<String>,
    },
    /// List available snapshots.
    List {
        /// Limit number of results.
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Upgrade drift baselines on an older schema version to the current one.
    ///
    /// Each baseline is backed up to `<file>.bak` before any in-place write.
    /// Baselines already on the current schema are left untouched; baselines
    /// from a newer anvil are skipped rather than downgraded.
    Migrate {
        /// Prune older drift-baseline backups, retaining the latest rollback
        /// backup per live snapshot.
        #[arg(long)]
        prune_backups: bool,
    },
}

// ── Snapshot types (JSON-serialisable, parity with Node.js) ─────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftSnapshot {
    pub schema_version: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub metrics: SnapshotMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub antipattern_breakdown: Option<BTreeMap<String, usize>>,
    pub violations: Vec<SnapshotViolation>,
    pub antipatterns: Vec<SnapshotAntipattern>,
    pub suppressions: Vec<SnapshotSuppression>,
    /// SURFSQL-006 (schema v1.1.0): baselined SQL governance findings. Absent
    /// in a v1.0.0 baseline (`default` keeps that readable) and omitted from
    /// the JSON when empty so pre-SURFSQL snapshots round-trip byte-stable.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sql_findings: Vec<SnapshotSqlFinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetrics {
    pub boundary_violations: usize,
    pub antipattern_count: usize,
    pub suppression_count: usize,
    pub expired_suppressions: usize,
    pub files_analysed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotViolation {
    pub id: String,
    #[serde(rename = "type")]
    pub violation_type: String,
    pub from_file: String,
    pub to_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_layer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_layer: Option<String>,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotAntipattern {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub pattern: String,
    pub severity: String,
}

/// A baselined SURFSQL finding (SURFSQL-006). `fingerprint` is the move-
/// resistant identity used by the gate's new-edges-only filter — it hashes the
/// rule plus the whitespace-normalised statement, so re-indenting or moving a
/// flagged statement does not re-warn. `file`/`line` are retained for the
/// human-readable drift report only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotSqlFinding {
    pub fingerprint: String,
    pub rule_id: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotSuppression {
    pub id: String,
    pub pattern_id: String,
    pub file: String,
    pub line: usize,
    pub reason: String,
    pub scope: String,
}

// ── Comparison types ────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct MetricDelta {
    before: usize,
    after: usize,
    delta: i64,
    trend: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct ComparisonOutput {
    before: SnapshotRef,
    after: SnapshotRef,
    duration_days: f64,
    metrics: ComparisonMetrics,
    net_change: NetChange,
    violations: ChangeCounts,
    antipatterns: ChangeCounts,
    overall_trend: &'static str,
}

#[derive(Debug, Serialize)]
struct ChangeCounts {
    added: usize,
    removed: usize,
}

#[derive(Debug, Serialize)]
struct SnapshotRef {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct ComparisonMetrics {
    boundary_violations: MetricDelta,
    antipattern_count: MetricDelta,
    suppression_count: MetricDelta,
    expired_suppressions: MetricDelta,
    files_analysed: MetricDelta,
}

#[derive(Debug, Serialize)]
struct NetChange {
    violations: i64,
    antipatterns: i64,
    suppressions: i64,
}

// ── Snapshot metadata (for list) ────────────────────────────────────

#[derive(Debug, Serialize)]
struct SnapshotListEntry {
    filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    created_at: String,
    metrics: SnapshotMetrics,
}

// ── Entry point ─────────────────────────────────────────────────────

pub fn run(args: &DriftArgs, global: &GlobalArgs) -> Result<()> {
    match &args.command {
        DriftCommand::Snapshot { name } => run_snapshot(name.as_deref(), global),
        DriftCommand::Compare {
            snapshot1,
            snapshot2,
        } => run_compare(snapshot1, snapshot2, global),
        DriftCommand::Report { since } => run_report(since.as_deref(), global),
        DriftCommand::List { limit } => run_list(*limit, global),
        DriftCommand::Migrate { prune_backups } => run_migrate(global, *prune_backups),
    }
}

// ── Snapshot subcommand ─────────────────────────────────────────────

fn run_snapshot(name: Option<&str>, global: &GlobalArgs) -> Result<()> {
    // DISTRIB-006 (ADR-060): a drift snapshot persists `.anvil/snapshots/*.json`
    // that `anvil drift compare` later reads — durable per-project state. Refuse
    // under a gated ANVIL_HOME; `drift compare` is read-only and unaffected.
    crate::install_root::ensure_project_write_allowed("drift snapshot")?;

    let mode = OutputMode::from_global(global);
    let cwd = std::env::current_dir()?;

    // Gather source files.
    let files = get_source_files(&cwd)?;
    if mode != OutputMode::Json && global.verbose {
        output::plain::info(&format!("Scanning {} files...", files.len()));
    }

    // Load architecture baseline for violations.
    let baseline = load_baseline(&cwd)?;
    let violations: Vec<SnapshotViolation> = baseline
        .as_ref()
        .map(|b| {
            b.baseline_snapshot
                .violations
                .iter()
                .map(|v| SnapshotViolation {
                    id: v.id.clone(),
                    violation_type: "boundary".to_string(),
                    from_file: v.from_file.clone(),
                    to_file: v.to_file.clone(),
                    from_layer: Some(v.from_layer.clone()),
                    to_layer: Some(v.to_layer.clone()),
                    line: v.import_line,
                })
                .collect()
        })
        .unwrap_or_default();

    // Run antipattern scan and collect results.
    let (antipatterns, suppressions, ap_result) = collect_antipatterns(&files, &cwd);

    // SURFSQL-006: capture SQL governance findings so the gate can baseline
    // them and warn only on new edges. Independent walk — SQL migration files
    // are outside the antipattern extension set.
    let sql_findings = collect_sql_findings(&cwd);

    // Build antipattern breakdown.
    let mut breakdown: BTreeMap<String, usize> = BTreeMap::new();
    for ap in &antipatterns {
        *breakdown.entry(ap.id.clone()).or_insert(0) += 1;
    }

    let git_ref = get_git_ref();

    let snapshot = DriftSnapshot {
        schema_version: current_schema().to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        name: name.map(String::from),
        metrics: SnapshotMetrics {
            boundary_violations: violations.len(),
            antipattern_count: antipatterns.len(),
            suppression_count: suppressions.len(),
            expired_suppressions: 0, // Expiry tracking not yet implemented.
            files_analysed: ap_result.files_scanned,
        },
        antipattern_breakdown: if breakdown.is_empty() {
            None
        } else {
            Some(breakdown)
        },
        violations,
        antipatterns,
        suppressions,
        sql_findings,
        git_ref,
    };

    // Save to .anvil/snapshots/.
    let filename = save_snapshot(&cwd, &snapshot, name)?;

    match mode {
        OutputMode::Json => output::json::print(&snapshot)?,
        OutputMode::Plain | OutputMode::Tui | OutputMode::Sarif => {
            output::plain::success(&format!("Snapshot saved: {filename}"));
            output::plain::blank();
            output::plain::section("Metrics");
            output::plain::label("Violations", snapshot.metrics.boundary_violations);
            output::plain::label("Anti-patterns", snapshot.metrics.antipattern_count);
            output::plain::label("Suppressions", snapshot.metrics.suppression_count);
            output::plain::label("Files", snapshot.metrics.files_analysed);
            if let Some(n) = name {
                output::plain::blank();
                output::plain::info(&format!("Use 'anvil drift compare {n} <other>' to compare"));
            }
        }
    }
    Ok(())
}

// ── Compare subcommand ──────────────────────────────────────────────

fn run_compare(name1: &str, name2: &str, global: &GlobalArgs) -> Result<()> {
    let mode = OutputMode::from_global(global);
    let cwd = std::env::current_dir()?;

    let before = load_snapshot(&cwd, name1)?
        .ok_or_else(|| anyhow::anyhow!("Snapshot not found: {name1}"))?;
    let after = load_snapshot(&cwd, name2)?
        .ok_or_else(|| anyhow::anyhow!("Snapshot not found: {name2}"))?;

    emit_stale_baseline_hint(&[&before, &after]);

    let comparison = compare_snapshots(&before, &after);

    match mode {
        OutputMode::Json => output::json::print(&comparison)?,
        OutputMode::Plain | OutputMode::Tui | OutputMode::Sarif => {
            print_comparison(&comparison, &before, &after);
        }
    }
    Ok(())
}

// ── Report subcommand ───────────────────────────────────────────────

fn run_report(since: Option<&str>, global: &GlobalArgs) -> Result<()> {
    let mode = OutputMode::from_global(global);
    let cwd = std::env::current_dir()?;

    let (before, after) = if let Some(since_name) = since {
        let b = load_snapshot(&cwd, since_name)?
            .ok_or_else(|| anyhow::anyhow!("Snapshot not found: {since_name}"))?;
        let a = get_latest_snapshot(&cwd)?.ok_or_else(|| {
            anyhow::anyhow!("No current snapshot found. Run `anvil drift snapshot` first.")
        })?;
        (b, a)
    } else {
        let entries = list_snapshot_files(&cwd)?;
        // Skip corrupt snapshots (consistent with `drift list` behaviour).
        let valid: Vec<DriftSnapshot> = entries
            .iter()
            .filter_map(|p| match load_snapshot_file(p) {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!("warning: skipping corrupt snapshot {}: {e}", p.display());
                    None
                }
            })
            .take(2)
            .collect();
        if valid.len() < 2 {
            bail!(
                "Need at least 2 valid snapshots to generate a report. Run `anvil drift snapshot` to create snapshots."
            );
        }
        (valid[1].clone(), valid[0].clone())
    };

    // OPSUP-004: if either baseline is on an older schema, surface a one-line
    // migrate hint rather than leaving the staleness implicit.
    emit_stale_baseline_hint(&[&before, &after]);

    let comparison = compare_snapshots(&before, &after);

    match mode {
        OutputMode::Json => output::json::print(&comparison)?,
        OutputMode::Plain | OutputMode::Tui | OutputMode::Sarif => {
            output::plain::success("Drift report");
            print_comparison(&comparison, &before, &after);
        }
    }
    Ok(())
}

// ── List subcommand ─────────────────────────────────────────────────

fn run_list(limit: Option<usize>, global: &GlobalArgs) -> Result<()> {
    let mode = OutputMode::from_global(global);
    let cwd = std::env::current_dir()?;

    let mut entries = list_snapshots(&cwd)?;
    if let Some(n) = limit {
        entries.truncate(n);
    }

    if entries.is_empty() {
        if mode == OutputMode::Json {
            output::json::print(&entries)?;
        } else {
            output::plain::info("No snapshots found. Run `anvil drift snapshot` to create one.");
        }
        return Ok(());
    }

    match mode {
        OutputMode::Json => output::json::print(&entries)?,
        OutputMode::Plain | OutputMode::Tui | OutputMode::Sarif => {
            output::plain::blank();
            output::plain::section(&format!(
                "{}  {}  {}",
                "NAME".to_string() + &" ".repeat(16),
                "DATE      ",
                "METRICS"
            ));
            for entry in &entries {
                let name = entry.name.as_deref().unwrap_or_else(|| {
                    entry
                        .filename
                        .trim_start_matches(SNAPSHOT_PREFIX)
                        .trim_end_matches(".json")
                });
                let date = entry
                    .created_at
                    .split('T')
                    .next()
                    .unwrap_or(&entry.created_at);
                println!(
                    "  {:<20} {}  V:{} AP:{} S:{}",
                    name,
                    date,
                    entry.metrics.boundary_violations,
                    entry.metrics.antipattern_count,
                    entry.metrics.suppression_count,
                );
            }
            output::plain::blank();
            output::plain::dim("V=violations, AP=anti-patterns, S=suppressions");
        }
    }
    Ok(())
}

// ── Snapshot storage ────────────────────────────────────────────────

fn snapshots_dir(workspace: &Path) -> PathBuf {
    workspace.join(ANVIL_DIR).join(SNAPSHOTS_DIR)
}

/// The set of SURFSQL fingerprints baselined by the latest drift snapshot.
/// `None` means there is no readable snapshot at all (so the gate warns on all
/// findings and hints to create one); `Some(set)` means a snapshot exists —
/// possibly with an empty SQL set, when the repo was clean at snapshot time —
/// so the gate baselines against it and does *not* claim a baseline is absent.
/// Read-only and total: any I/O or parse failure yields `None` so the gate
/// degrades to warn-on-all rather than erroring (warnings over blocks).
/// Consumed by the SURFSQL gate check to surface only new findings (SURFSQL-006).
pub(crate) fn latest_sql_baseline_fingerprints(workspace: &Path) -> Option<BTreeSet<String>> {
    match get_latest_snapshot(workspace) {
        Ok(Some(snapshot)) => Some(
            snapshot
                .sql_findings
                .into_iter()
                .map(|f| f.fingerprint)
                .collect(),
        ),
        _ => None,
    }
}

fn sanitise_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn save_snapshot(workspace: &Path, snapshot: &DriftSnapshot, name: Option<&str>) -> Result<String> {
    let dir = snapshots_dir(workspace);
    std::fs::create_dir_all(&dir)?;

    let filename = if let Some(n) = name {
        let sanitised = sanitise_name(n);
        if !sanitised.chars().any(char::is_alphanumeric) {
            bail!("Snapshot name must contain at least one alphanumeric character");
        }
        format!("{SNAPSHOT_PREFIX}{sanitised}.json")
    } else {
        let ts = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S-%3f");
        format!("{SNAPSHOT_PREFIX}{ts}.json")
    };

    let path = dir.join(&filename);

    // Prevent silent overwrite of named snapshots.
    if let Some(n) = name
        && path.exists()
    {
        bail!("Snapshot '{n}' already exists. Use a different name or delete the existing one.");
    }

    let json = serde_json::to_string_pretty(snapshot)?;
    crate::util::atomic_write(&path, json.as_bytes())?;

    Ok(filename)
}

fn load_snapshot(workspace: &Path, name: &str) -> Result<Option<DriftSnapshot>> {
    let dir = snapshots_dir(workspace);

    // Candidates in priority order: exact name, with prefix, without prefix.
    let candidates = [
        dir.join(name),
        dir.join(format!("{SNAPSHOT_PREFIX}{}.json", sanitise_name(name))),
        dir.join(format!("{}.json", sanitise_name(name))),
    ];

    for candidate in &candidates {
        if !candidate.exists() {
            continue;
        }
        // Verify the resolved path stays inside the snapshots directory to
        // prevent path traversal via names like "../../etc/passwd".
        let canonical = candidate
            .canonicalize()
            .with_context(|| format!("resolving snapshot path {}", candidate.display()))?;
        let canonical_dir = dir.canonicalize().unwrap_or_else(|_| dir.clone());
        if !canonical.starts_with(&canonical_dir) {
            bail!("Unsafe snapshot name: resolved path escapes snapshots directory");
        }
        return Ok(Some(load_snapshot_file(&canonical)?));
    }

    Ok(None)
}

pub(crate) fn load_snapshot_file(path: &Path) -> Result<DriftSnapshot> {
    let content = read_to_string_capped(path, MAX_SNAPSHOT_BYTES)
        .with_context(|| format!("reading snapshot {}", path.display()))?;
    let snapshot: DriftSnapshot = serde_json::from_str(&content)
        .with_context(|| format!("parsing snapshot {}", path.display()))?;
    let baseline = SchemaVersion::parse(&snapshot.schema_version).with_context(|| {
        format!(
            "unrecognised drift baseline schema version in {}",
            path.display()
        )
    })?;
    ensure_readable(baseline, current_schema())
        .with_context(|| format!("drift baseline {}", path.display()))?;
    Ok(snapshot)
}

fn get_latest_snapshot(workspace: &Path) -> Result<Option<DriftSnapshot>> {
    let files = list_snapshot_files(workspace)?;
    for path in &files {
        match load_snapshot_file(path) {
            Ok(snap) => return Ok(Some(snap)),
            Err(e) => {
                eprintln!("warning: skipping corrupt snapshot {}: {e}", path.display());
            }
        }
    }
    Ok(None)
}

pub(crate) fn list_snapshot_files(workspace: &Path) -> Result<Vec<PathBuf>> {
    Ok(list_snapshot_files_capped_report(workspace, MAX_SNAPSHOTS_SCANNED)?.files)
}

struct SnapshotFileList {
    files: Vec<PathBuf>,
    ignored: usize,
}

/// [`list_snapshot_files`] with an explicit scan cap, so the count guard is
/// testable without creating thousands of fixtures.
#[cfg(test)]
fn list_snapshot_files_capped(workspace: &Path, cap: usize) -> Result<Vec<PathBuf>> {
    Ok(list_snapshot_files_capped_report(workspace, cap)?.files)
}

fn list_snapshot_files_capped_report(workspace: &Path, cap: usize) -> Result<SnapshotFileList> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    use std::time::SystemTime;

    let dir = snapshots_dir(workspace);
    if !dir.exists() {
        return Ok(SnapshotFileList {
            files: Vec::new(),
            ignored: 0,
        });
    }

    // CIB-084: keep only the `cap` most-recent snapshots (by mtime) in memory as
    // we scan, so a pathological `.anvil/snapshots/` can force neither an
    // unbounded path list here nor an unbounded number of content reads below.
    // mtime is a cheap stat (no file open); the authoritative `created_at` sort
    // runs on the bounded set. Unreadable mtimes sort oldest (evicted first).
    let mut newest: BinaryHeap<Reverse<(SystemTime, PathBuf)>> = BinaryHeap::new();
    let mut total = 0usize;
    for entry in std::fs::read_dir(&dir)?.filter_map(std::result::Result::ok) {
        let path = entry.path();
        let is_snapshot = path.extension().is_some_and(|e| e == "json")
            && path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with(SNAPSHOT_PREFIX));
        if !is_snapshot {
            continue;
        }
        total += 1;
        let mtime = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        newest.push(Reverse((mtime, path)));
        if newest.len() > cap {
            newest.pop(); // evict the oldest, so memory never exceeds `cap`
        }
    }
    let files: Vec<PathBuf> = newest.into_iter().map(|Reverse((_, p))| p).collect();

    // Sort by the `created_at` timestamp embedded in each snapshot's JSON
    // (descending — newest first). Named snapshots don't encode a timestamp
    // in their filename, so sorting by filename alone breaks chronological
    // order in mixed sets. Cache timestamps to avoid O(n log n) file reads.
    let mut keyed: Vec<(PathBuf, Option<chrono::DateTime<chrono::FixedOffset>>)> = files
        .into_iter()
        .map(|p| {
            let ts = read_created_at(&p);
            (p, ts)
        })
        .collect();

    keyed.sort_by(|(a, ts_a), (b, ts_b)| {
        if let (Some(ta), Some(tb)) = (ts_a, ts_b) {
            tb.cmp(ta)
        } else {
            let na = a
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let nb = b
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            nb.cmp(&na)
        }
    });

    let files: Vec<PathBuf> = keyed.into_iter().map(|(p, _)| p).collect();

    Ok(SnapshotFileList {
        files,
        ignored: total.saturating_sub(cap),
    })
}

/// Count snapshot files in `.anvil/snapshots/` cheaply — no per-file reads and no
/// scan cap — so callers can report the true total even when
/// [`list_snapshot_files`] caps how many it scans (CIB-084).
pub(crate) fn count_snapshot_files(workspace: &Path) -> Result<usize> {
    let dir = snapshots_dir(workspace);
    if !dir.exists() {
        return Ok(0);
    }
    let count = std::fs::read_dir(&dir)?
        .filter_map(std::result::Result::ok)
        .filter(|e| {
            let p = e.path();
            p.extension().is_some_and(|x| x == "json")
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with(SNAPSHOT_PREFIX))
        })
        .count();
    Ok(count)
}

fn list_snapshots(workspace: &Path) -> Result<Vec<SnapshotListEntry>> {
    let files = list_snapshot_files(workspace)?;
    let mut entries = Vec::with_capacity(files.len());

    for path in files {
        let snapshot = match load_snapshot_file(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("warning: skipping corrupt snapshot {}: {e}", path.display());
                continue;
            }
        };
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        entries.push(SnapshotListEntry {
            filename,
            name: snapshot.name,
            created_at: snapshot.created_at,
            metrics: snapshot.metrics,
        });
    }

    Ok(entries)
}

// ── Comparison logic ────────────────────────────────────────────────

pub(crate) fn compare_snapshots(before: &DriftSnapshot, after: &DriftSnapshot) -> ComparisonOutput {
    #[allow(clippy::cast_precision_loss)] // duration in days — precision loss is acceptable
    let duration = chrono::DateTime::parse_from_rfc3339(&after.created_at)
        .ok()
        .zip(chrono::DateTime::parse_from_rfc3339(&before.created_at).ok())
        .map_or(0.0, |(a, b)| (a - b).num_seconds() as f64 / 86400.0);

    let viol_delta = metric_delta(
        before.metrics.boundary_violations,
        after.metrics.boundary_violations,
    );
    let ap_delta = metric_delta(
        before.metrics.antipattern_count,
        after.metrics.antipattern_count,
    );
    let sup_delta = metric_delta(
        before.metrics.suppression_count,
        after.metrics.suppression_count,
    );
    let exp_delta = metric_delta(
        before.metrics.expired_suppressions,
        after.metrics.expired_suppressions,
    );
    let files_delta = metric_delta(before.metrics.files_analysed, after.metrics.files_analysed);

    let total_delta = viol_delta.delta + ap_delta.delta;
    let overall = match total_delta.cmp(&0) {
        Ordering::Less => "improving",
        Ordering::Greater => "degrading",
        Ordering::Equal => "stable",
    };

    // Compute per-item added/removed by diffing IDs.
    let before_viol_ids: std::collections::BTreeSet<&str> =
        before.violations.iter().map(|v| v.id.as_str()).collect();
    let after_viol_ids: std::collections::BTreeSet<&str> =
        after.violations.iter().map(|v| v.id.as_str()).collect();
    let viols_added = after_viol_ids.difference(&before_viol_ids).count();
    let viols_removed = before_viol_ids.difference(&after_viol_ids).count();

    let before_ap_ids: std::collections::BTreeSet<String> = before
        .antipatterns
        .iter()
        .map(|a| format!("{}:{}:{}", a.file, a.line, a.id))
        .collect();
    let after_ap_ids: std::collections::BTreeSet<String> = after
        .antipatterns
        .iter()
        .map(|a| format!("{}:{}:{}", a.file, a.line, a.id))
        .collect();
    let aps_added = after_ap_ids.difference(&before_ap_ids).count();
    let aps_removed = before_ap_ids.difference(&after_ap_ids).count();

    ComparisonOutput {
        before: SnapshotRef {
            name: before.name.clone(),
            created_at: before.created_at.clone(),
        },
        after: SnapshotRef {
            name: after.name.clone(),
            created_at: after.created_at.clone(),
        },
        duration_days: duration,
        metrics: ComparisonMetrics {
            boundary_violations: viol_delta,
            antipattern_count: ap_delta,
            suppression_count: sup_delta,
            expired_suppressions: exp_delta,
            files_analysed: files_delta,
        },
        net_change: NetChange {
            violations: i64::try_from(after.metrics.boundary_violations).unwrap_or(i64::MAX)
                - i64::try_from(before.metrics.boundary_violations).unwrap_or(i64::MAX),
            antipatterns: i64::try_from(after.metrics.antipattern_count).unwrap_or(i64::MAX)
                - i64::try_from(before.metrics.antipattern_count).unwrap_or(i64::MAX),
            suppressions: i64::try_from(after.metrics.suppression_count).unwrap_or(i64::MAX)
                - i64::try_from(before.metrics.suppression_count).unwrap_or(i64::MAX),
        },
        violations: ChangeCounts {
            added: viols_added,
            removed: viols_removed,
        },
        antipatterns: ChangeCounts {
            added: aps_added,
            removed: aps_removed,
        },
        overall_trend: overall,
    }
}

fn metric_delta(before: usize, after: usize) -> MetricDelta {
    let delta =
        i64::try_from(after).unwrap_or(i64::MAX) - i64::try_from(before).unwrap_or(i64::MAX);
    let trend = match delta.cmp(&0) {
        Ordering::Less => "decreasing",
        Ordering::Greater => "increasing",
        Ordering::Equal => "stable",
    };
    MetricDelta {
        before,
        after,
        delta,
        trend,
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn collect_antipatterns(
    files: &[String],
    cwd: &Path,
) -> (
    Vec<SnapshotAntipattern>,
    Vec<SnapshotSuppression>,
    anvil_checks::antipattern::AntipatternCheckResult,
) {
    let config = AntipatternCheckConfig::default();
    let file_refs: Vec<&str> = files.iter().map(String::as_str).collect();
    let ap_result = run_antipattern_check(&file_refs, &config, Some(&cwd.to_string_lossy()));

    let antipatterns: Vec<SnapshotAntipattern> = ap_result
        .warnings
        .warnings
        .iter()
        .filter(|w| w.suppressed.is_none())
        .map(|w| SnapshotAntipattern {
            id: w.id.clone(),
            file: w.location.file.clone(),
            line: w.location.line,
            pattern: w.pattern.clone().unwrap_or_default(),
            severity: format!("{:?}", w.severity).to_lowercase(),
        })
        .collect();

    let suppressions: Vec<SnapshotSuppression> = ap_result
        .warnings
        .warnings
        .iter()
        .filter(|w| w.suppressed.is_some())
        .map(|w| {
            let sup = w.suppressed.as_ref().unwrap();
            SnapshotSuppression {
                id: format!("{}:{}:{}", w.location.file, w.location.line, w.id),
                pattern_id: w.id.clone(),
                file: w.location.file.clone(),
                line: w.location.line,
                reason: sup.reason.clone(),
                scope: serde_json::to_value(sup.scope)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_else(|| "line".to_string()),
            }
        })
        .collect();

    (antipatterns, suppressions, ap_result)
}

/// Human-readable rule id + the move-resistant fingerprint for a destructive
/// SURFSQL finding. The fingerprint is computed by the scanner over the *full*
/// (untruncated) normalised statement, so the drift snapshot writer
/// (`collect_sql_findings`) and the gate reader (`run_check_sql_migrations`)
/// agree without re-deriving it from the truncated display statement
/// (SURFSQL-006).
pub(crate) fn destructive_finding_id(
    f: &anvil_checks::surface::sql::SqlFinding,
) -> (String, String) {
    (
        format!("surfsql-destructive:{:?}", f.kind),
        f.fingerprint.clone(),
    )
}

/// As [`destructive_finding_id`] for a schema-hygiene finding.
pub(crate) fn hygiene_finding_id(
    f: &anvil_checks::surface::sql::SqlHygieneFinding,
) -> (String, String) {
    (
        format!("surfsql-hygiene:{:?}", f.kind),
        f.fingerprint.clone(),
    )
}

/// Discover SQL migration files under `workspace` and collect their
/// unsuppressed SURFSQL findings as baseline entries (SURFSQL-006). Mirrors
/// `get_source_files`' walk shape but selects `.sql` migration files via the
/// surface's own `is_sql_migration_file`. Suppressed findings are dropped —
/// an author's explicit `--` acknowledgement disqualifies them from the
/// baseline, exactly as antipattern suppressions are dropped above.
fn collect_sql_findings(workspace: &Path) -> Vec<SnapshotSqlFinding> {
    use anvil_checks::surface::sql::{is_sql_migration_file, run_surfsql_check};

    let walker = ignore::WalkBuilder::new(workspace)
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .filter_entry(|e| {
            if e.file_type().is_some_and(|ft| ft.is_dir()) {
                !is_ignored_dir_name(&e.file_name().to_string_lossy())
            } else {
                true
            }
        })
        .build();

    let mut sql_files: Vec<(PathBuf, String)> = walker
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
        .filter(|e| is_sql_migration_file(e.path()))
        .filter_map(|e| {
            std::fs::read_to_string(e.path())
                .ok()
                .map(|content| (e.path().to_path_buf(), content))
        })
        .collect();
    sql_files.sort_by(|a, b| a.0.cmp(&b.0)); // Deterministic snapshot ordering.

    let result = run_surfsql_check(&sql_files);
    let mut findings: Vec<SnapshotSqlFinding> = Vec::new();
    for f in result.destructive.iter().filter(|f| !f.suppressed) {
        let (rule_id, fingerprint) = destructive_finding_id(f);
        findings.push(SnapshotSqlFinding {
            fingerprint,
            rule_id,
            file: f.file.clone(),
            line: f.line,
        });
    }
    for f in result.hygiene.iter().filter(|f| !f.suppressed) {
        let (rule_id, fingerprint) = hygiene_finding_id(f);
        findings.push(SnapshotSqlFinding {
            fingerprint,
            rule_id,
            file: f.file.clone(),
            line: f.line,
        });
    }
    findings
}

// SCAN-001: drift discovery uses `ignore::WalkBuilder` to share the
// noise-pruning walk shape (skips target/, node_modules/, etc) with the
// welcome flow. `.gitignore` is intentionally NOT honoured — drift
// snapshots must see every file regardless of VCS state — and the
// `.standard_filters(false)` setting reflects that. Per-file scans are
// already parallelised inside `run_antipattern_check`
// (`files.par_iter()` in `anvil-checks::antipattern::check`), so we
// don't need a second rayon fan-out here — only the discovery layer
// needed swapping. Files are sorted post-collect for deterministic
// snapshot ordering.
//
// `Result` return is retained even though the body cannot currently fail —
// callers expect the signature, and future fallible discovery (e.g.
// permission errors surfacing through `ignore::WalkBuilder` once we stop
// silently swallowing them) will use it.
#[allow(clippy::unnecessary_wraps)]
fn get_source_files(workspace: &Path) -> Result<Vec<String>> {
    let extensions = AntipatternCheckConfig::default().extensions;

    let walker = ignore::WalkBuilder::new(workspace)
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .filter_entry(|e| {
            if e.file_type().is_some_and(|ft| ft.is_dir()) {
                let name = e.file_name().to_string_lossy();
                !is_ignored_dir_name(&name)
            } else {
                true
            }
        })
        .build();

    let mut files: Vec<String> = walker
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
        .filter_map(|e| {
            let path_str = e.path().to_string_lossy().to_string();
            extensions
                .iter()
                .any(|ext| path_str.ends_with(ext.as_str()))
                .then_some(path_str)
        })
        .collect();

    files.sort();
    Ok(files)
}

/// Read the `created_at` field from a snapshot file. Returns `None`
/// on any I/O or parse error — including a baseline written by a newer
/// schema than this binary understands — so the caller falls back to
/// filename ordering and never sorts on an unguarded future baseline.
fn read_created_at(path: &Path) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    let content = read_to_string_capped(path, MAX_SNAPSHOT_BYTES).ok()?;
    let snap: DriftSnapshot = serde_json::from_str(&content).ok()?;
    let baseline = SchemaVersion::parse(&snap.schema_version).ok()?;
    ensure_readable(baseline, current_schema()).ok()?;
    chrono::DateTime::parse_from_rfc3339(&snap.created_at).ok()
}

fn get_git_ref() -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn trend_icon(trend: &str) -> &str {
    match trend {
        "decreasing" | "improving" => "\u{2713}",
        "increasing" | "degrading" => "\u{26a0}",
        _ => "\u{2500}",
    }
}

fn print_comparison(comparison: &ComparisonOutput, before: &DriftSnapshot, after: &DriftSnapshot) {
    let before_name = before.name.as_deref().unwrap_or("(unnamed)");
    let after_name = after.name.as_deref().unwrap_or("(unnamed)");

    output::plain::blank();
    output::plain::section(&format!("{before_name} \u{2192} {after_name}"));
    output::plain::label("Duration", format!("{:.1} days", comparison.duration_days));
    output::plain::label("Trend", comparison.overall_trend);
    output::plain::blank();

    let m = &comparison.metrics;
    output::plain::section("Metrics");
    print_metric_row("Violations", &m.boundary_violations);
    print_metric_row("Anti-patterns", &m.antipattern_count);
    print_metric_row("Suppressions", &m.suppression_count);
    print_metric_row("Files", &m.files_analysed);
}

fn print_metric_row(label: &str, delta: &MetricDelta) {
    let sign = if delta.delta > 0 { "+" } else { "" };
    let icon = trend_icon(delta.trend);
    output::plain::item(
        icon,
        &format!(
            "{label:<16} {before} \u{2192} {after} ({sign}{delta})",
            before = delta.before,
            after = delta.after,
            delta = delta.delta,
        ),
    );
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Sanitise ────────────────────────────────────────────────

    #[test]
    fn sanitise_name_replaces_special_chars() {
        assert_eq!(sanitise_name("release-1.0"), "release-1-0");
        assert_eq!(sanitise_name("my snapshot!"), "my-snapshot-");
    }

    #[test]
    fn sanitise_name_preserves_valid_chars() {
        assert_eq!(sanitise_name("release-1_0"), "release-1_0");
    }

    // ── Metric delta ────────────────────────────────────────────

    #[test]
    fn metric_delta_improving() {
        let d = metric_delta(10, 5);
        assert_eq!(d.delta, -5);
        assert_eq!(d.trend, "decreasing");
    }

    #[test]
    fn metric_delta_degrading() {
        let d = metric_delta(5, 10);
        assert_eq!(d.delta, 5);
        assert_eq!(d.trend, "increasing");
    }

    #[test]
    fn metric_delta_stable() {
        let d = metric_delta(5, 5);
        assert_eq!(d.delta, 0);
        assert_eq!(d.trend, "stable");
    }

    // ── Comparison ──────────────────────────────────────────────

    fn make_snapshot(name: &str, violations: usize, aps: usize, sups: usize) -> DriftSnapshot {
        DriftSnapshot {
            schema_version: current_schema().to_string(),
            created_at: "2025-01-15T14:30:00+00:00".to_string(),
            name: Some(name.to_string()),
            metrics: SnapshotMetrics {
                boundary_violations: violations,
                antipattern_count: aps,
                suppression_count: sups,
                expired_suppressions: 0,
                files_analysed: 100,
            },
            antipattern_breakdown: None,
            violations: Vec::new(),
            antipatterns: Vec::new(),
            suppressions: Vec::new(),
            sql_findings: Vec::new(),
            git_ref: None,
        }
    }

    #[test]
    fn compare_improving() {
        let before = make_snapshot("before", 10, 20, 3);
        let after = make_snapshot("after", 5, 15, 2);
        let c = compare_snapshots(&before, &after);
        assert_eq!(c.overall_trend, "improving");
        assert_eq!(c.net_change.violations, -5);
        assert_eq!(c.net_change.antipatterns, -5);
    }

    #[test]
    fn compare_degrading() {
        let before = make_snapshot("before", 5, 10, 1);
        let after = make_snapshot("after", 10, 20, 3);
        let c = compare_snapshots(&before, &after);
        assert_eq!(c.overall_trend, "degrading");
    }

    #[test]
    fn compare_stable() {
        let before = make_snapshot("before", 5, 10, 1);
        let after = make_snapshot("after", 5, 10, 1);
        let c = compare_snapshots(&before, &after);
        assert_eq!(c.overall_trend, "stable");
    }

    // ── Snapshot serialisation round-trip ────────────────────────

    #[test]
    fn snapshot_round_trip() {
        let snap = make_snapshot("test", 3, 7, 1);
        let json = serde_json::to_string(&snap).unwrap();
        let parsed: DriftSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, Some("test".to_string()));
        assert_eq!(parsed.metrics.boundary_violations, 3);
        assert_eq!(parsed.metrics.antipattern_count, 7);
    }

    // ── SURFSQL-006 drift baseline ───────────────────────────────

    #[test]
    fn finding_id_passes_through_scanner_fingerprint_and_labels_rule() {
        use anvil_checks::surface::sql::run_surfsql_check;
        let scan = run_surfsql_check(&[(
            std::path::PathBuf::from("0001.sql"),
            "DROP TABLE users;".to_string(),
        )]);
        let f = &scan.destructive[0];
        let (rule_id, fp) = destructive_finding_id(f);
        assert_eq!(rule_id, format!("surfsql-destructive:{:?}", f.kind));
        assert_eq!(
            fp, f.fingerprint,
            "fingerprint comes straight from the scanner"
        );
        assert_eq!(fp.len(), 16, "16-hex digest");
    }

    #[test]
    fn sql_findings_survive_snapshot_round_trip() {
        let mut snap = make_snapshot("sql", 0, 0, 0);
        snap.sql_findings = vec![SnapshotSqlFinding {
            fingerprint: "deadbeefdeadbeef".to_string(),
            rule_id: "surfsql-destructive:DropTable".to_string(),
            file: "db/0001.sql".to_string(),
            line: 12,
        }];
        let json = serde_json::to_string(&snap).unwrap();
        let parsed: DriftSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sql_findings, snap.sql_findings);
    }

    #[test]
    fn v1_0_0_baseline_without_sql_field_still_reads() {
        // A pre-SURFSQL (v1.0.0) snapshot has no `sql_findings` key; `default`
        // must keep it loadable as an empty baseline (additive schema).
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(snapshots_dir(dir.path())).unwrap();
        let legacy = r#"{
  "schema_version": "1.0.0",
  "created_at": "2026-01-01T00:00:00+00:00",
  "name": "legacy",
  "metrics": {"boundary_violations":0,"antipattern_count":0,"suppression_count":0,"expired_suppressions":0,"files_analysed":0},
  "violations": [],
  "antipatterns": [],
  "suppressions": []
}"#;
        let path = snapshots_dir(dir.path()).join("snapshot-legacy.json");
        std::fs::write(&path, legacy).unwrap();
        let loaded = load_snapshot_file(&path).unwrap();
        assert!(loaded.sql_findings.is_empty());
        // A v1.0.0 snapshot still counts as a baseline that exists (Some), just
        // with an empty SQL set — so the gate does not claim "no baseline".
        assert_eq!(
            latest_sql_baseline_fingerprints(dir.path()),
            Some(BTreeSet::new())
        );
    }

    #[test]
    fn latest_sql_baseline_returns_snapshot_fingerprints() {
        let dir = tempfile::tempdir().unwrap();
        // No snapshot at all → None (gate falls back to warn-on-all + hint).
        assert_eq!(latest_sql_baseline_fingerprints(dir.path()), None);

        let mut snap = make_snapshot("base", 0, 0, 0);
        snap.sql_findings = vec![SnapshotSqlFinding {
            fingerprint: "abc123abc123abc1".to_string(),
            rule_id: "surfsql-destructive:DropTable".to_string(),
            file: "db/0001.sql".to_string(),
            line: 1,
        }];
        save_snapshot(dir.path(), &snap, Some("base")).unwrap();

        let baseline = latest_sql_baseline_fingerprints(dir.path()).expect("snapshot exists");
        assert!(baseline.contains("abc123abc123abc1"));
    }

    // ── Schema versioning (OPSUP-003) ────────────────────────────

    #[test]
    fn schema_version_parses_and_orders() {
        assert_eq!(
            SchemaVersion::parse("1.2.3").unwrap(),
            SchemaVersion::new(1, 2, 3)
        );
        assert!(SchemaVersion::new(1, 1, 0) > SchemaVersion::new(1, 0, 9));
        assert!(SchemaVersion::new(2, 0, 0) > SchemaVersion::new(1, 9, 9));
        assert!(SchemaVersion::parse("1.0").is_err());
        assert!(SchemaVersion::parse("1.0.0.0").is_err());
        assert!(SchemaVersion::parse("1.x.0").is_err());
    }

    /// A current-version baseline survives a save → load → re-serialise
    /// cycle byte-for-byte, and carries the derived schema version.
    #[test]
    fn current_version_baseline_round_trips_byte_stable() {
        let dir = tempfile::tempdir().unwrap();
        let snap = make_snapshot("rt", 1, 2, 0);
        assert_eq!(snap.schema_version, current_schema().to_string());

        let filename = save_snapshot(dir.path(), &snap, Some("rt")).unwrap();
        let path = snapshots_dir(dir.path()).join(&filename);

        let on_disk = std::fs::read_to_string(&path).unwrap();
        let loaded = load_snapshot_file(&path).unwrap();
        let reserialised = serde_json::to_string_pretty(&loaded).unwrap();
        assert_eq!(
            on_disk, reserialised,
            "loaded baseline must re-serialise byte-stable"
        );
    }

    /// A baseline written by a newer schema is refused with an upgrade
    /// message rather than silently loaded with its newer fields dropped.
    #[test]
    fn future_version_baseline_fails_with_upgrade_message() {
        let dir = tempfile::tempdir().unwrap();
        let mut snap = make_snapshot("future", 0, 0, 0);
        let future = SchemaVersion::new(current_schema().major + 1, 0, 0);
        snap.schema_version = future.to_string();

        let path = snapshots_dir(dir.path()).join("snapshot-future.json");
        std::fs::create_dir_all(snapshots_dir(dir.path())).unwrap();
        std::fs::write(&path, serde_json::to_string_pretty(&snap).unwrap()).unwrap();

        let err = load_snapshot_file(&path).unwrap_err();
        let chain = format!("{err:#}");
        assert!(
            chain.contains("upgrade"),
            "expected an upgrade hint, got: {chain}"
        );
    }

    // ── Migration (OPSUP-004) ────────────────────────────────────

    /// Write a snapshot file at an explicit schema version into a workspace's
    /// snapshots dir, returning its path.
    fn write_snapshot_at_version(workspace: &Path, name: &str, version: &str) -> PathBuf {
        let mut snap = make_snapshot(name, 1, 2, 0);
        snap.schema_version = version.to_string();
        let dir = snapshots_dir(workspace);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("snapshot-{name}.json"));
        std::fs::write(&path, serde_json::to_string_pretty(&snap).unwrap()).unwrap();
        path
    }

    fn force_migrate_write_failure(snapshots: &Path, snapshot: &Path) {
        #[cfg(windows)]
        {
            let _ = snapshots;
            let mut perms = std::fs::metadata(snapshot).unwrap().permissions();
            perms.set_readonly(true);
            std::fs::set_permissions(snapshot, perms).unwrap();
        }

        #[cfg(not(windows))]
        {
            let _ = snapshot;
            let mut perms = std::fs::metadata(snapshots).unwrap().permissions();
            perms.set_readonly(true);
            std::fs::set_permissions(snapshots, perms).unwrap();
        }
    }

    #[test]
    fn migrate_upgrades_older_baseline_and_writes_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_snapshot_at_version(dir.path(), "old", "1.0.0");
        let original = std::fs::read_to_string(&path).unwrap();

        let report = migrate_snapshots(dir.path(), false).unwrap();

        assert_eq!(report.migrated, 1);
        assert_eq!(report.already_current, 0);

        // Backup retains the original bytes before the in-place write.
        let backup = backup_path(&path);
        assert!(backup.exists(), "backup must be written");
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), original);

        // The upgraded file now carries the current schema version.
        let upgraded = load_snapshot_file(&path).unwrap();
        assert_eq!(upgraded.schema_version, current_schema().to_string());
    }

    #[test]
    fn migrate_current_baseline_is_a_noop() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_snapshot_at_version(dir.path(), "cur", &current_schema().to_string());

        let report = migrate_snapshots(dir.path(), false).unwrap();

        assert_eq!(report.migrated, 0);
        assert_eq!(report.already_current, 1);
        assert!(
            !backup_path(&path).exists(),
            "an already-current baseline must not be backed up or rewritten"
        );
    }

    #[test]
    fn migrate_preserves_unknown_fields() {
        // Additive schema evolution must be lossless: a field not in the
        // current `DriftSnapshot` struct (a user annotation, a future-patch
        // field) survives migration rather than being dropped by a struct
        // round-trip.
        let dir = tempfile::tempdir().unwrap();
        let mut snap = make_snapshot("annotated", 1, 2, 0);
        snap.schema_version = "1.0.0".to_string();
        let mut value = serde_json::to_value(&snap).unwrap();
        value["ci_run_id"] = serde_json::Value::String("run-42".to_string());
        let sdir = snapshots_dir(dir.path());
        std::fs::create_dir_all(&sdir).unwrap();
        let path = sdir.join("snapshot-annotated.json");
        std::fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();

        migrate_snapshots(dir.path(), false).unwrap();

        let migrated: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(migrated["schema_version"], current_schema().to_string());
        assert_eq!(
            migrated["ci_run_id"], "run-42",
            "unknown field must be preserved through migration"
        );
    }

    #[test]
    fn migrate_never_clobbers_an_existing_backup_and_still_backs_up_the_original() {
        // The backup is the spec's one-release rollback copy. With a `.bak`
        // already present, migration must (a) NOT overwrite it, and (b) STILL
        // back up the current pre-migration content to a fresh path — skipping
        // the backup would lose the original.
        let dir = tempfile::tempdir().unwrap();
        let path = write_snapshot_at_version(dir.path(), "old", "1.0.0");
        let original = std::fs::read_to_string(&path).unwrap();
        let base_backup = backup_path(&path);
        std::fs::write(&base_backup, b"PRIOR-BACKUP").unwrap();

        let report = migrate_snapshots(dir.path(), false).unwrap();

        assert_eq!(report.migrated, 1, "the baseline still migrates");
        assert_eq!(
            std::fs::read_to_string(&base_backup).unwrap(),
            "PRIOR-BACKUP",
            "an existing backup must never be clobbered"
        );
        // The pre-migration content was preserved to the next free backup.
        let fresh_backup = base_backup.with_file_name(format!(
            "{}.1",
            base_backup.file_name().unwrap().to_string_lossy()
        ));
        assert_eq!(
            std::fs::read_to_string(&fresh_backup).unwrap(),
            original,
            "the original must be backed up to a fresh path, never skipped"
        );
        assert_eq!(
            load_snapshot_file(&path).unwrap().schema_version,
            current_schema().to_string()
        );
    }

    #[test]
    fn write_fresh_backup_chains_past_existing_backups_without_clobbering() {
        // With `.bak` and `.bak.1` already present, the exclusive-create write
        // must land on `.bak.2` and leave both existing backups untouched.
        let dir = tempfile::tempdir().unwrap();
        let path = write_snapshot_at_version(dir.path(), "x", "1.0.0");
        let base = backup_path(&path);
        let bak1 =
            base.with_file_name(format!("{}.1", base.file_name().unwrap().to_string_lossy()));
        std::fs::write(&base, b"BAK0").unwrap();
        std::fs::write(&bak1, b"BAK1").unwrap();

        let written = write_fresh_backup(&path, b"FRESH").unwrap();

        assert_eq!(
            written.file_name().unwrap().to_string_lossy(),
            format!("{}.2", base.file_name().unwrap().to_string_lossy())
        );
        assert_eq!(std::fs::read_to_string(&base).unwrap(), "BAK0");
        assert_eq!(std::fs::read_to_string(&bak1).unwrap(), "BAK1");
        assert_eq!(std::fs::read_to_string(&written).unwrap(), "FRESH");
    }

    #[test]
    fn migrate_changes_only_the_schema_version_field() {
        // `preserve_order` (workspace-wide) keeps the Value round-trip stable,
        // so a migrated baseline differs from the original by exactly the
        // schema_version value — no field reordering or drops.
        let dir = tempfile::tempdir().unwrap();
        let path = write_snapshot_at_version(dir.path(), "old", "1.0.0");
        let mut before: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

        migrate_snapshots(dir.path(), false).unwrap();

        let after: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        // Normalise the one field that is meant to change, then require equality.
        before["schema_version"] = serde_json::Value::String(current_schema().to_string());
        assert_eq!(
            before, after,
            "migration must change only schema_version, preserving every other field"
        );
    }

    #[test]
    fn backup_files_are_not_picked_up_as_snapshots() {
        // Guards the load-bearing invariant that `<file>.json.bak` is excluded
        // from snapshot discovery — otherwise a migrate run would re-migrate
        // its own backups.
        let dir = tempfile::tempdir().unwrap();
        let path = write_snapshot_at_version(dir.path(), "x", &current_schema().to_string());
        std::fs::write(backup_path(&path), b"{}").unwrap();

        let files = list_snapshot_files(dir.path()).unwrap();
        assert!(
            files
                .iter()
                .all(|p| p.extension().and_then(|e| e.to_str()) == Some("json")),
            "no .bak file should be discovered as a snapshot, got: {files:?}"
        );
    }

    #[test]
    fn migrate_skips_future_baseline_without_downgrading() {
        let dir = tempfile::tempdir().unwrap();
        let future = SchemaVersion::new(current_schema().major + 1, 0, 0);
        let path = write_snapshot_at_version(dir.path(), "future", &future.to_string());
        let original = std::fs::read_to_string(&path).unwrap();

        let report = migrate_snapshots(dir.path(), false).unwrap();

        assert_eq!(report.migrated, 0);
        assert_eq!(report.newer, 1);
        assert_eq!(report.skipped, 1);
        assert!(report.partial);
        assert_eq!(
            report
                .skipped_by_reason
                .get(&MigrateSkipReason::NewerSchema),
            Some(&1)
        );
        // A future baseline is left untouched — never downgraded.
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert!(!backup_path(&path).exists());
    }

    #[test]
    fn migrate_counts_corrupt_invalid_schema_and_unreadable_snapshots_as_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let snapshots = snapshots_dir(dir.path());
        std::fs::create_dir_all(&snapshots).unwrap();
        std::fs::write(snapshots.join("snapshot-corrupt.json"), b"{not-json").unwrap();

        let mut bad_schema = serde_json::to_value(make_snapshot("bad-schema", 0, 0, 0)).unwrap();
        bad_schema["schema_version"] = serde_json::Value::String("not-semver".to_string());
        std::fs::write(
            snapshots.join("snapshot-bad-schema.json"),
            serde_json::to_string_pretty(&bad_schema).unwrap(),
        )
        .unwrap();
        std::fs::create_dir(snapshots.join("snapshot-unreadable.json")).unwrap();

        let report = migrate_snapshots(dir.path(), false).unwrap();

        assert_eq!(report.skipped, 3, "{report:?}");
        assert!(report.partial);
        assert_eq!(
            report
                .skipped_by_reason
                .get(&MigrateSkipReason::InvalidJson),
            Some(&1)
        );
        assert_eq!(
            report
                .skipped_by_reason
                .get(&MigrateSkipReason::InvalidSchemaVersion),
            Some(&1)
        );
        assert_eq!(
            report.skipped_by_reason.get(&MigrateSkipReason::Unreadable),
            Some(&1)
        );
    }

    #[test]
    fn migrate_write_failure_is_reported_as_partial_without_aborting() {
        let dir = tempfile::tempdir().unwrap();
        let good = write_snapshot_at_version(dir.path(), "good", "1.0.0");
        let first = migrate_snapshots(dir.path(), false).unwrap();
        assert_eq!(first.migrated, 1);

        let late = write_snapshot_at_version(dir.path(), "late", "1.0.0");
        let late_before = std::fs::read_to_string(&late).unwrap();
        let snapshots = snapshots_dir(dir.path());
        force_migrate_write_failure(&snapshots, &late);

        let report = migrate_snapshots(dir.path(), false).unwrap();

        assert_eq!(report.migrated, 0);
        assert_eq!(report.skipped, 1);
        assert!(report.partial);
        assert_eq!(
            report
                .skipped_by_reason
                .get(&MigrateSkipReason::WriteFailed),
            Some(&1)
        );
        assert_eq!(
            load_snapshot_file(&good).unwrap().schema_version,
            current_schema().to_string()
        );
        assert_eq!(std::fs::read_to_string(&late).unwrap(), late_before);
    }

    #[test]
    fn migrate_scan_cap_is_reported_as_partial() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["a", "b", "c"] {
            write_snapshot_at_version(dir.path(), name, &current_schema().to_string());
        }

        let report = migrate_snapshots_capped(dir.path(), false, 2).unwrap();

        assert_eq!(report.skipped, 1);
        assert!(report.partial);
        assert_eq!(
            report
                .skipped_by_reason
                .get(&MigrateSkipReason::ScanLimitExceeded),
            Some(&1)
        );
    }

    #[test]
    fn prune_backups_keeps_latest_generation_and_ignores_unrelated_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_snapshot_at_version(dir.path(), "prune", &current_schema().to_string());
        let base = backup_path(&path);
        let bak1 =
            base.with_file_name(format!("{}.1", base.file_name().unwrap().to_string_lossy()));
        let bak2 =
            base.with_file_name(format!("{}.2", base.file_name().unwrap().to_string_lossy()));
        std::fs::write(&base, b"bak0").unwrap();
        std::fs::write(&bak1, b"bak1").unwrap();
        std::fs::write(&bak2, b"bak2").unwrap();
        std::fs::write(base.with_file_name("snapshot-prune.json.bak.tmp"), b"tmp").unwrap();
        std::fs::write(base.with_file_name("snapshot-prune.json.back"), b"back").unwrap();
        std::fs::write(base.with_file_name("snapshot-prune.bak"), b"wrong").unwrap();

        let report = prune_drift_backups(dir.path()).unwrap();

        assert_eq!(report.pruned, 2);
        assert_eq!(report.retained, 1);
        assert!(path.exists(), "live baseline must never be pruned");
        assert!(!base.exists());
        assert!(!bak1.exists());
        assert_eq!(std::fs::read_to_string(&bak2).unwrap(), "bak2");
        assert!(base.with_file_name("snapshot-prune.json.bak.tmp").exists());
        assert!(base.with_file_name("snapshot-prune.json.back").exists());
        assert!(base.with_file_name("snapshot-prune.bak").exists());
    }

    #[test]
    fn prune_then_migrate_keeps_current_run_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_snapshot_at_version(dir.path(), "old-prune", "1.0.0");
        let base = backup_path(&path);
        std::fs::write(&base, b"old-backup").unwrap();

        let report = migrate_snapshots(dir.path(), true).unwrap();

        assert_eq!(report.backups.retained, 1);
        assert_eq!(report.migrated, 1);
        let fresh =
            base.with_file_name(format!("{}.1", base.file_name().unwrap().to_string_lossy()));
        assert!(
            fresh.exists(),
            "fresh rollback from this migration must survive prune"
        );
    }

    #[test]
    fn outdated_baseline_emits_migrate_hint_but_current_does_not() {
        let mut old = make_snapshot("old", 0, 0, 0);
        old.schema_version = "1.0.0".to_string();
        let hint = outdated_schema_hint(&old).expect("older baseline should hint");
        assert!(
            hint.contains("anvil drift migrate"),
            "hint must point at the migrate command: {hint}"
        );

        let current = make_snapshot("cur", 0, 0, 0);
        assert!(
            outdated_schema_hint(&current).is_none(),
            "a current baseline must not emit a migrate hint"
        );
    }

    /// Declaring a new surface field at a higher version advances the
    /// schema additively, and a baseline written at the older version is
    /// still readable under the advanced version.
    #[test]
    fn additive_surface_declaration_advances_version_without_breaking_old_reads() {
        let base = schema_version_for(FIELD_DECLARATIONS);

        let mut extended: Vec<FieldDeclaration> = Vec::new();
        for d in FIELD_DECLARATIONS {
            extended.push(FieldDeclaration {
                surface: d.surface,
                fields: d.fields,
                introduced_in: d.introduced_in,
            });
        }
        extended.push(FieldDeclaration {
            surface: "future-surface",
            fields: &["future_metric"],
            introduced_in: SchemaVersion::new(base.major, base.minor + 1, 0),
        });
        let advanced = schema_version_for(&extended);

        assert!(
            advanced > base,
            "an additive declaration must advance the version"
        );
        // An older baseline (written at `base`) remains readable once the
        // binary has advanced to `advanced`.
        assert!(ensure_readable(base, advanced).is_ok());
        // ...but a baseline at the advanced version is rejected by a binary
        // that only understands `base`.
        assert!(ensure_readable(advanced, base).is_err());
    }

    // ── Save/load round-trip ────────────────────────────────────

    #[test]
    fn save_and_load_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let snap = make_snapshot("test-save", 2, 4, 0);

        let filename = save_snapshot(dir.path(), &snap, Some("test-save")).unwrap();
        assert!(filename.starts_with(SNAPSHOT_PREFIX));
        assert!(
            std::path::Path::new(&filename)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        );

        let loaded = load_snapshot(dir.path(), "test-save").unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.name, Some("test-save".to_string()));
        assert_eq!(loaded.metrics.boundary_violations, 2);
    }

    #[test]
    fn load_snapshot_returns_none_for_missing() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = load_snapshot(dir.path(), "nonexistent").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn list_snapshots_sorted_by_created_at_descending() {
        let dir = tempfile::tempdir().unwrap();
        // "alpha" has a later created_at despite sorting earlier by filename.
        let mut snap_a = make_snapshot("alpha", 1, 1, 0);
        snap_a.created_at = "2025-06-01T00:00:00+00:00".to_string();
        let mut snap_b = make_snapshot("beta", 2, 2, 0);
        snap_b.created_at = "2025-01-01T00:00:00+00:00".to_string();

        save_snapshot(dir.path(), &snap_a, Some("alpha")).unwrap();
        save_snapshot(dir.path(), &snap_b, Some("beta")).unwrap();

        let entries = list_snapshots(dir.path()).unwrap();
        assert_eq!(entries.len(), 2);
        // alpha is newer by created_at, so it should appear first.
        assert_eq!(entries[0].name, Some("alpha".to_string()));
        assert_eq!(entries[1].name, Some("beta".to_string()));
    }

    #[test]
    fn save_snapshot_rejects_duplicate_name() {
        let dir = tempfile::tempdir().unwrap();
        let snap = make_snapshot("dup", 1, 1, 0);

        save_snapshot(dir.path(), &snap, Some("release")).unwrap();
        let err = save_snapshot(dir.path(), &snap, Some("release"));
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn save_snapshot_rejects_empty_name() {
        let dir = tempfile::tempdir().unwrap();
        let snap = make_snapshot("bad", 1, 1, 0);
        let err = save_snapshot(dir.path(), &snap, Some("!!!"));
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("alphanumeric"));
    }

    #[test]
    fn load_snapshot_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        // Create the snapshots dir so canonicalize works.
        std::fs::create_dir_all(snapshots_dir(dir.path())).unwrap();
        let result = load_snapshot(dir.path(), "../../etc/passwd");
        // Should either be None (file doesn't exist) or an error (path escape).
        match result {
            Ok(None) | Err(_) => {} // file doesn't exist or path escape caught — safe
            Ok(Some(_)) => panic!("should not load file outside snapshots dir"),
        }
    }

    #[test]
    fn list_snapshot_files_keeps_the_most_recent_when_capped() {
        use std::time::{Duration, SystemTime};
        // CIB-084: over the cap, the most-recent snapshots (by mtime) are kept;
        // below the cap, every snapshot is returned.
        let dir = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        for (name, age_secs) in [("oldest", 3000u64), ("middle", 2000), ("newest", 1000)] {
            let snap = make_snapshot(name, 1, 1, 0);
            let filename = save_snapshot(dir.path(), &snap, Some(name)).unwrap();
            let path = snapshots_dir(dir.path()).join(filename);
            std::fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(now - Duration::from_secs(age_secs))
                .unwrap();
        }

        let kept = list_snapshot_files_capped(dir.path(), 2).unwrap();
        let names: Vec<String> = kept
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(kept.len(), 2, "{names:?}");
        assert!(names.iter().any(|n| n.contains("newest")), "{names:?}");
        assert!(names.iter().any(|n| n.contains("middle")), "{names:?}");
        assert!(!names.iter().any(|n| n.contains("oldest")), "{names:?}");

        // Below the cap, all three are returned.
        assert_eq!(list_snapshot_files_capped(dir.path(), 10).unwrap().len(), 3);
    }

    // ── Clap parsing ────────────────────────────────────────────

    #[test]
    fn clap_parses_drift_snapshot() {
        use clap::Parser;
        let result = crate::Cli::try_parse_from(["anvil", "drift", "snapshot"]);
        assert!(result.is_ok());
    }

    #[test]
    fn clap_parses_drift_compare() {
        use clap::Parser;
        let result = crate::Cli::try_parse_from(["anvil", "drift", "compare", "s1", "s2"]);
        assert!(result.is_ok());
    }

    #[test]
    fn clap_parses_drift_list_with_limit() {
        use clap::Parser;
        let result = crate::Cli::try_parse_from(["anvil", "drift", "list", "--limit", "5"]);
        assert!(result.is_ok());
    }

    #[test]
    fn clap_parses_drift_report_with_since() {
        use clap::Parser;
        let result =
            crate::Cli::try_parse_from(["anvil", "drift", "report", "--since", "release-1"]);
        assert!(result.is_ok());
    }
}
