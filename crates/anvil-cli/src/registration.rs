//! ACTMO-014/015: the shared worktree-registration client.
//!
//! Relocated out of `activation/` (ADR-094 decision 2) so `anvil start`
//! (activation), `anvil workspace register/unregister/list` (ACTMO-015), and
//! `anvil workspace register --all` (ACTMO-018) all drive the daemon over one
//! primitive. The daemon classifies registration failures and the client maps
//! them to honest outcomes: a re-register of an already-owned worktree
//! heartbeats the existing owner rather than erroring (ADR-094 decision 3), a
//! fenced/cascaded worktree points at `anvil intercept unblock`, and a cap
//! breach gives a clear message. Paths are canonicalised with `dunce` so
//! identity is stable and display paths are free of the Windows `\\?\` prefix.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anvil_intercept_proto::session::AgentTag;
use anvil_intercept_proto::status::{
    DaemonStatusV1, SaveTimeDriverEvidenceV1, SaveTimeDriverStatusV1,
};
use anvil_intercept_proto::{SessionId, SessionRecord};
use serde_json::Value;

use crate::activation::daemon_evidence::ACTIVATION_DAEMON_QUERY_TIMEOUT;

/// JREL-003: after durable membership is confirmed, the ceiling on how long
/// the registration path waits for the daemon's supervisor to report the
/// worktree's save-time driver attached.
///
/// Almost never paid in full. The supervisor spawns (or restores) the child
/// within milliseconds of the membership signal, so a healthy registration
/// returns on the first snapshot that shows it; an outcome that cannot become
/// `attached` by waiting longer — a daemon with driver supervision disabled
/// (`ANVIL_NO_SAVE_TIME_DRIVER`), a worktree the supervisor's failure bound is
/// holding in `failed`, or an unreadable status — returns after
/// [`DRIVER_TRANSIENT_GRACE`] instead of polling out the budget for **absent**
/// (the documented opt-out and a daemon started without driver supervision).
/// `failed` keeps polling until the budget so a healthy respawn is not reported
/// as failed, and an unreadable snapshot is retried rather than giving up on
/// the first slow read.
///
/// Worst case in wall-clock terms: this budget plus one
/// [`ACTIVATION_DAEMON_QUERY_TIMEOUT`], because the deadline is only checked
/// between status reads and the read in flight when it expires is not
/// cancelled (review finding 6).
const DRIVER_READY_BUDGET: Duration = Duration::from_secs(1);
/// Gap between status polls while waiting for the driver.
const DRIVER_READY_INTERVAL: Duration = Duration::from_millis(50);
/// JREL-003: how long a `failed`/`absent` snapshot is still treated as "the
/// supervisor has not drained the membership signal yet". After a refresh the
/// first read can legitimately still show the dead child, so the wait keeps
/// polling for this long — but no longer, since nothing beyond a
/// still-starting driver changes those outcomes within a one-second budget.
/// Clamped to the budget for callers that pass a shorter one.
const DRIVER_TRANSIENT_GRACE: Duration = Duration::from_millis(200);

const REGISTER_METHOD: &str = "session.register";
const UNREGISTER_METHOD: &str = "session.unregister";
const HEARTBEAT_METHOD: &str = "heartbeat";
const RESPONSE_LINE_BYTES: u64 = 1 << 20;

/// The daemon acknowledges `session.register`/`heartbeat` before the record is
/// guaranteed to be visible in its `daemon.status` snapshot, so reading status
/// once immediately after the acknowledgement races the daemon and can report
/// an honest-but-wrong "membership absent" refusal. These bound a short wait
/// for the write to become visible.
///
/// The budget is only ever paid on the failing path: the first snapshot that
/// already carries the membership returns immediately, so a healthy
/// registration adds no latency. A registration the daemon genuinely refuses
/// (for example when the durable-claim peer check fails closed) still refuses,
/// just `MEMBERSHIP_CONFIRM_BUDGET` later.
const MEMBERSHIP_CONFIRM_BUDGET: Duration = Duration::from_secs(2);
/// Gap between status polls while waiting for the write to become visible.
const MEMBERSHIP_CONFIRM_INTERVAL: Duration = Duration::from_millis(50);

/// Outcome of a worktree registration attempt against the intercept daemon.
/// ACTMO-014 enriches the original four-variant enum with the daemon's
/// refusal classifications so callers can give honest, actionable guidance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorktreeRegistration {
    /// A fresh durable registration was recorded.
    Registered,
    /// The worktree was already registered (deterministic session id, or the
    /// daemon reported `WorktreeAlreadyOwned` for an equivalent path); the
    /// existing owner was heartbeated. Idempotent success.
    Refreshed,
    /// The daemon is not running / not reachable. Non-fatal for activation.
    DaemonUnavailable,
    /// The worktree is fenced or in fence-cascade mode and refuses
    /// registration until cleared. The string is user-facing guidance that
    /// points at `anvil intercept unblock`.
    Fenced(String),
    /// A registration cap was exceeded (per-worktree session cap, or the
    /// distinct registered-worktree membership cap). The string is the
    /// user-facing cap message.
    CapExceeded(String),
    /// Any other rejection. The string is the daemon's message.
    Rejected(String),
}

/// JREL-003: what the daemon reported about the worktree's supervised
/// save-time driver once durable membership was confirmed. Membership alone
/// is never readiness — this is the separate, bounded evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SaveTimeDriverReadiness {
    /// A supervised driver is attached. `evidence` is the daemon's readiness
    /// detail when it reports one (`None` from a pre-JREL-003 daemon).
    Attached {
        evidence: Option<SaveTimeDriverEvidenceV1>,
    },
    /// The daemon reported the driver `failed` — its spawn was refused or
    /// failed, or the child died and the supervisor's failure bound refused
    /// a respawn. Bounded: the wait ended, the driver is not coming.
    Failed,
    /// No driver entry appeared within the budget: driver supervision is
    /// disabled on this daemon, or its supervisor did not answer in time.
    Absent,
    /// Daemon status could not be read; the message is the last error.
    Unknown(String),
}

/// JREL-003: the outcome of [`register_worktree_with_daemon`] — the durable
/// membership outcome plus, when membership was confirmed, the separate
/// save-time driver readiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorktreeRegistrationReport {
    pub(crate) registration: WorktreeRegistration,
    /// `Some` only after a confirmed [`WorktreeRegistration::Registered`] or
    /// [`WorktreeRegistration::Refreshed`].
    pub(crate) driver: Option<SaveTimeDriverReadiness>,
}

impl WorktreeRegistrationReport {
    /// True when the daemon reported the driver `failed` — the one outcome a
    /// surface should name, since membership succeeded but coverage did not.
    pub(crate) fn driver_failed(&self) -> bool {
        matches!(self.driver, Some(SaveTimeDriverReadiness::Failed))
    }
}

/// ACTMO-014/015: register a worktree as durable membership with the daemon.
/// `worktree` may be any spelling of the path; it is canonicalised with
/// `dunce` before the deterministic activation session id is derived.
///
/// JREL-003: a confirmed registration or refresh then waits (bounded by
/// [`DRIVER_READY_BUDGET`]) for the daemon to report the worktree's save-time
/// driver attached. A refresh of an existing membership is how the daily
/// command restores a driver whose child died: the daemon respawns on the
/// gated duplicate `session.register`, and this wait is what turns that into
/// evidence rather than a race against the next status read.
pub(crate) fn register_worktree_with_daemon(worktree: &Path) -> WorktreeRegistrationReport {
    let canonical = canonicalise_for_registration(worktree);
    let registration = register_worktree_membership(&canonical);
    let driver = matches!(
        registration,
        WorktreeRegistration::Registered | WorktreeRegistration::Refreshed
    )
    .then(|| {
        await_save_time_driver(&canonical, || {
            crate::commands::intercept::query_daemon_status_with_timeout(
                ACTIVATION_DAEMON_QUERY_TIMEOUT,
            )
        })
    });
    if let Some(driver) = &driver {
        tracing::info!(
            worktree = %canonical.display(),
            driver = ?driver,
            "activation: save-time driver readiness after registration",
        );
    }
    WorktreeRegistrationReport {
        registration,
        driver,
    }
}

/// The durable-membership half of [`register_worktree_with_daemon`], against
/// an already-canonical worktree.
fn register_worktree_membership(canonical: &Path) -> WorktreeRegistration {
    let canonical = canonical.to_path_buf();
    let session_id = activation_session_id(&canonical);
    let request_id = format!("anvil-start-register-{}", session_id.as_str());

    match request_jsonrpc(
        REGISTER_METHOD,
        &session_register_params(&session_id, &canonical),
        &request_id,
        ACTIVATION_DAEMON_QUERY_TIMEOUT,
    ) {
        Ok(_) => match await_membership_snapshot(&canonical, &session_id, || {
            crate::commands::intercept::query_daemon_status_with_timeout(
                ACTIVATION_DAEMON_QUERY_TIMEOUT,
            )
            .map(|status| status.sessions)
        }) {
            Ok(sessions) => confirm_durable_registration(&canonical, &sessions),
            Err(err) => {
                let message = format!(
                    "daemon acknowledged registration for {}, but durable membership could not be confirmed: {err}",
                    canonical.display(),
                );
                tracing::warn!(
                    worktree = %canonical.display(),
                    session_id = session_id.as_str(),
                    error = %err,
                    "activation: daemon registration acknowledgement could not be confirmed",
                );
                WorktreeRegistration::Rejected(message)
            }
        },
        // ADR-094 decision 3: a re-register of the same canonical worktree
        // (our deterministic id, or a `WorktreeAlreadyOwned` for an equivalent
        // spelling) heartbeats the existing owner rather than erroring.
        Err(err) if err.is_session_already_registered() || err.is_worktree_already_owned() => {
            refresh_existing_activation_session(&session_id, &canonical)
        }
        Err(DaemonRegistrationError::DaemonUnavailable(message)) => {
            tracing::debug!(
                worktree = %canonical.display(),
                error = %message,
                "activation: intercept daemon unavailable for worktree registration",
            );
            WorktreeRegistration::DaemonUnavailable
        }
        Err(err) if err.is_fenced() => {
            let message = format!(
                "worktree {} is fenced; clear it with `anvil intercept unblock {}`",
                canonical.display(),
                canonical.display(),
            );
            tracing::warn!(worktree = %canonical.display(), "activation: worktree registration refused — fenced");
            WorktreeRegistration::Fenced(message)
        }
        Err(err) if err.is_cap_exceeded() => {
            let message = err.to_string();
            tracing::warn!(worktree = %canonical.display(), error = %message, "activation: worktree registration refused — cap exceeded");
            WorktreeRegistration::CapExceeded(message)
        }
        Err(err) => {
            let message = err.to_string();
            tracing::warn!(
                worktree = %canonical.display(),
                error = %message,
                "activation: worktree registration with intercept daemon failed",
            );
            WorktreeRegistration::Rejected(message)
        }
    }
}

fn confirm_durable_registration(
    canonical: &Path,
    sessions: &[SessionRecord],
) -> WorktreeRegistration {
    let session_id = activation_session_id(canonical);
    let outcome = confirm_registration_membership(
        canonical,
        &session_id,
        sessions,
        WorktreeRegistration::Registered,
    );
    if outcome == WorktreeRegistration::Registered {
        tracing::info!(
            worktree = %canonical.display(),
            "activation: registered worktree with intercept daemon",
        );
    }
    outcome
}

fn confirm_durable_refresh(canonical: &Path, sessions: &[SessionRecord]) -> WorktreeRegistration {
    let session_id = activation_session_id(canonical);
    confirm_registration_membership(
        canonical,
        &session_id,
        sessions,
        WorktreeRegistration::Refreshed,
    )
}

fn confirm_registration_membership(
    canonical: &Path,
    session_id: &SessionId,
    sessions: &[SessionRecord],
    confirmed: WorktreeRegistration,
) -> WorktreeRegistration {
    if durable_membership_present(canonical, session_id, sessions) {
        confirmed
    } else {
        missing_durable_membership(canonical)
    }
}

fn durable_membership_present(
    canonical: &Path,
    session_id: &SessionId,
    sessions: &[SessionRecord],
) -> bool {
    sessions.iter().any(|session| {
        &session.id == session_id
            && canonicalise_for_registration(&session.worktree) == canonical
            && session
                .agent_tag
                .as_ref()
                .is_some_and(AgentTag::is_durable_membership)
    })
}

/// Read daemon status until it reflects the durable membership we just wrote,
/// or the budget expires.
///
/// `session.register` and `heartbeat` are acknowledged before the record is
/// necessarily visible in `daemon.status`, so a single read immediately after
/// the acknowledgement races the daemon. Under load — a full-workspace test
/// run, for instance — that race loses often enough to turn a healthy
/// registration into a spurious "durable membership was absent" refusal.
///
/// Returns the first snapshot carrying the membership. If the budget expires
/// the last snapshot is returned unchanged so the caller still produces the
/// honest refusal established by CIB-252; if every read failed, the last error
/// is returned so the caller reports it as before.
///
/// Reads are retried after an `Err` as well as after a snapshot that simply
/// lacks the membership, because both mean "the daemon has not shown us the
/// write yet". That deliberately changes one outcome rather than none: a
/// transient status-read failure which the previous single read reported as
/// `Rejected` now recovers if a later read succeeds. It is the same class of
/// false refusal this function exists to remove. Every other outcome is
/// unchanged.
fn await_membership_snapshot<F, E>(
    canonical: &Path,
    session_id: &SessionId,
    fetch: F,
) -> Result<Vec<SessionRecord>, E>
where
    F: FnMut() -> Result<Vec<SessionRecord>, E>,
{
    await_membership_snapshot_within(
        canonical,
        session_id,
        MEMBERSHIP_CONFIRM_BUDGET,
        MEMBERSHIP_CONFIRM_INTERVAL,
        fetch,
    )
}

fn await_membership_snapshot_within<F, E>(
    canonical: &Path,
    session_id: &SessionId,
    budget: Duration,
    interval: Duration,
    mut fetch: F,
) -> Result<Vec<SessionRecord>, E>
where
    F: FnMut() -> Result<Vec<SessionRecord>, E>,
{
    let started = Instant::now();
    let deadline = started + budget;
    let mut reads = 1_usize;
    let mut last = fetch();
    loop {
        if let Ok(sessions) = &last
            && durable_membership_present(canonical, session_id, sessions)
        {
            return last;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            // Diagnostic only: the caller still owns the outcome and the
            // user-facing copy is unchanged. Recording how hard we looked
            // distinguishes a daemon write that never landed from a claim
            // the daemon refused outright, which the refusal copy cannot.
            tracing::warn!(
                worktree = %canonical.display(),
                session_id = session_id.as_str(),
                status_reads = reads,
                waited_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                last_read_failed = last.is_err(),
                "activation: durable membership did not appear before the confirm budget expired",
            );
            return last;
        }
        std::thread::sleep(interval.min(remaining));
        last = fetch();
        reads += 1;
    }
}

/// JREL-003: read daemon status until the worktree's save-time driver is
/// reported attached, or the budget expires. See [`DRIVER_READY_BUDGET`].
fn await_save_time_driver<F, E>(canonical: &Path, fetch: F) -> SaveTimeDriverReadiness
where
    F: FnMut() -> Result<DaemonStatusV1, E>,
    E: std::fmt::Display,
{
    await_save_time_driver_within(canonical, DRIVER_READY_BUDGET, DRIVER_READY_INTERVAL, fetch)
}

fn await_save_time_driver_within<F, E>(
    canonical: &Path,
    budget: Duration,
    interval: Duration,
    mut fetch: F,
) -> SaveTimeDriverReadiness
where
    F: FnMut() -> Result<DaemonStatusV1, E>,
    E: std::fmt::Display,
{
    let started = Instant::now();
    let deadline = started + budget;
    let grace = DRIVER_TRANSIENT_GRACE.min(budget);
    let mut reads = 1_usize;
    let mut last = fetch();
    let readiness = loop {
        let observed = classify_driver_snapshot(&last, canonical);
        match &observed {
            SaveTimeDriverReadiness::Attached { .. } => return observed,
            SaveTimeDriverReadiness::Unknown(_) | SaveTimeDriverReadiness::Failed => {
                // Unknown: a single slow or failed status read is not terminal.
                // Failed: keep polling — the snapshot can still show the dead
                // child before the supervisor drains the refresh and respawns.
                // Ending at the transient grace reported a healthy respawn as
                // failed.
            }
            SaveTimeDriverReadiness::Absent => {
                if started.elapsed() >= grace {
                    break observed;
                }
            }
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break observed;
        }
        std::thread::sleep(interval.min(remaining));
        last = fetch();
        reads += 1;
    };
    if matches!(readiness, SaveTimeDriverReadiness::Absent) {
        tracing::debug!(
            worktree = %canonical.display(),
            status_reads = reads,
            waited_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            "activation: save-time driver was absent after registration (opt-out, standalone daemon, or still starting)"
        );
    } else {
        tracing::warn!(
            worktree = %canonical.display(),
            status_reads = reads,
            waited_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            readiness = ?readiness,
            "activation: save-time driver was not attached before the readiness wait ended",
        );
    }
    readiness
}

/// Classify one daemon-status read as driver readiness for `canonical`.
fn classify_driver_snapshot<E>(
    last: &Result<DaemonStatusV1, E>,
    canonical: &Path,
) -> SaveTimeDriverReadiness
where
    E: std::fmt::Display,
{
    match last {
        Ok(status) => match worktree_driver_entry(status, canonical) {
            Some(entry) => match entry.save_time_driver {
                SaveTimeDriverStatusV1::Attached => SaveTimeDriverReadiness::Attached {
                    evidence: entry.save_time_driver_evidence,
                },
                SaveTimeDriverStatusV1::Failed => SaveTimeDriverReadiness::Failed,
                // A forward-compatible unknown state is treated fail-safe:
                // never readiness.
                SaveTimeDriverStatusV1::Absent | SaveTimeDriverStatusV1::Unknown => {
                    SaveTimeDriverReadiness::Absent
                }
            },
            None => SaveTimeDriverReadiness::Absent,
        },
        Err(err) => SaveTimeDriverReadiness::Unknown(err.to_string()),
    }
}

/// The daemon's status entry for `canonical`, matched on canonical identity
/// like [`durable_membership_present`].
fn worktree_driver_entry<'a>(
    status: &'a DaemonStatusV1,
    canonical: &Path,
) -> Option<&'a anvil_intercept_proto::status::WorktreeStatusV1> {
    status
        .worktrees
        .iter()
        .find(|w| canonicalise_for_registration(&w.worktree) == canonical)
}

/// CIB-252 established that this refusal must be honest rather than report a
/// false success. CIB-160 adds where to go next: the daemon only honours a
/// durable claim over the wire when it can verify the caller is running the
/// same `anvil` binary, and that check fails closed when the peer's executable
/// cannot be read. `--persist` records the worktree so the daemon registers it
/// **in-process at its next start**, which never crosses that check — so an
/// honest refusal now points at a path that works instead of looping the
/// operator through a retry that cannot succeed.
fn missing_durable_membership(canonical: &Path) -> WorktreeRegistration {
    let shown = canonical.display();
    WorktreeRegistration::Rejected(format!(
        "daemon acknowledged the workspace request for {shown}, but durable membership was absent from daemon status; \
         retry registration and inspect `anvil intercept status`. \
         If it keeps not sticking, run `anvil workspace register --persist` with that worktree path — it records the path so the daemon registers it at next start",
    ))
}

fn refresh_existing_activation_session(
    session_id: &SessionId,
    canonical: &Path,
) -> WorktreeRegistration {
    let request_id = format!("anvil-start-heartbeat-{}", session_id.as_str());
    match request_jsonrpc(
        HEARTBEAT_METHOD,
        &serde_json::json!({ "session_id": session_id.as_str() }),
        &request_id,
        ACTIVATION_DAEMON_QUERY_TIMEOUT,
    ) {
        Ok(_) => match await_membership_snapshot(canonical, session_id, || {
            crate::commands::intercept::query_daemon_status_with_timeout(
                ACTIVATION_DAEMON_QUERY_TIMEOUT,
            )
            .map(|status| status.sessions)
        }) {
            Ok(sessions) => {
                let outcome = confirm_durable_refresh(canonical, &sessions);
                if outcome == WorktreeRegistration::Refreshed {
                    tracing::info!(
                        worktree = %canonical.display(),
                        session_id = session_id.as_str(),
                        "activation: refreshed existing daemon worktree registration",
                    );
                }
                outcome
            }
            Err(err) => {
                let message = format!(
                    "daemon refreshed registration for {}, but durable membership could not be confirmed: {err}",
                    canonical.display(),
                );
                tracing::warn!(
                    worktree = %canonical.display(),
                    session_id = session_id.as_str(),
                    error = %err,
                    "activation: refreshed daemon registration could not be confirmed",
                );
                WorktreeRegistration::Rejected(message)
            }
        },
        Err(err) => {
            let message = err.to_string();
            tracing::warn!(
                worktree = %canonical.display(),
                session_id = session_id.as_str(),
                error = %message,
                "activation: existing worktree registration could not be refreshed",
            );
            WorktreeRegistration::Rejected(message)
        }
    }
}

/// Canonicalise with `dunce` so identity is stable and the path keeps a plain
/// form on Windows (no `\\?\` verbatim prefix) for both the wire key and any
/// display. Falls back to the raw path when canonicalisation fails; the daemon
/// is server-authoritative for identity, so a client fallback that diverges is
/// reconciled by the `WorktreeAlreadyOwned`→heartbeat path.
fn canonicalise_for_registration(worktree: &Path) -> PathBuf {
    dunce::canonicalize(worktree).unwrap_or_else(|err| {
        tracing::warn!(
            error = %err,
            worktree = %worktree.display(),
            "activation: worktree canonicalisation failed before daemon registration",
        );
        worktree.to_path_buf()
    })
}

// ACTMO-019: the activation session-id and agent-tag derivations live in
// `anvil-intercept` (`registration_store`) so the daemon's `register_on_start`
// startup path and this client derive an identical id and tag — a worktree
// registered either way shares membership instead of duplicating. This thin
// delegation is the single source of truth.
fn activation_session_id(worktree: &Path) -> SessionId {
    anvil_intercept::registration_store::activation_session_id(worktree)
}

fn activation_agent_tag() -> AgentTag {
    anvil_intercept::registration_store::activation_agent_tag()
}

fn session_register_params(session_id: &SessionId, worktree: &Path) -> Value {
    serde_json::json!({
        "session_id": session_id.as_str(),
        "worktree": worktree.to_string_lossy(),
        "agent_tag": activation_agent_tag(),
    })
}

/// Stable marker the daemon's registry uses when a `session.register`
/// reuses a live session id (`RegistryError::SessionAlreadyExists`,
/// `#[error("session already registered: …")]`). Activation derives a
/// deterministic session id from the worktree path, so a re-run of
/// `anvil start` against an already-registered worktree is the expected
/// way to hit this — we detect it and downgrade to a heartbeat instead
/// of treating it as a rejection.
///
/// Matched against the structured `error.data.error` field (not a blob of
/// the whole envelope) and case-insensitively, so reordering or casing
/// changes do not silently break the heartbeat fall-through. The
/// `marker_pins_registry_session_already_exists_display` test fails CI if
/// the daemon ever rephrases the wording out from under this constant.
const SESSION_ALREADY_REGISTERED_MARKER: &str = "session already registered";

/// ACTMO-014 markers pinning the daemon's `RegistryError` Display wording the
/// client classifies on. Each is covered by a cross-crate pin test below, so a
/// rephrase in the registry fails CI rather than silently degrading the
/// client's outcome mapping.
///
/// Each marker includes its adjacent **static** prefix word (`worktree …`,
/// `degraded …`) so it cannot be matched by a worktree *path* embedded in an
/// unrelated error's `{worktree:?}` field — e.g. a `SessionCapExceeded` message
/// for a path literally containing "is fenced" must not classify as fenced
/// (covered by `path_substring_does_not_misclassify`).
const WORKTREE_ALREADY_OWNED_MARKER: &str = "worktree already owned";
const WORKTREE_FENCED_MARKER: &str = "worktree is fenced";
const WORKTREE_CASCADED_MARKER: &str = "degraded fence-cascade mode";
const SESSION_CAP_MARKER: &str = "worktree session cap exceeded";
const REGISTERED_CAP_MARKER: &str = "registered worktree cap exceeded";

#[derive(Debug)]
enum DaemonRegistrationError {
    DaemonUnavailable(String),
    /// A JSON-RPC `error` response. `code` is the numeric error code when
    /// present; `message` is the most specific human string available
    /// (the daemon nests the registry detail under `error.data.error`).
    JsonRpc {
        code: Option<i64>,
        message: String,
    },
    Transport(String),
    Protocol(String),
}

impl DaemonRegistrationError {
    /// True when this error signals the worktree's activation session is
    /// already registered, i.e. a re-run that should heartbeat rather than
    /// re-register.
    fn is_session_already_registered(&self) -> bool {
        self.message_contains(SESSION_ALREADY_REGISTERED_MARKER)
    }

    /// ADR-094 decision 3: the same canonical worktree reached via a different
    /// spelling — the client heartbeats the existing owner.
    fn is_worktree_already_owned(&self) -> bool {
        self.message_contains(WORKTREE_ALREADY_OWNED_MARKER)
    }

    /// The worktree is fenced or in fence-cascade mode — the one genuine
    /// registration refusal, pointing the operator at `anvil intercept unblock`.
    fn is_fenced(&self) -> bool {
        self.message_contains(WORKTREE_FENCED_MARKER)
            || self.message_contains(WORKTREE_CASCADED_MARKER)
    }

    /// A per-worktree session cap or the distinct registered-worktree
    /// membership cap was exceeded.
    fn is_cap_exceeded(&self) -> bool {
        self.message_contains(SESSION_CAP_MARKER) || self.message_contains(REGISTERED_CAP_MARKER)
    }

    /// Case-insensitive substring match against the JSON-RPC error message.
    /// Transport / daemon-unavailable / protocol errors never carry a registry
    /// classification, so they always return `false`.
    fn message_contains(&self, marker: &str) -> bool {
        matches!(
            self,
            Self::JsonRpc { message, .. } if message.to_ascii_lowercase().contains(marker)
        )
    }
}

impl std::fmt::Display for DaemonRegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DaemonUnavailable(message)
            | Self::Transport(message)
            | Self::Protocol(message) => f.write_str(message),
            Self::JsonRpc { code, message } => match code {
                Some(code) => write!(f, "daemon error {code}: {message}"),
                None => f.write_str(message),
            },
        }
    }
}

impl std::error::Error for DaemonRegistrationError {}

fn request_jsonrpc(
    method: &str,
    params: &Value,
    id: &str,
    timeout: Duration,
) -> Result<Value, DaemonRegistrationError> {
    let body = jsonrpc_request_line(method, params, id);
    let response = round_trip(&body, timeout)?;
    parse_jsonrpc_response(&response, id)
}

fn jsonrpc_request_line(method: &str, params: &Value, id: &str) -> Vec<u8> {
    let frame = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": id,
    });
    let mut bytes = frame.to_string().into_bytes();
    bytes.push(b'\n');
    bytes
}

#[cfg(unix)]
fn round_trip(body: &[u8], timeout: Duration) -> Result<String, DaemonRegistrationError> {
    // `Write` is only needed for the Unix-socket transport; the Windows
    // named-pipe client (below) writes through an inherent method, so importing
    // it at module scope is an unused import on Windows (`-D warnings`).
    use std::io::Write;
    use std::os::unix::net::UnixStream;

    use anvil_intercept::ipc;

    let socket_path = match ipc::resolve_live_socket_path() {
        Ok(path) => path,
        Err(err) if ipc::live_socket_absent(&err) => {
            return Err(DaemonRegistrationError::DaemonUnavailable(format!(
                "anvil intercept daemon is not running ({err})",
            )));
        }
        Err(err) => return Err(DaemonRegistrationError::Transport(err.to_string())),
    };
    if let Err(err) = ipc::validate_socket_path_for_client(&socket_path) {
        return match err {
            ipc::IpcError::Io(io) if io.kind() == std::io::ErrorKind::NotFound => {
                Err(DaemonRegistrationError::DaemonUnavailable(format!(
                    "anvil intercept daemon is not running (no socket at {})",
                    socket_path.display()
                )))
            }
            other => Err(DaemonRegistrationError::Transport(format!(
                "anvil intercept daemon socket is unavailable: {other}"
            ))),
        };
    }

    let mut stream = UnixStream::connect(&socket_path).map_err(|err| {
        DaemonRegistrationError::Transport(format!(
            "failed to connect to intercept daemon socket {}: {err}",
            socket_path.display()
        ))
    })?;
    ipc::validate_connected_peer_for_client(&stream).map_err(|err| {
        DaemonRegistrationError::Transport(format!("daemon peer credentials rejected: {err}"))
    })?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|err| DaemonRegistrationError::Transport(err.to_string()))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|err| DaemonRegistrationError::Transport(err.to_string()))?;
    stream
        .write_all(body)
        .map_err(|err| DaemonRegistrationError::Transport(err.to_string()))?;
    stream
        .flush()
        .map_err(|err| DaemonRegistrationError::Transport(err.to_string()))?;
    read_one_line(stream)
}

#[cfg(windows)]
fn round_trip(body: &[u8], timeout: Duration) -> Result<String, DaemonRegistrationError> {
    use std::sync::mpsc;
    use std::thread;

    let pipe_name = anvil_intercept::ipc::resolve_pipe_name()
        .map_err(|err| DaemonRegistrationError::Transport(err.to_string()))?;
    let pipe_name_clone = pipe_name.clone();
    let body = body.to_vec();
    let (tx, rx) = mpsc::sync_channel::<Result<String, DaemonRegistrationError>>(1);
    thread::spawn(move || {
        let result = (|| {
            let mut client = anvil_intercept_win32::connect_owner_only_pipe_client(
                &pipe_name_clone,
            )
            .map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    DaemonRegistrationError::DaemonUnavailable(format!(
                        "anvil intercept daemon is not running (no pipe at {pipe_name_clone})"
                    ))
                } else {
                    DaemonRegistrationError::Transport(format!(
                        "failed to connect to intercept daemon pipe {pipe_name_clone}: {err}"
                    ))
                }
            })?;
            client
                .write_all(&body)
                .map_err(|err| DaemonRegistrationError::Transport(err.to_string()))?;
            read_one_line(client)
        })();
        let _ = tx.send(result);
    });
    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => Err(DaemonRegistrationError::Transport(format!(
            "timed out talking to the daemon on pipe {pipe_name}"
        ))),
    }
}

#[cfg(all(not(unix), not(windows)))]
fn round_trip(_body: &[u8], _timeout: Duration) -> Result<String, DaemonRegistrationError> {
    Err(DaemonRegistrationError::DaemonUnavailable(
        "intercept daemon IPC is not supported on this platform".to_owned(),
    ))
}

fn read_one_line<R: Read>(stream: R) -> Result<String, DaemonRegistrationError> {
    let mut reader = BufReader::new(stream);
    let mut buf = Vec::new();
    let read = reader
        .by_ref()
        .take(RESPONSE_LINE_BYTES + 1)
        .read_until(b'\n', &mut buf)
        .map_err(|err| DaemonRegistrationError::Transport(err.to_string()))?;
    if read == 0 {
        return Err(DaemonRegistrationError::Transport(
            "daemon closed the connection before responding".to_owned(),
        ));
    }
    // `read_until` appends the `\n` delimiter to `buf` when it found one;
    // exclude only that delimiter from the cap so a response of exactly
    // RESPONSE_LINE_BYTES content bytes is accepted, without under-counting a
    // (rare) response that hit the read limit before any newline.
    let content_len = if buf.last() == Some(&b'\n') {
        buf.len() - 1
    } else {
        buf.len()
    };
    if content_len as u64 > RESPONSE_LINE_BYTES {
        return Err(DaemonRegistrationError::Protocol(format!(
            "daemon response exceeded {RESPONSE_LINE_BYTES} byte cap"
        )));
    }
    let line = std::str::from_utf8(buf.trim_ascii_end())
        .map_err(|err| DaemonRegistrationError::Protocol(err.to_string()))?;
    Ok(line.to_owned())
}

fn parse_jsonrpc_response(
    response: &str,
    request_id: &str,
) -> Result<Value, DaemonRegistrationError> {
    let value: Value = serde_json::from_str(response)
        .map_err(|err| DaemonRegistrationError::Protocol(err.to_string()))?;
    if value.get("jsonrpc") != Some(&Value::String("2.0".to_owned())) {
        return Err(DaemonRegistrationError::Protocol(format!(
            "daemon response missing or wrong jsonrpc version: {value}"
        )));
    }
    if value.get("id") != Some(&Value::String(request_id.to_owned())) {
        return Err(DaemonRegistrationError::Protocol(format!(
            "daemon response id does not match request {request_id:?}: {value}"
        )));
    }
    if let Some(error) = value.get("error") {
        return Err(jsonrpc_error_from_value(error));
    }
    value.get("result").cloned().ok_or_else(|| {
        DaemonRegistrationError::Protocol(format!("daemon response missing result: {value}"))
    })
}

/// Map a JSON-RPC `error` object to a structured [`DaemonRegistrationError`].
///
/// The daemon flattens internal failures to a generic `-32603` envelope and
/// nests the specific registry detail under `error.data.error`, so we read
/// that field first (most specific), then `error.message`, and only fall back
/// to the whole object when neither is a string. Pulling out the focused field
/// is what lets [`DaemonRegistrationError::is_session_already_registered`]
/// match a stable marker rather than grepping the entire serialised envelope.
fn jsonrpc_error_from_value(error: &Value) -> DaemonRegistrationError {
    let code = error.get("code").and_then(Value::as_i64);
    let message = error
        .get("data")
        .and_then(|data| data.get("error"))
        .and_then(Value::as_str)
        .or_else(|| error.get("message").and_then(Value::as_str))
        .map_or_else(|| error.to_string(), str::to_owned);
    DaemonRegistrationError::JsonRpc { code, message }
}

/// ACTMO-015: outcome of an `anvil workspace unregister`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorktreeUnregistration {
    /// The durable registration was removed.
    Unregistered,
    /// The worktree was not registered — `unregister` is idempotent, so this
    /// is a benign success, not an error.
    NotRegistered,
    /// The daemon is not reachable.
    DaemonUnavailable,
    /// Any other rejection.
    Rejected(String),
}

/// ACTMO-015: unregister a worktree's durable membership. Derives the same
/// deterministic activation session id as `register`, so unregistering by path
/// hits the right entry. Idempotent: an unregistered worktree returns
/// [`WorktreeUnregistration::NotRegistered`].
pub(crate) fn unregister_worktree_with_daemon(worktree: &Path) -> WorktreeUnregistration {
    let canonical = canonicalise_for_registration(worktree);
    let session_id = activation_session_id(&canonical);
    let request_id = format!("anvil-workspace-unregister-{}", session_id.as_str());
    match request_jsonrpc(
        UNREGISTER_METHOD,
        &serde_json::json!({ "session_id": session_id.as_str() }),
        &request_id,
        ACTIVATION_DAEMON_QUERY_TIMEOUT,
    ) {
        Ok(result) => {
            // The daemon returns `{ "removed": <bool> }`; `false` means the id
            // was not present, which is the idempotent no-op case.
            let removed = result
                .get("removed")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if removed {
                WorktreeUnregistration::Unregistered
            } else {
                WorktreeUnregistration::NotRegistered
            }
        }
        Err(DaemonRegistrationError::DaemonUnavailable(_)) => {
            WorktreeUnregistration::DaemonUnavailable
        }
        Err(err) => WorktreeUnregistration::Rejected(err.to_string()),
    }
}

/// ACTMO-014/016: why a path is not a registerable Git worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NotRegisterable {
    /// A bare repository — no working tree to register.
    BareRepository,
    /// `cwd` is inside the `.git` directory, not the working tree.
    InsideGitDir,
    /// Not a Git worktree at all (or `git` is unavailable).
    NotAWorktree(String),
}

impl std::fmt::Display for NotRegisterable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BareRepository => {
                f.write_str("bare repositories have no working tree to register")
            }
            Self::InsideGitDir => {
                f.write_str("path is inside a .git directory, not a working tree")
            }
            Self::NotAWorktree(detail) => write!(f, "not a Git worktree: {detail}"),
        }
    }
}

/// ACTMO-014/016 (ADR-094 decision 4): resolve the registerable Git worktree
/// containing `start`, or explain why it is not registerable. Rejects bare
/// repositories and the `.git` internal directory; accepts ordinary, linked,
/// and submodule worktrees (where `.git` is a file pointer). Returns the
/// canonical top-level path that should be registered.
pub(crate) fn registerable_worktree(start: &Path) -> Result<PathBuf, NotRegisterable> {
    // The boolean probes do not require a working tree, so they succeed even
    // inside `.git/` — unlike `--show-toplevel`, which *fails* there and would
    // abort a combined `rev-parse` before the booleans could classify it.
    let probe = git_rev_parse(start, &["--is-bare-repository", "--is-inside-git-dir"])?;
    let mut lines = probe.lines();
    let is_bare = lines.next().map(str::trim) == Some("true");
    let is_inside_git_dir = lines.next().map(str::trim) == Some("true");
    if is_bare {
        return Err(NotRegisterable::BareRepository);
    }
    if is_inside_git_dir {
        return Err(NotRegisterable::InsideGitDir);
    }

    // A real (ordinary, linked, or submodule) worktree: resolve its top level.
    let toplevel = git_rev_parse(start, &["--show-toplevel"])?;
    let toplevel = toplevel.trim();
    if toplevel.is_empty() {
        return Err(NotRegisterable::NotAWorktree(
            "git did not report a working-tree top level".to_owned(),
        ));
    }
    Ok(canonicalise_for_registration(Path::new(toplevel)))
}

/// Run `git -C <start> rev-parse <args>` and return stdout, mapping any
/// failure (git missing, not a repo) to [`NotRegisterable::NotAWorktree`].
fn git_rev_parse(start: &Path, args: &[&str]) -> Result<String, NotRegisterable> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(start)
        .arg("rev-parse")
        .args(args)
        .output()
        .map_err(|err| NotRegisterable::NotAWorktree(format!("failed to run git: {err}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(NotRegisterable::NotAWorktree(if stderr.is_empty() {
            format!("git rev-parse failed with status {}", output.status)
        } else {
            stderr
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

// --------------------------------------------------------------------
// JREL-002 / ADR-141: live MCP client sessions.
// --------------------------------------------------------------------

/// `driver_id` for sessions registered by `anvil mcp serve --stdio`.
///
/// Distinct from `anvil-start` (durable activation membership) and
/// `anvil-run` (launcher-owned agent leases) so operators can tell at a
/// glance which lane produced a surface.
pub(crate) const MCP_SESSION_DRIVER_ID: &str = "anvil-mcp";

/// Register a **live** MCP client session with the daemon.
///
/// ADR-141: the tag's `claimed_agent_id` is the client's stable label
/// (`claude-code`, `cursor`, …), so the surface identifier reads
/// `anvil-mcp/<client>#<starttime>` and `daemon_evidence` attributes it
/// to exactly that client. It is never `activation-spine`, so the record
/// is a live lease that must keep beating — which is precisely what makes
/// it evidence that a client is attached.
///
/// Returns `false` when the daemon is unavailable or refuses; MCP serve
/// treats registration as best-effort and never fails a session over it.
pub(crate) fn register_live_mcp_session(
    session_id: &SessionId,
    worktree: &Path,
    client_label: &str,
    pid_starttime: u64,
) -> bool {
    let canonical = canonicalise_for_registration(worktree);
    let tag = AgentTag::new(MCP_SESSION_DRIVER_ID, client_label, pid_starttime);
    let params = serde_json::json!({
        "session_id": session_id.as_str(),
        "worktree": canonical.to_string_lossy(),
        "agent_tag": tag,
    });
    let request_id = format!("anvil-mcp-register-{}", session_id.as_str());
    match request_jsonrpc(
        REGISTER_METHOD,
        &params,
        &request_id,
        ACTIVATION_DAEMON_QUERY_TIMEOUT,
    ) {
        Ok(_) => {
            tracing::debug!(
                worktree = %canonical.display(),
                session_id = session_id.as_str(),
                client = client_label,
                "mcp: registered live client session with intercept daemon",
            );
            true
        }
        Err(err) => match classify_live_registration_refusal(&err) {
            // Our own id is already registered — a re-register from this
            // same process. Heartbeating it refreshes the record we own.
            LiveRegistrationRefusal::OwnSessionExists => heartbeat_live_mcp_session(session_id),
            // A record with this exact tag exists under a *different*
            // session id. Unlike activation — whose id is derived
            // deterministically from the worktree path, so the "existing
            // owner" is always the same id — an MCP session id carries
            // this process's pid, so the owner the daemon names is not
            // us. Heartbeating our own unregistered id would fail
            // silently while we reported success, and adopting theirs
            // would let our exit unregister another process's evidence.
            // Decline instead: their live record already attests this
            // client.
            LiveRegistrationRefusal::OwnedByAnotherSession => {
                tracing::debug!(
                    worktree = %canonical.display(),
                    client = client_label,
                    "mcp: an equally-tagged live session already attests this client; not duplicating it",
                );
                false
            }
            LiveRegistrationRefusal::Unavailable => {
                tracing::debug!(
                    worktree = %canonical.display(),
                    client = client_label,
                    error = %err,
                    "mcp: live client session registration unavailable",
                );
                false
            }
        },
    }
}

/// How a refused live-session registration should be handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiveRegistrationRefusal {
    /// The daemon already holds *this* session id.
    OwnSessionExists,
    /// The daemon holds an equally-tagged session under another id.
    OwnedByAnotherSession,
    /// Daemon unreachable, fenced, capped, or any other refusal.
    Unavailable,
}

fn classify_live_registration_refusal(err: &DaemonRegistrationError) -> LiveRegistrationRefusal {
    if err.is_session_already_registered() {
        LiveRegistrationRefusal::OwnSessionExists
    } else if err.is_worktree_already_owned() {
        LiveRegistrationRefusal::OwnedByAnotherSession
    } else {
        LiveRegistrationRefusal::Unavailable
    }
}

/// Refresh a live MCP session's TTL. Best-effort: a missed beat costs the
/// session its liveness after the registry TTL, which is the honest
/// outcome when the daemon is unreachable.
pub(crate) fn heartbeat_live_mcp_session(session_id: &SessionId) -> bool {
    let request_id = format!("anvil-mcp-heartbeat-{}", session_id.as_str());
    request_jsonrpc(
        HEARTBEAT_METHOD,
        &serde_json::json!({ "session_id": session_id.as_str() }),
        &request_id,
        ACTIVATION_DAEMON_QUERY_TIMEOUT,
    )
    .is_ok()
}

/// Drop a live MCP session on clean shutdown so a closed editor stops
/// attesting immediately rather than waiting out the registry TTL.
pub(crate) fn unregister_live_mcp_session(session_id: &SessionId) {
    let request_id = format!("anvil-mcp-unregister-{}", session_id.as_str());
    let _ = request_jsonrpc(
        UNREGISTER_METHOD,
        &serde_json::json!({ "session_id": session_id.as_str() }),
        &request_id,
        ACTIVATION_DAEMON_QUERY_TIMEOUT,
    );
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anvil_intercept_proto::{SessionRecord, SessionStatus};

    use super::*;

    fn durable_session(id: SessionId, worktree: &Path) -> SessionRecord {
        SessionRecord {
            id,
            worktree: worktree.to_path_buf(),
            pid: None,
            pgid: None,
            started_at_unix: 1,
            last_heartbeat_unix: 1,
            status: SessionStatus::Active,
            agent_tag: Some(activation_agent_tag()),
            daemon_issued_tag: None,
        }
    }

    /// JREL-002 (Copilot review on #4416): the two "already registered"
    /// refusals are not interchangeable for a live MCP session.
    ///
    /// Activation derives its session id deterministically from the
    /// worktree path, so `worktree already owned` there always names the
    /// same id and heartbeating our own id is correct. An MCP session id
    /// carries this process's pid, so the same refusal names a *different*
    /// process. Heartbeating our own unregistered id would fail silently
    /// while registration reported success, and adopting theirs would let
    /// our exit unregister another process's evidence.
    #[test]
    fn live_registration_refusals_distinguish_our_session_from_another_owner() {
        let ours = DaemonRegistrationError::JsonRpc {
            code: None,
            message: format!("{SESSION_ALREADY_REGISTERED_MARKER}: sess_mcp_claude_code_1_2"),
        };
        assert_eq!(
            classify_live_registration_refusal(&ours),
            LiveRegistrationRefusal::OwnSessionExists,
        );

        let theirs = DaemonRegistrationError::JsonRpc {
            code: None,
            message: format!("{WORKTREE_ALREADY_OWNED_MARKER} by session \"sess_other\""),
        };
        assert_eq!(
            classify_live_registration_refusal(&theirs),
            LiveRegistrationRefusal::OwnedByAnotherSession,
            "an equally-tagged session under another id must not be claimed as ours",
        );

        let down = DaemonRegistrationError::DaemonUnavailable("not running".into());
        assert_eq!(
            classify_live_registration_refusal(&down),
            LiveRegistrationRefusal::Unavailable,
        );
    }

    #[test]
    fn activation_session_id_is_stable_and_path_derived() {
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let other_dir = tempfile::tempdir().expect("other worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());
        let other = canonicalise_for_registration(other_dir.path());
        let first = activation_session_id(&worktree);
        let second = activation_session_id(&worktree);
        let different = activation_session_id(&other);

        assert_eq!(first, second);
        assert_ne!(first, different);
        assert!(first.as_str().starts_with("sess_activation_"));
    }

    #[test]
    fn register_params_use_activation_identity_without_lineage() {
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());
        let session_id = activation_session_id(&worktree);
        let params = session_register_params(&session_id, &worktree);

        assert_eq!(params["worktree"], worktree.display().to_string());
        assert_eq!(params["agent_tag"]["driver_id"], "anvil-start");
        assert_eq!(params["agent_tag"]["claimed_agent_id"], "activation-spine");
        assert!(
            params.get("lineage").is_none(),
            "activation registration must not require peer lineage support"
        );
    }

    #[test]
    fn acknowledged_registration_without_durable_membership_is_rejected() {
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());

        let outcome = confirm_durable_registration(&worktree, &[]);

        assert!(
            matches!(outcome, WorktreeRegistration::Rejected(ref message)
                if message.contains("durable membership")
                    && message.contains(&worktree.display().to_string())),
            "an acknowledged registration must not report success when daemon status has no durable membership: {outcome:?}",
        );
    }

    #[test]
    fn membership_visible_only_after_the_first_read_is_still_confirmed() {
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());
        let session_id = activation_session_id(&worktree);
        let session = durable_session(session_id.clone(), &worktree);

        let mut reads = 0_usize;
        let snapshot = await_membership_snapshot_within(
            &worktree,
            &session_id,
            Duration::from_secs(5),
            Duration::from_millis(1),
            || -> Result<Vec<SessionRecord>, std::convert::Infallible> {
                reads += 1;
                if reads < 3 {
                    Ok(Vec::new())
                } else {
                    Ok(vec![session.clone()])
                }
            },
        )
        .expect("status reads succeed");

        assert_eq!(
            reads, 3,
            "the wait must re-read daemon status rather than trust the first snapshot",
        );
        assert_eq!(
            confirm_durable_registration(&worktree, &snapshot),
            WorktreeRegistration::Registered,
            "a membership that lands after the acknowledgement must still confirm",
        );
    }

    #[test]
    fn membership_already_present_is_confirmed_without_a_second_read() {
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());
        let session_id = activation_session_id(&worktree);
        let session = durable_session(session_id.clone(), &worktree);

        let mut reads = 0_usize;
        let started = Instant::now();
        let snapshot = await_membership_snapshot_within(
            &worktree,
            &session_id,
            Duration::from_secs(30),
            Duration::from_secs(30),
            || -> Result<Vec<SessionRecord>, std::convert::Infallible> {
                reads += 1;
                Ok(vec![session.clone()])
            },
        )
        .expect("status reads succeed");

        assert_eq!(reads, 1, "the healthy path must not poll a second time");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the healthy path must not pay the wait budget",
        );
        assert_eq!(
            confirm_durable_registration(&worktree, &snapshot),
            WorktreeRegistration::Registered,
        );
    }

    #[test]
    fn membership_that_never_appears_still_refuses_within_the_budget() {
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());
        let session_id = activation_session_id(&worktree);

        let mut reads = 0_usize;
        let started = Instant::now();
        let snapshot = await_membership_snapshot_within(
            &worktree,
            &session_id,
            Duration::from_millis(120),
            Duration::from_millis(20),
            || -> Result<Vec<SessionRecord>, std::convert::Infallible> {
                reads += 1;
                Ok(Vec::new())
            },
        )
        .expect("status reads succeed");
        let elapsed = started.elapsed();

        assert!(reads > 1, "an absent membership must be re-checked");
        assert!(
            elapsed >= Duration::from_millis(100),
            "the wait must actually span the budget, got {elapsed:?}",
        );
        assert!(
            elapsed < Duration::from_secs(10),
            "the wait must stay bounded, got {elapsed:?}",
        );
        assert_eq!(
            confirm_durable_registration(&worktree, &snapshot),
            missing_durable_membership(&worktree),
            "CIB-252: a membership that never lands must still refuse honestly",
        );
    }

    #[test]
    fn repeated_status_read_failures_surface_the_last_error() {
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());
        let session_id = activation_session_id(&worktree);

        let error = await_membership_snapshot_within(
            &worktree,
            &session_id,
            Duration::from_millis(60),
            Duration::from_millis(20),
            || -> Result<Vec<SessionRecord>, String> { Err("daemon status unreadable".to_owned()) },
        )
        .expect_err("every read failed");

        assert_eq!(error, "daemon status unreadable");
    }

    #[test]
    fn refreshed_registration_without_durable_membership_is_rejected() {
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());

        let outcome = confirm_durable_refresh(&worktree, &[]);

        assert!(
            matches!(outcome, WorktreeRegistration::Rejected(ref message)
                if message.contains("workspace request")
                    && message.contains("durable membership")
                    && message.contains(&worktree.display().to_string())),
            "a successful heartbeat must not report refresh when daemon status has no durable membership: {outcome:?}",
        );
    }

    #[test]
    fn durable_membership_for_a_different_session_does_not_confirm_registration() {
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());
        let expected = activation_session_id(&worktree);
        let other = durable_session(SessionId::new("sess_activation_other"), &worktree);

        let outcome = confirm_registration_membership(
            &worktree,
            &expected,
            &[other],
            WorktreeRegistration::Registered,
        );

        assert!(matches!(outcome, WorktreeRegistration::Rejected(_)));
    }

    #[test]
    fn deterministic_session_for_a_different_canonical_worktree_is_rejected() {
        let expected_dir = tempfile::tempdir().expect("expected worktree tempdir");
        let other_dir = tempfile::tempdir().expect("other worktree tempdir");
        let expected_worktree = canonicalise_for_registration(expected_dir.path());
        let other_worktree = canonicalise_for_registration(other_dir.path());
        let expected_session = activation_session_id(&expected_worktree);
        let wrong_path_session = durable_session(expected_session.clone(), &other_worktree);

        let outcome = confirm_registration_membership(
            &expected_worktree,
            &expected_session,
            &[wrong_path_session],
            WorktreeRegistration::Registered,
        );

        assert!(matches!(outcome, WorktreeRegistration::Rejected(_)));
    }

    #[test]
    fn matching_session_and_canonical_worktree_without_durable_tag_is_rejected() {
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());
        let expected_session = activation_session_id(&worktree);
        let mut live_session = durable_session(expected_session.clone(), &worktree);
        live_session.agent_tag = Some(AgentTag::new("anvil-start", "live-agent", 1));

        let outcome = confirm_registration_membership(
            &worktree,
            &expected_session,
            &[live_session],
            WorktreeRegistration::Registered,
        );

        assert!(matches!(outcome, WorktreeRegistration::Rejected(_)));
    }

    #[test]
    fn matching_durable_membership_preserves_registered_and_refreshed_outcomes() {
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());
        let session = durable_session(activation_session_id(&worktree), &worktree);

        assert_eq!(
            confirm_durable_registration(&worktree, std::slice::from_ref(&session)),
            WorktreeRegistration::Registered,
        );
        assert_eq!(
            confirm_durable_refresh(&worktree, std::slice::from_ref(&session)),
            WorktreeRegistration::Refreshed,
        );
    }

    #[test]
    fn parse_detects_session_already_registered_from_nested_daemon_envelope() {
        // The shape the daemon actually emits: a generic -32603 with the
        // registry detail nested under error.data.error (ipc.rs).
        let response = r#"{"jsonrpc":"2.0","id":"req-1","error":{"code":-32603,"message":"Internal error","data":{"error":"session already registered: SessionId(\"sess_activation_abcd\")"}}}"#;
        let err = parse_jsonrpc_response(response, "req-1").unwrap_err();
        assert!(
            err.is_session_already_registered(),
            "nested registry detail must be detected: {err}"
        );
    }

    #[test]
    fn session_already_registered_detection_is_case_insensitive_and_field_scoped() {
        // Casing/whitespace drift in the message must still match.
        let upper = DaemonRegistrationError::JsonRpc {
            code: Some(-32603),
            message: "  SESSION Already Registered: sess_x".to_owned(),
        };
        assert!(upper.is_session_already_registered());

        // An unrelated error must not be mistaken for it.
        let other = DaemonRegistrationError::JsonRpc {
            code: Some(-32602),
            message: "invalid params: worktree".to_owned(),
        };
        assert!(!other.is_session_already_registered());

        // Transport/daemon-unavailable errors are never this signal.
        assert!(
            !DaemonRegistrationError::DaemonUnavailable("no socket".to_owned())
                .is_session_already_registered()
        );
    }

    #[test]
    fn marker_pins_registry_session_already_exists_display() {
        // Cross-crate guard (Council S1): the heartbeat fall-through depends on
        // the daemon's registry wording. If RegistryError::SessionAlreadyExists
        // is ever rephrased, this fails CI instead of silently breaking the
        // re-run heartbeat (which would let the session lapse on its TTL).
        let display = anvil_intercept::registry::RegistryError::SessionAlreadyExists(
            anvil_intercept_proto::SessionId::new("sess_activation_abcd"),
        )
        .to_string();
        assert!(
            display
                .to_ascii_lowercase()
                .contains(SESSION_ALREADY_REGISTERED_MARKER),
            "registry wording drifted from the activation marker: {display:?}"
        );
    }

    /// ACTMO-014 cross-crate guard: every classification marker must remain a
    /// substring of the `RegistryError` Display wording the client classifies
    /// on. A rephrase in the registry fails here rather than silently breaking
    /// the CLI's fenced / cap / already-owned outcome mapping.
    #[test]
    fn markers_pin_registry_error_wording() {
        use anvil_intercept::registry::RegistryError;
        use anvil_intercept_proto::SessionId;

        let cases: Vec<(String, &str)> = vec![
            (
                RegistryError::WorktreeAlreadyOwned {
                    existing: SessionId::new("sess_x"),
                }
                .to_string(),
                WORKTREE_ALREADY_OWNED_MARKER,
            ),
            (
                RegistryError::WorktreeFenced {
                    worktree: "/tmp/wt".into(),
                }
                .to_string(),
                WORKTREE_FENCED_MARKER,
            ),
            (
                RegistryError::WorktreeCascaded {
                    worktree: "/tmp/wt".into(),
                }
                .to_string(),
                WORKTREE_CASCADED_MARKER,
            ),
            (
                RegistryError::SessionCapExceeded {
                    worktree: "/tmp/wt".into(),
                    cap: 16,
                    live: 16,
                }
                .to_string(),
                SESSION_CAP_MARKER,
            ),
            (
                RegistryError::RegisteredWorktreeCapExceeded { cap: 64, live: 64 }.to_string(),
                REGISTERED_CAP_MARKER,
            ),
        ];
        for (display, marker) in cases {
            assert!(
                display.to_ascii_lowercase().contains(marker),
                "registry wording {display:?} drifted from marker {marker:?}",
            );
        }
    }

    /// ACTMO-014: the client maps each pinned marker to the right outcome
    /// classification.
    #[test]
    fn daemon_error_classification_routes_each_refusal() {
        let owned = DaemonRegistrationError::JsonRpc {
            code: Some(-32603),
            message: "worktree already owned by session SessionId(\"sess_x\")".to_owned(),
        };
        assert!(owned.is_worktree_already_owned());
        assert!(!owned.is_fenced());

        let fenced = DaemonRegistrationError::JsonRpc {
            code: Some(-32603),
            message: "worktree is fenced until explicit unblock: \"/tmp/wt\"".to_owned(),
        };
        assert!(fenced.is_fenced());
        assert!(!fenced.is_cap_exceeded());

        let cascaded = DaemonRegistrationError::JsonRpc {
            code: Some(-32603),
            message:
                "worktree is in degraded fence-cascade mode and refuses new sessions: \"/tmp/wt\""
                    .to_owned(),
        };
        assert!(cascaded.is_fenced());

        let registered_cap = DaemonRegistrationError::JsonRpc {
            code: Some(-32603),
            message: "registered worktree cap exceeded: 64 registered at cap=64".to_owned(),
        };
        assert!(registered_cap.is_cap_exceeded());
        assert!(!registered_cap.is_fenced());

        // Transport-class errors never carry a registry classification.
        let unavailable = DaemonRegistrationError::DaemonUnavailable("no socket".to_owned());
        assert!(!unavailable.is_fenced());
        assert!(!unavailable.is_cap_exceeded());
        assert!(!unavailable.is_worktree_already_owned());
    }

    /// ACTMO-014 (adversarial review F3): a worktree PATH containing a marker
    /// substring must not misclassify an unrelated error. A cap-exceeded
    /// message for a path containing "is fenced" / "fence-cascade mode" stays
    /// classified as a cap breach, because the markers are anchored to the
    /// error's static prefix words, not a bare substring.
    fn cap_error_for(path: &str) -> DaemonRegistrationError {
        DaemonRegistrationError::JsonRpc {
            code: Some(-32603),
            message: format!(
                "worktree session cap exceeded for {path:?}: 16 live sessions at cap=16"
            ),
        }
    }

    #[test]
    fn path_substring_does_not_misclassify() {
        let pathological = cap_error_for("/home/alice/is fenced/fence-cascade mode/project");
        assert!(
            pathological.is_cap_exceeded(),
            "the cap error must classify as cap-exceeded",
        );
        assert!(
            !pathological.is_fenced(),
            "a path containing 'is fenced'/'fence-cascade mode' must not classify as fenced",
        );
    }

    /// ACTMO-014/016: a real Git worktree resolves to its canonical top level;
    /// a path inside `.git` is rejected; a non-repo is rejected.
    #[test]
    fn registerable_worktree_resolves_real_worktree_and_rejects_git_internals() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = |args: &[&str]| {
            let ok = std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .output()
                .expect("git")
                .status
                .success();
            assert!(ok, "git {args:?} failed");
        };
        run(&["init", "-q"]);
        run(&["config", "user.email", "t@t"]);
        run(&["config", "user.name", "t"]);

        let toplevel = registerable_worktree(dir.path()).expect("worktree is registerable");
        assert_eq!(
            toplevel,
            dunce::canonicalize(dir.path()).expect("canonical")
        );

        // Inside the .git directory is rejected.
        let err =
            registerable_worktree(&dir.path().join(".git")).expect_err("inside .git rejected");
        assert_eq!(err, NotRegisterable::InsideGitDir);

        // A non-repo directory is rejected.
        let outside = tempfile::tempdir().expect("non-repo");
        assert!(matches!(
            registerable_worktree(outside.path()),
            Err(NotRegisterable::NotAWorktree(_))
        ));
    }

    // ── JREL-003: bounded save-time driver readiness after registration ──

    fn status_with_driver(
        worktree: &Path,
        driver: SaveTimeDriverStatusV1,
        evidence: Option<SaveTimeDriverEvidenceV1>,
    ) -> DaemonStatusV1 {
        use anvil_intercept_proto::status::{
            HealthStateV1, IpcStateV1, LatencyMidEditMapV1, WorktreeStatusV1,
        };
        DaemonStatusV1 {
            sessions: vec![],
            worktrees: vec![WorktreeStatusV1 {
                worktree: worktree.to_path_buf(),
                session_id: activation_session_id(worktree),
                fenced: false,
                cascaded: false,
                cascade_since: None,
                save_time_driver: driver,
                save_time_driver_evidence: evidence,
            }],
            fences: vec![],
            health: HealthStateV1 {
                uptime_seconds: 1,
                version: "test".to_owned(),
                ipc_state: IpcStateV1::Serving,
            },
            latency: LatencyMidEditMapV1 { mid_edit: None },
            cache_entries: None,
            cache_invalidations_total: None,
            in_flight_evaluations: None,
            cache_invalidations_rate_limited: None,
            telemetry_subscriber_count: None,
            telemetry_dropped_envelopes: None,
            generated_at_unix: 0,
        }
    }

    #[test]
    fn driver_wait_returns_on_the_first_attached_snapshot_with_evidence() {
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());
        let mut reads = 0_usize;
        let started = Instant::now();
        let readiness = await_save_time_driver_within(
            &worktree,
            Duration::from_secs(30),
            Duration::from_secs(30),
            || -> Result<DaemonStatusV1, std::convert::Infallible> {
                reads += 1;
                Ok(status_with_driver(
                    &worktree,
                    SaveTimeDriverStatusV1::Attached,
                    Some(SaveTimeDriverEvidenceV1::WatchesInstalled),
                ))
            },
        );
        assert_eq!(reads, 1, "the healthy path must not poll a second time");
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(
            readiness,
            SaveTimeDriverReadiness::Attached {
                evidence: Some(SaveTimeDriverEvidenceV1::WatchesInstalled),
            }
        );
    }

    #[test]
    fn driver_wait_outlasts_a_dead_child_snapshot_until_the_respawn_lands() {
        // After a refresh the first read still shows the dead child as
        // `failed`; the wait must keep reading until the supervisor's respawn
        // is visible rather than reporting the stale failure.
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());
        let mut reads = 0_usize;
        let readiness = await_save_time_driver_within(
            &worktree,
            Duration::from_secs(5),
            Duration::from_millis(5),
            || -> Result<DaemonStatusV1, std::convert::Infallible> {
                reads += 1;
                Ok(if reads < 3 {
                    status_with_driver(&worktree, SaveTimeDriverStatusV1::Failed, None)
                } else {
                    status_with_driver(
                        &worktree,
                        SaveTimeDriverStatusV1::Attached,
                        Some(SaveTimeDriverEvidenceV1::Spawned),
                    )
                })
            },
        );
        assert_eq!(reads, 3);
        assert_eq!(
            readiness,
            SaveTimeDriverReadiness::Attached {
                evidence: Some(SaveTimeDriverEvidenceV1::Spawned),
            }
        );
    }

    #[test]
    fn driver_wait_reports_failed_absent_and_unknown_within_the_budget() {
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());
        let budget = Duration::from_millis(120);
        let interval = Duration::from_millis(20);

        let started = Instant::now();
        let failed = await_save_time_driver_within(
            &worktree,
            budget,
            interval,
            || -> Result<DaemonStatusV1, std::convert::Infallible> {
                Ok(status_with_driver(
                    &worktree,
                    SaveTimeDriverStatusV1::Failed,
                    None,
                ))
            },
        );
        let elapsed = started.elapsed();
        assert_eq!(failed, SaveTimeDriverReadiness::Failed);
        assert!(
            elapsed < Duration::from_secs(10),
            "a persistent failure is reported within the bounded wait, got {elapsed:?}"
        );

        // Membership refresh alone is not readiness: an absent driver entry
        // (supervision disabled) never reads as attached.
        let absent = await_save_time_driver_within(
            &worktree,
            budget,
            interval,
            || -> Result<DaemonStatusV1, std::convert::Infallible> {
                Ok(status_with_driver(
                    &worktree,
                    SaveTimeDriverStatusV1::Absent,
                    None,
                ))
            },
        );
        assert_eq!(absent, SaveTimeDriverReadiness::Absent);

        let unknown_tag = await_save_time_driver_within(
            &worktree,
            budget,
            interval,
            || -> Result<DaemonStatusV1, std::convert::Infallible> {
                Ok(status_with_driver(
                    &worktree,
                    SaveTimeDriverStatusV1::Unknown,
                    None,
                ))
            },
        );
        assert_eq!(
            unknown_tag,
            SaveTimeDriverReadiness::Absent,
            "a forward-compat unknown state is treated fail-safe as absent"
        );

        let unreadable = await_save_time_driver_within(
            &worktree,
            budget,
            interval,
            || -> Result<DaemonStatusV1, String> { Err("daemon status unreadable".to_owned()) },
        );
        assert_eq!(
            unreadable,
            SaveTimeDriverReadiness::Unknown("daemon status unreadable".to_owned())
        );
    }

    #[test]
    fn driver_wait_short_circuits_absent_but_retries_failed() {
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());
        let budget = Duration::from_secs(30);
        let interval = Duration::from_millis(10);

        let started = Instant::now();
        let absent = await_save_time_driver_within(
            &worktree,
            budget,
            interval,
            || -> Result<DaemonStatusV1, std::convert::Infallible> {
                Ok(status_with_driver(
                    &worktree,
                    SaveTimeDriverStatusV1::Absent,
                    None,
                ))
            },
        );
        let elapsed = started.elapsed();
        assert_eq!(absent, SaveTimeDriverReadiness::Absent);
        assert!(
            elapsed >= DRIVER_TRANSIENT_GRACE,
            "absent keeps its grace, got {elapsed:?}"
        );
        assert!(
            elapsed < budget / 2,
            "absent must not poll out the budget, got {elapsed:?}"
        );

        let mut remaining_failed = 3_usize;
        let failed_then_attached = await_save_time_driver_within(
            &worktree,
            Duration::from_millis(200),
            Duration::from_millis(10),
            || -> Result<DaemonStatusV1, std::convert::Infallible> {
                if remaining_failed > 0 {
                    remaining_failed -= 1;
                    Ok(status_with_driver(
                        &worktree,
                        SaveTimeDriverStatusV1::Failed,
                        None,
                    ))
                } else {
                    Ok(status_with_driver(
                        &worktree,
                        SaveTimeDriverStatusV1::Attached,
                        None,
                    ))
                }
            },
        );
        assert!(
            matches!(
                failed_then_attached,
                SaveTimeDriverReadiness::Attached { .. }
            ),
            "a healthy respawn after failed snapshots must attach, got {failed_then_attached:?}"
        );
    }

    #[test]
    fn driver_wait_retries_an_unreadable_status() {
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());
        let mut reads = 0_usize;
        let readiness = await_save_time_driver_within(
            &worktree,
            Duration::from_millis(80),
            Duration::from_millis(10),
            || -> Result<DaemonStatusV1, String> {
                reads += 1;
                Err("daemon status unreadable".to_owned())
            },
        );
        assert!(
            reads > 1,
            "an unreadable snapshot must be retried, got {reads} reads"
        );
        assert_eq!(
            readiness,
            SaveTimeDriverReadiness::Unknown("daemon status unreadable".to_owned())
        );
    }

    #[test]
    fn driver_wait_matches_the_worktree_on_canonical_identity() {
        let worktree_dir = tempfile::tempdir().expect("worktree tempdir");
        let worktree = canonicalise_for_registration(worktree_dir.path());
        let other = tempfile::tempdir().expect("other worktree");
        let other = canonicalise_for_registration(other.path());
        let readiness = await_save_time_driver_within(
            &worktree,
            Duration::from_millis(40),
            Duration::from_millis(10),
            || -> Result<DaemonStatusV1, std::convert::Infallible> {
                Ok(status_with_driver(
                    &other,
                    SaveTimeDriverStatusV1::Attached,
                    Some(SaveTimeDriverEvidenceV1::FreshActivity),
                ))
            },
        );
        assert_eq!(
            readiness,
            SaveTimeDriverReadiness::Absent,
            "another worktree's driver is never borrowed as this one's evidence"
        );
    }

    #[test]
    fn registration_report_names_only_the_failed_driver() {
        let failed = WorktreeRegistrationReport {
            registration: WorktreeRegistration::Refreshed,
            driver: Some(SaveTimeDriverReadiness::Failed),
        };
        assert!(failed.driver_failed());
        for driver in [
            None,
            Some(SaveTimeDriverReadiness::Absent),
            Some(SaveTimeDriverReadiness::Attached { evidence: None }),
            Some(SaveTimeDriverReadiness::Unknown("x".to_owned())),
        ] {
            let report = WorktreeRegistrationReport {
                registration: WorktreeRegistration::Registered,
                driver,
            };
            assert!(!report.driver_failed(), "{report:?}");
        }
    }
}
