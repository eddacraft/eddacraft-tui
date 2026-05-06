# @eddacraft/anvil-driver-client

Shared TypeScript driver-client library for the `anvil-intercept`
daemon. Editor and future TS-side driver surfaces use it to speak the
daemon's JSON-RPC 2.0 + NDJSON protocol with platform-correct
transport selection (Unix domain socket on Linux/macOS, named pipe on
Windows), structured timeouts, transparent reconnection, and
reliability-budget quarantine.

Lands DRVR-001 of the [`surface-drivers`](../../plans/modules/surface-drivers.aps.md)
APS module; sits behind ADR-030 and consumes the daemon contracts
pinned by ADR-015.

## Public API

```ts
import { DriverClient } from '@eddacraft/anvil-driver-client';

const client = new DriverClient({
  driverIdentity: 'driver-id-from-handshake', // INTD-015 originating_driver_id
});
await client.connect();

const status = await client.request('session.list');
const unsubscribe = client.subscribe('anvil/publishDiagnostics', (event) => {
  for (const diagnostic of event.diagnostics) {
    // render…
  }
});

await client.close();
```

### Per-request timeouts

Defaults match the DRVR-001 brief:

- 10 s for read-only methods (`session.list`, `session.heartbeat`, …)
- 500 ms for enforcement-ack methods (`anvil/enforcement/ack`,
  `anvil/enforcement/refuse`)

Override per call via `request(method, params, { timeoutMs })` or per
client via `new DriverClient({ timeoutsMs: { ... } })`. Hitting the
timeout rejects with `{ error: 'anvil-daemon-timeout', retriable: true }`.

### Reconnect backoff

Exponential, capped at 30 s, 5 attempts before bubbling
`anvil-daemon-unavailable`. Configurable via the `reconnect` option
(see source for the full shape). The client emits `reconnecting` /
`reconnect_failed` / `disconnected` events through
`client.on(event, handler)` for status-bar UX.

### Reliability-budget quarantine

Per [DRVR-007 §2.3a](../../plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md#23a-driver-trust-boundary-v1)
- §2.6 the quarantine ledger keys on the **daemon-minted
`originating_driver_id`** — never on the self-declared `driverName`.
Pass it via `driverIdentity` in the constructor (or
`client.setDriverIdentity()` once the handshake observes it).

After `failureThreshold` (default 5) failures inside
`windowMs` (default 60 s), the client refuses subsequent requests for
`cooldownMs` (default 5 min) with `anvil-driver-quarantined`. The
ledger is in-process; it survives transport reconnect because the
`DriverClient` instance retains the `ReliabilityBudget`. Cross-process
persistence is not yet implemented — see the
`QUARANTINE_PERSISTENCE_NOTE` in the source for the schema if you need
to wire your own store.

### Structured errors

All rejection paths surface a `DriverClientError` with stable
`code` + `retriable` fields. `code` is one of:

- `anvil-daemon-timeout` *(retriable)*
- `anvil-daemon-unavailable` *(retriable)*
- `anvil-daemon-transport-drop` *(retriable)*
- `anvil-daemon-error` *(retriable iff daemon code is server-busy /
  scan-timeout)*
- `anvil-daemon-wrong-owner` *(NOT retriable)*
- `anvil-daemon-invalid-request` *(NOT retriable)*
- `anvil-driver-quarantined` *(NOT retriable)*
- `anvil-driver-closed` *(NOT retriable)*

`err.toJSON()` returns the wire-stable `{ error, retriable, message,
data?, timeout_ms? }` shape.

## Wrong-owner refusal

`connect()` refuses a Unix socket whose parent directory is not owned
by the current user with mode `0700`, or whose own mode is not `0600`.
This mirrors `crates/anvil-intercept/src/ipc.rs::validate_socket_path_for_client`
on the daemon side — the daemon also refuses; the client adds a
defence-in-depth gate so a hostile peer cannot trick the consumer
into sending content to a socket the user did not control. On Windows
the client validates the pipe-name pattern (`\\.\pipe\anvil-intercept-<sid>`);
the deeper ACL check is documented as a deferred gap (see source).

## Diagnostic shape

The client mirrors the canonical `anvil.diagnostic.v1` shape from
`crates/anvil-kernel-types/src/diagnostics.rs`. The Rust side is
authoritative; if the two drift, **the Rust side wins** and the TS
mirror is updated to match. See `src/diagnostics/types.ts`.

## Testing

```bash
pnpm --filter @eddacraft/anvil-driver-client test
pnpm --filter @eddacraft/anvil-driver-client typecheck
```

Unit tests run against an in-memory fake transport. The integration
test (`src/__tests__/integration-real-daemon.test.ts`) requires a
built daemon binary at `target/{debug,release}/eddacraft-anvil-intercept`.
If the binary is missing the test skips gracefully — the standard
validation gate runs without it; CI's Rust job is responsible for
building the daemon before invoking this suite.

## Out-of-scope (deferred)

- DRVR-002 protocol pinning + capability handshake. The client
  speaks the wire format already pinned by INTD; the structured
  handshake / capability negotiation is a future delivery item.
- DRVR-008 capability negotiation for non-VSCode LSP clients.
- DRVR-003 VSCode extension cutover (deferred under ADR-033).
- MCP-driver bridge (RMCP / RMCPF own that path).
- Cross-process persistence of the reliability-budget ledger.
- `SO_PEERCRED` post-connect verification on Linux (Node does not
  expose it as a stable API; the pre-connect path-stat gate is the
  v1 shape; see source for the documented TOCTOU note).
