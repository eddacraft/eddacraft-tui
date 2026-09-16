//! KDS-001 / PORT-011: `KindlingObservationSink` backed by the Kindling
//! daemon (with local spool fallback).

use std::ffi::OsStr;
use std::path::PathBuf;

use anvil_intercept::kindling_observation::{
    CommandInvokedObservation, GateEvaluatedObservation, KindlingObservationSink, KindlingSinkError,
};
use anyhow::Context as _;
use kindling_client::spool::{AppendOutcome, SpoolConfig, SpoolError, SpooledClient};
use kindling_client::{Client, ClientConfig, ObservationInput, ObservationKind, ScopeIds, Spawner};
use serde_json::{Map, Value};
use tokio::runtime::Runtime;

/// KDS-005: rolling retention caps on the emit spool, matching the NDJSON
/// sidecar the spool replaces (`USAGE_SIDECAR_MAX_BYTES` / `_MAX_AGE` in
/// `usage.rs`, council T5). With the bespoke writer retired the spool is the
/// only durable buffer, so it must be bounded — otherwise a prolonged daemon
/// outage grows it without limit.
const SPOOL_MAX_BYTES: u64 = 64 * 1024 * 1024;
/// 7 days in milliseconds (the spool cap is age-in-ms; `from_days` is unstable
/// on Rust 1.95, so compute from a seconds constant).
const SPOOL_MAX_AGE_MS: i64 = {
    const DAY_SECS: i64 = 86_400;
    7 * DAY_SECS * 1000
};

/// The Anvil observation-contract version stamped into Kindling provenance as
/// `anvil_contract_version`. Mirrors `OBSERVATION_CONTRACT_VERSION` in the TS
/// adapter (`packages/kindling-integration/src/adapter.ts`); bump in lock-step
/// with that contract.
pub const OBSERVATION_CONTRACT_VERSION: &str = "1.0.0";

/// Spool filename under `<credentials_dir>/kindling/`. Deliberately **not**
/// `usage.ndjson` (the legacy sidecar KDS-005 retires) — the spool is a
/// transient at-least-once write buffer drained into the daemon, not a parallel
/// source of truth.
const SPOOL_NDJSON: &str = "spool.ndjson";

/// Default path to the Kindling emit spool under the user-scoped state dir,
/// resolving the same `credentials_dir` (honouring a gated `ANVIL_HOME`) as the
/// NDJSON sidecar so a single deployment keeps one private `kindling/` dir.
pub fn default_spool_path() -> anyhow::Result<PathBuf> {
    let dir =
        crate::auth::credentials::credentials_dir().context("resolve kindling spool directory")?;
    Ok(dir.join("kindling").join(SPOOL_NDJSON))
}

/// Map a `command.invoked` Anvil observation to a Kindling [`ObservationInput`].
///
/// Mirrors the TS adapter's kind map (`command.invoked` → `Command`) and
/// provenance stamping. TRACE-003 redaction is already applied to `obs` before
/// the sink receives it; Kindling adds its own non-bypassable secret masking at
/// the service boundary. `id` is left `None` so the [`SpooledClient`] assigns a
/// stable v4 id before any spool (idempotent replay); `ts` carries the
/// observation's RFC 3339 timestamp as epoch-ms when parseable, else `None`
/// (the daemon assigns one).
pub(crate) fn to_kindling_input(
    obs: &CommandInvokedObservation,
    repo_id: Option<&str>,
) -> ObservationInput {
    // The observation is plain data with infallible serialisation; an error
    // here would be a serde bug, not runtime input, so expect is appropriate.
    let content = serde_json::to_string(obs).expect("CommandInvokedObservation serialises to JSON");

    let mut provenance = Map::new();
    provenance.insert("anvil_kind".to_string(), Value::String(obs.kind.clone()));
    provenance.insert(
        "anvil_contract_version".to_string(),
        Value::String(OBSERVATION_CONTRACT_VERSION.to_string()),
    );

    ObservationInput {
        id: None,
        kind: ObservationKind::Command,
        content,
        provenance: Some(provenance),
        ts: rfc3339_to_epoch_ms(&obs.timestamp),
        scope_ids: ScopeIds {
            session_id: Some(obs.session_id.clone()),
            repo_id: repo_id.map(str::to_string),
            ..Default::default()
        },
        redacted: None,
    }
}

/// Parse an RFC 3339 timestamp to epoch milliseconds, or `None` if it does not
/// parse (the daemon then stamps its own `ts`).
fn rfc3339_to_epoch_ms(timestamp: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

/// CIB-381: neutralise the upstream auto-spawn path when no `kindling` binary
/// can be resolved on `path_var`, returning whether the guard engaged.
///
/// KDS-005 made the daemon the default sink. On a host where `kindling` was
/// never installed every emit takes the daemon path, finds no socket, and asks
/// `kindling-client` to spawn the daemon; the spawn fails `ENOENT` and the
/// client appends one line to `~/.kindling/spawn.log` — a bare append with no
/// size, age, rotation, or dedup bound. One line per emit, forever (measured:
/// 5.01 MB / 59,259 identical lines on a dev host).
///
/// The unbounded writer is upstream (`kindling_client::config::append_spawn_log`
/// in 0.3.0) and a cap there is a separate follow-up. Anvil's interim is to not
/// reach it: when the binary is absent the spawn can only ever fail, so
///
/// - the spawner is replaced with one that never execs (skipping the attempt,
///   and the per-emit `fork`/`exec` it costs), and
/// - the spawn-log path is pointed at the platform null device, so the
///   diagnostic upstream insists on writing is discarded rather than
///   accumulating. It is redundant here: the absence is noted once, as a trace
///   event, when the sink is built.
///
/// Everything else is unchanged: the daemon sink stays the default, an already
/// running daemon still connects on the fast path (the spawner is only consulted
/// when no socket answers), and rows still buffer to the capped spool.
fn guard_absent_kindling_binary(config: &mut ClientConfig, path_var: Option<&OsStr>) -> bool {
    if resolves_kindling_binary(path_var) {
        return false;
    }
    config.spawn = Spawner::custom(|| {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no `kindling` binary on PATH — auto-spawn skipped (CIB-381)",
        ))
    });
    config.spawn_log_path = Some(PathBuf::from(NULL_DEVICE));
    true
}

/// The platform's discard device. Writes to it are dropped by the OS, so
/// pointing the upstream spawn log here bounds it at zero bytes without
/// truncating, deleting, or relocating any existing log.
#[cfg(not(windows))]
const NULL_DEVICE: &str = "/dev/null";
#[cfg(windows)]
const NULL_DEVICE: &str = "NUL";

/// Whether an executable named `kindling` can be resolved from `path_var`,
/// mirroring what `Command::new("kindling")` would search. An unset or empty
/// `PATH` resolves nothing, so it counts as absent (fail safe).
fn resolves_kindling_binary(path_var: Option<&OsStr>) -> bool {
    let Some(path_var) = path_var.filter(|value| !value.is_empty()) else {
        return false;
    };
    std::env::split_paths(path_var).any(|dir| {
        KINDLING_BINARY_NAMES
            .iter()
            .any(|name| is_executable_file(&dir.join(name)))
    })
}

/// Names `Command::new("kindling")` would try on this platform.
#[cfg(not(windows))]
const KINDLING_BINARY_NAMES: &[&str] = &["kindling"];
#[cfg(windows)]
const KINDLING_BINARY_NAMES: &[&str] = &["kindling.exe", "kindling"];

/// Whether `path` is a file this process could execute. On Unix a file without
/// any execute bit would fail `exec`, so it does not count as a resolved
/// binary; Windows has no such bit, so being a file is enough.
fn is_executable_file(path: &std::path::Path) -> bool {
    std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file() && is_executable(&metadata))
}

/// Unix: any of the three execute bits makes the file runnable.
#[cfg(unix)]
fn is_executable(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

/// Non-Unix: no execute bit exists, so a plain file is runnable.
#[cfg(not(unix))]
fn is_executable(_metadata: &std::fs::Metadata) -> bool {
    true
}

/// A [`KindlingObservationSink`] that appends observations to the Kindling
/// daemon via a [`SpooledClient`], buffering to a local spool on a daemon
/// outage. See the module docs for the boundary + async-bridge rationale.
#[derive(Debug)]
pub struct KindlingDaemonSink {
    spooled: SpooledClient,
    /// The `repo_id` scope stamped on each observation (the project / workspace
    /// root the rows belong to). Matches the client's `project_root`.
    repo_id: Option<String>,
    /// Current-thread runtime driving the async append. Held in an `Option` so
    /// `Drop` can hand it to [`Runtime::shutdown_background`] — dropping a
    /// runtime inline panics if the sink is ever dropped inside another tokio
    /// runtime's context (e.g. the daemon's event loop). Always `Some` until
    /// drop.
    runtime: Option<Runtime>,
}

impl KindlingDaemonSink {
    /// Build a sink talking to the default daemon socket
    /// (`~/.kindling/kindling.sock`), buffering to `spool_path`.
    ///
    /// `project_root` becomes both the client's `X-Kindling-Project` routing key
    /// and the `repo_id` scope on each row; `None` uses the client default (the
    /// current working directory).
    pub fn new(
        project_root: Option<String>,
        spool_path: PathBuf,
    ) -> Result<Self, KindlingSinkError> {
        // Create the spool's parent dir owner-only (`0700`) BEFORE first use.
        // `SpooledClient` only `create`s the file, not missing ancestors — so on
        // a fresh install the first daemon-down emit would otherwise fail to
        // buffer (breaking the durable-fallback contract). The `0700` dir also
        // gates access to the spool (which holds usage metadata) on a shared
        // host, matching the NDJSON sidecar's posture even though the spool file
        // itself is written by the upstream client without an explicit mode.
        if let Some(parent) = spool_path.parent() {
            crate::usage::create_private_dir(parent).map_err(|err| {
                KindlingSinkError::Unavailable(format!(
                    "create kindling spool dir {}: {err}",
                    parent.display()
                ))
            })?;
        }
        let mut config = ClientConfig::defaults().map_err(|err| {
            KindlingSinkError::Unavailable(format!("resolve kindling client config: {err}"))
        })?;
        if let Some(root) = project_root {
            config.project_root = root;
        }
        // CIB-381: on a host with no `kindling` binary the auto-spawn can only
        // ever fail, and each failure appended an unbounded line to
        // `~/.kindling/spawn.log`. Skip the attempt (see the guard's docs).
        if guard_absent_kindling_binary(&mut config, std::env::var_os("PATH").as_deref()) {
            tracing::debug!(
                target: "anvil::usage",
                sink = "kindling_daemon",
                "no `kindling` binary on PATH — daemon auto-spawn skipped; \
                 observations buffer to the spool until a daemon is reachable",
            );
        }
        let repo_id = Some(config.project_root.clone());
        // KDS-005: bound the spool (size + age) so the only durable NDJSON in the
        // system can't grow without limit under a prolonged daemon outage.
        let spool_config = SpoolConfig::new(spool_path)
            .with_max_bytes(SPOOL_MAX_BYTES)
            .with_max_age_ms(SPOOL_MAX_AGE_MS);
        let spooled = SpooledClient::with_config(Client::with_config(config), spool_config);
        Self::from_spooled(spooled, repo_id)
    }

    /// Build a sink over an already-constructed [`SpooledClient`]. Used by the
    /// parity / spool tests to point the client at an in-process temp-socket
    /// daemon.
    fn from_spooled(
        spooled: SpooledClient,
        repo_id: Option<String>,
    ) -> Result<Self, KindlingSinkError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| {
                KindlingSinkError::Unavailable(format!("start kindling sink runtime: {err}"))
            })?;
        Ok(Self {
            spooled,
            repo_id,
            runtime: Some(runtime),
        })
    }

    /// Append a `command.invoked` observation through the spooled client,
    /// returning the [`AppendOutcome`] (`Delivered` carries the daemon-stored
    /// [`kindling_client::Observation`]; `Spooled` means the daemon was down and
    /// the row was buffered):
    ///
    /// - `Delivered` / `Spooled` → `Ok(..)` (a daemon outage is buffered, never
    ///   surfaced as an error — matching the best-effort NDJSON contract).
    /// - A propagated client error (`Api` / `SchemaMismatch` / `Decode` — the
    ///   daemon *responded* and rejected the row) → [`KindlingSinkError::Rejected`].
    /// - A spool-file I/O / serde failure → [`KindlingSinkError::Unavailable`].
    ///
    /// The sync trait method drops the outcome; it is surfaced here so tests can
    /// drive the append directly under `#[tokio::test]` (no nested `block_on`)
    /// and inspect the persisted row.
    pub(crate) async fn emit_command_invoked_async(
        &self,
        observation: CommandInvokedObservation,
    ) -> Result<AppendOutcome, KindlingSinkError> {
        let input = to_kindling_input(&observation, self.repo_id.as_deref());
        match self
            .spooled
            .append_observation(input, None, Some(true))
            .await
        {
            Ok(outcome) => Ok(outcome),
            Err(SpoolError::Client(err)) => Err(KindlingSinkError::Rejected(err.to_string())),
            Err(other) => Err(KindlingSinkError::Unavailable(other.to_string())),
        }
    }

    fn runtime(&self) -> &Runtime {
        self.runtime
            .as_ref()
            .expect("runtime is present until KindlingDaemonSink is dropped")
    }
}

impl Drop for KindlingDaemonSink {
    fn drop(&mut self) {
        // The last `Arc<KindlingDaemonSink>` may be dropped on a thread that is
        // *inside* a tokio runtime (e.g. the resident daemon tearing down its
        // emitter chain on its own event loop). Dropping a `Runtime` inline in
        // that context panics ("cannot drop a runtime in a context where
        // blocking is not allowed"). `shutdown_background` is safe from any
        // context, and we only ever `block_on` (never `spawn`), so nothing is
        // abandoned. Hence the `Option<Runtime>` + explicit Drop.
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}

impl KindlingObservationSink for KindlingDaemonSink {
    fn try_emit(&self, _observation: GateEvaluatedObservation) -> Result<(), KindlingSinkError> {
        // PORT-011 routes command.invoked only; gate_evaluated daemon routing is
        // a KDS-001 fast follow. Ignore (no write) rather than spool a row the
        // proof slice does not yet map. Trace it so the suppression is
        // distinguishable from a misconfiguration when chasing missing rows.
        tracing::trace!(
            target: "anvil::usage",
            "KindlingDaemonSink: gate_evaluated suppressed (command.invoked only for PORT-011)",
        );
        Ok(())
    }

    fn try_emit_command_invoked(
        &self,
        observation: CommandInvokedObservation,
    ) -> Result<(), KindlingSinkError> {
        // Runs on the NonBlockingObservationSink drain thread (a std::thread with
        // no ambient runtime), so this block_on never blocks a hot path and
        // never nests inside another runtime. The persisted-row outcome is
        // irrelevant to producers — only success/failure is. A daemon outage is
        // NOT an error here (it is buffered to the spool); only a daemon
        // rejection / local spool failure surfaces, logged with its kind so an
        // operator can tell a schema/API rejection from spool buildup.
        match self
            .runtime()
            .block_on(self.emit_command_invoked_async(observation))
        {
            Ok(_outcome) => Ok(()),
            Err(err) => {
                // `debug!`, not `warn!`: the NonBlockingObservationSink drain
                // already logs the failure at `warn!`. This adds the sink-scoped
                // detail (so a schema/API rejection is greppable) without
                // double-warning for one failure.
                tracing::debug!(
                    target: "anvil::usage",
                    sink = "kindling_daemon",
                    error = %err,
                    "KindlingDaemonSink: command.invoked emit failed",
                );
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod mapper_tests {
    use super::*;
    use anvil_intercept::kindling_observation::{FlagSetEntry, KIND_COMMAND_INVOKED};
    use anvil_observability::redaction::redact_arg;

    /// A canonical `command.invoked` observation, post-redaction (the shape the
    /// sink receives). Mirrors a real CLI invocation: one redacted arg shape and
    /// one resolved flag.
    pub(super) fn fixture_observation() -> CommandInvokedObservation {
        CommandInvokedObservation {
            kind: KIND_COMMAND_INVOKED.to_string(),
            session_id: "11111111-2222-3333-4444-555555555555".to_string(),
            timestamp: "2026-06-24T12:34:56.789Z".to_string(),
            command: "status".to_string(),
            principal: "sha256:deadbeefcafef00d".to_string(),
            args: vec![redact_arg("path", Some("src/main.rs"))],
            flag_set: vec![FlagSetEntry {
                key: "api.broadcast".to_string(),
                variant: "on".to_string(),
                source: "default".to_string(),
                gate_affecting: false,
            }],
            traceparent: Some(
                "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
            ),
            // CIB-197: producer identity stamped on every new row.
            version: "0.9.0-beta".to_string(),
            install_method: "cargo_dist".to_string(),
        }
    }

    #[test]
    fn maps_command_invoked_to_kindling_command_kind() {
        let obs = fixture_observation();
        let input = to_kindling_input(&obs, Some("/repo/anvil"));
        assert_eq!(input.kind, ObservationKind::Command);
    }

    #[test]
    fn content_round_trips_the_anvil_observation() {
        let obs = fixture_observation();
        let input = to_kindling_input(&obs, Some("/repo/anvil"));
        let decoded: CommandInvokedObservation =
            serde_json::from_str(&input.content).expect("content is the serialised observation");
        assert_eq!(decoded, obs);
    }

    #[test]
    fn stamps_anvil_provenance() {
        let obs = fixture_observation();
        let input = to_kindling_input(&obs, Some("/repo/anvil"));
        let provenance = input.provenance.expect("provenance present");
        assert_eq!(
            provenance.get("anvil_kind").and_then(Value::as_str),
            Some("command.invoked"),
        );
        assert_eq!(
            provenance
                .get("anvil_contract_version")
                .and_then(Value::as_str),
            Some(OBSERVATION_CONTRACT_VERSION),
        );
    }

    #[test]
    fn carries_session_and_repo_scope() {
        let obs = fixture_observation();
        let input = to_kindling_input(&obs, Some("/repo/anvil"));
        assert_eq!(
            input.scope_ids.session_id.as_deref(),
            Some(obs.session_id.as_str())
        );
        assert_eq!(input.scope_ids.repo_id.as_deref(), Some("/repo/anvil"));
    }

    #[test]
    fn id_left_unset_for_spool_to_assign() {
        let input = to_kindling_input(&fixture_observation(), None);
        assert!(
            input.id.is_none(),
            "id must be None so SpooledClient assigns a stable v4"
        );
    }

    #[test]
    fn timestamp_maps_to_epoch_millis() {
        let input = to_kindling_input(&fixture_observation(), None);
        // 2026-06-24T12:34:56.789Z → epoch ms.
        assert_eq!(input.ts, Some(1_782_304_496_789));
    }

    #[test]
    fn unparseable_timestamp_defers_to_daemon() {
        let mut obs = fixture_observation();
        obs.timestamp = "not-a-timestamp".to_string();
        let input = to_kindling_input(&obs, None);
        assert!(
            input.ts.is_none(),
            "an unparseable ts is left for the daemon to assign"
        );
    }

    #[test]
    fn empty_session_id_passes_through() {
        // The producer owns session-id minting; the sink maps verbatim (no
        // validation). Document that an empty id is carried as `Some("")`, not
        // dropped — so a malformed upstream id stays visible, not silently
        // re-scoped to the whole repo.
        let mut obs = fixture_observation();
        obs.session_id = String::new();
        let input = to_kindling_input(&obs, None);
        assert_eq!(input.scope_ids.session_id.as_deref(), Some(""));
    }
}

// KDS-001 / KDS-003: daemon-backed tests spin up a real in-process
// `kindling-server` on a temp Unix domain socket. Gated `unix` because the
// helper binds a UDS (the platform default + CI target); the Windows TCP path
// is exercised upstream in `kindling-client`.
#[cfg(all(test, unix))]
mod daemon_tests {
    // `use super::*` already brings the sink's imports into scope (Client,
    // ClientConfig, ScopeIds, SpooledClient, AppendOutcome,
    // KindlingObservationSink, …); only the test-only names are added here.
    use super::mapper_tests::fixture_observation;
    use super::*;
    use kindling_client::{RetrieveOptions, RetrievedEntity, Spawner, Transport};
    use kindling_server::{ServerConfig, serve};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tempfile::TempDir;

    /// The store's canonical schema version, as the client's `u32`.
    fn schema_version_u32() -> u32 {
        u32::try_from(kindling_store::schema_version().version).expect("schema version fits in u32")
    }

    /// Long idle timeout so the test daemon never self-shuts mid-test. Routed
    /// through a named const (literal × const) to sidestep
    /// `clippy::duration_suboptimal_units`, matching the production retention
    /// constants in `usage.rs`.
    const TEST_IDLE_TIMEOUT: Duration = {
        const MINUTE_SECS: u64 = 60;
        Duration::from_secs(60 * MINUTE_SECS)
    };

    /// A running in-process daemon on a temp socket. Holds the temp home so it
    /// outlives the test.
    struct TestDaemon {
        socket_path: std::path::PathBuf,
        _home: TempDir,
        _handle: tokio::task::JoinHandle<Result<(), kindling_server::ServerError>>,
    }

    impl TestDaemon {
        async fn start() -> Self {
            let home = tempfile::tempdir().expect("temp kindling home");
            let home_path = home.path().to_path_buf();
            // Keep the socket name short — UDS paths cap at ~108 bytes.
            let socket_path = home_path.join("k.sock");
            let config = ServerConfig {
                socket_path: socket_path.clone(),
                kindling_home: home_path.clone(),
                pid_path: home_path.join("k.pid"),
                port_path: home_path.join("k.port"),
                idle_timeout: TEST_IDLE_TIMEOUT,
                transport: kindling_server::Transport::default(),
            };
            let handle = tokio::spawn(async move { serve(config).await });
            wait_for_socket(&socket_path).await;
            Self {
                socket_path,
                _home: home,
                _handle: handle,
            }
        }
    }

    async fn wait_for_socket(socket_path: &std::path::Path) {
        for _ in 0..400 {
            if socket_path.exists() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("daemon socket never appeared: {}", socket_path.display());
    }

    /// A client pointed at `socket_path`, with a spawner that panics if invoked
    /// (the daemon is expected to be up).
    fn live_client(socket_path: std::path::PathBuf, project_root: &str) -> Client {
        Client::with_config(ClientConfig {
            socket_path,
            port_path: std::path::PathBuf::from("unused.port"),
            project_root: project_root.to_string(),
            expected_schema_version: schema_version_u32(),
            spawn_log_path: None,
            connect_timeout: Duration::from_secs(2),
            poll_interval: Duration::from_millis(10),
            spawn: Spawner::custom(|| panic!("spawner must not be called when the daemon is up")),
            transport: Transport::Uds,
        })
    }

    /// A client whose spawner fails like a missing binary — every call resolves
    /// to `ClientError::Unavailable` within a short budget (simulated outage).
    fn down_client(socket_path: std::path::PathBuf, project_root: &str) -> Client {
        Client::with_config(ClientConfig {
            socket_path,
            port_path: std::path::PathBuf::from("unused.port"),
            project_root: project_root.to_string(),
            expected_schema_version: schema_version_u32(),
            spawn_log_path: None,
            connect_timeout: Duration::from_millis(150),
            poll_interval: Duration::from_millis(10),
            spawn: Spawner::custom(|| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "kindling binary not found (simulated daemon-down)",
                ))
            }),
            transport: Transport::Uds,
        })
    }

    const REPO_ID: &str = "/repo/anvil";

    /// CIB-381: how many observations the spawn-log growth tests emit. More
    /// than one, so "one line per emit" is distinguishable from "one
    /// first-failure note".
    const EMIT_COUNT: usize = 5;

    /// A spawner that fails exactly as a missing `kindling` binary would
    /// (`ErrorKind::NotFound`) without ever exec'ing anything.
    ///
    /// Upstream logs a spawn-failure line for *any* failing spawner
    /// (`log_spawn_failure` on the `spawner.spawn()` error arm in
    /// `kindling-client`'s connect path), so this reproduces the unguarded
    /// spawn-log growth faithfully while staying hermetic on a host that really
    /// does have `kindling` installed.
    fn simulated_missing_binary_spawner() -> Spawner {
        Spawner::custom(|| {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "kindling binary not found (simulated missing binary)",
            ))
        })
    }

    /// A config pointed at a socket that will never exist under `base`, with a
    /// tiny connect budget — the "no daemon running" shape the spawn path is
    /// reached from. Keeps the real `Spawner::Command` default so callers opt
    /// into a custom spawner explicitly.
    fn absent_daemon_config(base: &std::path::Path) -> ClientConfig {
        ClientConfig {
            socket_path: base.join("absent.sock"),
            port_path: base.join("absent.port"),
            project_root: REPO_ID.to_string(),
            expected_schema_version: schema_version_u32(),
            spawn_log_path: None,
            connect_timeout: Duration::from_millis(50),
            poll_interval: Duration::from_millis(10),
            spawn: Spawner::Command,
            transport: Transport::Uds,
        }
    }

    /// Line count of a spawn log, treating "never created" as zero.
    fn spawn_log_lines(path: &std::path::Path) -> usize {
        std::fs::read_to_string(path).map_or(0, |contents| contents.lines().count())
    }

    /// Find the stored observation for `session_id` among retrieval candidates.
    async fn retrieve_observation(
        client: &Client,
        session_id: &str,
        query: &str,
    ) -> Option<kindling_client::Observation> {
        let result = client
            .retrieve(RetrieveOptions {
                query: query.to_string(),
                scope_ids: ScopeIds {
                    repo_id: Some(REPO_ID.to_string()),
                    ..Default::default()
                },
                token_budget: None,
                max_candidates: Some(50),
                include_redacted: None,
            })
            .await
            .expect("retrieve succeeds");
        result
            .candidates
            .into_iter()
            .find_map(|candidate| match candidate.entity {
                RetrievedEntity::Observation(obs)
                    if obs.scope_ids.session_id.as_deref() == Some(session_id) =>
                {
                    Some(obs)
                }
                _ => None,
            })
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn delivered_when_daemon_up() {
        let daemon = TestDaemon::start().await;
        let spool_dir = tempfile::tempdir().expect("spool dir");
        let spool = spool_dir.path().join("spool.ndjson");
        let client = live_client(daemon.socket_path.clone(), REPO_ID);
        let sink = KindlingDaemonSink::from_spooled(
            SpooledClient::new(client, spool.clone()),
            Some(REPO_ID.to_string()),
        )
        .expect("sink builds");

        let outcome = sink
            .emit_command_invoked_async(fixture_observation())
            .await
            .expect("emit succeeds");

        assert!(
            matches!(outcome, AppendOutcome::Delivered(_)),
            "daemon up → row delivered, not spooled",
        );
        assert!(
            !spool.exists(),
            "nothing spooled when the daemon is reachable"
        );
    }

    /// KDS-003 acceptance: the daemon-stored row matches the NDJSON-path row for
    /// the same input (modulo daemon-assigned id/ts).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn parity_ndjson_vs_daemon() {
        let fixture = fixture_observation();

        // NDJSON path: write through the real sidecar writer, read the line back.
        let ndjson_dir = tempfile::tempdir().expect("ndjson dir");
        let ndjson_path = ndjson_dir.path().join("usage.ndjson");
        crate::usage::append_usage_observation_to(&ndjson_path, &fixture)
            .expect("ndjson append succeeds");
        let line = std::fs::read_to_string(&ndjson_path).expect("read ndjson");
        let ndjson_obs: CommandInvokedObservation =
            serde_json::from_str(line.trim()).expect("parse ndjson row");

        // Daemon path: emit through the sink, inspect the daemon-stored row.
        let daemon = TestDaemon::start().await;
        let spool_dir = tempfile::tempdir().expect("spool dir");
        let spool = spool_dir.path().join("spool.ndjson");
        let client = live_client(daemon.socket_path.clone(), REPO_ID);
        let sink = KindlingDaemonSink::from_spooled(
            SpooledClient::new(client, spool),
            Some(REPO_ID.to_string()),
        )
        .expect("sink builds");
        let outcome = sink
            .emit_command_invoked_async(fixture.clone())
            .await
            .expect("emit succeeds");
        let AppendOutcome::Delivered(result) = outcome else {
            panic!("expected Delivered");
        };
        // 0.3: Delivered carries an AppendResult { observation, deduplicated }.
        let stored = &result.observation;

        // The anvil payload inside `content` round-trips identically by both
        // paths.
        let daemon_obs: CommandInvokedObservation =
            serde_json::from_str(&stored.content).expect("parse daemon content");
        assert_eq!(ndjson_obs, daemon_obs, "NDJSON and daemon rows must match");
        assert_eq!(daemon_obs, fixture, "daemon row round-trips to the input");

        // Kindling envelope: kind + provenance + scope (id/ts are daemon-assigned
        // and intentionally not asserted).
        assert_eq!(stored.kind, ObservationKind::Command);
        assert_eq!(
            stored.provenance.get("anvil_kind").and_then(Value::as_str),
            Some("command.invoked"),
        );
        assert_eq!(
            stored
                .provenance
                .get("anvil_contract_version")
                .and_then(Value::as_str),
            Some(OBSERVATION_CONTRACT_VERSION),
        );
        assert_eq!(
            stored.scope_ids.session_id.as_deref(),
            Some(fixture.session_id.as_str())
        );
    }

    /// Daemon down → row spools and the caller still gets `Ok`; on restart a
    /// flush replays it into the daemon with identical content.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn spooled_when_down_then_replayed_on_flush() {
        let fixture = fixture_observation();
        let spool_dir = tempfile::tempdir().expect("spool dir");
        let spool = spool_dir.path().join("spool.ndjson");

        // Pick the socket path the (not-yet-started) daemon will bind, so the
        // same path works for both the down and live phases.
        let daemon_home = tempfile::tempdir().expect("daemon home");
        let socket_path = daemon_home.path().join("k.sock");

        // Phase 1: daemon down → spooled, Ok returned.
        {
            let sink = KindlingDaemonSink::from_spooled(
                SpooledClient::new(down_client(socket_path.clone(), REPO_ID), spool.clone()),
                Some(REPO_ID.to_string()),
            )
            .expect("sink builds");
            let outcome = sink
                .emit_command_invoked_async(fixture.clone())
                .await
                .expect("outage is buffered, not surfaced");
            assert!(
                matches!(outcome, AppendOutcome::Spooled),
                "daemon down → spooled"
            );
            assert_eq!(
                sink.spooled.pending_count().expect("count"),
                1,
                "one row buffered"
            );
        }

        // Phase 2: bring the daemon up on that socket, flush, confirm replay.
        let config = ServerConfig {
            socket_path: socket_path.clone(),
            kindling_home: daemon_home.path().to_path_buf(),
            pid_path: daemon_home.path().join("k.pid"),
            port_path: daemon_home.path().join("k.port"),
            idle_timeout: TEST_IDLE_TIMEOUT,
            transport: kindling_server::Transport::default(),
        };
        let _handle = tokio::spawn(async move { serve(config).await });
        wait_for_socket(&socket_path).await;

        let client = live_client(socket_path.clone(), REPO_ID);
        let sink = KindlingDaemonSink::from_spooled(
            SpooledClient::new(client.clone(), spool.clone()),
            Some(REPO_ID.to_string()),
        )
        .expect("sink builds");

        let report = sink.spooled.flush().await.expect("flush succeeds");
        assert_eq!(report.replayed, 1, "the buffered row replayed");
        assert_eq!(report.remaining, 0, "spool drained");
        assert_eq!(
            sink.spooled.pending_count().expect("count"),
            0,
            "spool empty after flush"
        );

        // The replayed row is retrievable with identical content + provenance.
        let stored = retrieve_observation(&client, &fixture.session_id, &fixture.command)
            .await
            .expect("replayed row retrievable");
        let replayed_obs: CommandInvokedObservation =
            serde_json::from_str(&stored.content).expect("parse replayed content");
        assert_eq!(replayed_obs, fixture, "replayed row matches the original");
        assert_eq!(
            stored.provenance.get("anvil_kind").and_then(Value::as_str),
            Some("command.invoked"),
        );
    }

    /// The synchronous trait method bridges to the async append via `block_on`
    /// on a thread with no ambient runtime — exactly the `NonBlockingObservationSink`
    /// drain-thread condition. A plain `#[test]` (no `#[tokio::test]`) reproduces
    /// that: no nested-runtime panic, outage buffered, `Ok` returned.
    #[test]
    fn sync_trait_path_blocks_on_without_ambient_runtime() {
        let spool_dir = tempfile::tempdir().expect("spool dir");
        let spool = spool_dir.path().join("spool.ndjson");
        let socket_path = spool_dir.path().join("nope.sock");
        let sink = KindlingDaemonSink::from_spooled(
            SpooledClient::new(down_client(socket_path, REPO_ID), spool),
            Some(REPO_ID.to_string()),
        )
        .expect("sink builds");

        // Drives the sink's own current-thread runtime via block_on on this
        // (runtime-free) thread.
        sink.try_emit_command_invoked(fixture_observation())
            .expect("outage buffered, Ok returned");
        assert_eq!(
            sink.spooled.pending_count().expect("count"),
            1,
            "row spooled via sync path"
        );
    }

    /// CIB-381 regression harness: the unguarded default. A client whose
    /// spawner fails like a missing binary appends at least one spawn-log line
    /// per emit, forever — the unbounded growth this item fixes. (More than one,
    /// in fact: once the spool has a backlog `append_observation` also tries an
    /// opportunistic flush, and that connect attempt spawns and logs too.)
    /// Documents the mechanism, and proves the guard test below can observe
    /// growth at all.
    ///
    /// Hermetic by construction: the spawner is a custom closure that always
    /// fails `NotFound`, never `Spawner::Command`. With the real command
    /// spawner a host that *does* have `kindling` on `PATH` would exec it and
    /// start a real daemon against real user state — the failure this canary is
    /// meant to simulate, not perform.
    #[test]
    fn unguarded_spawner_appends_at_least_one_spawn_log_line_per_emit() {
        let base = tempfile::tempdir().expect("base dir");
        let spawn_log = base.path().join("spawn.log");
        let mut config = absent_daemon_config(base.path());
        config.spawn_log_path = Some(spawn_log.clone());
        config.spawn = simulated_missing_binary_spawner();
        let sink = KindlingDaemonSink::from_spooled(
            SpooledClient::new(
                Client::with_config(config),
                base.path().join("spool.ndjson"),
            ),
            Some(REPO_ID.to_string()),
        )
        .expect("sink builds");

        for _ in 0..EMIT_COUNT {
            sink.try_emit_command_invoked(fixture_observation())
                .expect("outage buffered");
        }

        let lines = spawn_log_lines(&spawn_log);
        assert!(
            lines >= EMIT_COUNT,
            "unguarded: the spawn log grows with every emit (the CIB-381 bug); \
             {EMIT_COUNT} emits produced {lines} lines",
        );
    }

    /// CIB-381 acceptance: with a `PATH` that cannot resolve `kindling`, the
    /// guard must (a) never invoke the spawner and (b) leave the spawn log
    /// untouched, however many observations are emitted — while the rows still
    /// buffer to the (capped) spool.
    #[test]
    fn absent_binary_guard_skips_spawn_and_leaves_spawn_log_untouched() {
        let base = tempfile::tempdir().expect("base dir");
        let spawn_log = base.path().join("spawn.log");
        let empty_path_dir = base.path().join("bin");
        std::fs::create_dir_all(&empty_path_dir).expect("empty PATH dir");
        let spool = base.path().join("spool.ndjson");

        let spawn_calls = Arc::new(AtomicUsize::new(0));
        let mut config = absent_daemon_config(base.path());
        config.spawn_log_path = Some(spawn_log.clone());
        config.spawn = {
            let spawn_calls = Arc::clone(&spawn_calls);
            Spawner::custom(move || {
                spawn_calls.fetch_add(1, Ordering::SeqCst);
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "kindling binary not found",
                ))
            })
        };

        let guarded =
            super::guard_absent_kindling_binary(&mut config, Some(empty_path_dir.as_os_str()));
        assert!(guarded, "a PATH without `kindling` must engage the guard");

        let sink = KindlingDaemonSink::from_spooled(
            SpooledClient::new(Client::with_config(config), spool),
            Some(REPO_ID.to_string()),
        )
        .expect("sink builds");

        for _ in 0..EMIT_COUNT {
            sink.try_emit_command_invoked(fixture_observation())
                .expect("outage buffered");
        }

        assert_eq!(
            spawn_calls.load(Ordering::SeqCst),
            0,
            "the spawn attempt must be skipped when the binary is absent",
        );
        assert_eq!(
            spawn_log_lines(&spawn_log),
            0,
            "the spawn log must not grow per emit",
        );
        assert_eq!(
            sink.spooled.pending_count().expect("count"),
            EMIT_COUNT,
            "observations still buffer to the capped spool",
        );
    }

    /// The guard is scoped to the absent-binary case: when `kindling` IS
    /// resolvable the upstream spawner and its diagnostics stay exactly as they
    /// are (auto-spawn is the whole point of the daemon sink).
    #[test]
    fn present_binary_leaves_the_upstream_spawner_and_log_alone() {
        use std::os::unix::fs::PermissionsExt;
        let base = tempfile::tempdir().expect("base dir");
        let bin = base.path().join("bin");
        std::fs::create_dir_all(&bin).expect("bin dir");
        let binary = bin.join("kindling");
        std::fs::write(&binary, b"#!/bin/sh\n").expect("write fake binary");
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
            .expect("chmod fake binary");

        let spawn_log = base.path().join("spawn.log");
        let mut config = absent_daemon_config(base.path());
        config.spawn_log_path = Some(spawn_log.clone());

        let guarded = super::guard_absent_kindling_binary(&mut config, Some(bin.as_os_str()));

        assert!(
            !guarded,
            "a resolvable `kindling` must not engage the guard"
        );
        assert!(
            matches!(config.spawn, Spawner::Command),
            "the upstream command spawner is left in place",
        );
        assert_eq!(
            config.spawn_log_path.as_deref(),
            Some(spawn_log.as_path()),
            "the upstream spawn-log path is left in place",
        );
    }

    /// An unset or empty `PATH` cannot resolve anything, so it is treated as
    /// "binary absent" (fail safe: no spawn, no unbounded log).
    #[test]
    fn missing_path_variable_engages_the_guard() {
        let base = tempfile::tempdir().expect("base dir");
        let mut config = absent_daemon_config(base.path());
        assert!(super::guard_absent_kindling_binary(&mut config, None));

        let mut config = absent_daemon_config(base.path());
        assert!(super::guard_absent_kindling_binary(
            &mut config,
            Some(std::ffi::OsStr::new(""))
        ));
    }

    /// A non-executable file named `kindling` on `PATH` is not a runnable
    /// binary, so the guard still engages.
    #[test]
    fn non_executable_kindling_file_engages_the_guard() {
        use std::os::unix::fs::PermissionsExt;
        let base = tempfile::tempdir().expect("base dir");
        let bin = base.path().join("bin");
        std::fs::create_dir_all(&bin).expect("bin dir");
        let binary = bin.join("kindling");
        std::fs::write(&binary, b"not executable").expect("write file");
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o644))
            .expect("chmod file");

        let mut config = absent_daemon_config(base.path());
        assert!(
            super::guard_absent_kindling_binary(&mut config, Some(bin.as_os_str())),
            "a non-executable file is not a spawnable binary",
        );
    }

    /// `new` must create the spool's `kindling/` parent dir owner-only (`0700`)
    /// before first use — both so a cold-start daemon-down emit can buffer
    /// (durability) and so the spool's usage metadata is gated on a shared host.
    #[test]
    fn new_creates_spool_dir_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let base = tempfile::tempdir().expect("base dir");
        let spool = base.path().join("kindling").join("spool.ndjson");
        assert!(
            !spool.parent().expect("has parent").exists(),
            "precondition: spool dir absent",
        );

        let _sink = KindlingDaemonSink::new(Some(REPO_ID.to_string()), spool.clone())
            .expect("sink builds and creates the spool dir");

        let dir = spool.parent().expect("has parent");
        assert!(dir.exists(), "spool parent dir created");
        let mode = std::fs::metadata(dir)
            .expect("stat dir")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700, "spool dir is owner-only");
    }

    // ---------------------------------------------------------------------
    // CIB-381 constructor-level wiring proof.
    //
    // The guard tests above call `guard_absent_kindling_binary` directly, so
    // they cannot see whether `KindlingDaemonSink::new` still *calls* it, nor
    // whether it hands it the real process `PATH`. That wiring is what actually
    // stops `~/.kindling/spawn.log` growing in production, so it is proved here
    // through `new` itself.
    //
    // `new` reads `PATH` and `HOME` from the process environment, and
    // `temp_env`-style mutation is process-global (it races every concurrent
    // reader in this binary). So each case runs in a **child process** — this
    // same test binary, re-executed with `--exact` on this test plus a purpose
    // built `PATH` and a short throwaway `HOME`. Nothing touches the operator's
    // real `~/.kindling`, and no `kindling` binary is ever exec'd: the
    // absent/live children have an empty directory as their whole `PATH`, and
    // the present child never emits.
    // ---------------------------------------------------------------------

    /// Selects a child-process case below. Unset ⇒ this process is the parent.
    const GUARD_WIRING_CASE_ENV: &str = "ANVIL_CIB381_GUARD_WIRING_CASE";

    /// The libtest name of the test below (`module::path::test_name`), derived
    /// from `module_path!()` with the crate segment stripped — that is the form
    /// libtest's `--exact` filter matches.
    fn guard_wiring_test_name() -> String {
        let module = module_path!();
        let path = module.split_once("::").map_or(module, |(_, rest)| rest);
        format!("{path}::new_wires_the_absent_binary_guard_to_the_process_path")
    }

    /// Run one child case, failing the parent with the child's captured output.
    ///
    /// `bin_dir` becomes the child's entire `PATH`; `home` its entire `HOME`
    /// (kept short — a long `HOME` overflows `sockaddr_un.sun_path` and breaks
    /// the live-daemon case).
    fn run_guard_wiring_case(case: &str, home: &std::path::Path, bin_dir: &std::path::Path) {
        let exe = std::env::current_exe().expect("test binary path");
        let output = std::process::Command::new(exe)
            .args([
                "--exact",
                &guard_wiring_test_name(),
                "--nocapture",
                "--test-threads=1",
            ])
            .env(GUARD_WIRING_CASE_ENV, case)
            .env("HOME", home)
            .env("PATH", bin_dir)
            .output()
            .expect("re-exec the test binary as a child");

        assert!(
            output.status.success(),
            "child case `{case}` failed ({});\n--- stdout ---\n{}\n--- stderr ---\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        // A filter that matches nothing also exits 0, which would make every
        // case vacuous. Require libtest to report exactly one test run.
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("1 passed"),
            "child case `{case}` must actually run the wiring test (libtest \
             reported no passing test);\n--- stdout ---\n{stdout}",
        );
    }

    /// A short-lived `HOME` plus an empty `bin` directory to use as `PATH`.
    /// `prefix("k")` keeps the path short for the UDS socket under it.
    fn guard_wiring_sandbox() -> (TempDir, std::path::PathBuf) {
        let home = tempfile::Builder::new()
            .prefix("k")
            .tempdir()
            .expect("child home");
        let bin = home.path().join("bin");
        std::fs::create_dir_all(&bin).expect("child PATH dir");
        (home, bin)
    }

    /// Block until `socket_path` appears (the child has no ambient runtime, so
    /// this is the synchronous twin of [`wait_for_socket`]).
    fn wait_for_socket_blocking(socket_path: &std::path::Path) {
        for _ in 0..400 {
            if socket_path.exists() {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("daemon socket never appeared: {}", socket_path.display());
    }

    /// CIB-381 wiring acceptance: `KindlingDaemonSink::new` must consult
    /// `guard_absent_kindling_binary` with the **real process `PATH`**.
    ///
    /// Three child cases, all driven through `new` (never the guard directly):
    ///
    /// - `absent`: `PATH` cannot resolve `kindling` → the guard engages, the
    ///   spawner is replaced, the spawn log is redirected to the null device,
    ///   and `EMIT_COUNT` emits leave `~/.kindling/spawn.log` non-existent while
    ///   still buffering to the spool.
    /// - `present`: `PATH` *can* resolve `kindling` → the upstream spawner and
    ///   spawn-log path survive untouched. This is what pins the argument: a
    ///   constructor that passed a hard-coded empty `PATH` would fail here.
    /// - `live`: a daemon is already listening on the default socket while
    ///   `PATH` is empty → the guard must not disturb the fast path; the row is
    ///   delivered (nothing spooled) and no spawn log is written.
    #[test]
    fn new_wires_the_absent_binary_guard_to_the_process_path() {
        use std::os::unix::fs::PermissionsExt;

        let Ok(case) = std::env::var(GUARD_WIRING_CASE_ENV) else {
            // Parent: drive each case in its own process.
            let (home, bin) = guard_wiring_sandbox();
            run_guard_wiring_case("absent", home.path(), &bin);

            let (home, bin) = guard_wiring_sandbox();
            let binary = bin.join("kindling");
            // A file that is executable but never executed (no child with this
            // `PATH` emits, so nothing reaches the spawner).
            std::fs::write(&binary, b"#!/bin/sh\nexit 1\n").expect("write fake binary");
            std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
                .expect("chmod fake binary");
            run_guard_wiring_case("present", home.path(), &bin);

            let (home, bin) = guard_wiring_sandbox();
            run_guard_wiring_case("live", home.path(), &bin);
            return;
        };
        guard_wiring_child(&case);
    }

    /// One child-process case of the test above. Panics (failing the child, and
    /// through it the parent) on any violation.
    fn guard_wiring_child(case: &str) {
        let home = std::path::PathBuf::from(
            std::env::var_os("HOME").expect("parent sets a throwaway HOME"),
        );
        assert!(
            home.starts_with(std::env::temp_dir()),
            "refusing to run against a non-temporary HOME: {}",
            home.display(),
        );
        let spool = home.join("anvil").join("kindling").join("spool.ndjson");
        // Resolved the same way the client resolves them, from this child's
        // HOME — so the assertions cover the real production paths.
        let spawn_log = kindling_client::default_spawn_log_path().expect("child spawn-log path");
        let socket_path = kindling_client::default_socket_path().expect("child socket path");

        match case {
            "absent" => {
                let sink =
                    KindlingDaemonSink::new(Some(REPO_ID.to_string()), spool).expect("sink builds");
                assert_guard_engaged(&sink, "an unresolvable `kindling` on PATH");
                emit_n(&sink);
                assert!(
                    !spawn_log.exists(),
                    "`new` must keep the upstream spawn log unwritten; {} exists",
                    spawn_log.display(),
                );
                assert_eq!(
                    sink.spooled.pending_count().expect("count"),
                    EMIT_COUNT,
                    "observations still buffer to the capped spool",
                );
            }
            "present" => {
                let sink =
                    KindlingDaemonSink::new(Some(REPO_ID.to_string()), spool).expect("sink builds");
                let config = sink.spooled.client().config();
                assert!(
                    matches!(config.spawn, Spawner::Command),
                    "a resolvable `kindling` on the process PATH must leave the \
                     upstream spawner in place — `new` must pass the real PATH \
                     to the guard (got {:?})",
                    config.spawn,
                );
                assert!(
                    config.spawn_log_path.is_none(),
                    "the upstream spawn-log default must survive when the binary \
                     is present (got {:?})",
                    config.spawn_log_path,
                );
            }
            "live" => {
                // A daemon already listening on the default socket, started on
                // its own runtime thread (this thread must stay runtime-free so
                // the sink's `block_on` bridge works, as in production).
                std::fs::create_dir_all(socket_path.parent().expect("socket parent"))
                    .expect("kindling home");
                let config = ServerConfig {
                    socket_path: socket_path.clone(),
                    kindling_home: socket_path.parent().expect("socket parent").to_path_buf(),
                    pid_path: kindling_client::default_port_path()
                        .expect("port path")
                        .with_extension("pid"),
                    port_path: kindling_client::default_port_path().expect("port path"),
                    idle_timeout: TEST_IDLE_TIMEOUT,
                    transport: kindling_server::Transport::default(),
                };
                std::thread::spawn(move || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("daemon runtime");
                    let _ = runtime.block_on(serve(config));
                });
                wait_for_socket_blocking(&socket_path);

                let sink =
                    KindlingDaemonSink::new(Some(REPO_ID.to_string()), spool).expect("sink builds");
                assert_guard_engaged(&sink, "a live daemon with an unresolvable PATH");
                sink.try_emit_command_invoked(fixture_observation())
                    .expect("emit succeeds against the live daemon");
                assert_eq!(
                    sink.spooled.pending_count().expect("count"),
                    0,
                    "the guard must not disturb the fast path: with a daemon up \
                     the row is delivered, not spooled",
                );
                assert!(
                    !spawn_log.exists(),
                    "no spawn log is written when the daemon answers; {} exists",
                    spawn_log.display(),
                );
            }
            other => panic!("unknown child case `{other}`"),
        }
    }

    /// Assert the constructor installed the CIB-381 guard on the built client.
    fn assert_guard_engaged(sink: &KindlingDaemonSink, context: &str) {
        let config = sink.spooled.client().config();
        assert!(
            matches!(config.spawn, Spawner::Custom(_)),
            "`KindlingDaemonSink::new` must install the absent-binary spawner \
             ({context}); got {:?} — is the guard still called from the \
             constructor, with the process PATH?",
            config.spawn,
        );
        assert_eq!(
            config.spawn_log_path.as_deref(),
            Some(std::path::Path::new(super::NULL_DEVICE)),
            "`KindlingDaemonSink::new` must redirect the upstream spawn log to \
             the null device ({context})",
        );
    }

    /// Emit [`EMIT_COUNT`] observations through the synchronous trait path.
    fn emit_n(sink: &KindlingDaemonSink) {
        for _ in 0..EMIT_COUNT {
            sink.try_emit_command_invoked(fixture_observation())
                .expect("outage buffered");
        }
    }
}
