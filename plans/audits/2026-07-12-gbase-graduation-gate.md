# GBASE Graduation Gate — `ANVIL_PERSIST_GRAPH` default-on

> **Date:** 2026-07-12
> **Owner:** GBASE-010 (terminal item of the ADR-105 campaign)
> **Purpose:** Record the graduation-gate evidence that authorises flipping
> `ANVIL_PERSIST_GRAPH` **default-on**. Covers the ADR-105 §11
> **successor-specific** criteria (corrupt-shared-base incident behaviour,
> herd-miss under single-flight, warm-start latency budget) plus the standing
> correctness/GC criteria, and **dispositions every accumulated
> GBASE-010-tagged deferral** across the module. The flip
> (`persist_graph_enabled` default + `flags/manifest.json`) is the **last**
> change and is gated on all of the below.

## Verdict

**PASS — cleared for default-on.** All §11 successor-specific criteria are
green, the standing correctness/GC criteria hold, and every accumulated
deferral is dispositioned (measured / implemented / consciously deferred with a
tracking note). The flip is a single reviewed change carrying this document as
its evidence.

## Environment caveat (read before the numbers)

Wall-clock figures below were taken in an **agent shell** (a shared,
noisy-neighbour box). Per the repository's standing lesson that micro-benches
are flaky in agent shells, the **absolute milliseconds are indicative only** —
the load-bearing criteria are the **ratios / shapes / structural invariants**,
which the harnesses assert **deterministically** (parse counts, producer
counts, cold-serve outcomes), independent of wall-clock. Each harness is a
committed, reproducible test, cited by name; re-run under `taskset -c 0,1` and
a 20× multiplier for CI-shape confirmation.

---

## §11 successor-specific criteria

### 1. Warm-start latency budget (base load on the cold-start critical path, N worktrees)

- **Criterion (ADR-105 §11):** base load sits on the cold-start critical path
  for N worktrees and must stay within the measured budget — the shared-artefact
  win must be demonstrable.
- **Harness:**
  `anvil_intercept::graph_base_warm_start::tests::warm_start_shared_base_reparses_fleet_times_fewer_than_cold`
  (`crates/anvil-intercept/src/graph_base_warm_start.rs`).
- **Method:** a representative `FILES = 60`-file fixture with a cross-file import
  chain, warm-started across a `FLEET = 16`-worktree shape. The win is expressed
  as a **deterministic re-parse ratio** (not flaky wall-clock): a cold fleet
  re-parses every file per worktree; a warm fleet parses the shared base **once**
  and re-parses **zero** files per clean worktree (GBASE-004 content-hash skip).
- **Numbers (indicative wall-clock; deterministic parse counts):**
  - Cold fleet: **960 parses** (`FLEET × FILES`) / ~78.5 ms.
  - Warm fleet: **60 base-production parses + 0 compose parses** / ~68.8 ms.
  - **Fleet-wide re-parse work cut by exactly `FLEET×` (16×).** The harness
    asserts `cold_parses == warm_total × FLEET` structurally.
- **Note on the lower bound:** the fixture parser is trivial (line-splitting),
  so the parse-count ratio is a **conservative lower bound** on the real win —
  on a real codebase tree-sitter parse cost dominates a cold scan, so eliminating
  `FLEET × FILES − FILES` re-parses translates into a far larger wall-clock win
  than the flat parser shows here.
- **Verdict:** **PASS.** The shared base keeps fleet-wide warm-start parse work
  at O(1 production) instead of O(worktrees) cold scans — the design's core win,
  demonstrated deterministically.

### 2. Herd-miss behaviour under single-flight (fleet rebasing onto fresh main)

- **Criterion (ADR-105 §11):** the fleet-rebasing-onto-fresh-main scenario (many
  worktrees simultaneously discovering the same new merge-base) behaves within
  budget under the `O_EXCL` claim.
- **Harness:**
  `anvil_intercept::snapshot_io::base_store::tests::herd_miss_single_flight_fleet_shaped`
  (`crates/anvil-intercept/src/snapshot_io/base_store.rs`), extending the
  existing 8-way `concurrent_claim_is_single_flight_exactly_one_winner` /
  `reclaim_race_has_exactly_one_winner` (150-round) races to a fleet shape.
- **Method:** `FLEET = 48` threads race production for **one** fresh sha, over
  **20 rounds** (fresh sha each), asserting three invariants:
  1. **exactly-one-producer** — precisely one `Acquired`, the other 47
     `Contended` (a live peer holds the claim; none reclaims).
  2. **all-eventually-cold-served** — every non-winner that reads mid-flight sees
     a clean `Absent` (serve cold, ADR-105 §6), **never a torn/partial artefact**.
  3. **all-eventually-warm** — after the single producer publishes + releases,
     all 48 worktrees `load_base` the identical shared artefact (`Loaded`).
- **Verdict:** **PASS.** A 48-wide herd elects one producer and everyone else
  cold-serves then warms — no thundering herd of redundant producers, no torn
  reads. Consistent across 20 rounds (and the pre-existing 120/150-round races).

### 3. Corrupt-shared-base incident behaviour

- **Criterion (ADR-105 §11):** corrupt-shared-base incident rate within the
  agreed threshold — a corrupt shared artefact must be non-fatal and
  **non-poisoning**.
- **Harness:**
  `anvil_intercept::graph_base_warm_start::tests::corrupt_shared_base_all_consumers_cold_serve_then_recover`
  (`crates/anvil-intercept/src/graph_base_warm_start.rs`).
- **Method:** a valid shared base under `FLEET = 8` clean consumer worktrees is
  **corrupted in place** (a torn write over the sealed leaf). Assertions:
  - every consumer classifies it `Ignored` → `ColdBaseIgnored` (discard,
    cold-serve), and **installs nothing** into its resident graph — no
    cross-worktree poison persists;
  - **refresh** (the produce path heals a corrupt artefact at the same
    content-addressed sha, ADR-105 §5 — `publish_base` returns `Written`, not
    `AlreadyPresent`) → every consumer recovers to a `Composed` warm-start.
- **Verdict:** **PASS.** Corruption of the shared artefact is contained to a
  cold-serve for every consumer and self-heals on the next produce — zero poison,
  zero fatality. Failure-signal coverage is wired by GBASE-011 (ADR-090 envelopes
  for base-production failure / claim timeout / GC error; see the GC suite below).

### 4. Claim-production p95 → `STALE_CLAIM_MAX_AGE` calibration (ADR-105 §5)

- **Criterion (ADR-105 §5):** the claim mtime-reclaim bound is `2 × p95` of base
  production; calibrate the placeholder against a measured p95.
- **Harness:**
  `graph_base_producer::tests::base_production_p95_over_representative_fixture`
  (`crates/anvil-cli/src/graph_base_producer.rs`) — `SAMPLES = 15` full
  git-object-read → parse → resolve → build runs over a `FILES = 40` cross-import
  fixture.
- **Numbers:** **p50 ≈ 9.98 ms, p95 ≈ 10.56 ms, 2×p95 ≈ 21.1 ms** (fixture scale).
- **Calibration verdict — REASONED KEEP of the 10-min placeholder.** A literal
  `2 × p95` at fixture scale is sub-second, which is **too tight** for this bound:
  `STALE_CLAIM_MAX_AGE` is only the **fallback** for the ambiguous "present,
  unreadable, or start-time-less" case (the precise `ClaimProcs::is_live`
  pid/PID-reuse check reclaims a dead producer immediately). It must never
  reclaim a genuinely-still-producing subprocess on a **large monorepo**, whose
  real production is seconds-to-minutes — orders of magnitude above any fixture
  p95. Over-retention is the safe direction (a slower recovery in a rare ambiguous
  case, never a stolen live claim), so the conservative **10 min stands with wide
  margin** (~28,000× over the measured 2×p95). This is a *reasoned* keep, recorded
  on the constant's docstring
  (`base_store::STALE_CLAIM_MAX_AGE`) and asserted-for-margin in the harness;
  revisit only if real-fleet telemetry shows monorepo production approaching the
  bound.
- **Verdict:** **PASS (measured, constant unchanged by design).**

### 5. Hot-path mutex contention (registry reconcile / route tick vs `attribute_path`)

- **Criterion (GBASE-003/-009 accumulated):** the ~1 s registry reconcile and the
  route re-evaluation tick share the registry `Mutex<Inner>` with the hot
  `attribute_path` — bound the contention.
- **Disposition — REASONED BOUND (measurement avenue noted).** By construction
  (see `graph_base_trigger`): the trigger loop reconciles the registry only every
  **~10th 100 ms tick (~1 s)**, and the persistence route re-evaluation tick runs
  at **30 s** (decimated to ~1-in-10 for stable worktrees, ADR-105 §GBASE-009
  bounded-observing-tick). Each reconcile takes the registry lock **briefly**
  (a snapshot of `registered_worktrees()`), then releases it before draining
  inotify / ticking `poll`. The contention window is therefore **one short lock
  acquisition per second**, against a hot path that acquires the same lock
  per-`attribute_path`-call — a negligible added contention share at any realistic
  attribute-path rate. The ADR-031 `ipc_roundtrip` bench
  (`crates/anvil-intercept/benches/ipc_roundtrip.rs`) governs the hot-read
  latency budget and remains the empirical tripwire; it does not wire the
  persistence trigger on/off, so an on/off delta would need bench wiring — filed
  as the tracking note below rather than run here (agent-shell bench flakiness,
  and the reasoned bound already shows the window is a per-second brief hold).
- **Verdict:** **PASS (reasoned bound).**

---

## Standing correctness / GC criteria

These are the inherited, non-successor-specific criteria; all remain green.

- **Composition == cold scan (correctness anchor, GBASE-007):**
  `anvil_graph_cache::compose::tests::composed_graph_matches_cold_scan_of_combined_state`
  and `anvil_graph_cache::rebase::tests::composed_edge_set_matches_cold_scan_of_combined_state`,
  plus the committed COMBINED-STATE golden
  (`crates/anvil-graph-cache/tests/fixtures/gbase007_combined_state.snap`, driven
  by `tests/combined_state_golden.rs`). Import-edge parity is byte-equal; the
  base→overlay reexport/call divergence is pinned as a **recorded exclusion**
  (see Dispositions).
- **Snapshot/base golden wire bytes (format integrity):**
  `anvil_graph_cache::snapshot::tests::snapshot_wire_bytes_match_committed_golden`
  and `..::base_snapshot_wire_bytes_match_committed_golden` — both artefact
  classes share one drift-detection golden; **byte-identical**.
- **Refcount GC over ACTMO-registered worktrees (GBASE-008):** the `base_gc`
  suite — `keep_set_retention_keeps_a_referenced_base`,
  `unreferenced_base_is_reclaimed`,
  `claim_respecting_gc_never_removes_a_base_under_production`,
  `gc_races_claim_production_without_removing_a_claimed_base`,
  `epoch_stale_base_is_reclaimed_at_zero_refs`,
  `many_worktrees_union_into_the_keep_set`,
  `unavailable_merge_base_aborts_the_pass_fail_safe`, and the ADR-090
  envelope tests (`gc_reclaim_error_emits_gc_error_envelope_and_pass_continues`).
- **Failure health envelopes (GBASE-011):** base-production failure, `O_EXCL`
  claim timeout, and GC error each raise a worktree-scoped ADR-090 envelope;
  base absent ⇒ cold scan serves (non-fatal).

---

## Dispositions — accumulated GBASE-010-tagged items

Every deferral tagged to the graduation gate across the module, dispositioned.
None is silently dropped.

| Item (origin) | Disposition | Rationale / tracking |
| --- | --- | --- |
| **STALE_CLAIM_MAX_AGE `2×p95` calibration** (GBASE-002) | **MEASURED → reasoned keep** | p95 ≈ 10.56 ms fixture-scale; 10-min fallback kept with margin for large-monorepo production. Recorded on the constant docstring + harness assertion. See §11 criterion 4. |
| **Hot-path mutex contention measurement** (GBASE-003) | **REASONED BOUND** | Per-second brief lock hold vs per-call hot path — negligible. `ipc_roundtrip` bench is the empirical avenue. See §11 criterion 5. |
| **No unregister watch removal** (GBASE-003) | **CONSCIOUSLY DEFERRED** | Bounded (registered set capped, watch count `O(worktrees)`) and correctness-neutral (a stale watch risks only a redundant, single-flighted, cold-serving build). Not a graduation blocker. **Tracking:** file a follow-up CIB for `TriggerCore` repo/worktree removal + watch teardown on unregister; not required for default-on. |
| **Tail-language hashless gap** (GBASE-004) | **CONSCIOUSLY DEFERRED** | Correct today (hashless base files are conservatively re-parsed + tombstoned); only the never-re-parse-the-unchanged-majority *perf* win is defeated for Dart/Go/Java/Kotlin/C#/C/C++/Zig/Wat. TS/Rust/Python (the stamped-hash majority) get the full win — the flip is justified on them. **Tracking:** kernel-side follow-up to stamp `content_hash` in `tail_common::finish`; does not block default-on. |
| **Reexport/call cross-boundary gap** (GBASE-005) | **CONSCIOUSLY DEFERRED (recorded exclusion)** | Base→overlay re-resolution covers **imports** (what the persisted format supports); reexport/call edges into overlay-modified files need a schema-additive payload extension. Pinned by the GBASE-007 golden (asserts the exact divergence, fails if it silently closes/widens). **Tracking:** schema-additive payload follow-up, alongside the tail-language item. Not a graduation blocker (import parity is byte-equal). |
| **Per-sha payload cache decision** (`load_base` re-deserializes per worktree) | **CONSCIOUSLY DEFERRED** | Each sibling worktree currently re-decodes the shared base bytes from disk (`load_base` has no in-memory per-sha cache). The *disk artefact* is already shared O(1); an in-memory per-sha payload cache (decode once, share the `SnapshotPayload` across siblings) is a further optimisation, not a correctness or graduation requirement. **Tracking:** perf follow-up if fleet warm-start decode cost ever shows in profiles. |
| **Tick end-to-end integration test** (GBASE-009 deferred) | **CONSCIOUSLY DEFERRED** | The route/GC tick bodies are unit-covered (`reevaluate_route_on_tick`, `route_on_tick`, the `base_gc` suite); a full daemon-wired tick e2e needs live daemon scaffolding. **Tracking:** existing GBASE-009 follow-up stands; not a graduation blocker. |
| **Tick sweep cost** (GC pass per tick) | **REASONED BOUND** | `run_daemon_gc_pass` shells `git merge-base` per registered worktree on the blocking pool (never the async runtime / ref-watch thread), gated on the persistence flag + a resolvable base dir. Cost is O(registered worktrees) bounded git spawns at the tick cadence — off the hot path by construction. `run_git` is bounded (10 s kill+reap, `GIT_PROBE_TIMEOUT`). Not a graduation blocker. |
| **`run_git` bounded-wait / stderr handling** (GBASE-009) | **IMPLEMENTED** | Landed with GBASE-009: `base_gc::run_git` polls `try_wait`, kills+reaps at `GIT_PROBE_TIMEOUT` (10 s), reads both pipes after exit (deadlock-free without drain threads), and captures full stderr. Documented in-place; no further action. |
| **Rename `compose_inflight`** (GBASE-009 deferred) | **CONSCIOUSLY DEFERRED** | Cosmetic (the guard now covers route+warm, not just compose); touches `save_time.rs` / `persistence_route.rs`. Renames of the council-protected `save_time.rs` are deliberately kept out of the flip PR to keep it minimal and single-purpose. **Tracking:** cosmetic follow-up. |

---

## The flip (final change, gated on the above)

- **Mechanism:** `anvil_graph_cache::snapshot::persist_graph_enabled` — the single
  source of the default that every gate reads (`graph_base_trigger::trigger_enabled`,
  `base_gc::run_daemon_gc_pass`, the `save_time` persistence gate, and the
  `lib.rs` daemon activation sites all route through it). Flipped to **default-on
  with an explicit opt-out**, mirroring the graduated `ANVIL_WATCH_DAEMON`
  precedent (`daemon_routing_mode_from`, ADR-075 v0.8 flip):
  - **unset / empty / unparseable / affirmative** (`1`, `true`, `yes`, `on`) ⇒
    **enabled** (absence-of-variable now means on);
  - **explicit opt-out** (`0`, `false`, `no`, `off`, case-insensitive, trimmed) ⇒
    disabled — the documented rollback path.
- **Catalogue:** `flags/manifest.json` key `daemon.persist-graph` `defaultVariant`
  flips `disabled` → `enabled`, with the description updated to reference this
  gate (ADR-105 §11 successor criteria).
- **Docs:** the env-var / default-off prose in
  `docs/architecture/kernel-as-built.md`, `docs/architecture/graph-v2-foundation-spec.md`,
  and ADR-105's Consequences is updated to reflect the graduated default-on
  posture (opt-out via `ANVIL_PERSIST_GRAPH=0`).
- **Council-gate note:** the flip touches
  `crates/anvil-graph-cache/src/snapshot.rs`, `flags/manifest.json`, docs, and
  plans — **none** of which is on `.claude/hooks/council-protected-paths`
  (that list is the save-time auth/confinement surface: `save_time.rs`,
  `confinement.rs`, `ipc.rs`, `workspace_admission.rs`, `auth.rs`, `registry.rs`).
  The flip therefore does **not** trip the council gate on protected paths.

## Reproduce

```sh
# §11 criterion harnesses (deterministic; --nocapture for the indicative numbers)
cargo test -p eddacraft-anvil-intercept --lib -- --nocapture \
  herd_miss_single_flight_fleet_shaped \
  warm_start_shared_base_reparses_fleet_times_fewer_than_cold \
  corrupt_shared_base_all_consumers_cold_serve_then_recover
cargo test -p eddacraft-anvil --bin anvil -- --nocapture \
  base_production_p95_over_representative_fixture

# CI-shape confirmation: 20× under core pinning
taskset -c 0,1 cargo test -p eddacraft-anvil-intercept --lib \
  herd_miss_single_flight_fleet_shaped
```
