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
- **Scope of the `FLEET×` claim (honest bounds).** The `FLEET×` figure is the
  **clean-worktree best case** — every worktree's overlay is empty, so warm
  re-parses **zero** files. This is the upper bound of the win, and (because the
  fixture parser is trivial line-splitting) the parse-count ratio is itself a
  **conservative lower bound on the real wall-clock win**: on a real codebase
  tree-sitter parse dominates a cold scan, so eliminating `FLEET × FILES − FILES`
  re-parses is worth far more than the flat parser shows.
- **Companion case — the win scales with the unchanged majority (not just the
  clean case).** `warm_start_dirty_worktree_reparses_only_the_changed_minority`
  dirties `DIRTY = 3` of `FILES = 40` files per worktree and asserts the overlay
  re-parses **exactly 3** (the changed minority), not 40 — a **13×** per-worktree
  reduction. So the win degrades gracefully from `FILES / 0` (clean) toward a cold
  scan only as a worktree approaches fully-dirty; a realistic few-dirty-files
  worktree keeps most of it.
- **Churn-economics caveat.** The `FLEET×` win applies to a **stable-base
  window**. If merge-base churn outruns production (the fleet rebases faster than
  a base can be built), routing degrades to **cold-serve** for the not-yet-produced
  sha — safe and non-fatal (ADR-105 §6), but the shared win is not realised until
  the base settles. A churn-rate harness (production time vs merge-base movement
  rate) is dispositioned as a follow-up below.
- **Verdict:** **PASS.** The shared base keeps fleet-wide warm-start parse work
  at O(1 production) instead of O(worktrees) cold scans — the design's core win,
  demonstrated deterministically, with the win scaling with the unchanged majority.

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
- **Evidence scope (what is and is NOT proven).** Single-flight is proven **per
  repo, per sha**: N racers for one sha in one store elect one producer. It does
  **not** cover **cross-repo / cross-daemon** upgrade-day concurrency (an entire
  fleet of *different* repos all producing at once on the same machine). That is
  **dispositioned as accepted risk** with rationale: each repo's production is a
  single `O_EXCL`-claimed subprocess, so N distinct repos ⇒ at most N concurrent
  producer subprocesses (one per repo), OS-scheduled and self-limiting — there is
  no shared global lock to contend, and each subprocess is bounded (`run_git` 10 s
  probes, restart cap N=3 per sha-lineage). A cross-daemon spawn throttle (a global
  concurrency cap across repos) is a follow-up (dispositioned below), not a
  graduation blocker.
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
- **Authenticity vs corruption (scope of the guarantee).** The CRC-32 in the
  sealed envelope is a **corruption detector**, not an authenticity/tamper
  guarantee (an attacker who can write the file can recompute the CRC). Tamper
  resistance for the shared base rests on the **same-uid, owner-only boundary**:
  the store is `0600` under a `0700` state dir (ADR-069 §8), the accepted residual
  risk of the machine-local persistence model. The corrupt-base criterion proves
  the **integrity/availability** posture (a garbled artefact never poisons and
  self-heals); it does not — and is not claimed to — defend against a same-uid
  adversary, which is out of the ADR-069 threat model.
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
- **Numbers (recalibrated):** **p50 ≈ 9.94 ms, p95 ≈ 10.20 ms, 2×p95 ≈ 20.40 ms**
  (fixture scale). The percentile is now a correct **nearest-rank** index
  (`((N-1) × p) / 100` = index 13 of 15 for p95); the earlier draft's
  `ceil((N×95)/100) − 1` returned the **max** (index 14) at N=15, overstating p95
  — fixed in the harness and restated here.
- **Calibration verdict — REASONED KEEP of the 10-min placeholder.** A literal
  `2 × p95` at fixture scale is sub-second, which is **too tight** for this bound:
  `STALE_CLAIM_MAX_AGE` is only the **fallback** for the ambiguous "present,
  unreadable, or start-time-less" case (the precise `ClaimProcs::is_live`
  pid/PID-reuse check reclaims a dead producer immediately). It must never
  reclaim a genuinely-still-producing subprocess on a **large monorepo**, whose
  real production is seconds-to-minutes — orders of magnitude above any fixture
  p95. Over-retention is the safe direction (a slower recovery in a rare ambiguous
  case, never a stolen live claim), so the conservative **10 min stands with wide
  margin** (~29,000× over the measured 2×p95 ≈ 20.4 ms). This is a *reasoned* keep,
  recorded on the constant's docstring
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

## Evidence scope and caveats (read honestly)

What the harnesses **do** and **do not** establish, consolidated so the verdict
is not over-read:

- **Wall-clock is indicative, ratios are load-bearing.** All ms figures are
  agent-shell numbers; the asserted criteria are deterministic (parse counts,
  producer counts, cold-serve outcomes, percentile ranks), independent of the
  clock.
- **Warm-start `FLEET×` is the clean-worktree best case**, and a lower bound in
  wall-clock terms (trivial fixture parser). The dirty-worktree companion shows
  the win scales with the unchanged majority (`FILES / DIRTY`, 13× at 3/40).
- **Herd single-flight is per repo, per sha.** Cross-repo / cross-daemon
  upgrade-day concurrency is **not** covered — accepted risk (one bounded
  subprocess per repo, self-limiting), throttle dispositioned as a follow-up.
- **p95 is fixture-scale**, recalibrated to a correct nearest-rank index; the
  `STALE_CLAIM_MAX_AGE` keep is a *reasoned* over-retention for large monorepos,
  not a fit to the fixture number.
- **Corrupt-base proves integrity/availability, not authenticity** — tamper
  resistance is the same-uid `0600`/`0700` boundary (ADR-069 accepted residual).
- **Churn economics:** the win is a stable-base-window property; churn outrunning
  production degrades to safe cold-serve (churn-rate harness deferred).
- **Mutex contention is a reasoned bound**, not a measured on/off delta (the
  `ipc_roundtrip` bench does not wire persistence on/off).

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
| **STALE_CLAIM_MAX_AGE `2×p95` calibration** (GBASE-002) | **MEASURED → reasoned keep** | p95 ≈ 10.20 ms fixture-scale (nearest-rank); 10-min fallback kept with margin for large-monorepo production. Recorded on the constant docstring + harness assertion. See §11 criterion 4. |
| **Hot-path mutex contention measurement** (GBASE-003) | **REASONED BOUND** | Per-second brief lock hold vs per-call hot path — negligible. `ipc_roundtrip` bench is the empirical avenue. See §11 criterion 5. |
| **No unregister watch removal** (GBASE-003) | **CONSCIOUSLY DEFERRED** | Bounded (registered set capped, watch count `O(worktrees)`) and correctness-neutral (a stale watch risks only a redundant, single-flighted, cold-serving build). Not a graduation blocker. **Tracking:** file a follow-up CIB for `TriggerCore` repo/worktree removal + watch teardown on unregister; not required for default-on. |
| **Tail-language hashless gap** (GBASE-004) | **CONSCIOUSLY DEFERRED** | Correct today (hashless base files are conservatively re-parsed + tombstoned); only the never-re-parse-the-unchanged-majority *perf* win is defeated for Dart/Go/Java/Kotlin/C#/C/C++/Zig/Wat. TS/Rust/Python (the stamped-hash majority) get the full win — the flip is justified on them. **Tracking:** kernel-side follow-up to stamp `content_hash` in `tail_common::finish`; does not block default-on. |
| **Reexport/call cross-boundary gap** (GBASE-005) | **CONSCIOUSLY DEFERRED (recorded exclusion)** | Base→overlay re-resolution covers **imports** (what the persisted format supports); reexport/call edges into overlay-modified files need a schema-additive payload extension. Pinned by the GBASE-007 golden (asserts the exact divergence, fails if it silently closes/widens). **Tracking:** schema-additive payload follow-up, alongside the tail-language item. Not a graduation blocker (import parity is byte-equal). |
| **Per-sha payload cache decision** (`load_base` re-deserializes per worktree) | **CONSCIOUSLY DEFERRED** | Each sibling worktree currently re-decodes the shared base bytes from disk (`load_base` has no in-memory per-sha cache). The *disk artefact* is already shared O(1); an in-memory per-sha payload cache (decode once, share the `SnapshotPayload` across siblings) is a further optimisation, not a correctness or graduation requirement. **Tracking:** perf follow-up if fleet warm-start decode cost ever shows in profiles. |
| **Tick end-to-end integration test** (GBASE-009 deferred) | **CONSCIOUSLY DEFERRED** | The route/GC tick bodies are unit-covered (`reevaluate_route_on_tick`, `route_on_tick`, the `base_gc` suite); a full daemon-wired tick e2e needs live daemon scaffolding. **Tracking:** existing GBASE-009 follow-up stands; not a graduation blocker. |
| **Tick sweep cost** (GC pass per tick) | **REASONED BOUND** | `run_daemon_gc_pass` shells `git merge-base` per registered worktree on the blocking pool (never the async runtime / ref-watch thread), gated on the persistence flag + a resolvable base dir. Cost is O(registered worktrees) bounded git spawns at the tick cadence — off the hot path by construction. `run_git` is bounded (10 s kill+reap, `GIT_PROBE_TIMEOUT`). Not a graduation blocker. |
| **`run_git` bounded-wait / stderr handling** (GBASE-009) | **IMPLEMENTED** | Landed with GBASE-009: `base_gc::run_git` polls `try_wait`, kills+reaps at `GIT_PROBE_TIMEOUT` (10 s), reads both pipes after exit (deadlock-free without drain threads), and captures full stderr. Documented in-place; no further action. |
| **Rename `compose_inflight`** (GBASE-009 deferred) | **CONSCIOUSLY DEFERRED** | Cosmetic (the guard now covers route+warm, not just compose); touches `save_time.rs` / `persistence_route.rs`. Renames of the council-protected `save_time.rs` are deliberately kept out of the flip PR to keep it minimal and single-purpose. **Tracking:** cosmetic follow-up. |
| **Manual purge / on-demand GC escape hatch** (GBASE-010 council) | **IMPLEMENTED** | Hidden `anvil graph-base gc` runs one keep-set GC pass (durably-registered worktrees = keep-set, reusing `run_daemon_gc_pass` + the production resolver); `--purge-all` empties the store via `base_gc::purge_all_bases`. Safe semantic: an actively-claimed sha is **skipped and reported** (non-blocking, never yanked from a live producer), the rest is emptied, re-run after the claim settles. Tests: `purge_all_empties_the_store_on_demand`, `purge_all_empties_the_store_safely_while_a_claim_is_active`. Documented in the upgrade notes as the disk-pressure remediation. |
| **Manifest/code default parity** (GBASE-010 council) | **IMPLEMENTED** | `manifest_default_agrees_with_code_default` asserts the generated `flags/manifest.json` `daemon.persist-graph` default variant's boolean equals `persist_graph_enabled(None)`, so a manifest/code drift fails loudly. |
| **Keep-set growth bound** (GBASE-010 council 4f) | **DOCUMENTED** | Worst-case resident bases ≤ **(registered worktrees) × 2 keys per repo** — the `default-branch` merge-base and its `@{upstream}`-refined key (`base_gc`'s conservative superset), unioned per repo — **plus** any leftover the manual gap allows until GC/`--purge-all` runs. So an operator can bound disk: distinct merge-bases (typically 1 per repo) × snapshot size, reclaimed to the keep-set by refcount GC. |
| **GC whole-pass abort on one Unavailable worktree** (GBASE-010 council 4h) | **CONSCIOUSLY DEFERRED (tracked)** | `sweep_unreferenced_bases` reclaims **nothing** when any worktree's merge-base is `Unavailable` (fail-safe keep, `aborted_uncertain`), so one persistently-unavailable worktree can stall reclaim fleet-wide. This is **observable** meanwhile via the GBASE-011 GC-deferral ADR-090 envelope, and `--purge-all` is the operator override. A per-worktree Unavailable-exclusion policy (exclude the bad worktree, GC the rest) is a follow-up — **not** redesigned in this PR (a wrong exclusion could reclaim a referenced base). **Tracking:** GC Unavailable-exclusion policy follow-up. |
| **Cross-daemon / cross-repo spawn throttle** (GBASE-010 council 4e) | **CONSCIOUSLY DEFERRED (accepted risk)** | Single-flight is per-repo/per-sha; upgrade-day concurrency across N distinct repos yields ≤ N producer subprocesses (one per repo), OS-scheduled and self-limiting, each bounded (`run_git` 10 s, restart cap N=3). A global cross-repo concurrency cap is a follow-up. **Tracking:** cross-daemon spawn-throttle follow-up. See §11 criterion 2. |
| **Churn-rate harness** (GBASE-010 council 4d) | **CONSCIOUSLY DEFERRED (tracked)** | The `FLEET×` win holds in stable-base windows; a harness measuring production time vs merge-base churn rate (when churn outruns production, routing degrades to safe cold-serve) is a follow-up. **Tracking:** churn-rate harness. See §11 criterion 1's churn-economics caveat. |
| **Interactive "graph persistence: enabled" status line** (GBASE-010 council) | **CONSCIOUSLY DEFERRED (JOURNEY-adjacent)** | Surfacing the now-default-on persistence state on the `start`/`watch` surface collides with the in-flight JOURNEY first-run work (the start/welcome surface is being reworked). **Tracking:** record as a JOURNEY-adjacent follow-up to avoid churn on a moving surface. |

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
- **Rollback (spawn-environment scope).** `ANVIL_PERSIST_GRAPH=0` is the opt-out,
  but it must be set in the **daemon's spawn environment** — a login-shell rc does
  **not** reach a systemd-user-launched or IDE-launched daemon. This is stated in
  the changelog, upgrade notes, and feature-flag inventory so operators do not set
  it in `~/.bashrc` and wrongly believe persistence is off.
- **Disk-pressure remediation:** the hidden `anvil graph-base gc` /
  `anvil graph-base gc --purge-all` verb (documented in the upgrade notes) empties
  or GCs the base store on demand — the operator escape hatch for
  `<graph-cache>/base` growth.
- **Docs:** the env-var / default-off prose in
  `docs/architecture/kernel-as-built.md`, `docs/architecture/graph-v2-foundation-spec.md`,
  ADR-105's Consequences, the changelog, upgrade notes, and feature-flag inventory
  is updated to reflect the graduated default-on posture (opt-out via
  `ANVIL_PERSIST_GRAPH=0` in the daemon spawn env).
- **Council-gate note (corrected).** The PR touches
  `crates/anvil-intercept/src/save_time.rs` — **docstring-only** (the persistence
  docstrings now say "default-on since the GBASE-010 graduation; explicit opt-out
  honoured"), no behaviour change — and `save_time.rs` **is** on
  `.claude/hooks/council-protected-paths` (the save-time auth/confinement surface:
  `save_time.rs`, `confinement.rs`, `ipc.rs`, `workspace_admission.rs`, `auth.rs`,
  `registry.rs`). **The PR therefore DOES trip the council-protected-paths gate**
  and requires a converged Council review — which is exactly what this final
  Council pass provides. (An earlier draft of this doc asserted no protected path
  was touched; that self-attestation was factually wrong and is corrected here.)
  The functional flip itself lives in the unprotected
  `crates/anvil-graph-cache/src/snapshot.rs` (`persist_graph_enabled`), with the
  gates in `graph_base_trigger.rs` / `base_gc.rs` / `lib.rs` routing through it.

## Reproduce

```sh
# §11 criterion harnesses (deterministic; --nocapture for the indicative numbers)
cargo test -p eddacraft-anvil-intercept --lib -- --nocapture \
  herd_miss_single_flight_fleet_shaped \
  warm_start_shared_base_reparses_fleet_times_fewer_than_cold \
  warm_start_dirty_worktree_reparses_only_the_changed_minority \
  corrupt_shared_base_all_consumers_cold_serve_then_recover \
  purge_all_empties_the_store_on_demand \
  purge_all_empties_the_store_safely_while_a_claim_is_active
cargo test -p eddacraft-anvil --bin anvil -- --nocapture \
  base_production_p95_over_representative_fixture
cargo test -p eddacraft-anvil-graph-cache --lib \
  manifest_default_agrees_with_code_default

# CI-shape confirmation: 20× under core pinning
taskset -c 0,1 cargo test -p eddacraft-anvil-intercept --lib \
  herd_miss_single_flight_fleet_shaped
```
