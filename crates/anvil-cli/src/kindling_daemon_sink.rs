//! KDS-001 / PORT-011: `KindlingObservationSink` backed by the Kindling
//! daemon (with local spool fallback).

use std::path::PathBuf;

use anvil_intercept::kindling_observation::{
    CommandInvokedObservation, GateEvaluatedObservation, KindlingObservationSink, KindlingSinkError,
};
use anyhow::Context as _;
use kindling_client::spool::{AppendOutcome, SpoolConfig, SpoolError, SpooledClient};
use kindling_client::{Client, ClientConfig, ObservationInput, ObservationKind, ScopeIds};
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
}
