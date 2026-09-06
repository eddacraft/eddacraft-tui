# ADR-141: MCP serve registers a client-attributed live session

## Status

Accepted

## Date

2026-09-07

## Context

Protection reporting promotes an MCP client to `McpTier::LiveValidation` — the
tier that claims live pre-write protection inside this repo — using daemon
attestation evidence (`activation/daemon_evidence.rs`). Today that evidence
cannot distinguish an open editor from a closed one, because nothing in the MCP
path ever tells the daemon a client is attached:

- `anvil mcp serve --stdio` never calls `session.register` and never
  heartbeats. It is invisible to the daemon.
- Live, client-attributed leases come only from `anvil run <agent>`.
- `anvil start` records a **durable** activation-spine membership whose
  `last_heartbeat_unix` is frozen at registration by design (ACTMO-014,
  CIB-384).

So for the ordinary user — editor open, anvil configured as an MCP server, not
launched through `anvil run` — the only session for the worktree is the durable
spine record, which is byte-identical whether the editor is running or was
closed hours ago. `evaluate_and_promote` then promotes the sole
handshake-verified client through its unique-unattributed fallback, and
`RestartHandshakeVerified` itself only ever meant "anvil's own disposable probe
spawned the configured command successfully" — not "a client is attached".

JREL-002 requires that a configured but closed editor never gains live
pre-write status from durable registration or a disposable handshake probe, and
that stale evidence from one client cannot borrow another client's freshness.
Neither is achievable at the evidence layer alone: the missing input is a live
signal, not a better inference over the existing ones.

The scale constraint is real. This workstation runs 95 concurrent
`anvil mcp serve --stdio` processes across 34 parents, so any per-session cost
is multiplied by ~100, and `dos.rs` caps the daemon at
`DEFAULT_MAX_CONCURRENT_CONNECTIONS = 64` with a `DEFAULT_IDLE_TIMEOUT` of one
minute.

## Decision

`anvil mcp serve --stdio` registers a **live, client-attributed session** with
the intercept daemon, using the existing transient-connection
register/heartbeat/unregister lane:

- Identity comes from the MCP handshake's `clientInfo.name` (legacy
  `initialize.params.clientInfo`, modern
  `_meta["io.modelcontextprotocol/clientInfo"]`), mapped to an `McpClientId`
  through the same token table that already attributes surfaces.
- The session registers with `AgentTag { driver_id: "anvil-mcp",
  claimed_agent_id: <client label>, pid_starttime }`, so the surface identifier
  is `anvil-mcp/<client>#<starttime>` and existing attribution matches it
  unchanged. `claimed_agent_id` is never `activation-spine`, so the record is a
  live lease and not durable membership.
- A background thread heartbeats every `HEARTBEAT_INTERVAL` (10 s, inside the
  30 s registry TTL) and the session unregisters on process exit.
- A client that never identifies itself is never registered. No attribution
  means no promotion, by design.

Promotion to `LiveValidation` then requires per-client live evidence: a
participating surface that names the client **and** whose own session is a live
lease inside the freshness window. Durable membership and the disposable probe
continue to attest the *worktree* (`DaemonAttestation::Enforced`, the ACTMO-003
MCP-optional success case) but can no longer promote any client.

`anvil-run`'s heartbeat thread is changed from a 50 ms sleep-poll to a
condvar-based `wait_timeout`, so idle cost is one wakeup per interval rather
than twenty per second.

## Rationale

Transient connections are what make this scale. Each heartbeat connects,
writes one line, and closes, so ~95 sessions produce ~9.5 brief connections per
second with only a handful concurrent at any instant — comfortably under the
64-connection cap.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Transient register + heartbeat (chosen) | Reuses the proven `anvil run` lane unchanged; stays far under the connection cap; no new daemon-side lifetime rules | Closed-editor detection lags up to the 30 s TTL; sustained ~9.5 connects/s at this scale |
| Connection-held liveness (session lives as long as the socket) | Immediate closure detection; no heartbeat traffic | 95 held connections against a cap of 64 would starve `anvil status`, pre-write validation and `anvil run`; the 1-minute idle timeout forces keepalives anyway, erasing most of the benefit |
| Evidence layer only, no new signal | Smallest diff | Re-parks every spine-registered worktree at `ready_restart_required` — precisely the regression CIB-384 fixed — because no live signal exists to replace durable evidence |

## Consequences

- **Positive:** protection claims become evidence-backed per client. A closed
  editor drops to `RestartHandshakeVerified` within the TTL; two clients are
  independently attributed; one client's staleness can no longer borrow
  another's freshness.
- **Positive:** the daemon gains real visibility of attached MCP clients, which
  `anvil status` and future readiness work (JREL-005) can consume.
- **Negative:** ~95 sessions at this scale add ~9.5 short-lived connections per
  second and ~95 extra `SessionRecord`s to every `DaemonStatusV1` snapshot.
- **Risks:** `QueryStatus` is unscoped and `mcp/validation.rs` fetches the full
  snapshot on the **pre-write** path, so snapshot growth is paid on a hot path.
- **Mitigations:** the `park_timeout` change removes the dominant idle cost; a
  worktree-scoped attestation query is filed as follow-up work rather than
  bundled here, so the hot-path payload can shrink independently of this
  decision.

## References

- Related ADRs: ADR-094 (registration semantics), ADR-101 (save-time driver
  observability), ADR-120
- APS modules: JREL-002, JREL-005, MCPLH-005, ACTMO-003, ACTMO-014, CIB-384
- Wire contract: `crates/anvil-intercept-proto/src/lib.rs` `IpcCommand`
