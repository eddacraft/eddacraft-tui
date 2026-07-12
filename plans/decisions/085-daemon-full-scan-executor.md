# ADR-085: Daemon full-scan executor — background-pool population of the warm graph cache

## Status

**Accepted** — 2026-06-16, Josh. Synthesised by a planning council
(`plan-898d9222`) at the owner's request, after
[ADR-084](084-gctx-graph-handle-access.md) fixed GCTX's graph-handle access as
daemon-RPC over `anvil/gctx/*` and surfaced **binding condition C1** (cold-start
warm-up) on [GCTX-010](../archive/modules/graph-context-delivery.aps.md): the daemon's
`KernelGraphCache` is *save-populated*, so a fresh MCP session sees an empty
graph until a file is saved. This ADR decides the executor that drives a
`request_full_scan` to completion and populates the cache, so the warm-up
triggers C1 demands have something to drive. The executor is owned by a new DSV
work item, [DSV-045](../archive/modules/daemon-save-time-validation.aps.md), which merges
**before** the GCTX-010 PR that adds the session-init + on-demand triggers.

## Date

2026-06-16

## Context

`anvil/request_full_scan` exists on the frozen save-time wire (DSV-002) and is
dispatched in `crates/anvil-intercept/src/save_time.rs`, but today it only sets
the per-`WorktreeKey` assurance machine to `Pending`. **There is no executor loop
that drives `Pending → Running → Clean` and populates the `KernelGraphCache`.**
The graph is populated *only* as a side effect of save-time `validate_paths` →
`apply_delta`. So:

- A fresh MCP session's symbol graph stays cold until the user saves a file,
  which is exactly the failure [ADR-084](084-gctx-graph-handle-access.md) C1
  ("reach a useful graph without manual saves") rules out as product-death.
- `request_full_scan` is an accepted-but-inert verb: callers can enqueue, but
  nothing dequeues.

The primitives the executor needs already shipped under **DSV-006** (PR #2272 /
#2283): `walk_capped` (symlink-skipping bounded walk), `run_chunked_scan` +
`ScanCancel`/`ScanOutcome` (chunked yield with a resume offset), `DosCaps`
(parse-size + walk-depth), and the two-pool `WorkScheduler`
(`crates/anvil-intercept/src/workspace_pool.rs` — a small interactive pool + a
background pool). What is missing is the loop that wires them together behind
`request_full_scan` and feeds parsed symbols into the cache.

Code facts the council relied on (cite as given):

- **The parser is injected, not linked.** Per [ADR-064](064-intercept-graph-cache-crate-boundary.md)
  / [ADR-067](067-daemon-symbol-feed-parse-hook.md), the daemon defines the
  `SymbolParser` trait and links no tree-sitter; `anvil-cli` injects a
  kernel-backed impl via `ForegroundOpts`, held on `SaveTimeState.parser`.
  Save-time `validate_paths` parses the exact guarded bytes it hashed.
- **The cache write-path is parse-free and lock-guarded.** `apply_delta` consumes
  already-parsed `FileSymbols`; the cache sits behind a per-key
  `Arc<Mutex<…>>`-style machine (DSV-004/005). Eviction bumps a generation and
  emits `WarmStateEvicted`.
- **The read anchor survives client disconnect.** The platform-neutral
  `WorkspaceAnchor` (`read_rel`, DSV-010b) reads workspace-relative paths against
  a held directory handle (Unix `O_PATH` dirfd / Windows ADR-068 guard),
  independent of any client connection.
- **The assurance enum is on the frozen wire — and is NOT forward-compatible
  today.** `AssuranceState`
  (`Clean`/`Stale`/`Pending`/`Running`/`Unavailable`) lives in
  `crates/anvil-intercept-proto/src/protocol.rs` (DSV-002) and is consumed by
  `workspace_status`, `watch`, `status`, and GCTX's CE-7 marker. Unlike its
  sibling `StaleReason`, `AssuranceState` carries **no `#[serde(other)]`
  fallback** — so a newly added variant *hard-fails* `serde` deserialisation of
  the entire `WorkspaceAssurance`-bearing response on every shipped
  v0.8.0-beta client. Adding a variant is therefore a **breaking** wire change
  unless the fallback is added in the same change (Decision 5).
- **`Coverage::Partial` already exists with an unrelated meaning.** The
  `Coverage` enum (`Certified`/`Partial`, wire `"partial"`) describes whether a
  *verdict* attested the full check-family set. It is a different axis from
  walk-truncation. Any new truncation signal MUST NOT reuse the word/variant
  `Partial`, or the two collide on the wire string `"partial"`.

The owner set the posture before the council: build the **full** executor (not a
minimal populate-only loop), reuse the existing parser seam and two-pool
isolation, and never produce a phantom `Clean` graph that could be certified or
served as complete when it is not.

## Decision

**Add a background-pool executor that reacts to a `Pending` assurance state by
driving a full workspace scan to completion — walk → parse (on the background
pool) → `apply_delta` → `complete_scan` — populating the warm `KernelGraphCache`,
with cancellation, a timeout watchdog, eviction-rewarm, a scan↔save race guard,
truncation handling, and DoS coalescing.** The executor is a background rayon-pool
job in `anvil-intercept`; it opens its own `WorkspaceAnchor` on the admitted
canonical root and never holds the per-key machine lock across the walk/parse/
apply work.

The decisions below (1–13) map the council's authoritative outcomes.

1. **Full executor, not populate-only.** On a `Pending` state the executor spawns
   a background-pool job that drives `start_scan` → walk + parse + `apply_delta`
   per file → `complete_scan`, and includes cancellation, a `scan_timeout`
   watchdog, and eviction-rewarm. (Decision 2 of the council.)

2. **Parser feed reuses the injected `SymbolParser`, on the background pool.**
   The executor parses each walked file with the same `Arc<dyn SymbolParser>` held
   on `SaveTimeState.parser`, but installs each parse on `scheduler.background()`
   — never `interactive()` — so the two-pool isolation that protects the
   save-time verdict's interactive budget (ADR-031) is preserved. The daemon still
   links no parser (ADR-064/067 hold). (Decision 3.)

3. **No-parser policy: abort, never phantom-`Clean`.** If `parser` is `None`
   (e.g. the Windows daemon today, DSV-010b), the executor MUST `mark_stale` and
   MUST NOT call `start_scan`/`complete_scan`. No empty `Clean` graph is ever
   produced. (Decision 4.)

4. **Scan↔save race: per-key "dirty during scan" flag, origin-agnostic +
   compare-and-clear under lock.** **ANY** `apply_delta` for the key while the
   scan is `Running` sets a per-key dirty flag — *regardless of call origin*:
   interactive `validate_paths` **and** a GCTX on-demand re-warm (or any other
   future caller) all set it. `complete_scan` **reads-and-clears the dirty flag
   atomically within the same per-key machine-lock critical section** that
   performs the `Clean` transition (a compare-and-clear under the lock), so a
   set-dirty cannot interleave between the check and the `Clean` write. If dirty,
   the worktree is marked `Stale(CrossFileResolutionNeeded)` and a fresh scan is
   re-queued; only a clean (never-dirtied) scan completes to `Clean`. This
   prevents a phantom-`Clean` that raced a concurrent save. (Decision 5.)

5. **Truncation → a new `AssuranceState::Bounded` state + a forward-compat
   fallback, in one wire change.** Three coordinated edits land together in
   `crates/anvil-intercept-proto/src/protocol.rs`:
   - **(a) New variant `AssuranceState::Bounded`** (wire `"bounded"`) — *not*
     `Partial`. The name `Bounded` is chosen deliberately to avoid colliding
     with the pre-existing `Coverage::Partial` (wire `"partial"`), which is an
     unrelated check-family-coverage axis; the proto doc-comment must state this
     distinction explicitly.
   - **(b) `#[serde(other)] Unknown` added to `AssuranceState`** in the *same*
     change. Without it, `Bounded` (and any future variant) hard-fails
     deserialisation on shipped v0.8.0-beta clients — i.e. the addition would be
     **breaking, not additive**. With the fallback, an older client that meets a
     newer daemon's unrecognised state string degrades to `Unknown` rather than
     failing the whole response parse, so the change is genuinely additive on
     both ends (no `protocolVersion` bump). **Consumers MUST treat `Unknown`
     fail-safe as `Stale`** (never as `Clean`).
   - **(c) A concrete coverage field on `WorkspaceAssurance`**, *not* on
     `ValidatePathsResponse.coverage` (which is `Coverage`) — to stop
     overloading the word "coverage". The field is
     `scan_coverage: Option<ScanCoverage>` where
     `ScanCoverage { scanned_files: u64, total_files: u64 }`, carried with
     serde `default` + `skip_serializing_if = "Option::is_none"` so it is
     forward-compatible (absent on older daemons, ignored by older clients).

   The walk is pre-filtered through gitignore **before** counting files against
   `max_walk_files`; if the worktree is *still* over the cap, it is
   warm-but-bounded → `Bounded` (carrying `scan_coverage`), never `Clean`.
   **`Bounded` is a lifecycle state with no staleness cause: its
   `WorkspaceAssurance.reason` is `None`, exactly like `Clean`** — the proto's
   `reason`-present-iff invariant doc-comment is updated to add `Bounded` to the
   "`reason` always `None`" set. GCTX maps `Bounded` to
   identity-results-marked-bounded (truncation metadata), never `NotReady`. An
   acceptance criterion requires that `status`/`watch`/GCTX each **explicitly**
   handle `Bounded` (no wildcard-to-`Clean`), verified by an exhaustiveness
   (compile) or per-consumer test. (Decision 6.)

6. **Executor home/anchor: background job, lock held only for the brief calls.**
   The executor is a background rayon-pool job. It holds the per-key machine lock
   ONLY for the short `start_scan`/`complete_scan` transitions, never across
   walk + parse + `apply_delta` (ADR-084 C2). It opens its **own**
   `WorkspaceAnchor` on the admitted canonical root and reads each file via
   `read_rel` (so it survives a client disconnect), converting `walk_capped`
   absolute paths to root-relative before each read. (Decision 7.)

7. **Cross-file resolution: order-independent convergence, validated.** Each
   walked file is fed as `ChangeKind::Create` through the existing `apply_delta`;
   the executor relies on GV2-011 incremental `re_resolve_imports` to converge
   regardless of walk order. This is an **acceptance criterion**, not an
   assumption: a property/equivalence test MUST prove the scan-driven graph is
   equivalent to a save-driven baseline. (Decision 8.)

8. **Yield/cancel: keep applied deltas, re-queue a continuation.** The walk list
   is chunked through `run_chunked_scan` with a per-key `ScanCancel`. An
   interactive `validate_paths` preempts a mid-chunk scan. On a yield, applied
   deltas are kept, the worktree is set `Stale`, and a Background continuation is
   re-queued from the processed offset (`ScanOutcome.processed`). (Decision 9.)

9. **Coalescing / DoS: per-key scan-enqueued CAS flag, reset on ANY job exit.**
   A per-`WorktreeKey` `scan-enqueued` CAS flag dedupes: `request_full_scan`
   enqueues a background job only on a `false → true` CAS. The flag MUST reset on
   **any** executor-job exit — completion, panic, **or** cancellation — via an
   RAII/drop guard, never only on the happy-path completion. Otherwise a panicked
   or cancelled job leaves the flag stuck `true` and `request_full_scan` goes
   **permanently inert** for that key until daemon restart (a liveness hole). N
   repeated `request_full_scan` calls for the same key produce one full scan, not
   N. (Decision 10.)

10. **Triggers: reactive to `Pending` + auto-enqueue on first contact.** The
    executor is reactive to a `Pending` state from any caller. The daemon also
    auto-enqueues a scan on first contact (`validate_paths` / `workspace_status`)
    against a fresh cold key. GCTX-010's PR adds the MCP session-init
    `request_full_scan`, the on-demand re-warm on a cold/`Pending` GCTX query, and
    the `NotReady` + hint — those are GCTX-track triggers on top of this executor.
    (Decision 11.)

11. **DLIFE is a soft dependency.** DSV-045 + GCTX-010 code ship independently of
    DLIFE-002; only the "zero-ceremony start" UX docs (GCTX-032 / release notes)
    gate on DLIFE making the daemon the normal path. (Decision 12.)

12. **Restart: rely on the existing CE-7 `Stale`/`NotReady` signal.** On daemon
    restart the cache is empty → the worktree is `Stale` → the first GCTX call
    returns `NotReady`, and the first-contact auto-enqueue (decision 10) warms it.
    No epoch token and no wire change for restart. (Decision 13.)

13. **SLO: extend the existing `ipc_roundtrip` resource-budget bench, with a
    self-test.** Add a "scan running over a small fixture corpus" scenario to
    `crates/anvil-intercept/benches/ipc_roundtrip.rs` (additive, no new CI job).
    Interactive `validate_paths` p95 MUST hold the ADR-031 80 ms budget with a
    real background scan in flight. The scenario MUST include a **synthetic
    regression self-test mirroring DSV-006**: inject an artificial scan stall and
    assert the gate exits **non-zero** — so the bench is proven to actually catch
    a budget regression, not merely "be present". (Decision 14.)

## Rationale

- **Reusing the injected parser + two-pool scheduler keeps ADR-064/067 intact.**
  The daemon still links no tree-sitter; the executor adds a *consumer* of the
  existing `SymbolParser` seam, on the background pool, so the interactive verdict
  path is never starved.
- **The `Bounded` variant is the honest answer to truncation.** A warm-but-bounded
  worktree is materially different from a complete one; collapsing it into `Clean`
  would let an over-budget repo be served/certified as fully covered. Critically,
  the variant is **only additive once the `#[serde(other)] Unknown` fallback is
  added in the same change** — `AssuranceState` has no such fallback today (unlike
  `StaleReason`), so a bare new variant would *break* deserialisation on shipped
  v0.8.0-beta clients. We pay for the variant + the fallback + a `scan_coverage`
  field on `WorkspaceAssurance` (named distinctly from the unrelated
  `Coverage::Partial` axis) — cheaper than the silent-incompleteness bug it
  prevents, and now genuinely forward-compatible on both ends.
- **The dirty-during-scan flag closes the only phantom-`Clean` window.** Without
  it, a save that lands mid-scan could be overwritten by a `complete_scan` that
  certifies a graph missing that save. The flag is set by **any** `apply_delta`
  during `Running` (interactive or GCTX re-warm), and `complete_scan`
  compare-and-clears it **under the same per-key lock** as the `Clean`
  transition, so set/check cannot interleave. The flag makes the race fail safe
  (`Stale` + re-queue), preserving the ADR-061 "never certify on doubt" posture.
- **Lock-held-only-for-transitions matches ADR-084 C2.** Holding the per-key
  machine lock across the whole walk would block every concurrent `validate_paths`
  for that worktree and blow the ADR-031 budget; scoping the lock to the brief
  `start_scan`/`complete_scan` calls keeps the hot path responsive.
- **Validating order-independent convergence, not assuming it.** GV2-011's
  incremental resolver is the load-bearing claim; an equivalence test against the
  save-driven baseline is the only way to keep a scan-warmed graph trustworthy as
  the GCTX read source.
- **The coalescing CAS makes `request_full_scan` DoS-safe.** A hostile or
  enthusiastic client cannot multiply full scans by spamming the verb.

### Alternatives considered

| Option | Pros | Cons |
|--------|------|------|
| Full executor on the background pool (chosen) | Populates the cache for cold sessions; cancellation + watchdog + rewarm; preserves two-pool isolation; DoS-safe; honest `Bounded` | Two-state-machine surface (scan + verdict); a wire change (new `Bounded` variant + `#[serde(other)] Unknown` fallback + `scan_coverage` field); an executor module to keep correct |
| Fold the executor into GCTX-010 | One PR; no DSV-track split | Couples a daemon-internal loop to the GCTX consumer; muddies track ownership; GCTX-010 already carries C1–C5. Rejected — clean track separation: DSV owns the executor |
| Minimal populate-only loop (no cancel/watchdog/rewarm) | Smaller diff | A long scan can't yield to interactive work (ADR-031 risk); an evicted cache never re-warms; no race guard ⇒ phantom-`Clean`. Rejected |
| Treat truncation as a silent `Clean` | No wire change | Serves/certifies an over-budget repo as complete — the exact incompleteness bug `Bounded` exists to prevent. Rejected |
| Add `Bounded` *without* the `#[serde(other)]` fallback | Smallest proto diff | **Breaking**: hard-fails deserialisation of every `WorkspaceAssurance`-bearing response on shipped v0.8.0-beta clients. Rejected — the fallback ships in the same change |
| Reuse the existing `Partial` name/variant for truncation | No new variant | Collides with `Coverage::Partial` (wire `"partial"`), an unrelated check-family axis; conflates two meanings. Rejected |
| Epoch token / wire change for restart | Explicit restart signal | The existing CE-7 `Stale`/`NotReady` + first-contact auto-enqueue already covers restart; a new token is unneeded wire surface. Deferred |

## Consequences

- **Positive.** A fresh MCP session reaches a useful graph without manual saves
  (closes GCTX-010 C1's product-death risk); `request_full_scan` becomes a live,
  DoS-safe verb; `watch`/`status`/GCTX gain an honest `Bounded` state for bounded
  worktrees; the cache re-warms after eviction and restart. The executor lands on
  the existing DSV-006 primitives, so the new surface is the loop, not the
  building blocks.
- **Negative / accepted.** The daemon gains a second state machine (scan lifecycle
  alongside the verdict lifecycle) and a new executor module. The `Bounded`
  variant is a change to the frozen assurance enum — additive and
  forward-compatible **only because the `#[serde(other)] Unknown` fallback ships
  in the same change**; it touches the frozen proto crate (`AssuranceState`, the
  new `ScanCoverage` struct + `scan_coverage` field on `WorkspaceAssurance`, and
  the updated `reason`-invariant doc-comment) and every consumer's state match,
  which must now explicitly handle `Bounded` (no wildcard-to-`Clean`). The
  scan↔save dirty flag and the coalescing CAS add per-key state.
- **Risks.** Order-dependent convergence (mitigated by the equivalence
  acceptance test over a corpus that includes a cycle, a diamond, and ≥10
  cross-file-import files); a long scan starving interactive work (mitigated by
  the background pool + chunked yield + the ADR-031 bench scenario with its
  injected-stall self-test); a phantom-`Clean` on a raced save (mitigated by the
  origin-agnostic dirty flag with compare-and-clear under lock); the no-parser
  path producing an empty graph (mitigated by the abort-to-`Stale` policy); a
  **coalescing-CAS liveness hole** where a panicked/cancelled job leaves the
  `scan-enqueued` flag stuck and `request_full_scan` goes permanently inert for
  that key (mitigated by resetting the flag via an RAII/drop guard on **any** job
  exit — completion, panic, or cancellation); a **breaking wire change** if a new
  `AssuranceState` variant ever ships without the `#[serde(other)]` fallback
  (mitigated by adding the fallback in this same change).
- **Sequencing.** DSV-045 merges first; GCTX-010's PR rebases onto a main that has
  DSV-045 and adds only the session-init + on-demand warm-up triggers and the
  `NotReady` hint. DLIFE is a soft dependency (UX docs only).

## References

- [ADR-084](084-gctx-graph-handle-access.md) — GCTX graph-handle access; surfaced C1 cold-start warm-up
- [ADR-064](064-intercept-graph-cache-crate-boundary.md) — parser-free daemon / graph-cache crate boundary
- [ADR-067](067-daemon-symbol-feed-parse-hook.md) — daemon symbol feed via injected `SymbolParser`
- [ADR-031](031-validation-latency-rubric.md) — save-time latency budget (80 ms p95)
- [ADR-061](061-save-time-daemon-delta-validation.md) — save-time daemon delta validation; assurance states
- [DSV-006](../archive/modules/daemon-save-time-validation.aps.md) — two-pool scheduler, `walk_capped`, `run_chunked_scan`, `ScanCancel`, `DosCaps`
- [DSV-045](../archive/modules/daemon-save-time-validation.aps.md) — full-scan executor (owns this ADR's implementation)
- [GCTX-010](../archive/modules/graph-context-delivery.aps.md) — `anvil_search_symbols`; consumes the warmed cache, adds the warm-up triggers
- [daemon-lifecycle (DLIFE)](../archive/modules/daemon-lifecycle.aps.md) — soft dependency; makes daemon-backed protection the normal path
- Planning council session: `plan-898d9222`
