use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::Args;
use serde::{Deserialize, Serialize};

use anvil_architecture::load_baseline;
use anvil_checks::antipattern::{AntipatternCheckConfig, run_antipattern_check};

use crate::GlobalArgs;
use crate::output::{self, OutputMode};
use crate::util::is_ignored_dir_name;

const SNAPSHOTS_DIR: &str = "snapshots";
const ANVIL_DIR: &str = ".anvil";
const SNAPSHOT_PREFIX: &str = "snapshot-";

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
        let mut parts = s.split('.');
        let mut next = |label: &str| -> Result<u32> {
            parts
                .next()
                .and_then(|p| p.parse::<u32>().ok())
                .ok_or_else(|| {
                    anyhow::anyhow!("schema version '{s}' has no valid {label} component")
                })
        };
        let major = next("major")?;
        let minor = next("minor")?;
        let patch = next("patch")?;
        if parts.next().is_some() {
            bail!("schema version '{s}' has too many components (expected major.minor.patch)");
        }
        Ok(Self::new(major, minor, patch))
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
        surface: "metrics",
        fields: &[
            "boundary_violations",
            "antipattern_count",
            "suppression_count",
            "expired_suppressions",
            "files_analysed",
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
struct ComparisonOutput {
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
    let content = std::fs::read_to_string(path)
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
    let dir = snapshots_dir(workspace);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let files: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|e| e == "json")
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with(SNAPSHOT_PREFIX))
        })
        .collect();

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

    Ok(files)
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

fn compare_snapshots(before: &DriftSnapshot, after: &DriftSnapshot) -> ComparisonOutput {
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
    let content = std::fs::read_to_string(path).ok()?;
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
