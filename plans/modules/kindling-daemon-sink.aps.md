<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->

# Kindling Daemon Sink

| ID  | Owner      | Status      | Progress |
| --- | ---------- | ----------- | -------- |
| KDS | @eddacraft | In Progress | 0/5      |

> **In Progress** — the module passed Ready (placement + async-bridge decisions
> settled; see Open Questions, now answered) and KDS-001 + KDS-003 are underway.
> The PORT-011 handoff
> ([`../execution/PORT-011-anvil-handoff.md`](../execution/PORT-011-anvil-handoff.md))
> supersedes the original draft on the spool transport. The Kindling
> prerequisite (KINTEG-001) is **Done** — all seven Kindling crates published at
> `0.2.0` on crates.io (2026-06-24). The durable-emit layer ships **inside
> `kindling-client` behind `features = ["spool"]`** — there is **no** standalone
> `kindling-spool` crate. A provisional index row is present under
> [Usage Analytics](../index.aps.md#usage-analytics).

## Cross-cutting convention

Observations are produced behind the existing `KindlingObservationSink` trait
(`crates/anvil-intercept/src/kindling_observation.rs`). This module adds a new
**sink implementation** — it does NOT change the producers (the USAGE
`command.invoked` / `gate.evaluated` emitters) or the observation shapes. The
sink lives in the **app layer** (`anvil-cli`), never inside `anvil-intercept`,
so no networking client crosses the ADR-064 daemon dependency boundary.

## Purpose

Anvil's governance observations currently land in a per-user `usage.ndjson`
sidecar (`<credentials_dir>/kindling/usage.ndjson`), written by the USAGE
module's `DaemonUsageSink`. That NDJSON path was a **workaround**: Kindling was
not reachable in-process when USAGE shipped, so Anvil appended observations to a
local file and `anvil kindling usage <view>` read them back.

The Rust-canonical Kindling port has since shipped a local daemon
(`kindling serve`, HTTP/1 over a Unix domain socket), a thin auto-spawning Rust
client (`kindling-client`), and a durable-emit layer (`SpooledClient`, shipped
inside `kindling-client` behind `features = ["spool"]` — there is **no**
standalone `kindling-spool` crate). These crates are **published at `0.2.0` on
crates.io** (KINTEG-001, 2026-06-24), so the publish prerequisite is satisfied.
Once Kindling is properly reachable, the **normal path** is:

```
Anvil event → validate/redact observation → Kindling client → SQLite-backed Kindling store
```

This module makes the **Kindling daemon (SQLite) the authoritative store** for
Anvil's observations, with NDJSON demoted to a **transient fallback spool** (via
`kindling-client`'s `SpooledClient`) rather than a parallel source of truth. That removes the
duplicate-ingestion / replay-edge-case / retention-drift / "which store is
authoritative?" ambiguity the NDJSON-primary design carries — see
**D-035** (the three-pipe rule: "Kindling = governance facts, write-once,
source-of-truth"); this module is the write-side realisation of D-035 and may
need a short ADR to reconcile the wording.

## In scope

- A `KindlingDaemonSink` implementing the `KindlingObservationSink` emit
  methods — `try_emit_command_invoked` for `command.invoked` and `try_emit` for
  `gate.evaluated` — in `anvil-cli` (app layer), backed by
  `kindling_client::spool::SpooledClient` over `kindling_client::Client`.
- A new crates.io dependency on `kindling-client` (caret `0.2`, `features =
  ["spool"]`), pinned per Anvil's dependency policy. The `kindling` daemon
  binary is auto-spawned by the client on first call.
- Mapping `CommandInvokedObservation` / `GateEvaluatedObservation` →
  Kindling `append_observation` (kind / content / provenance / scope ids),
  applying Anvil's existing TRACE-003 redaction before the call (Kindling adds
  its own non-bypassable secret masking at the service boundary).
- Wiring the daemon sink as the **primary** sink for the USAGE producers, with
  the spool as the fallback when the daemon is unreachable (the emit method
  spools the row and returns `Ok(())` → replayed on the next reachable call / on
  `flush`).
- A config flag selecting the sink (`daemon` | `ndjson` | `off`), preserving the
  privacy-first default (Kindling stays opt-in / locally-scoped).
- Re-sourcing `anvil kindling usage <view>` from the daemon (or the spool) so
  the query views stay correct once the authoritative store moves.
- Retiring the standalone `DaemonUsageSink` NDJSON writer in favour of the
  shared `kindling-spool` file.

## Out of scope

- The Kindling daemon, its schema, or its storage (upstream, eddacraft/kindling).
- New observation kinds or producer changes (USAGE owns the producers).
- Exactly-once delivery — the `SpooledClient` is at-least-once in v1;
  exactly-once needs daemon-side dedup-on-id (an upstream Kindling follow-up,
  KINTEG-002).
- Cross-language NDJSON handoff to a TS Kindling consumer (the TS implementation
  packages are deprecated; the daemon is canonical).

## Interfaces

**Depends on:**

- `kindling-client` (crates.io caret `0.2`, `features = ["spool"]`) — the
  transport, auto-spawn, and durable-emit (`SpooledClient`) layers in one crate.
  The daemon (SQLite) is authoritative; the spool is a transient buffer drained
  into it.
- The existing `KindlingObservationSink` trait + observation types in
  `anvil-intercept` (unchanged).
- USAGE (Done) — the `command.invoked` / `gate.evaluated` producers this sink
  receives from.
- D-035 — the three-pipe observability rule; this module realises its write side
  (possible reconciling ADR).

**Exposes:**

- `KindlingDaemonSink` (in `anvil-cli`) — a drop-in `KindlingObservationSink`.
- A sink-selection config surface (`daemon` | `ndjson` | `off`).

**Boundary note (ADR-064):** the networking client stays in `anvil-cli`, never
`anvil-intercept` — the daemon crate must not gain an HTTP/tokio dependency. The
`daemon_dep_boundary` guard should continue to pass unchanged.

## Ready Checklist

- [x] `kindling-client` (`spool` feature) published to crates.io — `0.2.0`
      landed (KINTEG-001, 2026-06-24); no standalone `kindling-spool` crate
- [x] Sink placement confirmed — `anvil-cli` for PORT-011 (extract a small
      `anvil-kindling` adapter crate later if the JSON-RPC path also needs it)
- [x] Dependency pinning policy confirmed — caret `0.2`; the client fails loud
      on schema-version mismatch (`EXPECTED_SCHEMA_VERSION`, currently 5)
- [ ] D-035 reconciliation decided (reword vs short ADR) — deferred to KDS-002,
      the wiring change that actually makes the daemon the primary write path
- [ ] Usage-views read-path migration approach agreed (KDS-004)

## Work Items

> Status: In Progress. The kindling crates have landed on crates.io (`0.2.0`),
> unblocking the module. KDS-001 + KDS-003 are the PORT-011 proof slice
> (`command.invoked` only); KDS-002/004/005 follow.

### KDS-001: `KindlingDaemonSink` over the spooled client

- **Intent:** A `KindlingObservationSink` implementation that writes an
  observation to the Kindling daemon, falling back to the spool when it is
  unreachable.
- **Expected Outcome:** `KindlingDaemonSink` (in `anvil-cli`) holds a
  `kindling_client::spool::SpooledClient`; the emit methods map the Anvil
  observation to a Kindling `ObservationInput` (kind/content/provenance/scope
  ids, with TRACE-003 redaction applied first) and call `append_observation` —
  `command.invoked` via `try_emit_command_invoked` for the PORT-011 slice
  (`gate.evaluated` via `try_emit` is a fast follow). A daemon outage spools the
  row and returns `Ok(())` (the outage is never surfaced as an error to the
  caller); a daemon API rejection (`Api`/`SchemaMismatch`) propagates as
  `KindlingSinkError`. The sync `KindlingObservationSink` trait is bridged to the
  async `SpooledClient` via an owned current-thread tokio runtime, only ever
  driven on the `NonBlockingObservationSink` drain thread (never the hot path).
  New crates.io dep added; `daemon_dep_boundary` guard still green (client
  confined to `anvil-cli`).
- **Validation:** unit tests against an in-process / temp-socket daemon —
  delivered-when-up (row retrievable), spooled-when-down, replay-on-reconnect,
  rejection-propagates; the existing `daemon_dep_boundary` guard asserts
  `anvil-intercept` gained no networking dep.
- **Status:** In Progress
- **Dependencies:** none remaining — kindling crates on crates.io (`0.2.0`, Done)

### KDS-002: Wire the daemon sink as primary, with sink selection

- **Intent:** Route the USAGE producers through `KindlingDaemonSink` by default,
  with the spool as fallback and a config flag to choose the sink.
- **Expected Outcome:** the producer wiring (`daemon_usage_emitter` and the
  JSON-RPC path) constructs `KindlingDaemonSink` when the resolved sink is
  `daemon`; `ndjson` keeps today's behaviour; `off` disables capture. The
  privacy-first default is preserved (Kindling opt-in, local-only). Documented in
  the usage-analytics runbook.
- **Validation:** tests covering each sink variant; a default-config test
  confirming the privacy contract is unchanged.
- **Status:** Proposed
- **Dependencies:** KDS-001

### KDS-003: Daemon-vs-NDJSON parity

- **Intent:** Prove the daemon-written observation is equivalent to the
  NDJSON/TS-bridge output for the same input (the PORT-011 acceptance).
- **Expected Outcome:** a parity test emits the same `CommandInvokedObservation`
  via both the NDJSON sink and the daemon sink and asserts the persisted rows
  match (kind/content/provenance/scope, modulo daemon-assigned id/timestamp);
  spool flush/replay lands the row identically after a simulated outage.
- **Validation:** the parity test above, green in CI. (Lives as a `#[cfg(test)]`
  module beside the sink, not a `tests/` integration file — `anvil-cli` is a
  bin-only crate with no library target for `tests/` to link against.)
- **Status:** In Progress
- **Dependencies:** KDS-001

### KDS-004: Re-source the usage views from the authoritative store

- **Intent:** Keep `anvil kindling usage <view>` correct once the daemon is the
  source of truth (the views read `usage.ndjson` today).
- **Expected Outcome:** the views query the daemon (via `kindling-client`
  retrieval / a read path) — or, as an interim, read the spool file as a local
  cache — without changing the view semantics or output shapes. The chosen
  approach is recorded.
- **Validation:** existing `usage_views` tests pass against the new read source;
  a view returns the same rows whether they were delivered live or via spool
  replay.
- **Status:** Proposed
- **Dependencies:** KDS-002

### KDS-005: Retire the standalone NDJSON writer

- **Intent:** Remove the bespoke `DaemonUsageSink` NDJSON append path; the only
  NDJSON in the system becomes `kindling-spool`'s fallback file.
- **Expected Outcome:** `DaemonUsageSink` and its hand-rolled append/rotation
  logic are deleted; the spool owns durability; docs/runbook updated; no
  behaviour change when the daemon is reachable.
- **Validation:** the suite is green with `DaemonUsageSink` removed; a
  daemon-down run still durably buffers (now via the spool).
- **Status:** Proposed
- **Dependencies:** KDS-002, KDS-004

## Risks

| Risk                                                            | Impact | Mitigation                                                                                  |
| -------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------- |
| Daemon unreachable at emit time                                 | Low    | `SpooledClient` fallback + client auto-spawn; emit never errors on outage                   |
| Spool grows unbounded under a persistent daemon outage (no age/size cap like the NDJSON sidecar's council-T5 trim) | Medium | KDS-001 documents the limitation; trimming the spool without dropping un-delivered rows belongs in `SpooledClient` (its `SpoolConfig` reserves size-cap knobs) — tracked for KDS-002 / upstream. Opt-in + default-off bounds exposure |
| Spool file holds usage metadata on a shared host                | Low    | The sink creates the `kindling/` parent dir `0700` before first write, so the dir gates access even though the upstream client writes the spool file without an explicit `0600` mode |
| Networking client leaks into the daemon crate (ADR-064)         | High   | Sink confined to `anvil-cli`; `daemon_dep_boundary` guard enforced; KDS-001 boundary test   |
| D-035 wording vs an active daemon-write path                    | Medium | Reconcile in KDS / a short ADR before Ready                                                  |
| At-least-once replay duplicates a row after a crash mid-flush   | Low    | Stable ids already stamped by the spool; exactly-once is an upstream daemon-dedup follow-up  |
| Cold-spawn reliability / silent cold-start failure              | Low    | Upstream fix (kindling PR #86); upstream follow-up to log cold-start to `~/.kindling/`       |
| Usage views drift if read-path migration (KDS-004) is deferred  | Medium | Sequence KDS-004 before KDS-005; spool-as-read-cache interim keeps views working             |

## Open questions

1. **Sink crate placement** — **Resolved (KDS-001):** `KindlingDaemonSink` lives
   in `anvil-cli` for the PORT-011 proof. Extracting a small `anvil-kindling`
   adapter crate is deferred to whenever a second consumer (e.g. the daemon's
   JSON-RPC producer) needs the sink; until then the CLI is the only caller.
2. **Async bridge** — **Resolved (KDS-001):** the sync `KindlingObservationSink`
   trait is bridged to the async `SpooledClient` via a current-thread tokio
   runtime owned by the sink, `block_on`-driven only on the
   `NonBlockingObservationSink` drain thread (never the dispatch / save-time hot
   path). No ambient runtime is assumed, so the bridge is safe off the daemon's
   own event loop.
3. **Dependency pinning** — **Resolved (KDS-001):** caret `0.2` on
   `kindling-client`; the client checks the daemon's reported schema version
   against its compile-time `EXPECTED_SCHEMA_VERSION` (5) and fails loud on
   mismatch, so Anvil need not track the schema version separately.
4. **Views read-path** — query the daemon directly, or treat the spool as a
   read-through cache for `anvil kindling usage`? (Open; affects KDS-004 scope.)
5. **`gate.evaluated`** — does it also route to the daemon, or stay a local-only
   signal? (Open; deferred to the KDS-001 fast follow / USAGE-002 context.)
