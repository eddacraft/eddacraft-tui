# ADR-069: Graph V2 persistence and snapshot strategy

## Status

**Accepted** — 2026-06-04, Josh (sole owner of the GV2 surface). The GV2-021
review ran 2026-06-04 (full council: council-reviewer, kernel-maintainer,
security-analyst, adversarial-reviewer, operations-reviewer) returning
**SOUND-WITH-FIXES**; the majors are folded in, and the one **critical** (a false
serialization claim — the live graph types are not serde-serializable) was
resolved by a **design council** (2026-06-04) that chose the §1 sealed-DTO +
golden-fixture + `postcard` mechanism. Closes the open "Persistence strategy ADR
drafted" item the GV2 Ready checklist assigns to GV2-021, and is the direct unblock
for **daemon save-time sub-phase B** (warm-start persistence), which `index.aps.md`
records as blocked on this ADR.

## Date

2026-06-04

## Context

[ADR-061](061-save-time-daemon-delta-validation.md) makes the intercept daemon
the save-time validation authority over a warm per-`WorktreeKey` graph cache.
Sub-phase A ships that over an interim `SymbolGraph` cache that **rebuilds on
restart** (no persistence). Sub-phase B adds **warm-start persistence** so a
restarted daemon does not pay a full cold rebuild before it can serve verdicts.
[ADR-064](064-intercept-graph-cache-crate-boundary.md) gives the graph state a
parser-free home in `eddacraft-anvil-graph-cache`; [ADR-063](063-gv2-hot-path-boundary.md)
fixes which reads are hot-path-admissible and explicitly classes `persist` as a
**background-only** operation. The save-time contract
[§9](../specs/2026-06-01-daemon-save-time-validation-contract.md) sketches the
persistence requirements concretely and names this ADR (GV2-021) as the owner of
the binding decision and the privacy allow/deny checklist.

A decision is needed now because sub-phase B cannot start without one, and
because the persistence shape constrains the GV2-022 hot-index types
(`format_version` / `backing_schema_version` fields) that A′ wants to freeze.

**The assumptions to reconcile.** Earlier GV2/GCTX thinking floated a *queryable*
persistent graph store — zero-copy `rkyv` for mmap-resident reads, or an embedded
`SQLite` graph DB (the hearth-rearchitecture brainstorm assumes SQLite + mmap'd
symbol graph). The save-time daemon requirements collapse those assumptions:

- Graph state is **derivable cache state** (GV2 constraint; ADR-061 "restore
  indexes, never the verdict"). The persisted artefact is read **once** at
  warm-start, fully loaded into the resident indexes, then never touched again —
  it is not queried from disk.
- A restored workspace comes up `stale`/`pending` and a cheap reconcile
  re-establishes `clean`, so a stale or even corrupt snapshot is **never** a
  correctness hazard — at worst it is a discarded file and one cold rebuild.
- ADR-061/ADR-064 work hard to keep the resident daemon **lean** (no
  tree-sitter, no `notify`, no `walkdir`). Adding `rusqlite` (native build) or
  `rkyv` (a second serialization framework + `unsafe` zero-copy surface) for a
  load-once cache spends dependency weight those ADRs explicitly refuse.

So the load-bearing question is narrow: **what is the on-disk shape of a
load-once, disposable, default-off warm-index cache that the lean daemon can
write atomically and discard safely?**

## Decision

Persist the warm graph indexes as a **versioned, sealed-canonical-DTO serde
binary snapshot** (`postcard`) — not an embedded database, not a zero-copy mmap
store, and **not** a self-describing codec (§1 rejects CBOR) — owned by
`eddacraft-anvil-graph-cache`, written and read by the daemon
(`anvil-intercept`), **default-off** in v1.

### 1. Format — sealed canonical DTO + translator, load-once

> This section is the **design-council verdict** (2026-06-04) on the serialization
> mechanism, resolving the critical raised in the GV2-021 council review. It chose
> **Candidate A-plus-golden** over a self-validating "structural fingerprint"
> (proven infeasible in Rust without reflection) and over a self-describing codec
> (wrong for a discard-on-doubt cache). EIP framing: **Canonical Data Model** +
> **Message Translator** + a golden fixture as a tamper-evident **Format Indicator**.

- **Do not serialize the live graph.** `SymbolGraph`/`DependencyGraph` do not
  derive serde, petgraph's `serde-1` is off, and `petgraph::NodeIndex` is an
  **ephemeral arena slot** — persisting and trusting it corrupts the id→slot index
  across a deserialize or a petgraph bump. Instead, serialize a **sealed
  `SnapshotPayload` DTO** built from the canonical key the graph **already owns**:
  the semantic `SymbolNode.id: u64`. (`SymbolEdge` is already `{ from: u64, to:
  u64 }`, so edges persist by semantic id with no translation cost.)
- **Save = translate; load = replay-and-reconstruct.** Write walks the live graph
  into the DTO: `nodes: Vec<SymbolNode>` (**including** the synthetic external-module
  nodes `resolve_import` allocates), `edges: Vec<(u64, u64, EdgeType)>`, `next_id`,
  and the dependency **forward map with raw specifier strings** `Vec<(String,
  String)>` (the resolved ids alone cannot reconstruct `re_resolve_imports`). Load
  replays through `add_symbol`/`add_edge`, so petgraph re-derives its own
  `NodeIndex`es and the `HashMap<u64, NodeIndex>` / `files` / `reverse` indexes are
  rebuilt by their existing derivation paths. The on-disk format never sees a
  `NodeIndex` — the bug is gone by construction.
- **Codec: `postcard`** (pure-Rust, no_std, `alloc`), **not bincode** — bincode v1
  and v2 are wire-incompatible, so a dep bump silently misreads old snapshots;
  postcard has a formally versioned, stable wire format. This is the one new dep,
  within the ADR-064 budget. No `rusqlite`, no `rkyv`, no native build step.
- **Envelope + integrity (caught at load, not deferred to reconcile).** A fixed
  header — magic bytes `b"ANVILGC\0"`, `format_version: u32`, `schema_epoch: u8` —
  precedes the postcard body; the body carries **node-count + edge-count** and a
  **payload checksum (CRC32)**. On load the daemon validates magic, epoch, counts,
  and checksum **before** accepting the indexes; any mismatch, a truncated/torn
  body, a duplicate id, or a dangling edge endpoint ⇒ `SnapshotLoadError` ⇒ cold
  rebuild. This load-time integrity gate is **mandatory**: a silent decode
  corruption that preserved on-disk file content (e.g. an enum-variant shift
  decoding every `Function` as another kind) would otherwise survive the
  content-hash reconcile (§3) unchallenged and could be certified — counts +
  checksum + framed postcard decode close that gap at load.
- Loading deserializes the whole payload into resident state in one pass; nothing
  is queried from the file at runtime.

### 2. Location — owner-only, machine-local, never in the repo

- Snapshots live under the `ANVIL_HOME` / XDG state dir, resolved via the
  existing `anvil_home_prefix()` + socket-dir resolver (ADR-060), at e.g.
  `<state-dir>/graph-cache/<worktree-key-hash>.snap`.
- Permissions are **owner-only** (`0700` dir / `0600` file), per-uid. Snapshots
  are **never** written into the repo tree; a `.gitignore` entry is added as
  belt-and-suspenders.

### 3. Rebuild behaviour — restore indexes, never the verdict

- Warm-start **restores the indexes only**. The restored workspace comes up
  `stale`/`pending`; a reconcile re-establishes `clean`. There is no persisted
  verdict and **no `delta_sequence`/watermark** — the staleness race is removed by
  construction (ADR-061).
- **Verdict gate (load-bearing safety property — frozen).** A snapshot load sets
  the workspace assurance state to `stale(cross-file-resolution-needed)` and the
  daemon **MUST NOT return a `Certified` verdict** for that `WorktreeKey` until the
  reconcile completes. Between snapshot-load and reconcile-complete, `validate_paths`
  returns `partial`/`stale`, never `Certified`. The reconcile is the existing
  connect-time full scan the watch client re-issues on reconnect (ADR-061 §9 / spec
  §6); the daemon does not self-certify off un-reconciled warm indexes. This is what
  makes a stale-or-wrong snapshot a non-hazard: the worst case is a delayed `clean`,
  never a false `Certified`.
- **The reconcile is content-hash-authoritative, never mtime-trusting.** It
  re-hashes file content to confirm the restored indexes still match disk; it does
  **not** trust a snapshot-recorded `(inode, mtime, size)` tuple to skip a file
  (per spec §3 "mtime alone is not sufficient; the daemon-computed content hash is
  the authoritative tiebreak"). This closes the TOCTOU/clock-skew hole where a
  same-size, mtime-reset off-daemon edit would otherwise leave a silently wrong
  index. (Re-hashing is cheap relative to re-parse + graph-rebuild — the parse and
  resolve work is what the snapshot actually saves, not the hash.)
- **Reconcile is not the integrity backstop for the snapshot itself.** Reconcile
  invalidates on *content-hash mismatch vs disk*, so a silent **decode** corruption
  that left a node's backing file unchanged (e.g. an enum-variant shift) would
  survive it. That class is caught at **load** by the §1 integrity gate
  (magic/epoch/counts/checksum + framed decode), never deferred to reconcile.
- A snapshot that is **missing, unreadable, corrupt, or version-mismatched** ⇒
  **cold rebuild with a typed, structured log** (see §10). The daemon **never
  refuses to start and never panics** on a bad snapshot. `load_snapshot` returns a
  typed `SnapshotLoadError` whose every variant maps to "discard and cold-rebuild,"
  not to a hard failure.

### 4. Crash-safety & atomic write

The write is **atomic and durable**, and these points are **implementation
requirements**, not implementation details:

- **Temp in the same directory, durably renamed.** Serialize to a temp file
  **created `O_CREAT | O_EXCL` at mode `0600` from the first syscall** (never
  default-umask-then-`chmod`, which opens a perms window) with a randomized suffix
  (e.g. `<key>.snap.<rand>.tmp`) to defeat same-uid pre-create/symlink races; `fsync`
  the file; `rename` over the target; then **`fsync` the parent directory** so the
  directory entry is durable (a file `fsync` alone does not guarantee the rename
  survives a crash on ext4). A crash at any point leaves the old snapshot or no
  snapshot — never a torn one.
- **Same-filesystem by construction.** Temp and target are both under the state
  dir, so `rename` is atomic. If `rename` returns `EXDEV` the write is **aborted
  and logged** — there is **no non-atomic copy fallback**.
- **All write/open is symlink-safe.** Create/open the `graph-cache` dir and the
  snapshot via `openat` relative to an `O_PATH` fd on the state dir, with
  `O_NOFOLLOW` on the snapshot file (the same `openat2`/`RESOLVE_NO_SYMLINKS`
  read-safety discipline ADR-061 / the save-time contract mandate for workspace
  paths, applied here to `ANVIL_HOME` itself), so a planted symlink at the expected
  path cannot redirect the daemon's write or read.
- **Write failure degrades, never wedges.** Any `io::Error` from the write path
  (`ENOSPC`, `EROFS`, `fsync` failure, …) **unlinks the temp**, logs `WARN` with
  path + errno, increments a failure counter (§10), and returns `Ok(())` — the
  daemon continues with no persistence. A snapshot failing to write is never fatal.
- **Bounded load.** On read, an envelope-level **size cap** is checked before the
  codec is invoked, so a crafted/oversized length prefix cannot trigger an
  allocation bomb; over-cap ⇒ `SnapshotLoadError::Oversized` ⇒ cold rebuild.

**Write cadence (mechanism named, numbers not frozen).** The snapshot is written
**after a successful full/background scan** and **on graceful daemon shutdown** —
coalesced behind a dirty-window debounce, **never per-save**. It is **not** written
on crash, so a crash-then-restart still pays one cold rebuild per `WorktreeKey`
until the next successful scan writes a fresh snapshot. (This sets honest
expectations: persistence accelerates restart-after-graceful-stop, not
restart-after-crash.) The debounce window, LRU capacity, and re-warm cap are tunable
impl details validated by RLB benches.

**Re-warm strategy.** Re-warm is **lazy by default** — a snapshot is loaded on the
first `validate_paths` for its `WorktreeKey`, not eagerly for all keys at daemon
start — so the daemon is immediately available and a many-worktree host (14+ here)
does not pay a startup memory/CPU spike. Concurrency is capped **per host**, not
per key. Eager vs lazy is a non-frozen tuning choice validated by RLB benches; the
**per-host cap** and **immediate-availability** properties are the frozen ones.

### 5. Multi-process reader stance — single private owner, no shared-reader contract

- A snapshot is the **private cache of one daemon instance** for one
  `WorktreeKey`. There is **no cross-process shared-reader contract** and no
  WAL/lock protocol: single daemon per `ANVIL_HOME` is already enforced by the
  existing socket/PID model (ADR-036/ADR-060).
- The **daemon-absent fallback does not read snapshots** *and MUST NOT write
  them* (a forward-compatibility constraint, not just current behaviour) — it runs a
  scoped cold scan and reports `unavailable` (ADR-061). This keeps "single writer"
  true by construction and avoids a shared-reader coordination surface for a
  load-once cache.
- **All snapshot I/O happens after the daemon holds its exclusive lock**, so a
  stale-PID startup race cannot have two instances reading-then-writing the same
  file. A snapshot observed before the lock is acquired is treated as untrusted and
  re-validated under §3's "any anomaly ⇒ cold rebuild" rule. The single-owner
  guarantee assumes `ANVIL_HOME` resolves to a unique physical path; two
  `ANVIL_HOME`s symlinked onto the same state dir are unsupported.

### 6. Versioning & migration — disposable, so never migrate

- The header carries `format_version` (envelope/codec version) and `schema_epoch`
  (the index schema generation — bumped when the interim `SymbolGraph` backing is
  swapped for the A′ GV2 hot-index slice, invalidating all pre-swap snapshots: one
  cold rebuild on first start after the swap).
- Any version, epoch, count, or checksum mismatch is a **cold-rebuild trigger,
  never a migration**. The snapshot is disposable cache state, so **no migration
  code is ever written** — we discard and rebuild. This is the single biggest
  simplification the "derivable cache" stance buys.
- **Drift is caught automatically, not by human discipline.** A hand-bumped
  version string alone is unsafe (a forgotten bump + a tagless decode = silent
  wrong data) — and a "machine-derived structural fingerprint" is not achievable in
  Rust without reflection (it collapses to a hand-maintained const or a build-script
  source-hash that is **blind** to cross-crate enum-variant inserts and type-alias
  changes — the worst cases). Instead a **committed golden round-trip fixture test**
  in `anvil-graph-cache` fails CI the moment the wire bytes change for *any* reason
  (field add/reorder, enum variant, codec change), forcing an explicit
  fixture-regeneration + `schema_epoch` bump. The fixture uses distinguishable
  field values so a positional field-swap is caught. CI-time (golden) + load-time
  (counts + checksum, §1) together make silent schema drift non-shippable and
  silent corruption non-loadable.

### 7. Default-off, with graduation criteria

> **Graduated 2026-07-12:** the successor gate in
> [ADR-105](105-shared-base-graph-persistence.md) (Consequences) was met and
> `ANVIL_PERSIST_GRAPH` is now **default-on** with an explicit opt-out — see
> `plans/audits/2026-07-12-gbase-graduation-gate.md`. The criteria below are
> retained as the historical bar this section set.

- Persistence is gated by `ANVIL_PERSIST_GRAPH`, **defaulting off** in v1.
  Warm-start persistence is opt-in until the privacy line and crash-safety have
  soaked; the unset default is byte-for-byte today's rebuild-on-restart behaviour.
- **Fail-closed.** Any unparseable or non-affirmative value of `ANVIL_PERSIST_GRAPH`
  resolves to **off** (no persistence), matching the project's "no silent defaults /
  config-load-failure fails closed" discipline. A test asserts that with the flag
  unset the daemon writes **nothing** under the `graph-cache` state dir across a
  start/stop/restart cycle (no temp file, no dir creation, no probe).
- **Graduation criteria (so it does not ship dead).** Default flips to **on** only
  when all hold: (a) the privacy reachability test (§8) and snapshot round-trip
  tests have been green in CI for the agreed soak window; (b) opt-in field telemetry
  shows zero `SnapshotLoadError::Corrupt` events and zero data-loss reports across
  the soak; (c) snapshot write latency stays within the coalescing window on the RLB
  SLO bench; (d) the GV2-021 owner signs off. Flipping the flag back off leaves
  existing snapshots inert (ignored, not deleted, no error) until re-enabled.

### 8. Privacy line — the concrete allow/deny checklist (GV2-021)

Persist **structural identity only**. The gate is a concrete, testable
allow/deny list:

**ALLOW (structural identity needed for boundary checks):**

- Symbol names / qualified names (the `dependents_of` reverse-index is keyed on
  them) — **stored as cleartext** (see residual-risk note below)
- Import / module / path identity and edge endpoints (`A→B` existence) — also
  cleartext
- File / path identity — **MUST be workspace-root-relative**, never an absolute path
  or home-dir prefix (asserted in the no-leak test below; not left to convention)
- File-content hashes / digests **where present** (a digest is one-way; this is
  *not* a backstop for the cleartext name/path fields above — see note)
- Boundary / layer membership flags and architectural-index results
- Language, visibility, and symbol-kind metadata
- Source span **positions** — represented as a `ByteRange { start: u32, end: u32 }`
  (or equivalent struct with **no text field**), so the type is structurally
  incapable of holding the spanned text

**DENY (never persisted):**

- Raw source bodies, file contents, or code snippets
- Comment text, docstrings, or the text a span points at
- String / numeric literal **values**, especially secret-shaped literals
- Any opaque container (`Vec<u8>`, `Bytes`, `serde(flatten)` maps, `serde(other)`
  catch-alls) that could smuggle arbitrary bytes past a field-name check
- Any field whose value is the source text rather than a name, identity, hash, or
  position

**Residual risk, stated honestly (not papered over by the hash claim).** The
payload stores **cleartext** symbol names, import specifiers, and relative paths —
which are author-controlled text and *can* embed secret-shaped values (a generated
constant name, a token-in-a-URL import specifier). The earlier framing leaned on
"hashes are one-way"; that does not apply to these cleartext identity fields. For a
**default-off, single-uid, owner-only, machine-local** cache this residual risk is
**explicitly accepted** as bounded by the same-uid trust boundary (the snapshot does
not widen it) — it is not scrubbed in v1. The span-position reassurance is genuine
but is *not* where the risk lives; the risk is in the names/specifiers, and we name
it rather than hide it.

**Enforcement — structural, not a name match.** The DENY list is enforced by making
the serialized payload a **sealed allowlist-only DTO** (the same `SnapshotPayload`
type the §1 serialization decision defines): it is built only from an allowlisted
primitive set (named-field structs of integers, identity `String`s, enums, span
ranges — **no** `Vec<u8>`/`flatten`/`other`), the resident indexes are *translated
into* it at write time, and a structural test rejects any transitive field type
outside the allowlist. So a future field added to the live graph types for an
unrelated reason cannot reach the snapshot without an explicit, reviewed change to
the DTO — the guarantee is "the smuggling field will not compile in," not "we
remembered to test." The no-leak test additionally asserts every path field is
relative. At rest the snapshot is further protected by owner-only perms +
machine-local location (§2).

> The exact DTO shape (and how it reconstructs the `petgraph` node index on load) is
> the subject of the §1 serialization decision, which is under design-council review;
> this privacy section depends on that DTO being sealed and allowlist-only, however
> its fields are finalised.

### 9. Crate home & API

- Serialization lives in `eddacraft-anvil-graph-cache` (ADR-064) so the interim
  sub-phase A cache and the A′ GV2 hot-index slice share **one** snapshot
  contract. The daemon (`anvil-intercept`) drives **timing** (when to write, when
  to load); it links no new dependency beyond the serde codec.
- Shape of the API (names indicative, not frozen): the crate **derives the snapshot
  path from the `WorktreeKey`** so path policy (§2) is enforced by the crate, not the
  caller —
  `write_snapshot(indexes, key: &WorktreeKey) -> io::Result<()>` (atomic per §4) and
  `load_snapshot(key: &WorktreeKey) -> Result<WarmIndexes, SnapshotLoadError>`.
- `SnapshotLoadError` is a **typed enum, every variant ⇒ cold-rebuild** (§3), and
  each variant carries the fields §10 needs to log/meter it:
  `NotFound`, `VersionMismatch { found, expected }`, `Oversized`, `Corrupt { source }`,
  `Io { source }`.

### 10. Lifecycle, disk, and observability

- **Snapshot lifecycle / GC.** Snapshots are disposable cache, but they must not
  leak: on daemon start the crate **sweeps `graph-cache/` and removes any snapshot
  whose `WorktreeKey` has no registered worktree** (and removes a key's snapshot when
  that workspace is evicted from the LRU / unregistered). Orphaned `*.tmp` files from
  an interrupted write are also swept on start. **Deleting anything under
  `graph-cache/` is safe at any time** (documented operator escape hatch) — the
  daemon cold-rebuilds the affected key.
- **Disk-full / write failure** is handled per §4: log + meter + degrade to
  no-persistence, never wedge.
- **Observability — distinguishable cold-rebuild causes.** A discard is logged at a
  level that lets an operator tell the three apart without a dashboard:
  `NotFound` ⇒ `DEBUG` (expected first run); `VersionMismatch` ⇒ `INFO` with the
  found/expected version pair (expected, one-time, after a schema bump);
  `Corrupt`/`Io`/`Oversized` ⇒ `WARN` with path + error (investigate). A
  `snapshot_load_result{outcome=…}` counter and a `snapshot_write_result{outcome=…}`
  counter make corruption/failure rates trackable across a fleet rollout. When
  persistence is **explicitly enabled**, a write failure also raises an ADR-035
  Notification (so an opted-in user sees degradation rather than silent loss).

## Rationale

The decision turns on the fact that the persisted artefact is **load-once,
disposable cache state**, which removes every advantage the heavier options
exist to provide:

- **Zero-copy (`rkyv`)** pays off when you mmap a large structure and read fields
  in place without deserializing. Here we deserialize the whole snapshot once at
  start and then serve from resident indexes — the mmap-in-place path is never
  exercised, so `rkyv` buys nothing while adding a second serialization framework
  and an `unsafe` zero-copy surface to a daemon two ADRs keep deliberately lean.
- **Embedded DB (`SQLite`/`rusqlite`)** pays off when you query persisted state
  with predicates, need concurrent readers/writers, or need durable transactional
  mutation. The hot path is **forbidden** from reading the graph off disk
  (ADR-063 classes `persist` as background-only); there is one private writer and
  no shared readers (§5); and the store is disposable, so transactional durability
  is irrelevant. `rusqlite` would add a native build step to the resident daemon
  for zero realised benefit.
- **A serde blob over a sealed canonical DTO** matches the access pattern exactly:
  write whole, read whole, discard on any doubt. It reuses the `serde` derives the
  *leaf* graph types already carry (translating the live, non-serde container types
  into the DTO at write time — §1), adds no native dependency, and makes "version
  mismatch ⇒ throw it away" trivially correct.

**Serialization mechanism (design-council verdict, §1).** Within the serde-blob
family the mechanism is a **Canonical Data Model** keyed by the semantic `u64`
symbol-id (which the graph already owns and enforces) with save/load **Message
Translators**; load reconstructs petgraph's `NodeIndex` by replaying inserts, so the
ephemeral arena slot is never persisted. Schema-drift safety is a **committed golden
round-trip fixture** (CI fails on any wire change) plus load-time **counts +
checksum**, chosen over a hand-bumped version (silent-drift footgun) and over a
"structural fingerprint" (not derivable in Rust without reflection — collapses to a
const or a drift-blind source-hash). `postcard` is chosen over `bincode` for a
formally stable wire format (no v1/v2 incompatibility footgun). A self-describing
codec (CBOR) and event-sourced delta-replay were rejected — the first reintroduces
graceful-tolerance risk a discard-on-doubt cache does not want, the second was
already rejected by ADR-061 in favour of a full snapshot.

Pinning versioning to **discard-and-rebuild rather than migrate** is the same
"derivable by default" discipline GV2 already commits to: persisted graph
snapshots are cache, never a source of truth, so the project never carries
snapshot-migration code — a permanent maintenance saving. The privacy line is
drawn at **structural identity vs source text** because boundary checks need
names, edges, hashes, and positions but never the bytes in between; making the
snapshot payload type structurally incapable of holding source text turns the
privacy gate into a compile-time + unit-tested property rather than a review
convention.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Serde binary snapshot over a sealed canonical DTO, load-once, discard-on-mismatch (chosen)** | Matches write-whole/read-whole/discard access pattern; reuses the leaf graph types' existing `serde` derives via a translation step (sealed DTO + `NodeIndex` reconstruction, §1); no native dep into the lean daemon; version mismatch ⇒ throw away (no migration code ever); durable atomic write | Not queryable from disk (not needed); whole-file rewrite per snapshot (bounded by the §4 coalescing cadence) |
| **Zero-copy `rkyv` mmap store** | In-place reads without deserialize; compact | Mmap-in-place path never exercised (we load-once then serve from RAM); adds a 2nd serialization framework + `unsafe` zero-copy surface to a daemon ADR-061/064 keep lean; zero realised benefit for a load-once cache |
| **Embedded `SQLite` (`rusqlite`) graph DB** | Queryable; mature; concurrent reader story | Hot path may not read graph off disk (ADR-063); single private writer, no shared readers (§5); disposable store needs no transactions; native build step into the resident daemon — cost with no matching benefit |
| **Persist-and-trust `Clean` across restart** | Warm-start is "instantly clean" | Snapshot-staleness race (delta after snapshot, then crash ⇒ false `Clean`); already rejected by ADR-061 — restore indexes, re-derive verdict |
| **No persistence (status quo)** | Simplest; nothing to secure at rest | Every daemon restart pays a full cold rebuild before serving verdicts — the sub-phase B cost this ADR exists to remove |

**Serialization mechanism within the serde-blob family** (design-council, 2026-06-04):

| Mechanism | Verdict | Why |
|-----------|---------|-----|
| **Sealed DTO + `postcard` + golden fixture + load-time counts/checksum (chosen)** | **Accepted** | Canonical model on the existing `u64` id; `NodeIndex` reconstructed on load; drift caught at CI (golden) + corruption at load (counts/checksum); `postcard` has a stable wire format |
| Sealed DTO + `bincode` + hand-bumped version (Candidate A as-stated) | Improved upon | Correct spine, but a forgotten version bump + tagless `bincode` ⇒ silent wrong decode; bincode v1/v2 wire-incompatible |
| Sealed DTO + machine-derived "structural fingerprint" (Candidate B) | Rejected | Not derivable in Rust without reflection — collapses to a hand const (same footgun) or a build-script source-hash **blind** to cross-crate enum-variant inserts + type-alias changes; a golden fixture catches strictly more |
| Self-describing codec (CBOR / `ciborium`, Candidate C) | Rejected | Graceful additive tolerance is the wrong instinct for a discard-on-doubt cache; reintroduces "loaded the wrong thing" risk; larger/slower for no queryability gain |
| Serialize live `petgraph` via `serde-1` (Candidate D) | Rejected | Reintroduces the `NodeIndex`-stability bug; couples the on-disk format to petgraph internals; no sealed privacy boundary |
| Event-sourced delta-replay (`GraphDelta` log, Candidate E) | Rejected | Already rejected by ADR-061 in favour of a full snapshot — replay reintroduces the staleness/torn-state hazards a self-correcting snapshot avoids |

## Consequences

- **Positive:** sub-phase B is unblocked with a minimal, dependency-honest store;
  a restarted daemon re-warms from a snapshot instead of a full cold rebuild;
  GV2-022 can freeze its `format_version`/`backing_schema_version` fields; the
  "discard-and-rebuild" rule means no snapshot-migration code is ever written;
  the privacy gate is a compile-time + unit-tested property, not a convention.
- **Negative:** persistence adds an at-rest artefact to secure (mitigated by
  owner-only perms, machine-local location, never-in-repo, sealed structural-identity
  payload, default-off); each snapshot is a whole-file rewrite (bounded by the §4
  coalescing cadence); a restore reconcile re-hashes file content (cheaper than
  re-parse, but not free).
- **Risks:** (1) a future field on the live graph types reaching the snapshot and
  smuggling source text — mitigated by the **sealed allowlist-only DTO** (§8: a
  non-allowlisted field cannot compile in) + the relative-path no-leak test;
  (2) snapshot write amplification under rapid saves — mitigated by the §4 coalescing
  window + RLB SLO benches; (3) a schema change loading valid-but-wrong data — closed
  by the §1/§6 decision: a **golden round-trip fixture** fails CI on any wire change
  (forcing a `schema_epoch` bump) and load-time **counts + checksum** reject a
  corrupted/truncated body, so neither silent drift nor silent corruption is loadable;
  (4) cleartext names/specifiers embedding secret-shaped values — accepted residual
  under the same-uid, default-off, owner-only boundary (§8).
- **Mitigations:** default-off + graduation criteria (§7) buy soak time; the verdict
  gate (§3) means a stale/wrong snapshot can only delay `clean`, never produce a false
  `Certified`; the load path treats *every* anomaly as cold-rebuild, so the worst case
  is always "slower start," never a wrong verdict or a crash; orphaned snapshots are
  GC'd on start (§10).

## References

- Related ADRs: [ADR-061](061-save-time-daemon-delta-validation.md) (save-time
  daemon delta validation — §9 specs the persistence requirements and assigns
  this ADR), [ADR-063](063-gv2-hot-path-boundary.md) (`persist` is background-only;
  hot path does no I/O), [ADR-064](064-intercept-graph-cache-crate-boundary.md)
  (`anvil-graph-cache` home + lean-daemon dep budget),
  [ADR-060](060-anvil-home-install-root-override.md) (`ANVIL_HOME` / state-dir
  resolution), [ADR-031](031-validation-latency-rubric.md) (latency budget the
  re-warm must respect),
  [ADR-035](035-three-pipe-observability-rule.md) (Notification pipe used for
  opted-in write-failure alerts, §10)
- Spec: [`plans/specs/2026-06-01-daemon-save-time-validation-contract.md`](../specs/2026-06-01-daemon-save-time-validation-contract.md)
  §9 (Backing & persistence)
- APS modules: GV2-021 (this ADR's owner), GV2-010/011/020/022 (the warm indexes
  serialized), DSV sub-phase B (consumer)
  ([`plans/modules/graph-v2-foundation.aps.md`](../modules/graph-v2-foundation.aps.md),
  [`plans/modules/daemon-save-time-validation.aps.md`](../modules/daemon-save-time-validation.aps.md))
- Design-council provenance (the §1 serialization verdict): Enterprise Integration
  Patterns — [Canonical Data Model](https://www.enterpriseintegrationpatterns.com/patterns/messaging/CanonicalDataModel.html),
  [Message Translator](https://www.enterpriseintegrationpatterns.com/patterns/messaging/MessageTranslator.html),
  [Format Indicator](https://www.enterpriseintegrationpatterns.com/patterns/messaging/FormatIndicator.html),
  [Idempotent Receiver](https://www.enterpriseintegrationpatterns.com/patterns/messaging/IdempotentReceiver.html);
  code grounding `crates/anvil-graph-cache/src/{symbol_graph,dependency,incremental}.rs`,
  `crates/anvil-kernel-types/src/graph.rs`
