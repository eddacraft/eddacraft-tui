<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->

# Kindling Daemon Sink

| ID  | Owner      | Status      | Progress |
| --- | ---------- | ----------- | -------- |
| KDS | @eddacraft | In Progress | 3/5      |

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
- [x] D-035 reconciliation decided — **no reword / new ADR needed** (KDS-002).
      ADR-035 frames Kindling as the _"SQLite-backed, write-once, source of
      truth"_ pipe — that pins Kindling's **role**, not the **write mechanism**.
      The daemon sink realises that SQLite-source-of-truth framing directly (the
      NDJSON sidecar was the workaround), and KDS-002 keeps NDJSON the default
      with the daemon opt-in, so nothing in ADR-035's matrix is contradicted. A
      wording change would only be warranted if/when the daemon becomes the
      **default** authoritative write path (a future graduated flip, not KDS-002).
- [x] Usage-views read-path migration approach agreed (KDS-004) — **query the
      daemon**, Blocked on an upstream kindling list/aggregate read API
      (anvil-001#2910); `retrieve` (ranked/capped) and spool-as-cache are both
      unsuitable. A source-aware guard ships in the interim. See KDS-004.

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
- **Status:** Merged 2026-06-24 via PR #2897
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
- **Status:** Merged 2026-06-24 via PR #2906
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
- **Status:** Merged 2026-06-24 via PR #2897
- **Dependencies:** KDS-001

### KDS-004: Re-source the usage views from the authoritative store

- **Intent:** Keep `anvil kindling usage <view>` correct once the daemon is the
  source of truth (the views read `usage.ndjson` today).
- **Expected Outcome:** _(chosen approach, recorded)_ **query the daemon** is the
  only correct path — but it is **Blocked on an upstream kindling read API**
  (tracked in
  [eddacraft/anvil-001#2910](https://github.com/eddacraft/anvil-001/issues/2910)).
  The four views compute exact **counts** and a **set-difference**
  (`never_invoked`) over _all_ `command.invoked` rows; `kindling-client` 0.2's
  only observation-read endpoint is `retrieve` — _deterministic **ranked**
  retrieval capped by `max_candidates`_ — which returns top-K candidates, not an
  exhaustive list, so it yields wrong counts and false "never invoked" results.
  The **spool-as-cache** alternative is rejected: the spool drains into the
  daemon on delivery, so it is empty in the normal daemon-up case and never holds
  the full record. A **dual-write local cache** is rejected too — it reintroduces
  the duplication KDS-001 demoted and KDS-005 must then undo. **Interim shipped:**
  a **source-aware guard** — under `ANVIL_KINDLING_SINK=daemon` the
  `anvil kindling usage` command warns (stderr, so `--json` stdout stays clean)
  that the views read the local sidecar, which is not authoritative under the
  daemon sink, and points at #2910. This stops silently-misleading empty/stale
  output for the opt-in / default-off daemon-sink users; the views and their
  output shapes are unchanged.
- **Validation:** done for the interim — a unit test (`sidecar_source_warning`)
  and CLI integration tests assert the note fires only under the daemon sink and
  not by default. The full acceptance (views query the daemon; same rows live vs
  spool-replay; existing `usage_views` tests pass against the daemon source) is
  deferred to the upstream API landing (#2910).
- **Status:** Blocked
- **Dependencies:** KDS-002 (Merged); upstream kindling list/aggregate read API
  (anvil-001#2910)

### KDS-005: Retire the standalone NDJSON writer

- **Intent:** Remove the bespoke `DaemonUsageSink` NDJSON append path; the only
  NDJSON in the system becomes the `SpooledClient` fallback file.
- **Expected Outcome:** `DaemonUsageSink` and its hand-rolled append/rotation
  logic are deleted; the spool owns durability; docs/runbook updated; no
  behaviour change when the daemon is reachable.
- **Blocked — cannot proceed yet (verified 2026-06-24).** Three blockers, two of
  them on upstream kindling work:
  1. **Depends on KDS-004 (Blocked on anvil-001#2910).** Deleting `DaemonUsageSink`
     forces the daemon `command.invoked` producer to daemon-only (the default
     flips off `ndjson`). The `anvil kindling usage` views read `usage.ndjson` and
     cannot read the daemon until KDS-004's read path lands — so with the daemon
     **reachable** the rows go to the daemon and the views lose them, directly
     violating this item's own "no behaviour change when the daemon is reachable"
     acceptance.
  2. **Retention regression — needs a spool size/age cap (anvil-001#2916).** The
     NDJSON sidecar is trimmed to a rolling 7-day / 64 MiB window; the
     `SpooledClient` spool has no cap (its `SpoolConfig` reserves the knob), so
     "the spool owns durability" would lose retention and grow unbounded under a
     prolonged outage.
  3. **Shared append logic.** `DaemonUsageSink` and the still-active **CLI**
     producer (`usage::record_invocation`) both use `append_usage_observation_to`
     → `append_observation_to` + `trim_usage_sidecar`; the "hand-rolled
     append/rotation logic" cannot be deleted while the CLI path (out of KDS
     scope) still uses it.
- **Validation:** the suite is green with `DaemonUsageSink` removed; a
  daemon-down run still durably buffers (now via the spool). _(Deferred — see
  Blocked above.)_
- **Status:** Blocked
- **Dependencies:** KDS-002 (Merged); KDS-004 (Blocked on anvil-001#2910); a
  spool size/age cap (anvil-001#2916)

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
4. **Views read-path** — **Resolved (KDS-004):** query the daemon directly is the
   only correct path, but it is Blocked on an upstream kindling list/aggregate
   read API (anvil-001#2910) — `retrieve` is ranked/capped (wrong counts) and the
   spool is transient. A source-aware guard ships in the interim; the full
   re-source awaits the API.
5. **`gate.evaluated`** — does it also route to the daemon, or stay a local-only
   signal? (Open; deferred to the KDS-001 fast follow / USAGE-002 context.)
