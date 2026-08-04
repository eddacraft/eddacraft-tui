use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anvil_kernel_types::{
    Category, Diagnostic, DiagnosticSource, GateSnapshot, GateSnapshotWarning, Location, Mode,
    Notification, NotificationClass, NotificationContext, NotificationPriority, Severity,
    diagnostics::KnownMode,
};
use anvil_policy_engine::{Engine, EngineConfig, PolicyInput};
use anyhow::{Context, Result, bail};
use clap::Args;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::GlobalArgs;
use crate::commands::check_catalog::{
    GATE_INTERNAL_CHECKS, canonical_check_name, closest_registered_id, definition_by_internal,
    gate_canonical_name_from_internal, gate_canonical_names, gate_internal_name,
};
use crate::commands::check_guards::{WallTimeGuard, evaluate_file_presence, evaluate_wall_time};
use crate::util::is_ignored_dir_name;

#[derive(Debug, Default, Args)]
#[allow(clippy::struct_excessive_bools)]
pub struct GateArgs {
    /// Plan file to run gates against (omit for full codebase scan)
    plan: Option<String>,

    /// Gate profile: dev, ci, production, ai
    #[arg(long, short)]
    profile: Option<String>,

    /// Comma-separated list of checks to skip
    #[arg(long)]
    skip_checks: Option<String>,

    /// Only run specified checks (comma-separated)
    #[arg(long)]
    only_checks: Option<String>,

    /// Stop on first check failure
    #[arg(long)]
    fail_fast: bool,

    /// Treat warning-severity anti-pattern findings as blocking (exit non-zero).
    /// Off by default (warnings do not block); also settable via
    /// `ANVIL_FAIL_ON_WARNINGS`. Error-severity rules always block regardless.
    #[arg(long)]
    fail_on_warnings: bool,

    /// Show real-time progress
    #[arg(long)]
    progress: bool,

    /// List available gate profiles
    #[arg(long)]
    list_profiles: bool,

    /// Output format: auto (default), tui, plain, json, or sarif. `json` is the
    /// `--json` alias; `sarif` emits SARIF 2.1.0 and is never auto-selected.
    #[arg(long, value_enum)]
    format: Option<crate::output::Format>,
}

impl GateArgs {
    /// True when an explicit `--format json|sarif` requests structured
    /// output, so the pre-dispatch auth gate emits a JSON envelope rather
    /// than human text. (The AI-guardrail profile's implicit JSON default
    /// is resolved later in `resolve_gate_output_mode` and is intentionally
    /// out of scope for the pre-auth check.)
    pub(crate) fn wants_structured_output(&self) -> bool {
        self.format
            .is_some_and(crate::output::Format::is_structured)
    }
}

const PROFILES: &[(&str, &str, &[&str])] = &[
    (
        "dev",
        "Development mode \u{2014} skips coverage and dependency checks",
        &["coverage", "dependency"],
    ),
    ("ci", "CI mode \u{2014} runs all checks", &[]),
    (
        "production",
        "Production mode \u{2014} runs all checks with strict thresholds",
        &[],
    ),
    (
        "ai",
        "AI guardrail mode \u{2014} curated checks for AI-generated code",
        &["lint", "test", "coverage", "dependency"],
    ),
];

/// AI guardrail profile (AIGUARD-001 + AIGUARD-003).
///
/// Declares the curated rule set that the AI guardrail runs to validate
/// AI-generated changes. The profile bundles structural-governance checks
/// (architecture, policy, antipattern, secret detection, command safety)
/// into a single coherent set so external AI tools have a predictable
/// safety harness.
///
/// `--profile ai` is wired end-to-end as of AIGUARD-003: the gate
/// runner selects from [`AI_GUARDRAIL_CHECKS`] as an allow-list (not
/// the inverse skip list), `strict_config = true` converts the
/// "missing config, skipping" path into a blocking diagnostic for
/// architecture/policy/command-safety, `json_output_default = true`
/// pins JSON output for AI consumers unless the caller passes a
/// non-JSON output mode explicitly, and the JSON envelope uses the
/// canonical `anvil.diagnostic.v1` shape published by AIGUARD-002.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AiGuardrailProfile {
    /// Canonical check names included in the profile.
    pub checks: &'static [&'static str],
    /// When true, missing or invalid configuration is treated as a
    /// blocking diagnostic rather than a soft warning.
    pub strict_config: bool,
    /// When true, output defaults to structured JSON for AI consumption.
    pub json_output_default: bool,
}

/// Canonical check names included in the AI guardrail profile.
///
/// Selection rationale: every check here flags a structural concern that
/// AI-generated changes regularly trip over — secret leakage, antipatterns,
/// import-boundary violations, OPA policy breaches, and command-safety
/// rules. Lint/test/coverage/dependency are intentionally excluded:
/// they're language-toolchain concerns the host project already enforces
/// and they would push the profile past the 5s budget set out in the
/// AIGUARD acceptance criteria.
pub(crate) const AI_GUARDRAIL_CHECKS: &[&str] = &[
    "secret-detection",
    "import-boundaries",
    "antipattern-scan",
    "policy",
    "command-safety",
];

impl AiGuardrailProfile {
    /// Default AI guardrail profile.
    pub(crate) const DEFAULT: Self = Self {
        checks: AI_GUARDRAIL_CHECKS,
        strict_config: true,
        json_output_default: true,
    };

    /// Profile name as used on the CLI (`--profile ai`).
    pub(crate) const NAME: &'static str = "ai";
}

/// Return the canonical check names that make up the AI guardrail
/// profile. Used by the gate runner to filter checks when
/// `--profile ai` is selected.
pub(crate) fn ai_guardrail_profile_checks() -> &'static [&'static str] {
    AiGuardrailProfile::DEFAULT.checks
}

#[derive(Debug, Serialize)]
struct GateResult {
    overall: bool,
    score: f64,
    checks: Vec<CheckResult>,
    notifications: Vec<Notification>,
    duration_ms: u64,
}

#[derive(Debug, Default, Serialize)]
struct CheckResult {
    name: String,
    passed: bool,
    score: f64,
    message: String,
    /// CIB-011 / #1803 — true when the check is unavailable on this
    /// repo because its configuration is missing. Excluded from the
    /// gate score denominator and rendered as `CONFIG NEEDED` with a
    /// `next:` hint, rather than as a `FAIL`. Skipped from JSON output
    /// when false so the schema stays additive.
    // serde idiom: skip the field in JSON when false. Equivalent to
    // `if !*x` — the `std::ops::Not::not` path lets serde call the
    // free function without needing a custom `is_false` helper.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    requires_config: bool,
}

/// CIB-011 / #1803 — aggregation result for the gate render + envelope.
///
/// Config-gap checks (where the check could not run because its config
/// is absent under `--profile ai` strict mode) are excluded from the
/// score denominator: a fresh repo with three missing configs and no
/// actual failures must read as `2/2 available passed (100%)`, not
/// `1/5 passed (20%)`. The pre-CIB-011 grading was the most-cited
/// reason new users believed anvil was broken on first contact.
#[derive(Debug, Clone, Copy)]
struct GateAggregate {
    passed_count: usize,
    available_total: usize,
    config_gaps: usize,
    overall: bool,
    score: f64,
}

fn aggregate_gate_outcome(checks: &[CheckResult]) -> GateAggregate {
    let available: Vec<&CheckResult> = checks.iter().filter(|c| !c.requires_config).collect();
    let available_total = available.len();
    let passed_count = available.iter().filter(|c| c.passed).count();
    let config_gaps = checks.len() - available_total;
    let overall = available.iter().all(|c| c.passed);
    #[allow(clippy::cast_precision_loss)]
    let score = if available_total > 0 {
        (passed_count as f64 / available_total as f64) * 100.0
    } else {
        // No real checks ran — there is nothing to fail, so the gate
        // is vacuously green. The render layer surfaces the config
        // gaps alongside this so the user is not misled into thinking
        // a fully-passing 100% means "everything is checked".
        100.0
    };
    GateAggregate {
        passed_count,
        available_total,
        config_gaps,
        overall,
        score,
    }
}

/// Filename of the persisted last-gate-run snapshot under `.anvil/`.
const GATE_SNAPSHOT_FILE: &str = "gates.json";
const GATE_HISTORY_FILE: &str = "gate-history.ndjson";
const GATE_HISTORY_LOCK_FILE: &str = ".gate-history.lock";
const GATE_HISTORY_LINE_CAP: usize = 500;
const GATE_HISTORY_MAX_BYTES: usize = GATE_HISTORY_LINE_CAP * 2048;
// 2s absorbs contended CI runners (Windows exclusive locks report as OS
// error 33 rather than WouldBlock; macOS/Windows smoke co-schedules writers).
const GATE_HISTORY_LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
const GATE_HISTORY_LOCK_RETRY: std::time::Duration = std::time::Duration::from_millis(20);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateHistoryPoint {
    recorded_at: String,
    score: f64,
    status: String,
    status_label: String,
    warning_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_seconds: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    checks_run: Option<String>,
}

fn gate_history_point(
    snapshot: &GateSnapshot,
    recorded_at: chrono::DateTime<chrono::Utc>,
) -> GateHistoryPoint {
    GateHistoryPoint {
        recorded_at: recorded_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        score: snapshot.score,
        status: snapshot.status.clone(),
        status_label: snapshot.status_label.clone(),
        warning_count: snapshot.warning_list.len(),
        duration_seconds: Some(snapshot.duration_seconds.clone()),
        checks_run: Some(snapshot.checks_run.clone()),
    }
}

fn retain_gate_history_lines(
    lines: Vec<Vec<u8>>,
    now: chrono::DateTime<chrono::Utc>,
) -> Vec<Vec<u8>> {
    let cutoff = now - chrono::Duration::days(90);
    let mut retained = lines
        .into_iter()
        .filter(|line| {
            // Preserve malformed physical lines so the read path can report
            // corruption as partial. They are not dated points, and remain
            // bounded by the physical-line cap below.
            serde_json::from_slice::<GateHistoryPoint>(line).map_or(true, |point| {
                chrono::DateTime::parse_from_rfc3339(&point.recorded_at)
                    .map_or(true, |recorded_at| {
                        recorded_at.with_timezone(&chrono::Utc) >= cutoff
                    })
            })
        })
        .collect::<Vec<_>>();
    if retained.len() > GATE_HISTORY_LINE_CAP {
        let excess = retained.len() - GATE_HISTORY_LINE_CAP;
        retained.drain(..excess);
    }
    retained
}

/// A display-ready view of the last gate run, persisted to `.anvil/gates.json`
/// for the `gate-summary` TUI dashboard to bind against (#2242).
///
/// This is intentionally **not** the internal [`GateResult`]: the json-render
/// dashboard components read *string* props (`MetricCard.value`,
/// `StatusBadge.status`/`label`) and `Table.rows` as an array-of-arrays, so the
/// snapshot pre-formats values into the exact shapes a spec's `$data` paths bind
/// to (`gates.status`, `gates.checkRows`, …). camelCase to match json-render
/// prop conventions.
fn gate_snapshot_from_result(result: &GateResult, aggregate: &GateAggregate) -> GateSnapshot {
    let check_rows = result
        .checks
        .iter()
        .map(|c| {
            let status = if c.requires_config {
                "config"
            } else if c.passed {
                "passed"
            } else {
                "failed"
            };
            vec![
                c.name.clone(),
                status.to_owned(),
                format!("{:.0}", c.score),
                c.message.clone(),
            ]
        })
        .collect();

    let warning_list: Vec<GateSnapshotWarning> = result
        .checks
        .iter()
        .filter_map(|c| {
            // Attention items: a real failure, or a check that could not run
            // for want of config. Passing checks are not warnings.
            let severity = if c.requires_config {
                "warn"
            } else if !c.passed {
                "error"
            } else {
                return None;
            };
            let message = if c.message.is_empty() {
                c.name.clone()
            } else {
                format!("{}: {}", c.name, c.message)
            };
            Some(GateSnapshotWarning {
                severity: severity.to_owned(),
                message,
            })
        })
        .collect();

    // Tri-state: a failure is "fail"; an overall pass that still has
    // attention items (config gaps) is "warn"; a clean pass is "pass".
    let (status, status_word) = if !result.overall {
        ("fail", "FAILED")
    } else if warning_list.is_empty() {
        ("pass", "PASSED")
    } else {
        ("warn", "PASSED")
    };
    let status_label = format!("{status_word} — score {:.0}/100", result.score);

    GateSnapshot {
        status: status.to_owned(),
        status_label,
        score: result.score,
        checks_run: aggregate.available_total.to_string(),
        warnings: warning_list.len().to_string(),
        // Tenths of a second via integer math (avoids a lossy f64 cast): a
        // sub-second run shows e.g. "0.4", not a misleading "0".
        duration_seconds: format!(
            "{}.{}",
            result.duration_ms / 1000,
            (result.duration_ms % 1000) / 100
        ),
        check_rows,
        warning_list,
    }
}

/// Persist the last gate run to `.anvil/gates.json` for the dashboard.
///
/// Best-effort: a write failure is logged at debug and otherwise ignored, so it
/// can never change the gate's exit code — persistence is a side effect, and the
/// gate stays "warnings over blocks, exit 0 by default".
fn persist_gate_snapshot(result: &GateResult, aggregate: &GateAggregate) {
    let Ok(root) = crate::util::workspace_root() else {
        tracing::debug!("gate snapshot: workspace root unresolved; skipping persist");
        return;
    };
    let snapshot = gate_snapshot_from_result(result, aggregate);
    persist_gate_snapshot_at_root(&root, &snapshot);
}

fn persist_gate_snapshot_at_root(root: &Path, snapshot: &GateSnapshot) {
    let json = match serde_json::to_vec_pretty(&snapshot) {
        Ok(json) => json,
        Err(e) => {
            tracing::debug!(error = %e, "gate snapshot: serialize failed; skipping persist");
            return;
        }
    };
    match persist_gate_snapshot_json(root, &json) {
        Ok(()) => {
            if let Err(e) = append_gate_history(root, snapshot, chrono::Utc::now()) {
                tracing::debug!(error = %e, "gate history: best-effort append failed");
            }
        }
        Err(e) => {
            tracing::debug!(error = %e, "gate snapshot: write to .anvil/gates.json failed");
        }
    }
}

fn append_gate_history(
    root: &Path,
    snapshot: &GateSnapshot,
    recorded_at: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    let _lock = lock_gate_history(root).context("locking gate history transaction")?;
    let existing = match read_gate_history(root) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error).context("reading held gate history"),
    };
    let mut lines = existing
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(<[u8]>::to_vec)
        .collect::<Vec<_>>();
    lines.push(serde_json::to_vec(&gate_history_point(
        snapshot,
        recorded_at,
    ))?);
    let lines = retain_gate_history_lines(lines, recorded_at);
    let mut bytes = lines.join(&b'\n');
    bytes.push(b'\n');
    persist_gate_named_json(root, GATE_HISTORY_FILE, &bytes)
}

fn read_gate_history_file(mut file: std::fs::File) -> std::io::Result<Vec<u8>> {
    use std::io::{Read as _, Seek as _, SeekFrom};

    let file_len = file.metadata()?.len();
    let max_bytes = u64::try_from(GATE_HISTORY_MAX_BYTES).unwrap_or(u64::MAX);
    let suffix_start = file_len.saturating_sub(max_bytes);
    file.seek(SeekFrom::Start(suffix_start))?;
    let mut bytes = Vec::with_capacity(GATE_HISTORY_MAX_BYTES.min(16 * 1024));
    file.by_ref().take(max_bytes).read_to_end(&mut bytes)?;

    if suffix_start > 0 {
        let leading_partial_len = bytes
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(bytes.len(), |newline| newline + 1);
        bytes.drain(..leading_partial_len);
    }
    Ok(bytes)
}

fn gate_history_lock_is_contended(error: &std::io::Error) -> bool {
    // Unix flock contention is WouldBlock. Windows ERROR_LOCK_VIOLATION (33)
    // and ERROR_SHARING_VIOLATION (32) often surface as PermissionDenied or
    // Other, not WouldBlock — treat them as retryable contention.
    matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::Interrupted
            | std::io::ErrorKind::PermissionDenied
    ) || matches!(error.raw_os_error(), Some(32 | 33 | 11 | 35 | 16))
}

fn acquire_gate_history_lock(lock: std::fs::File) -> std::io::Result<std::fs::File> {
    use fs2::FileExt as _;

    let deadline = std::time::Instant::now() + GATE_HISTORY_LOCK_TIMEOUT;
    loop {
        match lock.try_lock_exclusive() {
            Ok(()) => return Ok(lock),
            Err(error) if gate_history_lock_is_contended(&error) => {
                let Some(remaining) = deadline.checked_duration_since(std::time::Instant::now())
                else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "timed out waiting for gate history transaction lock",
                    ));
                };
                std::thread::sleep(GATE_HISTORY_LOCK_RETRY.min(remaining));
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(unix)]
fn lock_gate_history(root: &Path) -> std::io::Result<std::fs::File> {
    use std::os::fd::AsFd as _;

    use nix::fcntl::{OFlag, openat};
    use nix::sys::stat::Mode;

    let canonical_root = root.canonicalize()?;
    let root_fd = nix::fcntl::open(
        &canonical_root,
        OFlag::O_DIRECTORY | OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    )?;
    let anvil_fd = openat(
        root_fd.as_fd(),
        ".anvil",
        OFlag::O_DIRECTORY | OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    )?;
    let lock_fd = openat(
        anvil_fd.as_fd(),
        GATE_HISTORY_LOCK_FILE,
        OFlag::O_CREAT | OFlag::O_RDWR | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::S_IRUSR | Mode::S_IWUSR,
    )?;
    let lock = std::fs::File::from(lock_fd);
    acquire_gate_history_lock(lock)
}

#[cfg(windows)]
fn lock_gate_history(root: &Path) -> std::io::Result<std::fs::File> {
    use std::os::windows::fs::{MetadataExt as _, OpenOptionsExt as _};

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

    let canonical_root = root.canonicalize()?;
    let directory = canonical_root.join(".anvil");
    validate_gate_snapshot_parent(&canonical_root, &directory)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let path = directory.join(GATE_HISTORY_LOCK_FILE);
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let metadata = lock.metadata()?;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(std::io::Error::other(
            "gate history lock is a symlink or reparse point",
        ));
    }
    acquire_gate_history_lock(lock)
}

#[cfg(not(any(unix, windows)))]
fn lock_gate_history(root: &Path) -> std::io::Result<std::fs::File> {
    let canonical_root = root.canonicalize()?;
    let directory = canonical_root.join(".anvil");
    validate_gate_snapshot_parent(&canonical_root, &directory)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let path = directory.join(GATE_HISTORY_LOCK_FILE);
    if let Ok(metadata) = std::fs::symlink_metadata(&path)
        && gate_snapshot_parent_is_redirect(metadata.file_type().is_symlink(), &metadata)
    {
        return Err(std::io::Error::other("gate history lock is a symlink"));
    }
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)?;
    acquire_gate_history_lock(lock)
}

#[cfg(unix)]
fn read_gate_history(root: &Path) -> std::io::Result<Vec<u8>> {
    use std::fs::File;
    use std::os::fd::AsFd as _;

    use nix::fcntl::{OFlag, openat};
    use nix::sys::stat::Mode;

    let canonical_root = root.canonicalize()?;
    let root_fd = nix::fcntl::open(
        &canonical_root,
        OFlag::O_DIRECTORY | OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    )?;
    let anvil_fd = openat(
        root_fd.as_fd(),
        ".anvil",
        OFlag::O_DIRECTORY | OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    )?;
    let history_fd = openat(
        anvil_fd.as_fd(),
        GATE_HISTORY_FILE,
        OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    )?;
    read_gate_history_file(File::from(history_fd))
}

#[cfg(not(unix))]
fn read_gate_history(root: &Path) -> std::io::Result<Vec<u8>> {
    let canonical_root = root.canonicalize()?;
    let directory = canonical_root.join(".anvil");
    validate_gate_snapshot_parent(&canonical_root, &directory)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let history = directory.join(GATE_HISTORY_FILE);
    if let Ok(metadata) = std::fs::symlink_metadata(&history)
        && gate_snapshot_parent_is_redirect(metadata.file_type().is_symlink(), &metadata)
    {
        return Err(std::io::Error::other(
            "gate history is a symlink or reparse point",
        ));
    }
    read_gate_history_file(std::fs::File::open(history)?)
}

#[cfg(unix)]
fn persist_gate_snapshot_json(root: &Path, json: &[u8]) -> Result<()> {
    persist_gate_named_json(root, GATE_SNAPSHOT_FILE, json)
}

#[cfg(unix)]
fn persist_gate_named_json(root: &Path, filename: &str, json: &[u8]) -> Result<()> {
    use std::fs::File;
    use std::io::Write as _;
    use std::os::fd::AsFd as _;

    use nix::errno::Errno;
    use nix::fcntl::{OFlag, openat, renameat};
    use nix::sys::stat::{Mode, mkdirat};
    use nix::unistd::{UnlinkatFlags, unlinkat};

    let canonical_root = root
        .canonicalize()
        .with_context(|| format!("canonicalising gate workspace {}", root.display()))?;
    let root_fd = nix::fcntl::open(
        &canonical_root,
        OFlag::O_DIRECTORY | OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    )
    .with_context(|| format!("opening gate workspace {}", canonical_root.display()))?;

    match mkdirat(&root_fd, ".anvil", Mode::S_IRWXU) {
        Ok(()) | Err(Errno::EEXIST) => {}
        Err(error) => return Err(error).context("creating held .anvil directory"),
    }
    let anvil_fd = openat(
        root_fd.as_fd(),
        ".anvil",
        OFlag::O_DIRECTORY | OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
        Mode::empty(),
    )
    .context("opening .anvil without following symlinks")?;

    let temporary = format!(".{filename}.{}.tmp", uuid::Uuid::new_v4());
    let flags =
        OFlag::O_CREAT | OFlag::O_EXCL | OFlag::O_WRONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC;
    let file_fd = openat(
        anvil_fd.as_fd(),
        temporary.as_str(),
        flags,
        Mode::S_IRUSR | Mode::S_IWUSR,
    )
    .context("creating held gate snapshot temporary file")?;
    let mut file = File::from(file_fd);
    if let Err(error) = file.write_all(json).and_then(|()| file.flush()) {
        let _ = unlinkat(
            anvil_fd.as_fd(),
            temporary.as_str(),
            UnlinkatFlags::NoRemoveDir,
        );
        return Err(error).context("writing held gate snapshot temporary file");
    }
    drop(file);
    if let Err(error) = renameat(
        anvil_fd.as_fd(),
        temporary.as_str(),
        anvil_fd.as_fd(),
        filename,
    ) {
        let _ = unlinkat(
            anvil_fd.as_fd(),
            temporary.as_str(),
            UnlinkatFlags::NoRemoveDir,
        );
        return Err(error).context("publishing held gate snapshot");
    }
    Ok(())
}

#[cfg(not(unix))]
fn persist_gate_snapshot_json(root: &Path, json: &[u8]) -> Result<()> {
    persist_gate_named_json(root, GATE_SNAPSHOT_FILE, json)
}

#[cfg(not(unix))]
fn persist_gate_named_json(root: &Path, filename: &str, json: &[u8]) -> Result<()> {
    let canonical_root = root
        .canonicalize()
        .with_context(|| format!("canonicalising gate workspace {}", root.display()))?;
    let directory = canonical_root.join(".anvil");
    match std::fs::create_dir(&directory) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error).context("creating .anvil directory"),
    }
    validate_gate_snapshot_parent(&canonical_root, &directory)?;
    // Re-check immediately before the path-based atomic write. On Windows this
    // rejects both symlinks and junction/reparse points and proves the resolved
    // parent is still beneath the canonical workspace.
    validate_gate_snapshot_parent(&canonical_root, &directory)?;
    crate::util::atomic_write(&directory.join(filename), json)
}

#[cfg(not(unix))]
fn validate_gate_snapshot_parent(canonical_root: &Path, directory: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(directory)
        .with_context(|| format!("inspecting gate snapshot parent {}", directory.display()))?;
    if gate_snapshot_parent_is_redirect(metadata.file_type().is_symlink(), &metadata) {
        bail!(
            "refusing gate snapshot parent {} because it is a symlink or reparse point",
            directory.display()
        );
    }
    if !metadata.is_dir() {
        bail!(
            "refusing gate snapshot parent {} because it is not a directory",
            directory.display()
        );
    }
    let canonical_parent = directory.canonicalize().with_context(|| {
        format!(
            "canonicalising gate snapshot parent {}",
            directory.display()
        )
    })?;
    if !canonical_parent.starts_with(canonical_root) {
        bail!(
            "refusing gate snapshot parent {} outside workspace {}",
            canonical_parent.display(),
            canonical_root.display()
        );
    }
    Ok(())
}

#[cfg(windows)]
fn gate_snapshot_parent_is_redirect(is_symlink: bool, metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    is_symlink || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(any(unix, windows)))]
fn gate_snapshot_parent_is_redirect(is_symlink: bool, _metadata: &std::fs::Metadata) -> bool {
    is_symlink
}

/// CIB-011 / #1803 — actionable next-step hint shown beneath a
/// config-gap check. Names match the internal dispatch keys in
/// `run_single_check`; the hints point at the canonical onboarding
/// docs so the user can move from "anvil is broken" to a working
/// configuration without guessing.
fn config_gap_next_hint(name: &str) -> &'static str {
    match name {
        "architecture" => {
            "Create .anvil/architecture.yaml — see docs/public/anvil/tutorials/architecture.md"
        }
        "policy" => {
            "Create a .rego rule under .anvil/policies/ — see docs/public/anvil/tutorials/policies.md"
        }
        "command-safety" => {
            "Pass --plan <path/to/plan.aps.md> to anvil gate so command-safety has commands to analyse"
        }
        _ => "See `anvil gate --help` and the public docs for setup steps",
    }
}

fn notifications_for_gate_result(checks: &[CheckResult], overall: bool) -> Vec<Notification> {
    let gate_context = || NotificationContext {
        file: None,
        source: Some("gate".to_string()),
    };

    let mut notifications: Vec<Notification> = checks
        .iter()
        .map(|check| {
            // CIB-011 / #1803 — config-gap checks emit a Normal-priority
            // info notification carrying the `next:` hint rather than a
            // high-priority Failure (the check could not run, but the
            // user is not in a failing state until they configure).
            let class = if check.requires_config || check.passed {
                NotificationClass::Info
            } else {
                NotificationClass::Failure
            };
            let priority = if check.requires_config {
                NotificationPriority::Normal
            } else if check.passed {
                NotificationPriority::Low
            } else {
                NotificationPriority::High
            };
            Notification::new(
                class,
                priority,
                format!("Gate check: {}", check.name),
                if check.message.is_empty() {
                    if check.passed {
                        "Passed".to_string()
                    } else {
                        "Failed".to_string()
                    }
                } else {
                    check.message.clone()
                },
            )
            .with_context(gate_context())
        })
        .collect();

    notifications.push(
        Notification::new(
            if overall {
                NotificationClass::Info
            } else {
                NotificationClass::Failure
            },
            if overall {
                NotificationPriority::Normal
            } else {
                NotificationPriority::High
            },
            "Gate result",
            if overall {
                "All quality gates passed"
            } else {
                "Quality gates failed"
            },
        )
        .with_context(gate_context()),
    );

    notifications
}

/// Extract file paths referenced in a `.aps.md` plan file.
///
/// Parses `- **Files:** ...` lines and returns deduplicated paths.
/// Returns an empty set (and emits a warning) if the file cannot be read.
fn extract_plan_files(plan_path: &Path) -> std::collections::HashSet<String> {
    let content = match std::fs::read_to_string(plan_path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!(
                "Warning: failed to read plan file '{}': {err}. Falling back to full codebase scan.",
                plan_path.display()
            );
            return std::collections::HashSet::new();
        }
    };

    let file_re = Regex::new(r"`([^`]+)`").expect("valid regex");
    let mut files = std::collections::HashSet::new();

    // Track whether we're in a Files: continuation (multi-line entries).
    let mut in_files_block = false;

    for line in content.lines() {
        let trimmed = line.trim_start_matches([' ', '-']);
        if trimmed.starts_with("**Files:**") {
            in_files_block = true;
            for cap in file_re.captures_iter(trimmed) {
                let path = cap[1].to_string();
                if path.contains('/') || path.contains('.') {
                    files.insert(path);
                }
            }
        } else if in_files_block {
            // Continuation lines: indented lines with backticked paths.
            let has_backticks = trimmed.contains('`');
            let is_continuation =
                has_backticks && !trimmed.starts_with("**") && !trimmed.starts_with('#');
            if is_continuation {
                for cap in file_re.captures_iter(trimmed) {
                    let path = cap[1].to_string();
                    if path.contains('/') || path.contains('.') {
                        files.insert(path);
                    }
                }
            } else {
                in_files_block = false;
            }
        }
    }

    files
}

/// Resolve a plan argument to a path: either an absolute path, or relative to
/// the workspace root. Searches `plans/modules/` if not found directly.
fn resolve_plan_path(plan_arg: &str, root: &Path) -> Option<PathBuf> {
    let direct = PathBuf::from(plan_arg);
    if direct.exists() {
        return Some(direct);
    }

    // Try relative to workspace root.
    let relative = root.join(plan_arg);
    if relative.exists() {
        return Some(relative);
    }

    // Try in plans/modules/.
    let in_modules = root.join("plans/modules").join(plan_arg);
    if in_modules.exists() {
        return Some(in_modules);
    }

    // Try with .aps.md extension.
    let with_ext = root
        .join("plans/modules")
        .join(format!("{plan_arg}.aps.md"));
    if with_ext.exists() {
        return Some(with_ext);
    }

    None
}

fn run_check_lint(name: &str, root: &Path) -> CheckResult {
    let output = std::process::Command::new("pnpm")
        .args(["lint:check"])
        .current_dir(root)
        .output();
    match output {
        Ok(o) if o.status.success() => CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: "No lint errors".to_string(),
            requires_config: false,
        },
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            CheckResult {
                name: name.to_string(),
                passed: false,
                score: 0.0,
                message: format!("Lint errors found\n{stdout}\n{stderr}"),
                requires_config: false,
            }
        }
        Err(e) => CheckResult {
            name: name.to_string(),
            passed: false,
            score: 0.0,
            message: format!("Failed to run lint: {e}"),
            requires_config: false,
        },
    }
}

fn run_check_test(name: &str, root: &Path) -> CheckResult {
    let output = std::process::Command::new("pnpm")
        .args(["test"])
        .current_dir(root)
        .output();
    match output {
        Ok(o) if o.status.success() => CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: "All tests passed".to_string(),
            requires_config: false,
        },
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            CheckResult {
                name: name.to_string(),
                passed: false,
                score: 0.0,
                message: format!("Tests failed\n{stdout}\n{stderr}"),
                requires_config: false,
            }
        }
        Err(e) => CheckResult {
            name: name.to_string(),
            passed: false,
            score: 0.0,
            message: format!("Failed to run tests: {e}"),
            requires_config: false,
        },
    }
}

/// Maximum directory depth for the secret scan walk. Prevents runaway
/// recursion into deeply nested or symlink-heavy trees.
const SECRET_SCAN_MAX_DEPTH: usize = 20;

fn run_check_secret(
    name: &str,
    root: &Path,
    plan_files: &std::collections::HashSet<String>,
) -> CheckResult {
    let hook_mode = std::env::var("ANVIL_HOOK").is_ok_and(|value| value == "1");
    run_check_secret_with_hook_mode(name, root, plan_files, hook_mode)
}

struct StagedGateChange {
    head_path: Option<String>,
    index_oid: String,
    head_oid: Option<String>,
}

struct StagedGateInventory {
    changes: BTreeMap<String, StagedGateChange>,
    quarantined_paths: std::collections::BTreeSet<String>,
}

const GIT_RAW_INVENTORY_ARGS: [&str; 8] = [
    "diff",
    "--cached",
    "--raw",
    "--no-abbrev",
    "-z",
    "--find-renames",
    "--diff-filter=ACMRT",
    "--",
];

fn read_to_limit(reader: impl std::io::Read, max_bytes: usize) -> Option<Vec<u8>> {
    use std::io::Read as _;

    let read_limit = u64::try_from(max_bytes).ok()?.checked_add(1)?;
    let mut bytes = Vec::new();
    reader.take(read_limit).read_to_end(&mut bytes).ok()?;
    (bytes.len() <= max_bytes).then_some(bytes)
}

fn command_stdout_bounded(
    command: &mut std::process::Command,
    max_bytes: usize,
) -> Option<Vec<u8>> {
    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let Some(bytes) = read_to_limit(child.stdout.take()?, max_bytes) else {
        let _ = child.kill();
        let _ = child.wait();
        return None;
    };
    child.wait().ok()?.success().then_some(bytes)
}

fn escaped_inventory_path(path: &[u8]) -> String {
    let mut escaped = String::from("<staged path:");
    for byte in path {
        escaped.extend(std::ascii::escape_default(*byte).map(char::from));
    }
    escaped.push('>');
    escaped
}

fn quarantined_inventory_path(path: &[u8]) -> String {
    std::str::from_utf8(path).map_or_else(|_| escaped_inventory_path(path), str::to_string)
}

fn strict_inventory_path(path: &[u8]) -> Result<String, String> {
    let path_text = std::str::from_utf8(path).map_err(|_| escaped_inventory_path(path))?;
    if path_text.bytes().any(|byte| byte.is_ascii_control()) {
        return Err(escaped_inventory_path(path));
    }
    Ok(path_text.to_string())
}

fn render_gate_path(path: &str) -> String {
    let mut rendered = String::new();
    for byte in path.bytes() {
        rendered.extend(std::ascii::escape_default(byte).map(char::from));
    }
    rendered
}

fn parse_raw_inventory_header(header: &[u8]) -> Option<(&str, &str, u8)> {
    let header = std::str::from_utf8(header).ok()?;
    let mut fields = header.split_ascii_whitespace();
    let head_mode = fields.next()?.strip_prefix(':')?;
    let index_mode = fields.next()?;
    let head_oid = fields.next()?;
    let index_oid = fields.next()?;
    let status = fields.next()?;
    if fields.next().is_some() {
        return None;
    }
    let valid_mode =
        |mode: &str| mode.len() == 6 && mode.bytes().all(|byte| matches!(byte, b'0'..=b'7'));
    if !valid_mode(head_mode) || !valid_mode(index_mode) {
        return None;
    }
    let valid_oid = |oid: &str| {
        matches!(oid.len(), 40 | 64) && oid.bytes().all(|byte| byte.is_ascii_hexdigit())
    };
    if head_oid.len() != index_oid.len() || !valid_oid(head_oid) || !valid_oid(index_oid) {
        return None;
    }
    let status_bytes = status.as_bytes();
    let code = *status_bytes.first()?;
    match code {
        b'A' | b'M' | b'T' if status_bytes.len() == 1 => {}
        b'C' | b'R'
            if matches!(status_bytes.len(), 2..=4)
                && status_bytes[1..].iter().all(u8::is_ascii_digit)
                && std::str::from_utf8(&status_bytes[1..])
                    .ok()
                    .and_then(|score| score.parse::<u8>().ok())
                    .is_some_and(|score| score <= 100) => {}
        _ => return None,
    }
    Some((head_oid, index_oid, code))
}

fn parse_staged_gate_inventory(raw: &[u8]) -> Option<StagedGateInventory> {
    if !raw.is_empty() && raw.last() != Some(&0) {
        return None;
    }
    let mut fields = raw.split(|byte| *byte == b'\0');
    let mut changes = BTreeMap::new();
    let mut quarantined_paths = std::collections::BTreeSet::new();
    while let Some(header) = fields.next() {
        if header.is_empty() {
            return fields.next().is_none().then_some(StagedGateInventory {
                changes,
                quarantined_paths,
            });
        }
        let (head_oid, index_oid, code) = parse_raw_inventory_header(header)?;
        let (path, head_path) = if matches!(code, b'R' | b'C') {
            let old_path_bytes = fields.next()?;
            let new_path_bytes = fields.next()?;
            if old_path_bytes.is_empty() || new_path_bytes.is_empty() {
                return None;
            }
            let old_path = strict_inventory_path(old_path_bytes);
            let new_path = strict_inventory_path(new_path_bytes);
            if old_path.is_err() {
                quarantined_paths.insert(quarantined_inventory_path(old_path_bytes));
            }
            if new_path.is_err() {
                quarantined_paths.insert(quarantined_inventory_path(new_path_bytes));
            }
            let (Ok(old_path), Ok(new_path)) = (old_path, new_path) else {
                continue;
            };
            (new_path, Some(old_path))
        } else {
            let path_bytes = fields.next()?;
            if path_bytes.is_empty() {
                return None;
            }
            let Ok(path) = strict_inventory_path(path_bytes) else {
                quarantined_paths.insert(quarantined_inventory_path(path_bytes));
                continue;
            };
            (path.clone(), (code != b'A').then_some(path))
        };
        changes.insert(
            path,
            StagedGateChange {
                head_path,
                index_oid: index_oid.to_string(),
                head_oid: head_oid
                    .bytes()
                    .any(|byte| byte != b'0')
                    .then(|| head_oid.to_string()),
            },
        );
    }
    Some(StagedGateInventory {
        changes,
        quarantined_paths,
    })
}

fn staged_gate_changes(
    root: &Path,
    git_subprocess_count: &mut usize,
) -> Option<StagedGateInventory> {
    *git_subprocess_count += 1;
    let mut command = std::process::Command::new("git");
    command.args(GIT_RAW_INVENTORY_ARGS).current_dir(root);
    parse_staged_gate_inventory(&command_stdout_bounded(
        &mut command,
        MAX_GIT_DIFF_OUTPUT_SIZE,
    )?)
}

fn finding_workspace_path(root: &Path, file: &str) -> String {
    Path::new(file).strip_prefix(root).map_or_else(
        |_| file.trim_start_matches(['/', '\\']).replace('\\', "/"),
        |relative| relative.to_string_lossy().replace('\\', "/"),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RawSecretFindingKey {
    finding_type: u8,
    pattern_name: String,
    match_hash: [u8; 32],
}

fn finding_raw_match<'a>(line: &'a str, redacted_line: &str) -> Option<&'a str> {
    let (prefix, _) = redacted_line.split_once("[REDACTED]")?;
    let (_, suffix) = redacted_line.rsplit_once("[REDACTED]")?;
    line.strip_prefix(prefix)?
        .strip_suffix(suffix)
        .filter(|matched| !matched.is_empty())
}

fn raw_secret_finding_key(
    finding: &anvil_checks::secret::SecretFinding,
    line: &str,
) -> RawSecretFindingKey {
    use sha2::{Digest as _, Sha256};

    let finding_type = match finding.finding_type {
        anvil_checks::secret::FindingType::Pattern => 0,
        anvil_checks::secret::FindingType::Entropy => 1,
    };
    RawSecretFindingKey {
        finding_type,
        pattern_name: finding.pattern_name.clone(),
        match_hash: Sha256::digest(
            finding_raw_match(line, &finding.redacted_line)
                .unwrap_or(finding.redacted_match.as_str())
                .as_bytes(),
        )
        .into(),
    }
}

fn content_line(content: &str, line: usize) -> Option<&str> {
    content.lines().nth(line.checked_sub(1)?)
}

const MAX_STAGED_BLOB_SIZE: u64 = 1024 * 1024;
// Hook provenance stays bounded even when a repository stages many unique
// objects or produces an unexpectedly large patch/inventory stream.
const MAX_STAGED_BLOB_COUNT: usize = 1024;
const MAX_PROVENANCE_PATH_COUNT: usize = 1024;
const MAX_STAGED_BLOB_TOTAL_SIZE: u64 = 16 * 1024 * 1024;
const MAX_WORKTREE_PROVENANCE_TOTAL_SIZE: u64 = 16 * 1024 * 1024;
const MAX_PROVENANCE_SCAN_TOTAL_SIZE: u64 = 16 * 1024 * 1024;
const MAX_GIT_DIFF_OUTPUT_SIZE: usize = 8 * 1024 * 1024;
const GIT_BATCH_CHECK_ARGS: [&str; 2] = ["cat-file", "--batch-check"];
const GIT_BATCH_CONTENT_ARGS: [&str; 2] = ["cat-file", "--batch"];
const GIT_PROVENANCE_DIFF_ARGS: [&str; 7] = [
    "--no-ext-diff",
    "--no-textconv",
    "--no-color",
    "--no-prefix",
    "--find-renames",
    "--unified=0",
    "--",
];

enum GitBlobContent {
    Text(String),
    Oversized,
    Unavailable,
}

fn batch_header_size(header: &[u8]) -> Option<u64> {
    let header = std::str::from_utf8(header).ok()?.trim_end();
    let mut fields = header.rsplitn(3, ' ');
    fields.next()?.parse().ok()
}

fn blob_budget_admit(
    size: u64,
    used_count: &mut usize,
    used_bytes: &mut u64,
    max_count: usize,
    max_bytes: u64,
) -> bool {
    let Some(next_count) = used_count.checked_add(1) else {
        return false;
    };
    let Some(next_bytes) = used_bytes.checked_add(size) else {
        return false;
    };
    if next_count > max_count || next_bytes > max_bytes {
        return false;
    }
    *used_count = next_count;
    *used_bytes = next_bytes;
    true
}

fn inspect_next_blob(inspected_count: &mut usize, max_count: usize) -> bool {
    if *inspected_count >= max_count {
        return false;
    }
    *inspected_count += 1;
    true
}

fn batched_git_blob_contents(
    root: &Path,
    objects: &[String],
    git_subprocess_count: &mut usize,
) -> Option<BTreeMap<String, GitBlobContent>> {
    use std::io::{BufRead as _, Read as _, Write as _};

    if objects.is_empty() {
        return Some(BTreeMap::new());
    }
    *git_subprocess_count += 1;
    let mut child = std::process::Command::new("git")
        .args(GIT_BATCH_CHECK_ARGS)
        .current_dir(root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .ok()?;
    let mut input = child.stdin.take()?;
    let mut output = std::io::BufReader::new(child.stdout.take()?);
    let mut blobs = BTreeMap::new();
    let mut eligible = Vec::new();
    let mut inspected_count = 0;
    let mut eligible_count = 0;
    let mut eligible_bytes = 0;

    for object in objects {
        if !inspect_next_blob(&mut inspected_count, MAX_STAGED_BLOB_COUNT) {
            blobs.insert(object.clone(), GitBlobContent::Oversized);
            continue;
        }
        let metadata = (|| {
            input.write_all(object.as_bytes()).ok()?;
            input.write_all(b"\n").ok()?;
            input.flush().ok()?;
            let mut header = Vec::new();
            output.read_until(b'\n', &mut header).ok()?;
            (!header.ends_with(b" missing\n")).then(|| batch_header_size(&header))?
        })();
        match metadata {
            Some(size)
                if size < MAX_STAGED_BLOB_SIZE
                    && blob_budget_admit(
                        size,
                        &mut eligible_count,
                        &mut eligible_bytes,
                        MAX_STAGED_BLOB_COUNT,
                        MAX_STAGED_BLOB_TOTAL_SIZE,
                    ) =>
            {
                eligible.push((object.clone(), size));
            }
            Some(_) => {
                blobs.insert(object.clone(), GitBlobContent::Oversized);
            }
            None => {
                blobs.insert(object.clone(), GitBlobContent::Unavailable);
            }
        }
    }

    drop(input);
    if !child.wait().ok()?.success() {
        return None;
    }
    if eligible.is_empty() {
        return Some(blobs);
    }

    *git_subprocess_count += 1;
    let mut child = std::process::Command::new("git")
        .args(GIT_BATCH_CONTENT_ARGS)
        .current_dir(root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .ok()?;
    let mut input = child.stdin.take()?;
    let mut output = std::io::BufReader::new(child.stdout.take()?);
    for (object, expected_size) in &eligible {
        let content = (|| {
            input.write_all(object.as_bytes()).ok()?;
            input.write_all(b"\n").ok()?;
            input.flush().ok()?;
            let mut header = Vec::new();
            output.read_until(b'\n', &mut header).ok()?;
            (batch_header_size(&header)? == *expected_size).then_some(())?;
            let mut content = vec![0; usize::try_from(*expected_size).ok()?];
            output.read_exact(&mut content).ok()?;
            let mut terminator = [0];
            output.read_exact(&mut terminator).ok()?;
            (terminator == *b"\n").then_some(())?;
            String::from_utf8(content).ok()
        })();
        blobs.insert(
            object.clone(),
            content.map_or(GitBlobContent::Unavailable, GitBlobContent::Text),
        );
    }
    drop(input);
    if !child.wait().ok()?.success() {
        for (object, _) in eligible {
            blobs.insert(object, GitBlobContent::Unavailable);
        }
    }
    Some(blobs)
}

fn batch_diff_line_hunks(
    root: &Path,
    pathspecs: &std::collections::BTreeSet<String>,
    result_paths: &std::collections::BTreeSet<String>,
    cached: bool,
    git_subprocess_count: &mut usize,
) -> Option<BTreeMap<String, Option<Vec<LineHunk>>>> {
    let mut hunks = result_paths
        .iter()
        .cloned()
        .map(|path| (path, Some(Vec::new())))
        .collect::<BTreeMap<_, _>>();
    if pathspecs.is_empty() {
        return Some(hunks);
    }

    *git_subprocess_count += 1;
    let mut command = std::process::Command::new("git");
    command.args(["--literal-pathspecs", "diff"]);
    if cached {
        command.arg("--cached");
    }
    command.args(GIT_PROVENANCE_DIFF_ARGS);
    command.args(pathspecs);
    command.current_dir(root);
    let diff = String::from_utf8(command_stdout_bounded(
        &mut command,
        MAX_GIT_DIFF_OUTPUT_SIZE,
    )?)
    .ok()?;
    let mut current_path = None;
    let mut in_hunk = false;
    for line in diff.lines() {
        if line.starts_with("diff --git ") {
            current_path = None;
            in_hunk = false;
            continue;
        }
        if !in_hunk && let Some(path) = line.strip_prefix("+++ ") {
            current_path = result_paths.contains(path).then(|| path.to_string());
            continue;
        }
        if !line.starts_with("@@ ") {
            continue;
        }
        in_hunk = true;
        let path = current_path.as_ref()?;
        let parsed = (|| {
            let mut fields = line.split_whitespace();
            (fields.next()? == "@@").then_some(())?;
            let (index_start, index_len) = parse_diff_range(fields.next()?, '-')?;
            let (_worktree_start, worktree_len) = parse_diff_range(fields.next()?, '+')?;
            Some(LineHunk {
                index_start,
                index_len,
                worktree_len,
            })
        })();
        match (hunks.get_mut(path), parsed) {
            (Some(Some(path_hunks)), Some(hunk)) => path_hunks.push(hunk),
            (Some(path_hunks), None) => *path_hunks = None,
            _ => {}
        }
    }
    Some(hunks)
}

#[derive(Debug, Clone, Copy)]
struct LineHunk {
    index_start: usize,
    index_len: usize,
    worktree_len: usize,
}

fn parse_diff_range(token: &str, prefix: char) -> Option<(usize, usize)> {
    let range = token.strip_prefix(prefix)?;
    let (start, len) = range.split_once(',').unwrap_or((range, "1"));
    Some((start.parse().ok()?, len.parse().ok()?))
}

fn map_index_line_to_worktree(index_line: usize, hunks: &[LineHunk]) -> Option<usize> {
    let mut offset: isize = 0;
    for hunk in hunks {
        if hunk.index_len == 0 {
            if index_line <= hunk.index_start {
                break;
            }
        } else {
            if index_line < hunk.index_start {
                break;
            }
            if index_line < hunk.index_start.saturating_add(hunk.index_len) {
                if hunk.index_len == hunk.worktree_len {
                    return index_line.checked_add_signed(offset);
                }
                return None;
            }
        }
        offset +=
            isize::try_from(hunk.worktree_len).ok()? - isize::try_from(hunk.index_len).ok()?;
    }
    index_line.checked_add_signed(offset)
}

#[derive(Default)]
struct HookSecretProvenance {
    primary_worktree: BTreeMap<(String, usize, RawSecretFindingKey), usize>,
    index_only: Vec<anvil_checks::secret::SecretFinding>,
    indeterminate_paths: std::collections::BTreeSet<String>,
    worktree_keys: Vec<Option<RawSecretFindingKey>>,
    global_indeterminate: bool,
    git_subprocess_count: usize,
}

struct StagedProvenanceSnapshot {
    changes: Vec<(String, StagedGateChange)>,
    blobs: BTreeMap<String, GitBlobContent>,
    head_to_index: Option<BTreeMap<String, Option<Vec<LineHunk>>>>,
    index_to_worktree: Option<BTreeMap<String, Option<Vec<LineHunk>>>>,
    indeterminate_paths: std::collections::BTreeSet<String>,
    git_subprocess_count: usize,
}

fn git_patch_path_safe(path: &str) -> bool {
    path.is_ascii()
        && !path
            .bytes()
            .any(|byte| byte.is_ascii_control() || matches!(byte, b'\\' | b'"'))
}

fn blob_is_bounded_text(blobs: &BTreeMap<String, GitBlobContent>, oid: &str) -> bool {
    matches!(blobs.get(oid), Some(GitBlobContent::Text(_)))
}

fn worktree_file_is_bounded(root: &Path, path: &str) -> bool {
    std::fs::metadata(root.join(path)).is_ok_and(|metadata| metadata.len() < MAX_STAGED_BLOB_SIZE)
}

fn mapping_failed(hunks: Option<&BTreeMap<String, Option<Vec<LineHunk>>>>, path: &str) -> bool {
    hunks.is_none() || matches!(hunks.and_then(|hunks| hunks.get(path)), Some(None))
}

struct PathLineHunks<'a> {
    staged: Option<&'a [LineHunk]>,
    worktree: Option<&'a [LineHunk]>,
}

fn path_line_hunks<'a>(
    head_to_index: Option<&'a BTreeMap<String, Option<Vec<LineHunk>>>>,
    index_to_worktree: Option<&'a BTreeMap<String, Option<Vec<LineHunk>>>>,
    path: &str,
) -> Option<PathLineHunks<'a>> {
    if mapping_failed(head_to_index, path) || mapping_failed(index_to_worktree, path) {
        return None;
    }
    let staged = head_to_index
        .and_then(|hunks| hunks.get(path))
        .and_then(|hunks| hunks.as_deref());
    let worktree = index_to_worktree
        .and_then(|hunks| hunks.get(path))
        .and_then(|hunks| hunks.as_deref());
    Some(PathLineHunks { staged, worktree })
}

fn staged_head_content<'a>(
    change: &StagedGateChange,
    blobs: &'a BTreeMap<String, GitBlobContent>,
) -> &'a str {
    change
        .head_path
        .as_deref()
        .filter(|head_path| secret_path_scannable(Path::new(head_path)))
        .and(change.head_oid.as_deref())
        .and_then(|head_oid| blobs.get(head_oid))
        .and_then(|blob| match blob {
            GitBlobContent::Text(content) => Some(content.as_str()),
            GitBlobContent::Oversized | GitBlobContent::Unavailable => None,
        })
        .unwrap_or("")
}

fn split_bounded_paths<T>(
    mut changes: Vec<(String, T)>,
    limit: usize,
) -> (Vec<(String, T)>, std::collections::BTreeSet<String>) {
    if changes.len() <= limit {
        return (changes, std::collections::BTreeSet::new());
    }
    let overflow = changes
        .drain(limit..)
        .map(|(path, _)| path)
        .collect::<std::collections::BTreeSet<_>>();
    (changes, overflow)
}

fn staged_provenance_snapshot(root: &Path) -> Option<StagedProvenanceSnapshot> {
    let mut git_subprocess_count = 0_usize;
    let inventory = staged_gate_changes(root, &mut git_subprocess_count)?;
    let mut indeterminate_paths = inventory.quarantined_paths;
    let changes = inventory
        .changes
        .into_iter()
        .filter(|(path, _)| secret_path_scannable(Path::new(path)))
        .collect::<Vec<_>>();
    let (changes, overflow_paths) = split_bounded_paths(changes, MAX_PROVENANCE_PATH_COUNT);
    indeterminate_paths.extend(overflow_paths);
    let mut objects = std::collections::BTreeSet::new();
    for (path, change) in &changes {
        if !git_patch_path_safe(path) {
            continue;
        }
        objects.insert(change.index_oid.clone());
        if let Some(head_path) = &change.head_path
            && secret_path_scannable(Path::new(head_path))
            && let Some(head_oid) = &change.head_oid
        {
            objects.insert(head_oid.clone());
        }
    }
    let objects = objects.into_iter().collect::<Vec<_>>();
    let blobs =
        batched_git_blob_contents(root, &objects, &mut git_subprocess_count).unwrap_or_default();
    let cached_result_paths = changes
        .iter()
        .filter(|(path, change)| {
            git_patch_path_safe(path)
                && blob_is_bounded_text(&blobs, &change.index_oid)
                && change
                    .head_path
                    .as_deref()
                    .filter(|head_path| secret_path_scannable(Path::new(head_path)))
                    .zip(change.head_oid.as_deref())
                    .is_none_or(|(head_path, head_oid)| {
                        git_patch_path_safe(head_path) && blob_is_bounded_text(&blobs, head_oid)
                    })
        })
        .map(|(path, _)| path.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let mut cached_pathspecs = cached_result_paths.clone();
    for (path, change) in &changes {
        if cached_result_paths.contains(path)
            && let Some(head_path) = &change.head_path
            && git_patch_path_safe(head_path)
        {
            cached_pathspecs.insert(head_path.clone());
        }
    }
    let worktree_result_paths = changes
        .iter()
        .filter(|(path, change)| {
            git_patch_path_safe(path)
                && blob_is_bounded_text(&blobs, &change.index_oid)
                && worktree_file_is_bounded(root, path)
        })
        .map(|(path, _)| path.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let head_to_index = batch_diff_line_hunks(
        root,
        &cached_pathspecs,
        &cached_result_paths,
        true,
        &mut git_subprocess_count,
    );
    let index_to_worktree = batch_diff_line_hunks(
        root,
        &worktree_result_paths,
        &worktree_result_paths,
        false,
        &mut git_subprocess_count,
    );
    Some(StagedProvenanceSnapshot {
        changes,
        blobs,
        head_to_index,
        index_to_worktree,
        indeterminate_paths,
        git_subprocess_count,
    })
}

fn index_positions_by_path<T>(
    items: &[T],
    mut path_for: impl FnMut(&T) -> String,
) -> BTreeMap<String, Vec<usize>> {
    let mut indexed = BTreeMap::<String, Vec<_>>::new();
    for (index, item) in items.iter().enumerate() {
        indexed.entry(path_for(item)).or_default().push(index);
    }
    indexed
}

fn bounded_path_values<T, K>(
    root: &Path,
    items: &[T],
    mut path_for: impl FnMut(&T) -> String,
    mut value_for: impl FnMut(&T, &str) -> Option<K>,
    max_count: usize,
    max_bytes: u64,
) -> (Vec<Option<K>>, std::collections::BTreeSet<String>) {
    let positions = index_positions_by_path(items, &mut path_for);
    let mut values = std::iter::repeat_with(|| None)
        .take(items.len())
        .collect::<Vec<_>>();
    let mut indeterminate = std::collections::BTreeSet::new();
    let mut inspected_count = 0;
    let mut inspected_bytes = 0;
    for (path, path_positions) in positions {
        let Some(size) = std::fs::metadata(root.join(&path))
            .ok()
            .map(|metadata| metadata.len())
        else {
            indeterminate.insert(path);
            continue;
        };
        if size >= MAX_STAGED_BLOB_SIZE
            || !blob_budget_admit(
                size,
                &mut inspected_count,
                &mut inspected_bytes,
                max_count,
                max_bytes,
            )
        {
            indeterminate.insert(path);
            continue;
        }
        let Ok(content) = std::fs::read_to_string(root.join(&path)) else {
            indeterminate.insert(path);
            continue;
        };
        for index in path_positions {
            values[index] = value_for(&items[index], &content);
        }
    }
    (values, indeterminate)
}

#[allow(clippy::too_many_lines)] // Linear, fail-closed attribution pipeline; splitting would hide phase ordering.
fn staged_secret_provenance(
    root: &Path,
    worktree_findings: &[anvil_checks::secret::SecretFinding],
    config: &anvil_checks::secret::SecretCheckConfig,
) -> Option<HookSecretProvenance> {
    let StagedProvenanceSnapshot {
        changes,
        blobs,
        head_to_index,
        index_to_worktree,
        indeterminate_paths,
        git_subprocess_count,
    } = staged_provenance_snapshot(root)?;
    let (worktree_keys, worktree_indeterminate) = bounded_path_values(
        root,
        worktree_findings,
        |finding| finding_workspace_path(root, &finding.file),
        |finding, content| {
            content_line(content, finding.line).map(|line| raw_secret_finding_key(finding, line))
        },
        MAX_STAGED_BLOB_COUNT,
        MAX_WORKTREE_PROVENANCE_TOTAL_SIZE,
    );
    let worktree_positions = index_positions_by_path(worktree_findings, |finding| {
        finding_workspace_path(root, &finding.file)
    });
    let mut provenance = HookSecretProvenance {
        indeterminate_paths,
        worktree_keys: worktree_keys.clone(),
        ..HookSecretProvenance::default()
    };
    provenance
        .indeterminate_paths
        .extend(worktree_indeterminate);
    let mut scanned_path_count = 0;
    let mut scanned_bytes = 0;

    for (path, change) in changes {
        if !git_patch_path_safe(&path) {
            provenance.indeterminate_paths.insert(path);
            continue;
        }
        let index_content = match blobs.get(&change.index_oid) {
            Some(GitBlobContent::Text(content)) => content,
            Some(GitBlobContent::Oversized | GitBlobContent::Unavailable) | None => {
                provenance.indeterminate_paths.insert(path);
                continue;
            }
        };
        if change
            .head_path
            .as_deref()
            .is_some_and(|head_path| secret_path_scannable(Path::new(head_path)))
            && change.head_oid.as_deref().is_some_and(|head_oid| {
                !matches!(blobs.get(head_oid), Some(GitBlobContent::Text(_)))
            })
        {
            provenance.indeterminate_paths.insert(path);
            continue;
        }
        let head_content = staged_head_content(&change, &blobs);
        let Some(path_scan_size) = index_content.len().checked_add(head_content.len()) else {
            provenance.indeterminate_paths.insert(path);
            continue;
        };
        let Ok(path_scan_size) = u64::try_from(path_scan_size) else {
            provenance.indeterminate_paths.insert(path);
            continue;
        };
        if !blob_budget_admit(
            path_scan_size,
            &mut scanned_path_count,
            &mut scanned_bytes,
            MAX_PROVENANCE_PATH_COUNT,
            MAX_PROVENANCE_SCAN_TOTAL_SIZE,
        ) {
            provenance.indeterminate_paths.insert(path);
            continue;
        }
        let Some(PathLineHunks {
            staged: staged_hunks,
            worktree: worktree_hunks,
        }) = path_line_hunks(head_to_index.as_ref(), index_to_worktree.as_ref(), &path)
        else {
            provenance.indeterminate_paths.insert(path);
            continue;
        };
        let mut base_counts = BTreeMap::<(usize, RawSecretFindingKey), usize>::new();
        for finding in anvil_checks::secret::scan_content(head_content, &path, config) {
            let Some(key) = content_line(head_content, finding.line)
                .map(|line| raw_secret_finding_key(&finding, line))
            else {
                continue;
            };
            let Some(index_line) =
                staged_hunks.and_then(|hunks| map_index_line_to_worktree(finding.line, hunks))
            else {
                continue;
            };
            *base_counts.entry((index_line, key)).or_default() += 1;
        }

        let mut introduced = Vec::new();
        for mut finding in anvil_checks::secret::scan_content(index_content, &path, config) {
            let Some(key) = content_line(index_content, finding.line)
                .map(|line| raw_secret_finding_key(&finding, line))
            else {
                finding.file.clone_from(&path);
                provenance.index_only.push(finding);
                continue;
            };
            if let Some(count) = base_counts.get_mut(&(finding.line, key.clone()))
                && *count > 0
            {
                *count -= 1;
            } else {
                introduced.push((finding, key));
            }
        }
        let mut worktree_counts = BTreeMap::<(usize, RawSecretFindingKey), usize>::new();
        let mut worktree_lines = std::collections::BTreeSet::new();
        for position in worktree_positions.get(&path).into_iter().flatten() {
            let finding = &worktree_findings[*position];
            worktree_lines.insert(finding.line);
            if let Some(key) = &worktree_keys[*position] {
                *worktree_counts
                    .entry((finding.line, key.clone()))
                    .or_default() += 1;
            }
        }

        for (mut finding, key) in introduced {
            let mapped_line =
                worktree_hunks.and_then(|hunks| map_index_line_to_worktree(finding.line, hunks));
            let mapped_key = mapped_line.map(|line| (line, key.clone()));
            if let Some((line, key)) = mapped_key
                && let Some(count) = worktree_counts.get_mut(&(line, key.clone()))
                && *count > 0
            {
                *count -= 1;
                *provenance
                    .primary_worktree
                    .entry((path.clone(), line, key))
                    .or_default() += 1;
            } else {
                if mapped_line.is_some_and(|line| worktree_lines.contains(&line)) {
                    provenance.indeterminate_paths.insert(path.clone());
                }
                finding.file.clone_from(&path);
                provenance.index_only.push(finding);
            }
        }
    }

    provenance.git_subprocess_count = git_subprocess_count;
    Some(provenance)
}

fn secret_path_scannable(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with(".env"))
        || path.extension().is_some_and(|ext| {
            matches!(
                ext.to_string_lossy().as_ref(),
                "ts" | "js" | "rs" | "json" | "yaml" | "yml" | "toml" | "env"
            )
        })
}

fn secret_scan_files(root: &Path, plan_files: &std::collections::HashSet<String>) -> Vec<String> {
    let mut files_to_scan: Vec<String> = Vec::new();

    // SCAN-001: gate-secret discovery uses `ignore::WalkBuilder`. Per-file
    // scans run on the rayon pool inside `run_secret_check` (rolled out as
    // part of this slice). The depth cap is preserved for full-codebase
    // scans only; plan-scoped runs must reach explicitly referenced files
    // regardless of nesting depth.
    let mut walker_builder = ignore::WalkBuilder::new(root);
    walker_builder
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .filter_entry(|e| {
            // ADOPT-004: prune via the kernel-canonical ignore-dir list so
            // the gate secret walk stays in lock-step with audit/check/drift
            // (and the watcher). The previous hand-rolled `SECRET_SCAN_IGNORE`
            // omitted framework build trees (`.next`, `.nx`, `.turbo`) and
            // agent worktrees (`.claude`, `.worktrees`), flooding the report
            // with high-entropy generated chunks. The lockfile skip lives in
            // `run_secret_check`.
            //
            // Prune only *directories* whose name is ignored — never skip a
            // file that happens to share a name with an ignore-dir. This
            // matches audit/check/drift, which gate the same check on
            // `is_dir()`.
            if e.file_type().is_some_and(|ft| ft.is_dir())
                && let Some(name) = e.file_name().to_str()
            {
                return !crate::util::is_ignored_dir_name(name);
            }
            true
        });
    if plan_files.is_empty() {
        walker_builder.max_depth(Some(SECRET_SCAN_MAX_DEPTH));
    }
    let walker = walker_builder.build();

    for entry in walker
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
    {
        let path = entry.path();

        // Plan scoping: skip files not referenced in the plan.
        if !plan_files.is_empty() {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            if !plan_files.iter().any(|pf| {
                if pf.ends_with('/') || root.join(pf).is_dir() {
                    rel.starts_with(pf.as_str())
                } else {
                    rel == pf.as_str()
                }
            }) {
                continue;
            }
        }

        if !secret_path_scannable(path) {
            continue;
        }

        files_to_scan.push(path.to_string_lossy().into_owned());
    }

    files_to_scan
}

fn run_check_secret_with_hook_mode(
    name: &str,
    root: &Path,
    plan_files: &std::collections::HashSet<String>,
    hook_mode: bool,
) -> CheckResult {
    run_check_secret_with_hook_mode_and_provenance(name, root, plan_files, hook_mode, false)
}

fn resolve_hook_provenance(
    hook_mode: bool,
    force_inventory_failure: bool,
    load: impl FnOnce() -> Option<HookSecretProvenance>,
) -> Option<HookSecretProvenance> {
    hook_mode.then(|| {
        let loaded = (!force_inventory_failure).then(load).flatten();
        loaded.unwrap_or_else(|| HookSecretProvenance {
            global_indeterminate: true,
            ..HookSecretProvenance::default()
        })
    })
}

fn hook_indeterminate_count(provenance: Option<&HookSecretProvenance>) -> usize {
    provenance.map_or(0, |provenance| {
        provenance.indeterminate_paths.len() + usize::from(provenance.global_indeterminate)
    })
}

fn run_check_secret_with_hook_mode_and_provenance(
    name: &str,
    root: &Path,
    plan_files: &std::collections::HashSet<String>,
    hook_mode: bool,
    force_inventory_failure: bool,
) -> CheckResult {
    let files_to_scan = secret_scan_files(root, plan_files);

    let file_refs: Vec<&str> = files_to_scan.iter().map(String::as_str).collect();
    let config = crate::util::secret_check_config(root);
    let root_str = root.to_string_lossy();
    let result = anvil_checks::secret::run_secret_check(&file_refs, &config, Some(&root_str));

    // Surface allowlist suppressions so a withheld match is never silent.
    let suppression_suffix = crate::util::secret_suppression_suffix(&result.suppressions);

    let pattern_errors_suffix = secret_pattern_errors_suffix(&result.pattern_errors);
    let mut hook_provenance = resolve_hook_provenance(hook_mode, force_inventory_failure, || {
        staged_secret_provenance(root, &result.findings, &config)
    });
    let index_only_count = hook_provenance
        .as_ref()
        .map_or(0, |provenance| provenance.index_only.len());
    let indeterminate_count = hook_indeterminate_count(hook_provenance.as_ref());

    if result.passed && index_only_count == 0 && indeterminate_count == 0 {
        CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: format!(
                "No hardcoded secrets found{suppression_suffix}{pattern_errors_suffix}"
            ),
            requires_config: false,
        }
    } else {
        // CIB-239: the hook still scans the full tree, but exact finding
        // provenance comes from raw-occurrence fingerprints held only in memory and
        // index-to-worktree line mapping. This keeps staged new occurrences
        // primary even when redaction collides or an identical unstaged
        // duplicate appears earlier. Index-only staged findings are appended
        // so an unstaged removal cannot make the hook pass. Unavailable Git
        // truth fails closed in hook mode. Non-hook output is unchanged.
        let mut locations: Vec<String> = result
            .findings
            .iter()
            .enumerate()
            .map(|(index, f)| {
                let location = secret_finding_location(f, &file_refs, root);
                let location = if hook_mode {
                    render_gate_path(&location)
                } else {
                    location
                };
                let path = finding_workspace_path(root, &f.file);
                let is_indeterminate = hook_provenance.as_ref().is_some_and(|provenance| {
                    provenance.global_indeterminate
                        || provenance.indeterminate_paths.contains(&path)
                });
                let key = hook_provenance
                    .as_ref()
                    .and_then(|provenance| provenance.worktree_keys.get(index))
                    .and_then(Clone::clone);
                let is_primary = key.and_then(|key| {
                    let count = hook_provenance
                        .as_mut()?
                        .primary_worktree
                        .get_mut(&(path, f.line, key))?;
                    (*count > 0).then(|| *count -= 1)
                });
                if hook_provenance.is_some() && !is_indeterminate && is_primary.is_none() {
                    format!("{location} (pre-existing; not staged)")
                } else {
                    location
                }
            })
            .collect();
        if let Some(provenance) = &hook_provenance {
            if provenance.global_indeterminate {
                locations.push("staged changes [staged content unavailable]".to_string());
            }
            locations.extend(provenance.index_only.iter().map(|finding| {
                format!(
                    "{}:{} [{}]",
                    render_gate_path(&finding.file),
                    finding.line,
                    finding.pattern_name
                )
            }));
            locations.extend(
                provenance
                    .indeterminate_paths
                    .iter()
                    .map(|path| format!("{} [staged content unavailable]", render_gate_path(path))),
            );
        }
        let finding_count = result.findings.len() + index_only_count + indeterminate_count;
        CheckResult {
            name: name.to_string(),
            passed: false,
            score: if result.findings.is_empty() {
                0.0
            } else {
                f64::from(result.score)
            },
            message: format!(
                "Potential secrets found in {} location(s):\n{}{suppression_suffix}{pattern_errors_suffix}",
                finding_count,
                locations.join("\n")
            ),
            requires_config: false,
        }
    }
}

/// Format one antipattern or AST warning as a gate location line.
///
/// These already arrive workspace-relative; the shared renderer normalises
/// separators and suppresses a whole-file sentinel line so every location in
/// a gate run reads the same way (CIB-237).
fn warning_location(file: &str, line: usize, id: &str) -> String {
    let file = crate::display_path::render(file, None);
    format!(
        "{} [{}]",
        crate::display_path::format_location(&file, line),
        id
    )
}

/// Format one secret finding as a gate location line.
///
/// CIB-237: the secret scanner returns `/{relative}` while every other check
/// family in the same gate run returns `{relative}`, so an unfiltered format
/// printed `/.env:1` directly above `src/app.py:12`. Routing through
/// [`crate::display_path`] gives the whole gate output one path style.
fn secret_finding_location(
    f: &anvil_checks::secret::SecretFinding,
    scanned: &[&str],
    root: &Path,
) -> String {
    let file = crate::display_path::render_secret_finding(&f.file, scanned, Some(root));
    format!(
        "{} [{}]",
        crate::display_path::format_location(&file, f.line),
        f.pattern_name
    )
}

fn secret_pattern_errors_suffix(pattern_errors: &[String]) -> String {
    if pattern_errors.is_empty() {
        return String::new();
    }

    format!(
        "\n\n⚠ {} custom secret pattern(s) failed to compile and were skipped:\n{}",
        pattern_errors.len(),
        pattern_errors
            .iter()
            .map(|err| format!("  - {err}"))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

/// SURFSQL (Track 3) — scan SQL migration files for destructive patterns.
///
/// Warn-only by design: per the architecture rules (warnings over blocks,
/// exit 0 by default, new-edges-only), this surface never fails the gate.
/// It reports unsuppressed findings in the message; escalation to a failing
/// verdict on *new* edges waits on the SURFSQL-006 drift baseline.
///
/// The OPSUP-006 file-presence guard is a *coarse* pre-filter on the declared
/// globs (which include migration directories), so reaching here means there
/// *might* be SQL; `is_sql_migration_file` does the precise per-file
/// selection. A migration directory holding only non-`.sql` files therefore
/// yields zero scanned files and a clean result. Files that match but fail to
/// read are counted and surfaced in the message rather than silently dropped.
fn run_check_sql_migrations(name: &str, root: &Path, walked_files: &[String]) -> CheckResult {
    use anvil_checks::surface::sql::{is_sql_migration_file, run_surfsql_check};

    let mut sql_files: Vec<(std::path::PathBuf, String)> = Vec::new();
    let mut unreadable = 0usize;
    for rel in walked_files
        .iter()
        .filter(|rel| is_sql_migration_file(std::path::Path::new(rel)))
    {
        match std::fs::read_to_string(root.join(rel)) {
            Ok(content) => sql_files.push((std::path::PathBuf::from(rel), content)),
            Err(_) => unreadable += 1,
        }
    }
    let unreadable_note = if unreadable > 0 {
        format!(" ({unreadable} file(s) unreadable, skipped)")
    } else {
        String::new()
    };

    let result = run_surfsql_check(&sql_files);

    // SURFSQL-006: warn only on *new* edges. Each unsuppressed finding carries
    // the same fingerprint the drift snapshot stored (shared derivation in
    // `commands::drift`), so a finding present in the latest `anvil drift
    // snapshot` is baselined and omitted. `None` = no snapshot exists at all,
    // so every finding is surfaced — the pre-baseline warn-on-all behaviour
    // (warnings over blocks). `Some(set)` = a snapshot exists (its SQL set may
    // be empty, when the repo was clean at snapshot time).
    let baseline = crate::commands::drift::latest_sql_baseline_fingerprints(root);
    let has_baseline = baseline.is_some();
    let baseline = baseline.unwrap_or_default();

    // SURFSQL-002 destructive + SURFSQL-003 schema-hygiene, unsuppressed.
    let mut new_locations: Vec<String> = Vec::new();
    let mut baselined = 0usize;
    for f in result.destructive.iter().filter(|f| !f.suppressed) {
        let (_, fingerprint) = crate::commands::drift::destructive_finding_id(f);
        if baseline.contains(&fingerprint) {
            baselined += 1;
        } else {
            new_locations.push(format!(
                "{}:{} [{}] {}",
                f.file,
                f.line,
                rule_label(f.kind),
                f.statement
            ));
        }
    }
    for f in result.hygiene.iter().filter(|f| !f.suppressed) {
        let (_, fingerprint) = crate::commands::drift::hygiene_finding_id(f);
        if baseline.contains(&fingerprint) {
            baselined += 1;
        } else {
            new_locations.push(format!(
                "{}:{} [{}] {}",
                f.file,
                f.line,
                hygiene_label(f.kind),
                f.statement
            ));
        }
    }

    if new_locations.is_empty() {
        let scanned = sql_files.len() + unreadable;
        let summary = if has_baseline {
            format!(
                "No new SQL migration issues vs drift baseline ({baselined} baselined) across {scanned} file(s){unreadable_note}"
            )
        } else {
            format!(
                "No destructive or schema-hygiene SQL migration issues found across {scanned} file(s){unreadable_note}"
            )
        };
        return CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: summary,
            requires_config: false,
        };
    }

    let baseline_note = if has_baseline {
        format!(" ({baselined} baselined)")
    } else {
        " (no drift baseline — run `anvil drift snapshot` to baseline existing findings)"
            .to_string()
    };

    CheckResult {
        name: name.to_string(),
        // Warn-only: surfaced, never blocking (architecture default).
        passed: true,
        score: 100.0,
        message: format!(
            "⚠ {} new SQL migration issue(s) flagged (warn-only{baseline_note}){}:\n{}",
            new_locations.len(),
            unreadable_note,
            new_locations.join("\n")
        ),
        requires_config: false,
    }
}

/// Short label for a destructive finding kind, for the gate message.
fn rule_label(kind: anvil_checks::surface::sql::DestructiveKind) -> &'static str {
    use anvil_checks::surface::sql::DestructiveKind as K;
    match kind {
        K::DropTable => "DROP TABLE",
        K::DropColumn => "DROP COLUMN",
        K::Truncate => "TRUNCATE",
        K::DeleteWithoutWhere => "DELETE without WHERE",
        K::UpdateWithoutWhere => "UPDATE without WHERE",
        K::DropConstraint => "DROP CONSTRAINT",
    }
}

/// Short label for a schema-hygiene finding kind, for the gate message.
fn hygiene_label(kind: anvil_checks::surface::sql::HygieneKind) -> &'static str {
    use anvil_checks::surface::sql::HygieneKind as K;
    match kind {
        K::MissingCreateTableGuard => "CREATE TABLE without IF NOT EXISTS",
        K::MissingCreateIndexGuard => "CREATE INDEX without IF NOT EXISTS",
    }
}

/// SURFGHA (Track 3) — scan workflow files for supply-chain risks. Warn-only
/// (architecture default); the file-presence guard (OPSUP-006) already
/// short-circuits when no workflow files are present.
fn run_check_github_actions(name: &str, root: &Path, walked_files: &[String]) -> CheckResult {
    use anvil_checks::surface::github_actions::{is_workflow_file, run_surfgha_check};

    let mut files: Vec<(std::path::PathBuf, String)> = Vec::new();
    let mut unreadable = 0usize;
    for rel in walked_files
        .iter()
        .filter(|rel| is_workflow_file(std::path::Path::new(rel)))
    {
        match std::fs::read_to_string(root.join(rel)) {
            Ok(content) => files.push((std::path::PathBuf::from(rel), content)),
            Err(_) => unreadable += 1,
        }
    }
    let unreadable_note = if unreadable > 0 {
        format!(" ({unreadable} file(s) unreadable, skipped)")
    } else {
        String::new()
    };

    let result = run_surfgha_check(&files);
    let locations: Vec<String> = result
        .risks
        .iter()
        .filter(|f| !f.suppressed)
        .map(|f| {
            format!(
                "{}:{} [{}] {}",
                f.file,
                f.line,
                gha_risk_label(f.risk),
                f.snippet
            )
        })
        .collect();

    if locations.is_empty() {
        return CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: format!(
                "No GitHub Actions supply-chain risks found across {} file(s){unreadable_note}",
                files.len() + unreadable
            ),
            requires_config: false,
        };
    }

    CheckResult {
        name: name.to_string(),
        // Warn-only: surfaced, never blocking (architecture default).
        passed: true,
        score: 100.0,
        message: format!(
            "⚠ {} GitHub Actions supply-chain risk(s) flagged (warn-only){}:\n{}",
            locations.len(),
            unreadable_note,
            locations.join("\n")
        ),
        requires_config: false,
    }
}

/// Short label for a SURFGHA risk kind, for the gate message.
fn gha_risk_label(risk: anvil_checks::surface::github_actions::GhaRisk) -> &'static str {
    use anvil_checks::surface::github_actions::GhaRisk as R;
    match risk {
        R::UnpinnedActionRef => "unpinned action ref",
        R::PullRequestTarget => "pull_request_target",
        R::SelfHostedRunner => "self-hosted runner",
    }
}

/// SURFDOCK (Track 3) — scan Dockerfiles for build-hygiene risks. Warn-only.
/// Self-filters via `is_dockerfile` (the check declares no file-presence
/// globs — Dockerfile naming doesn't fit the glob vocabulary).
fn run_check_dockerfile(name: &str, root: &Path, walked_files: &[String]) -> CheckResult {
    use anvil_checks::surface::dockerfile::{is_dockerfile, run_surfdock_check};

    let mut files: Vec<(std::path::PathBuf, String)> = Vec::new();
    let mut unreadable = 0usize;
    for rel in walked_files
        .iter()
        .filter(|rel| is_dockerfile(std::path::Path::new(rel)))
    {
        match std::fs::read_to_string(root.join(rel)) {
            Ok(content) => files.push((std::path::PathBuf::from(rel), content)),
            Err(_) => unreadable += 1,
        }
    }
    let unreadable_note = if unreadable > 0 {
        format!(" ({unreadable} file(s) unreadable, skipped)")
    } else {
        String::new()
    };

    let result = run_surfdock_check(&files);
    let locations: Vec<String> = result
        .risks
        .iter()
        .filter(|f| !f.suppressed)
        .map(|f| {
            format!(
                "{}:{} [{}] {}",
                f.file,
                f.line,
                dock_risk_label(f.risk),
                f.instruction
            )
        })
        .collect();

    if locations.is_empty() {
        return CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: format!(
                "No Dockerfile build-hygiene risks found across {} file(s){unreadable_note}",
                files.len() + unreadable
            ),
            requires_config: false,
        };
    }

    CheckResult {
        name: name.to_string(),
        // Warn-only: surfaced, never blocking (architecture default).
        passed: true,
        score: 100.0,
        message: format!(
            "⚠ {} Dockerfile build-hygiene risk(s) flagged (warn-only){}:\n{}",
            locations.len(),
            unreadable_note,
            locations.join("\n")
        ),
        requires_config: false,
    }
}

/// Short label for a SURFDOCK risk kind, for the gate message.
fn dock_risk_label(risk: anvil_checks::surface::dockerfile::DockerRisk) -> &'static str {
    use anvil_checks::surface::dockerfile::DockerRisk as R;
    match risk {
        R::AddRemoteFetch => "ADD remote fetch",
        R::PipeToShell => "pipe-to-shell",
        R::LatestBaseImage => ":latest base image",
        R::SudoInRun => "sudo in layer",
        R::AptMissingNoRecommends => "apt-get without --no-install-recommends",
    }
}

/// SURFSH (Track 3, T1) — scan checked-in shell scripts for dangerous commands
/// via the shared `command_safety` catalogue. Warn-only; self-filters via
/// `is_shell_file` (the `*.sh`/`*.bash` file-presence guard is a coarse
/// pre-filter on top).
fn run_check_shell(name: &str, root: &Path, walked_files: &[String]) -> CheckResult {
    use anvil_checks::surface::shell::{is_shell_file, run_surfsh_check};

    let mut files: Vec<(std::path::PathBuf, String)> = Vec::new();
    let mut unreadable = 0usize;
    for rel in walked_files
        .iter()
        .filter(|rel| is_shell_file(std::path::Path::new(rel)))
    {
        match std::fs::read_to_string(root.join(rel)) {
            Ok(content) => files.push((std::path::PathBuf::from(rel), content)),
            Err(_) => unreadable += 1,
        }
    }
    let unreadable_note = if unreadable > 0 {
        format!(" ({unreadable} file(s) unreadable, skipped)")
    } else {
        String::new()
    };

    let result = run_surfsh_check(&files);
    let locations: Vec<String> = result
        .commands
        .iter()
        .filter(|f| !f.suppressed)
        .map(|f| format!("{}:{} [{}] {}", f.file, f.line, f.reason, f.command))
        .collect();

    if locations.is_empty() {
        return CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: format!(
                "No dangerous shell-script commands found across {} file(s){unreadable_note}",
                files.len() + unreadable
            ),
            requires_config: false,
        };
    }

    CheckResult {
        name: name.to_string(),
        // Warn-only: surfaced, never blocking (architecture default).
        passed: true,
        score: 100.0,
        message: format!(
            "⚠ {} dangerous shell-script command(s) flagged (warn-only){}:\n{}",
            locations.len(),
            unreadable_note,
            locations.join("\n")
        ),
        requires_config: false,
    }
}

/// CIB-199 / ADR-112: the anti-pattern gate honours the accepted ADR-002
/// posture — warnings do not block by default (`Error` threshold), so only
/// `error`-severity rules fail the gate. `fail_on_warnings` (the opt-in
/// `--fail-on-warnings` flag / `ANVIL_FAIL_ON_WARNINGS`) lowers the threshold to
/// `Warning`, restoring strict blocking for teams that want it. Security rules
/// that must always block (weak-cipher WC-002, JWT-`none` WC-003, plus the
/// dynamic-execution family) carry `error` severity in the registry, so they
/// block regardless of this flag.
/// The workspace-relative files the anti-pattern scan should read: walked source
/// files, narrowed to `plan_files` when plan-scoped, minus files the repo
/// declares generated via `.gitattributes` `linguist-generated` (CIB-199). The
/// `.gitattributes` filter is anti-pattern-scan only — the secret scan and other
/// gate engines walk independently and still see these files.
fn antipattern_scan_files(
    root: &Path,
    plan_files: &std::collections::HashSet<String>,
) -> Vec<String> {
    let mut files = walk_source_files(root, &[]);
    if !plan_files.is_empty() {
        files.retain(|f| {
            plan_files.iter().any(|pf| {
                if pf.ends_with('/') || root.join(pf).is_dir() {
                    f.starts_with(pf.as_str())
                } else {
                    f == pf.as_str()
                }
            })
        });
    }

    let generated = crate::util::git_generated_paths(root, &files);
    if !generated.is_empty() {
        files.retain(|f| !generated.contains(f));
    }
    files
}

/// CIB-199: read `antipattern.exclude` globs, or a failing [`CheckResult`]
/// naming the config surface when the project config cannot be read or parsed.
/// Operator-declared exclusions must never be silently dropped.
fn resolve_antipattern_excludes(name: &str, root: &Path) -> Result<Vec<String>, CheckResult> {
    read_anvilrc_antipattern_excludes(root).map_err(|err| CheckResult {
        name: name.to_string(),
        passed: false,
        score: 0.0,
        message: format!("Failed to read `antipattern.exclude` from project config: {err}"),
        requires_config: false,
    })
}

fn run_check_antipattern(
    name: &str,
    root: &Path,
    plan_files: &std::collections::HashSet<String>,
    fail_on_warnings: bool,
) -> CheckResult {
    let severity_threshold = if fail_on_warnings {
        anvil_checks::antipattern::WarningSeverity::Warning
    } else {
        anvil_checks::antipattern::WarningSeverity::Error
    };
    let files_to_scan = antipattern_scan_files(root, plan_files);

    // CIB-199: project-config `antipattern.exclude` globs (for generators the
    // path/banner auto-detector does not recognise). A malformed config fails
    // the check loudly rather than silently disabling the exclude list.
    let exclude_globs = match resolve_antipattern_excludes(name, root) {
        Ok(globs) => globs,
        Err(failure) => return failure,
    };

    let absolute_files: Vec<String> = files_to_scan
        .iter()
        .map(|rel| root.join(rel).to_string_lossy().into_owned())
        .collect();
    let file_refs: Vec<&str> = absolute_files.iter().map(String::as_str).collect();
    let root_str = root.to_string_lossy();
    let result = anvil_checks::antipattern::run_antipattern_check(
        &file_refs,
        &anvil_checks::antipattern::AntipatternCheckConfig {
            severity_threshold,
            exclude_globs,
            ..anvil_checks::antipattern::AntipatternCheckConfig::default()
        },
        Some(&root_str),
    );

    // ADR-071 §7: `anvil gate` runs the AST tier alongside the regex scanner.
    // AST rules in the Rust catalogue are advisory (`info`) today, so they are
    // surfaced but do not fail the gate; a future `warning`/`error` AST rule
    // joins the blocking set here without further plumbing.
    let ast = anvil_checks_ast::scan_paths(
        &file_refs,
        Some(&root_str),
        &anvil_checks_ast::AstScanOptions::default(),
    );
    // Surface scanner-init failures (council operations MAJOR) on stderr.
    for err in &ast.init_errors {
        eprintln!("anvil: AST anti-pattern rule load error: {err}");
    }

    if result.files_scanned == 0 && ast.files_scanned == 0 {
        return CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: "No analysable files found for anti-pattern scan. Skipping.".to_string(),
            requires_config: false,
        };
    }

    let ast_blocking: Vec<&anvil_checks::antipattern::Warning> = ast
        .warnings
        .iter()
        .filter(|w| {
            w.suppressed.is_none()
                && match w.severity {
                    anvil_checks::antipattern::WarningSeverity::Error => true,
                    // Warning-severity AST rules block only under fail-on-warnings,
                    // matching the regex-tier threshold above (ADR-112). No such
                    // rule ships today, so this is forward-compatibility.
                    anvil_checks::antipattern::WarningSeverity::Warning => fail_on_warnings,
                    anvil_checks::antipattern::WarningSeverity::Info => false,
                }
        })
        .collect();

    if result.passed && ast_blocking.is_empty() {
        CheckResult {
            name: name.to_string(),
            passed: true,
            score: f64::from(result.score),
            message: result.message,
            requires_config: false,
        }
    } else {
        let mut locations: Vec<String> = result
            .warnings
            .warnings
            .iter()
            .filter(|w| w.suppressed.is_none())
            .map(|w| warning_location(&w.location.file, w.location.line, &w.id))
            .collect();
        locations.extend(
            ast_blocking
                .iter()
                .map(|w| warning_location(&w.location.file, w.location.line, &w.id)),
        );
        let details = if locations.is_empty() {
            result.message
        } else {
            format!("{}\n{}", result.message, locations.join("\n"))
        };
        CheckResult {
            name: name.to_string(),
            passed: false,
            score: f64::from(result.score),
            message: details,
            requires_config: false,
        }
    }
}

const DEFAULT_COVERAGE_THRESHOLD: f64 = 80.0;

fn run_check_coverage(project_root: &Path, threshold: f64) -> CheckResult {
    let lcov_path = project_root.join("coverage/lcov.info");
    let cobertura_path = project_root.join("coverage/cobertura.xml");

    if lcov_path.exists() {
        match std::fs::read_to_string(&lcov_path) {
            Ok(content) => {
                let mut total_lines: u64 = 0;
                let mut hit_lines: u64 = 0;
                for line in content.lines() {
                    if let Some(val) = line.strip_prefix("LF:") {
                        if let Ok(n) = val.trim().parse::<u64>() {
                            total_lines += n;
                        }
                    } else if let Some(val) = line.strip_prefix("LH:")
                        && let Ok(n) = val.trim().parse::<u64>()
                    {
                        hit_lines += n;
                    }
                }
                if total_lines == 0 {
                    return CheckResult {
                        name: "coverage".to_string(),
                        passed: true,
                        score: 100.0,
                        message: "Coverage report empty (no lines tracked). Skipping.".to_string(),
                        requires_config: false,
                    };
                }
                #[allow(clippy::cast_precision_loss)]
                let pct = (hit_lines as f64 / total_lines as f64) * 100.0;
                let passed = pct >= threshold;
                CheckResult {
                    name: "coverage".to_string(),
                    passed,
                    score: pct,
                    message: format!("Line coverage: {pct:.1}% (threshold: {threshold:.0}%)"),
                    requires_config: false,
                }
            }
            Err(e) => CheckResult {
                name: "coverage".to_string(),
                passed: false,
                score: 0.0,
                message: format!("Failed to read lcov.info: {e}"),
                requires_config: false,
            },
        }
    } else if cobertura_path.exists() {
        match std::fs::read_to_string(&cobertura_path) {
            Ok(content) => {
                // Extract line-rate="X.XX" attribute from cobertura XML
                let rate = Regex::new(r#"line-rate="([0-9.]+)""#)
                    .ok()
                    .and_then(|re| re.captures(&content))
                    .and_then(|cap| cap.get(1))
                    .and_then(|m| m.as_str().parse::<f64>().ok());
                match rate {
                    Some(r) => {
                        let pct = r * 100.0;
                        let passed = pct >= threshold;
                        CheckResult {
                            name: "coverage".to_string(),
                            passed,
                            score: pct,
                            message: format!(
                                "Line coverage: {pct:.1}% (threshold: {threshold:.0}%)"
                            ),
                            requires_config: false,
                        }
                    }
                    None => CheckResult {
                        name: "coverage".to_string(),
                        passed: false,
                        score: 0.0,
                        message: "Failed to parse line-rate from cobertura.xml".to_string(),
                        requires_config: false,
                    },
                }
            }
            Err(e) => CheckResult {
                name: "coverage".to_string(),
                passed: false,
                score: 0.0,
                message: format!("Failed to read cobertura.xml: {e}"),
                requires_config: false,
            },
        }
    } else {
        CheckResult {
            name: "coverage".to_string(),
            passed: true,
            score: 100.0,
            message:
                "No coverage report found (coverage/lcov.info or coverage/cobertura.xml). Skipping."
                    .to_string(),
            requires_config: false,
        }
    }
}

const BLOCKED_NPM_PACKAGES: &[&str] = &[
    "event-stream",
    "flatmap-stream",
    "ua-parser-js",
    "colors",
    "faker",
    "node-ipc",
];

fn run_check_dependency(project_root: &Path) -> CheckResult {
    let npm_lock = project_root.join("package-lock.json");
    let cargo_lock = project_root.join("Cargo.lock");

    let has_npm = npm_lock.exists();
    let has_cargo = cargo_lock.exists();

    if !has_npm && !has_cargo {
        return CheckResult {
            name: "dependency".to_string(),
            passed: true,
            score: 100.0,
            message: "No lockfile found (package-lock.json or Cargo.lock). Skipping.".to_string(),
            requires_config: false,
        };
    }

    let mut blocked_found: Vec<String> = Vec::new();

    if has_npm {
        match std::fs::read_to_string(&npm_lock) {
            Ok(content) => {
                for pkg in BLOCKED_NPM_PACKAGES {
                    let pattern = format!("\"node_modules/{pkg}\"");
                    if content.contains(&pattern) {
                        blocked_found.push((*pkg).to_string());
                    }
                }
            }
            Err(e) => {
                return CheckResult {
                    name: "dependency".to_string(),
                    passed: false,
                    score: 0.0,
                    message: format!("Failed to read {}: {e}", npm_lock.display()),
                    requires_config: false,
                };
            }
        }
    }

    // Cargo.lock scanning can be extended later; for now only npm is checked.

    if blocked_found.is_empty() {
        CheckResult {
            name: "dependency".to_string(),
            passed: true,
            score: 100.0,
            message: "No blocked dependencies found".to_string(),
            requires_config: false,
        }
    } else {
        CheckResult {
            name: "dependency".to_string(),
            passed: false,
            score: 0.0,
            message: format!("Blocked dependencies found: {}", blocked_found.join(", ")),
            requires_config: false,
        }
    }
}

/// Extract import edges from source files using the kernel's tree-sitter parser.
///
/// When `source_files` is provided, only those files are parsed (avoids a
/// redundant directory walk). Otherwise falls back to walking `project_root`.
fn extract_import_edges(
    project_root: &Path,
    source_files: Option<&[String]>,
) -> Vec<anvil_architecture::ImportEdge> {
    let mut parser = anvil_kernel::parser::Parser::new();
    let mut edges = Vec::new();

    // RSTLAN-005 / PYLAN-006: `.rs` and `.py` join the JS/TS family so Rust
    // crates and Python packages participate in architecture/boundary analysis.
    // The kernel parser dispatches by extension, so `parse_bytes` handles `.rs`
    // via the Rust extractor (RSTLAN-002) and `.py` via the Python extractor
    // (PYLAN-002).
    // LANGTAIL: tail-wave extensions join too, kept in step with
    // `Language::from_path`. Their symbols flow into the graph; cross-file
    // import-edge *resolvers* for these languages are T2 work, so edges may
    // resolve to `None` until then — parsing them now is correct and harmless.
    let include_extensions = [
        "ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "py", "pyi", "dart", "go", "java", "kt",
        "kts", "cs", "c", "h", "cpp", "cc", "cxx", "c++", "hpp", "hh", "hxx", "h++",
    ];

    // Collect file paths to parse — either from the pre-collected list or via walkdir.
    let owned_paths: Vec<String>;
    let file_paths: &[String] = if let Some(files) = source_files {
        files
    } else {
        owned_paths = walk_source_files(project_root, &include_extensions);
        &owned_paths
    };

    for rel_path in file_paths {
        // Filter to parseable languages — the pre-collected list from
        // collect_source_files may include other file types matched by
        // architecture layer globs.
        let ext = std::path::Path::new(rel_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if !include_extensions.contains(&ext) {
            continue;
        }

        let path = project_root.join(rel_path);

        let Ok(content) = std::fs::read(&path) else {
            continue;
        };

        let Ok(parse_result) = parser.parse_bytes(&path, &content) else {
            continue;
        };

        let file_symbols =
            anvil_kernel::parser::extract::extract_symbols(&parse_result.tree, &content, &path, 0);

        for import in &file_symbols.imports {
            // Resolution is language-aware on the importing file's extension: a
            // Python relative import (`.sibling`) and a TS relative specifier
            // (`./sibling`) both begin with `.` but resolve under different
            // rules, and a Python absolute import (`foo.bar`) must resolve
            // against the package tree rather than be dropped.
            //
            // - `.py` / `.pyi` → Python module resolution (PYLAN-006: relative
            //   dot prefixes against the file's package, absolute against
            //   flat/`src` roots; stdlib/third-party drop out). Stub files
            //   (`.pyi`) share Python's import grammar, so they must take this
            //   path too — not the TS/Rust fallback.
            // - else: TS/JS relative specifiers (`./`, `../`) resolve lexically;
            //   Rust module paths (containing `::`) resolve against the owning
            //   crate's module tree. Everything else (bare npm packages,
            //   `std::`/external crates) targets code outside the workspace and
            //   is skipped — never a boundary violation.
            let resolved = if ext == "py" || ext == "pyi" {
                anvil_architecture::resolve_python_import(project_root, rel_path, &import.to_source)
            } else if import.to_source.starts_with('.') {
                resolve_import(rel_path, &import.to_source)
            } else if import.to_source.contains("::") {
                anvil_architecture::resolve_rust_import(project_root, rel_path, &import.to_source)
            } else {
                None
            };

            if let Some(to_file) = resolved {
                edges.push(anvil_architecture::ImportEdge {
                    from_file: rel_path.clone(),
                    to_file,
                    line: import.line,
                });
            }
        }
    }

    edges
}

/// Walk the workspace directory and collect source file paths (relative).
///
/// When `extensions` is non-empty, only files with a matching extension are
/// included. When empty, all files are collected.
///
/// SCAN-001: discovery routed through `ignore::WalkBuilder` to share the
/// welcome-screen walker shape. The per-file boundary scan downstream
/// already parallelises on rayon.
fn walk_source_files(project_root: &Path, extensions: &[&str]) -> Vec<String> {
    let walker = ignore::WalkBuilder::new(project_root)
        .follow_links(false)
        .standard_filters(false)
        .hidden(false)
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            if e.file_type().is_some_and(|ft| ft.is_dir()) {
                return !is_ignored_dir_name(&name);
            }
            true
        })
        .build();

    let mut files = Vec::new();
    for entry in walker.filter_map(std::result::Result::ok) {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let path = entry.path();
        if !extensions.is_empty() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !extensions.contains(&ext) {
                continue;
            }
        }
        let rel_path = path
            .strip_prefix(project_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        files.push(rel_path);
    }
    files
}

/// Resolve a relative import specifier to a workspace-relative path.
///
/// Given `from_file = "src/app/service.ts"` and `specifier = "../core/entity"`,
/// returns `"src/core/entity"`. Does not verify the file exists on disk;
/// the validator matches against assigned files by prefix.
///
/// Returns `None` if the specifier traverses above the workspace root
/// (e.g. `"../../../outside"`). These imports are silently excluded from
/// boundary analysis since they reference external code.
fn resolve_import(from_file: &str, specifier: &str) -> Option<String> {
    let from_dir = from_file.rsplit_once('/').map_or("", |(dir, _)| dir);

    // Combine from_dir with the specifier and normalise.
    let combined = if from_dir.is_empty() {
        specifier.to_string()
    } else {
        format!("{from_dir}/{specifier}")
    };

    // Normalise path segments (resolve .. and .).
    let mut parts: Vec<&str> = Vec::new();
    for segment in combined.split('/') {
        match segment {
            "." | "" => {}
            ".." => {
                // Returns None if traversal goes above workspace root.
                parts.pop()?;
            }
            s => parts.push(s),
        }
    }

    if parts.is_empty() {
        return None;
    }

    Some(parts.join("/"))
}

fn run_check_architecture(project_root: &Path) -> CheckResult {
    let config_path = project_root.join(".anvil/architecture.yaml");

    if !config_path.exists() {
        return CheckResult {
            name: "architecture".to_string(),
            passed: true,
            score: 100.0,
            message: "No architecture config found (.anvil/architecture.yaml). Skipping."
                .to_string(),
            requires_config: false,
        };
    }

    let definition = match anvil_architecture::parse_architecture_definition(project_root) {
        Ok(def) => def,
        Err(e) => {
            return CheckResult {
                name: "architecture".to_string(),
                passed: false,
                score: 0.0,
                message: format!("Architecture validation failed: {e}"),
                requires_config: false,
            };
        }
    };

    let diagnostics = anvil_architecture::diagnose_definition(&definition);
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.is_error())
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        return CheckResult {
            name: "architecture".to_string(),
            passed: false,
            score: 0.0,
            message: format!(
                "Architecture config preflight failed:\n{}",
                errors
                    .iter()
                    .map(|diagnostic| diagnostic.message.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            requires_config: false,
        };
    }

    // Collect source files once and share between edge extraction and validation
    // to avoid redundant directory walks (RCLI-053).
    let source_files = anvil_architecture::collect_source_files(project_root, &definition);
    let edges = extract_import_edges(project_root, Some(&source_files));

    let result =
        anvil_architecture::validate_with_files_and_edges(&definition, &source_files, &edges);

    if result.valid {
        CheckResult {
            name: "architecture".to_string(),
            passed: true,
            score: 100.0,
            message: "Architecture config is valid".to_string(),
            requires_config: false,
        }
    } else {
        let msgs: Vec<String> = result
            .violations
            .iter()
            .map(|v| {
                let boundary_name = v.boundary.as_ref().map_or("unknown", |b| b.name.as_str());
                let message = v
                    .boundary
                    .as_ref()
                    .map_or("boundary violation", |b| b.message.as_str());
                format!("{}: {} ({})", boundary_name, message, v.edge.from)
            })
            .collect();
        CheckResult {
            name: "architecture".to_string(),
            passed: false,
            score: 0.0,
            message: format!(
                "{} violation(s):\n{}",
                result.violations.len(),
                msgs.join("\n")
            ),
            requires_config: false,
        }
    }
}

/// Collect changed files from git status (unstaged + staged).
fn git_changed_files(project_root: &Path) -> Vec<String> {
    std::process::Command::new("git")
        .args(["status", "--porcelain", "-u"])
        .current_dir(project_root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|line| {
                    // porcelain format: XY filename
                    // Renamed/copied files: XY old -> new
                    let trimmed = line.get(3..)?;
                    if trimmed.contains(" -> ") {
                        trimmed.rsplit_once(" -> ").map(|(_, new)| new.to_string())
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Build policy input with project context so policies can reference
/// `input.workspace`, `input.files`, `input.changed_files`, etc.
///
/// When `all_files` is provided, filters it by policy-relevant extensions
/// instead of walking the directory tree again.
fn build_policy_input(
    project_root: &Path,
    profile: Option<&str>,
    plan_path: Option<&str>,
    plan_files: &std::collections::HashSet<String>,
    all_files: Option<&[String]>,
) -> serde_json::Value {
    let policy_extensions = [
        "ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "json", "yaml", "yml",
    ];

    let source_files: Vec<String> = if let Some(files) = all_files {
        files
            .iter()
            .filter(|f| {
                std::path::Path::new(f.as_str())
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|ext| policy_extensions.contains(&ext))
            })
            .cloned()
            .collect()
    } else {
        walk_source_files(project_root, &policy_extensions)
    };

    let changed_files = git_changed_files(project_root);

    // When plan-scoped, filter files to only those referenced in the plan.
    let files = if plan_files.is_empty() {
        source_files
    } else {
        source_files
            .into_iter()
            .filter(|f| {
                plan_files.iter().any(|pf| {
                    if pf.ends_with('/') {
                        f.starts_with(pf.as_str())
                    } else {
                        f == pf.as_str()
                    }
                })
            })
            .collect()
    };

    let mut input = serde_json::json!({
        "workspace": project_root.to_string_lossy(),
        "files": files,
        "changed_files": changed_files,
        "profile": profile.unwrap_or("default"),
    });

    if let Some(plan) = plan_path {
        input["plan_path"] = serde_json::Value::String(plan.to_string());
    }

    input
}

/// A single policy finding extracted from a `data.anvil.policies` result.
struct PolicyFinding {
    policy_id: String,
    /// Normalised severity label — `error` (from `violation`/`deny` rules) or
    /// `warning` (from `warn` rules), preserving the legacy severity mapping.
    severity: String,
    message: String,
}

/// A reported policy-check failure (score 0, not a skip). Used for compile
/// errors and evaluation errors — fail-fast, never a silent pass.
fn failed_policy_result(message: String) -> CheckResult {
    CheckResult {
        name: "policy".to_string(),
        passed: false,
        score: 0.0,
        message,
        requires_config: false,
    }
}

/// A passing policy result (score 100). Used for clean skips (no bundle —
/// message carries "Skipping" so the strict-config guard treats an absent
/// bundle as a config gap) and for evaluations that surface only warnings.
fn passing_policy_result(message: String) -> CheckResult {
    CheckResult {
        name: "policy".to_string(),
        passed: true,
        score: 100.0,
        message,
        requires_config: false,
    }
}

/// Per-policy byte cap. Mirrors `MAX_POLICY_BYTES` in commands/policy/eval.rs
/// (1 MiB, regorus's own default). Kept as a local constant because that one is
/// private to the eval module.
const GATE_MAX_POLICY_BYTES: u64 = 1 << 20;
/// Total-bundle byte ceiling across all discovered policies. Mirrors the input
/// cap in commands/policy/eval.rs (8 MiB) so a bundle of many small files
/// cannot exhaust memory.
const GATE_MAX_BUNDLE_BYTES: u64 = 8 << 20;

/// How many leading bytes of a policy file to inspect for the generation
/// header. The header is a first-line comment, so a small prefix suffices and
/// keeps the check cheap enough to run before the size caps.
const GENERATED_HEADER_PROBE_BYTES: usize = 256;

/// Recursively discover candidate `.rego` policy files under `dir`.
///
/// Mirrors the legacy loader's discovery semantics: `*.rego` files only,
/// `*_test.rego` excluded, generated output under a `.generated/` path segment
/// excluded, returned in deterministic (path-sorted) order. Symlinks are
/// followed so a link escaping the workspace is surfaced (and rejected by the
/// per-file containment check) rather than silently skipped.
///
/// A traversal error (permission denied, symlink loop) is surfaced as a
/// reported check failure rather than silently omitting policies — "a bundle
/// exists" must mean "the bundle was fully evaluated".
fn discover_policy_files(dir: &Path) -> Result<Vec<PathBuf>, CheckResult> {
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in walkdir::WalkDir::new(dir).follow_links(true) {
        let entry = entry.map_err(|e| {
            let path = e
                .path()
                .map_or_else(|| dir.display().to_string(), |p| p.display().to_string());
            failed_policy_result(format!(
                "Failed to read policy bundle ({path}): {e}; refusing to evaluate a partial bundle."
            ))
        })?;
        let name = entry.file_name().to_string_lossy();
        if !name.ends_with(".rego") || name.ends_with("_test.rego") {
            continue;
        }
        // Generated policies (compiler output, e.g. the architecture compiler)
        // live under a `.generated` segment and are already evaluated by
        // run_check_architecture; excluding them avoids double evaluation.
        if entry
            .path()
            .components()
            .any(|c| c.as_os_str() == ".generated")
        {
            continue;
        }
        if entry.path().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

/// Whether a policy file carries the Anvil auto-generation header, read from a
/// bounded prefix only. The `.generated/` path segment is handled at discovery;
/// this covers generated files written outside that directory (legacy loader
/// parity). Kept cheap so it can run *before* the size caps — a large generated
/// file is skipped, not reported as oversized.
fn file_has_generated_header(path: &Path) -> std::io::Result<bool> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    file.take(GENERATED_HEADER_PROBE_BYTES as u64)
        .read_to_end(&mut buf)?;
    // Accept both current lowercase brand and the pre-rebrand capitalised form.
    let prefix = String::from_utf8_lossy(&buf);
    Ok(
        prefix.contains("# Auto-generated by anvil")
            || prefix.contains("# Auto-generated by Anvil"),
    )
}

/// Resolve a policy-supplied severity override to the gate's two-valued
/// vocabulary: `error` (blocks) or `warning` (does not).
///
/// `error`/`err` map to error; `warning`/`warn` and `info` map to the
/// non-blocking warning class (preserving the legacy behaviour where only
/// `violation`/`deny` rules blocked). Any unrecognised string fails closed to
/// the rule's own default severity (`violation`/`deny` → error, `warn` →
/// warning) rather than being accepted verbatim — since severity now controls
/// pass/fail, a typo must not silently land a violation in the non-blocking
/// bucket.
fn resolve_policy_severity(override_sev: Option<&str>, default_sev: &str) -> String {
    match override_sev {
        Some(s) => match s.to_lowercase().as_str() {
            "error" | "err" => "error".to_string(),
            "warning" | "warn" | "info" => "warning".to_string(),
            _ => default_sev.to_string(),
        },
        None => default_sev.to_string(),
    }
}

/// Build a single [`PolicyFinding`] from one rule-set item, honouring the
/// legacy shapes: a bare string message, or an object carrying
/// `message`/`msg` and an optional `severity` override.
fn finding_from_item(
    item: &serde_json::Value,
    policy_id: &str,
    default_sev: &str,
) -> PolicyFinding {
    match item {
        serde_json::Value::String(msg) => PolicyFinding {
            policy_id: policy_id.to_string(),
            severity: default_sev.to_string(),
            message: msg.clone(),
        },
        serde_json::Value::Object(obj) => {
            let message = obj
                .get("message")
                .or_else(|| obj.get("msg"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("policy violation")
                .to_string();
            let severity = resolve_policy_severity(
                obj.get("severity").and_then(serde_json::Value::as_str),
                default_sev,
            );
            PolicyFinding {
                policy_id: policy_id.to_string(),
                severity,
                message,
            }
        }
        other => PolicyFinding {
            policy_id: policy_id.to_string(),
            severity: default_sev.to_string(),
            message: other.to_string(),
        },
    }
}

/// Extract findings from a `data.anvil.policies`-rooted result value.
///
/// The value is an object mapping each policy sub-package to its rule outputs.
/// The recognised rule vocabulary is the crate single-source
/// [`crate::policy_vocab::VIOLATION_FAMILY_KEYS`] (error-class) and
/// [`crate::policy_vocab::WARNING_FAMILY_KEYS`] (warning-class, including the
/// documented `warning` rule set) — the same consts the pre-write extractor
/// (`mcp::policy_prewrite`) consumes, so the two surfaces cannot drift. Other
/// keys (helper rules, `info`) are ignored, as on the OPA path.
fn extract_policy_findings(value: &serde_json::Value) -> Vec<PolicyFinding> {
    use crate::policy_vocab::{VIOLATION_FAMILY_KEYS, WARNING_FAMILY_KEYS};

    let mut out = Vec::new();
    let Some(map) = value.as_object() else {
        return out;
    };
    for (policy_id, output) in map {
        let Some(obj) = output.as_object() else {
            continue;
        };
        let families = [
            (VIOLATION_FAMILY_KEYS, "error"),
            (WARNING_FAMILY_KEYS, "warning"),
        ];
        for (keys, default_sev) in families {
            for key in keys {
                if let Some(arr) = obj.get(*key).and_then(serde_json::Value::as_array) {
                    for item in arr {
                        out.push(finding_from_item(item, policy_id, default_sev));
                    }
                }
            }
        }
    }
    out
}

/// Map the gate's evaluation context onto the canonical `PolicyInput` v1 the
/// policy-engine facade consumes: changed and workspace file lists feed
/// `diff.changed_files` and `repo_state.files` respectively.
fn policy_input_from_gate(
    project_root: &Path,
    profile: Option<&str>,
    plan_path: Option<&str>,
    plan_files: &std::collections::HashSet<String>,
    all_files: Option<&[String]>,
) -> PolicyInput {
    let value = build_policy_input(project_root, profile, plan_path, plan_files, all_files);
    let string_array = |key: &str| -> Vec<String> {
        value
            .get(key)
            .and_then(serde_json::Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };

    let mut input = PolicyInput::default();
    input.repo_state.files = string_array("files");
    input.diff.changed_files = string_array("changed_files");
    input
}

/// Load discovered policies into `engine`, enforcing per-file path containment,
/// resource caps, and generated-policy exclusion. Returns the count of policies
/// actually admitted, or a reported [`CheckResult`] failure (fail-fast).
fn admit_policy_files(
    engine: &mut Engine,
    policy_files: &[PathBuf],
    canonical_root: &Path,
) -> Result<usize, CheckResult> {
    let mut evaluated = 0usize;
    let mut total_bytes = 0u64;
    for path in policy_files {
        // Per-file containment: an individual `.rego` symlinked out of the
        // workspace has the same leak risk as a symlinked bundle dir — its
        // compile-error message could echo external file contents.
        let canonical_path = path.canonicalize().map_err(|e| {
            failed_policy_result(format!("Failed to resolve policy {}: {e}", path.display()))
        })?;
        if !canonical_path.starts_with(canonical_root) {
            return Err(failed_policy_result(format!(
                "Policy file {} resolves outside the workspace root \
                 (path containment breach); refusing to evaluate.",
                path.display()
            )));
        }

        // Generated policies (auto-generation header) are compiler output that
        // run_check_architecture already evaluates — skip entirely, *before*
        // the size caps, so a large generated file is excluded rather than
        // reported as oversized. The header is read from a bounded prefix.
        let generated = file_has_generated_header(&canonical_path).map_err(|e| {
            failed_policy_result(format!("Failed to read policy {}: {e}", path.display()))
        })?;
        if generated {
            continue;
        }

        // Resource caps (mirroring commands/policy/eval.rs) apply to admitted
        // files only: reject an oversized file before it lands in memory, and
        // cap the whole bundle.
        let meta = std::fs::metadata(&canonical_path).map_err(|e| {
            failed_policy_result(format!("Failed to stat policy {}: {e}", path.display()))
        })?;
        if meta.len() > GATE_MAX_POLICY_BYTES {
            return Err(failed_policy_result(format!(
                "Policy {} is {} bytes, over the {GATE_MAX_POLICY_BYTES}-byte per-policy limit.",
                path.display(),
                meta.len(),
            )));
        }
        total_bytes = total_bytes.saturating_add(meta.len());
        if total_bytes > GATE_MAX_BUNDLE_BYTES {
            return Err(failed_policy_result(format!(
                "Policy bundle exceeds the {GATE_MAX_BUNDLE_BYTES}-byte total limit at {}.",
                path.display(),
            )));
        }

        let source = std::fs::read_to_string(&canonical_path).map_err(|e| {
            failed_policy_result(format!("Failed to read policy {}: {e}", path.display()))
        })?;
        let name = canonical_path
            .strip_prefix(canonical_root)
            .unwrap_or(&canonical_path)
            .display()
            .to_string();
        // Fail-fast pack admission: a policy that fails to compile is a
        // reported check failure, never a silent skip.
        engine
            .add_policy(name.clone(), source)
            .map_err(|e| failed_policy_result(format!("Policy failed to compile ({name}): {e}")))?;
        evaluated += 1;
    }
    Ok(evaluated)
}

fn run_check_policy(
    project_root: &Path,
    profile: Option<&str>,
    plan_path: Option<&str>,
    plan_files: &std::collections::HashSet<String>,
    all_files: Option<&[String]>,
) -> CheckResult {
    let policy_dir = project_root.join(".anvil/policies");

    if !policy_dir.exists() || !policy_dir.is_dir() {
        return passing_policy_result(
            "No policy bundle found (.anvil/policies/). Skipping.".to_string(),
        );
    }

    // Path containment: canonicalise the workspace root and the policy dir and
    // refuse to evaluate a bundle that resolves outside the root (e.g. a cloned
    // repo shipping `.anvil/policies` as a symlink to an arbitrary host path).
    // Fail-fast, never a silent skip — a breach must be visible.
    let canonical_root = match project_root.canonicalize() {
        Ok(root) => root,
        Err(e) => {
            return failed_policy_result(format!(
                "Failed to resolve workspace root {}: {e}",
                project_root.display()
            ));
        }
    };
    let canonical_dir = match policy_dir.canonicalize() {
        Ok(dir) => dir,
        Err(e) => {
            return failed_policy_result(format!(
                "Failed to resolve policy directory {}: {e}",
                policy_dir.display()
            ));
        }
    };
    if !canonical_dir.starts_with(&canonical_root) {
        return failed_policy_result(format!(
            "Policy directory {} resolves outside the workspace root {} \
             (path containment breach); refusing to evaluate.",
            policy_dir.display(),
            project_root.display()
        ));
    }

    // Discovery: *.rego minus *_test.rego (and generated output), deterministic
    // order. A traversal error is a reported failure, not a partial pass. An
    // empty bundle has nothing to evaluate — a clean skip, as today.
    let policy_files = match discover_policy_files(&canonical_dir) {
        Ok(files) => files,
        Err(result) => return result,
    };
    if policy_files.is_empty() {
        return passing_policy_result(
            "No policies found in .anvil/policies/. Skipping.".to_string(),
        );
    }

    // regorus is embedded via the policy-engine facade, so evaluation always
    // runs when a bundle exists — there is no host `opa` binary to be missing
    // and no silent skip on its absence.
    let mut engine = match Engine::new(EngineConfig::default()) {
        Ok(engine) => engine,
        Err(e) => return failed_policy_result(format!("Policy engine unavailable: {e}")),
    };
    if let Err(e) = anvil_policy_engine::builtins::register_all(&mut engine) {
        return failed_policy_result(format!("Policy engine setup failed: {e}"));
    }

    let evaluated = match admit_policy_files(&mut engine, &policy_files, &canonical_root) {
        Ok(evaluated) => evaluated,
        Err(result) => return result,
    };

    let input = policy_input_from_gate(project_root, profile, plan_path, plan_files, all_files);
    let value = match engine.eval(&input, "data.anvil.policies") {
        Ok(result) => result.value,
        Err(e) => return failed_policy_result(format!("Policy evaluation failed: {e}")),
    };

    let findings = value
        .as_ref()
        .map(extract_policy_findings)
        .unwrap_or_default();
    let (errors, warnings): (Vec<_>, Vec<_>) =
        findings.into_iter().partition(|f| f.severity == "error");

    let render = |f: &PolicyFinding| format!("[{}] {}: {}", f.severity, f.policy_id, f.message);

    if !errors.is_empty() {
        // Warnings never block (warnings-over-blocks); list them after the
        // blocking violations so the failure message stays complete.
        let mut lines: Vec<String> = errors.iter().map(render).collect();
        lines.extend(warnings.iter().map(render));
        failed_policy_result(format!(
            "{} violation(s):\n{}",
            errors.len(),
            lines.join("\n")
        ))
    } else if !warnings.is_empty() {
        let lines: Vec<String> = warnings.iter().map(render).collect();
        passing_policy_result(format!(
            "{evaluated} policies evaluated, {} warning(s):\n{}",
            warnings.len(),
            lines.join("\n")
        ))
    } else {
        passing_policy_result(format!("{evaluated} policies evaluated, no violations"))
    }
}

/// Test-only bridge to the private policy gate check for the cross-module
/// starter-pack end-to-end proof (`commands::policy::starter_proof`), which
/// lives outside this module and so cannot reach [`run_check_policy`]
/// directly. Returns the check's `(passed, message)`. Compiled only under
/// test — nothing here ships in the binary.
#[cfg(test)]
pub(crate) fn run_policy_check_for_proof(
    project_root: &Path,
    all_files: &[String],
) -> (bool, String) {
    let result = run_check_policy(
        project_root,
        None,
        None,
        &std::collections::HashSet::new(),
        Some(all_files),
    );
    (result.passed, result.message)
}

fn run_single_check(name: &str, ctx: &GateContext) -> CheckResult {
    let root = &ctx.workspace_root;

    // OPSUP-006 — file-presence guard. A check that declares file-shape
    // patterns short-circuits when none of the walked workspace files
    // match. All current core checks declare none and therefore always
    // run (Unguarded). Surface/pack checks added under Track 3 and Track
    // 4 will opt in by populating `file_shape_globs` on their
    // CheckDefinition.
    let definition = definition_by_internal(name);
    if let Some(def) = definition {
        let presence = evaluate_file_presence(def.file_shape_globs, &ctx.walked_files);
        if !presence.should_run() {
            // Use a "No files in scope" prefix rather than "Skipping" so
            // the file-presence short-circuit cannot be mistaken for the
            // missing-config skip pattern that `is_skipped_for_missing_config`
            // matches on. The two are semantically different (no work to
            // do vs. no config to evaluate) and the AI strict-config mode
            // must only elevate the latter.
            return CheckResult {
                name: gate_canonical_name_from_internal(name),
                passed: true,
                score: 100.0,
                message: format!(
                    "No files in scope for {}: no workspace files match declared shapes ({})",
                    def.canonical_name,
                    def.file_shape_globs.join(", "),
                ),
                requires_config: false,
            };
        }
    }

    // OPSUP-006 — measure elapsed wall-time so the post-flight guard can
    // surface a precise overrun reason. Side-effect-free if no budget is
    // declared (every current core check).
    let started = std::time::Instant::now();

    let mut result = match name {
        "lint" => run_check_lint(name, root),
        "test" => run_check_test(name, root),
        "antipattern-scan" => {
            run_check_antipattern(name, root, &ctx.plan_files, ctx.fail_on_warnings)
        }
        "secret" => run_check_secret(name, root, &ctx.plan_files),
        "sql-migrations" => run_check_sql_migrations(name, root, &ctx.walked_files),
        "github-actions" => run_check_github_actions(name, root, &ctx.walked_files),
        "dockerfile" => run_check_dockerfile(name, root, &ctx.walked_files),
        "shell-scripts" => run_check_shell(name, root, &ctx.walked_files),
        "coverage" => run_check_coverage(root, DEFAULT_COVERAGE_THRESHOLD),
        "dependency" => run_check_dependency(root),
        "architecture" => run_check_architecture(root),
        "policy" => run_check_policy(
            root,
            ctx.profile.as_deref(),
            ctx.plan_path.as_deref(),
            &ctx.plan_files,
            Some(&ctx.walked_files),
        ),
        "command-safety" => run_check_command_safety(name, root, ctx.plan_path.as_deref()),
        _ => CheckResult {
            name: name.to_string(),
            passed: false,
            score: 0.0,
            message: format!("Unknown check: {name}"),
            requires_config: false,
        },
    };

    if let Some(def) = definition {
        let wall_time = evaluate_wall_time(def.wall_time_soft_budget_secs, started.elapsed());
        if let WallTimeGuard::Exceeded { .. } = &wall_time
            && let Some(reason) = wall_time.timeout_reason()
        {
            // Append the timeout reason so the overrun is surfaced
            // without losing the check's own message. The check itself is
            // not cancelled — Rust threads cannot be safely pre-empted —
            // but the reason makes the budget breach actionable.
            if result.message.is_empty() {
                result.message = format!("{}: {}", def.canonical_name, reason);
            } else {
                result.message = format!("{} ({reason})", result.message);
            }
        }
    }

    // AI guardrail strict-config (CIB-011 / #1803): missing/invalid
    // config marks the check as a config-gap with an actionable
    // `next:` hint, rather than flipping the soft skip into a hard
    // FAIL. Score is graded against **available** checks — a fresh
    // repo with no project config and no actual violations reads as
    // a green run with three config-needed notifications, not a 20%
    // score that screams "anvil is broken".
    //
    // Architecture and policy checks return passed=true with a
    // "Skipping" message when their config files are absent — that's
    // the precise signal we mark as config-gap.
    if ctx.strict_config && result.passed && is_skipped_for_missing_config(name, &result.message) {
        result.requires_config = true;
        result.message = format!("{}\n  next: {}", result.message, config_gap_next_hint(name));
    }

    result.name = gate_canonical_name_from_internal(name);
    result
}

/// Detect the canonical "no project config found, skipping" signal
/// emitted by architecture, policy, and command-safety checks. Used by
/// the AI guardrail's strict-config flag (CIB-011 / #1803) to mark
/// the check as a **config-gap** (rendered as `CONFIG NEEDED` with a
/// `next:` hint, excluded from the score denominator) rather than to
/// flip the soft skip into a hard FAIL. The pre-CIB-011 behaviour was
/// to elevate to a blocking diagnostic; that produced a "1/5 passed,
/// score: 20%" UX on fresh repos and is no longer the contract.
///
/// This intentionally distinguishes **missing project config** (which
/// strict mode marks as a config-gap) from **missing host tooling**
/// like a missing OPA binary (which is an environment problem, not a
/// project posture problem — left as a normal soft skip). The two
/// were previously conflated via a substring match on "Skipping",
/// with the result that any developer or CI runner without OPA in
/// PATH would get a blocked AI-guardrail run.
fn is_skipped_for_missing_config(name: &str, message: &str) -> bool {
    match name {
        "architecture" => message.contains("Skipping"),
        "policy" => {
            // OPA-not-installed is host tooling, not project config — do
            // not mark it as a config-gap under strict mode.
            message.contains("Skipping") && !message.contains("OPA not installed")
        }
        "command-safety" => {
            // Two project-config gaps map to a strict-mode config-gap:
            //   * "Skipping" — the check is disabled via config.
            //   * "No commands to analyse" — the gate ran without a plan
            //     file at all, so the command-safety guarantee is empty.
            message.contains("Skipping") || message.contains("No commands to analyse")
        }
        _ => false,
    }
}

/// Dispatch the command-safety check from `anvil-checks`.
///
/// The plan file (if any) is parsed for fenced shell-script blocks; the
/// commands extracted are evaluated against the default rule set. When
/// no plan is provided the check has nothing to evaluate and reports as
/// skipped (passed with a clear message).
fn run_check_command_safety(name: &str, root: &Path, plan_path: Option<&str>) -> CheckResult {
    use anvil_checks::command_safety::{
        CommandSafetyCheckContext, run_command_safety_check,
        types::{ScriptChange, ScriptChangeType, ScriptPlan},
    };

    let plan = match plan_path {
        Some(raw) => {
            let path = Path::new(raw);
            match std::fs::read_to_string(path) {
                Ok(content) => Some(ScriptPlan {
                    proposed_changes: vec![ScriptChange {
                        change_type: ScriptChangeType::ScriptExecute,
                        description: Some(content),
                        path: Some(raw.to_string()),
                    }],
                }),
                Err(e) => {
                    // Treat unreadable plans as a check failure rather than
                    // silently passing as "no commands to analyse" — under
                    // --profile ai (strict mode) this would otherwise mask
                    // permission errors and CI-only IO failures behind a
                    // green gate.
                    return CheckResult {
                        name: name.to_string(),
                        passed: false,
                        score: 0.0,
                        message: format!("failed to read plan file '{}': {e}", path.display()),
                        requires_config: false,
                    };
                }
            }
        }
        None => None,
    };

    let context = CommandSafetyCheckContext {
        plan,
        check_config: None,
        workspace_root: Some(root.to_string_lossy().into_owned()),
    };

    let result = run_command_safety_check(&context);

    if result.skipped {
        return CheckResult {
            name: name.to_string(),
            passed: true,
            score: 100.0,
            message: "Command-safety check disabled. Skipping.".to_string(),
            requires_config: false,
        };
    }

    if result.passed {
        let message = if result.message.is_empty() {
            "No unsafe commands detected".to_string()
        } else {
            result.message
        };
        CheckResult {
            name: name.to_string(),
            passed: true,
            score: f64::from(result.score),
            message,
            requires_config: false,
        }
    } else {
        let mut details: Vec<String> = result
            .blocked
            .iter()
            .map(|f| {
                format!(
                    "[blocked:{}] {} \u{2014} {}",
                    f.rule_id, f.command, f.reason
                )
            })
            .collect();
        details.extend(
            result
                .warnings
                .iter()
                .map(|f| format!("[warn:{}] {} \u{2014} {}", f.rule_id, f.command, f.reason)),
        );

        let header = if result.message.is_empty() {
            format!(
                "{} blocked, {} warning(s)",
                result.blocked.len(),
                result.warnings.len()
            )
        } else {
            result.message
        };

        let message = if details.is_empty() {
            header
        } else {
            format!("{header}\n{}", details.join("\n"))
        };

        CheckResult {
            name: name.to_string(),
            passed: false,
            score: f64::from(result.score),
            message,
            requires_config: false,
        }
    }
}

fn list_profiles() {
    println!();
    println!("Available Gate Profiles");
    println!();
    for (name, desc, skips) in PROFILES {
        println!("  {name}");
        println!("    {desc}");
        if !skips.is_empty() {
            println!("    Skips: {}", skips.join(", "));
        }
        println!();
    }
    println!("Usage: anvil gate [plan] --profile <name>");
}

fn resolve_profile_skips(profile: Option<&str>) -> Result<std::collections::HashSet<&str>> {
    let Some(name) = profile else {
        return Ok(std::collections::HashSet::new());
    };
    for (pname, _, skips) in PROFILES {
        if *pname == name {
            return Ok(skips.iter().copied().collect());
        }
    }
    let valid: Vec<&str> = PROFILES.iter().map(|(n, _, _)| *n).collect();
    bail!(
        "unknown profile '{name}', valid profiles: {}",
        valid.join(", ")
    );
}

/// Resolve a profile's skip list to canonical gate-runner internal names.
///
/// Profile skip-list entries can use either canonical names like
/// `secret-detection` or internal names like `secret`. Routing them
/// through [`gate_internal_name`] guarantees `--profile <name>` and
/// `--skip-checks <name>` use the same vocabulary downstream.
///
/// Any entry that does not resolve through the catalog is treated as a
/// hard error rather than silently dropped — a typo in `PROFILES` (or
/// any future profile definition) used to fail open, letting the
/// supposedly-skipped check run anyway.
fn resolve_profile_skip_set(
    profile: Option<&str>,
) -> Result<std::collections::HashSet<&'static str>> {
    let raw = resolve_profile_skips(profile)?;
    let invalid: Vec<&str> = raw
        .iter()
        .copied()
        .filter(|name| gate_internal_name(name).is_none())
        .collect();
    if !invalid.is_empty() {
        let mut sorted = invalid;
        sorted.sort_unstable();
        bail!(
            "profile '{}' references unknown check name(s): {}",
            profile.unwrap_or("<none>"),
            sorted.join(", ")
        );
    }
    Ok(raw.iter().filter_map(|n| gate_internal_name(n)).collect())
}

/// Canonical check names included in the AI guardrail profile, expressed
/// as gate-runner internal names. Acts as an allow-list when
/// `--profile ai` is selected so the runner never executes a check
/// outside the curated set, even if the project's `.anvilrc` would
/// otherwise enable it.
fn ai_guardrail_only_set() -> Result<std::collections::HashSet<&'static str>> {
    let names: std::collections::HashSet<&str> =
        ai_guardrail_profile_checks().iter().copied().collect();
    normalize_gate_check_set(&names)
}

/// Read the project's `checks` filter, preferring MLP-011's multi-format
/// `.anvil.<ext>` (yaml/yml/json/toml) discovery and falling back to the
/// legacy `.anvilrc` for projects that have not migrated yet.
///
/// Returns `Ok(None)` when no config file is found, no `checks` field is
/// present, or the list is empty. Parsing or shape errors are surfaced so
/// gate can fail clearly instead of silently acting on a malformed filter.
///
/// `pub(crate)` so the planless `anvil check` dispatcher in `commands/check.rs`
/// can share the same discovery + parsing path as gate (issue #1797).
pub(crate) fn read_anvilrc_checks(
    workspace_root: &Path,
) -> Result<Option<std::collections::HashSet<String>>> {
    // MLP2-040 — prefer `.anvil.<ext>` via MLP-011's `discover` precedence
    // (yaml → yml → json → toml). When discover finds nothing, we fall
    // back to the legacy `.anvilrc` reader below.
    if let Some(discovered) = anvil_config::discover(workspace_root, ".anvil")
        .with_context(|| format!("scanning {} for .anvil.<ext>", workspace_root.display()))?
    {
        let value = anvil_config::parse_file(&discovered.path)
            .with_context(|| format!("failed to parse {}", discovered.path.display()))?;
        return finalise_checks_from_value(&value);
    }

    // Legacy `.anvilrc` fallback. Format detection mirrors the pre-MLP2-040
    // behaviour: try JSON, TOML, then YAML in order. The first parser that
    // produces an object wins. This path is the deprecation tail; new
    // projects land via `.anvil.<ext>` instead.
    let path = workspace_root.join(".anvilrc");
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(anyhow::anyhow!("failed to read {}: {err}", path.display())),
    };

    let value = parse_anvilrc_contents(&contents, &path)?;
    finalise_checks_from_value(&value)
}

/// CIB-199: workspace-relative exclude globs declared under `antipattern.exclude`
/// in the project config (`.anvil.<ext>` or legacy `.anvilrc`). Files matching
/// these are skipped by the anti-pattern scan, letting users declare generated
/// paths whose naming convention or banner the auto-detector does not recognise.
/// Missing config, missing key, or non-array value all yield an empty list.
pub(crate) fn read_anvilrc_antipattern_excludes(workspace_root: &Path) -> Result<Vec<String>> {
    let value = if let Some(discovered) = anvil_config::discover(workspace_root, ".anvil")
        .with_context(|| format!("scanning {} for .anvil.<ext>", workspace_root.display()))?
    {
        anvil_config::parse_file(&discovered.path)
            .with_context(|| format!("failed to parse {}", discovered.path.display()))?
    } else {
        let path = workspace_root.join(".anvilrc");
        match std::fs::read_to_string(&path) {
            Ok(contents) => parse_anvilrc_contents(&contents, &path)?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(anyhow::anyhow!("failed to read {}: {err}", path.display())),
        }
    };

    let globs = value
        .get("antipattern")
        .and_then(|antipattern| antipattern.get("exclude"))
        .and_then(serde_json::Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Ok(globs)
}

fn finalise_checks_from_value(
    value: &serde_json::Value,
) -> Result<Option<std::collections::HashSet<String>>> {
    let view = crate::config_view::GateConfigView::from_value(value)
        .map_err(|e| anyhow::anyhow!("invalid config: {e}"))?;
    if view.checks.is_empty() {
        return Ok(None);
    }
    let canonical: std::collections::HashSet<String> = view
        .checks
        .into_iter()
        .map(|name| canonical_check_name(&name).unwrap_or(&name).to_string())
        .collect();
    Ok(Some(canonical))
}

fn parse_anvilrc_contents(contents: &str, path: &Path) -> Result<serde_json::Value> {
    for format in [
        anvil_config::ConfigFormat::Json,
        anvil_config::ConfigFormat::Toml,
        anvil_config::ConfigFormat::Yaml,
    ] {
        if let Ok(value) = anvil_config::parse_str(contents, format, path)
            && value.is_object()
        {
            return Ok(value);
        }
    }
    Err(anyhow::anyhow!(
        "failed to parse {} as JSON, YAML, or TOML",
        path.display()
    ))
}

fn validate_check_names(names: &std::collections::HashSet<&str>) -> Result<()> {
    let mut unknown: Vec<&str> = names
        .iter()
        .copied()
        .filter(|n| gate_internal_name(n).is_none())
        .collect();
    if !unknown.is_empty() {
        // Deterministic ordering so the error message is stable across runs
        // (the input is a HashSet).
        unknown.sort_unstable();
        let available = gate_canonical_names();
        bail!(
            "unknown check(s): {}; available: {}",
            unknown
                .iter()
                .map(|n| describe_unknown_check(n))
                .collect::<Vec<_>>()
                .join(", "),
            available.join(", ")
        );
    }
    Ok(())
}

/// Render an unknown check identifier with a registry-backed "did you mean…"
/// suggestion when one is near enough (OPSUP-002). Falls back to the bare name
/// when nothing in the registry is close.
fn describe_unknown_check(name: &str) -> String {
    match closest_registered_id(name) {
        Some(suggestion) => format!("'{name}' (did you mean '{suggestion}'?)"),
        None => format!("'{name}'"),
    }
}

fn normalize_gate_check_set(
    names: &std::collections::HashSet<&str>,
) -> Result<std::collections::HashSet<&'static str>> {
    validate_check_names(names)?;
    Ok(names
        .iter()
        .filter_map(|name| gate_internal_name(name))
        .collect())
}

/// Resolve the `.anvilrc#checks` filter into a set of gate-runner internal
/// names. Canonical names like `secret-detection` are mapped to their
/// internal form (`secret`) so they match `GATE_INTERNAL_CHECKS` in the
/// downstream dispatch loop. Returns `None` when `--only-checks` is set
/// (explicit flag wins) or when `.anvilrc#checks` is absent/empty.
///
/// CIB-089 deliberately preserves the compatibility posture for project config:
/// unknown config entries warn and the known subset still runs, so a stale or
/// forward-looking `.anvilrc` does not brick every local/CI gate. Explicit CLI
/// filters (`--only-checks` / `--skip-checks`) remain strict and fatal because
/// they describe one invocation and can be corrected immediately.
fn resolve_anvilrc_check_filter(
    root: &Path,
    only_set: Option<&std::collections::HashSet<&'static str>>,
) -> Result<Option<std::collections::HashSet<String>>> {
    if only_set.is_some() {
        return Ok(None);
    }

    let anvilrc_checks = read_anvilrc_checks(root)?;
    if let Some(ref rc) = anvilrc_checks {
        let unknown: Vec<&str> = rc
            .iter()
            .filter(|n| gate_internal_name(n).is_none())
            .map(String::as_str)
            .collect();
        if !unknown.is_empty() {
            eprintln!("{}", format_anvilrc_unknown_checks_warning(&unknown));
        }

        // Map each known name to its internal form so it matches the
        // gate runner vocabulary (GATE_INTERNAL_CHECKS).
        let known: std::collections::HashSet<String> = rc
            .iter()
            .filter_map(|n| gate_internal_name(n).map(str::to_string))
            .collect();
        if known.is_empty() {
            let valid = gate_canonical_names();
            bail!(
                ".anvilrc#checks contains no valid gate checks. Valid: {}",
                valid.join(", ")
            );
        }
        return Ok(Some(known));
    }

    Ok(None)
}

fn format_anvilrc_unknown_checks_warning(unknown: &[&str]) -> String {
    let mut unknown = unknown.to_vec();
    unknown.sort_unstable();
    let valid = gate_canonical_names();
    format!(
        "Warning: .anvilrc#checks contains unknown check(s): {}. Known checks will still run. Valid: {}",
        unknown
            .iter()
            .map(|n| describe_unknown_check(n))
            .collect::<Vec<_>>()
            .join(", "),
        valid.join(", ")
    )
}

/// Run all gate checks with default settings and return TUI-ready data.
pub fn collect_gate_data() -> anvil_tui::surfaces::gate::GateResult {
    let start = std::time::Instant::now();
    let default_args = GateArgs::default();
    let checks = run_checks(&default_args).unwrap_or_default();

    let passed_count = checks.iter().filter(|c| c.passed).count();
    let total = checks.len();
    let overall = checks.iter().all(|c| c.passed);
    #[allow(clippy::cast_precision_loss)]
    let score = if total > 0 {
        passed_count as f64 / total as f64
    } else {
        1.0
    };
    let elapsed = start.elapsed().as_millis();

    let tui_checks: Vec<anvil_tui::surfaces::gate::GateCheck> = checks
        .into_iter()
        .map(|c| {
            let status = if c.passed {
                anvil_tui::surfaces::gate::GateCheckStatus::Passed
            } else {
                anvil_tui::surfaces::gate::GateCheckStatus::Failed
            };
            anvil_tui::surfaces::gate::GateCheck {
                id: c.name.clone(),
                name: c.name,
                status,
                score: c.score / 100.0,
                message: c.message,
                details: None,
                file: None,
                line: None,
            }
        })
        .collect();

    anvil_tui::surfaces::gate::GateResult {
        plan_id: "cli".to_string(),
        overall_passed: overall,
        score,
        checks: tui_checks,
        duration_ms: u64::try_from(elapsed).unwrap_or(u64::MAX),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

/// Resolved gate context from CLI arguments.
struct GateContext {
    workspace_root: PathBuf,
    profile: Option<String>,
    /// Files referenced by the plan (empty = full codebase scan).
    plan_files: std::collections::HashSet<String>,
    /// Path to the plan file, if provided.
    plan_path: Option<String>,
    /// All workspace files (walked once, shared across checks).
    walked_files: Vec<String>,
    /// When true (set by `--profile ai`), missing or invalid config is
    /// treated as a blocking diagnostic rather than a soft warning.
    strict_config: bool,
    /// When true (`--fail-on-warnings` / `ANVIL_FAIL_ON_WARNINGS`), warning-
    /// severity anti-pattern findings block the gate (ADR-112 / ADR-002 opt-in).
    fail_on_warnings: bool,
}

/// True when `check_name` is a flag-gated Track 3 surface whose
/// `track.surface.*` flag is currently off. Such a surface is omitted from the
/// run entirely (no result emitted) so opt-out gate runs stay byte-identical.
fn surface_check_disabled(check_name: &str) -> bool {
    use crate::feature_flags::{
        track_surface_dock_enabled, track_surface_gha_enabled, track_surface_sh_enabled,
        track_surface_sql_enabled,
    };
    match check_name {
        "sql-migrations" => !track_surface_sql_enabled(),
        "github-actions" => !track_surface_gha_enabled(),
        "dockerfile" => !track_surface_dock_enabled(),
        "shell-scripts" => !track_surface_sh_enabled(),
        _ => false,
    }
}

/// True when env var `name` holds a truthy value (present, non-empty, and not
/// `0`/`false`/`no`/`off`). Mirrors `install_root::is_truthy` locally so the
/// gate does not widen that module's visibility.
fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| {
        let value = value.trim();
        !value.is_empty()
            && !matches!(
                value.to_ascii_lowercase().as_str(),
                "0" | "false" | "no" | "off"
            )
    })
}

fn run_checks(args: &GateArgs) -> Result<Vec<CheckResult>> {
    let root = crate::util::workspace_root()?;

    // Profile skip lists are canonicalised through `gate_internal_name`
    // so entries declared as canonical names (`secret-detection`) and
    // internal names (`secret`) both resolve consistently against
    // `GATE_INTERNAL_CHECKS`. AIGUARD-003 lifted the literal-skip-list
    // constraint that PR #1097 deferred.
    let profile_skip_set = resolve_profile_skip_set(args.profile.as_deref())?;

    let skip_names: std::collections::HashSet<&str> = args
        .skip_checks
        .as_deref()
        .map(|s| s.split(',').map(str::trim).collect())
        .unwrap_or_default();
    let mut skip_set = normalize_gate_check_set(&skip_names)?;
    skip_set.extend(profile_skip_set.iter().copied());

    let only_names: Option<std::collections::HashSet<&str>> = args
        .only_checks
        .as_deref()
        .map(|s| s.split(',').map(str::trim).collect());
    let mut only_set = only_names
        .as_ref()
        .map(normalize_gate_check_set)
        .transpose()?;

    // `--profile ai` selects from `AI_GUARDRAIL_CHECKS` as an allow-list
    // (intersected with any explicit `--only-checks` if both were
    // supplied) so the curated rule set is the floor, not just an
    // inverse skip list. Mirrors the path `--only-checks` already takes.
    if args.profile.as_deref() == Some(AiGuardrailProfile::NAME) {
        let ai_only = ai_guardrail_only_set()?;
        only_set = Some(match only_set.take() {
            Some(existing) => existing.intersection(&ai_only).copied().collect(),
            None => ai_only,
        });
    }

    // `.anvilrc#checks` acts as a persistent default filter. When the user
    // passes `--only-checks`, that wins — but otherwise we restrict the run
    // to whatever the project configured. Missing/empty file = run everything.
    let anvilrc_known_checks = resolve_anvilrc_check_filter(&root, only_set.as_ref())?;

    // Resolve plan-scoped file set.
    let (plan_files, plan_path) = if let Some(ref plan_arg) = args.plan {
        match resolve_plan_path(plan_arg, &root) {
            Some(path) => {
                let files = extract_plan_files(&path);
                if args.progress {
                    eprintln!(
                        "  \u{2139} plan scope: {} files from {}",
                        files.len(),
                        path.display()
                    );
                }
                (files, Some(path.to_string_lossy().to_string()))
            }
            None => {
                bail!("plan file not found: {plan_arg}");
            }
        }
    } else {
        (std::collections::HashSet::new(), None)
    };

    // Walk workspace files once — shared across architecture and policy checks.
    let walked_files = walk_source_files(&root, &[]);

    let strict_config = args.profile.as_deref() == Some(AiGuardrailProfile::NAME)
        && AiGuardrailProfile::DEFAULT.strict_config;

    // ADR-112: warnings block only when explicitly opted in, via the flag or
    // the `ANVIL_FAIL_ON_WARNINGS` env var (any non-empty, non-"0"/"false"
    // value). Errors always block regardless.
    let fail_on_warnings = args.fail_on_warnings || env_flag_enabled("ANVIL_FAIL_ON_WARNINGS");

    let ctx = GateContext {
        workspace_root: root,
        profile: args.profile.clone(),
        plan_files,
        plan_path,
        walked_files,
        strict_config,
        fail_on_warnings,
    };

    let mut checks = Vec::new();
    for check_name in GATE_INTERNAL_CHECKS {
        if skip_set.contains(check_name) {
            continue;
        }
        if let Some(ref only_s) = only_set
            && !only_s.contains(check_name)
        {
            continue;
        }
        if let Some(ref rc) = anvilrc_known_checks
            && !rc.contains(*check_name)
        {
            continue;
        }
        // Track 3 surface opt-in (OPSUP-005): a flag-gated surface is omitted
        // from the run entirely while its track flag is off — it emits NO
        // result, so the default gate run (check count, score denominator,
        // output) is byte-identical for anyone who has not opted in.
        if surface_check_disabled(check_name) {
            continue;
        }

        let display_name = gate_canonical_name_from_internal(check_name);

        if args.progress {
            eprintln!("  \u{25b6} {display_name} running...");
        }

        let result = run_single_check(check_name, &ctx);

        if args.progress {
            let icon = if result.passed {
                "\u{2713}"
            } else {
                "\u{2717}"
            };
            eprintln!("  {icon} {display_name}");
        }

        let failed = !result.passed;
        checks.push(result);

        if args.fail_fast && failed {
            break;
        }
    }
    Ok(checks)
}

/// AI guardrail return-value envelope (`anvil.gate-result.v1`).
///
/// Wraps a list of canonical [`Diagnostic`] payloads — the inner shape
/// pinned by AIGUARD-002 / `anvil.diagnostic.v1` — with a summary and
/// exit code so external AI consumers can branch without re-deriving
/// counts from `diagnostics[]`. Per the diagnostic-envelope spec at
/// `plans/specs/2026-04-26-diagnostic-envelope-coordination.md`.
///
/// **v1 extension policy:** the schema string stays `v1` for
/// backwards-compatible additive fields (fields skipped from output
/// when at their default value). CIB-011 added `summary.config_gaps`
/// under this policy — existing strict consumers on a
/// fully-configured repo see no shape change, and consumers using
/// `#[serde(deny_unknown_fields)]` who hit a partial-config repo
/// will need to update to v1.1+ semantics. Breaking changes (renames,
/// removals, type changes) require a v2 schema string.
#[derive(Debug, Serialize)]
struct AiGateResultEnvelope {
    schema: &'static str,
    exit_code: u8,
    summary: AiGateResultSummary,
    diagnostics: Vec<Diagnostic>,
    duration_ms: u64,
}

#[derive(Debug, Serialize)]
struct AiGateResultSummary {
    total: usize,
    by_severity: AiGateBySeverity,
    by_category: std::collections::BTreeMap<String, usize>,
    overall_passed: bool,
    score: f64,
    /// CIB-011 / #1803 — number of checks the gate could not run
    /// because their project config is missing under strict mode
    /// (e.g. no `.anvil/architecture.yaml`). These do not count
    /// toward `total` (they are not failures), but consumers may
    /// want to surface them to the user as "configure these next".
    /// Skipped from JSON output when zero so the schema stays
    /// additive — existing v1 consumers reading the envelope on a
    /// fully-configured repo see no shape change.
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    config_gaps: usize,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero_usize(n: &usize) -> bool {
    *n == 0
}

#[derive(Debug, Serialize)]
struct AiGateBySeverity {
    error: usize,
    warning: usize,
    info: usize,
}

fn build_ai_gate_result_envelope(result: &GateResult) -> AiGateResultEnvelope {
    // CIB-011 / #1803 — diagnostics are real failures only. Config-gap
    // checks stay passed=true so they are filtered out here as well as
    // by the `!c.passed` clause; surfacing them via `summary.config_gaps`
    // lets consumers count them without re-deriving from `result.checks`.
    let diagnostics: Vec<Diagnostic> = result
        .checks
        .iter()
        .filter(|c| !c.passed && !c.requires_config)
        .map(check_result_to_diagnostic)
        .collect();

    let config_gaps = result.checks.iter().filter(|c| c.requires_config).count();

    let mut by_category: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut error_count: usize = 0;
    for diag in &diagnostics {
        let cat_key = serde_json::to_value(diag.category)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "other".to_string());
        *by_category.entry(cat_key).or_insert(0) += 1;
        if matches!(diag.severity, Severity::Error) {
            error_count += 1;
        }
    }

    AiGateResultEnvelope {
        schema: "anvil.gate-result.v1",
        exit_code: if result.overall { 0 } else { 2 },
        summary: AiGateResultSummary {
            total: diagnostics.len(),
            by_severity: AiGateBySeverity {
                error: error_count,
                warning: 0,
                info: 0,
            },
            by_category,
            overall_passed: result.overall,
            score: result.score,
            config_gaps,
        },
        diagnostics,
        duration_ms: result.duration_ms,
    }
}

/// Map a failed [`CheckResult`] to a canonical `anvil.diagnostic.v1`
/// payload for the AI guardrail envelope. The outer envelope sets
/// `mode = "gate"` and the file anchor is the workspace root when no
/// per-finding location is available — diagnostics that need
/// finer-grained location data are emitted by the underlying check
/// itself in future work.
fn check_result_to_diagnostic(check: &CheckResult) -> Diagnostic {
    let category = check_name_to_category(&check.name);
    let rule_id = format!("gate-{}", check.name);
    let id = format!("diag_gate_{}", check.name);

    Diagnostic::new(
        id,
        Severity::Error,
        check.message.lines().next().unwrap_or("").to_string(),
        Location {
            file: "<workspace>".to_string(),
            line: None,
            column: None,
            end_line: None,
            end_column: None,
        },
        category,
        DiagnosticSource {
            rule_id,
            source_module: format!("anvil-cli::gate::{}", check.name),
        },
        Mode::known(KnownMode::Gate),
    )
    .with_remediation_hint(check.message.clone())
}

fn check_name_to_category(name: &str) -> Category {
    match name {
        "secret-detection" => Category::Secret,
        "antipattern-scan" => Category::Antipattern,
        "import-boundaries" => Category::Boundary,
        "architecture" => Category::Architecture,
        "policy" => Category::Policy,
        "command-safety" => Category::CommandSafety,
        _ => Category::Other,
    }
}

/// Build a SARIF document from gate results (SARIFOUT-005).
///
/// Gate findings are per-check aggregates, not per-location warnings, so each
/// emitted `result` is repo-level (no `locations[]`). Failed checks map to
/// `error`-level results; config-gap checks (`requires_config`) map to
/// `note`-level results so they surface without inflating the failure set;
/// passed checks are not findings and are omitted. `ruleId` is the check name.
/// SARIF emission does not affect the gate exit code.
fn build_gate_sarif(result: &GateResult) -> crate::output::sarif::SarifLog {
    use crate::output::sarif;

    let mut rules: BTreeMap<String, sarif::ReportingDescriptor> = BTreeMap::new();
    let mut results = Vec::new();
    for check in &result.checks {
        // A passing, fully-configured check is not a finding.
        if check.passed && !check.requires_config {
            continue;
        }
        let level = if check.requires_config {
            sarif::Level::Note
        } else {
            sarif::Level::Error
        };
        let message = if check.message.is_empty() {
            if check.requires_config {
                format!("{} requires configuration to run", check.name)
            } else {
                format!("{} did not pass", check.name)
            }
        } else {
            check.message.clone()
        };
        rules
            .entry(check.name.clone())
            .or_insert_with(|| sarif::ReportingDescriptor::new(check.name.clone()));
        results.push(
            sarif::SarifResult::new(check.name.clone(), level, message.clone()).fingerprint(
                "anvilFingerprint/v1",
                sarif::stable_fingerprint(&check.name, "", None, &message),
            ),
        );
    }
    sarif::SarifLog::new(sarif::Run::new(rules.into_values().collect(), results))
}

/// Resolve the gate command's output mode.
///
/// An explicit, non-`auto` `--format` wins outright (including over the
/// AI-guardrail JSON default). `--format auto` and an absent `--format` are
/// equivalent "use the defaults" requests: with the AI guardrail profile they
/// keep the JSON default (unless `--no-tui`), otherwise the legacy `--json` /
/// `--no-tui` / TTY resolver applies.
fn resolve_gate_output_mode(
    format: Option<crate::output::Format>,
    profile_is_ai: bool,
    ai_json_default: bool,
    global: &GlobalArgs,
    is_tty: bool,
) -> crate::output::OutputMode {
    use crate::output::{Format, OutputMode};
    match format {
        Some(f) if f != Format::Auto => {
            OutputMode::resolve_format(Some(f), global.json, global.no_tui, is_tty)
        }
        _ if profile_is_ai && ai_json_default && !global.no_tui => OutputMode::Json,
        _ => OutputMode::resolve(global.json, global.no_tui, is_tty),
    }
}

/// Run gate checks and return whether all gates passed.
///
/// Returns `Ok(true)` when every check passes and `Ok(false)` when at
/// least one check fails (caller maps this to `EXIT_GATE_FAIL`).
pub fn run(args: &GateArgs, global: &GlobalArgs) -> Result<bool> {
    use crate::output::OutputMode;

    if args.list_profiles {
        list_profiles();
        return Ok(true);
    }

    // The AI guardrail profile pins JSON output by default so AI
    // consumers reading the gate result get the documented schema
    // without a flag. Callers can still opt out with `--no-tui` (which
    // resolves to plain text) when they pass `--profile ai`.
    let mode = resolve_gate_output_mode(
        args.format,
        args.profile.as_deref() == Some(AiGuardrailProfile::NAME),
        AiGuardrailProfile::DEFAULT.json_output_default,
        global,
        std::io::stdout().is_terminal(),
    );

    let start = std::time::Instant::now();
    let checks = run_checks(args)?;

    // CIB-011 / #1803 — score and overall computed against available
    // checks only; config-gap checks (set under strict mode when a
    // required project config is missing) surface as info, not FAIL,
    // and do not bring the score down.
    let aggregate = aggregate_gate_outcome(&checks);
    let passed_count = aggregate.passed_count;
    let total = aggregate.available_total;
    let overall = aggregate.overall;
    let score = aggregate.score;

    let elapsed = start.elapsed().as_millis();
    let notifications = notifications_for_gate_result(&checks, overall);
    let result = GateResult {
        overall,
        score,
        checks,
        notifications,
        duration_ms: u64::try_from(elapsed).unwrap_or(u64::MAX),
    };

    // Persist the run for the `gate-summary` dashboard (#2242). Best-effort:
    // never affects the gate's exit code.
    persist_gate_snapshot(&result, &aggregate);

    match mode {
        OutputMode::Json => {
            if args.profile.as_deref() == Some(AiGuardrailProfile::NAME) {
                let envelope = build_ai_gate_result_envelope(&result);
                crate::output::json::print(&envelope)?;
            } else {
                crate::output::json::print(&result)?;
            }
        }
        OutputMode::Sarif => crate::output::json::print(&build_gate_sarif(&result))?,
        OutputMode::Plain | OutputMode::Tui => {
            // TUI surface for gate is not yet implemented; fall back to plain.
            use crate::output::plain;

            plain::header("Gate Results");
            plain::section("Checks");
            for check in &result.checks {
                if check.requires_config {
                    // CIB-011 — render config-gaps as INFO with their
                    // full message (which carries the `next:` hint).
                    plain::info(&format!("{:<20} CONFIG NEEDED", check.name));
                } else if check.passed {
                    plain::success(&format!("{:<20} PASS", check.name));
                } else {
                    plain::error(&format!("{:<20} FAIL", check.name));
                }
                let show_message = global.verbose || !check.passed || check.requires_config;
                if !check.message.is_empty() && show_message {
                    for line in check.message.lines() {
                        plain::dim(&format!("  {line}"));
                    }
                }
            }
            plain::blank();
            if overall {
                if aggregate.config_gaps > 0 {
                    plain::success(&format!(
                        "All available gates passed! ({passed_count}/{total} available, {} config-needed, score: {:.0}%)",
                        aggregate.config_gaps, result.score,
                    ));
                } else {
                    plain::success(&format!(
                        "All quality gates passed! (score: {:.0}%)",
                        result.score,
                    ));
                }
            } else {
                plain::error(&format!(
                    "Quality gates failed ({passed_count}/{total} passed, score: {:.0}%)",
                    result.score,
                ));
            }
        }
    }

    Ok(overall)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Wrapper {
        #[command(flatten)]
        inner: GateArgs,
    }

    fn git_for_hook_fixture(root: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git should be available for hook fixture");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn write_detectable_env(path: &Path) {
        let value = "abcd".repeat(10);
        std::fs::write(path, format!("aws_secret_access_key='{value}'\n")).unwrap();
    }

    fn raw_inventory_record(header: &str, path: &[u8]) -> Vec<u8> {
        let mut record = header.as_bytes().to_vec();
        record.push(0);
        record.extend_from_slice(path);
        record.push(0);
        record
    }

    fn valid_raw_inventory_header(status: &str) -> String {
        format!(
            ":000000 100644 {} {} {status}",
            "0".repeat(40),
            "1".repeat(40)
        )
    }

    #[test]
    fn hook_secret_check_fails_closed_when_inventory_is_globally_unavailable() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);

        let result = run_check_secret_with_hook_mode_and_provenance(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
            true,
        );

        assert!(!result.passed);
        assert!(
            result
                .message
                .contains("staged changes [staged content unavailable]")
        );
    }

    #[test]
    fn staged_inventory_rejects_unknown_status() {
        let raw = raw_inventory_record(&valid_raw_inventory_header("X"), b".env");
        assert!(parse_staged_gate_inventory(&raw).is_none());
    }

    #[test]
    fn staged_inventory_rejects_trailing_header_fields() {
        let raw = raw_inventory_record(&valid_raw_inventory_header("M trailing"), b".env");
        assert!(parse_staged_gate_inventory(&raw).is_none());
    }

    #[test]
    fn staged_inventory_rejects_malformed_header_and_score() {
        let malformed_mode = valid_raw_inventory_header("M").replacen(":000000", ":000008", 1);
        assert!(
            parse_staged_gate_inventory(&raw_inventory_record(&malformed_mode, b".env")).is_none()
        );
        assert!(
            parse_staged_gate_inventory(&raw_inventory_record(
                &valid_raw_inventory_header("R101"),
                b".env"
            ))
            .is_none()
        );
        assert!(GIT_RAW_INVENTORY_ARGS.contains(&"--no-abbrev"));
    }

    #[test]
    fn staged_inventory_keeps_invalid_bytes_distinct_from_valid_unicode() {
        let mut raw = raw_inventory_record(&valid_raw_inventory_header("A"), b".env.\xff");
        raw.extend(raw_inventory_record(
            &valid_raw_inventory_header("A"),
            ".env.�".as_bytes(),
        ));

        let inventory = parse_staged_gate_inventory(&raw).expect("inventory should be parsed");

        assert!(inventory.changes.contains_key(".env.�"));
        assert_eq!(inventory.quarantined_paths.len(), 1);
        assert!(
            inventory
                .quarantined_paths
                .iter()
                .all(|path| !path.contains('�'))
        );
        assert_eq!(
            strict_inventory_path(br".env\literal").unwrap(),
            r".env\literal"
        );
        assert!(parse_staged_gate_inventory(&raw[..raw.len() - 1]).is_none());
    }

    #[test]
    fn staged_inventory_escapes_control_path_diagnostics() {
        let mut control_path = b".env\n".to_vec();
        control_path.push(0x1b);
        control_path.extend_from_slice(b"[31m");
        let raw = raw_inventory_record(&valid_raw_inventory_header("A"), &control_path);

        let inventory = parse_staged_gate_inventory(&raw).expect("inventory should be parsed");
        let diagnostic = inventory
            .quarantined_paths
            .iter()
            .next()
            .expect("control path should be quarantined");

        assert!(diagnostic.contains('\n'));
        assert!(diagnostic.contains('\u{1b}'));
        let rendered_diagnostic = render_gate_path(diagnostic);
        assert!(!rendered_diagnostic.contains('\n'));
        assert!(!rendered_diagnostic.contains('\u{1b}'));
        assert!(rendered_diagnostic.contains(r"\n"));
        assert!(rendered_diagnostic.contains(r"\x1b"));
        let rendered = render_gate_path(".env\n\u{1b}[31m");
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert_eq!(rendered, r".env\n\x1b[31m");
    }

    #[test]
    fn hook_blob_budgets_bound_count_and_aggregate_bytes() {
        let mut used_count = 0;
        let mut used_bytes = 0;
        let admitted = (0..128)
            .filter(|_| blob_budget_admit(900, &mut used_count, &mut used_bytes, 64, 14_400))
            .count();
        assert_eq!(admitted, 16);

        let mut inspected = 0;
        let inspected_items = (0..2048)
            .filter(|_| inspect_next_blob(&mut inspected, 1024))
            .count();
        assert_eq!(inspected_items, 1024);
        assert_eq!(inspected, 1024);
    }

    #[test]
    fn hook_diff_reader_rejects_output_over_budget() {
        let bounded = read_to_limit(std::io::Cursor::new(vec![b'x'; 65]), 64);
        assert!(bounded.is_none());
        assert_eq!(
            read_to_limit(std::io::Cursor::new(vec![b'x'; 64]), 64),
            Some(vec![b'x'; 64])
        );
    }

    #[test]
    fn hook_worktree_finding_index_normalises_each_item_once() {
        let items = [".env", ".env", ".env.other"];
        let calls = std::cell::Cell::new(0);
        let index = index_positions_by_path(&items, |path| {
            calls.set(calls.get() + 1);
            (*path).to_string()
        });

        assert_eq!(calls.get(), items.len());
        assert_eq!(index[".env"].len(), 2);
        assert_eq!(index[".env.other"].len(), 1);
    }

    #[test]
    fn hook_secret_check_keeps_committed_occurrence_pre_existing_after_same_line_comment_edit() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let env_path = tmp.path().join(".env");
        let value = "abcd".repeat(10);
        std::fs::write(
            &env_path,
            format!("aws_secret_access_key='{value}' # committed\n"),
        )
        .unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);
        git_for_hook_fixture(
            tmp.path(),
            &[
                "-c",
                "user.name=anvil test",
                "-c",
                "user.email=anvil@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        std::fs::write(
            &env_path,
            format!("aws_secret_access_key='{value}' # staged comment\n"),
        )
        .unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(
            result
                .message
                .lines()
                .any(|line| line.contains(".env:1") && line.contains("pre-existing"))
        );
    }

    #[test]
    fn hook_secret_check_marks_oversized_head_mapping_indeterminate() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let env_path = tmp.path().join(".env.large-head");
        let value = "abcd".repeat(10);
        let prefix = format!("aws_secret_access_key='{value}'\n");
        std::fs::write(
            &env_path,
            format!(
                "{prefix}{}",
                "x".repeat(usize::try_from(MAX_STAGED_BLOB_SIZE).unwrap() - prefix.len())
            ),
        )
        .unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env.large-head"]);
        git_for_hook_fixture(
            tmp.path(),
            &[
                "-c",
                "user.name=anvil test",
                "-c",
                "user.email=anvil@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        std::fs::write(&env_path, prefix).unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env.large-head"]);

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(
            result
                .message
                .contains(".env.large-head [staged content unavailable]")
        );
        let finding = result
            .message
            .lines()
            .find(|line| line.contains(".env.large-head:1"))
            .unwrap();
        assert!(!finding.contains("pre-existing"));
    }

    #[test]
    fn hook_secret_check_leaves_partially_staged_modified_occurrence_unqualified() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let env_path = tmp.path().join(".env");
        std::fs::write(&env_path, "SAFE_VALUE=committed\n").unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);
        git_for_hook_fixture(
            tmp.path(),
            &[
                "-c",
                "user.name=anvil test",
                "-c",
                "user.email=anvil@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        let staged_value = format!("abcd{}wxyz", "1".repeat(32));
        std::fs::write(
            &env_path,
            format!("aws_secret_access_key='{staged_value}'\n"),
        )
        .unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);
        let worktree_value = format!("abcd{}wxyz", "2".repeat(32));
        std::fs::write(
            &env_path,
            format!("aws_secret_access_key='{worktree_value}'\n"),
        )
        .unwrap();

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(result.message.contains(".env [staged content unavailable]"));
        assert!(
            result
                .message
                .lines()
                .filter(|line| line.contains(".env:1"))
                .all(|line| !line.contains("pre-existing"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn hook_secret_check_escapes_control_path_in_finding_display() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let exotic_path = ".env\n\u{1b}[31m";
        write_detectable_env(&tmp.path().join(exotic_path));
        git_for_hook_fixture(tmp.path(), &["add", "-f", exotic_path]);

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(result.message.contains(r".env\n\x1b[31m:1"));
        assert!(!result.message.contains(&format!("{exotic_path}:1")));
        assert!(!result.message.contains("pre-existing"));
    }

    #[test]
    fn hook_provenance_budgets_bound_paths_and_worktree_inspection() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a"), "x").unwrap();
        std::fs::write(tmp.path().join("b"), "yy").unwrap();
        let items = ["a", "b"];
        let (values, indeterminate) = bounded_path_values(
            tmp.path(),
            &items,
            |path| (*path).to_string(),
            |_path, content| Some(content.len()),
            2,
            2,
        );
        assert_eq!(values, vec![Some(1), None]);
        assert!(indeterminate.contains("b"));

        let changes = vec![
            ("a".to_string(), ()),
            ("b".to_string(), ()),
            ("c".to_string(), ()),
        ];
        let (bounded, overflow) = split_bounded_paths(changes, 2);
        assert_eq!(bounded.len(), 2);
        assert_eq!(
            overflow,
            std::collections::BTreeSet::from(["c".to_string()])
        );
        assert!(GIT_PROVENANCE_DIFF_ARGS.contains(&"--no-textconv"));
    }

    #[test]
    fn hook_secret_check_labels_committed_finding_pre_existing() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        write_detectable_env(&tmp.path().join(".env"));
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);
        git_for_hook_fixture(
            tmp.path(),
            &[
                "-c",
                "user.name=anvil test",
                "-c",
                "user.email=anvil@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        std::fs::write(
            tmp.path().join("clean.rs"),
            "pub const CLEAN: bool = true;\n",
        )
        .unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "clean.rs"]);

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(!result.passed, "committed secret must still block the gate");
        assert!(
            result
                .message
                .lines()
                .any(|line| line.contains(".env:1") && line.contains("pre-existing")),
            "committed finding should be qualified in hook output: {}",
            result.message
        );
    }

    #[test]
    fn hook_secret_check_labels_unstaged_finding_pre_existing() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        std::fs::write(tmp.path().join(".env"), "SAFE_VALUE=local\n").unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);
        git_for_hook_fixture(
            tmp.path(),
            &[
                "-c",
                "user.name=anvil test",
                "-c",
                "user.email=anvil@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        write_detectable_env(&tmp.path().join(".env"));

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(!result.passed, "unstaged secret must still block the gate");
        assert!(
            result
                .message
                .lines()
                .any(|line| line.contains(".env:1") && line.contains("pre-existing")),
            "unstaged finding should be qualified in hook output: {}",
            result.message
        );
    }

    #[test]
    fn hook_secret_check_labels_committed_finding_with_unrelated_staged_edit() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let env_path = tmp.path().join(".env");
        write_detectable_env(&env_path);
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);
        git_for_hook_fixture(
            tmp.path(),
            &[
                "-c",
                "user.name=anvil test",
                "-c",
                "user.email=anvil@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        let value = "abcd".repeat(10);
        std::fs::write(
            &env_path,
            format!("aws_secret_access_key='{value}'\nSAFE_VALUE=staged\n"),
        )
        .unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(!result.passed);
        assert!(
            result
                .message
                .lines()
                .any(|line| line.contains(".env:1") && line.contains("pre-existing")),
            "a staged path must not make committed debt primary: {}",
            result.message
        );
    }

    #[test]
    fn hook_secret_check_labels_unstaged_finding_with_clean_staged_edit() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let env_path = tmp.path().join(".env");
        std::fs::write(&env_path, "SAFE_VALUE=committed\n").unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);
        git_for_hook_fixture(
            tmp.path(),
            &[
                "-c",
                "user.name=anvil test",
                "-c",
                "user.email=anvil@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        std::fs::write(&env_path, "SAFE_VALUE=staged\n").unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);
        let value = "abcd".repeat(10);
        std::fs::write(
            &env_path,
            format!("SAFE_VALUE=staged\naws_secret_access_key='{value}'\n"),
        )
        .unwrap();

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(!result.passed);
        assert!(
            result
                .message
                .lines()
                .any(|line| line.contains(".env:2") && line.contains("pre-existing")),
            "unstaged debt must not become primary because its path is staged: {}",
            result.message
        );
    }

    #[test]
    fn hook_secret_check_keeps_redaction_collision_staged_change_primary() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let env_path = tmp.path().join(".env");
        let committed = format!("aws_secret_access_key='abcd{}wxyz'\n", "1".repeat(32));
        std::fs::write(&env_path, committed).unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);
        git_for_hook_fixture(
            tmp.path(),
            &[
                "-c",
                "user.name=anvil test",
                "-c",
                "user.email=anvil@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        let staged = format!("aws_secret_access_key='abcd{}wxyz'\n", "2".repeat(32));
        std::fs::write(&env_path, staged).unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        let finding_line = result
            .message
            .lines()
            .find(|line| line.contains(".env:1"))
            .expect("changed staged secret should be rendered");
        assert!(
            !finding_line.contains("pre-existing"),
            "a raw staged change hidden by redaction must remain primary: {finding_line}"
        );
    }

    #[test]
    fn hook_secret_check_maps_identical_staged_and_unstaged_duplicate_occurrences() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let env_path = tmp.path().join(".env");
        std::fs::write(&env_path, "ANCHOR=before\nANCHOR=after\n").unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);
        git_for_hook_fixture(
            tmp.path(),
            &[
                "-c",
                "user.name=anvil test",
                "-c",
                "user.email=anvil@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        let value = "abcd".repeat(10);
        let staged = format!(
            "ANCHOR=before\nSTAGED_CONTEXT=keep\naws_secret_access_key='{value}'\nANCHOR=after\n"
        );
        std::fs::write(&env_path, &staged).unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);
        let working = format!(
            "ANCHOR=before\naws_secret_access_key='{value}'\nSTAGED_CONTEXT=keep\naws_secret_access_key='{value}'\nANCHOR=after\n"
        );
        std::fs::write(&env_path, working).unwrap();

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        let unstaged_duplicate = result
            .message
            .lines()
            .find(|line| line.contains(".env:2"))
            .expect("unstaged duplicate should be rendered");
        let staged_occurrence = result
            .message
            .lines()
            .find(|line| line.contains(".env:4"))
            .expect("staged occurrence should be rendered");
        assert!(
            unstaged_duplicate.contains("pre-existing"),
            "unstaged duplicate must not consume staged attribution: {unstaged_duplicate}"
        );
        assert!(
            !staged_occurrence.contains("pre-existing"),
            "mapped staged occurrence must remain primary: {staged_occurrence}"
        );
    }

    #[test]
    fn hook_secret_check_labels_unchanged_staged_rename_pre_existing() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        write_detectable_env(&tmp.path().join(".env.old"));
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env.old"]);
        git_for_hook_fixture(
            tmp.path(),
            &[
                "-c",
                "user.name=anvil test",
                "-c",
                "user.email=anvil@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        git_for_hook_fixture(tmp.path(), &["mv", ".env.old", ".env.new"]);

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(!result.passed);
        assert!(
            result
                .message
                .lines()
                .any(|line| line.contains(".env.new:1") && line.contains("pre-existing")),
            "an unchanged staged rename must retain HEAD provenance: {}",
            result.message
        );
    }

    #[test]
    fn hook_secret_check_marks_rename_into_scannable_scope_primary() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        write_detectable_env(&tmp.path().join("safe.txt"));
        git_for_hook_fixture(tmp.path(), &["add", "safe.txt"]);
        git_for_hook_fixture(
            tmp.path(),
            &[
                "-c",
                "user.name=anvil test",
                "-c",
                "user.email=anvil@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        git_for_hook_fixture(tmp.path(), &["mv", "safe.txt", ".env"]);

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(!result.passed);
        let renamed_finding = result
            .message
            .lines()
            .find(|line| line.contains(".env:1"))
            .expect("renamed finding should be rendered");
        assert!(
            !renamed_finding.contains("pre-existing"),
            "renaming a secret into scan scope introduces gate debt: {renamed_finding}"
        );
    }

    #[test]
    fn hook_secret_check_isolates_non_utf8_staged_blob() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        std::fs::write(tmp.path().join(".env.invalid"), [0xff]).unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env.invalid"]);
        let secret_path = tmp.path().join(".env.secret");
        write_detectable_env(&secret_path);
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env.secret"]);
        std::fs::write(&secret_path, "SAFE_VALUE=working-tree\n").unwrap();

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(!result.passed, "a readable staged secret must still block");
        assert!(
            result
                .message
                .lines()
                .any(|line| line.contains(".env.secret:1")),
            "one invalid blob must not discard other staged provenance: {}",
            result.message
        );
    }

    #[test]
    fn hook_secret_check_fails_closed_for_uninspectable_staged_blob() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let env_path = tmp.path().join(".env.invalid");
        std::fs::write(&env_path, [0xff]).unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env.invalid"]);
        std::fs::write(&env_path, "SAFE_VALUE=working-tree\n").unwrap();

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(
            !result.passed,
            "indeterminate staged content must fail closed"
        );
        assert!(
            result
                .message
                .contains(".env.invalid [staged content unavailable]"),
            "the affected staged path must be identified: {}",
            result.message
        );
    }

    #[test]
    fn hook_secret_check_leaves_indeterminate_worktree_finding_unqualified() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let env_path = tmp.path().join(".env.invalid");
        std::fs::write(&env_path, [0xff]).unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env.invalid"]);
        write_detectable_env(&env_path);

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        let worktree_finding = result
            .message
            .lines()
            .find(|line| line.contains(".env.invalid:1"))
            .expect("worktree finding should be rendered");
        assert!(
            !worktree_finding.contains("pre-existing"),
            "indeterminate provenance must not downgrade the affected finding: {worktree_finding}"
        );
    }

    #[test]
    fn hook_secret_check_batches_git_subprocesses_for_many_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        for index in 0..32 {
            std::fs::write(
                tmp.path().join(format!(".env.{index}")),
                format!("SAFE_VALUE={index}\n"),
            )
            .unwrap();
        }
        git_for_hook_fixture(tmp.path(), &["add", "-f", "."]);
        let config = crate::util::secret_check_config(tmp.path());

        let provenance = staged_secret_provenance(tmp.path(), &[], &config)
            .expect("staged provenance should be available");

        assert!(
            provenance.git_subprocess_count <= 5,
            "Git subprocess count must be constant, got {}",
            provenance.git_subprocess_count
        );
    }

    #[test]
    fn hook_secret_check_excludes_oversized_worktree_from_diff_snapshot() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let env_path = tmp.path().join(".env.large-worktree");
        write_detectable_env(&env_path);
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env.large-worktree"]);
        std::fs::write(
            &env_path,
            "x".repeat(usize::try_from(MAX_STAGED_BLOB_SIZE).expect("test size must fit in usize")),
        )
        .unwrap();

        let snapshot =
            staged_provenance_snapshot(tmp.path()).expect("staged snapshot should be available");

        assert!(
            !snapshot
                .index_to_worktree
                .is_some_and(|hunks| hunks.contains_key(".env.large-worktree")),
            "an oversized worktree file must not be captured by the batched diff"
        );
    }

    #[test]
    fn hook_secret_check_uses_git_230_batch_protocol() {
        assert_eq!(GIT_BATCH_CHECK_ARGS, ["cat-file", "--batch-check"]);
        assert_eq!(GIT_BATCH_CONTENT_ARGS, ["cat-file", "--batch"]);
    }

    #[test]
    fn hook_secret_check_isolates_newline_path_from_normal_provenance() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let exotic_path = ".env\nexotic";
        write_detectable_env(&tmp.path().join(exotic_path));
        git_for_hook_fixture(tmp.path(), &["add", "-f", exotic_path]);
        let normal_path = tmp.path().join(".env.normal");
        write_detectable_env(&normal_path);
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env.normal"]);

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(result.message.contains("[staged content unavailable]"));
        let normal_finding = result
            .message
            .lines()
            .find(|line| line.contains(".env.normal:1"))
            .expect("normal finding should be rendered");
        assert!(
            !normal_finding.contains("pre-existing"),
            "an exotic path must not compromise normal attribution: {normal_finding}"
        );
    }

    #[test]
    fn hook_secret_check_indexes_dense_finding_lines_once() {
        let value = "abcd".repeat(10);
        let finding_line = format!("aws_secret_access_key='{value}'\n");
        let content = finding_line.repeat(512);
        let config = anvil_checks::secret::SecretCheckConfig::default();
        let findings = anvil_checks::secret::scan_content(&content, ".env", &config);

        let keyed = findings
            .iter()
            .filter_map(|finding| {
                content_line(&content, finding.line)
                    .map(|line| raw_secret_finding_key(finding, line))
            })
            .count();

        assert_eq!(keyed, findings.len());
    }

    #[test]
    fn hook_secret_check_keeps_smaller_worktree_finding_unqualified_for_oversized_index() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let env_path = tmp.path().join(".env.large");
        std::fs::write(
            &env_path,
            "x".repeat(usize::try_from(anvil_checks::secret::MAX_FILE_SIZE).unwrap()),
        )
        .unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env.large"]);
        write_detectable_env(&env_path);

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        let worktree_finding = result
            .message
            .lines()
            .find(|line| line.contains(".env.large:1"))
            .expect("smaller worktree finding should be rendered");
        assert!(
            !worktree_finding.contains("pre-existing"),
            "oversized staged provenance must not downgrade the worktree finding: {worktree_finding}"
        );
        assert!(
            result
                .message
                .contains(".env.large [staged content unavailable]"),
            "oversized staged path must be diagnosed: {}",
            result.message
        );
    }

    #[test]
    fn hook_secret_check_skips_oversized_staged_blob_without_discarding_others() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let large_path = tmp.path().join(".env.a-large");
        let value = "abcd".repeat(10);
        let prefix = format!("aws_secret_access_key='{value}'\n");
        let oversized = format!(
            "{prefix}{}",
            "x".repeat(
                usize::try_from(anvil_checks::secret::MAX_FILE_SIZE).unwrap() - prefix.len()
            )
        );
        std::fs::write(&large_path, oversized).unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env.a-large"]);
        std::fs::write(&large_path, "SAFE_VALUE=working-tree\n").unwrap();
        let secret_path = tmp.path().join(".env.z-secret");
        write_detectable_env(&secret_path);
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env.z-secret"]);
        std::fs::write(&secret_path, "SAFE_VALUE=working-tree\n").unwrap();

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(
            !result.passed,
            "the bounded readable staged secret must block"
        );
        assert!(
            !result.message.contains(".env.a-large:"),
            "the existing size boundary must apply to index blobs: {}",
            result.message
        );
        assert!(
            result
                .message
                .lines()
                .any(|line| line.contains(".env.z-secret:1")),
            "an oversized blob must not discard other staged provenance: {}",
            result.message
        );
    }

    #[cfg(unix)]
    #[test]
    fn hook_secret_check_includes_symlink_to_regular_type_change() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        std::fs::write(tmp.path().join("safe.txt"), "SAFE_VALUE=committed\n").unwrap();
        std::os::unix::fs::symlink("safe.txt", tmp.path().join(".env.link")).unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env.link", "safe.txt"]);
        git_for_hook_fixture(
            tmp.path(),
            &[
                "-c",
                "user.name=anvil test",
                "-c",
                "user.email=anvil@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        std::fs::remove_file(tmp.path().join(".env.link")).unwrap();
        write_detectable_env(&tmp.path().join(".env.link"));
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env.link"]);

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(!result.passed);
        let finding = result
            .message
            .lines()
            .find(|line| line.contains(".env.link:1"))
            .expect("type-changed finding should be rendered");
        assert!(
            !finding.contains("pre-existing"),
            "a type-changed staged secret must be primary: {finding}"
        );
    }

    #[test]
    fn hook_secret_check_treats_pathspec_magic_filename_literally() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let magic_path = ":(literal).env";
        let env_path = tmp.path().join(magic_path);
        write_detectable_env(&env_path);
        git_for_hook_fixture(
            tmp.path(),
            &["--literal-pathspecs", "add", "-f", magic_path],
        );
        let staged = std::fs::read_to_string(&env_path).unwrap();
        std::fs::write(&env_path, format!("SAFE_VALUE=working-tree\n{staged}")).unwrap();

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        let shifted_finding = result
            .message
            .lines()
            .find(|line| line.contains(":(literal).env:2"))
            .expect("shifted pathspec-magic finding should be rendered");
        assert!(
            !shifted_finding.contains("pre-existing"),
            "literal pathspec attribution must follow the worktree line: {shifted_finding}"
        );
    }

    #[test]
    fn hook_secret_check_does_not_parse_added_content_as_file_header() {
        use std::fmt::Write as _;

        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let env_path = tmp.path().join(".env");
        let mut committed = String::new();
        for line in 1..=12 {
            writeln!(committed, "LINE_{line}=safe").unwrap();
        }
        std::fs::write(&env_path, &committed).unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);
        git_for_hook_fixture(
            tmp.path(),
            &[
                "-c",
                "user.name=anvil test",
                "-c",
                "user.email=anvil@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        let value = "abcd".repeat(10);
        let staged = committed.replace(
            "LINE_9=safe\n",
            &format!("aws_secret_access_key='{value}'\nLINE_9=safe\n"),
        );
        std::fs::write(&env_path, &staged).unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);
        let working = staged
            .replace("LINE_2=safe\n", "++ counter\nLINE_2=safe\n")
            .replace(
                &format!("aws_secret_access_key='{value}'\n"),
                &format!("WORKTREE_SHIFT=safe\naws_secret_access_key='{value}'\n"),
            );
        std::fs::write(&env_path, working).unwrap();

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        let shifted_finding = result
            .message
            .lines()
            .find(|line| line.contains(".env:11"))
            .expect("twice-shifted finding should be rendered");
        assert!(
            !shifted_finding.contains("pre-existing"),
            "added content must not poison later hunk attribution: {shifted_finding}"
        );
    }

    #[test]
    fn hook_secret_check_matches_identical_head_occurrence_by_line() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        let env_path = tmp.path().join(".env");
        let value = "abcd".repeat(10);
        let secret = format!("aws_secret_access_key='{value}'");
        std::fs::write(
            &env_path,
            format!("ANCHOR=before\n{secret}\nANCHOR=after\n"),
        )
        .unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);
        git_for_hook_fixture(
            tmp.path(),
            &[
                "-c",
                "user.name=anvil test",
                "-c",
                "user.email=anvil@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        std::fs::write(
            &env_path,
            format!("{secret}\nANCHOR=before\n{secret}\nANCHOR=after\n"),
        )
        .unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        let inserted = result
            .message
            .lines()
            .find(|line| line.contains(".env:1"))
            .expect("inserted occurrence should be rendered");
        let surviving = result
            .message
            .lines()
            .find(|line| line.contains(".env:3"))
            .expect("surviving occurrence should be rendered");
        assert!(
            !inserted.contains("pre-existing"),
            "the inserted identical occurrence must remain primary: {inserted}"
        );
        assert!(
            surviving.contains("pre-existing"),
            "the HEAD occurrence must retain provenance after a line shift: {surviving}"
        );
    }

    #[test]
    fn hook_secret_check_blocks_index_only_staged_secret() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        std::fs::write(tmp.path().join("README.md"), "fixture\n").unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "README.md"]);
        git_for_hook_fixture(
            tmp.path(),
            &[
                "-c",
                "user.name=anvil test",
                "-c",
                "user.email=anvil@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        let env_path = tmp.path().join(".env");
        write_detectable_env(&env_path);
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env"]);
        std::fs::write(&env_path, "SAFE_VALUE=working-tree\n").unwrap();

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(!result.passed, "a staged index secret must block the hook");
        let staged_line = result
            .message
            .lines()
            .find(|line| line.contains(".env:1"))
            .expect("index-only staged finding should be rendered");
        assert!(
            !staged_line.contains("pre-existing"),
            "index-only staged debt must remain primary: {staged_line}"
        );
    }

    #[test]
    fn hook_secret_check_preserves_unqualified_fallback_without_git_index() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Stop Git discovery at this fixture rather than inheriting the
        // enclosing worktree (TMPDIR is task-local and sits inside it).
        std::fs::write(tmp.path().join(".git"), "gitdir: missing-git-dir\n").unwrap();
        write_detectable_env(&tmp.path().join(".env"));

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(!result.passed);
        assert!(result.message.lines().any(|line| line.contains(".env:1")));
        assert!(
            !result.message.contains("pre-existing"),
            "without Git index truth the established output must be retained: {}",
            result.message
        );
    }

    #[test]
    fn hook_secret_check_keeps_staged_finding_primary() {
        let tmp = tempfile::TempDir::new().unwrap();
        git_for_hook_fixture(tmp.path(), &["init", "--quiet"]);
        std::fs::write(tmp.path().join("README.md"), "fixture\n").unwrap();
        git_for_hook_fixture(tmp.path(), &["add", "README.md"]);
        git_for_hook_fixture(
            tmp.path(),
            &[
                "-c",
                "user.name=anvil test",
                "-c",
                "user.email=anvil@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "fixture",
            ],
        );
        write_detectable_env(&tmp.path().join(".env.local"));
        git_for_hook_fixture(tmp.path(), &["add", "-f", ".env.local"]);

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );

        assert!(!result.passed, "staged secret must block the gate");
        let staged_line = result
            .message
            .lines()
            .find(|line| line.contains(".env.local:1"))
            .expect("staged finding should be rendered");
        assert!(
            !staged_line.contains("pre-existing"),
            "staged finding must remain primary: {staged_line}"
        );
    }

    #[test]
    fn non_hook_secret_check_keeps_existing_location_copy() {
        let tmp = tempfile::TempDir::new().unwrap();
        write_detectable_env(&tmp.path().join(".env"));

        let result = run_check_secret_with_hook_mode(
            "secret",
            tmp.path(),
            &std::collections::HashSet::new(),
            false,
        );

        assert!(!result.passed);
        assert!(result.message.lines().any(|line| line.contains(".env:1")));
        assert!(
            !result.message.contains("pre-existing"),
            "non-hook output must retain established location copy: {}",
            result.message
        );
    }

    #[test]
    fn sql_migrations_check_warns_but_does_not_block() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().join("db/migrations");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("001.sql"), "DROP TABLE legacy_events;\n").unwrap();

        let result = run_check_sql_migrations(
            "sql-migrations",
            tmp.path(),
            &["db/migrations/001.sql".to_string()],
        );
        // Warn-only: surfaced in the message, never blocks the gate.
        assert!(result.passed, "SURFSQL is warn-only and must not block");
        assert!(
            result.message.contains("DROP TABLE"),
            "finding should be surfaced: {}",
            result.message
        );
    }

    #[test]
    fn sql_migrations_check_clean_on_guarded_migration() {
        let tmp = tempfile::TempDir::new().unwrap();
        // A versioned migration name, not `schema.sql`, so this exercises the
        // guarded-migration path rather than the SURFSQL-008 dump skip.
        std::fs::write(
            tmp.path().join("001_init.sql"),
            "CREATE TABLE IF NOT EXISTS t (id int);\n",
        )
        .unwrap();
        let result =
            run_check_sql_migrations("sql-migrations", tmp.path(), &["001_init.sql".to_string()]);
        assert!(result.passed);
        assert!(result.message.contains("No destructive or schema-hygiene"));
    }

    #[test]
    fn sql_migrations_check_surfaces_hygiene_warning() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("001.sql"), "CREATE TABLE users (id int);\n").unwrap();
        let result =
            run_check_sql_migrations("sql-migrations", tmp.path(), &["001.sql".to_string()]);
        assert!(result.passed, "warn-only, never blocks");
        assert!(
            result
                .message
                .contains("CREATE TABLE without IF NOT EXISTS"),
            "hygiene finding surfaced: {}",
            result.message
        );
    }

    #[test]
    fn sql_migrations_check_baselines_existing_and_warns_only_on_new() {
        use anvil_checks::surface::sql::run_surfsql_check;
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("0001.sql"), "DROP TABLE users;\n").unwrap();
        let files = ["0001.sql".to_string()];

        // No drift snapshot → warn-on-all, with a hint to baseline.
        let r1 = run_check_sql_migrations("sql-migrations", tmp.path(), &files);
        assert!(r1.passed);
        assert!(
            r1.message.contains("new SQL migration issue"),
            "{}",
            r1.message
        );
        assert!(r1.message.contains("no drift baseline"), "{}", r1.message);

        // Baseline every existing finding by writing a snapshot carrying their
        // fingerprints (shared derivation via commands::drift).
        let scan = run_surfsql_check(&[(
            std::path::PathBuf::from("0001.sql"),
            "DROP TABLE users;\n".to_string(),
        )]);
        let mut fps: Vec<String> = Vec::new();
        for f in scan.destructive.iter().filter(|f| !f.suppressed) {
            fps.push(crate::commands::drift::destructive_finding_id(f).1);
        }
        for f in scan.hygiene.iter().filter(|f| !f.suppressed) {
            fps.push(crate::commands::drift::hygiene_finding_id(f).1);
        }
        let entries: String = fps
            .iter()
            .map(|fp| {
                format!(
                    r#"{{"fingerprint":"{fp}","rule_id":"surfsql","file":"0001.sql","line":1}}"#
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let snap_dir = tmp.path().join(".anvil").join("snapshots");
        std::fs::create_dir_all(&snap_dir).unwrap();
        let snapshot = format!(
            r#"{{"schema_version":"1.1.0","created_at":"2026-01-01T00:00:00+00:00","metrics":{{"boundary_violations":0,"antipattern_count":0,"suppression_count":0,"expired_suppressions":0,"files_analysed":1}},"violations":[],"antipatterns":[],"suppressions":[],"sql_findings":[{entries}]}}"#
        );
        std::fs::write(snap_dir.join("snapshot-base.json"), snapshot).unwrap();

        // Baselined finding is now silent.
        let r2 = run_check_sql_migrations("sql-migrations", tmp.path(), &files);
        assert!(
            r2.message
                .contains("No new SQL migration issues vs drift baseline"),
            "{}",
            r2.message
        );

        // A genuinely new destructive op warns; the baselined one stays silent.
        std::fs::write(tmp.path().join("0002.sql"), "DROP TABLE accounts;\n").unwrap();
        let r3 = run_check_sql_migrations(
            "sql-migrations",
            tmp.path(),
            &["0001.sql".to_string(), "0002.sql".to_string()],
        );
        assert!(
            r3.message.contains("1 new SQL migration issue"),
            "{}",
            r3.message
        );
        // Statements are surfaced normalised to upper-case.
        assert!(
            r3.message.to_uppercase().contains("ACCOUNTS"),
            "{}",
            r3.message
        );
        assert!(r3.message.contains("1 baselined"), "{}", r3.message);
    }

    #[test]
    fn sql_migrations_check_with_sql_empty_snapshot_does_not_claim_missing_baseline() {
        // A snapshot exists but carries no SQL findings (repo was SQL-clean at
        // snapshot time). A finding introduced afterwards must warn, but the
        // message must NOT tell the user to run a snapshot they already have.
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("0001.sql"), "DROP TABLE users;\n").unwrap();
        let snap_dir = tmp.path().join(".anvil").join("snapshots");
        std::fs::create_dir_all(&snap_dir).unwrap();
        // Snapshot with an empty (omitted) sql_findings set.
        let snapshot = r#"{"schema_version":"1.1.0","created_at":"2026-01-01T00:00:00+00:00","metrics":{"boundary_violations":0,"antipattern_count":0,"suppression_count":0,"expired_suppressions":0,"files_analysed":0},"violations":[],"antipatterns":[],"suppressions":[]}"#;
        std::fs::write(snap_dir.join("snapshot-empty.json"), snapshot).unwrap();

        let r = run_check_sql_migrations("sql-migrations", tmp.path(), &["0001.sql".to_string()]);
        assert!(
            r.message.contains("new SQL migration issue"),
            "{}",
            r.message
        );
        assert!(
            !r.message.contains("no drift baseline"),
            "a snapshot exists; must not claim none: {}",
            r.message
        );
        assert!(r.message.contains("0 baselined"), "{}", r.message);
    }

    #[test]
    fn github_actions_check_warns_but_does_not_block() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().join(".github/workflows");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("ci.yml"),
            "on: pull_request_target\njobs:\n  b:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@main\n",
        )
        .unwrap();
        let result = run_check_github_actions(
            "github-actions",
            tmp.path(),
            &[".github/workflows/ci.yml".to_string()],
        );
        assert!(result.passed, "warn-only, never blocks");
        assert!(result.message.contains("pull_request_target"));
        assert!(result.message.contains("unpinned action ref"));
    }

    #[test]
    fn github_actions_check_clean_on_safe_workflow() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().join(".github/workflows");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("ci.yml"),
            "on:\n  push:\njobs:\n  b:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n",
        )
        .unwrap();
        let result = run_check_github_actions(
            "github-actions",
            tmp.path(),
            &[".github/workflows/ci.yml".to_string()],
        );
        assert!(result.passed);
        assert!(
            result
                .message
                .contains("No GitHub Actions supply-chain risks")
        );
    }

    #[test]
    fn dockerfile_check_warns_but_does_not_block() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("Dockerfile"),
            "FROM node:latest\nRUN curl -fsSL https://x | sh\n",
        )
        .unwrap();
        let result = run_check_dockerfile("dockerfile", tmp.path(), &["Dockerfile".to_string()]);
        assert!(result.passed, "warn-only, never blocks");
        assert!(result.message.contains(":latest base image"));
        assert!(result.message.contains("pipe-to-shell"));
    }

    #[test]
    fn dockerfile_check_clean_on_safe_dockerfile() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("Dockerfile"),
            "FROM node:20-alpine\nWORKDIR /app\nCOPY . .\nRUN npm ci\nUSER node\n",
        )
        .unwrap();
        let result = run_check_dockerfile("dockerfile", tmp.path(), &["Dockerfile".to_string()]);
        assert!(result.passed);
        assert!(result.message.contains("No Dockerfile build-hygiene risks"));
    }

    #[test]
    fn shell_check_warns_but_does_not_block() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("danger.sh"), "#!/bin/sh\nrm -rf /\n").unwrap();
        let result = run_check_shell("shell-scripts", tmp.path(), &["danger.sh".to_string()]);
        assert!(result.passed, "warn-only, never blocks");
        assert!(
            result.message.contains("dangerous shell-script command"),
            "rm -rf / surfaced: {}",
            result.message
        );
    }

    #[test]
    fn shell_check_clean_on_safe_script() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("ok.sh"),
            "#!/bin/sh\nset -euo pipefail\necho building\nnpm ci\n",
        )
        .unwrap();
        let result = run_check_shell("shell-scripts", tmp.path(), &["ok.sh".to_string()]);
        assert!(result.passed);
        assert!(
            result
                .message
                .contains("No dangerous shell-script commands")
        );
    }

    #[test]
    fn args_parses_empty() {
        let w = Wrapper::try_parse_from(["test"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_with_plan() {
        let w = Wrapper::try_parse_from(["test", "plan.aps.md"]).unwrap();
        assert_eq!(w.inner.plan.as_deref(), Some("plan.aps.md"));
    }

    #[test]
    fn args_parses_profile() {
        let w = Wrapper::try_parse_from(["test", "--profile", "dev"]).unwrap();
        let _ = format!("{:?}", w.inner);
    }

    #[test]
    fn args_parses_list_profiles() {
        let w = Wrapper::try_parse_from(["test", "--list-profiles"]).unwrap();
        assert!(w.inner.list_profiles);
    }

    // Regression guard: ensures --no-cache is not re-introduced (was dead code, removed in TCOV-006).
    #[test]
    fn no_cache_flag_removed() {
        let result = Wrapper::try_parse_from(["test", "--no-cache"]);
        assert!(result.is_err(), "--no-cache should not be accepted");
    }

    #[test]
    fn resolve_profile_dev_skips_coverage_and_dependency() {
        let skips = resolve_profile_skips(Some("dev")).unwrap();
        assert!(skips.contains("coverage"));
        assert!(skips.contains("dependency"));
    }

    #[test]
    fn resolve_profile_unknown_errors() {
        assert!(resolve_profile_skips(Some("bogus")).is_err());
    }

    #[test]
    fn resolve_profile_none_returns_empty() {
        let skips = resolve_profile_skips(None).unwrap();
        assert!(skips.is_empty());
    }

    // ── AI guardrail profile (AIGUARD-001) ────────────────────────────

    #[test]
    fn ai_guardrail_profile_is_registered() {
        // The "ai" profile must be discoverable via --list-profiles so users
        // (and AI tools reading the help output) can find it.
        let names: Vec<&str> = PROFILES.iter().map(|(n, _, _)| *n).collect();
        assert!(
            names.contains(&AiGuardrailProfile::NAME),
            "ai profile missing from PROFILES table: {names:?}"
        );
    }

    #[test]
    fn ai_guardrail_profile_bundles_expected_rule_set() {
        // The profile must declare the structural-governance rule families
        // documented in plans/modules/ai-guardrail-profile.aps.md.
        let checks = ai_guardrail_profile_checks();
        assert!(checks.contains(&"secret-detection"));
        assert!(checks.contains(&"import-boundaries"));
        assert!(checks.contains(&"antipattern-scan"));
        assert!(checks.contains(&"policy"));
        assert!(checks.contains(&"command-safety"));
    }

    #[test]
    fn ai_guardrail_profile_excludes_toolchain_checks() {
        // Lint/test/coverage/dependency are project-toolchain concerns
        // outside the AI guardrail's structural focus and would push the
        // profile past its <5s budget. Guard against accidental inclusion.
        let checks = ai_guardrail_profile_checks();
        for excluded in ["lint", "test", "coverage", "dependency"] {
            assert!(
                !checks.contains(&excluded),
                "ai profile must not include {excluded}: got {checks:?}"
            );
        }
    }

    #[test]
    fn ai_guardrail_profile_defaults_are_strict() {
        // Per AIGUARD acceptance criteria: missing/invalid config blocks,
        // and JSON output is the documented default for AI consumers.
        let profile = AiGuardrailProfile::DEFAULT;
        assert!(profile.strict_config);
        assert!(profile.json_output_default);
    }

    #[test]
    fn gate_output_mode_honours_ai_default_and_explicit_format() {
        use crate::output::{Format, OutputMode};
        let global = GlobalArgs::default();

        // `--format auto` and an absent `--format` both keep the AI-profile
        // JSON default — auto must NOT be treated as an explicit override.
        assert_eq!(
            resolve_gate_output_mode(Some(Format::Auto), true, true, &global, true),
            OutputMode::Json,
        );
        assert_eq!(
            resolve_gate_output_mode(None, true, true, &global, true),
            OutputMode::Json,
        );

        // An explicit, non-auto `--format` overrides the AI JSON default.
        assert_eq!(
            resolve_gate_output_mode(Some(Format::Plain), true, true, &global, true),
            OutputMode::Plain,
        );
        assert_eq!(
            resolve_gate_output_mode(Some(Format::Sarif), true, true, &global, true),
            OutputMode::Sarif,
        );

        // `--no-tui` opts out of the AI JSON default to plain text.
        let no_tui = GlobalArgs {
            no_tui: true,
            ..GlobalArgs::default()
        };
        assert_eq!(
            resolve_gate_output_mode(None, true, true, &no_tui, true),
            OutputMode::Plain,
        );

        // Without the AI profile, auto/absent falls through to the legacy
        // resolver (TTY → Tui).
        assert_eq!(
            resolve_gate_output_mode(None, false, true, &global, true),
            OutputMode::Tui,
        );
    }

    #[test]
    fn ai_guardrail_profile_skips_match_inverse_of_rule_set() {
        // The PROFILES skip list and AI_GUARDRAIL_CHECKS allow list must
        // stay in sync — every gate-supported check is either in the
        // profile's rule set or in its skip list (modulo command-safety,
        // which is wired in by AIGUARD-003).
        let skips = resolve_profile_skips(Some(AiGuardrailProfile::NAME)).unwrap();
        assert!(skips.contains("lint"));
        assert!(skips.contains("test"));
        assert!(skips.contains("coverage"));
        assert!(skips.contains("dependency"));

        // Checks that should run under the profile must NOT appear in skips.
        assert!(!skips.contains("antipattern-scan"));
        assert!(!skips.contains("secret"));
        assert!(!skips.contains("architecture"));
        assert!(!skips.contains("policy"));
    }

    #[test]
    fn ai_guardrail_profile_check_names_are_canonical() {
        // The profile exposes canonical names so it composes with the
        // public check catalog and the `--profile ai` flag wired in
        // AIGUARD-003. Each entry must round-trip through the catalog
        // — command-safety was registered in AIGUARD-003 so the earlier
        // skip is no longer required.
        for name in ai_guardrail_profile_checks() {
            assert!(
                canonical_check_name(name).is_some(),
                "ai profile references unknown check: {name}"
            );
        }
    }

    #[test]
    fn ai_guardrail_only_set_resolves_to_internal_names() {
        // Allow-list path used by `run_checks` when --profile ai is
        // selected — every entry must resolve to a gate-runner internal
        // name, otherwise the dispatcher loop in `run_checks` silently
        // drops it.
        let resolved = ai_guardrail_only_set().expect("should resolve");
        for internal in &resolved {
            assert!(GATE_INTERNAL_CHECKS.contains(internal));
        }
        // And every canonical entry in the rule set has a corresponding
        // internal name in the resolved set.
        assert_eq!(resolved.len(), ai_guardrail_profile_checks().len());
    }

    #[test]
    fn resolve_profile_skip_set_canonicalises_through_internal_names() {
        // The dev profile's skip list (`coverage`, `dependency`) must
        // resolve through the catalog so it stays in lock-step with
        // user-supplied --skip-checks vocabulary.
        let skips = resolve_profile_skip_set(Some("dev")).unwrap();
        assert!(skips.contains("coverage"));
        assert!(skips.contains("dependency"));
        // Round-trip: every entry must be a real gate-internal name.
        for entry in &skips {
            assert!(GATE_INTERNAL_CHECKS.contains(entry));
        }
    }

    #[test]
    fn check_result_to_diagnostic_emits_canonical_envelope_fields() {
        let check = CheckResult {
            name: "secret-detection".to_string(),
            passed: false,
            score: 0.0,
            message: "Potential secret on src/leak.ts:12".to_string(),
            requires_config: false,
        };
        let diag = check_result_to_diagnostic(&check);
        assert_eq!(diag.schema_version, "anvil.diagnostic.v1");
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.category, Category::Secret);
        assert_eq!(diag.mode, Mode::known(KnownMode::Gate));
        assert!(diag.source.rule_id.contains("secret-detection"));
    }

    #[test]
    fn build_ai_gate_result_envelope_pins_schema_and_summary() {
        let checks = vec![
            CheckResult {
                name: "secret-detection".to_string(),
                passed: false,
                score: 0.0,
                message: "leak".to_string(),
                requires_config: false,
            },
            CheckResult {
                name: "policy".to_string(),
                passed: true,
                score: 100.0,
                message: "ok".to_string(),
                requires_config: false,
            },
        ];
        let notifications = notifications_for_gate_result(&checks, false);
        let result = GateResult {
            overall: false,
            score: 50.0,
            checks,
            notifications,
            duration_ms: 17,
        };
        let envelope = build_ai_gate_result_envelope(&result);
        let value = serde_json::to_value(&envelope).unwrap();
        assert_eq!(value["schema"], "anvil.gate-result.v1");
        assert_eq!(value["exit_code"], 2);
        assert_eq!(value["summary"]["total"], 1);
        assert_eq!(value["summary"]["overall_passed"], false);
        assert_eq!(value["diagnostics"][0]["mode"], "gate");
        assert_eq!(value["diagnostics"][0]["category"], "secret");
    }

    #[test]
    fn gate_snapshot_maps_status_rows_and_warnings() {
        let checks = vec![
            CheckResult {
                name: "lint".into(),
                passed: true,
                score: 100.0,
                message: "clean".into(),
                requires_config: false,
            },
            CheckResult {
                name: "secret".into(),
                passed: false,
                score: 0.0,
                message: "leak found".into(),
                requires_config: false,
            },
            CheckResult {
                name: "architecture".into(),
                passed: false,
                score: 0.0,
                message: "no config".into(),
                requires_config: true,
            },
        ];
        let aggregate = aggregate_gate_outcome(&checks);
        let result = GateResult {
            overall: aggregate.overall,
            score: aggregate.score,
            notifications: vec![],
            duration_ms: 4200,
            checks,
        };
        let snap = gate_snapshot_from_result(&result, &aggregate);

        // One available check (secret) failed -> fail; the config-gap is excluded.
        assert_eq!(snap.status, "fail");
        assert!(
            snap.status_label.starts_with("FAILED"),
            "{}",
            snap.status_label
        );
        assert_eq!(snap.checks_run, "2", "config-gap excluded from checks run");
        assert_eq!(snap.duration_seconds, "4.2", "4200ms -> 4.2s");

        assert_eq!(snap.check_rows.len(), 3);
        assert_eq!(snap.check_rows[0], ["lint", "passed", "100", "clean"]);
        assert_eq!(snap.check_rows[1][1], "failed");
        assert_eq!(snap.check_rows[2][1], "config");

        // Warnings: secret (error) + architecture (warn); passing lint excluded.
        assert_eq!(snap.warnings, "2");
        assert_eq!(snap.warning_list.len(), 2);
        assert_eq!(snap.warning_list[0].severity, "error");
        assert!(snap.warning_list[0].message.contains("secret: leak found"));
        assert_eq!(snap.warning_list[1].severity, "warn");

        // The persisted JSON uses the camelCase keys the dashboard `$data` paths
        // bind to.
        let v = serde_json::to_value(&snap).unwrap();
        for key in [
            "status",
            "statusLabel",
            "checksRun",
            "warnings",
            "durationSeconds",
            "checkRows",
            "warningList",
        ] {
            assert!(v.get(key).is_some(), "missing key {key}");
        }
        assert_eq!(v["checkRows"][1][1], "failed");
    }

    #[test]
    fn gate_snapshot_status_is_warn_when_passing_with_config_gaps() {
        let checks = vec![
            CheckResult {
                name: "lint".into(),
                passed: true,
                score: 100.0,
                message: "ok".into(),
                requires_config: false,
            },
            CheckResult {
                name: "architecture".into(),
                passed: false,
                score: 0.0,
                message: "no config".into(),
                requires_config: true,
            },
        ];
        let aggregate = aggregate_gate_outcome(&checks);
        let result = GateResult {
            overall: aggregate.overall,
            score: aggregate.score,
            notifications: vec![],
            duration_ms: 500,
            checks,
        };
        assert!(result.overall, "no available check failed -> overall pass");
        let snap = gate_snapshot_from_result(&result, &aggregate);
        assert_eq!(
            snap.status, "warn",
            "passing-with-config-gaps is warn, not pass"
        );
        assert!(snap.status_label.starts_with("PASSED"));
        assert_eq!(
            snap.duration_seconds, "0.5",
            "sub-second run shows tenths, not 0"
        );
    }

    #[test]
    fn gate_snapshot_creates_a_missing_real_anvil_directory() {
        let workspace = tempfile::tempdir().expect("workspace");

        persist_gate_snapshot_json(workspace.path(), br#"{"status":"pass"}"#)
            .expect("missing .anvil is created safely");

        assert_eq!(
            std::fs::read(workspace.path().join(".anvil/gates.json")).expect("gate snapshot"),
            br#"{"status":"pass"}"#
        );
    }

    #[test]
    fn gate_history_retention_drops_strictly_older_points_then_caps_at_500() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-07-27T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let mut lines = vec![
            br#"{"recorded_at":"2026-04-28T11:59:59Z","score":1,"status":"fail","status_label":"old","warning_count":1}"#.to_vec(),
            br#"{"recorded_at":"2026-04-28T12:00:00Z","score":2,"status":"warn","status_label":"boundary","warning_count":1}"#.to_vec(),
            b"{corrupt-but-visible".to_vec(),
        ];
        for second in 0..501 {
            lines.push(
                format!(
                    "{{\"recorded_at\":\"2026-07-27T11:{:02}:{:02}Z\",\"score\":3,\"status\":\"pass\",\"status_label\":\"ok\",\"warning_count\":0}}",
                    (second / 60) % 60,
                    second % 60
                )
                .into_bytes(),
            );
        }

        let retained = retain_gate_history_lines(lines, now);
        assert_eq!(retained.len(), 500);
        assert!(
            !retained
                .iter()
                .any(|line| line.windows(3).any(|w| w == b"old"))
        );
        assert!(!retained.iter().any(|line| line == b"{corrupt-but-visible"));
    }

    #[test]
    fn gate_history_point_uses_warning_list_length_and_utc_timestamp() {
        let snapshot: GateSnapshot = serde_json::from_value(serde_json::json!({
            "status": "warn",
            "statusLabel": "PASSED — score 90/100",
            "score": 90.0,
            "checksRun": "4",
            "warnings": "wrong",
            "durationSeconds": "0.5",
            "checkRows": [],
            "warningList": [{"severity":"warn", "message":"gap"}]
        }))
        .unwrap();
        let recorded_at = chrono::DateTime::parse_from_rfc3339("2026-07-27T12:34:56Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        let point = gate_history_point(&snapshot, recorded_at);
        assert_eq!(point.recorded_at, "2026-07-27T12:34:56Z");
        assert_eq!(point.warning_count, 1);
        assert_eq!(point.checks_run.as_deref(), Some("4"));
    }

    #[test]
    fn gate_history_append_preserves_existing_corruption_visibly() {
        let workspace = tempfile::tempdir().expect("workspace");
        std::fs::create_dir(workspace.path().join(".anvil")).unwrap();
        std::fs::write(
            workspace.path().join(".anvil/gate-history.ndjson"),
            b"{corrupt-but-visible\n",
        )
        .unwrap();
        let snapshot: GateSnapshot = serde_json::from_value(serde_json::json!({
            "status": "pass", "statusLabel": "PASSED", "score": 100.0,
            "checksRun": "4", "warnings": "0", "durationSeconds": "0.5",
            "checkRows": [], "warningList": []
        }))
        .unwrap();
        let recorded_at = chrono::DateTime::parse_from_rfc3339("2026-07-27T12:34:56Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        append_gate_history(workspace.path(), &snapshot, recorded_at).unwrap();

        let history =
            std::fs::read_to_string(workspace.path().join(".anvil/gate-history.ndjson")).unwrap();
        assert!(history.starts_with("{corrupt-but-visible\n"));
        assert!(history.contains("\"recorded_at\":\"2026-07-27T12:34:56Z\""));
    }

    #[cfg(unix)]
    #[test]
    fn gate_history_refuses_a_symlinked_history_file() {
        use std::os::unix::fs::symlink;

        let workspace = tempfile::tempdir().expect("workspace");
        let outside = tempfile::NamedTempFile::new().expect("outside");
        std::fs::create_dir(workspace.path().join(".anvil")).unwrap();
        symlink(
            outside.path(),
            workspace.path().join(".anvil/gate-history.ndjson"),
        )
        .unwrap();
        let snapshot: GateSnapshot = serde_json::from_value(serde_json::json!({
            "status": "pass", "statusLabel": "PASSED", "score": 100.0,
            "checksRun": "4", "warnings": "0", "durationSeconds": "0.5",
            "checkRows": [], "warningList": []
        }))
        .unwrap();

        let error = append_gate_history(workspace.path(), &snapshot, chrono::Utc::now())
            .expect_err("history link must fail closed");
        assert!(format!("{error:#}").contains("held gate history"));
        assert!(std::fs::read(outside.path()).unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn gate_history_refuses_a_symlinked_transaction_lock() {
        use std::os::unix::fs::symlink;

        let workspace = tempfile::tempdir().expect("workspace");
        let outside = tempfile::NamedTempFile::new().expect("outside");
        std::fs::create_dir(workspace.path().join(".anvil")).unwrap();
        symlink(
            outside.path(),
            workspace.path().join(".anvil/.gate-history.lock"),
        )
        .unwrap();
        let snapshot: GateSnapshot = serde_json::from_value(serde_json::json!({
            "status": "pass", "statusLabel": "PASSED", "score": 100.0,
            "checksRun": "4", "warnings": "0", "durationSeconds": "0.5",
            "checkRows": [], "warningList": []
        }))
        .unwrap();

        let error = append_gate_history(workspace.path(), &snapshot, chrono::Utc::now())
            .expect_err("history transaction lock link must fail closed");
        assert!(format!("{error:#}").contains("locking gate history"));
        assert!(std::fs::read(outside.path()).unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn history_io_failure_never_prevents_the_latest_snapshot_write() {
        use std::os::unix::fs::symlink;

        let workspace = tempfile::tempdir().expect("workspace");
        let outside = tempfile::NamedTempFile::new().expect("outside");
        std::fs::create_dir(workspace.path().join(".anvil")).unwrap();
        symlink(
            outside.path(),
            workspace.path().join(".anvil/gate-history.ndjson"),
        )
        .unwrap();
        let snapshot: GateSnapshot = serde_json::from_value(serde_json::json!({
            "status": "pass", "statusLabel": "PASSED", "score": 100.0,
            "checksRun": "4", "warnings": "0", "durationSeconds": "0.5",
            "checkRows": [], "warningList": []
        }))
        .unwrap();

        persist_gate_snapshot_at_root(workspace.path(), &snapshot);

        let latest = std::fs::read_to_string(workspace.path().join(".anvil/gates.json")).unwrap();
        assert!(latest.contains("\"status\": \"pass\""));
        assert!(std::fs::read(outside.path()).unwrap().is_empty());
    }

    #[test]
    fn concurrent_gate_history_writers_preserve_both_points() {
        use std::sync::{Arc, Barrier};

        let workspace = tempfile::tempdir().expect("workspace");
        let anvil_dir = workspace.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        // Pre-create the lock file so concurrent first-writers cannot race on
        // create-open under openat/CreateFile.
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(anvil_dir.join(GATE_HISTORY_LOCK_FILE))
            .unwrap();
        let root = Arc::new(workspace.path().to_path_buf());
        let barrier = Arc::new(Barrier::new(2));
        let handles = [1, 2].map(|second| {
            let root = Arc::clone(&root);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let snapshot: GateSnapshot = serde_json::from_value(serde_json::json!({
                    "status": "pass", "statusLabel": format!("point-{second}"), "score": 100.0,
                    "checksRun": "4", "warnings": "0", "durationSeconds": "0.5",
                    "checkRows": [], "warningList": []
                }))
                .unwrap();
                let recorded_at =
                    chrono::DateTime::parse_from_rfc3339(&format!("2026-07-27T12:34:{second:02}Z"))
                        .unwrap()
                        .with_timezone(&chrono::Utc);
                barrier.wait();
                append_gate_history(&root, &snapshot, recorded_at).unwrap_or_else(|error| {
                    panic!("append_gate_history failed for point-{second}: {error:#}");
                });
            })
        });
        for handle in handles {
            handle.join().expect("writer thread");
        }

        let history = std::fs::read_to_string(root.join(".anvil").join(GATE_HISTORY_FILE)).unwrap();
        assert!(
            history.contains("point-1"),
            "history missing point-1: {history}"
        );
        assert!(
            history.contains("point-2"),
            "history missing point-2: {history}"
        );
        assert_eq!(history.lines().count(), 2);
    }

    #[test]
    fn cap_evicts_oldest_physical_corrupt_line_for_a_new_valid_point() {
        let mut lines = (0..GATE_HISTORY_LINE_CAP)
            .map(|index| format!("corrupt-{index}").into_bytes())
            .collect::<Vec<_>>();
        lines.push(
            br#"{"recorded_at":"2026-07-27T12:34:56Z","score":100,"status":"pass","status_label":"new-valid","warning_count":0}"#.to_vec(),
        );
        let now = chrono::DateTime::parse_from_rfc3339("2026-07-27T12:34:56Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        let retained = retain_gate_history_lines(lines, now);

        assert_eq!(retained.len(), GATE_HISTORY_LINE_CAP);
        assert!(!retained.iter().any(|line| line == b"corrupt-0"));
        assert!(
            retained
                .iter()
                .any(|line| line.windows(9).any(|part| part == b"new-valid"))
        );
    }

    #[test]
    fn oversized_history_is_non_fatal_to_latest_snapshot() {
        let workspace = tempfile::tempdir().expect("workspace");
        std::fs::create_dir(workspace.path().join(".anvil")).unwrap();
        let oversized = (0..600)
            .map(|index| format!("corrupt-{index:04}-{}", "x".repeat(2035)))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(
            workspace.path().join(".anvil/gate-history.ndjson"),
            oversized,
        )
        .unwrap();
        let snapshot: GateSnapshot = serde_json::from_value(serde_json::json!({
            "status": "pass", "statusLabel": "PASSED", "score": 100.0,
            "checksRun": "4", "warnings": "0", "durationSeconds": "0.5",
            "checkRows": [], "warningList": []
        }))
        .unwrap();

        persist_gate_snapshot_at_root(workspace.path(), &snapshot);

        assert!(workspace.path().join(".anvil/gates.json").is_file());
        let history = std::fs::read(workspace.path().join(".anvil/gate-history.ndjson")).unwrap();
        assert!(history.len() <= GATE_HISTORY_MAX_BYTES);
        assert_eq!(
            history
                .split(|byte| *byte == b'\n')
                .filter(|line| !line.is_empty())
                .count(),
            500
        );
        assert!(
            String::from_utf8(history)
                .unwrap()
                .contains("\"status_label\":\"PASSED\"")
        );
    }

    #[test]
    fn contended_history_lock_is_prompt_and_latest_snapshot_still_persists() {
        use fs2::FileExt as _;

        let workspace = tempfile::tempdir().expect("workspace");
        std::fs::create_dir(workspace.path().join(".anvil")).unwrap();
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(workspace.path().join(".anvil/.gate-history.lock"))
            .unwrap();
        lock.lock_exclusive().unwrap();
        // Hold longer than GATE_HISTORY_LOCK_TIMEOUT (2s) so the writer
        // times out rather than waiting for the holder.
        let release = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(3_500));
            drop(lock);
        });
        let snapshot: GateSnapshot = serde_json::from_value(serde_json::json!({
            "status": "pass", "statusLabel": "PASSED", "score": 100.0,
            "checksRun": "4", "warnings": "0", "durationSeconds": "0.5",
            "checkRows": [], "warningList": []
        }))
        .unwrap();

        let started = std::time::Instant::now();
        persist_gate_snapshot_at_root(workspace.path(), &snapshot);
        let elapsed = started.elapsed();
        release.join().unwrap();

        assert!(
            elapsed >= GATE_HISTORY_LOCK_TIMEOUT,
            "should wait for the lock timeout before giving up: {elapsed:?}"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(3),
            "must not block on the lock holder for the full hold duration: {elapsed:?}"
        );
        assert!(workspace.path().join(".anvil/gates.json").is_file());
        assert!(!workspace.path().join(".anvil/gate-history.ndjson").exists());
    }

    #[cfg(unix)]
    #[test]
    fn gate_snapshot_refuses_a_symlinked_anvil_parent() {
        use std::os::unix::fs::symlink;

        let workspace = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        symlink(outside.path(), workspace.path().join(".anvil")).expect("symlink .anvil");

        let error = persist_gate_snapshot_json(workspace.path(), b"redirected")
            .expect_err("symlinked .anvil must fail closed");

        assert!(
            format!("{error:#}").contains("symlink"),
            "error should identify the unsafe component: {error:#}"
        );
        assert!(
            !outside.path().join("gates.json").exists(),
            "the snapshot must not escape the workspace"
        );
    }

    #[cfg(windows)]
    #[test]
    fn gate_snapshot_refuses_a_junctioned_anvil_parent() {
        let workspace = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(workspace.path().join(".anvil"))
            .arg(outside.path())
            .status()
            .expect("run mklink");
        assert!(
            status.success(),
            "mklink /J creates an unprivileged junction"
        );

        let error = persist_gate_snapshot_json(workspace.path(), b"redirected")
            .expect_err("junctioned .anvil must fail closed");

        assert!(
            format!("{error:#}").contains("reparse"),
            "error should identify the unsafe component: {error:#}"
        );
        assert!(
            !outside.path().join("gates.json").exists(),
            "the snapshot must not escape the workspace"
        );
    }

    #[test]
    fn is_skipped_for_missing_config_only_fires_for_config_dependent_checks() {
        assert!(is_skipped_for_missing_config(
            "architecture",
            "No architecture config found. Skipping."
        ));
        assert!(is_skipped_for_missing_config(
            "policy",
            "No policy bundle found. Skipping."
        ));
        assert!(is_skipped_for_missing_config(
            "command-safety",
            "Command-safety check disabled. Skipping."
        ));
        // Command-safety with no plan supplied — also a project-config gap.
        assert!(is_skipped_for_missing_config(
            "command-safety",
            "No commands to analyse"
        ));
        // OPA-not-installed is a host-tooling gap, not a project-config
        // gap; strict mode must NOT block on it.
        assert!(!is_skipped_for_missing_config(
            "policy",
            "OPA not installed. Skipping policy evaluation."
        ));
        // Secret detection skipping is content-driven, not config-driven —
        // strict_config must not elevate it to a blocking diagnostic.
        assert!(!is_skipped_for_missing_config("secret", "Skipping."));
        assert!(!is_skipped_for_missing_config("architecture", "All good"));
    }

    #[test]
    fn check_name_to_category_covers_every_ai_guardrail_check() {
        // Every check listed in AI_GUARDRAIL_CHECKS must map to a
        // dedicated Category — Other is a routing failure that hides the
        // signal from `summary.by_category` in the AI envelope.
        for canonical in [
            "secret-detection",
            "antipattern-scan",
            "import-boundaries",
            "architecture",
            "policy",
            "command-safety",
        ] {
            let cat = check_name_to_category(canonical);
            assert!(
                !matches!(cat, Category::Other),
                "{canonical} must map to a non-Other category"
            );
        }
    }

    #[test]
    fn resolve_profile_skip_set_rejects_unknown_check_names() {
        // Mock a profile whose skip list contains a typo. Currently
        // PROFILES are static so we can only assert the present-day
        // contents always resolve. This guard prevents future profile
        // edits from silently failing open.
        for (name, _, skips) in PROFILES {
            let result = resolve_profile_skip_set(Some(*name));
            assert!(
                result.is_ok(),
                "profile '{name}' has unresolvable skip entries: {skips:?}"
            );
        }
    }

    // ── Coverage check tests ──────────────────────────────────────────

    #[test]
    fn coverage_no_report_skips() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = run_check_coverage(tmp.path(), 80.0);
        assert!(result.passed);
        assert!(result.message.contains("Skipping"));
    }

    #[test]
    fn coverage_lcov_above_threshold() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cov_dir = tmp.path().join("coverage");
        std::fs::create_dir_all(&cov_dir).unwrap();
        std::fs::write(
            cov_dir.join("lcov.info"),
            "SF:src/main.rs\nLF:100\nLH:90\nend_of_record\n",
        )
        .unwrap();
        let result = run_check_coverage(tmp.path(), 80.0);
        assert!(result.passed);
        assert!(result.message.contains("90.0%"));
    }

    #[test]
    fn coverage_lcov_below_threshold() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cov_dir = tmp.path().join("coverage");
        std::fs::create_dir_all(&cov_dir).unwrap();
        std::fs::write(
            cov_dir.join("lcov.info"),
            "SF:src/main.rs\nLF:100\nLH:50\nend_of_record\n",
        )
        .unwrap();
        let result = run_check_coverage(tmp.path(), 80.0);
        assert!(!result.passed);
        assert!(result.message.contains("50.0%"));
    }

    #[test]
    fn coverage_cobertura_above_threshold() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cov_dir = tmp.path().join("coverage");
        std::fs::create_dir_all(&cov_dir).unwrap();
        std::fs::write(
            cov_dir.join("cobertura.xml"),
            r#"<?xml version="1.0"?><coverage line-rate="0.95"></coverage>"#,
        )
        .unwrap();
        let result = run_check_coverage(tmp.path(), 80.0);
        assert!(result.passed);
        assert!(result.message.contains("95.0%"));
    }

    // ── Dependency check tests ──────────────────────────────────────

    #[test]
    fn dependency_no_lockfile_skips() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = run_check_dependency(tmp.path());
        assert!(result.passed);
        assert!(result.message.contains("Skipping"));
    }

    #[test]
    fn dependency_clean_lockfile_passes() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("package-lock.json"),
            r#"{"lockfileVersion":3,"packages":{"node_modules/express":{}}}"#,
        )
        .unwrap();
        let result = run_check_dependency(tmp.path());
        assert!(result.passed);
        assert!((result.score - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn dependency_blocked_package_fails() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("package-lock.json"),
            r#"{"lockfileVersion":3,"packages":{"node_modules/event-stream":{"version":"4.0.1"}}}"#,
        )
        .unwrap();
        let result = run_check_dependency(tmp.path());
        assert!(!result.passed);
        assert!(result.message.contains("event-stream"));
    }

    // ── Architecture check tests ────────────────────────────────────

    #[test]
    fn architecture_no_config_skips() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = run_check_architecture(tmp.path());
        assert!(result.passed);
        assert!(result.message.contains("Skipping"));
    }

    #[test]
    fn architecture_valid_config_passes() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(
            anvil_dir.join("architecture.yaml"),
            "boundaries:\n  - name: core\n    path: src/core\n",
        )
        .unwrap();
        let result = run_check_architecture(tmp.path());
        assert!(result.passed);
    }

    #[test]
    fn architecture_invalid_yaml_fails() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(anvil_dir.join("architecture.yaml"), "bad: [unclosed").unwrap();
        let result = run_check_architecture(tmp.path());
        assert!(!result.passed);
    }

    #[test]
    fn architecture_config_preflight_blocks_unknown_layer_dependency() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(
            anvil_dir.join("architecture.yaml"),
            "template: layered\nlayers:\n  ui:\n    patterns: [\"src/ui/**\"]\n    depends_on: [domain]\n",
        )
        .unwrap();

        let result = run_check_architecture(tmp.path());

        assert!(!result.passed);
        assert!(result.message.contains("preflight"));
        assert!(result.message.contains("unknown layer"));
    }

    #[test]
    fn architecture_config_preflight_blocks_overlapping_layers() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(
            anvil_dir.join("architecture.yaml"),
            "template: custom\nlayers:\n  app:\n    patterns: [\"src/**\"]\n    depends_on: []\n  ui:\n    patterns: [\"src/ui/**\"]\n    depends_on: []\n",
        )
        .unwrap();

        let result = run_check_architecture(tmp.path());

        assert!(!result.passed);
        assert!(result.message.contains("preflight"));
        assert!(result.message.contains("overlaps"));
    }

    // ── Policy check tests ──────────────────────────────────────────

    #[test]
    fn policy_no_bundle_skips() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = run_check_policy(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(result.passed);
        assert!(result.message.contains("Skipping"));
    }

    #[test]
    fn policy_with_bundle_evaluates() {
        let tmp = tempfile::TempDir::new().unwrap();
        let policy_dir = tmp.path().join(".anvil/policies");
        std::fs::create_dir_all(&policy_dir).unwrap();
        // A valid, empty policy under the anvil.policies namespace produces no
        // findings — regorus is embedded, so this always evaluates and passes.
        std::fs::write(
            policy_dir.join("noop.rego"),
            "package anvil.policies.noop\n",
        )
        .unwrap();
        let result = run_check_policy(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(result.passed, "unexpected failure: {}", result.message);
        assert!(
            result.message.contains("no violations"),
            "{}",
            result.message
        );
    }

    #[test]
    fn policy_empty_bundle_dir_skips() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".anvil/policies")).unwrap();
        let result = run_check_policy(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(result.passed);
        assert!(result.message.contains("Skipping"), "{}", result.message);
    }

    #[test]
    fn policy_violation_surfaces_as_failure() {
        let tmp = tempfile::TempDir::new().unwrap();
        let policy_dir = tmp.path().join(".anvil/policies");
        std::fs::create_dir_all(&policy_dir).unwrap();
        std::fs::write(
            policy_dir.join("boom.rego"),
            "package anvil.policies.boom\nimport rego.v1\n\nviolation contains msg if { msg := \"boom happened\" }\n",
        )
        .unwrap();
        let result = run_check_policy(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(
            !result.passed,
            "violation must fail the check: {}",
            result.message
        );
        assert!(
            result.message.contains("boom happened"),
            "{}",
            result.message
        );
        assert!(result.message.contains("[error]"), "{}", result.message);
    }

    #[test]
    fn policy_warning_does_not_fail() {
        let tmp = tempfile::TempDir::new().unwrap();
        let policy_dir = tmp.path().join(".anvil/policies");
        std::fs::create_dir_all(&policy_dir).unwrap();
        std::fs::write(
            policy_dir.join("heads_up.rego"),
            "package anvil.policies.heads_up\nimport rego.v1\n\nwarn contains msg if { msg := \"heads up\" }\n",
        )
        .unwrap();
        let result = run_check_policy(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(
            result.passed,
            "warning must not fail the check: {}",
            result.message
        );
        assert!(result.message.contains("heads up"), "{}", result.message);
        assert!(result.message.contains("warning"), "{}", result.message);
    }

    #[test]
    fn policy_uncompilable_rego_is_reported_not_skipped() {
        let tmp = tempfile::TempDir::new().unwrap();
        let policy_dir = tmp.path().join(".anvil/policies");
        std::fs::create_dir_all(&policy_dir).unwrap();
        std::fs::write(
            policy_dir.join("broken.rego"),
            "package anvil.policies.broken\nthis is not valid rego @#$%\n",
        )
        .unwrap();
        let result = run_check_policy(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(
            !result.passed,
            "uncompilable policy must be reported: {}",
            result.message
        );
        assert!(
            result.message.contains("compile"),
            "expected a compile-failure message, got: {}",
            result.message
        );
        assert!(
            !result.message.contains("Skipping"),
            "must not skip: {}",
            result.message
        );
    }

    #[test]
    fn policy_test_rego_files_are_excluded_from_evaluation() {
        let tmp = tempfile::TempDir::new().unwrap();
        let policy_dir = tmp.path().join(".anvil/policies");
        std::fs::create_dir_all(&policy_dir).unwrap();
        // A valid policy that only warns (so the check passes)...
        std::fs::write(
            policy_dir.join("ok.rego"),
            "package anvil.policies.ok\nimport rego.v1\n\nwarn contains msg if { msg := \"note\" }\n",
        )
        .unwrap();
        // ...plus a *_test.rego that would fail to compile if it were loaded.
        std::fs::write(
            policy_dir.join("ok_test.rego"),
            "package anvil.policies.ok_test\nnot valid rego @#$%\n",
        )
        .unwrap();
        let result = run_check_policy(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(
            result.passed,
            "the *_test.rego must be excluded, so no compile failure: {}",
            result.message
        );
        assert!(!result.message.contains("compile"), "{}", result.message);
    }

    #[test]
    fn extract_policy_findings_maps_severities() {
        let value = serde_json::json!({
            "sec": {"violation": ["a bare string violation"]},
            "scope": {"warn": [{"message": "obj warning"}]},
            "cov": {"info": ["ignored info"], "helper_rule": 42},
        });
        let findings = extract_policy_findings(&value);
        let errors: Vec<_> = findings.iter().filter(|f| f.severity == "error").collect();
        let warns: Vec<_> = findings
            .iter()
            .filter(|f| f.severity == "warning")
            .collect();
        assert_eq!(errors.len(), 1, "one error-class finding");
        assert_eq!(warns.len(), 1, "one warning-class finding");
        // `info` and non-array helper rules are ignored, as on the OPA path.
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn extract_policy_findings_recognises_documented_warning_rule() {
        // `docs/guides/opa-policy-testing.md`: "Both `violation` and `warning`
        // rule sets are recognised by the gate". The `warning` (singular) rule
        // set is the documented contract — regression guard for the warn/warning
        // vocabulary fix (the starter pack emits `warning`).
        let value = serde_json::json!({
            "sensitive_paths": {"warning": ["review this sensitive change"]},
        });
        let findings = extract_policy_findings(&value);
        assert_eq!(
            findings.len(),
            1,
            "the documented `warning` rule must surface"
        );
        assert_eq!(findings[0].severity, "warning");
        assert_eq!(findings[0].message, "review this sensitive change");
        // The documented families are the crate single source of truth and
        // include the canonical `violation` / `warning` names.
        assert!(crate::policy_vocab::VIOLATION_FAMILY_KEYS.contains(&"violation"));
        assert!(crate::policy_vocab::WARNING_FAMILY_KEYS.contains(&"warning"));
    }

    #[test]
    fn resolve_policy_severity_fails_closed_on_unrecognised() {
        // A recognised override is honoured.
        assert_eq!(resolve_policy_severity(Some("error"), "warning"), "error");
        assert_eq!(resolve_policy_severity(Some("warn"), "error"), "warning");
        // `info` folds into the non-blocking warning class — the gate
        // vocabulary is two-valued (error/warning), never `info`.
        assert_eq!(resolve_policy_severity(Some("info"), "error"), "warning");
        // An unrecognised override falls back to the rule's own default rather
        // than being accepted verbatim (which would land it non-blocking).
        assert_eq!(resolve_policy_severity(Some("critical"), "error"), "error");
        assert_eq!(
            resolve_policy_severity(Some("garbage"), "warning"),
            "warning"
        );
        assert_eq!(resolve_policy_severity(None, "error"), "error");
    }

    #[test]
    fn resolve_policy_severity_is_two_valued() {
        // Whatever the override, the result is only ever error or warning —
        // never `info`, so the downstream error/warning partition is total.
        for over in [
            None,
            Some("error"),
            Some("warn"),
            Some("info"),
            Some("nope"),
        ] {
            for default in ["error", "warning"] {
                let got = resolve_policy_severity(over, default);
                assert!(got == "error" || got == "warning", "got {got:?}");
            }
        }
    }

    #[test]
    fn policy_unrecognised_severity_on_violation_still_blocks() {
        // A `violation` rule item with an unrecognised severity string must
        // fail closed to error (block), not slip into the non-blocking bucket.
        let value = serde_json::json!({
            "sec": {"violation": [{"message": "boom", "severity": "critical"}]},
            "scope": {"warn": [{"message": "note", "severity": "garbage"}]},
        });
        let findings = extract_policy_findings(&value);
        let blocking = findings.iter().find(|f| f.message == "boom").unwrap();
        assert_eq!(
            blocking.severity, "error",
            "critical must fail closed to error"
        );
        let warning = findings.iter().find(|f| f.message == "note").unwrap();
        assert_eq!(warning.severity, "warning", "garbage must stay warning");
    }

    #[test]
    fn policy_generated_dir_files_are_not_evaluated() {
        let tmp = tempfile::TempDir::new().unwrap();
        let generated = tmp.path().join(".anvil/policies/.generated");
        std::fs::create_dir_all(&generated).unwrap();
        // A generated policy that would fail the check if it were evaluated.
        std::fs::write(
            generated.join("auto.rego"),
            "package anvil.policies.auto\nimport rego.v1\n\nviolation contains msg if { msg := \"generated boom\" }\n",
        )
        .unwrap();
        let result = run_check_policy(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(
            result.passed,
            "generated policy must be excluded: {}",
            result.message
        );
        assert!(
            !result.message.contains("generated boom"),
            "{}",
            result.message
        );
    }

    #[test]
    fn policy_generated_header_files_are_not_evaluated() {
        let tmp = tempfile::TempDir::new().unwrap();
        let policy_dir = tmp.path().join(".anvil/policies");
        std::fs::create_dir_all(&policy_dir).unwrap();
        std::fs::write(
            policy_dir.join("auto.rego"),
            "# Auto-generated by anvil\npackage anvil.policies.auto\nimport rego.v1\n\nviolation contains msg if { msg := \"header boom\" }\n",
        )
        .unwrap();
        let result = run_check_policy(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(
            result.passed,
            "header-generated policy must be excluded: {}",
            result.message
        );
        assert!(
            !result.message.contains("header boom"),
            "{}",
            result.message
        );
    }

    #[test]
    fn policy_legacy_capitalised_generated_header_files_are_not_evaluated() {
        let tmp = tempfile::TempDir::new().unwrap();
        let policy_dir = tmp.path().join(".anvil/policies");
        std::fs::create_dir_all(&policy_dir).unwrap();
        // Pre-rebrand header form must remain excluded so existing generated
        // policies keep working after the lowercase brand change.
        std::fs::write(
            policy_dir.join("legacy.rego"),
            "# Auto-generated by Anvil\npackage anvil.policies.legacy\nimport rego.v1\n\nviolation contains msg if { msg := \"legacy header boom\" }\n",
        )
        .unwrap();
        let result = run_check_policy(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(
            result.passed,
            "legacy capitalised header-generated policy must be excluded: {}",
            result.message
        );
        assert!(
            !result.message.contains("legacy header boom"),
            "{}",
            result.message
        );
    }

    #[test]
    fn policy_oversized_generated_file_is_skipped_not_reported() {
        let tmp = tempfile::TempDir::new().unwrap();
        let policy_dir = tmp.path().join(".anvil/policies");
        std::fs::create_dir_all(&policy_dir).unwrap();
        // A generated file over the per-policy cap: generated-ness is decided
        // before the caps, so it is skipped, not reported as oversized.
        let mut content = String::from("# Auto-generated by anvil\n");
        content.push_str(&"x".repeat(usize::try_from(GATE_MAX_POLICY_BYTES).unwrap() + 1));
        std::fs::write(policy_dir.join("huge_auto.rego"), content).unwrap();
        let result = run_check_policy(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(
            result.passed,
            "oversized generated file must be skipped: {}",
            result.message
        );
        assert!(
            !result.message.contains("per-policy limit"),
            "must not be reported as oversized: {}",
            result.message
        );
    }

    #[cfg(unix)]
    #[test]
    fn policy_unreadable_subdirectory_is_reported_not_passed() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::TempDir::new().unwrap();
        let policy_dir = tmp.path().join(".anvil/policies");
        std::fs::create_dir_all(&policy_dir).unwrap();
        std::fs::write(policy_dir.join("ok.rego"), "package anvil.policies.ok\n").unwrap();
        let locked = policy_dir.join("locked");
        std::fs::create_dir(&locked).unwrap();
        std::fs::write(
            locked.join("hidden.rego"),
            "package anvil.policies.hidden\n",
        )
        .unwrap();
        // Make the subdirectory untraversable so walkdir yields an error.
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

        let result = run_check_policy(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );

        // Restore permissions so the TempDir can be cleaned up.
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert!(
            !result.passed,
            "a traversal error must be reported, not silently passed: {}",
            result.message
        );
        assert!(
            result.message.contains("Failed to read policy bundle"),
            "{}",
            result.message
        );
    }

    #[test]
    fn policy_oversized_file_is_reported() {
        let tmp = tempfile::TempDir::new().unwrap();
        let policy_dir = tmp.path().join(".anvil/policies");
        std::fs::create_dir_all(&policy_dir).unwrap();
        // 1 MiB + 1 byte — over the per-policy cap. Content need not compile;
        // the size guard fires before the file is read.
        let oversized = "x".repeat(usize::try_from(GATE_MAX_POLICY_BYTES).unwrap() + 1);
        std::fs::write(policy_dir.join("huge.rego"), oversized).unwrap();
        let result = run_check_policy(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(
            !result.passed,
            "oversized policy must be reported: {}",
            result.message
        );
        assert!(
            result.message.contains("per-policy limit"),
            "{}",
            result.message
        );
        assert!(!result.message.contains("Skipping"), "{}", result.message);
    }

    #[cfg(unix)]
    #[test]
    fn policy_symlinked_bundle_dir_escaping_root_is_reported() {
        let workspace = tempfile::TempDir::new().unwrap();
        let external = tempfile::TempDir::new().unwrap();
        std::fs::write(
            external.path().join("evil.rego"),
            "package anvil.policies.evil\nimport rego.v1\n\nviolation contains msg if { msg := \"leak\" }\n",
        )
        .unwrap();
        std::fs::create_dir_all(workspace.path().join(".anvil")).unwrap();
        // .anvil/policies is a symlink pointing outside the workspace root.
        std::os::unix::fs::symlink(external.path(), workspace.path().join(".anvil/policies"))
            .unwrap();
        let result = run_check_policy(
            workspace.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(
            !result.passed,
            "escaping bundle dir must be reported: {}",
            result.message
        );
        assert!(result.message.contains("containment"), "{}", result.message);
    }

    #[cfg(unix)]
    #[test]
    fn policy_symlinked_file_escaping_root_is_reported() {
        let workspace = tempfile::TempDir::new().unwrap();
        let external = tempfile::TempDir::new().unwrap();
        let external_policy = external.path().join("secret.rego");
        std::fs::write(&external_policy, "package anvil.policies.secret\n").unwrap();
        let policy_dir = workspace.path().join(".anvil/policies");
        std::fs::create_dir_all(&policy_dir).unwrap();
        // A single `.rego` inside a legitimate bundle dir, symlinked out of root.
        std::os::unix::fs::symlink(&external_policy, policy_dir.join("linked.rego")).unwrap();
        let result = run_check_policy(
            workspace.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(
            !result.passed,
            "escaping policy file must be reported: {}",
            result.message
        );
        assert!(result.message.contains("containment"), "{}", result.message);
    }

    // ── Policy input context tests ─────────────────────────────────────

    #[test]
    fn build_policy_input_populates_workspace() {
        let tmp = tempfile::TempDir::new().unwrap();
        let input = build_policy_input(
            tmp.path(),
            Some("ci"),
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert_eq!(
            input["workspace"].as_str().unwrap(),
            tmp.path().to_string_lossy()
        );
        assert_eq!(input["profile"].as_str().unwrap(), "ci");
        assert!(input["files"].as_array().is_some());
        assert!(input["changed_files"].as_array().is_some());
    }

    #[test]
    fn build_policy_input_includes_source_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("main.ts"), "export const x = 1;").unwrap();
        std::fs::write(src.join("readme.md"), "# Hi").unwrap();

        let input = build_policy_input(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        let files: Vec<&str> = input["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        assert!(files.contains(&"src/main.ts"));
        assert!(!files.iter().any(|f| f.contains("readme.md")));
    }

    #[test]
    fn build_policy_input_defaults_profile() {
        let tmp = tempfile::TempDir::new().unwrap();
        let input = build_policy_input(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert_eq!(input["profile"].as_str().unwrap(), "default");
    }

    // ── Import resolution tests ────────────────────────────────────────

    #[test]
    fn resolve_import_sibling() {
        let resolved = resolve_import("src/app/service.ts", "./helper");
        assert_eq!(resolved.as_deref(), Some("src/app/helper"));
    }

    #[test]
    fn resolve_import_parent() {
        let resolved = resolve_import("src/app/service.ts", "../core/entity");
        assert_eq!(resolved.as_deref(), Some("src/core/entity"));
    }

    #[test]
    fn resolve_import_escapes_root() {
        let resolved = resolve_import("src/main.ts", "../../outside");
        assert!(resolved.is_none());
    }

    #[test]
    fn resolve_import_from_root_file() {
        let resolved = resolve_import("index.ts", "./src/lib");
        assert_eq!(resolved.as_deref(), Some("src/lib"));
    }

    // ── Architecture boundary detection tests ──────────────────────────

    #[test]
    fn architecture_detects_violations_with_edges() {
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();

        // Set up layers: core has no deps, app depends on core.
        // A core→app import is forbidden.
        std::fs::write(
            anvil_dir.join("architecture.yaml"),
            r#"
schema_version: "0.1.0"
template: custom
layers:
  core:
    patterns: ["src/core/**"]
    depends_on: []
  app:
    patterns: ["src/app/**"]
    depends_on: ["core"]
rules: []
"#,
        )
        .unwrap();

        // Create source files that produce an import edge.
        let core_dir = tmp.path().join("src/core");
        let app_dir = tmp.path().join("src/app");
        std::fs::create_dir_all(&core_dir).unwrap();
        std::fs::create_dir_all(&app_dir).unwrap();
        std::fs::write(
            core_dir.join("entity.ts"),
            "import { service } from '../app/service';\nexport const x = 1;\n",
        )
        .unwrap();
        std::fs::write(app_dir.join("service.ts"), "export const service = 1;\n").unwrap();

        let edges = extract_import_edges(tmp.path(), None);
        assert!(!edges.is_empty(), "should extract at least one import edge");

        let definition = anvil_architecture::parse_architecture_definition(tmp.path()).unwrap();
        let result =
            anvil_architecture::validate_with_edges(tmp.path(), &definition, &edges).unwrap();

        assert!(
            !result.violations.is_empty(),
            "core importing from app should produce a boundary violation"
        );
        assert!(!result.valid);
    }

    #[test]
    fn architecture_detects_rust_crate_path_violations() {
        // RSTLAN-005: a cross-layer Rust `use crate::…` import must produce a
        // boundary violation, just like the TS case above.
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(
            anvil_dir.join("architecture.yaml"),
            r#"
schema_version: "0.1.0"
template: custom
layers:
  core:
    patterns: ["src/core/**"]
    depends_on: []
  app:
    patterns: ["src/app/**"]
    depends_on: ["core"]
rules: []
"#,
        )
        .unwrap();

        // A Cargo.toml at the root is what `resolve_rust_import` walks to for the
        // crate `src/` root.
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        let core_dir = tmp.path().join("src/core");
        let app_dir = tmp.path().join("src/app");
        std::fs::create_dir_all(&core_dir).unwrap();
        std::fs::create_dir_all(&app_dir).unwrap();
        // core → app is forbidden (core depends_on []).
        std::fs::write(
            core_dir.join("entity.rs"),
            "use crate::app::service::Service;\npub struct Entity;\n",
        )
        .unwrap();
        std::fs::write(app_dir.join("service.rs"), "pub struct Service;\n").unwrap();

        let edges = extract_import_edges(tmp.path(), None);
        assert!(
            edges
                .iter()
                .any(|e| e.from_file == "src/core/entity.rs" && e.to_file == "src/app/service.rs"),
            "the crate::app::service import should resolve to src/app/service.rs, got {edges:?}"
        );

        let definition = anvil_architecture::parse_architecture_definition(tmp.path()).unwrap();
        let result =
            anvil_architecture::validate_with_edges(tmp.path(), &definition, &edges).unwrap();
        assert!(
            !result.violations.is_empty(),
            "core importing from app (Rust crate:: path) should violate the boundary"
        );
        assert!(!result.valid);
    }

    #[test]
    fn architecture_rust_external_imports_are_not_violations() {
        // `std::`/external-crate imports target code outside the workspace and
        // must never be flagged.
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(
            anvil_dir.join("architecture.yaml"),
            r#"
schema_version: "0.1.0"
template: custom
layers:
  core:
    patterns: ["src/core/**"]
    depends_on: []
rules: []
"#,
        )
        .unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        let core_dir = tmp.path().join("src/core");
        std::fs::create_dir_all(&core_dir).unwrap();
        std::fs::write(
            core_dir.join("entity.rs"),
            "use std::collections::HashMap;\nuse serde::Deserialize;\npub struct Entity;\n",
        )
        .unwrap();

        let edges = extract_import_edges(tmp.path(), None);
        assert!(
            edges.is_empty(),
            "external/std imports must not produce in-workspace edges, got {edges:?}"
        );
    }

    #[test]
    fn architecture_detects_python_cross_layer_violation() {
        // PYLAN-006: a Python absolute import that crosses a forbidden layer
        // boundary must resolve to the `.py` file and produce a violation, via
        // the language-aware dispatch (`ext == "py"` → resolve_python_import).
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(
            anvil_dir.join("architecture.yaml"),
            r#"
schema_version: "0.1.0"
template: custom
layers:
  core:
    patterns: ["src/core/**"]
    depends_on: []
  app:
    patterns: ["src/app/**"]
    depends_on: ["core"]
rules: []
"#,
        )
        .unwrap();

        let core_dir = tmp.path().join("src/core");
        let app_dir = tmp.path().join("src/app");
        std::fs::create_dir_all(&core_dir).unwrap();
        std::fs::create_dir_all(&app_dir).unwrap();
        // core → app is forbidden (core depends_on []).
        std::fs::write(
            core_dir.join("entity.py"),
            "from app.service import Service\n\nclass Entity:\n    pass\n",
        )
        .unwrap();
        std::fs::write(app_dir.join("service.py"), "class Service:\n    pass\n").unwrap();

        let edges = extract_import_edges(tmp.path(), None);
        assert!(
            edges
                .iter()
                .any(|e| e.from_file == "src/core/entity.py" && e.to_file == "src/app/service.py"),
            "the `from app.service import` should resolve to src/app/service.py, got {edges:?}"
        );

        let definition = anvil_architecture::parse_architecture_definition(tmp.path()).unwrap();
        let result =
            anvil_architecture::validate_with_edges(tmp.path(), &definition, &edges).unwrap();
        assert!(
            !result.violations.is_empty(),
            "core importing from app (Python import) should violate the boundary"
        );
        assert!(!result.valid);
    }

    #[test]
    fn architecture_python_external_imports_are_not_violations() {
        // stdlib / third-party Python imports target code outside the workspace
        // and must never be flagged.
        let tmp = tempfile::TempDir::new().unwrap();
        let anvil_dir = tmp.path().join(".anvil");
        std::fs::create_dir_all(&anvil_dir).unwrap();
        std::fs::write(
            anvil_dir.join("architecture.yaml"),
            r#"
schema_version: "0.1.0"
template: custom
layers:
  core:
    patterns: ["src/core/**"]
    depends_on: []
rules: []
"#,
        )
        .unwrap();
        let core_dir = tmp.path().join("src/core");
        std::fs::create_dir_all(&core_dir).unwrap();
        std::fs::write(
            core_dir.join("entity.py"),
            "import os\nfrom collections import OrderedDict\nimport numpy as np\n\nclass Entity:\n    pass\n",
        )
        .unwrap();

        let edges = extract_import_edges(tmp.path(), None);
        assert!(
            edges.is_empty(),
            "external/stdlib Python imports must not produce in-workspace edges, got {edges:?}"
        );
    }

    // ── Plan scoping tests ─────────────────────────────────────────────

    #[test]
    fn extract_plan_files_parses_files_lines() {
        let tmp = tempfile::TempDir::new().unwrap();
        let plan = tmp.path().join("test.aps.md");
        std::fs::write(
            &plan,
            r"
### ITEM-001: do something

- **Status:** In Progress
- **Intent:** Some work
- **Files:** `src/core/entity.ts`, `src/app/service.ts`
- **Confidence:** high
",
        )
        .unwrap();

        let files = extract_plan_files(&plan);
        assert!(files.contains("src/core/entity.ts"));
        assert!(files.contains("src/app/service.ts"));
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn extract_plan_files_skips_non_path_backticks() {
        let tmp = tempfile::TempDir::new().unwrap();
        let plan = tmp.path().join("test.aps.md");
        std::fs::write(
            &plan,
            "- **Files:** `src/main.ts`\n\nSome text with `inline code` here.\n",
        )
        .unwrap();

        let files = extract_plan_files(&plan);
        assert!(files.contains("src/main.ts"));
        assert!(!files.contains("inline code"));
    }

    #[test]
    fn extract_plan_files_returns_empty_for_missing_file() {
        let files = extract_plan_files(Path::new("/nonexistent/plan.aps.md"));
        assert!(files.is_empty());
    }

    #[test]
    fn resolve_plan_path_finds_in_modules() {
        let root = crate::util::workspace_root().unwrap();
        let modules_dir = root.join("plans/modules");
        if modules_dir.exists() {
            // Only run on actual workspace with plans.
            if let Some(path) = resolve_plan_path("rust-cli", &root) {
                assert!(path.to_string_lossy().ends_with(".aps.md"));
            }
        }
    }

    #[test]
    fn build_policy_input_includes_plan_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let input = build_policy_input(
            tmp.path(),
            None,
            Some("/plans/test.aps.md"),
            &std::collections::HashSet::new(),
            None,
        );
        assert_eq!(input["plan_path"].as_str().unwrap(), "/plans/test.aps.md");
    }

    #[test]
    fn build_policy_input_omits_plan_when_none() {
        let tmp = tempfile::TempDir::new().unwrap();
        let input = build_policy_input(
            tmp.path(),
            None,
            None,
            &std::collections::HashSet::new(),
            None,
        );
        assert!(input.get("plan_path").is_none());
    }

    // ── Secret check integration tests ────────────────────────────────
    //
    // These exercise the anvil-checks wiring that gate.rs delegates to,
    // using temp files to avoid coupling to the real workspace.

    #[test]
    fn secret_check_clean_file_passes() {
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("clean.ts");
        std::fs::write(&file, "export const x = 1;\n").unwrap();

        let files = [file.to_string_lossy().to_string()];
        let file_refs: Vec<&str> = files.iter().map(String::as_str).collect();
        let config = anvil_checks::secret::SecretCheckConfig::default();
        let result = anvil_checks::secret::run_secret_check(&file_refs, &config, None);

        assert!(result.passed);
        assert_eq!(result.findings.len(), 0);
    }

    #[test]
    fn secret_check_detects_aws_secret_key() {
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("creds.ts");
        let secret = "abcdabcdabcdabcdabcdabcdabcdabcdabcdabcd";
        std::fs::write(&file, format!("aws_secret_access_key='{secret}'")).unwrap();

        let files = [file.to_string_lossy().to_string()];
        let file_refs: Vec<&str> = files.iter().map(String::as_str).collect();
        let config = anvil_checks::secret::SecretCheckConfig::default();
        let result = anvil_checks::secret::run_secret_check(&file_refs, &config, None);

        assert!(!result.passed);
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.pattern_name == "AWS Secret Key"),
            "should detect AWS Secret Key pattern"
        );
    }

    #[test]
    fn secret_check_detects_stripe_key_with_pattern_name() {
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("billing.ts");
        let stripe = format!("sk_live_{}", "1234567890abcdefghijABCD");
        std::fs::write(&file, format!("const secret = '{stripe}';")).unwrap();

        let files = [file.to_string_lossy().to_string()];
        let file_refs: Vec<&str> = files.iter().map(String::as_str).collect();
        let config = anvil_checks::secret::SecretCheckConfig::default();
        let result = anvil_checks::secret::run_secret_check(&file_refs, &config, None);

        assert!(!result.passed);
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.pattern_name.contains("Stripe")),
            "should detect Stripe key pattern by name"
        );
    }

    #[test]
    fn secret_check_result_maps_to_check_result_format() {
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("leak.ts");
        let secret = "abcdabcdabcdabcdabcdabcdabcdabcdabcdabcd";
        std::fs::write(&file, format!("aws_secret_access_key='{secret}'")).unwrap();

        let files = [file.to_string_lossy().to_string()];
        let file_refs: Vec<&str> = files.iter().map(String::as_str).collect();
        let config = anvil_checks::secret::SecretCheckConfig::default();
        let root_str = tmp.path().to_string_lossy().to_string();
        let result = anvil_checks::secret::run_secret_check(&file_refs, &config, Some(&root_str));

        // Call the same formatter `run_check_secret` uses. Re-implementing the
        // format here is what let the secret and antipattern styles drift
        // apart unnoticed (CIB-237).
        let locations: Vec<String> = result
            .findings
            .iter()
            .map(|f| secret_finding_location(f, &file_refs, tmp.path()))
            .collect();
        assert!(!locations.is_empty());
        assert!(
            locations[0].contains("[AWS Secret Key]"),
            "location should include pattern name in brackets, got: {}",
            locations[0]
        );
        assert!(
            locations[0].starts_with("leak.ts:"),
            "secret location must be workspace-relative like every other check \
             family in the same gate run, got: {}",
            locations[0]
        );
    }

    /// The two styles CIB-237 saw side by side in one gate run.
    #[test]
    fn secret_and_antipattern_locations_share_one_path_style() {
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join(".env");
        let secret = "abcdabcdabcdabcdabcdabcdabcdabcdabcdabcd";
        std::fs::write(&file, format!("aws_secret_access_key='{secret}'")).unwrap();

        let files = [file.to_string_lossy().to_string()];
        let file_refs: Vec<&str> = files.iter().map(String::as_str).collect();
        let config = anvil_checks::secret::SecretCheckConfig::default();
        let root_str = tmp.path().to_string_lossy().to_string();
        let result = anvil_checks::secret::run_secret_check(&file_refs, &config, Some(&root_str));

        let secret_location = secret_finding_location(&result.findings[0], &file_refs, tmp.path());
        let antipattern_location = warning_location("src/app.py", 12, "AP-003");

        assert!(
            !secret_location.starts_with('/'),
            "secret location must not lead with the scanner's root marker, got: {secret_location}"
        );
        assert!(
            secret_location.starts_with(".env:"),
            "got: {secret_location}"
        );
        assert_eq!(antipattern_location, "src/app.py:12 [AP-003]");
    }

    #[test]
    fn warning_location_omits_the_whole_file_sentinel() {
        assert_eq!(warning_location(".env", 0, "AP-003"), ".env [AP-003]");
    }

    #[test]
    fn antipattern_check_detects_explicit_any() {
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("warn.ts");
        std::fs::write(&file, "const value: any = source;\n").unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
            false,
        );

        // AP-003 is warning-severity: under the default posture (ADR-112) it is
        // recorded — the score drops — but it does not block the gate.
        assert!(result.passed, "AP-003 (warning) must not block by default");
        assert!(
            result.score < 100.0,
            "AP-003 finding should lower the score, got: {}",
            result.score
        );
    }

    #[test]
    fn antipattern_fail_on_warnings_blocks_warning_rule() {
        // ADR-112 opt-in: the same warning-severity finding (AP-003) that
        // passes by default must block once fail-on-warnings is set.
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("warn.ts"), "const value: any = source;\n").unwrap();

        let default = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
            false,
        );
        assert!(default.passed, "warning must not block by default");

        let strict = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
            true,
        );
        assert!(
            !strict.passed,
            "warning must block under fail-on-warnings; got: {}",
            strict.message
        );
        assert!(strict.message.contains("AP-003"));
    }

    #[test]
    fn antipattern_wc003_error_blocks_by_default() {
        // ADR-112: JWT `none` (WC-003) was promoted to error severity, so it
        // blocks the gate even without --fail-on-warnings.
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("auth.ts"),
            "export const options = { algorithm: 'none' };\n",
        )
        .unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
            false,
        );

        assert!(
            !result.passed,
            "WC-003 (error) must block by default; got: {}",
            result.message
        );
        assert!(result.message.contains("WC-003"), "got: {}", result.message);
    }

    #[test]
    fn antipattern_fails_loudly_on_malformed_exclude_config() {
        // CIB-199: a malformed project config must fail the check with a visible
        // message, not silently disable `antipattern.exclude`.
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvilrc"), "{").unwrap();
        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
            false,
        );
        assert!(!result.passed, "malformed config must fail the check");
        assert!(
            result.message.contains("antipattern.exclude"),
            "message must name the failing config surface; got: {}",
            result.message
        );
    }

    // ── LANGTS-006 / #1801: dynamic-execution rules ──────────────────

    #[test]
    fn antipattern_check_detects_dynamic_eval() {
        // AP-008 — eval(<identifier>) must fire under default profile.
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("smelly.ts");
        std::fs::write(
            &file,
            "export function unsafe(input: any): unknown {\n    return eval(input);\n}\n",
        )
        .unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
            false,
        );

        assert!(!result.passed, "AP-008 must trip on eval(<identifier>)");
        assert!(
            result.message.contains("AP-008"),
            "expected AP-008 in message, got: {}",
            result.message
        );
    }

    #[test]
    fn antipattern_check_detects_new_function() {
        // AP-009 — `new Function(...)` always fires (the dynamic-string
        // ergonomics are never worth the audit cost).
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("smelly.ts");
        std::fs::write(
            &file,
            "export const compiled = new Function('a', 'b', 'return a + b');\n",
        )
        .unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
            false,
        );

        assert!(!result.passed, "AP-009 must trip on `new Function(...)`");
        assert!(
            result.message.contains("AP-009"),
            "expected AP-009 in message, got: {}",
            result.message
        );
    }

    #[test]
    fn antipattern_check_detects_template_literal_eval() {
        // Council follow-up: `eval(`${userInput}`)` is the most ergonomic
        // way to build a dynamic-string eval call in modern TS; the
        // regex extends to backtick so the template-literal shape
        // does not slip through.
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("smelly.ts");
        std::fs::write(
            &file,
            "export function unsafe(input: string): unknown {\n    return eval(`run(${input})`);\n}\n",
        )
        .unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
            false,
        );

        assert!(
            !result.passed,
            "AP-008 must trip on eval(`...`) template-literal arg; got: {}",
            result.message
        );
        assert!(result.message.contains("AP-008"));
    }

    #[test]
    fn antipattern_check_skips_static_eval() {
        // AP-008 is intentionally narrow: a literal-string `eval("1+1")`
        // is rare but benign, and false positives here would erode
        // trust in the rule. The detector requires an identifier char
        // (A-Za-z_$) immediately after the opening paren — a quote
        // skips the rule.
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("benign.ts");
        std::fs::write(
            &file,
            "export const two = eval(\"1 + 1\");\nexport function evalQueue() {}\n",
        )
        .unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
            false,
        );

        assert!(
            result.passed,
            "AP-008 must not fire on static-string eval or on `evalQueue` word boundary; got: {}",
            result.message
        );
        assert!(!result.message.contains("AP-008"));
    }

    // ── LANGTS-004 / TS-G5: Zod-creep rules ──────────────────────────

    #[test]
    fn antipattern_check_detects_zod_creep() {
        // AP-015 — the on-by-default Zod escape hatches (`z.any()` and a
        // Zod `.passthrough()`) must trip the type-system-evasion gate the
        // same way `: any` (AP-003) does. Each fixture runs on its own so a
        // regex that only caught one alternative would fail here.
        for snippet in [
            "export const S = z.any();\n",
            "export const S = z.object({ id: z.string() }).passthrough();\n",
        ] {
            let tmp = tempfile::TempDir::new().unwrap();
            std::fs::write(tmp.path().join("schema.ts"), snippet).unwrap();

            let result = run_check_antipattern(
                "antipattern-scan",
                tmp.path(),
                &std::collections::HashSet::new(),
                false,
            );

            // AP-015 is warning-severity: recorded (score < 100) but
            // non-blocking by default (ADR-112).
            assert!(
                result.passed,
                "AP-015 (warning) must not block by default for `{snippet}`"
            );
            assert!(
                result.score < 100.0,
                "AP-015 finding should lower the score for `{snippet}`, got: {}",
                result.score
            );
        }
    }

    #[test]
    fn antipattern_check_skips_zod_unknown_by_default() {
        // `z.unknown()` is the opt-in rule AP-016 (idiomatic as a typed-record
        // leaf), so the default gate must stay quiet on it.
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("schema.ts"),
            "export const Meta = z.record(z.string(), z.unknown());\n",
        )
        .unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
            false,
        );

        assert!(
            result.passed,
            "z.unknown() (AP-016 opt-in) must not trip the default gate; got: {}",
            result.message
        );
        assert!(!result.message.contains("AP-016"));
    }

    #[test]
    fn antipattern_check_skips_typed_zod_schema() {
        // A fully-typed Zod schema must NOT fire AP-015 — the rule targets
        // the escape hatches, not Zod itself (Zod is the recommended fix
        // for `: any`, per the type-system-evasion family doc).
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("schema.ts"),
            "export const User = z.object({ id: z.string(), age: z.number() });\n",
        )
        .unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
            false,
        );

        assert!(
            result.passed,
            "typed Zod schema must not trip the gate; got: {}",
            result.message
        );
        assert!(!result.message.contains("AP-015"));
    }

    #[test]
    fn antipattern_check_skips_when_no_supported_files_exist() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("notes.md"), "hello\n").unwrap();

        let result = run_check_antipattern(
            "antipattern-scan",
            tmp.path(),
            &std::collections::HashSet::new(),
            false,
        );

        assert!(result.passed);
        assert!(result.message.contains("Skipping"));
    }

    // ── Validate check names ──────────────────────────────────────────

    #[test]
    fn validate_check_names_accepts_known() {
        let names: std::collections::HashSet<&str> = [
            "lint",
            "secret-detection",
            "import-boundaries",
            "antipattern-scan",
        ]
        .into_iter()
        .collect();
        assert!(validate_check_names(&names).is_ok());
    }

    #[test]
    fn normalize_gate_check_set_accepts_stable_ids_and_aliases() {
        let names: std::collections::HashSet<&str> =
            ["ANV-CORE-001", "architecture"].into_iter().collect();

        let normalised = normalize_gate_check_set(&names).unwrap();

        assert!(normalised.contains("secret"));
        assert!(normalised.contains("architecture"));
    }

    #[test]
    fn normalize_gate_check_set_resolves_check_by_stable_id_only() {
        // OPSUP-002: a check that is absent from the legacy name map (no
        // alias) but present by `ANV-*` ID resolves and is skippable — so a
        // newly shipped check is skippable by ID without a binary downgrade.
        // ANV-CORE-003 (antipattern-scan) has no aliases.
        let names: std::collections::HashSet<&str> = ["ANV-CORE-003"].into_iter().collect();

        let normalised = normalize_gate_check_set(&names).unwrap();

        assert!(
            normalised.contains("antipattern-scan"),
            "stable-ID-only check must resolve to its internal name, got: {normalised:?}"
        );
    }

    #[test]
    fn normalize_gate_check_set_unknown_errors_with_suggestion() {
        // OPSUP-002: an unknown identifier produces a deterministic error
        // that names the closest registered ID rather than a flat dump.
        let names: std::collections::HashSet<&str> = ["lnt"].into_iter().collect();

        let err = normalize_gate_check_set(&names).unwrap_err();
        let msg = err.to_string();

        assert!(msg.contains("lnt"), "error must name the bad input: {msg}");
        // Assert the full suggestion phrase, not just "lint" — the available
        // list also contains "lint", so a bare substring check would pass even
        // if the did-you-mean suggestion regressed.
        assert!(
            msg.contains("did you mean 'lint'?"),
            "error must carry the closest-ID suggestion: {msg}"
        );
    }

    #[test]
    fn validate_check_names_rejects_unknown() {
        let names: std::collections::HashSet<&str> = ["lint", "bogus"].into_iter().collect();
        let err = validate_check_names(&names).unwrap_err();
        assert!(err.to_string().contains("bogus"));
    }

    #[test]
    fn validate_check_names_empty_is_ok() {
        let names: std::collections::HashSet<&str> = std::collections::HashSet::new();
        assert!(validate_check_names(&names).is_ok());
    }

    #[test]
    fn anvilrc_unknown_warning_includes_suggestion_and_known_subset_policy() {
        let msg = format_anvilrc_unknown_checks_warning(&["lnt"]);
        assert!(
            msg.contains(".anvilrc#checks contains unknown check(s): 'lnt'"),
            "warning must name bad config entry: {msg}"
        );
        assert!(
            msg.contains("did you mean 'lint'?"),
            "warning must carry OPSUP-002 suggestion text: {msg}"
        );
        assert!(
            msg.contains("Known checks will still run"),
            "warning must document CIB-089 warn-and-continue policy: {msg}"
        );
    }

    #[test]
    fn resolve_anvilrc_check_filter_warns_and_continues_with_known_subset() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            r#"{"checks": ["secret-detection", "lnt"]}"#,
        )
        .unwrap();

        let filter = resolve_anvilrc_check_filter(tmp.path(), None)
            .expect("config with at least one known check should continue")
            .expect("filter");

        assert_eq!(filter.len(), 1);
        assert!(filter.contains("secret"));
    }

    #[test]
    fn resolve_anvilrc_check_filter_errors_when_no_known_checks_remain() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvilrc"), r#"{"checks": ["lnt"]}"#).unwrap();

        let err = resolve_anvilrc_check_filter(tmp.path(), None).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("contains no valid gate checks"),
            "all-unknown config should still fail: {msg}"
        );
    }

    // ── GateResult serialisation ──────────────────────────────────────

    #[test]
    fn gate_result_serialises_to_json() {
        let overall = true;
        let checks = vec![CheckResult {
            name: "secret-detection".to_string(),
            passed: true,
            score: 100.0,
            message: "clean".to_string(),
            requires_config: false,
        }];
        let notifications = notifications_for_gate_result(&checks, overall);
        let result = GateResult {
            overall,
            score: 100.0,
            checks,
            notifications,
            duration_ms: 42,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["overall"], true);
        assert_eq!(parsed["checks"][0]["name"], "secret-detection");
        assert_eq!(parsed["duration_ms"], 42);

        let notifications = parsed["notifications"].as_array().expect("notifications");
        assert_eq!(notifications.len(), 2, "per-check + overall notifications");

        let per_check = &notifications[0];
        assert_eq!(per_check["class"], "info");
        assert_eq!(per_check["priority"], "low");
        assert_eq!(per_check["title"], "Gate check: secret-detection");
        assert_eq!(per_check["message"], "clean");
        assert_eq!(per_check["context"]["source"], "gate");

        let overall_notif = &notifications[1];
        assert_eq!(overall_notif["class"], "info");
        assert_eq!(overall_notif["priority"], "normal");
        assert_eq!(overall_notif["title"], "Gate result");
        assert_eq!(overall_notif["message"], "All quality gates passed");
        assert_eq!(overall_notif["context"]["source"], "gate");
    }

    // ── CIB-011 / #1803: strict-config produces config-gap, not FAIL ──

    fn strict_ai_ctx(root: &Path) -> GateContext {
        GateContext {
            workspace_root: root.to_path_buf(),
            profile: Some(AiGuardrailProfile::NAME.to_string()),
            plan_files: std::collections::HashSet::new(),
            plan_path: None,
            walked_files: Vec::new(),
            strict_config: true,
            fail_on_warnings: false,
        }
    }

    #[test]
    fn strict_config_missing_arch_becomes_config_gap_not_fail() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = strict_ai_ctx(dir.path());
        let result = run_single_check("architecture", &ctx);
        assert!(
            result.passed,
            "missing-config must NOT flip to fail under strict mode; got message: {}",
            result.message
        );
        assert!(
            result.requires_config,
            "missing-config must set requires_config=true; got: passed={}, message={}",
            result.passed, result.message
        );
        assert!(
            result.message.contains("next:"),
            "config-gap message must carry an actionable `next:` hint; got: {}",
            result.message
        );
        assert!(
            !result.message.starts_with("Strict mode"),
            "pre-CIB-011 FAIL prefix must be removed; got: {}",
            result.message
        );
    }

    #[test]
    fn strict_config_missing_policy_becomes_config_gap_not_fail() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = strict_ai_ctx(dir.path());
        let result = run_single_check("policy", &ctx);
        assert!(
            result.passed,
            "missing policy bundle must not fail under strict mode"
        );
        assert!(result.requires_config);
        assert!(result.message.contains("next:"));
        assert!(result.message.contains(".anvil/policies"));
    }

    #[test]
    fn strict_config_missing_command_safety_becomes_config_gap_not_fail() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = strict_ai_ctx(dir.path());
        let result = run_single_check("command-safety", &ctx);
        assert!(
            result.passed,
            "no plan supplied must not fail under strict mode"
        );
        assert!(result.requires_config);
        assert!(result.message.contains("next:"));
        assert!(result.message.contains("--plan"));
    }

    #[test]
    fn non_strict_skip_does_not_set_requires_config() {
        // Outside strict mode, the same architecture-missing scenario
        // returns a soft skip with no config-gap marker. Regression
        // pin against accidentally treating every soft skip as a
        // config-gap.
        let dir = tempfile::tempdir().unwrap();
        let ctx = GateContext {
            workspace_root: dir.path().to_path_buf(),
            profile: None,
            plan_files: std::collections::HashSet::new(),
            plan_path: None,
            walked_files: Vec::new(),
            strict_config: false,
            fail_on_warnings: false,
        };
        let result = run_single_check("architecture", &ctx);
        assert!(result.passed);
        assert!(
            !result.requires_config,
            "non-strict skip must not mark config-gap"
        );
        assert!(
            !result.message.contains("next:"),
            "non-strict path stays clean"
        );
    }

    #[test]
    fn aggregate_excludes_config_gap_from_score_denominator() {
        let checks = vec![
            CheckResult {
                name: "antipattern-scan".into(),
                passed: true,
                score: 100.0,
                message: "clean".into(),
                requires_config: false,
            },
            CheckResult {
                name: "secret-detection".into(),
                passed: false,
                score: 0.0,
                message: "secret found".into(),
                requires_config: false,
            },
            CheckResult {
                name: "import-boundaries".into(),
                passed: true,
                score: 100.0,
                message: "...Skipping. next: ...".into(),
                requires_config: true,
            },
            CheckResult {
                name: "policy".into(),
                passed: true,
                score: 100.0,
                message: "...Skipping. next: ...".into(),
                requires_config: true,
            },
            CheckResult {
                name: "command-safety".into(),
                passed: true,
                score: 100.0,
                message: "...No commands... next: ...".into(),
                requires_config: true,
            },
        ];
        let agg = aggregate_gate_outcome(&checks);
        assert_eq!(agg.available_total, 2, "3 config-gap checks excluded");
        assert_eq!(agg.passed_count, 1);
        assert_eq!(agg.config_gaps, 3);
        assert!((agg.score - 50.0).abs() < f64::EPSILON, "1/2 = 50%");
        assert!(!agg.overall);
    }

    #[test]
    fn aggregate_overall_true_when_all_available_pass() {
        let checks = vec![
            CheckResult {
                name: "antipattern-scan".into(),
                passed: true,
                score: 100.0,
                message: "clean".into(),
                requires_config: false,
            },
            CheckResult {
                name: "secret-detection".into(),
                passed: true,
                score: 100.0,
                message: "clean".into(),
                requires_config: false,
            },
            CheckResult {
                name: "import-boundaries".into(),
                passed: true,
                score: 100.0,
                message: "...next: ...".into(),
                requires_config: true,
            },
        ];
        let agg = aggregate_gate_outcome(&checks);
        assert!(agg.overall);
        assert!((agg.score - 100.0).abs() < f64::EPSILON);
        assert_eq!(agg.available_total, 2);
        assert_eq!(agg.config_gaps, 1);
    }

    #[test]
    fn aggregate_score_100_when_only_config_gaps() {
        // Edge case: every check is a config-gap. Nothing actually ran,
        // so nothing failed; the gate is vacuously green and the
        // render layer surfaces the config gaps so the user is not
        // misled into thinking "100%" means "fully covered".
        let checks = vec![CheckResult {
            name: "import-boundaries".into(),
            passed: true,
            score: 100.0,
            message: "...next: ...".into(),
            requires_config: true,
        }];
        let agg = aggregate_gate_outcome(&checks);
        assert!(agg.overall);
        assert!((agg.score - 100.0).abs() < f64::EPSILON);
        assert_eq!(agg.available_total, 0);
        assert_eq!(agg.config_gaps, 1);
    }

    #[test]
    fn ai_envelope_excludes_config_gaps_from_diagnostics_and_counts_them() {
        let checks = vec![
            CheckResult {
                name: "secret-detection".into(),
                passed: false,
                score: 0.0,
                message: "secret found".into(),
                requires_config: false,
            },
            CheckResult {
                name: "import-boundaries".into(),
                passed: true,
                score: 100.0,
                message: "...next: ...".into(),
                requires_config: true,
            },
        ];
        let result = GateResult {
            overall: false,
            score: 0.0,
            checks,
            notifications: vec![],
            duration_ms: 1,
        };
        let envelope = build_ai_gate_result_envelope(&result);
        assert_eq!(
            envelope.summary.total, 1,
            "only the real failure surfaces as a diagnostic"
        );
        assert_eq!(
            envelope.summary.config_gaps, 1,
            "config-gap is counted separately"
        );
        assert_eq!(envelope.diagnostics.len(), 1);
    }

    #[test]
    fn config_gap_next_hint_covers_strict_mode_check_names() {
        // All three checks that the strict-mode flip touches must have
        // a real hint (not the generic fallback).
        let generic = config_gap_next_hint("__no_such_check__");
        for name in ["architecture", "policy", "command-safety"] {
            let hint = config_gap_next_hint(name);
            assert_ne!(hint, generic, "{name} must have a dedicated hint");
            assert!(!hint.is_empty());
        }
    }

    #[test]
    fn config_gap_check_keeps_passed_true_for_fail_fast_continuity() {
        // The fail_fast path in `run_checks` derives `failed =
        // !result.passed` to decide whether to short-circuit. Config-gap
        // checks must keep `passed: true` so they do NOT trip fail_fast
        // (otherwise a config-gap in the first check would silently drop
        // every subsequent real failure). Pin the invariant at its
        // source — run_single_check under strict mode for a missing
        // architecture config.
        let dir = tempfile::tempdir().unwrap();
        let ctx = strict_ai_ctx(dir.path());
        let result = run_single_check("architecture", &ctx);
        assert!(result.requires_config, "test pre-condition");
        assert!(
            result.passed,
            "config-gap must keep passed=true so fail_fast does not trip on it"
        );
        // Confirm the derived `failed` flag matches the expectation.
        let failed = !result.passed;
        assert!(!failed, "config-gap must derive failed=false for fail_fast");
    }

    // ── run_single_check unknown ─────────────────────────────────────

    #[test]
    fn unknown_check_fails() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = GateContext {
            workspace_root: dir.path().to_path_buf(),
            profile: None,
            plan_files: std::collections::HashSet::new(),
            plan_path: None,
            walked_files: Vec::new(),
            strict_config: false,
            fail_on_warnings: false,
        };
        let result = run_single_check("nonexistent", &ctx);
        assert!(!result.passed);
        assert!(result.message.contains("Unknown check"));
    }

    // ── Plan-scoped policy input filtering ────────────────────────────

    #[test]
    fn build_policy_input_filters_by_plan_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("included.ts"), "export const x = 1;").unwrap();
        std::fs::write(src.join("excluded.ts"), "export const y = 2;").unwrap();

        let mut plan_files = std::collections::HashSet::new();
        plan_files.insert("src/included.ts".to_string());

        let input = build_policy_input(tmp.path(), None, None, &plan_files, None);
        let files: Vec<&str> = input["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        assert!(files.contains(&"src/included.ts"));
        assert!(!files.contains(&"src/excluded.ts"));
    }

    // ── Extract plan files multi-line ─────────────────────────────────

    #[test]
    fn extract_plan_files_multi_line_continuation() {
        let tmp = tempfile::TempDir::new().unwrap();
        let plan = tmp.path().join("test.aps.md");
        std::fs::write(
            &plan,
            "- **Files:** `src/a.ts`,\n  `src/b.ts`, `src/c.ts`\n- **Status:** Done\n",
        )
        .unwrap();

        let files = extract_plan_files(&plan);
        assert!(files.contains("src/a.ts"));
        assert!(files.contains("src/b.ts"));
        assert!(files.contains("src/c.ts"));
    }

    // ── Coverage edge cases ──────────────────────────────────────────

    #[test]
    fn coverage_lcov_empty_report_skips() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cov_dir = tmp.path().join("coverage");
        std::fs::create_dir_all(&cov_dir).unwrap();
        std::fs::write(
            cov_dir.join("lcov.info"),
            "SF:src/main.rs\nLF:0\nLH:0\nend_of_record\n",
        )
        .unwrap();
        let result = run_check_coverage(tmp.path(), 80.0);
        assert!(result.passed);
        assert!(result.message.contains("empty"));
    }

    #[test]
    fn coverage_cobertura_unparseable_rate() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cov_dir = tmp.path().join("coverage");
        std::fs::create_dir_all(&cov_dir).unwrap();
        std::fs::write(
            cov_dir.join("cobertura.xml"),
            r#"<?xml version="1.0"?><coverage></coverage>"#,
        )
        .unwrap();
        let result = run_check_coverage(tmp.path(), 80.0);
        assert!(!result.passed);
        assert!(result.message.contains("Failed to parse"));
    }

    // ── .anvilrc#checks filter (#1016) ────────────────────────────────

    #[test]
    fn read_anvilrc_checks_none_when_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(read_anvilrc_checks(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn read_anvilrc_antipattern_excludes_parses_globs() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            r#"{"antipattern": {"exclude": ["src/generated/**", "*.pb.ts"]}}"#,
        )
        .unwrap();
        let globs = read_anvilrc_antipattern_excludes(tmp.path()).unwrap();
        assert_eq!(
            globs,
            vec!["src/generated/**".to_string(), "*.pb.ts".to_string()]
        );
    }

    #[test]
    fn read_anvilrc_antipattern_excludes_empty_when_absent_or_unset() {
        let tmp = tempfile::TempDir::new().unwrap();
        // No config file at all.
        assert!(
            read_anvilrc_antipattern_excludes(tmp.path())
                .unwrap()
                .is_empty()
        );
        // Config present but no `antipattern.exclude` key.
        std::fs::write(tmp.path().join(".anvilrc"), r#"{"checks": ["secret"]}"#).unwrap();
        assert!(
            read_anvilrc_antipattern_excludes(tmp.path())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn read_anvilrc_checks_none_for_empty_list() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvilrc"), r#"{"checks": []}"#).unwrap();
        assert!(read_anvilrc_checks(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn read_anvilrc_checks_parses_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            r#"{"checks": ["secret", "architecture"]}"#,
        )
        .unwrap();
        let checks = read_anvilrc_checks(tmp.path()).unwrap().unwrap();
        assert_eq!(checks.len(), 2);
        assert!(checks.contains("secret-detection"));
        assert!(checks.contains("import-boundaries"));
    }

    #[test]
    fn read_anvilrc_checks_parses_stable_ids() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            r#"{"checks": ["ANV-CORE-001", "ANV-CORE-002"]}"#,
        )
        .unwrap();

        let checks = read_anvilrc_checks(tmp.path()).unwrap().unwrap();

        assert_eq!(checks.len(), 2);
        assert!(checks.contains("secret-detection"));
        assert!(checks.contains("import-boundaries"));
    }

    #[test]
    fn read_anvilrc_checks_parses_yaml() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            "schemaVersion: \"1.0.0\"\nchecks:\n  - \"secret\"\n  - \"architecture\"\n",
        )
        .unwrap();
        let checks = read_anvilrc_checks(tmp.path()).unwrap().unwrap();
        assert!(checks.contains("secret-detection"));
        assert!(checks.contains("import-boundaries"));
    }

    #[test]
    fn read_anvilrc_checks_parses_toml_inline() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            "schema_version = \"1.0.0\"\nchecks = [\"secret\", \"policy\"]\n",
        )
        .unwrap();
        let checks = read_anvilrc_checks(tmp.path()).unwrap().unwrap();
        assert!(checks.contains("secret-detection"));
        assert!(checks.contains("policy"));
    }

    #[test]
    fn read_anvilrc_checks_errors_on_unparseable_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".anvilrc"), "checks: [\n").unwrap();
        let err = read_anvilrc_checks(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("failed to parse"));
    }

    // MLP2-040 — `.anvil.<ext>` discovery via MLP-011 takes precedence over
    // the legacy `.anvilrc`. The fallback only triggers when no
    // `.anvil.<ext>` is present.

    #[test]
    fn read_anvilrc_checks_prefers_anvil_yaml_when_present() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.yaml"),
            "checks: [\"secret-detection\"]\n",
        )
        .unwrap();
        // Legacy `.anvilrc` exists too with a different value to prove
        // precedence — discover should win.
        std::fs::write(
            tmp.path().join(".anvilrc"),
            r#"{"checks":["import-boundaries"]}"#,
        )
        .unwrap();

        let checks = read_anvilrc_checks(tmp.path()).unwrap().unwrap();
        assert!(checks.contains("secret-detection"));
        assert!(!checks.contains("import-boundaries"));
    }

    #[test]
    fn read_anvilrc_checks_reads_anvil_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.json"),
            r#"{"checks":["secret-detection","import-boundaries"]}"#,
        )
        .unwrap();
        let checks = read_anvilrc_checks(tmp.path()).unwrap().unwrap();
        assert_eq!(checks.len(), 2);
        assert!(checks.contains("secret-detection"));
    }

    #[test]
    fn read_anvilrc_checks_reads_anvil_toml() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvil.toml"),
            "checks = [\"secret-detection\"]\n",
        )
        .unwrap();
        let checks = read_anvilrc_checks(tmp.path()).unwrap().unwrap();
        assert!(checks.contains("secret-detection"));
    }

    #[test]
    fn read_anvilrc_checks_falls_back_to_anvilrc_when_no_anvil_ext() {
        // Sanity guard against accidentally inverting the precedence: when
        // there is no `.anvil.<ext>`, the legacy reader must still pick
        // up `.anvilrc`. This test catches a regression where the
        // fallback was lost entirely.
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join(".anvilrc"),
            "checks: [\"secret-detection\"]\n",
        )
        .unwrap();
        let checks = read_anvilrc_checks(tmp.path()).unwrap().unwrap();
        assert!(checks.contains("secret-detection"));
    }

    // ── SARIF adapter (SARIFOUT-005) ────────────────────────────────

    fn check_result(name: &str, passed: bool, requires_config: bool, message: &str) -> CheckResult {
        CheckResult {
            name: name.to_string(),
            passed,
            score: if passed { 100.0 } else { 0.0 },
            message: message.to_string(),
            requires_config,
        }
    }

    #[test]
    fn gate_sarif_maps_failed_and_config_gap_checks_only() {
        let result = GateResult {
            overall: false,
            score: 50.0,
            checks: vec![
                check_result("secret-detection", false, false, "2 hardcoded secrets"),
                check_result("policy", false, true, "needs .anvil/policy"),
                check_result("antipattern-scan", true, false, ""),
            ],
            notifications: Vec::new(),
            duration_ms: 1,
        };
        let value = serde_json::to_value(build_gate_sarif(&result)).expect("serialise");

        let schema: serde_json::Value =
            serde_json::from_str(anvil_sarif::SARIF_SCHEMA_JSON).expect("schema json");
        let validator = jsonschema::validator_for(&schema).expect("compile schema");
        let errors: Vec<String> = validator
            .iter_errors(&value)
            .map(|e| format!("{} at {}", e, e.instance_path()))
            .collect();
        assert!(errors.is_empty(), "schema errors:\n{}", errors.join("\n"));

        let results = value["runs"][0]["results"].as_array().expect("results");
        // Passing, fully-configured checks are not findings.
        assert_eq!(results.len(), 2, "failed + config-gap only");

        let failed = results
            .iter()
            .find(|r| r["ruleId"] == "secret-detection")
            .unwrap();
        assert_eq!(failed["level"], "error");
        // Repo-level aggregate: no physical location.
        assert!(failed.get("locations").is_none());

        let config_gap = results.iter().find(|r| r["ruleId"] == "policy").unwrap();
        assert_eq!(
            config_gap["level"], "note",
            "config-gap does not inflate failures"
        );

        assert!(
            !results.iter().any(|r| r["ruleId"] == "antipattern-scan"),
            "passing check omitted"
        );
    }
}
