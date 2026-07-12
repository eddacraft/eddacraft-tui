<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->

# Kindling Daemon Sink

| ID  | Owner      | Status      | Progress |
| --- | ---------- | ----------- | -------- |
| KDS | @eddacraft | Complete | 5/5      |

2026-07-13: all Merged items confirmed in the v0.9.0-beta tag (record:
plans/releases/v0.9.0-beta.md) and advanced to Released/Shipped; module
ready to archive per the archive cascade.

> **All 5 work items Merged (5/5)** — the daemon is now the authoritative store
> for `command.invoked`: `KindlingDaemonSink` over `kindling-client` (KDS-001/-003,
> #2897), `ANVIL_KINDLING_SINK` selection (KDS-002, #2906), views read the daemon
> via `list_observations` unioned with the sidecar (KDS-004, #2945), and the
> bespoke NDJSON writer is retired with the default flipped to `daemon` and the
> spool capped (KDS-005, #2949). The two upstream prerequisites — the kindling
> read API (#2910) and the spool cap (#2916) — landed in `kindling-client` 0.3.
> The module stays **In Progress** only pending a release tag (then Complete +
> archival). The durable-emit layer ships **inside `kindling-client` behind
> `features = ["spool"]`** — there is **no** standalone `kindling-spool` crate. A
> provisional index row is present under
> [Usage Analytics](../../index.aps.md#usage-analytics).

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
- A new crates.io dependency on `kindling-client` (caret `0.3`, `features =
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

- `kindling-client` (crates.io caret `0.3`, `features = ["spool"]`) — the
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
- [x] Dependency pinning policy confirmed — caret `0.3`; the client fails loud
      on schema-version mismatch (`EXPECTED_SCHEMA_VERSION`, currently 5)
- [x] D-035 reconciliation decided — **no reword / new ADR needed** (KDS-002).
      ADR-035 frames Kindling as the _"SQLite-backed, write-once, source of
      truth"_ pipe — that pins Kindling's **role**, not the **write mechanism**.
      The daemon sink realises that SQLite-source-of-truth framing directly (the
      NDJSON sidecar was the workaround), and KDS-002 keeps NDJSON the default
      with the daemon opt-in, so nothing in ADR-035's matrix is contradicted. A
      wording change would only be warranted if/when the daemon becomes the
      **default** authoritative write path (a future graduated flip, not KDS-002).
- [x] Usage-views read-path migration approach agreed + **implemented** (KDS-004)
      — **query the daemon** via `kindling-client` 0.3 `list_observations`
      (KINTEG-003, the #2910 read API), unioned with the sidecar. `retrieve`
      (ranked/capped) and spool-as-cache were both unsuitable. See KDS-004.

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
- **Status:** Released/Shipped via v0.9.0-beta (2026-07-12). Merged 2026-06-24 via PR #2897
- **Dependencies:** none remaining — kindling crates on crates.io (`0.2.0`, Done)

### KDS-002: Wire the daemon sink as primary, with sink selection

- **Intent:** Make the daemon `command.invoked` producer's sink selectable, with
  the spool as fallback, via an operator toggle.
- **Expected Outcome:** `daemon_usage_emitter` (the single construction site the
  JSON-RPC dispatch path consumes via `with_usage_emitter`) resolves
  `ANVIL_KINDLING_SINK` to `daemon | ndjson | off`: `daemon` builds
  `KindlingDaemonSink`; `ndjson` keeps today's `DaemonUsageSink` behaviour; `off`
  disables the daemon producer (no emitter wired); unset / unrecognised →
  `ndjson` (default) with a warn on an unrecognised value. The whole-observation
  break-glass `ANVIL_INTERCEPT_DISABLE_OBSERVATION=1` now also gates this
  producer (it previously didn't — a consent gap vs the documented "silences
  every producer" claim). The privacy-first default is preserved (capture stays
  local-only; the default is unchanged). Documented in the usage-analytics
  runbook. **Default stays `ndjson`** — making the daemon the default
  authoritative write path is a deferred graduated flip (pending a spool
  size/age cap and broad `kindling`-binary availability), not this item.
  **`repo_id`:** the `daemon` sink keeps the client's default project root (its
  CWD); the daemon is per-user and routes per-call via the `X-Kindling-Project`
  header, so authoritative per-call `repo_id` scoping is a KDS-004 follow-up (a
  static workspace root resolved at startup would mis-scope rows when the
  daemon's CWD is not the served project).
- **Validation:** unit tests for the selection resolver (each variant incl.
  case / whitespace / unrecognised); tests that `off` and the break-glass both
  yield no emitter, that `daemon` wires an emitter, and that the default/unset
  path still wires the NDJSON sink (privacy contract unchanged).
- **Status:** Released/Shipped via v0.9.0-beta (2026-07-12). Merged 2026-06-24 via PR #2906
- **Dependencies:** KDS-001 (Merged)

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
- **Status:** Released/Shipped via v0.9.0-beta (2026-07-12). Merged 2026-06-24 via PR #2897
- **Dependencies:** KDS-001

### KDS-004: Re-source the usage views from the authoritative store

- **Intent:** Keep `anvil kindling usage <view>` correct once the daemon is the
  source of truth (the views read `usage.ndjson` today).
- **Expected Outcome:** the views read the **authoritative daemon store** under
  `ANVIL_KINDLING_SINK=daemon`, via the upstream read API that landed in
  **`kindling-client` 0.3** (`list_observations` — exhaustive, keyset-paginated,
  `kind`/scope filtered; KINTEG-003, the read API #2910 asked for). `run_usage`
  paginates `list_observations(kinds=[Command], repo scope)` to completeness,
  parses each observation's `content` back into a `UsageRow`, and **unions** the
  result with the sidecar rows. (The union is required and correct: the CLI
  producer always writes the sidecar while the daemon JSON-RPC producer writes the
  daemon, so the two row sets are disjoint — different invocations — and together
  they are the full picture. This closes exactly the gap the KDS-004 guard
  warned about.) Degrades gracefully to sidecar-only with a stderr note if the
  daemon can't be read; no daemon read under `ndjson`/`off` or when capture is
  disabled. The view logic and output shapes are unchanged. The interim
  source-aware guard is removed (superseded by the real read). A full daemon
  end-to-end cutover (the CLI producer also writing the daemon, so the union
  collapses to a single source) tracks with the CLI-producer migration, outside
  KDS scope.
- **Validation:** `collect_daemon_rows` unit tests against a real in-process
  `kindling-server` 0.3 — exhaustive enumeration across **multiple pages**
  (keyset cursor) and skip-unparseable-content; a CLI integration test that the
  command succeeds and keeps the sidecar rows under the daemon sink. Existing
  `usage_views` tests unchanged (pure view logic).
- **Status:** Released/Shipped via v0.9.0-beta (2026-07-12). Merged 2026-06-26 via PR #2945
- **Dependencies:** KDS-002 (Merged); `kindling-client` 0.3 read API (KINTEG-003,
  #2910 — **landed**)

### KDS-005: Retire the standalone NDJSON writer

- **Intent:** Remove the bespoke `DaemonUsageSink` NDJSON append path for the
  daemon `command.invoked` producer; the only NDJSON that producer leaves behind
  becomes the `SpooledClient` fallback file.
- **Expected Outcome:** `DaemonUsageSink` is deleted; the daemon
  `command.invoked` producer routes **only** through `KindlingDaemonSink`, so the
  **default sink flips from `ndjson` to `daemon`** (owner-approved graduated flip
  — `ANVIL_KINDLING_SINK` is now `daemon` (default) | `off`; the retired `ndjson`
  value resolves to `daemon` with a deprecation warn). The spool owns the
  producer's durability and is now **bounded** (7-day / 64 MiB caps via 0.3
  `SpoolConfig::with_max_bytes` / `with_max_age_ms`), matching the sidecar it
  replaces. No NDJSON fallback on a sink-build failure (degrade to no export).
  The shared `append_observation_to` / `trim_usage_sidecar` helpers **remain** —
  the CLI producer (`usage::record_invocation`) and the DPO `DaemonObservationSink`
  still write the sidecar (both outside KDS scope), so the sidecar persists for
  those; the `anvil kindling usage` views read it unioned with the daemon
  (KDS-004), so "what's used" stays complete under the flip.
- **Validation:** suite green with `DaemonUsageSink` (and its NDJSON-writer tests)
  removed; the selection-resolver tests cover the new default (unset / `ndjson` /
  unrecognised → `daemon`; `off` → off); `usage_observation` confirms the CLI
  producer still writes the sidecar under the new default. The spool caps are
  covered by `kindling-client` 0.3's own retention tests.
- **Status:** Released/Shipped via v0.9.0-beta (2026-07-12). Merged 2026-06-26 via PR #2949
- **Dependencies:** KDS-002 (Merged); KDS-004 (Merged, #2945 — read path so the
  views stay complete under the flip); `kindling-client` 0.3 spool cap (KINTEG-009,
  #2916 — landed)

## Risks

| Risk                                                            | Impact | Mitigation                                                                                  |
| -------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------------- |
| Daemon unreachable at emit time                                 | Low    | `SpooledClient` fallback + client auto-spawn; emit never errors on outage                   |
| Spool grows unbounded under a persistent daemon outage | ~~Medium~~ Resolved | **Fixed (KDS-005):** the spool is now bounded to 7-day / 64 MiB via 0.3 `SpoolConfig::with_max_bytes` / `with_max_age_ms`, matching the sidecar's council-T5 trim |
| Spool file holds usage metadata on a shared host                | Low    | The sink creates the `kindling/` parent dir `0700` before first write, so the dir gates access even though the upstream client writes the spool file without an explicit `0600` mode |
| Networking client leaks into the daemon crate (ADR-064)         | High   | Sink confined to `anvil-cli`; `daemon_dep_boundary` guard enforced; KDS-001 boundary test   |
| D-035 wording vs an active daemon-write path                    | Medium | Reconcile in KDS / a short ADR before Ready                                                  |
| At-least-once replay duplicates a row after a crash mid-flush   | Low    | Stable ids already stamped by the spool; exactly-once is an upstream daemon-dedup follow-up  |
| Cold-spawn reliability / silent cold-start failure              | Low    | Upstream fix (kindling PR #86); upstream follow-up to log cold-start to `~/.kindling/`       |
| Usage views drift if read-path migration (KDS-004) is deferred  | ~~Medium~~ Resolved | **Fixed (KDS-004, #2945):** the views read the daemon (`list_observations`) unioned with the sidecar, so they stay complete under the default flip |

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
3. **Dependency pinning** — **Resolved (KDS-001):** caret `0.3` on
   `kindling-client`; the client checks the daemon's reported schema version
   against its compile-time `EXPECTED_SCHEMA_VERSION` (5) and fails loud on
   mismatch, so Anvil need not track the schema version separately.
4. **Views read-path** — **Resolved + implemented (KDS-004):** query the daemon
   directly via `kindling-client` 0.3 `list_observations` (KINTEG-003, the #2910
   read API), unioned with the sidecar. `retrieve` (ranked/capped) and the
   transient spool were both unsuitable.
5. **`gate.evaluated`** — does it also route to the daemon, or stay a local-only
   signal? (Open; deferred to the KDS-001 fast follow / USAGE-002 context.)
