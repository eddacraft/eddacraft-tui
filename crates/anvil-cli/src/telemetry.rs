//! Fleet telemetry consent state, first-run disclosure, and anonymous
//! install identity, canonical payload, and detached fleet-beacon worker.
//! Every send path consumes [`send_gate`] / [`send_allowed`] immediately
//! before network egress.
//!
//! The consent posture is **disclosed opt-out** (decided in
//! `plans/decisions/107-fleet-telemetry-consent-posture.md`):
//!
//! - **Consent state** lives at `telemetry.json` in the user-scoped
//!   state directory (the same `credentials_dir` convention as the
//!   usage salt, so it re-roots under a gated `ANVIL_HOME`): `enabled`
//!   plus `notice_shown`.
//! - **Hard offs**, all honoured before any send: `anvil telemetry off`
//!   (persisted), `ANVIL_TELEMETRY=off`, and `DO_NOT_TRACK=1`.
//! - **Disclosure strictly precedes the first beacon**: `notice_shown`
//!   is persisted only at the moment the notice is actually shown on a
//!   terminal, and the send gate refuses while it is false — so gated
//!   `ANVIL_HOME` environments and non-TTY first runs that never showed
//!   the notice can never beacon.
//! - **Identity** is a random UUID v4 install id minted on first use,
//!   stored beside the per-deployment salt in the credentials dir with
//!   `0600` perms, derived from nothing (no hardware, no user identity).
//!   `anvil telemetry reset-id` rotates it. The salted usage principal
//!   never appears here: this module exposes no accessor for it and the
//!   identity file contains only the UUID.
//!
//! Operator-config rule: a corrupt consent file is an error, never a
//! silent default. The send gate fails safe (blocked) and surfaces a
//! warning; only an explicit `anvil telemetry on|off` rewrites it.

use std::env;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::credentials;

/// Consent-state filename under the user-scoped state directory.
pub const CONSENT_FILE: &str = "telemetry.json";

/// Anonymous install-id filename — a sibling of `usage.salt` under the
/// same credentials dir.
pub const INSTALL_ID_FILE: &str = "telemetry.install-id";

/// Env hard-off: `ANVIL_TELEMETRY=off` disables the beacon regardless of
/// persisted consent.
pub const TELEMETRY_ENV: &str = "ANVIL_TELEMETRY";

/// Cross-tool consent convention — a superset off for every collection
/// surface, honoured before any send.
pub const DO_NOT_TRACK_ENV: &str = "DO_NOT_TRACK";

/// Internal marker for the detached beacon worker process. It is intercepted
/// before CLI parsing, so it never records a telemetry-management invocation.
pub const BEACON_WORKER_ENV: &str = "ANVIL_INTERNAL_TELEMETRY_BEACON";

const BEACON_HTTP_TIMEOUT: Duration = Duration::from_secs(2);
const BEACON_CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
const BEACON_RESERVATION_LEASE: chrono::Duration = chrono::Duration::minutes(5);
const MAX_FEATURES_PER_BEACON: usize = 128;

/// Current consent-state schema version.
pub const CONSENT_SCHEMA_VERSION: u32 = 1;

/// Wire-format version accepted by the `/api/v1/telemetry` ingest route.
pub const BEACON_SCHEMA_VERSION: u32 = 1;

/// One allowlisted feature-key usage count since the last successful beacon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeatureUsage {
    pub key: String,
    pub count: u64,
}

/// Canonical ADR-107 beacon body. Both the sender and transparency command
/// serialise this type so the audited payload cannot drift from the wire body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BeaconPayload {
    pub schema_version: u32,
    pub install_id: String,
    pub version: String,
    pub install_method: String,
    pub platform: String,
    pub channel: String,
    pub flag_snapshot_version: String,
    pub features: Vec<FeatureUsage>,
}

impl BeaconPayload {
    #[must_use]
    pub fn new(
        install_id: Uuid,
        version: &str,
        install_method: &str,
        platform: &str,
        channel: &str,
        flag_snapshot_version: &str,
        features: Vec<FeatureUsage>,
    ) -> Self {
        Self {
            schema_version: BEACON_SCHEMA_VERSION,
            install_id: install_id.to_string(),
            version: version.to_owned(),
            install_method: install_method.to_owned(),
            platform: platform.to_owned(),
            channel: channel.to_owned(),
            flag_snapshot_version: flag_snapshot_version.to_owned(),
            features,
        }
    }
}

/// Aggregate feature resolutions recorded by the local Kindling usage pipe
/// after the last successful beacon. The sorted output makes the wire body and
/// transparency rendering deterministic.
#[must_use]
pub fn feature_usage_since(
    rows: &[crate::usage_views::UsageRow],
    last_success: Option<chrono::DateTime<chrono::Utc>>,
) -> Vec<FeatureUsage> {
    let mut counts = std::collections::BTreeMap::<String, u64>::new();
    for row in rows {
        if let Some(cutoff) = last_success {
            let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(&row.timestamp) else {
                continue;
            };
            if timestamp.with_timezone(&chrono::Utc) <= cutoff {
                continue;
            }
        }
        for flag in &row.flag_set {
            if !anvil_kernel_types::feature_flags_catalogue::all::KEYS.contains(&flag.key.as_str())
            {
                continue;
            }
            let count = counts.entry(flag.key.clone()).or_default();
            *count = count.saturating_add(1);
        }
    }
    counts
        .into_iter()
        .take(MAX_FEATURES_PER_BEACON)
        .map(|(key, count)| FeatureUsage { key, count })
        .collect()
}

const BEACON_STATE_FILE: &str = "telemetry.beacon-state.json";
const BEACON_RESERVATION_FILE: &str = "telemetry.beacon-reservation.json";
const BEACON_INTERVAL: chrono::Duration = chrono::Duration::hours(24);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
struct BeaconState {
    last_success_install_id: Option<String>,
    last_success_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeaconReservation {
    token: String,
    install_id: String,
    reserved_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReserveError {
    TooRecent,
    Busy,
    StateUnreadable,
}

fn beacon_state_path(state_dir: &Path) -> PathBuf {
    state_dir.join(BEACON_STATE_FILE)
}

fn beacon_reservation_path(state_dir: &Path) -> PathBuf {
    state_dir.join(BEACON_RESERVATION_FILE)
}

fn load_beacon_state_in(state_dir: &Path) -> Result<BeaconState> {
    match fs::read_to_string(beacon_state_path(state_dir)) {
        Ok(raw) => serde_json::from_str(&raw).context("parse beacon delivery state"),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(BeaconState::default()),
        Err(err) => Err(err).context("read beacon delivery state"),
    }
}

fn parse_utc(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|time| time.with_timezone(&chrono::Utc))
}

fn write_reservation_exclusive(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)
}

fn remove_stale_reservation(path: &Path, now: chrono::DateTime<chrono::Utc>) -> io::Result<bool> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(true),
        Err(err) => return Err(err),
    };
    let reserved_at = serde_json::from_str::<BeaconReservation>(&raw)
        .ok()
        .and_then(|reservation| parse_utc(&reservation.reserved_at))
        .or_else(|| {
            fs::metadata(path)
                .ok()?
                .modified()
                .ok()
                .map(chrono::DateTime::<chrono::Utc>::from)
        });
    if reserved_at
        .is_none_or(|reserved| now.signed_duration_since(reserved) < BEACON_RESERVATION_LEASE)
    {
        return Ok(false);
    }
    if !fs::read_to_string(path).is_ok_and(|current| current == raw) {
        return Ok(false);
    }
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(true),
        Err(err) => Err(err),
    }
}

fn reservation_is_fresh(path: &Path, now: chrono::DateTime<chrono::Utc>) -> io::Result<bool> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err),
    };
    let reserved_at = serde_json::from_str::<BeaconReservation>(&raw)
        .ok()
        .and_then(|reservation| parse_utc(&reservation.reserved_at))
        .or_else(|| {
            fs::metadata(path)
                .ok()?
                .modified()
                .ok()
                .map(chrono::DateTime::<chrono::Utc>::from)
        });
    Ok(reserved_at
        .is_none_or(|reserved| now.signed_duration_since(reserved) < BEACON_RESERVATION_LEASE))
}

pub fn reserve_beacon_in(
    state_dir: &Path,
    install_id: Uuid,
    now: chrono::DateTime<chrono::Utc>,
) -> std::result::Result<BeaconReservation, ReserveError> {
    let state = load_beacon_state_in(state_dir).map_err(|_| ReserveError::StateUnreadable)?;
    if state.last_success_install_id.as_deref() == Some(&install_id.to_string())
        && state
            .last_success_at
            .as_deref()
            .and_then(parse_utc)
            .is_some_and(|last| now.signed_duration_since(last) < BEACON_INTERVAL)
    {
        return Err(ReserveError::TooRecent);
    }

    crate::usage::create_private_dir(state_dir).map_err(|_| ReserveError::StateUnreadable)?;
    let path = beacon_reservation_path(state_dir);
    let reservation = BeaconReservation {
        token: Uuid::new_v4().to_string(),
        install_id: install_id.to_string(),
        reserved_at: now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    };
    let bytes = serde_json::to_vec(&reservation).map_err(|_| ReserveError::StateUnreadable)?;
    match write_reservation_exclusive(&path, &bytes) {
        Ok(()) => Ok(reservation),
        Err(err)
            if err.kind() == io::ErrorKind::AlreadyExists
                && remove_stale_reservation(&path, now)
                    .map_err(|_| ReserveError::StateUnreadable)? =>
        {
            write_reservation_exclusive(&path, &bytes)
                .map_err(|err| {
                    if err.kind() == io::ErrorKind::AlreadyExists {
                        ReserveError::Busy
                    } else {
                        ReserveError::StateUnreadable
                    }
                })
                .map(|()| reservation)
        }
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Err(ReserveError::Busy),
        Err(_) => Err(ReserveError::StateUnreadable),
    }
}

fn reservation_is_current_in(state_dir: &Path, reservation: &BeaconReservation) -> bool {
    existing_install_id_in(state_dir).is_some_and(|id| id.to_string() == reservation.install_id)
        && fs::read_to_string(beacon_reservation_path(state_dir))
            .ok()
            .and_then(|raw| serde_json::from_str::<BeaconReservation>(&raw).ok())
            .as_ref()
            == Some(reservation)
}

fn invalidate_beacon_reservation_in(state_dir: &Path) -> Result<()> {
    match fs::remove_file(beacon_reservation_path(state_dir)) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).context("invalidate beacon reservation"),
    }
}

pub fn release_beacon_reservation_in(
    state_dir: &Path,
    reservation: &BeaconReservation,
) -> Result<()> {
    let path = beacon_reservation_path(state_dir);
    let current = fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<BeaconReservation>(&raw).ok());
    if current.as_ref() == Some(reservation) {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err).context("release beacon reservation"),
        }
    }
    Ok(())
}

pub fn commit_beacon_in(
    state_dir: &Path,
    reservation: &BeaconReservation,
    successful_at: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    let path = beacon_reservation_path(state_dir);
    let current: BeaconReservation = serde_json::from_str(
        &fs::read_to_string(&path).context("read beacon reservation before commit")?,
    )
    .context("parse beacon reservation before commit")?;
    anyhow::ensure!(
        current == *reservation,
        "beacon reservation changed before commit"
    );
    let state = BeaconState {
        last_success_install_id: Some(reservation.install_id.clone()),
        last_success_at: Some(successful_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
    };
    write_private_atomic(
        &beacon_state_path(state_dir),
        &serde_json::to_vec(&state).context("serialise beacon delivery state")?,
    )
    .context("write beacon delivery state")?;
    release_beacon_reservation_in(state_dir, reservation)
}

fn release_channel(version: &str) -> &'static str {
    let prerelease = version.split_once('-').map(|(_, suffix)| suffix);
    match prerelease {
        Some(value) if value.starts_with("nightly") => "nightly",
        Some(value) if value.starts_with("alpha") => "alpha",
        Some(value) if value.starts_with("beta") => "beta",
        Some(value) if value.starts_with("rc") => "rc",
        Some(_) => "prerelease",
        None => "stable",
    }
}

fn release_is_eligible(version: &str) -> bool {
    matches!(release_channel(version), "beta" | "rc" | "stable")
}

fn platform_triple() -> String {
    let arch = std::env::consts::ARCH;
    if cfg!(target_os = "macos") {
        return format!("{arch}-apple-darwin");
    }
    if cfg!(target_os = "windows") {
        let environment = if cfg!(target_env = "gnu") {
            "gnu"
        } else {
            "msvc"
        };
        return format!("{arch}-pc-windows-{environment}");
    }
    let environment = if cfg!(target_env = "musl") {
        "musl"
    } else if cfg!(target_env = "gnu") {
        "gnu"
    } else {
        "unknown"
    };
    format!("{arch}-unknown-{}-{environment}", std::env::consts::OS)
}

fn build_payload_in(state_dir: &Path, install_id: Uuid) -> Result<BeaconPayload> {
    let delivery = load_beacon_state_in(state_dir)?;
    let last_success = delivery.last_success_at.as_deref().and_then(parse_utc);
    let usage_path = state_dir.join("kindling").join("usage.ndjson");
    let rows = crate::usage_views::load_rows(&usage_path).context("read local feature usage")?;
    Ok(BeaconPayload::new(
        install_id,
        env!("CARGO_PKG_VERSION"),
        crate::commands::version::detect_install_method_cached().label(),
        &platform_triple(),
        release_channel(env!("CARGO_PKG_VERSION")),
        // No remote snapshot is installed today. `0` is an explicit,
        // ingest-valid fallback rather than an empty token the API rejects.
        "0",
        feature_usage_since(&rows, last_success),
    ))
}

/// Explain a delivery-state block that is separate from consent: either the
/// 24-hour success cap or an in-flight reservation.
pub fn next_delivery_block_reason_in(
    state_dir: &Path,
    install_id: Uuid,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Option<&'static str>> {
    let state = load_beacon_state_in(state_dir)?;
    if state.last_success_install_id.as_deref() == Some(&install_id.to_string())
        && state
            .last_success_at
            .as_deref()
            .and_then(parse_utc)
            .is_some_and(|last| now.signed_duration_since(last) < BEACON_INTERVAL)
    {
        return Ok(Some(
            "the last successful beacon was less than 24 hours ago",
        ));
    }
    if reservation_is_fresh(&beacon_reservation_path(state_dir), now)
        .context("inspect beacon reservation")?
    {
        return Ok(Some("another beacon is already in progress"));
    }
    Ok(None)
}

/// Build the exact next body for an already-evaluated send gate. Blocked
/// status is read-only; an allowed first use mints the anonymous random id so
/// the transparency surface and eventual sender can serialise the same value.
pub fn next_payload_for_gate_in(state_dir: &Path, gate: SendGate) -> Result<Option<BeaconPayload>> {
    if gate != SendGate::Allowed {
        return Ok(None);
    }
    let install_id = load_or_create_install_id_in(state_dir)?;
    if next_delivery_block_reason_in(state_dir, install_id, chrono::Utc::now())?.is_some() {
        return Ok(None);
    }
    build_payload_in(state_dir, install_id).map(Some)
}

/// Start the session beacon in an independent process and return immediately.
/// No network work or child wait occurs on the `anvil start` command path.
pub fn spawn_start_beacon() {
    if !send_allowed() {
        return;
    }
    let Ok(executable) = env::current_exe() else {
        return;
    };
    let _ = spawn_beacon_process(&executable);
}

fn spawn_beacon_process(executable: &Path) -> io::Result<()> {
    std::process::Command::new(executable)
        .env(BEACON_WORKER_ENV, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(drop)
}

/// Run the detached worker. All failures are deliberately silent and leave no
/// payload spool; a failed reservation is released for a later start.
pub fn run_beacon_worker() {
    let _ = try_run_beacon_worker();
}

fn try_run_beacon_worker() -> Result<()> {
    if send_gate() != SendGate::Allowed {
        return Ok(());
    }
    let state_dir = credentials::credentials_dir().context("resolve telemetry state directory")?;
    let install_id = load_or_create_install_id_in(&state_dir)?;
    let now = chrono::Utc::now();
    let Ok(reservation) = reserve_beacon_in(&state_dir, install_id, now) else {
        return Ok(());
    };
    let outcome = (|| -> Result<bool> {
        let payload = build_payload_in(&state_dir, install_id)?;
        let body = serde_json::to_vec(&payload).context("serialise telemetry beacon")?;

        let endpoint = format!("{}/api/v1/telemetry", crate::auth::api_url()?);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("build telemetry runtime")?;
        let client = reqwest::Client::builder()
            .timeout(BEACON_HTTP_TIMEOUT)
            .connect_timeout(BEACON_CONNECT_TIMEOUT)
            .build()
            .context("build telemetry client")?;
        // Re-check every hard off immediately before the network request.
        if send_gate() != SendGate::Allowed || !reservation_is_current_in(&state_dir, &reservation)
        {
            return Ok(false);
        }
        let response = runtime.block_on(async {
            client
                .post(endpoint)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body)
                .send()
                .await
        });
        Ok(response.is_ok_and(|response| response.status().is_success()))
    })();

    if outcome.unwrap_or(false) {
        commit_beacon_in(&state_dir, &reservation, chrono::Utc::now())?;
    } else {
        release_beacon_reservation_in(&state_dir, &reservation)?;
    }
    Ok(())
}

/// The exact dimension allowlist the disclosure names. Nothing outside
/// this list is ever sent; adding a dimension requires a dated amendment
/// to the governing decision record.
pub const DISCLOSED_DIMENSIONS: [&str; 8] = [
    "schema version",
    "anvil version",
    "install method",
    "platform",
    "release channel",
    "flag snapshot version",
    "feature usage counts",
    "anonymous install id (random; derived from nothing)",
];

/// Persisted telemetry consent state.
///
/// No field carries a serde default: a consent file missing a field is
/// corrupt operator state and must surface as an error (fail-safe:
/// blocked), never silently become "on".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentState {
    /// Schema version of the persisted file.
    pub schema_version: u32,
    /// Whether the user has telemetry on (`anvil telemetry on|off`).
    pub enabled: bool,
    /// Whether the first-run disclosure notice has actually been shown.
    /// The send gate refuses while this is false, so the notice strictly
    /// precedes any first beacon.
    pub notice_shown: bool,
}

impl Default for ConsentState {
    /// Disclosed opt-out posture: enabled by default, but no beacon can
    /// fire until the notice has been shown (`notice_shown` starts
    /// false).
    fn default() -> Self {
        Self {
            schema_version: CONSENT_SCHEMA_VERSION,
            enabled: true,
            notice_shown: false,
        }
    }
}

/// Outcome of a consent update (`anvil telemetry on|off`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentUpdate {
    /// The state now persisted on disk.
    pub state: ConsentState,
    /// Whether a previously unreadable consent file was rewritten from
    /// scratch (explicit user action is the only path allowed to repair
    /// corrupt operator state).
    pub repaired: bool,
}

/// Verdict of the pre-send gate the beacon must pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendGate {
    /// Every consent condition holds; a beacon may be sent.
    Allowed,
    /// No beacon may be sent, for the given reason.
    Blocked(BlockReason),
}

/// Why the send gate refused. Ordered by precedence: hard offs first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    /// `DO_NOT_TRACK` is set (cross-tool superset off).
    DoNotTrack,
    /// `ANVIL_TELEMETRY=off` is set.
    EnvOff,
    /// Running under a non-default `ANVIL_HOME` (gated / CI
    /// environment) — such environments never beacon.
    GatedInstallRoot,
    /// The persisted consent state could not be read or parsed;
    /// fail-safe is off, never a silent "on".
    ConsentUnreadable,
    /// The user turned telemetry off (`anvil telemetry off`).
    PersistedOff,
    /// The first-run disclosure notice has not been shown yet; the
    /// notice must strictly precede the first beacon.
    NoticeNotShown,
    /// ADR-107 permits beacons only from beta, release-candidate, and
    /// stable builds.
    IneligibleRelease,
}

impl BlockReason {
    /// Short human description for the transparency surface.
    #[must_use]
    pub fn describe(self) -> &'static str {
        match self {
            Self::DoNotTrack => "DO_NOT_TRACK is set",
            Self::EnvOff => "ANVIL_TELEMETRY=off is set",
            Self::GatedInstallRoot => {
                "running under a non-default ANVIL_HOME (gated/CI environment)"
            }
            Self::ConsentUnreadable => {
                "the consent state could not be read (fail-safe: off; \
                 run `anvil telemetry on` or `anvil telemetry off` to rewrite it)"
            }
            Self::PersistedOff => "telemetry is turned off (`anvil telemetry off`)",
            Self::NoticeNotShown => "the disclosure notice has not been shown yet",
            Self::IneligibleRelease => {
                "this build is earlier than beta (only beta, release-candidate, and stable builds beacon)"
            }
        }
    }
}

// ── Consent state persistence ────────────────────────────────────────

/// Path of the consent file under an explicit state directory.
#[must_use]
pub fn consent_path(state_dir: &Path) -> PathBuf {
    state_dir.join(CONSENT_FILE)
}

/// Load the persisted consent state from `state_dir`.
///
/// A missing file is the documented default (enabled, notice not yet
/// shown) — that is the opt-out posture, and the gate still refuses
/// until the notice is shown. An unreadable or unparseable file is an
/// `Err`: corrupt operator state is never silently defaulted.
pub fn load_consent_in(state_dir: &Path) -> Result<ConsentState> {
    let path = consent_path(state_dir);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(ConsentState::default()),
        Err(err) => {
            return Err(err).with_context(|| format!("read telemetry consent {}", path.display()));
        }
    };
    serde_json::from_str(&raw).with_context(|| {
        format!(
            "parse telemetry consent {} (corrupt operator state is never \
             silently defaulted; run `anvil telemetry on` or `anvil telemetry \
             off` to rewrite it)",
            path.display()
        )
    })
}

/// Persist `state` to `state_dir` with the salt-file posture: owner-only
/// (`0600`) file under an owner-only (`0700`) directory, written via a
/// unique temp sibling + rename so a crash never leaves a torn file.
pub fn save_consent_in(state_dir: &Path, state: &ConsentState) -> Result<()> {
    crate::usage::create_private_dir(state_dir)
        .with_context(|| format!("create state dir {}", state_dir.display()))?;
    let path = consent_path(state_dir);
    let json = serde_json::to_string_pretty(state).context("serialise telemetry consent")?;
    write_private_atomic(&path, json.as_bytes())
        .with_context(|| format!("write telemetry consent {}", path.display()))
}

/// Flip persisted consent (`anvil telemetry on|off`).
///
/// Turning telemetry **on** also marks the notice as shown: an explicit
/// opt-in is strictly stronger consent than having read the notice.
/// This is the one path allowed to rewrite a corrupt consent file —
/// the user is explicitly setting the state, so repair is honest; the
/// returned `repaired` flag lets the caller surface that it happened.
pub fn set_enabled_in(state_dir: &Path, enabled: bool) -> Result<ConsentUpdate> {
    let (mut state, repaired) = match load_consent_in(state_dir) {
        Ok(state) => (state, false),
        Err(_) => (ConsentState::default(), true),
    };
    state.schema_version = CONSENT_SCHEMA_VERSION;
    state.enabled = enabled;
    if enabled {
        state.notice_shown = true;
    }
    save_consent_in(state_dir, &state)?;
    Ok(ConsentUpdate { state, repaired })
}

/// Record that the disclosure notice has been shown. Propagates a
/// corrupt-state error rather than overwriting it — only the explicit
/// `anvil telemetry on|off` path may repair corrupt operator state.
pub fn mark_notice_shown_in(state_dir: &Path) -> Result<ConsentState> {
    let mut state = load_consent_in(state_dir)?;
    state.notice_shown = true;
    save_consent_in(state_dir, &state)?;
    Ok(state)
}

// ── Send gate ────────────────────────────────────────────────────────

/// Whether an `ANVIL_TELEMETRY` value means "off". The documented value
/// is `off`; `0` and `false` are accepted as the safe superset so a
/// plausible spelling of "off" never accidentally leaves telemetry on.
fn telemetry_env_is_off(value: Option<&str>) -> bool {
    value.is_some_and(|v| {
        matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "off" | "0" | "false"
        )
    })
}

/// Whether a `DO_NOT_TRACK` value opts out. The convention value is `1`;
/// any non-empty value other than an explicit `0`/`false` is honoured as
/// an opt-out — when in doubt, do not track.
fn do_not_track_is_set(value: Option<&str>) -> bool {
    value.is_some_and(|v| {
        let v = v.trim();
        !v.is_empty() && !matches!(v.to_ascii_lowercase().as_str(), "0" | "false")
    })
}

/// Pure core of the send gate. Every condition is injected so tests can
/// pin each one without touching the process environment:
///
/// - `consent`: the loaded consent state, or `None` when it could not be
///   read (fail-safe: blocked).
/// - `anvil_telemetry` / `do_not_track`: raw env values.
/// - `install_root_gated`: whether a non-default `ANVIL_HOME` is active.
///
/// Hard offs are checked first so the reported reason always names the
/// strongest override in effect.
#[must_use]
pub fn evaluate_send_gate(
    consent: Option<&ConsentState>,
    anvil_telemetry: Option<&str>,
    do_not_track: Option<&str>,
    install_root_gated: bool,
) -> SendGate {
    if do_not_track_is_set(do_not_track) {
        return SendGate::Blocked(BlockReason::DoNotTrack);
    }
    if telemetry_env_is_off(anvil_telemetry) {
        return SendGate::Blocked(BlockReason::EnvOff);
    }
    if install_root_gated {
        return SendGate::Blocked(BlockReason::GatedInstallRoot);
    }
    let Some(consent) = consent else {
        return SendGate::Blocked(BlockReason::ConsentUnreadable);
    };
    if !consent.enabled {
        return SendGate::Blocked(BlockReason::PersistedOff);
    }
    if !consent.notice_shown {
        return SendGate::Blocked(BlockReason::NoticeNotShown);
    }
    SendGate::Allowed
}

/// Resolve the send gate from the real environment and persisted state.
/// The beacon producer MUST consult this immediately before every send.
#[must_use]
pub fn send_gate() -> SendGate {
    let install_root_gated = crate::install_root::install_root().is_overridden();
    let anvil_telemetry = env::var(TELEMETRY_ENV).ok();
    let do_not_track = env::var(DO_NOT_TRACK_ENV).ok();
    let consent = credentials::credentials_dir().and_then(|dir| load_consent_in(&dir));
    let gate = match consent {
        Ok(state) => evaluate_send_gate(
            Some(&state),
            anvil_telemetry.as_deref(),
            do_not_track.as_deref(),
            install_root_gated,
        ),
        Err(err) => {
            let gate = evaluate_send_gate(
                None,
                anvil_telemetry.as_deref(),
                do_not_track.as_deref(),
                install_root_gated,
            );
            // Surface the corruption only when it is the deciding reason —
            // a hard off already silences the beacon regardless.
            if gate == SendGate::Blocked(BlockReason::ConsentUnreadable) {
                tracing::warn!(
                    target: "anvil::telemetry",
                    error = %format!("{err:#}"),
                    "telemetry consent state unreadable — beacon disabled \
                     (fail-safe); run `anvil telemetry on` or `anvil telemetry \
                     off` to rewrite it",
                );
            }
            gate
        }
    };
    if gate == SendGate::Allowed && !release_is_eligible(env!("CARGO_PKG_VERSION")) {
        return SendGate::Blocked(BlockReason::IneligibleRelease);
    }
    gate
}

/// Convenience predicate over [`send_gate`] for the beacon producer.
#[must_use]
pub fn send_allowed() -> bool {
    send_gate() == SendGate::Allowed
}

// ── First-run disclosure ─────────────────────────────────────────────

/// The disclosure notice: names the exact dimension allowlist and the
/// one-line off switch. Shown on the first-run surface strictly before
/// any beacon can fire.
#[must_use]
pub fn disclosure_text() -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "  Anonymous usage telemetry");
    let _ = writeln!(
        out,
        "  anvil sends at most one anonymous beacon per day so active installs"
    );
    let _ = writeln!(
        out,
        "  and feature adoption can be counted. It only ever contains:"
    );
    for dimension in DISCLOSED_DIMENSIONS {
        let _ = writeln!(out, "    - {dimension}");
    }
    let _ = writeln!(
        out,
        "  Never: paths, repository names, arguments, hostnames, or emails."
    );
    let _ = writeln!(out, "  Turn it off any time: `anvil telemetry off`");
    let _ = write!(
        out,
        "  (or set ANVIL_TELEMETRY=off or DO_NOT_TRACK=1). Audit it: `anvil telemetry`."
    );
    out
}

/// Resolve the first-run disclosure for a surface about to print its
/// closing output.
///
/// Returns the notice text when it is due, or `None` when it has already
/// been shown or the user has already opted out (no point disclosing a
/// beacon that will not fire). `notice_shown` is persisted **only when
/// `stdout_is_tty` is true** — a piped/CI run may print the text, but a
/// notice no human saw must never unlock the beacon, so non-TTY first
/// runs stay blocked (gated/CI environments never beacon).
///
/// A corrupt consent file propagates as `Err` (the caller warns; the
/// send gate is already fail-safe blocked) and is never overwritten from
/// this passive path.
pub fn first_run_disclosure_in(state_dir: &Path, stdout_is_tty: bool) -> Result<Option<String>> {
    let consent = load_consent_in(state_dir)?;
    if consent.notice_shown || !consent.enabled {
        return Ok(None);
    }
    if stdout_is_tty {
        mark_notice_shown_in(state_dir)?;
    }
    Ok(Some(disclosure_text()))
}

/// Print the first-run disclosure to stdout if it is due. Best-effort:
/// a consent-state failure warns on stderr and never fails the calling
/// surface (the send gate independently fails safe).
pub fn print_first_run_disclosure(stdout_is_tty: bool) {
    let outcome =
        credentials::credentials_dir().and_then(|dir| first_run_disclosure_in(&dir, stdout_is_tty));
    match outcome {
        Ok(Some(text)) => println!("\n{text}"),
        Ok(None) => {}
        Err(err) => {
            eprintln!("[telemetry] warning: could not resolve telemetry consent state: {err:#}");
        }
    }
}

// ── Anonymous install identity ───────────────────────────────────────

/// Path of the install-id file under an explicit state directory — a
/// sibling of the per-deployment `usage.salt`.
#[must_use]
pub fn install_id_path(state_dir: &Path) -> PathBuf {
    state_dir.join(INSTALL_ID_FILE)
}

/// Read the existing install id without minting one. Read-only surfaces
/// (`anvil telemetry` status) use this so inspecting the state never
/// creates identity.
#[must_use]
pub fn existing_install_id_in(state_dir: &Path) -> Option<Uuid> {
    let raw = fs::read_to_string(install_id_path(state_dir)).ok()?;
    Uuid::parse_str(raw.trim()).ok()
}

/// Load the anonymous install id, minting a fresh random UUID v4 on
/// first use — derived from nothing (no hardware, no user identity, no
/// salted principal). Stored owner-only (`0600`) beside the usage salt.
///
/// A corrupt id file is replaced by a fresh mint: regenerating a random
/// identity is equivalent to a rotation (the privacy-safe direction),
/// mirroring the salt-regeneration posture in the usage pipe. A
/// first-use race with a sibling process adopts the winner's id so both
/// processes report the same install.
pub fn load_or_create_install_id_in(state_dir: &Path) -> Result<Uuid> {
    let path = install_id_path(state_dir);
    match fs::read_to_string(&path) {
        Ok(raw) => {
            if let Ok(id) = Uuid::parse_str(raw.trim()) {
                return Ok(id);
            }
            rotate_install_id_in(state_dir)
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            crate::usage::create_private_dir(state_dir)
                .with_context(|| format!("create state dir {}", state_dir.display()))?;
            let id = Uuid::new_v4();
            match write_install_id_exclusive(&path, id) {
                Ok(()) => Ok(id),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                    let raw = fs::read_to_string(&path)
                        .with_context(|| format!("re-read install id {}", path.display()))?;
                    Uuid::parse_str(raw.trim()).with_context(|| {
                        format!("install id {} exists but is unreadable", path.display())
                    })
                }
                Err(err) => {
                    Err(err).with_context(|| format!("write install id {}", path.display()))
                }
            }
        }
        Err(err) => Err(err).with_context(|| format!("read install id {}", path.display())),
    }
}

/// Rotate the install id to a fresh random UUID v4 (`anvil telemetry
/// reset-id`). Rotation is deletion from the operator's ability to
/// correlate: previously reported usage becomes unjoinable.
pub fn rotate_install_id_in(state_dir: &Path) -> Result<Uuid> {
    crate::usage::create_private_dir(state_dir)
        .with_context(|| format!("create state dir {}", state_dir.display()))?;
    let path = install_id_path(state_dir);
    let id = Uuid::new_v4();
    invalidate_beacon_reservation_in(state_dir)?;
    write_private_atomic(&path, id.as_hyphenated().to_string().as_bytes())
        .with_context(|| format!("write install id {}", path.display()))?;
    Ok(id)
}

/// Atomically create the install-id file (fails if it already exists),
/// mode `0600` on Unix — the same exclusive-create posture as the usage
/// salt so a first-use race never overwrites the winner.
fn write_install_id_exclusive(path: &Path, id: Uuid) -> io::Result<()> {
    let mut opts = fs::OpenOptions::new();
    opts.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts.open(path)?;
    file.write_all(id.as_hyphenated().to_string().as_bytes())
}

/// Write `bytes` to a unique owner-only (`0600`) temp sibling of `path`,
/// then rename it over `path`. A failure leaves any existing file
/// intact; the temp file is removed on error.
fn write_private_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} has no parent directory", path.display()),
        )
    })?;
    let leaf = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} has no file name", path.display()),
        )
    })?;
    let temp = parent.join(format!(
        ".{}.{}.tmp",
        leaf.to_string_lossy(),
        Uuid::new_v4().as_simple()
    ));
    let mut opts = fs::OpenOptions::new();
    opts.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts.open(&temp)?;
    if let Err(err) = file.write_all(bytes) {
        drop(file);
        let _ = fs::remove_file(&temp);
        return Err(err);
    }
    drop(file);
    if let Err(err) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(err);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beacon_payload_serialises_to_the_exact_ingest_allowlist() {
        let payload = BeaconPayload::new(
            Uuid::parse_str("018f78e4-49b5-7f23-a33f-7db9ad9a2f45").unwrap(),
            "0.9.0-beta",
            "cargo_dist",
            "x86_64-unknown-linux-gnu",
            "beta",
            "1",
            vec![FeatureUsage {
                key: "cli.licence-gate".to_string(),
                count: 3,
            }],
        );

        assert_eq!(
            serde_json::to_value(payload).unwrap(),
            serde_json::json!({
                "schema_version": 1,
                "install_id": "018f78e4-49b5-7f23-a33f-7db9ad9a2f45",
                "version": "0.9.0-beta",
                "install_method": "cargo_dist",
                "platform": "x86_64-unknown-linux-gnu",
                "channel": "beta",
                "flag_snapshot_version": "1",
                "features": [{"key": "cli.licence-gate", "count": 3}],
            })
        );
    }

    #[test]
    fn feature_counts_include_only_usage_after_the_last_success() {
        use crate::usage_views::{FlagEntry, UsageRow};

        let row = |timestamp: &str, keys: &[&str]| UsageRow {
            command: "start".to_string(),
            principal: "must-not-enter-payload".to_string(),
            timestamp: timestamp.to_string(),
            flag_set: keys
                .iter()
                .map(|key| FlagEntry {
                    key: (*key).to_string(),
                    variant: "enabled".to_string(),
                    gate_affecting: true,
                })
                .collect(),
        };
        let rows = vec![
            row("2026-07-15T23:59:59Z", &["cli.licence-gate"]),
            row(
                "2026-07-16T00:00:01Z",
                &["daemon.persist-graph", "cli.licence-gate", "planted.secret"],
            ),
            row("2026-07-16T01:00:00Z", &["cli.licence-gate"]),
        ];
        let cutoff = chrono::DateTime::parse_from_rfc3339("2026-07-16T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        assert_eq!(
            feature_usage_since(&rows, Some(cutoff)),
            vec![
                FeatureUsage {
                    key: "cli.licence-gate".to_string(),
                    count: 2,
                },
                FeatureUsage {
                    key: "daemon.persist-graph".to_string(),
                    count: 1,
                },
            ]
        );
    }

    #[test]
    fn reservation_commit_enforces_one_success_per_install_per_day() {
        let dir = temp_dir();
        let install_id = Uuid::new_v4();
        let now = chrono::DateTime::parse_from_rfc3339("2026-07-16T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        let failed = reserve_beacon_in(dir.path(), install_id, now).unwrap();
        release_beacon_reservation_in(dir.path(), &failed).unwrap();
        let reservation = reserve_beacon_in(dir.path(), install_id, now).unwrap();
        commit_beacon_in(dir.path(), &reservation, now).unwrap();

        assert!(matches!(
            reserve_beacon_in(
                dir.path(),
                install_id,
                now + chrono::Duration::hours(23) + chrono::Duration::minutes(59)
            ),
            Err(ReserveError::TooRecent)
        ));
        assert!(
            reserve_beacon_in(dir.path(), install_id, now + chrono::Duration::hours(24)).is_ok()
        );
    }

    #[test]
    fn stale_reservation_is_recovered_for_a_later_start() {
        let dir = temp_dir();
        let install_id = Uuid::new_v4();
        let now = chrono::DateTime::parse_from_rfc3339("2026-07-16T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);

        reserve_beacon_in(dir.path(), install_id, now).unwrap();
        assert!(matches!(
            reserve_beacon_in(dir.path(), install_id, now + chrono::Duration::minutes(1)),
            Err(ReserveError::Busy)
        ));
        assert_eq!(
            next_delivery_block_reason_in(
                dir.path(),
                install_id,
                now + chrono::Duration::minutes(5)
            )
            .unwrap(),
            None
        );
        assert!(
            reserve_beacon_in(dir.path(), install_id, now + chrono::Duration::minutes(5)).is_ok()
        );
    }

    #[test]
    fn payload_preview_mints_only_when_the_send_gate_is_allowed() {
        let blocked_dir = temp_dir();
        let blocked = next_payload_for_gate_in(
            blocked_dir.path(),
            SendGate::Blocked(BlockReason::NoticeNotShown),
        )
        .unwrap();
        assert!(blocked.is_none());
        assert!(!install_id_path(blocked_dir.path()).exists());

        let allowed_dir = temp_dir();
        let payload = next_payload_for_gate_in(allowed_dir.path(), SendGate::Allowed)
            .unwrap()
            .expect("allowed preview has a canonical payload");
        assert_eq!(
            existing_install_id_in(allowed_dir.path())
                .unwrap()
                .to_string(),
            payload.install_id
        );
    }

    #[test]
    fn payload_preview_names_the_daily_cap_after_success() {
        let dir = temp_dir();
        let install_id = load_or_create_install_id_in(dir.path()).unwrap();
        let now = chrono::DateTime::parse_from_rfc3339("2026-07-16T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let reservation = reserve_beacon_in(dir.path(), install_id, now).unwrap();
        commit_beacon_in(dir.path(), &reservation, now).unwrap();

        assert_eq!(
            next_delivery_block_reason_in(dir.path(), install_id, now + chrono::Duration::hours(1))
                .unwrap(),
            Some("the last successful beacon was less than 24 hours ago")
        );
    }

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[cfg(unix)]
    fn mode_of(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    fn on_and_noticed() -> ConsentState {
        ConsentState {
            notice_shown: true,
            ..ConsentState::default()
        }
    }

    // ── Consent persistence ──────────────────────────────────────────

    #[test]
    fn missing_consent_file_defaults_to_enabled_notice_not_shown() {
        let dir = temp_dir();
        let state = load_consent_in(dir.path()).unwrap();
        assert_eq!(state, ConsentState::default());
        assert!(state.enabled);
        assert!(!state.notice_shown);
    }

    #[test]
    fn consent_round_trips_via_save_and_load() {
        let dir = temp_dir();
        let state = ConsentState {
            schema_version: CONSENT_SCHEMA_VERSION,
            enabled: false,
            notice_shown: true,
        };
        save_consent_in(dir.path(), &state).unwrap();
        assert_eq!(load_consent_in(dir.path()).unwrap(), state);
    }

    #[test]
    fn corrupt_consent_file_is_an_error_not_a_silent_default() {
        let dir = temp_dir();
        fs::write(consent_path(dir.path()), "{not json").unwrap();
        assert!(load_consent_in(dir.path()).is_err());
    }

    #[test]
    fn consent_file_missing_a_field_is_an_error() {
        // Operator-config rule: no serde defaults — a truncated file must
        // surface, never silently parse as "on".
        let dir = temp_dir();
        fs::write(consent_path(dir.path()), r#"{"schema_version":1}"#).unwrap();
        assert!(load_consent_in(dir.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn consent_file_is_written_owner_only() {
        let dir = temp_dir();
        save_consent_in(dir.path(), &ConsentState::default()).unwrap();
        assert_eq!(mode_of(&consent_path(dir.path())), 0o600);
    }

    #[test]
    fn set_enabled_on_marks_notice_shown() {
        let dir = temp_dir();
        let update = set_enabled_in(dir.path(), true).unwrap();
        assert!(!update.repaired);
        assert!(update.state.enabled);
        assert!(update.state.notice_shown);
        assert_eq!(load_consent_in(dir.path()).unwrap(), update.state);
    }

    #[test]
    fn set_enabled_off_persists_disabled() {
        let dir = temp_dir();
        let update = set_enabled_in(dir.path(), false).unwrap();
        assert!(!update.state.enabled);
        let loaded = load_consent_in(dir.path()).unwrap();
        assert!(!loaded.enabled);
    }

    #[test]
    fn set_enabled_repairs_a_corrupt_consent_file() {
        let dir = temp_dir();
        fs::write(consent_path(dir.path()), "{not json").unwrap();
        let update = set_enabled_in(dir.path(), false).unwrap();
        assert!(update.repaired);
        assert!(!update.state.enabled);
        // Repaired file is readable again.
        assert_eq!(load_consent_in(dir.path()).unwrap(), update.state);
    }

    #[test]
    fn mark_notice_shown_propagates_corrupt_state() {
        let dir = temp_dir();
        fs::write(consent_path(dir.path()), "{not json").unwrap();
        assert!(mark_notice_shown_in(dir.path()).is_err());
        // The corrupt file is left untouched for the explicit repair path.
        assert_eq!(
            fs::read_to_string(consent_path(dir.path())).unwrap(),
            "{not json"
        );
    }

    // ── Send gate: pin every condition ───────────────────────────────

    #[test]
    fn gate_allows_only_when_enabled_noticed_and_no_hard_off() {
        let consent = on_and_noticed();
        assert_eq!(
            evaluate_send_gate(Some(&consent), None, None, false),
            SendGate::Allowed
        );
    }

    #[test]
    fn gate_blocks_when_notice_not_shown() {
        let consent = ConsentState::default();
        assert_eq!(
            evaluate_send_gate(Some(&consent), None, None, false),
            SendGate::Blocked(BlockReason::NoticeNotShown)
        );
    }

    #[test]
    fn gate_blocks_when_persisted_off() {
        let consent = ConsentState {
            enabled: false,
            notice_shown: true,
            ..ConsentState::default()
        };
        assert_eq!(
            evaluate_send_gate(Some(&consent), None, None, false),
            SendGate::Blocked(BlockReason::PersistedOff)
        );
    }

    #[test]
    fn gate_blocks_on_anvil_telemetry_off_env() {
        let consent = on_and_noticed();
        for value in ["off", "OFF", " off ", "0", "false"] {
            assert_eq!(
                evaluate_send_gate(Some(&consent), Some(value), None, false),
                SendGate::Blocked(BlockReason::EnvOff),
                "ANVIL_TELEMETRY={value:?} must block",
            );
        }
        // A non-off value does not block.
        assert_eq!(
            evaluate_send_gate(Some(&consent), Some("on"), None, false),
            SendGate::Allowed
        );
    }

    #[test]
    fn gate_blocks_on_do_not_track() {
        let consent = on_and_noticed();
        for value in ["1", "true", "yes"] {
            assert_eq!(
                evaluate_send_gate(Some(&consent), None, Some(value), false),
                SendGate::Blocked(BlockReason::DoNotTrack),
                "DO_NOT_TRACK={value:?} must block",
            );
        }
    }

    #[test]
    fn gate_ignores_explicitly_unset_do_not_track_values() {
        let consent = on_and_noticed();
        for value in ["0", "false", "", "  "] {
            assert_eq!(
                evaluate_send_gate(Some(&consent), None, Some(value), false),
                SendGate::Allowed,
                "DO_NOT_TRACK={value:?} must not block",
            );
        }
    }

    #[test]
    fn gate_blocks_under_gated_install_root() {
        let consent = on_and_noticed();
        assert_eq!(
            evaluate_send_gate(Some(&consent), None, None, true),
            SendGate::Blocked(BlockReason::GatedInstallRoot)
        );
    }

    #[test]
    fn gate_blocks_when_consent_unreadable() {
        assert_eq!(
            evaluate_send_gate(None, None, None, false),
            SendGate::Blocked(BlockReason::ConsentUnreadable)
        );
    }

    #[test]
    fn hard_offs_override_full_consent() {
        // Even a fully consented state never beats an env hard off, and
        // DO_NOT_TRACK outranks everything.
        let consent = on_and_noticed();
        assert_eq!(
            evaluate_send_gate(Some(&consent), Some("off"), Some("1"), true),
            SendGate::Blocked(BlockReason::DoNotTrack)
        );
        assert_eq!(
            evaluate_send_gate(Some(&consent), Some("off"), None, true),
            SendGate::Blocked(BlockReason::EnvOff)
        );
    }

    #[test]
    fn only_beta_release_candidates_and_stable_builds_are_eligible() {
        for version in ["0.9.0-beta.1", "0.9.0-rc.1", "0.9.0"] {
            assert!(release_is_eligible(version), "{version} should be eligible");
        }
        for version in ["0.9.0-nightly.1", "0.9.0-alpha.1", "0.9.0-dev"] {
            assert!(!release_is_eligible(version), "{version} should be blocked");
        }
    }

    #[cfg(unix)]
    #[test]
    fn detached_beacon_spawn_adds_no_child_runtime_latency() {
        use std::os::unix::fs::PermissionsExt as _;
        use std::time::Instant;

        let dir = temp_dir();
        let executable = dir.path().join("slow-worker");
        fs::write(&executable, "#!/bin/sh\nsleep 2\n").unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();

        let started = Instant::now();
        spawn_beacon_process(&executable).unwrap();
        assert!(
            started.elapsed() < Duration::from_millis(500),
            "detached spawn waited for worker runtime"
        );
    }

    #[test]
    fn send_gate_end_to_end_with_temp_state_dir() {
        // Wrapper-level check against the real env readers, isolated to a
        // temp XDG_CONFIG_HOME so the real user state is never touched.
        let dir = temp_dir();
        temp_env::with_vars(
            [
                ("XDG_CONFIG_HOME", Some(dir.path().to_str().unwrap())),
                ("ANVIL_HOME", None::<&str>),
                (TELEMETRY_ENV, None),
                (DO_NOT_TRACK_ENV, None),
            ],
            || {
                let state_dir = credentials::credentials_dir().unwrap();
                // Notice not shown yet → blocked.
                assert!(!send_allowed());
                save_consent_in(&state_dir, &on_and_noticed()).unwrap();
                assert!(send_allowed());
                temp_env::with_var(DO_NOT_TRACK_ENV, Some("1"), || {
                    assert!(!send_allowed());
                });
                temp_env::with_var(TELEMETRY_ENV, Some("off"), || {
                    assert!(!send_allowed());
                });
                // A gated ANVIL_HOME re-roots the state dir AND hard-blocks.
                temp_env::with_var("ANVIL_HOME", Some(dir.path().to_str().unwrap()), || {
                    assert_eq!(
                        send_gate(),
                        SendGate::Blocked(BlockReason::GatedInstallRoot)
                    );
                });
                // Corrupt state → fail-safe blocked, not silently "on".
                fs::write(consent_path(&state_dir), "{not json").unwrap();
                assert_eq!(
                    send_gate(),
                    SendGate::Blocked(BlockReason::ConsentUnreadable)
                );
            },
        );
    }

    // ── Disclosure ───────────────────────────────────────────────────

    #[test]
    fn disclosure_names_every_allowlisted_dimension_and_the_off_switches() {
        let text = disclosure_text();
        assert_eq!(DISCLOSED_DIMENSIONS.len(), 8);
        assert!(text.contains("schema version"));
        for dimension in DISCLOSED_DIMENSIONS {
            assert!(
                text.contains(dimension),
                "disclosure must name {dimension:?}"
            );
        }
        assert!(text.contains("anvil telemetry off"));
        assert!(text.contains("ANVIL_TELEMETRY=off"));
        assert!(text.contains("DO_NOT_TRACK=1"));
    }

    #[test]
    fn first_run_disclosure_on_tty_marks_notice_shown_once() {
        let dir = temp_dir();
        let first = first_run_disclosure_in(dir.path(), true).unwrap();
        assert!(first.is_some());
        let state = load_consent_in(dir.path()).unwrap();
        assert!(state.notice_shown);
        // Notice unlocks the gate (no hard offs, not gated).
        assert_eq!(
            evaluate_send_gate(Some(&state), None, None, false),
            SendGate::Allowed
        );
        // Second run: already shown → silent.
        assert!(first_run_disclosure_in(dir.path(), true).unwrap().is_none());
    }

    #[test]
    fn first_run_disclosure_non_tty_prints_but_never_marks_shown() {
        // A non-TTY first run may print the text, but a notice no human
        // saw must never unlock the beacon.
        let dir = temp_dir();
        let first = first_run_disclosure_in(dir.path(), false).unwrap();
        assert!(first.is_some());
        let state = load_consent_in(dir.path()).unwrap();
        assert!(!state.notice_shown);
        assert_eq!(
            evaluate_send_gate(Some(&state), None, None, false),
            SendGate::Blocked(BlockReason::NoticeNotShown)
        );
        // And it stays due for a future interactive run.
        assert!(
            first_run_disclosure_in(dir.path(), false)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn first_run_disclosure_skipped_when_opted_out() {
        let dir = temp_dir();
        set_enabled_in(dir.path(), false).unwrap();
        assert!(first_run_disclosure_in(dir.path(), true).unwrap().is_none());
    }

    #[test]
    fn first_run_disclosure_errors_on_corrupt_state_without_rewriting_it() {
        let dir = temp_dir();
        fs::write(consent_path(dir.path()), "{not json").unwrap();
        assert!(first_run_disclosure_in(dir.path(), true).is_err());
        assert_eq!(
            fs::read_to_string(consent_path(dir.path())).unwrap(),
            "{not json"
        );
    }

    // ── Install identity ─────────────────────────────────────────────

    #[test]
    fn install_id_minted_once_and_stable_across_calls() {
        let dir = temp_dir();
        let first = load_or_create_install_id_in(dir.path()).unwrap();
        let second = load_or_create_install_id_in(dir.path()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn install_id_is_uuid_v4() {
        let dir = temp_dir();
        let id = load_or_create_install_id_in(dir.path()).unwrap();
        assert_eq!(id.get_version_num(), 4);
    }

    #[test]
    fn reset_rotates_install_id() {
        let dir = temp_dir();
        let original = load_or_create_install_id_in(dir.path()).unwrap();
        let rotated = rotate_install_id_in(dir.path()).unwrap();
        assert_ne!(original, rotated);
        // The rotated id is what subsequent loads see.
        assert_eq!(load_or_create_install_id_in(dir.path()).unwrap(), rotated);
        assert_eq!(rotated.get_version_num(), 4);
    }

    #[test]
    fn reset_invalidates_an_in_flight_old_identity_reservation() {
        let dir = temp_dir();
        let original = load_or_create_install_id_in(dir.path()).unwrap();
        let reservation = reserve_beacon_in(dir.path(), original, chrono::Utc::now()).unwrap();

        let rotated = rotate_install_id_in(dir.path()).unwrap();

        assert_ne!(original, rotated);
        assert!(!reservation_is_current_in(dir.path(), &reservation));
        assert!(!beacon_reservation_path(dir.path()).exists());
    }

    #[cfg(unix)]
    #[test]
    fn install_id_file_is_owner_only_after_mint_and_rotate() {
        let dir = temp_dir();
        load_or_create_install_id_in(dir.path()).unwrap();
        assert_eq!(mode_of(&install_id_path(dir.path())), 0o600);
        rotate_install_id_in(dir.path()).unwrap();
        assert_eq!(mode_of(&install_id_path(dir.path())), 0o600);
    }

    #[test]
    fn install_id_file_contains_only_the_uuid() {
        let dir = temp_dir();
        let id = load_or_create_install_id_in(dir.path()).unwrap();
        let raw = fs::read_to_string(install_id_path(dir.path())).unwrap();
        assert_eq!(raw, id.as_hyphenated().to_string());
        assert_eq!(raw.len(), 36);
    }

    #[test]
    fn corrupt_install_id_file_is_replaced_by_fresh_mint() {
        let dir = temp_dir();
        fs::create_dir_all(dir.path()).unwrap();
        fs::write(install_id_path(dir.path()), "not-a-uuid").unwrap();
        let id = load_or_create_install_id_in(dir.path()).unwrap();
        assert_eq!(id.get_version_num(), 4);
        assert_eq!(
            fs::read_to_string(install_id_path(dir.path())).unwrap(),
            id.as_hyphenated().to_string()
        );
    }

    #[test]
    fn install_ids_are_independent_random_mints() {
        // Derived from nothing: two state dirs on the same host must not
        // agree — there is no shared (hardware/user) input to derive from.
        let a = temp_dir();
        let b = temp_dir();
        let id_a = load_or_create_install_id_in(a.path()).unwrap();
        let id_b = load_or_create_install_id_in(b.path()).unwrap();
        assert_ne!(id_a, id_b);
    }

    #[test]
    fn identity_file_never_contains_the_salted_principal() {
        // Payload-boundary contract: the identity file is exactly one
        // 36-char UUID, so it cannot carry the salted usage principal (a
        // 64-char hex digest) — and this module exposes no accessor for
        // the principal at all.
        let dir = temp_dir();
        let id = load_or_create_install_id_in(dir.path()).unwrap();
        let principal = crate::usage::anonymise_principal(Some("user@example.com"), b"test-salt");
        let raw = fs::read_to_string(install_id_path(dir.path())).unwrap();
        assert_eq!(raw, id.as_hyphenated().to_string());
        assert!(!raw.contains(&principal));
        assert!(!principal.contains(&raw));
    }

    #[test]
    fn existing_install_id_never_mints() {
        let dir = temp_dir();
        assert!(existing_install_id_in(dir.path()).is_none());
        assert!(!install_id_path(dir.path()).exists());
        let id = load_or_create_install_id_in(dir.path()).unwrap();
        assert_eq!(existing_install_id_in(dir.path()), Some(id));
    }
}
