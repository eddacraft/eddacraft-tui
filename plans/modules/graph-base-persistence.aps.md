# Graph Base Persistence

| ID    | Owner | Status      | Progress |
| ----- | ----- | ----------- | -------- |
| GBASE | —     | In Progress | 0/11     |

**Last reviewed:** 2026-07-11 (created from planning council `plan-89a47ac7`,
synthesised as [ADR-105](../decisions/105-shared-base-graph-persistence.md) —
the [ADR-069](../decisions/069-graph-v2-persistence.md) storage-layout
successor). Replaces ADR-069's per-`WorktreeKey` snapshot (O(worktrees) on disk
and scan even with a correct sweep — the per-worktree orphan race is already
closed by CIB-096; layout blocking `ANVIL_PERSIST_GRAPH` default-on) with **one
write-once, content-addressed base per repo per merge-base commit + live
per-worktree overlays**. ADR-105 inherits ADR-069's format machinery, trust
line, and privacy line unchanged, and amends only ADR-069 §5 (single-owner) and
§10 (orphan sweep). All work items are Proposed and reference ADR-105.

## Purpose

Give the save-time daemon a **shared, dependency-honest warm-start store** so a
restarted or newly-registered worktree re-warms from a base snapshot of its
merge-base commit's committed tree plus a cheap live overlay, instead of a full
cold rebuild or a private per-worktree snapshot. This shrinks persistence from
O(worktrees) to O(distinct merge-bases), lets sibling worktrees reuse
parse/resolve work, reclaims the new shared-base orphan class it introduces (via
merge-base refcounting; the per-worktree orphan race was already closed by
CIB-096), and clears the layout blocker to flipping `ANVIL_PERSIST_GRAPH`
default-on.

## Boundaries

**In scope:**

- Reading the merge-base commit's committed tree from git objects (never a
  working tree) and producing its full graph in a CLI subprocess.
- A content-addressed, write-once base store keyed by merge-base sha, with
  `O_EXCL` single-flight production/reclaim and the schema-epoch-vs-base rule.
- Proactive pre-production triggered by directory-level ref watches, with
  debounce, restart cap, and ENOSPC degrade.
- Live overlay computation (ADR-085 executor scoped to changed-vs-base files),
  disjoint id allocation + compose-time cross-boundary re-resolution, and
  per-worktree materialisation/composition.
- A COMBINED-STATE golden parity fixture (base + scripted overlay == cold scan).
- Refcount GC over ACTMO-registered worktrees' merge-bases.
- Re-entrant `persistence_route` topology/staleness routing with structured
  fallback events, ADR-090 health-envelope wiring, and the successor-specific
  graduation gate that flips the default-on flag.

**Out of scope:**

- Shared-RAM base + overlay-query tiering / COW base (deferred; ADR-031 latency
  risk).
- Nearest-ancestor base fallback (deferred; cutoff complexity).
- Cross-machine / distributed base distribution.
- CI-produced or server-produced bases (off-trunk misses).
- Telemetry polish beyond the ADR-090 failure envelopes.
- Migration code (bases are discard-and-rebuild, never migrated).

## Dependencies

- [ADR-105](../decisions/105-shared-base-graph-persistence.md) (this module's
  binding decision) and [ADR-069](../decisions/069-graph-v2-persistence.md)
  (inherited format, trust line, privacy line).
- [ADR-085](../decisions/085-daemon-full-scan-executor.md) (the executor that
  computes the overlay scoped to changed-vs-base files).
- [ADR-090](../decisions/090-daemon-worktree-scoped-health-envelopes.md)
  (worktree-scoped failure signalling).
- [ADR-094](../decisions/094-worktree-registration-ux.md) (ACTMO durable
  worktree registration = the GC keep-set).
- ADR-061/063/064/067 (lean resident daemon; parser injected via the CLI, never
  resident — honoured by the CLI-subprocess producer).
- `anvil-graph-cache` (`SnapshotPayload` DTO, replay load), `anvil-intercept`
  (`watcher.rs`, `snapshot_io`), `anvil-cli` (`graph_base_producer`, the
  `l4_engine.rs` git-object read pattern).

## Notes

- **Entry gate.** The no-behaviour-diff `anvil_intercept::snapshot_io::store`
  extraction (key-agnostic I/O seam) ships as its **own PR first**, gated by
  byte-identical existing golden tests, **before** GBASE-002. It is not a
  separate work item here — it is the acceptance precondition for GBASE-002.
- **Out of this module.** CIB-092d and CIB-092h land **independently** outside
  this module — do **not** create GBASE items for them.
- **GBASE-007 is the top schedule risk** — the COMBINED-STATE parity fixture is
  the correctness anchor for the whole design.
- **Ordering.** GBASE-011 (health-envelope wiring) runs as a parallel track but
  is a **prerequisite of the terminal GBASE-010** — the graduation gate depends
  on its failure-envelope coverage. (Do not renumber.)
- **Checkpoint clause — return to council if:** the module exceeds **13**
  decision-log-granularity items, **or** GBASE-007 requires a **second full
  redesign** (a different scheme, not a bug patch).

## Work Items

#### GBASE-001: Merge-base tree reader

- **Status:** Merged 2026-07-11 via PR #3268
- **Intent:** Produce the full graph of a merge-base commit's committed tree by
  reading git objects, never a working tree.
- **Expected Outcome:** A CLI-subprocess reader walks the merge-base commit's
  tree via git objects (the `l4_engine.rs` batch-read pattern, zero new deps),
  parses it with tree-sitter, and yields a full base graph; the resident daemon
  links no parser. Includes a **warm-start-latency acceptance criterion** —
  base load sits on the cold-start critical path and stays within the measured
  budget.
- **Validation:** Fixture tests build a base from a known commit tree
  deterministically; a latency check asserts base load meets the warm-start
  budget; `daemon_dep_boundary` stays green.
- **Files:** `crates/anvil-cli/`, `crates/anvil-intercept/`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** —

---

#### GBASE-002: Content-addressed write-once base store + single-flight claim

- **Status:** Merged 2026-07-11 via PR #3269
- **Intent:** Persist a base as a content-addressed, write-once artefact keyed by
  merge-base sha, with race-safe single-flight production.
- **Expected Outcome:** A base is written once per merge-base sha (magic
  `ANVILGB1`, `SnapshotPayload` DTO reused verbatim, shared versioning policy);
  production is single-flight via an `O_EXCL` `.producing/<sha>.lock` stamped
  `{pid, start_time}` with a PID-reuse guard; reclaim happens iff the pid is dead
  (or PID-reused) or the lock mtime exceeds a conservative bound (GBASE-010
  calibrates 2× p95). All destruction of a claim record — reclaim and release —
  runs under a per-dir `flock(LOCK_EX)` guard, re-verifying the lock's identity
  through a dirfd-anchored open before the `unlinkat`, so the hot-path `O_EXCL`
  create stays lock-free while classify→destroy and read-nonce→unlink are
  TOCTOU-free. Includes the **schema-epoch-vs-shared-base clause**: an
  epoch-mismatched base is ignored (cold path) and GC-eligible once unreferenced
  at the old epoch — discard-and-rebuild, never migrate, never a mixed-epoch
  composition. §9's "left in place" governs the **read/load path** (a loader
  refuses a mismatched base rather than returning it); a fresh produce may
  overwrite a corrupt/stale-epoch artefact at the same content-addressed sha
  (atomic, fail-closed readers). **Entry gate:** the no-behaviour-diff
  `snapshot_io::store` extraction PR must have landed first.
- **Validation:** Tests cover write-once, concurrent-claim single-flight,
  stale-claim reclaim (dead pid + timeout paths), and epoch-mismatch discard;
  existing golden tests stay byte-identical through the seam extraction.
- **Files:** `crates/anvil-intercept/` (`snapshot_io/store`), `crates/anvil-cli/`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GBASE-001

---

#### GBASE-003: Proactive pre-production trigger

- **Status:** Merged 2026-07-11 via PR #3270
- **Intent:** Pre-produce the base when the repo's merge-base moves, driven by
  ref changes rather than a save.
- **Expected Outcome:** A new `graph_base_trigger` module beside `watcher.rs`
  (the item's named integration point; kept beside it rather than bloating the
  INTD-004 channel receiver) provides directory-level inotify
  (`IN_MOVED_TO | IN_CREATE`) on the refs dir, packed-refs parent, primary HEAD,
  and **each registered worktree's** HEAD dir. Ref changes are debounced
  (~500 ms) and drive production; a newer trigger cancels-and-restarts (SIGTERM
  the child) with a cap of N=3 per sha-lineage (over-cap ⇒ serve cold + log +
  ADR-090 envelope through the real fan-out + re-arm on quiescence). Production
  runs as a **detached `anvil graph-base build --repo <root>` subprocess**: the
  background-pool thread only spawns + records, and a **dedicated reaper
  `std::thread`** owns `child.wait()` — the background-pool thread **never
  blocks**. **Wired live in the daemon:** `run_foreground` calls
  `graph_base_trigger::activate` on Linux, gated on the same condition the
  save-time path uses for persistence (`ANVIL_PERSIST_GRAPH` affirmative **and** a
  resolvable base store dir); it seeds the registered worktree roots and runs a
  dedicated ref-watch `std::thread` that reconciles roots (picking up late
  registrations by polling `SessionRegistry::registered_worktrees`, since the
  single membership hook is already owned by the save-time driver supervisor),
  drains inotify events into the debounce, and ticks `poll`; it is shut down +
  joined on every daemon exit path.
  - **Recorded scope notes (GBASE-003 as landed):**
    - **Multi-worktree model + budget.** State is keyed by a repo's **common
      gitdir**, so every worktree of a repo shares one lineage/debounce/in-flight.
      `reconcile_roots` groups registered roots by common gitdir and reconciles
      each repo against its **full** current worktree set: the shared ref dirs are
      watched **once per repo** and **each worktree's own HEAD dir** is watched
      (so a `git checkout` in a sibling worktree — which rewrites only its own
      HEAD — is caught, not just the first registrant's). The honest per-repo
      descriptor invariant is **`O(1) per registered workspace`**: shared
      (`MAX_SHARED_REF_WATCHES_PER_REPO = 3`, deduping to 2 in the standard layout)
      **plus one per registered worktree** (`ref_watch_budget()`), asserted in
      tests. The daemon owns **no** pre-existing inotify/descriptor budget to
      extend (the recursive `notify` watcher lives in the un-linked `anvil-kernel`
      crate), so this is the single authoritative ref-watch accounting.
    - **Envelope scoping.** The cap-exceeded ADR-090 health envelope is emitted
      **once per currently-registered worktree** of the repo (ADR-090 delivers by
      worktree ownership), scoped from the live worktree set — never pinned to the
      first registrant.
    - **Single-flight = the child's claim (design 4a).** The daemon does **not**
      pre-claim. The subprocess resolves the merge-base sha itself (`graph-base
      build` with `--merge-base` omitted) and runs `base_store::claim()`
      internally, so the daemon stays git-free; a daemon-side claim would be a
      redundant double-claim. The "sha-lineage" restart cap is therefore tracked
      git-free as the chain of ref-change triggers per repo between quiescent
      periods (a conservative proxy for §7's per-sha lineage).
    - **Cancel uses SIGTERM, not `ScanCancel`.** `ScanCancel` cancels an
      in-process background *scan*; a detached *subprocess* is cancelled by
      signalling it (the reaper reaps). The daemon holds no claim to release.
    - **Registry reconcile cadence.** `registered_worktrees()` shares the
      registry's hot `Mutex<Inner>` with `attribute_path`, so the loop reconciles
      the registry only every ~10th 100 ms tick (~1 s late-registration latency —
      fine for a seconds-scale base warm) while draining inotify + ticking `poll`
      every tick. GBASE-010's graduation gate should measure this lock contention.
    - **Known gap — no unregister removal (rides GBASE-010).** `TriggerCore` never
      removes a repo/worktree or its watches, so a worktree that unregisters leaves
      its watches + latched lineage state resident until daemon restart. Bounded
      (registered set capped; watch count `O(worktrees)`) and correctness-neutral
      (a stale watch risks only a redundant, single-flighted, cold-serving build).
      Real removal is deferred to the GBASE-010 graduation gate.
    - **ENOSPC degrade fallback is check-and-request (documented stub).** On
      `ENOSPC` (or any add-watch failure) the trigger degrades that repo to a
      disabled state (logged once, structured) and ignores its ref events; the
      fallback is **CLI-invocation check-and-request** (`anvil start`/`watch`/
      `status` requesting production on demand). Wiring that request path into the
      CLI command surface is the one remaining follow-up outside this item's
      `anvil-intercept` scope and is left as a documented seam.
    - **Platform gap.** Non-Linux `cfg(unix)` (macOS) needs a kqueue/FSEvents ref
      backend — the inherited ADR-105 §8 `cfg(unix)` platform gap; the state
      machine, spawn/reap seam, and envelope path are platform-neutral and the
      Linux inotify backend + `run_foreground` activation ship live.
- **Validation:** Tests cover ref-rename detection, debounce coalescing, the
  restart cap, and the ENOSPC degrade path; the descriptor budget is asserted; a
  test asserts the **ADR-090 envelope is emitted** (not merely logged) when the
  restart cap is exceeded.
- **Files:** `crates/anvil-intercept/` (`watcher.rs`)
- **Confidence:** low
- **Priority:** High
- **Dependencies:** GBASE-002

---

#### GBASE-004: Overlay computation via the ADR-085 executor

- **Status:** Merged 2026-07-11 via PR #3271
- **Intent:** Compute a worktree's live overlay as the diff of its changed files
  versus the base tree.
- **Expected Outcome:** The ADR-085 executor is scoped to the files that differ
  between the worktree and the base tree, producing an overlay graph fragment
  (adds, removes, tombstones) ready to compose onto a loaded base.
- **Validation:** Tests assert the overlay covers exactly the changed-vs-base
  file set for representative dirty-worktree states; deterministic.
- **Files:** `crates/anvil-intercept/`, `crates/anvil-graph-cache/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GBASE-001
- **Note (tail-language hashless gap):** the overlay diff is content-hash exact
  only for base files that carry a hash. Today only TS/Rust/Python stamp one; the
  tail-language extractors (Dart/Go/Java/Kotlin/C#/C/C++/Zig/Wat via
  `tail_common::finish`) stamp `None`, so those base files are reconciled
  **conservatively (always re-parsed + tombstoned)** — correct, but the
  never-re-parse-the-unchanged-majority win is defeated for them. The real fix is
  kernel-side follow-up (stamp `content_hash` in `tail_common::finish` like
  TS/Rust/Python). Flagged for the **GBASE-007** parity fixture (polyglot base +
  overlay must still equal a cold scan) and the **GBASE-010** graduation gate (a
  polyglot repo's warm-start overlay perf depends on closing it).

---

#### GBASE-005: Disjoint id allocation + cross-boundary re-resolution

- **Status:** Merged 2026-07-11 via PR #3273
- **Intent:** Keep base and overlay id spaces disjoint and re-resolve
  cross-boundary imports at compose time.
- **Expected Outcome:** The base owns `[0, base_next_id)`; the overlay allocates
  ids above a reserved watermark; imports that cross the base↔overlay boundary
  are re-resolved from the persisted raw-specifier map at compose time, never
  trusted by stale id.
- **Validation:** Tests cover overlay-references-base, base-references-overlay,
  and watermark disjointness; a cross-edge fixture resolves correctly after
  composition.
- **Files:** `crates/anvil-graph-cache/`, `crates/anvil-intercept/`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GBASE-001, GBASE-004
- **Note (reexport/call cross-boundary gap):** base→overlay re-resolution is
  implemented for **imports** — exactly what ADR-105 §3's mechanism specifies
  and what the persisted format supports (the `DependencyGraph` forward map is
  deliberately Imports-only per GV2-031; the base stores *resolved*
  reexport/call edges, not raw `ReexportEdge`/`CallSite`). A surviving base
  file's reexport or call into an overlay-modified file therefore cannot be
  re-resolved from the persisted format; closing that needs a schema-additive
  payload change (an extension of this design, not a redesign). GBASE-007's
  fixture must scope its cross-edge assertions accordingly and record the gap;
  full reexport/call parity is follow-up work alongside the tail-language
  hashless gap.

---

#### GBASE-006: Per-worktree materialisation / composition

- **Status:** Proposed
- **Intent:** Materialise one resident graph per worktree by loading the shared
  base and applying its overlay.
- **Expected Outcome:** Warm-start loads the base by replay and applies the
  overlay into one materialised petgraph per worktree; the base is shared on disk
  only (each worktree materialises its own resident graph). The composed
  workspace comes up **stale** per the inherited ADR-069 trust line.
- **Validation:** Tests assert a composed worktree comes up stale (never
  `Certified` pre-reconcile) and that two sibling worktrees share the base
  artefact on disk while holding independent resident graphs.
- **Files:** `crates/anvil-intercept/`, `crates/anvil-graph-cache/`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GBASE-002, GBASE-004, GBASE-005

---

#### GBASE-007: Combined-state golden parity fixture

- **Status:** Proposed
- **Intent:** Prove that a composed base+overlay is identical to a cold scan of
  the combined on-disk state. **(Top schedule risk — correctness anchor.)**
- **Expected Outcome:** A golden COMBINED-STATE fixture composes `base(X)` with a
  scripted overlay exercising adds, removes, tombstones, and cross-edges, and
  asserts the result is byte-for-byte equal to a cold scan of the combined state.
  Cross-edge scope: **import** edges (the ADR-105 §3 mechanism GBASE-005
  implements); base→overlay reexport/call edges are not reconstructable from the
  persisted format (see the GBASE-005 note) — the fixture must either exclude
  them from the byte-equality set with an explicit recorded exclusion, or this
  item grows the schema-additive payload extension that closes the gap.
- **Validation:** The COMBINED-STATE golden test passes deterministically and
  fails on any composition divergence.
- **Files:** `crates/anvil-graph-cache/`, `crates/anvil-intercept/`
- **Confidence:** low
- **Priority:** Critical
- **Dependencies:** GBASE-005, GBASE-006

---

#### GBASE-008: Refcount GC over ACTMO-registered worktrees' merge-bases

- **Status:** Merged 2026-07-11 via PR #3272
- **Intent:** Reclaim base artefacts no live worktree references, without racing
  producers.
- **Expected Outcome:** The daemon holds a refcount over the current merge-bases
  of ACTMO durably-registered worktrees (the keep-set); a base is GC-eligible
  only when no live registered worktree references its sha, and GC respects
  active `O_EXCL` claims. This reclaims the **new shared-base orphan class** this
  module introduces — a merge-base-keyed analogue of, not a replacement for, the
  per-worktree `<hash>.root` companion sweep (CIB-096, which already closed the
  historic per-worktree orphan race); the two operate on different orphan
  classes.
- **Validation:** Tests cover keep-set retention, unreferenced-base reclaim,
  and claim-respecting GC (a base under active production is never removed).
- **Files:** `crates/anvil-intercept/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GBASE-002

---

#### GBASE-009: Re-entrant persistence-route topology / staleness routing

- **Status:** Proposed
- **Intent:** Route each worktree to the base path or the per-worktree path, and
  re-evaluate on merge-base movement and coverage transitions.
- **Expected Outcome:** A daemon-side `persistence_route` module returns
  `Base { merge_base_sha }` or `PerWorktree { canonical_root }`, re-entrant on
  the same ref-change trigger (re-evaluates on merge-base movement and
  covered↔uncovered transitions); every fallback emits a structured
  `persistence.route{route, reason}` event. Uncovered topologies (detached HEAD,
  no merge-base, no default branch) route permanently to the per-worktree path.
- **Validation:** Tests cover base↔per-worktree transitions, re-entrancy on
  merge-base movement, and the uncovered-topology fallbacks; each fallback emits
  the structured event.
- **Files:** `crates/anvil-intercept/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GBASE-006

---

#### GBASE-010: Graduation gate + default-on flip

- **Status:** Proposed
- **Intent:** Flip `ANVIL_PERSIST_GRAPH` default-on only after the
  successor-specific soak criteria hold. **(Terminal item.)**
- **Expected Outcome:** A documented graduation gate covers the
  successor-specific criteria — corrupt-shared-base incident rate, herd-miss
  behaviour under single-flight (fleet rebasing onto fresh main simultaneously),
  and warm-start latency budget (base load on the cold-start critical path for N
  worktrees) — plus the standing correctness/GC criteria; the default-on flip is
  the last change and is gated on all of it.
- **Validation:** All gate criteria are green for the agreed soak window; the
  default-on flip is a single reviewed change with the gate evidence attached.
- **Files:** `plans/`, `crates/anvil-intercept/`, configuration
- **Confidence:** low
- **Priority:** Medium
- **Dependencies:** GBASE-001, GBASE-002, GBASE-003, GBASE-004, GBASE-005, GBASE-006, GBASE-007, GBASE-008, GBASE-009, GBASE-011

---

#### GBASE-011: ADR-090 health-envelope wiring for base failures

- **Status:** Merged 2026-07-11 via PR #3276
- **Intent:** Surface base-production failure, claim timeout, and GC error as
  worktree-scoped health signals. **(Parallel track.)**
- **Expected Outcome:** Base-production failure, `O_EXCL` claim timeout, and GC
  error each raise an ADR-090 worktree-scoped health envelope; all such failures
  are non-fatal (base absent ⇒ cold scan serves).
- **Validation:** Tests assert each failure class emits the correct
  worktree-scoped envelope and that the daemon continues serving cold.
- **Files:** `crates/anvil-intercept/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GBASE-002, GBASE-008
