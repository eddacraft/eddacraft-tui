#![cfg(unix)]

//! RTAI-008: shared mid-edit error contract fixture.
//!
//! # Single source of truth
//!
//! This file is the canonical contract for the mid-edit RPC's error
//! envelope. The contract is:
//!
//! > A `scan_buffer` response is **always** either
//! > `result.diagnostics: [...]` (possibly empty) **or**
//! > `error: { code, message, ... }`. There is no third state.
//! > A driver MUST NOT swallow a daemon error into "no diagnostics".
//!
//! Every consumer of the mid-edit RPC MUST run this fixture against
//! their own transport. New drivers fail this contract if they treat a
//! daemon error as a silent pass.
//!
//! # Consumers
//!
//! - **Rust integration test** — the test in this file exercises the
//!   live IPC listener over a Unix socket. This is the reference
//!   implementation; if a transport diverges from this behaviour, the
//!   transport is wrong, not the contract.
//! - **RMCP (`rust-mcp-launch-shim`)** — RTAI-006. RMCP's mid-edit
//!   validation tool sits on top of the same RPC and MUST import the
//!   `request_*` / `assert_*_response` pairs here and run them against
//!   its `apply_edit` / validate-write surface. Test file:
//!   `crates/anvil-rmcp/tests/midedit_contract_consumer.rs` (to be added
//!   in RTAI-006).
//! - **Future TS `DriverClient`** — RTAI-004 / DRVR-001. When the TS
//!   envelope lands, a TS port of these fixtures (same wire shape, same
//!   assertions) ships under `packages/anvil-driver-client/test/` and
//!   imports the JSON frames produced here as a check-in fixture.
//! - **`VSCode` editor driver** — RTAI-005. The editor driver wraps
//!   `DriverClient`; running the TS fixture covers it.
//!
//! # Adding a new error variant
//!
//! 1. Add the variant to `midedit::ScanBufferError` and map it in
//!    `ipc::scan_buffer_failure` (the JSON-RPC code mapping).
//! 2. Add a `*_request()` / `assert_*_response()` pair below.
//! 3. Add the new pair to [`run_full_contract`] so every consumer
//!    picks it up automatically.
//! 4. Update each consumer's contract test (RMCP today; TS / `VSCode`
//!    as they land). CI failures on the consumer side are the contract
//!    doing its job.
//!
//! # Documented gaps
//!
//! - **Cross-session subscription rejection** is **not** enforced today
//!   — `scan_buffer` carries no `sessionId` parameter. The fixture for
//!   that case is gated behind `#[ignore = "RTAI-008 gap cross-session
//!   rejection not yet implemented"]` so it stays visible without
//!   breaking CI. Wire it in when the daemon grows session-scoped
//!   `scan_buffer` enforcement.
//! - **`WorkerFailed` (-32603)** is currently exercised only via the
//!   `ServiceUnavailable` cousin. The remaining branch — a failure of
//!   `std::thread::Builder::spawn` — is not portably reproducible from
//!   a test (it requires forcing the OS to refuse a thread, which is
//!   platform-specific and racy). The contract therefore pins the code
//!   mapping (`-32603`) but does not yet have a dedicated fixture for
//!   the spawn-failure path. Wire one in if a portable hook lands.
//! - **Rule panic isolation** — see [`assert_rule_panic_response`] for
//!   the full caveat. The workspace release profile is
//!   `panic = "unwind"` (ADR-051), so the registry's `catch_unwind`
//!   isolation holds in release as well as debug / test. The
//!   previously-tracked abort-path follow-up
//!   (`daemon_aborts_on_rule_panic_in_release`) is OBSOLETE: release
//!   no longer aborts, so there is no distinct release contract to
//!   pin.

use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::time::Duration;

use anvil_intercept::Shutdown;
use anvil_intercept::enforcement::{CONTENT_SIZE_CAP_BYTES_USIZE, EnforcementPipeline};
use anvil_intercept::ipc::{IpcListener, NoopDispatcher};
use anvil_intercept::midedit::{
    MAX_CONCURRENT_SCAN_BUFFERS, MAX_SCAN_BUFFER_PATH_BYTES, ScanBufferService,
};
use anvil_intercept_rules::{InterceptRule, RuleDecision, RuleInput, RuleRegistry};
use anvil_kernel_types::{Diagnostic, Mode};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

// ---------------------------------------------------------------------
// Public fixture API.
// ---------------------------------------------------------------------

/// Every fixture exposed by this contract. Consumers can iterate over
/// this list to drive every case through their transport without
/// hard-coding the names.
///
/// Keep in sync with [`run_full_contract`] when a new fixture lands.
pub const FIXTURE_NAMES: &[&str] = &[
    "over_cap_content",
    "malformed_request",
    "invalid_path",
    "path_too_long",
    "unsupported_mode",
    "rule_panic_isolated",
    "transport_timeout",
    "server_busy",
    "cross_session_rejection",
];

/// Build the JSON-RPC request for the over-cap content case.
///
/// Sends a `scan_buffer` request whose `text` exceeds the
/// 1 MB content cap. The daemon must reject with a structured
/// `Invalid params` error before any rule runs.
#[must_use]
pub fn over_cap_content_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "src/contract/over-cap.ts",
            "text": "a".repeat(CONTENT_SIZE_CAP_BYTES_USIZE + 1),
            "version": 1,
            "mode": "midEdit"
        },
        "id": "contract-over-cap"
    })
}

/// Assert the over-cap response shape.
///
/// Pins:
/// - JSON-RPC envelope (`jsonrpc = "2.0"`, id echoed).
/// - `error.code = -32602` (Invalid params, per `scan_buffer_failure`).
/// - `error.data.reason` mentions the cap so a driver can surface a
///   human-readable hint.
/// - No silent pass: `result` MUST NOT be present.
pub fn assert_over_cap_response(response: &Value) {
    assert_envelope_is_error_or_diagnostics(response);
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "contract-over-cap");
    assert_eq!(
        response["error"]["code"], -32602,
        "over-cap content must map to Invalid params (-32602): {response}",
    );
    assert_eq!(response["error"]["message"], "Invalid params");
    let reason = response["error"]["data"]["reason"]
        .as_str()
        .expect("over-cap reason string");
    assert!(
        reason.contains("content exceeds"),
        "over-cap reason must mention the cap, got: {reason}",
    );
    assert!(
        response.get("result").is_none(),
        "over-cap must NOT silently pass with a result: {response}",
    );
}

/// Build the JSON-RPC request for a malformed mid-edit call.
///
/// Omits the required `text` field. The daemon must reject as
/// `Invalid params` rather than running with empty content.
#[must_use]
pub fn malformed_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "src/contract/malformed.ts",
            "version": 1,
            "mode": "midEdit"
        },
        "id": "contract-malformed"
    })
}

/// Assert the malformed-request response shape.
pub fn assert_malformed_response(response: &Value) {
    assert_envelope_is_error_or_diagnostics(response);
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "contract-malformed");
    assert_eq!(
        response["error"]["code"], -32602,
        "missing required field must map to Invalid params (-32602): {response}",
    );
    assert_eq!(response["error"]["message"], "Invalid params");
    assert!(
        response["error"].get("data").is_some(),
        "malformed request error must carry structured data: {response}",
    );
    assert!(
        response.get("result").is_none(),
        "malformed request must NOT silently pass with a result: {response}",
    );
}

/// Build the JSON-RPC request for the invalid-path case.
///
/// Sends an empty path string, which `validate_scan_buffer_path`
/// rejects as `ScanBufferError::InvalidPath`. The daemon must reject
/// with `Invalid params` (-32602).
#[must_use]
pub fn invalid_path_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "",
            "text": "const value = 1;\n",
            "version": 1,
            "mode": "midEdit"
        },
        "id": "contract-invalid-path"
    })
}

/// Assert the invalid-path response shape.
pub fn assert_invalid_path_response(response: &Value) {
    assert_envelope_is_error_or_diagnostics(response);
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "contract-invalid-path");
    assert_eq!(
        response["error"]["code"], -32602,
        "invalid path must map to Invalid params (-32602): {response}",
    );
    assert_eq!(response["error"]["message"], "Invalid params");
    let reason = response["error"]["data"]["reason"]
        .as_str()
        .expect("invalid-path reason string");
    assert!(
        reason.contains("path"),
        "invalid-path reason must mention the path, got: {reason}",
    );
    assert!(
        response.get("result").is_none(),
        "invalid path must NOT silently pass with a result: {response}",
    );
}

/// Build the JSON-RPC request for the path-too-long case.
///
/// Sends a path string whose byte length exceeds
/// `MAX_SCAN_BUFFER_PATH_BYTES`. The daemon must reject with
/// `Invalid params` (-32602) before any rule runs.
#[must_use]
pub fn path_too_long_request() -> Value {
    let oversized_path = "a".repeat(MAX_SCAN_BUFFER_PATH_BYTES + 1);
    json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": oversized_path,
            "text": "const value = 1;\n",
            "version": 1,
            "mode": "midEdit"
        },
        "id": "contract-path-too-long"
    })
}

/// Assert the path-too-long response shape.
pub fn assert_path_too_long_response(response: &Value) {
    assert_envelope_is_error_or_diagnostics(response);
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "contract-path-too-long");
    assert_eq!(
        response["error"]["code"], -32602,
        "path-too-long must map to Invalid params (-32602): {response}",
    );
    assert_eq!(response["error"]["message"], "Invalid params");
    let reason = response["error"]["data"]["reason"]
        .as_str()
        .expect("path-too-long reason string");
    assert!(
        reason.contains("path"),
        "path-too-long reason must mention the path, got: {reason}",
    );
    assert!(
        response.get("result").is_none(),
        "path-too-long must NOT silently pass with a result: {response}",
    );
}

/// Build the JSON-RPC request for the unsupported-mode case.
///
/// Sends `mode: "saveTime"`, which `ScanBufferMode::parse` rejects as
/// `ScanBufferError::UnsupportedMode`. The daemon must reject with
/// `Invalid params` (-32602).
#[must_use]
pub fn unsupported_mode_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "src/contract/unsupported-mode.ts",
            "text": "const value = 1;\n",
            "version": 1,
            "mode": "saveTime"
        },
        "id": "contract-unsupported-mode"
    })
}

/// Assert the unsupported-mode response shape.
pub fn assert_unsupported_mode_response(response: &Value) {
    assert_envelope_is_error_or_diagnostics(response);
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "contract-unsupported-mode");
    assert_eq!(
        response["error"]["code"], -32602,
        "unsupported mode must map to Invalid params (-32602): {response}",
    );
    assert_eq!(response["error"]["message"], "Invalid params");
    assert!(
        response["error"].get("data").is_some(),
        "unsupported-mode error must carry structured data: {response}",
    );
    assert!(
        response.get("result").is_none(),
        "unsupported mode must NOT silently pass with a result: {response}",
    );
}

/// Build the JSON-RPC request that drives the rule-panic-isolated case.
///
/// The fixture itself is just a normal `scan_buffer` call; the
/// panicking rule must be installed by the consumer when it builds the
/// transport (see [`panicking_rule_service`] for the Rust side).
#[must_use]
pub fn rule_panic_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "src/contract/panic.ts",
            "text": "const value = 1;\n",
            "version": 9,
            "mode": "midEdit"
        },
        "id": "contract-rule-panic"
    })
}

/// Assert the rule-panic response shape.
///
/// **Scope of this contract.** The workspace release profile is
/// `panic = "unwind"` (root `Cargo.toml`, per ADR-051 — Accepted), so
/// the behaviour pinned here is identical in debug, `cargo test`, and
/// release builds:
///
/// - The registry's `catch_unwind` swallows the panic and emits
///   `result.diagnostics: []` with `result.truncated = false`. There is
///   no structured error — the panic isolation contract is "rules
///   cannot crash the daemon", not "panics surface as errors". A driver
///   MUST therefore treat empty diagnostics as a valid outcome and not
///   assume the daemon would have flagged a panicking rule's would-be
///   findings.
/// - Because release unwinds (it does NOT `panic = "abort"`), there is
///   no separate abort path where the daemon dies on the panicking
///   thread. The previously-referenced follow-up contract
///   (`daemon_aborts_on_rule_panic_in_release`, a multi-process abort
///   fixture) is OBSOLETE — ADR-051 made release behave like the unwind
///   path asserted below, so no distinct release fixture is meaningful.
///
/// The assertion below is correct for all profiles. Do not weaken it on
/// the assumption that release aborts instead of isolating.
pub fn assert_rule_panic_response(response: &Value) {
    assert_envelope_is_error_or_diagnostics(response);
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "contract-rule-panic");
    assert!(
        response.get("error").is_none(),
        "rule panic must be isolated, not surfaced as an error: {response}",
    );
    let diagnostics = response["result"]["diagnostics"]
        .as_array()
        .expect("diagnostics array on isolated panic");
    assert!(
        diagnostics.is_empty(),
        "rule panic must yield empty diagnostics: {response}",
    );
    assert_eq!(response["result"]["truncated"], false);
}

/// Build the JSON-RPC request that drives the transport-timeout case.
///
/// The consumer must install a rule that blocks past
/// `SCAN_BUFFER_TIMEOUT` (2 s today) so the daemon's internal timeout
/// fires. See [`timing_out_rule_service`].
#[must_use]
pub fn transport_timeout_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "src/contract/timeout.ts",
            "text": "const value = 1;\n",
            "version": 11,
            "mode": "midEdit"
        },
        "id": "contract-timeout"
    })
}

/// Assert the transport-timeout response shape.
///
/// `ScanBufferError::TimedOut` maps specifically to JSON-RPC code
/// `-32001` ("Scan timed out", a server-defined error per JSON-RPC 2.0
/// §5.1 reserved range). Only `-32001` is accepted here.
///
/// Note: `ServiceUnavailable` and `WorkerFailed` both map to `-32603`
/// (Internal error). Those are different setups (semaphore closed,
/// thread spawn failure, channel hang-up) — intentionally NOT asserted
/// here. A bespoke fixture for those would have to force the failure
/// path, and that is out of scope for the timeout contract.
pub fn assert_transport_timeout_response(response: &Value) {
    assert_envelope_is_error_or_diagnostics(response);
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "contract-timeout");
    assert!(
        response.get("result").is_none(),
        "transport timeout must NOT silently pass with a result: {response}",
    );
    let code = response["error"]["code"]
        .as_i64()
        .expect("timeout error code");
    assert_eq!(
        code, -32001,
        "timeout must map to -32001 (TimedOut). -32603 is \
         ServiceUnavailable / WorkerFailed — a different setup, not \
         asserted here. Got code {code}: {response}",
    );
    assert!(
        response["error"]["message"].is_string(),
        "timeout error must carry a message: {response}",
    );
}

/// Build the JSON-RPC request that drives the busy fixture for slot
/// `idx`.
///
/// The consumer must saturate `MAX_CONCURRENT_SCAN_BUFFERS` blocking
/// requests first; the next request fails fast with a structured
/// `Server busy` (-32000). See [`saturating_rule_service`].
#[must_use]
pub fn busy_request(idx: usize) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": format!("src/contract/busy-{idx}.ts"),
            "text": "const value = 1;\n",
            "version": 1,
            "mode": "midEdit"
        },
        "id": format!("contract-busy-{idx}")
    })
}

/// Assert the busy response shape.
///
/// Pins:
/// - JSON-RPC envelope (`jsonrpc = "2.0"`, id echoed by the consumer).
/// - `error.code = -32000` (Server busy, server-defined per JSON-RPC
///   2.0 §5.1).
/// - `error.message = "Server busy"`.
/// - No silent pass: `result` MUST NOT be present.
pub fn assert_busy_response(response: &Value) {
    assert_envelope_is_error_or_diagnostics(response);
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(
        response["error"]["code"], -32000,
        "busy must map to -32000 (Server busy): {response}",
    );
    assert_eq!(response["error"]["message"], "Server busy");
    assert!(
        response.get("result").is_none(),
        "busy must NOT silently pass with a result: {response}",
    );
}

/// Build the JSON-RPC request for the cross-session rejection case.
///
/// Today this is just a normal `scan_buffer` call — there is no
/// `sessionId` parameter on the wire. Once cross-session rejection is
/// implemented, this fixture should add a `sessionId` field that
/// belongs to a different connection and assert the structured
/// rejection.
#[must_use]
pub fn cross_session_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "src/contract/cross-session.ts",
            "text": "const value = 1;\n",
            "version": 12,
            "mode": "midEdit"
            // TODO(RTAI-008): once scan_buffer takes a sessionId, set
            // it to a session that does not belong to this connection
            // and expect a structured rejection.
        },
        "id": "contract-cross-session"
    })
}

/// Assert the cross-session response shape.
///
/// **Gap:** `scan_buffer` does not currently accept a `sessionId`, so the
/// cross-session rejection contract cannot be exercised. This assertion
/// is the placeholder shape for the day it is — when the daemon grows
/// session-scoped `scan_buffer` enforcement, tighten this to require an
/// `error.code` and a `reason` mentioning the session mismatch. Until
/// then the only invariant we can pin is the universal "either
/// diagnostics or error, never silent" envelope rule.
pub fn assert_cross_session_response(response: &Value) {
    assert_envelope_is_error_or_diagnostics(response);
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], "contract-cross-session");
}

/// Universal invariant: a `scan_buffer` response is exactly one of
/// `result.diagnostics` (array, possibly empty) or `error` (object with
/// `code` and `message`). Never both, never neither.
///
/// Every `assert_*_response` calls into this so the "no silent pass"
/// rule is checked even before the case-specific assertions.
pub fn assert_envelope_is_error_or_diagnostics(response: &Value) {
    assert_eq!(
        response["jsonrpc"], "2.0",
        "response must declare jsonrpc 2.0: {response}",
    );
    let has_result = response.get("result").is_some();
    let has_error = response.get("error").is_some();
    assert!(
        has_result ^ has_error,
        "response must carry exactly one of `result` or `error`, never both or neither: \
         {response}",
    );
    if has_result {
        assert!(
            response["result"]["diagnostics"].is_array(),
            "result.diagnostics must be an array: {response}",
        );
    } else {
        let error = &response["error"];
        assert!(
            error["code"].is_i64() || error["code"].is_u64(),
            "error.code must be a number: {response}",
        );
        assert!(
            error["message"].is_string(),
            "error.message must be a string: {response}",
        );
    }
}

/// Drive a single fixture by name through `transport`, applying the
/// matching assertion to the response.
///
/// Centralised dispatch so consumers iterate [`FIXTURE_NAMES`] and call
/// this helper rather than hard-coding the request/assertion pairs.
/// Returns `true` if the name maps to a transport-agnostic fixture
/// (over-cap, malformed, invalid-path, path-too-long, unsupported-mode);
/// returns `false` for fixtures that need bespoke transport wiring
/// (rule-panic, transport-timeout, server-busy, cross-session). This
/// lets [`run_full_contract`] iterate every name and skip the bespoke
/// ones in one place.
fn run_named_fixture<F>(name: &str, transport: &mut F) -> bool
where
    F: FnMut(Value) -> Value,
{
    match name {
        "over_cap_content" => {
            let response = transport(over_cap_content_request());
            assert_over_cap_response(&response);
            true
        }
        "malformed_request" => {
            let response = transport(malformed_request());
            assert_malformed_response(&response);
            true
        }
        "invalid_path" => {
            let response = transport(invalid_path_request());
            assert_invalid_path_response(&response);
            true
        }
        "path_too_long" => {
            let response = transport(path_too_long_request());
            assert_path_too_long_response(&response);
            true
        }
        "unsupported_mode" => {
            let response = transport(unsupported_mode_request());
            assert_unsupported_mode_response(&response);
            true
        }
        // Bespoke setup required — consumer wires these via the
        // helper services (`panicking_rule_service`,
        // `timing_out_rule_service`, `saturating_rule_service`) or the
        // `#[ignore]`d cross-session test.
        "rule_panic_isolated" | "transport_timeout" | "server_busy" | "cross_session_rejection" => {
            false
        }
        other => panic!("run_named_fixture: unknown fixture name {other:?}"),
    }
}

/// Drive every transport-agnostic fixture through `transport`,
/// applying the matching assertion to each response.
///
/// `transport` is a synchronous closure that takes a JSON-RPC frame and
/// returns the daemon's response. Async consumers wrap their async
/// transport in a `block_on` shim. The helper iterates
/// [`FIXTURE_NAMES`] via [`run_named_fixture`], so omitting any new
/// fixture name from the central match arm becomes visibly wrong (the
/// helper panics on unknown names).
///
/// Fixtures that need bespoke transport wiring (rule-panic via
/// [`panicking_rule_service`], transport-timeout via
/// [`timing_out_rule_service`], server-busy via
/// [`saturating_rule_service`], and the still-`#[ignore]`d
/// cross-session case) are intentionally NOT driven here; consumers
/// must run them via the dedicated helpers.
pub fn run_full_contract<F>(mut transport: F)
where
    F: FnMut(Value) -> Value,
{
    for name in FIXTURE_NAMES {
        // `run_named_fixture` returns `false` for bespoke-setup
        // fixtures; we skip them so callers wire them up explicitly
        // with the right service.
        let _ = run_named_fixture(name, &mut transport);
    }
}

// ---------------------------------------------------------------------
// Public test-rule helpers. Consumers of this contract import these to
// build a transport that exercises panic-isolation, timeout, and
// busy-saturation paths.
// ---------------------------------------------------------------------

/// Build a `ScanBufferService` whose registry contains a single rule
/// that panics during diagnostics. Drives the rule-panic-isolated
/// fixture.
///
/// Panic isolation relies on the binary unwinding rather than aborting
/// on panic. The workspace release profile is `panic = "unwind"`
/// (ADR-051 — Accepted), so isolation holds in release as well as debug
/// / test; a panicking rule never kills the daemon. See
/// `crates/anvil-intercept-rules/src/registry.rs` module docs and the
/// scope note on [`assert_rule_panic_response`].
#[must_use]
pub fn panicking_rule_service() -> ScanBufferService {
    struct PanickingRule;

    impl InterceptRule for PanickingRule {
        fn rule_id(&self) -> &'static str {
            "contract-panicking-rule"
        }

        fn needs_content(&self) -> bool {
            true
        }

        fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
            RuleDecision::Allow
        }

        fn diagnostics_with_limit(
            &self,
            _input: &RuleInput<'_>,
            _mode: &Mode,
            _limit: usize,
        ) -> Vec<Diagnostic> {
            panic!("contract-panicking-rule intentionally panics to exercise isolation");
        }
    }

    let registry = RuleRegistry::with_rules(vec![Box::new(PanickingRule)])
        .expect("contract panicking rule has a unique id");
    ScanBufferService::new(EnforcementPipeline::new(registry))
}

/// Build a `ScanBufferService` whose registry contains a single rule
/// that blocks long enough to exceed `SCAN_BUFFER_TIMEOUT`, exercising
/// the transport-timeout fixture.
///
/// Returns the service plus a [`Barrier`] handle the caller MUST hold
/// until after they assert the timeout, then `wait()` on to release the
/// blocked worker thread cleanly.
#[must_use]
pub fn timing_out_rule_service() -> (ScanBufferService, Arc<Barrier>) {
    struct BlockingRule {
        barrier: Arc<Barrier>,
    }

    impl InterceptRule for BlockingRule {
        fn rule_id(&self) -> &'static str {
            "contract-blocking-rule"
        }

        fn needs_content(&self) -> bool {
            true
        }

        fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
            RuleDecision::Allow
        }

        fn diagnostics_with_limit(
            &self,
            _input: &RuleInput<'_>,
            _mode: &Mode,
            _limit: usize,
        ) -> Vec<Diagnostic> {
            self.barrier.wait();
            Vec::new()
        }
    }

    // Two participants: the worker thread and the test that releases it
    // after the timeout assertion has fired.
    let barrier = Arc::new(Barrier::new(2));
    let registry = RuleRegistry::with_rules(vec![Box::new(BlockingRule {
        barrier: Arc::clone(&barrier),
    })])
    .expect("contract blocking rule has a unique id");
    let service = ScanBufferService::new(EnforcementPipeline::new(registry));
    (service, barrier)
}

/// Build a `ScanBufferService` that lets the consumer saturate the
/// permit pool, exercising the server-busy fixture.
///
/// The returned service holds a single rule that blocks on the
/// returned [`Barrier`] until the consumer calls `wait()` to release
/// it. The barrier is sized for `MAX_CONCURRENT_SCAN_BUFFERS + 1`
/// participants (the in-flight workers plus the consumer thread); the
/// consumer is expected to:
///
/// 1. Saturate the service with [`MAX_CONCURRENT_SCAN_BUFFERS`]
///    blocking requests built via [`busy_request`], confirming each
///    worker has entered `diagnostics_with_limit` by polling the
///    returned `started` counter.
/// 2. Send one more request and assert
///    [`assert_busy_response`] on the response (code `-32000`).
/// 3. Call `barrier.wait()` to release the blocked workers.
#[must_use]
pub fn saturating_rule_service() -> (ScanBufferService, Arc<Barrier>, Arc<AtomicUsize>) {
    struct BlockingRule {
        started: Arc<AtomicUsize>,
        barrier: Arc<Barrier>,
    }

    impl InterceptRule for BlockingRule {
        fn rule_id(&self) -> &'static str {
            "contract-busy-rule"
        }

        fn needs_content(&self) -> bool {
            true
        }

        fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
            RuleDecision::Allow
        }

        fn diagnostics_with_limit(
            &self,
            _input: &RuleInput<'_>,
            _mode: &Mode,
            _limit: usize,
        ) -> Vec<Diagnostic> {
            self.started.fetch_add(1, Ordering::SeqCst);
            self.barrier.wait();
            Vec::new()
        }
    }

    let started = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(MAX_CONCURRENT_SCAN_BUFFERS + 1));
    let registry = RuleRegistry::with_rules(vec![Box::new(BlockingRule {
        started: Arc::clone(&started),
        barrier: Arc::clone(&barrier),
    })])
    .expect("contract busy rule has a unique id");
    let service = ScanBufferService::new(EnforcementPipeline::new(registry));
    (service, barrier, started)
}

// ---------------------------------------------------------------------
// Local IPC harness. The Rust integration test below uses this; RMCP
// and TS consumers wire their own transport.
// ---------------------------------------------------------------------

struct Harness {
    shutdown: Shutdown,
    handle: tokio::task::JoinHandle<Result<(), anvil_intercept::ipc::IpcError>>,
    _tmp: TempDir,
    socket: std::path::PathBuf,
}

impl Harness {
    fn start(scan_buffer: ScanBufferService) -> Self {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o700))
            .expect("secure tempdir permissions");
        let socket = tmp.path().join("intercept.sock");
        let listener =
            IpcListener::bind_with_scan_buffer_service(&socket, NoopDispatcher, scan_buffer)
                .expect("bind listener");
        let (shutdown, token) = Shutdown::new();
        let handle = tokio::spawn(async move { listener.serve(token).await });
        Self {
            shutdown,
            handle,
            _tmp: tmp,
            socket,
        }
    }

    async fn connect(&self) -> Conn {
        let stream = UnixStream::connect(&self.socket).await.expect("connect");
        Conn::new(stream)
    }

    async fn shutdown(self) {
        self.shutdown.trigger();
        tokio::time::timeout(Duration::from_secs(5), self.handle)
            .await
            .expect("listener timeout")
            .expect("listener join")
            .expect("listener ok");
    }
}

/// A reusable JSON-RPC connection over the test Unix socket.
///
/// Holds a single `BufReader<UnixStream>` for the lifetime of the
/// connection. Creating a fresh `BufReader` per request would discard
/// any bytes the buffered reader had read past the newline, which would
/// silently corrupt subsequent frames on the same connection.
struct Conn {
    reader: BufReader<UnixStream>,
}

impl Conn {
    fn new(stream: UnixStream) -> Self {
        Self {
            reader: BufReader::new(stream),
        }
    }

    async fn send(&mut self, frame: &Value) -> Value {
        self.write(frame).await;
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(10), self.reader.read_line(&mut line))
            .await
            .expect("response timeout")
            .expect("read response");
        serde_json::from_str(line.trim_end()).expect("response json")
    }

    /// Send a frame without waiting for the response. Pairs with
    /// [`Conn::drain_pending`] for tests that want to saturate workers
    /// before reading.
    async fn write(&mut self, frame: &Value) {
        self.reader
            .get_mut()
            .write_all(format!("{frame}\n").as_bytes())
            .await
            .expect("write frame");
    }

    /// Drain any pending response line, ignoring timeouts. Used during
    /// teardown to release blocked workers without leaking threads.
    async fn drain_pending(&mut self) {
        let mut line = String::new();
        let _ =
            tokio::time::timeout(Duration::from_secs(5), self.reader.read_line(&mut line)).await;
    }
}

async fn send_frame(client: &mut Conn, frame: &Value) -> Value {
    client.send(frame).await
}

// ---------------------------------------------------------------------
// Reference Rust integration test. Drives every fixture through the
// live IPC listener.
// ---------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rust_consumer_runs_full_contract() {
    // Default service (default registry: secret + reasoning) covers
    // the transport-agnostic cases — none reach a rule. We drive
    // every name in FIXTURE_NAMES via run_full_contract; bespoke
    // fixtures (rule_panic_isolated, transport_timeout, server_busy,
    // cross_session_rejection) are run by their own dedicated tests
    // below using the helper services. This guarantees every entry in
    // FIXTURE_NAMES has a live consumer in this file.
    let harness = Harness::start(ScanBufferService::default());
    let mut client = harness.connect().await;
    let runtime = tokio::runtime::Handle::current();
    // The connection's `BufReader` is held for the lifetime of the
    // closure; rebuilding one per call would risk discarding any bytes
    // the buffered reader read ahead past the newline, silently
    // corrupting the next frame.
    let mut driver = |frame: Value| -> Value {
        // We are inside a multi-threaded runtime (see attribute
        // above). `block_in_place` yields the current worker thread so
        // the nested `block_on` does not deadlock the executor.
        tokio::task::block_in_place(|| runtime.block_on(async { client.send(&frame).await }))
    };

    run_full_contract(&mut driver);

    drop(client);
    harness.shutdown().await;
}

#[tokio::test]
async fn rust_consumer_isolates_rule_panic() {
    let harness = Harness::start(panicking_rule_service());
    let mut client = harness.connect().await;

    let response = send_frame(&mut client, &rule_panic_request()).await;
    assert_rule_panic_response(&response);

    drop(client);
    harness.shutdown().await;
}

#[tokio::test]
async fn rust_consumer_surfaces_transport_timeout() {
    // A blocking rule that never returns within SCAN_BUFFER_TIMEOUT
    // (2 s today). The barrier keeps the worker thread parked until we
    // explicitly release it after the timeout fires — leaking the
    // worker would leave a zombie thread, so the cleanup is
    // load-bearing.
    let (service, barrier) = timing_out_rule_service();
    let harness = Harness::start(service);
    let mut client = harness.connect().await;

    let response = send_frame(&mut client, &transport_timeout_request()).await;
    assert_transport_timeout_response(&response);

    // Release the parked worker so the listener shuts down cleanly.
    barrier.wait();
    drop(client);
    harness.shutdown().await;
}

#[tokio::test]
async fn rust_consumer_surfaces_server_busy() {
    // The busy path is covered in detail by jsonrpc_conformance.rs; the
    // contract pins the wire shape (`-32000` / "Server busy") and
    // confirms every entry in FIXTURE_NAMES has a live exerciser. We
    // use the shared `saturating_rule_service` helper so external
    // consumers can drive the same scenario.
    let (service, barrier, started) = saturating_rule_service();
    let harness = Harness::start(service);

    // Saturate the worker permits with `MAX_CONCURRENT_SCAN_BUFFERS`
    // blocking requests built from the public `busy_request` fixture.
    let mut blockers = Vec::new();
    for idx in 0..MAX_CONCURRENT_SCAN_BUFFERS {
        let mut blocker = harness.connect().await;
        blocker.write(&busy_request(idx)).await;
        blockers.push(blocker);
    }
    // 100 × 10ms = 1s ceiling; widened from 500ms because the IPC
    // round-trip plus thread spawn can take >500ms on shared CI
    // hardware. If 1s is still not enough, the daemon is not actually
    // saturating — fail loudly.
    for _ in 0..100 {
        if started.load(Ordering::SeqCst) == MAX_CONCURRENT_SCAN_BUFFERS {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(
        started.load(Ordering::SeqCst),
        MAX_CONCURRENT_SCAN_BUFFERS,
        "expected {MAX_CONCURRENT_SCAN_BUFFERS} blocking workers to enter the rule within 1s; \
         the busy fixture cannot exercise the saturation path until they do",
    );

    // The next request fails fast with a structured `Server busy`. The
    // shared assertion pins the wire shape (`-32000` / "Server busy")
    // for every consumer.
    let mut busy_client = harness.connect().await;
    let busy_response = send_frame(&mut busy_client, &busy_request(99)).await;
    assert_busy_response(&busy_response);
    assert_eq!(busy_response["id"], "contract-busy-99");

    // Release blockers and tidy up.
    barrier.wait();
    for mut blocker in blockers {
        blocker.drain_pending().await;
    }
    drop(busy_client);
    harness.shutdown().await;
}

#[tokio::test]
async fn rust_consumer_busy_response_satisfies_envelope_invariant() {
    // Belt-and-braces: re-run the saturation flow with an inline
    // blocking rule and assert ONLY the universal envelope invariant.
    // This catches regressions where the busy path stops emitting an
    // `error` and starts returning a silent `result` — independent of
    // whether the code mapping changes.
    struct BlockingRule {
        started: Arc<AtomicUsize>,
        barrier: Arc<Barrier>,
    }

    impl InterceptRule for BlockingRule {
        fn rule_id(&self) -> &'static str {
            "contract-busy-rule-envelope"
        }

        fn needs_content(&self) -> bool {
            true
        }

        fn evaluate(&self, _input: &RuleInput<'_>) -> RuleDecision {
            RuleDecision::Allow
        }

        fn diagnostics_with_limit(
            &self,
            _input: &RuleInput<'_>,
            _mode: &Mode,
            _limit: usize,
        ) -> Vec<Diagnostic> {
            self.started.fetch_add(1, Ordering::SeqCst);
            self.barrier.wait();
            Vec::new()
        }
    }

    let started = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(MAX_CONCURRENT_SCAN_BUFFERS + 1));
    let registry = RuleRegistry::with_rules(vec![Box::new(BlockingRule {
        started: Arc::clone(&started),
        barrier: Arc::clone(&barrier),
    })])
    .expect("contract busy rule has a unique id");
    let service = ScanBufferService::new(EnforcementPipeline::new(registry));
    let harness = Harness::start(service);

    // Saturate the worker permits.
    let mut blockers = Vec::new();
    for idx in 0..MAX_CONCURRENT_SCAN_BUFFERS {
        let mut blocker = harness.connect().await;
        let frame = json!({
            "jsonrpc": "2.0",
            "method": "scan_buffer",
            "params": {
                "path": format!("src/contract/busy-{idx}.ts"),
                "text": "const value = 1;\n",
                "version": 1,
                "mode": "midEdit"
            },
            "id": format!("contract-busy-blocker-{idx}")
        });
        blocker.write(&frame).await;
        blockers.push(blocker);
    }
    // 100 × 10ms = 1s ceiling, matching the public busy contract test.
    for _ in 0..100 {
        if started.load(Ordering::SeqCst) == MAX_CONCURRENT_SCAN_BUFFERS {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(
        started.load(Ordering::SeqCst),
        MAX_CONCURRENT_SCAN_BUFFERS,
        "expected {MAX_CONCURRENT_SCAN_BUFFERS} blocking workers to enter the rule within 1s; \
         the envelope fixture cannot exercise the saturation path until they do",
    );

    // Third connection fails fast with a structured `Server busy`. We
    // assert the universal envelope invariant first, then pin the
    // expected code so a future change to the busy mapping breaks here.
    let mut busy_client = harness.connect().await;
    let busy_request = json!({
        "jsonrpc": "2.0",
        "method": "scan_buffer",
        "params": {
            "path": "src/contract/busy-third.ts",
            "text": "const value = 1;\n",
            "version": 1,
            "mode": "midEdit"
        },
        "id": "contract-busy-third"
    });
    let busy = send_frame(&mut busy_client, &busy_request).await;
    assert_envelope_is_error_or_diagnostics(&busy);
    assert_eq!(busy["error"]["code"], -32000);
    assert_eq!(busy["error"]["message"], "Server busy");

    // Release blockers and tidy up.
    barrier.wait();
    for mut blocker in blockers {
        blocker.drain_pending().await;
    }
    drop(busy_client);
    harness.shutdown().await;
}

#[tokio::test]
#[ignore = "RTAI-008 gap: scan_buffer does not yet accept sessionId; \
            cross-session rejection is not enforced. Wire when the daemon \
            grows session-scoped mid-edit enforcement, then drop the ignore."]
async fn rust_consumer_rejects_cross_session_subscription() {
    let harness = Harness::start(ScanBufferService::default());
    let mut client = harness.connect().await;

    let response = send_frame(&mut client, &cross_session_request()).await;
    assert_cross_session_response(&response);

    drop(client);
    harness.shutdown().await;
}

// ---------------------------------------------------------------------
// Sanity: every fixture name in FIXTURE_NAMES has an exposed pair.
// ---------------------------------------------------------------------

#[test]
fn fixture_names_are_unique_and_non_empty() {
    let mut seen = std::collections::HashSet::new();
    for name in FIXTURE_NAMES {
        assert!(!name.is_empty(), "fixture name must not be empty");
        assert!(
            seen.insert(*name),
            "fixture name {name:?} listed twice in FIXTURE_NAMES",
        );
    }
}

#[test]
fn over_cap_request_has_expected_id() {
    let frame = over_cap_content_request();
    assert_eq!(frame["id"], "contract-over-cap");
    assert_eq!(frame["method"], "scan_buffer");
}

#[test]
fn malformed_request_omits_text_field() {
    let frame = malformed_request();
    assert!(frame["params"].get("text").is_none());
    assert_eq!(frame["method"], "scan_buffer");
}

#[test]
fn invalid_path_request_uses_empty_path() {
    let frame = invalid_path_request();
    assert_eq!(frame["params"]["path"], "");
    assert_eq!(frame["method"], "scan_buffer");
}

#[test]
fn path_too_long_request_exceeds_path_cap() {
    let frame = path_too_long_request();
    let path = frame["params"]["path"].as_str().expect("path string");
    assert!(path.len() > MAX_SCAN_BUFFER_PATH_BYTES);
}

#[test]
fn unsupported_mode_request_uses_unknown_mode() {
    let frame = unsupported_mode_request();
    assert_eq!(frame["params"]["mode"], "saveTime");
    assert_eq!(frame["method"], "scan_buffer");
}

#[test]
fn busy_request_has_expected_shape() {
    let frame = busy_request(7);
    assert_eq!(frame["id"], "contract-busy-7");
    assert_eq!(frame["params"]["path"], "src/contract/busy-7.ts");
    assert_eq!(frame["method"], "scan_buffer");
}
