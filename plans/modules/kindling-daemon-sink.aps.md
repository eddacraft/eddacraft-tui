<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->

# Kindling Daemon Sink

| ID  | Owner      | Status   | Progress |
| --- | ---------- | -------- | -------- |
| KDS | @eddacraft | Proposed | 0/5      |

> **DRAFT** — authored from the Kindling side (the Rust-canonical Kindling port
> shipped the daemon + `kindling-client` + `kindling-spool`). Needs Anvil-side
> review + council before it moves to Ready. A provisional index row is already
> present under [Usage Analytics](../index.aps.md#usage-analytics); on acceptance,
> confirm that placement (or move it to a new "Memory" section).

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
(`kindling serve`, HTTP/1 over a Unix domain socket), a thin Rust client
(`kindling-client`, auto-spawning), and a durable-emit layer
(`kindling-spool`). These crates are code-complete upstream; their crates.io
publish (`>=0.1`) is **queued, not yet landed** — which is why this module
stays Blocked on the publish (see the Ready Checklist and Work Items). Once
Kindling is properly reachable, the **normal path** is:

```
Anvil event → validate/redact observation → Kindling client → SQLite-backed Kindling store
```

This module makes the **Kindling daemon (SQLite) the authoritative store** for
Anvil's observations, with NDJSON demoted to a **transient fallback spool** (via
`kindling-spool`) rather than a parallel source of truth. That removes the
duplicate-ingestion / replay-edge-case / retention-drift / "which store is
authoritative?" ambiguity the NDJSON-primary design carries — see
**D-035** (the three-pipe rule: "Kindling = governance facts, write-once,
source-of-truth"); this module is the write-side realisation of D-035 and may
need a short ADR to reconcile the wording.

## In scope

- A `KindlingDaemonSink` implementing the `KindlingObservationSink` emit
  methods — `try_emit_command_invoked` for `command.invoked` and `try_emit` for
  `gate.evaluated` — in `anvil-cli` (app layer), backed by
  `kindling-spool::SpooledClient` over `kindling-client::Client`.
- New crates.io dependencies on `kindling-client` and `kindling-spool`
  (`>=0.1`), pinned per Anvil's dependency policy. The `kindling` daemon binary
  is auto-spawned by the client on first call (the cold-spawn `--daemonize`
  path was fixed upstream in kindling PR #86).
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
- Exactly-once delivery — `kindling-spool` is at-least-once in v1; exactly-once
  needs daemon-side dedup-on-id (an upstream Kindling follow-up).
- Cross-language NDJSON handoff to a TS Kindling consumer (the TS implementation
  packages are deprecated; the daemon is canonical).

## Interfaces

**Depends on:**

- `kindling-client` / `kindling-spool` (crates.io `>=0.1`) — the transport,
  auto-spawn, and durable-emit layers. The daemon (SQLite) is authoritative; the
  spool is a transient buffer drained into it.
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

- [ ] `kindling-client` + `kindling-spool` published to crates.io (`>=0.1`)
- [ ] D-035 reconciliation decided (reword vs short ADR)
- [ ] Sink placement confirmed (`anvil-cli` vs a small `anvil-kindling` adapter
      crate) — see Open Questions
- [ ] Usage-views read-path migration approach agreed (KDS-004)
- [ ] Dependency pinning policy for the kindling crates confirmed

## Work Items

> Status: Proposed. Blocked on the kindling crates landing on crates.io
> (`>=0.1`); all work items are author-side estimates pending Anvil review.

### KDS-001: `KindlingDaemonSink` over the spooled client

- **Intent:** A `KindlingObservationSink` implementation that writes an
  observation to the Kindling daemon, falling back to the spool when it is
  unreachable.
- **Expected Outcome:** `KindlingDaemonSink` (in `anvil-cli`) holds a
  `kindling_spool::SpooledClient`; the emit methods map the Anvil observation to
  a Kindling `ObservationInput` (kind/content/provenance/scope ids, with
  TRACE-003 redaction applied first) and call `append_observation` —
  `command.invoked` via `try_emit_command_invoked`, `gate.evaluated` via
  `try_emit`. A daemon outage spools the row and returns `Ok(())` (the outage is
  never surfaced as an error to the caller); a daemon `Rejected` propagates as
  `KindlingSinkError`. New crates.io deps added; `daemon_dep_boundary` guard
  still green (client confined to `anvil-cli`).
- **Validation:** unit tests against an in-process / temp-socket daemon —
  delivered-when-up (row retrievable), spooled-when-down, replay-on-reconnect,
  rejection-propagates; a boundary test asserting `anvil-intercept` gained no
  networking dep.
- **Status:** Proposed
- **Dependencies:** kindling crates on crates.io (`>=0.1`)

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
- **Validation:** the parity test above, green in CI.
- **Status:** Proposed
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
| Daemon unreachable at emit time                                 | Low    | `kindling-spool` fallback + client auto-spawn; emit never errors on outage                  |
| Networking client leaks into the daemon crate (ADR-064)         | High   | Sink confined to `anvil-cli`; `daemon_dep_boundary` guard enforced; KDS-001 boundary test   |
| D-035 wording vs an active daemon-write path                    | Medium | Reconcile in KDS / a short ADR before Ready                                                  |
| At-least-once replay duplicates a row after a crash mid-flush   | Low    | Stable ids already stamped by the spool; exactly-once is an upstream daemon-dedup follow-up  |
| Cold-spawn reliability / silent cold-start failure              | Low    | Upstream fix (kindling PR #86); upstream follow-up to log cold-start to `~/.kindling/`       |
| Usage views drift if read-path migration (KDS-004) is deferred  | Medium | Sequence KDS-004 before KDS-005; spool-as-read-cache interim keeps views working             |

## Open questions

1. **Sink crate placement** — keep `KindlingDaemonSink` in `anvil-cli`, or
   introduce a small `anvil-kindling` adapter crate (still app-layer) so the
   integration is reusable beyond the CLI (e.g. the daemon's JSON-RPC producer)?
2. **Views read-path** — query the daemon directly, or treat the spool as a
   read-through cache for `anvil kindling usage`? (Affects KDS-004 scope.)
3. **Dependency pinning** — exact-version pin vs caret on the kindling crates;
   and whether Anvil tracks the daemon's schema version explicitly.
4. **`gate.evaluated`** — does it also route to the daemon, or stay a local-only
   signal? (USAGE-002 context.)
