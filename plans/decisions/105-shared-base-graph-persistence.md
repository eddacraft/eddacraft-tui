# ADR-105: Shared base-graph persistence (ADR-069 successor)

## Status

**Proposed** — 2026-07-11, planning council `plan-89a47ac7` (architect,
kernel-maintainer, adversarial-reviewer, operations-reviewer, security-analyst).
Successor to [ADR-069](069-graph-v2-persistence.md): it replaces the on-disk
**storage layout** of the warm-graph snapshot (per-worktree file → one shared
write-once base + live overlays) while inheriting ADR-069's format machinery,
trust line, and privacy line unchanged. It **amends** ADR-069 §5
(single-private-owner) and §10 (orphan sweep); the rest of ADR-069 stands. This
ADR changes **storage layout only** — it is explicitly **not** the event-sourced
persist-and-trust delta-replay design ADR-069 rejected.

## Date

2026-07-11

## Context

[ADR-069](069-graph-v2-persistence.md) persists the daemon's warm graph as a
**per-`WorktreeKey`** sealed-DTO `postcard` snapshot: one file per worktree,
written after a successful background scan and swept on start. That layout has
three structural costs that have kept `ANVIL_PERSIST_GRAPH` **default-off**:

- **O(worktrees) on disk and on scan.** A host with 14+ worktrees of the same
  repo (common here) stores 14+ near-identical snapshots and pays a cold-ish
  rebuild per key, since a worktree's snapshot is private to that worktree even
  when the underlying committed tree is byte-identical to a sibling's.
- **Per-key growth even with a correct sweep.** The per-worktree orphan race is
  already closed — CIB-096 (Merged 2026-06-22 via PR #2870) added the
  `<hash>.root` companion-file existence check to
  `sweep_orphan_snapshots_on_start`, so the cold-boot sweep reclaims orphaned
  per-worktree snapshots with no session-registry keep-set (historical context;
  the historic CIB-092c orphan race is closed by it). A correct sweep does not
  remove the *structural* cost, though: one file per worktree still grows disk
  and scan-work as O(worktrees), and it does not cover the **new orphan class**
  a shared base introduces — a base artefact no live worktree references, keyed
  by merge-base sha rather than by worktree.
- **No sharing.** Two worktrees checked out at the same commit cannot reuse each
  other's parse/resolve work, which is exactly the expensive part a snapshot is
  meant to save.

The insight this ADR turns on: the overwhelming majority of a worktree's graph
is the graph of its **merge-base commit's committed tree** — shared across every
worktree of the repo sitting on that base — and only a small, changed-file
**overlay** is worktree-private. So persist the shared part **once per repo per
merge-base commit** and compose it with a cheap live overlay at warm-start.

This is a storage-layout successor, not a new trust model. Everything ADR-069
froze about *what a snapshot means* — restore indexes never the verdict, come up
stale, content-hash-authoritative reconcile before any certify, discard-on-doubt,
sealed allowlist-only privacy payload — is inherited verbatim. The constraints
from ADR-061/063/064/067 (lean resident daemon: no tree-sitter, no `notify`, no
`walkdir`; parser injected from the CLI) remain load-bearing and are honoured by
producing the base in a **CLI subprocess**, never in the resident daemon.

## Decision

Persist the shared graph as **one write-once, content-addressed base snapshot per
repo per merge-base commit**, composed at warm-start with a **live per-worktree
overlay**, superseding ADR-069's per-worktree snapshot layout. The base is read
from **git objects** (never a working tree), produced in a **detached CLI
subprocess** (never the resident daemon), and shared **on disk only** (shared-RAM
is explicitly deferred). The old per-worktree path is **kept permanently** for
topologies a base cannot cover.

### 1. Core model — shared write-once base + live overlay

- The **base** is the full graph of the **merge-base commit's committed tree**,
  read via git objects (`ls-tree -r` + `cat-file --batch`, reusing the
  `l4_engine.rs` `cat-file --batch` object-read pattern — the full-tree walk via
  `ls-tree -r` is new; `l4_engine` uses `diff-tree` — **zero new dependencies**)
  and parsed
  by tree-sitter in a **CLI subprocess**. The base is **never** read from a
  working tree (a dirty tree would poison a shared artefact) and **never** parsed
  in the resident daemon (ADR-061/064 upheld; ADR-067 parser-injection is the
  precedent — tree-sitter links into the binary, not `anvil-intercept`).
- A **worktree warm-start** loads the base by replay and applies a **live
  overlay** — the diff of the worktree's changed files versus the base tree,
  computed by the [ADR-085](085-daemon-full-scan-executor.md) executor scoped to
  those changed files — into **one materialised `petgraph` per worktree**.
- The base is shared **on disk only**. Each worktree still materialises its own
  resident graph; **shared-RAM base + overlay-query tiering is explicitly
  deferred** (see Alternatives).

### 2. Format — content-addressed write-once artefact

- The base is a **content-addressed, write-once** artefact keyed by the
  **merge-base sha**. Its magic bytes are **`ANVILGB1`**; the per-worktree
  snapshot class keeps **`ANVILGC1`** (from ADR-069), so the two classes are
  distinguishable at load without a header-kind byte.
- The **`SnapshotPayload` DTO is reused verbatim** from ADR-069 — **no new
  header-kind byte**, because adding one would shift `HEADER_LEN` and break the
  committed golden fixtures. The class is carried by the magic bytes, not by a
  new field.
- **One versioning policy covers both classes.** A single
  `SNAPSHOT_FORMAT_VERSION` and a single `SNAPSHOT_BACKING_SCHEMA_VERSION` govern
  both `ANVILGB1` and `ANVILGC1`; a `schema_epoch` bump invalidates **both**
  classes together (discard-and-rebuild per ADR-069 §6 — never migrate).
- All ADR-069 §1/§4 integrity and crash-safety machinery (magic/epoch/counts/CRC
  gate, framed decode, load size-cap, `O_EXCL`/`0600` temp, parent-dir fsync,
  `EXDEV` abort, `O_NOFOLLOW`/openat2) applies unchanged to the base artefact.

### 3. Warm-start composition — id/parity contract

- **Composition target.** Load the base by replay (`add_symbol`/`add_edge`, so
  `petgraph` re-derives its own `NodeIndex`es per ADR-069 §1), then apply the
  live overlay into the same materialised graph.
- **Id contract.** The base owns the id range `[0, base_next_id)`. The overlay
  allocates ids **above a reserved watermark** so base and overlay id spaces are
  **disjoint by construction** (this is the invariant the schema-epoch clause in
  §10 protects). Cross-boundary imports (an overlay symbol referencing a base
  symbol, or vice versa) are **re-resolved at compose time** from the persisted
  raw-specifier map (the `Vec<(String, String)>` forward map ADR-069 §1 already
  stores).
- **Parity fixture.** A golden **COMBINED-STATE** fixture asserts that
  `base(X)` composed with a scripted overlay (adds, removes, tombstones,
  cross-edges) is **identical to a cold scan of the combined on-disk state**.
  This is the correctness anchor for the whole design.

### 4. Trust line — unchanged from ADR-069

The ADR-069 trust line is inherited **verbatim** and remains load-bearing:

> Warm-start **restores the indexes, never the verdict.** A restored workspace
> comes up **stale**; the daemon **MUST NOT** certify until the
> **content-hash-authoritative** reconcile completes.

Composing a shared base with an overlay changes **where the restored indexes
come from on disk**; it does **not** change what a restored index is worth. A
composed graph comes up stale exactly as a per-worktree snapshot did, and the
same content-hash reconcile re-establishes `clean` before any `Certified`
verdict. Because this ADR touches **storage layout only**, it is explicitly
**not** the event-sourced persist-and-trust delta-replay model ADR-069 rejected:
the base is a full-state snapshot of a committed tree, not a replayed mutation
log, and the overlay is recomputed live, never trusted from disk.

### 5. Single-flight production & GC — amends ADR-069 §5 and §10

This section **amends** ADR-069 §5 (single private owner) and §10 (orphan
sweep). The shared base is a **new orphan class** the per-worktree
`<hash>.root` companion sweep (CIB-096) does not cover, and the
single-owner-by-socket model does not extend to a shared artefact written by
transient subprocesses, so:

- **Production is single-flight via an `O_EXCL` claim.** A producer claims
  `.producing/<sha>.lock` created `O_EXCL`, stamped `{pid, start_time}` (the
  save-time-driver convention, with a PID-reuse guard on `start_time`). Only the
  claim holder produces the base for that sha.
- **Claim reclaim.** A stale claim is reclaimable **iff** the stamped pid is dead
  **or** the claim mtime exceeds **2× the p95 production time**. Reclaim itself is
  race-safe: `O_EXCL` rename of a fresh claim + atomic swap, so two reclaimers
  cannot both win.
- **GC is in-daemon refcounting.** The daemon holds a refcount over the
  **current merge-bases of live worktrees**; the keep-set is the
  [ADR-094](094-worktree-registration-ux.md) **ACTMO durable worktree
  registration** set. A base is GC-eligible only when no live registered worktree
  references its sha, and GC **respects active claims** (never removes a base a
  claim is producing).
- Together this **reclaims the shared-base orphan class this ADR introduces**:
  a single well-defined producer per sha, a bounded reclaim rule, and a
  refcount-driven keep-set over ACTMO-registered worktrees. This is the
  merge-base-keyed analogue of the per-worktree `<hash>.root` sweep (CIB-096),
  not a replacement for it — the two operate on different orphan classes.

### 6. Trigger & scheduling — proactive pre-production

- **Directory-level inotify.** `watcher.rs` gains **directory-level** watches
  (`IN_MOVED_TO | IN_CREATE`) on: the common gitdir `refs` dir, the `packed-refs`
  parent, the primary `HEAD`, and per-worktree `HEAD`s. Git ref updates are
  **rename-based**, so a file-inode watch misses the swap — the directory watch
  catches the rename-into-place. Budget: **≤4 descriptors per repo**, counted
  against the **existing watcher budget**. On `ENOSPC`, **degrade** to
  CLI-invocation check-and-request (`anvil start` / `watch` / `status` request
  production on demand). Debounce ~**500 ms**.
- **Merge-base key resolution.** The merge-base is resolved against the
  **default branch** (`origin/HEAD` or configured); `@{upstream}` is used only as
  a **refinement when set**. An unresolvable merge-base ⇒ **skip production, serve
  cold** (non-fatal).

### 7. Production execution model — detached subprocess

- Production runs as a **detached** `anvil graph-base build --merge-base <sha>`
  subprocess. The background-pool thread does **claim + spawn + enqueue only**; a
  **dedicated reaper `std::thread`** owns `child.wait()` and releases the claim.
  The background pool floors at one thread — **blocking it is forbidden**.
- **Newer-sha cancels-and-restarts** via `ScanCancel`. A restart cap of **N=3**
  per sha-lineage ⇒ **serve cold, log, emit an
  [ADR-090](090-daemon-worktree-scoped-health-envelopes.md) health envelope**, and
  **re-arm on quiescence**.
- **All failure is non-fatal.** Base absent ⇒ cold scan serves. Failures surface
  via ADR-090 **worktree-scoped health envelopes**, never a hard error.

### 8. Topology routing & fallback

- A new daemon-side **`persistence_route`** module returns
  `PersistenceRoute::Base { merge_base_sha }` or
  `PersistenceRoute::PerWorktree { canonical_root }`. Routing is **re-entrant on
  the same ref-change trigger**: it re-evaluates on merge-base movement and on
  covered↔uncovered transitions.
- Every fallback emits a **structured event** `persistence.route{route, reason}`.
- The **old per-worktree path is kept permanently** for uncovered topologies:
  detached HEAD, no merge-base, no default branch. The base path **inherits the
  identical `cfg(unix)` Windows gap** as the per-worktree snapshots — tracked
  separately (per ADR-070), **not a new gap**.

### 9. Schema-epoch vs shared base

On an **epoch mismatch** between a loaded base and the running daemon, the base
is **ignored** (cold path) and becomes **GC-eligible once no live worktree
references its sha at the old epoch**. This is **discard-and-rebuild, never
migrate, and never a mixed-epoch composition** — a mixed-epoch base+overlay would
violate the §3 disjoint-id invariant, so it is forbidden by construction.

### 10. Packaging — zero new crates

- **Key-agnostic I/O seam** extracted as a **named submodule**
  `anvil_intercept::snapshot_io::store`, exposing **only** `write_sealed` /
  `load_sealed` / dirfd+`openat2` helpers / size caps. The per-worktree `.root` /
  sweep logic stays **private** — no `pub` scatter.
- **Base producer** is a module `graph_base_producer` in **anvil-cli** (the
  `intercept_symbol_parser.rs` precedent; `daemon_dep_boundary` stays green — the
  parser links into the binary, not the daemon crate).
- **Entry-gate PR.** The `snapshot_io::store` extraction ships as its **own
  no-behaviour-diff PR first**, acceptance-gated by **byte-identical existing
  golden tests**, **before** any base-path code lands.
- **No new crates.** Two new crates (`anvil-snapshot-store` /
  `anvil-graph-base`) were rejected for their workspace-hack + ACKNOWLEDGEMENTS +
  CI-surface cost; module/submodule packaging is chosen instead.

### 11. Graduation gate — successor-specific

The default-on flip of `ANVIL_PERSIST_GRAPH` is the **last** item and is gated on
**all** of the following (these are **successor-specific** criteria, **not**
inherited from ADR-069 §7), plus the standing correctness/GC criteria:

- **Corrupt-shared-base incident rate** within the agreed soak threshold.
- **Herd-miss behaviour under single-flight** — the fleet-rebasing-onto-fresh-main
  scenario (many worktrees simultaneously discovering the same new merge-base)
  behaves within budget under the `O_EXCL` claim.
- **Warm-start latency budget** — base load sits on the cold-start critical path
  for N worktrees and must stay within the measured budget.

## Rationale

The shared base directly removes the two ADR-069 layout costs a correct sweep
cannot: disk and scan-work go from O(worktrees) to O(distinct merge-bases)
(typically one), and sibling worktrees reuse the expensive parse/resolve. The
per-worktree orphan race is already closed by CIB-096; what the base adds is a
new merge-base-keyed orphan class, reclaimed by a refcount keep-set over
ACTMO-registered worktrees.

Reading the base from **git objects** (not a working tree) is the load-bearing
safety choice: a shared artefact must be reproducible and dirty-tree-immune, and
the merge-base commit's tree is exactly that. Producing it in a **CLI
subprocess** keeps the ADR-061/064 lean-daemon invariant intact — the resident
daemon never links tree-sitter — while the ADR-067 injection precedent shows the
parser belonging in the binary, not the daemon crate.

Reusing the `SnapshotPayload` DTO **verbatim** (magic-byte class discriminator,
no header-kind byte) preserves the committed golden fixtures and the single
versioning policy, so the two snapshot classes share one drift-detection and one
integrity gate rather than forking the format. The disjoint-id watermark plus
compose-time re-resolution is what makes composition equal a cold scan — pinned
by the COMBINED-STATE golden fixture, which is the design's top schedule risk and
its correctness anchor.

Keeping the per-worktree path **permanently** (not deleting it) is deliberate:
detached HEAD / no-merge-base / no-default-branch are real topologies a base
cannot cover, and the re-entrant `persistence_route` with structured fallback
events makes every routing decision observable.

### Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| **Shared write-once base (per merge-base) + live overlay, git-object-read, CLI-produced (chosen)** | **Accepted** | O(merge-bases) disk/scan; sibling worktrees reuse parse/resolve; dirty-immune (committed tree); lean daemon preserved (subprocess); new shared-base orphan class reclaimed by refcount GC (per-worktree race already closed by CIB-096) |
| Opportunistic clean-worktree base promotion | Rejected | Dirty-tree poisoning of a shared artefact; the clean-gate is load-bearing and a working tree cannot guarantee it — read committed objects instead |
| CI/server-produced bases | Rejected | Off-trunk work misses (a base is only useful for the developer's actual merge-base); adds a server dependency to a local-first tool |
| Nearest-ancestor fallback (walk to a nearby cached base) | **Deferred** | Cutoff/complexity not justified yet; revisit if merge-base churn proves costly |
| Shared-RAM base + overlay query tier / COW base | **Deferred** | Latency risk to the ADR-031 hot-read budget; on-disk sharing captures most of the win first |
| Idempotent no-lock production | Rejected | Burns redundant scans; cross-process determinism of concurrent producers is unproven — single-flight `O_EXCL` claim is cheaper to reason about |
| `@{upstream}` keying | Rejected | Fails on upstream-less local branches (the majority); default-branch keying with `@{upstream}` refinement covers both |
| Designated owner-worktree writer | Rejected | Election/failover complexity; the `O_EXCL` single-flight claim needs no election |
| Event-sourced persist-and-trust delta-replay | Rejected | Already rejected by ADR-069/ADR-061 in favour of a full snapshot; unchanged here |
| Two new crates (`anvil-snapshot-store` / `anvil-graph-base`) | Rejected | workspace-hack + ACKNOWLEDGEMENTS + CI-surface cost; a named submodule + a CLI module carry the same boundary with none of the packaging weight |

## Consequences

- **Positive:** unblocks the `ANVIL_PERSIST_GRAPH` default-on path (gated on §11);
  disk and scan-work drop to O(distinct merge-bases); sibling worktrees reuse
  parse/resolve; the new shared-base orphan class is refcount-reclaimed (the
  per-worktree orphan race was already closed by CIB-096); the format, trust
  line, and privacy line are
  inherited from ADR-069 with no re-litigation; **zero new dependencies and zero
  new crates**.
- **Negative:** adds a shared write-once artefact class, a single-flight
  production protocol, directory-level ref watches (≤4 descriptors/repo against
  the existing budget), and a compose step to warm-start; the COMBINED-STATE
  parity fixture is a genuine schedule risk.
- **Risks:** (1) composition diverging from a cold scan — anchored by the
  COMBINED-STATE golden fixture (GBASE-007); (2) herd-miss under a fleet rebasing
  onto fresh main — bounded by the `O_EXCL` claim + restart cap, measured at the
  §11 gate; (3) warm-start latency of base load on the cold-start critical path —
  a §11 gate criterion; (4) mixed-epoch composition corrupting the disjoint-id
  invariant — forbidden by construction (§9, discard-and-rebuild).
- **Mitigations:** all failure is non-fatal (base absent ⇒ cold scan serves);
  ADR-090 worktree-scoped health envelopes surface base-production failure, claim
  timeout, and GC error; the entry-gate no-behaviour-diff extraction PR de-risks
  the packaging change before any base-path code; default-off + the §11 graduation
  gate buy soak time.

## Amendment note (ADR-069)

ADR-069 remains **Accepted**. This ADR amends **only** its §5 (single-owner) and
§10 (orphan sweep) for the shared-base class, and supersedes its **on-disk layout**
(per-worktree file → shared base + overlay). ADR-069's §1 format machinery, §3/§4
trust and crash-safety, §6 discard-and-rebuild policy, and §8 privacy line are
**inherited unchanged**. The per-worktree layout ADR-069 describes is retained
permanently for uncovered topologies (§8 above).

## References

- Related ADRs: [ADR-069](069-graph-v2-persistence.md) (predecessor — format,
  trust line, privacy line inherited; §5/§10 amended, layout superseded),
  [ADR-061](061-save-time-daemon-delta-validation.md) /
  [ADR-064](064-intercept-graph-cache-crate-boundary.md) /
  [ADR-067](067-daemon-symbol-feed-parse-hook.md) (lean daemon; parser injected,
  never resident — the base producer honours this as a CLI subprocess),
  [ADR-085](085-daemon-full-scan-executor.md) (the executor that computes the
  overlay scoped to changed-vs-base files),
  [ADR-090](090-daemon-worktree-scoped-health-envelopes.md) (base-production
  failure / claim-timeout / GC-error signalling),
  [ADR-094](094-worktree-registration-ux.md) (ACTMO durable worktree registration
  = the GC keep-set), [ADR-070](070-daemon-windows-buildability.md) (the inherited
  `cfg(unix)` Windows gap), [ADR-031](031-validation-latency-rubric.md) (the hot-read
  budget shared-RAM tiering must not risk — deferred)
- APS module: [`graph-base-persistence`](../modules/graph-base-persistence.aps.md)
  (GBASE-001..011)
- Council provenance: planning council `plan-89a47ac7`
