# Daemon save-time validation — contract spec

Companion to [ADR-061](../decisions/061-save-time-daemon-delta-validation.md)
(Accepted 2026-06-01 via Planning Council `plan-5768ae0c`). The ADR records the
decision and consequences; this spec freezes the **field-level contract** and
records the implementation notes the ADR deliberately left out of its freeze
surface.

This spec is the authority for "is this wire addition legal": the wire commits
to a verdict + assurance vocabulary, never to a graph representation. The only
backing-derived value crossing the wire is an opaque monotonic `generation`. Any
field that would require a consumer to model graph structure is forbidden by
construction.

## 1. Methods

All four methods ride the existing intercept transport (Unix socket
`…/anvil/intercept.sock`, 0700 dir / 0600 sock; Windows named pipe), framing
(JSON-RPC 2.0 + serde-tagged NDJSON), and the existing handshake + capability
lattice. A caller reaches the `Attached` floor and presents its workspace
root(s) exactly as `scan_buffer` clients do today. `scan_buffer` is unchanged.

### `anvil/validate_paths`

The single save-time verb — the verdict-shaped generalisation of the existing
buffer path and of MLP2-067's `kernel.evaluate`.

```
request {
  workspace_root: string,            // root-relative paths below resolve under this; §4 auth
  paths: [ ChangeDescriptor ],       // daemon re-classifies; see §3
}
response {
  diagnostics: <scan_buffer envelope, verbatim>,   // rule-id/path/line-col/severity/catalog-message;
                                                    // NO raw source spans/snippets by default
  evaluated: [ { path: string, content_hash: string } ],  // the state actually evaluated (post-coalesce)
  workspace_assurance: WorkspaceAssurance,
  coverage: "certified" | "partial"  // pure function of state: certified iff state == clean
}
```

`ChangeDescriptor`:

```
{
  path: string,                      // workspace_root-relative, forward-slash normalised
  change: "created" | "modified" | "deleted" | "renamed",
  from?: string,                     // required iff change == "renamed"; root-relative, slash-normalised, auth-checked
  content_hash?: string,             // advisory hint only; daemon never trusts it for a verdict
  mtime?: integer                    // advisory hint only
}
```

Notes:

- `content_hash`/`mtime` are advisory cache hints. The daemon MUST derive every
  verdict from bytes it reads itself under the §4 dirfd. A client that omits them
  is fully supported (a cold MCP client will).
- `evaluated[]` echoes the exact `content_hash` the diagnostics were computed
  against — which, under latest-state coalescing (§5), may be a *later* save than
  the request named. Clients compare against their current buffer hash; a
  mismatch means "superseded, ignore or re-request". This is the field that makes
  coalescing safe across a frozen wire.
- There is **no** `mode`/`force_full`. A full scan is `request_full_scan` only.

### `anvil/workspace_status`

```
request  { workspace_root: string }
response {
  state: "clean" | "stale" | "pending" | "running" | "unavailable",
  reason?: StaleReason,              // non-optional when state == stale
  scan_started_at?: timestamp,       // present when state == running
  last_full_scan?: timestamp,
  generation: u64                    // opaque; equality ⇒ same assurance basis
}
```

### `anvil/request_full_scan`

```
request  { workspace_root: string, priority: "interactive" | "background" }
response { job_id: string, state: "pending" | "running" }
```

### `WorkspaceAssurance` (embedded in `validate_paths`)

```
{
  state: "clean" | "stale" | "pending" | "running" | "unavailable",
  reason?: StaleReason,
  generation: u64,
  last_full_scan?: timestamp
}
```

- `generation` is an **opaque monotonic token**, not a graph identity. Equality
  means "same assurance basis"; it bumps whenever the certifying basis changes
  for any reason (delta applied, full scan completed, eviction, warm-start). It
  carries zero structural promise and survives the SymbolGraph → GV2 backing swap
  because it names nothing about the graph. There is no `graph_version` on the
  wire.
- `unavailable` is the daemon-absent fallback state (§6). It is never `clean`.

## 2. `StaleReason` — the frozen invalidation taxonomy

Default-deny by change-class: any class not provably certifiable from resident
hot indexes maps to `stale`. The enum is frozen as part of the contract and is a
direct projection of the §3 classification:

| `StaleReason` | Triggered by |
|---|---|
| `cross-file-resolution-needed` | a create/modify whose new edges need resolution beyond resident hot indexes |
| `deleted` | a `Delete` whose `dependents_of()` importers may now dangle/violate |
| `renamed` | a `Rename` (decomposed to delete+create; old-path importers affected) |
| `symlink-retarget` | content shifted behind a stable path |
| `config-boundary-policy-edit` | `.anvil.yaml`, boundary config, or policy edit |
| `gitignore-scope-change` | a `.gitignore` mutation altering the scanned corpus |
| `impact-set-overflow` | the §3 reverse-impact closure exceeded budget |
| `warm-state-evicted` | the certifying indexes for this workspace were evicted |
| `scan-timeout` | a background scan exceeded its timeout (§6) |
| `daemon-absent` | fallback path; no daemon to certify |
| `unknown-class` | an FS-event class not yet modelled — explicit default-deny fallthrough |

Adding a new modelled class adds a variant; it never silently maps to `clean`.

## 3. Change classification & certifiability

### Classification (the per-path identity table)

The daemon keeps a per-path `(inode, mtime, size)` table and maps raw `notify`
events to a canonical class — never trusting the OS event type:

- **ContentModify** — same path, same inode, new mtime.
- **Atomic-save** — same path, **new inode** (editor temp+rename). Classified
  `ContentModify`, not `Rename` (so it neither forces a full scan nor
  mis-certifies). The inode flip is recorded.
- **Rename(Q→P)** — old path Q gone, new path P present → decomposed to
  `Delete(Q)` + `Create(P)`.
- **Delete(Q)** — path gone.
- **Case-only rename** — handled via a startup case-sensitivity probe +
  case-normalised graph keys.
- **Hardlink / no-event drift** — `validate_paths` does a `stat` per claimed path
  and compares to the table; silent drift is caught here.

Inode-based classification is **mandatory** in the gated correctness bar.

### Certifiability (bounded reverse-impact closure)

For each canonical change to file F, using the existing
`DependencyGraph.reverse` index (`dependents_of(F)`, O(1)) and the `update_file`
`GraphDelta`:

1. `ContentModify` with **no export-surface change** → validate F only →
   `certified`.
2. Export-surface change, `Delete`, or `Rename` → validate F **plus
   `dependents_of(F)`** (1-hop importers) inline; re-exports recurse, bounded by
   budget; stay `certified` if clean.
3. Closure exceeds budget → `coverage: partial` →
   `stale(reason: impact-set-overflow)` → background full scan reconciles.

No parse / resolve / transitive traversal on the hot path. This is strictly more
precise than "any file with importers is never certified".

## 4. Authorisation & read-safety

- **Trust boundary:** SO_PEERCRED uid == daemon uid. No intra-uid cross-workspace
  boundary exists or is claimed.
- **Authority:** reuse the existing handshake + `auth.rs`
  `validate_workspace_roots`. A connection carries a **growable set** of roots;
  additions auto-granted within the uid in `open` mode. No `/proc/<pid>/cwd`
  check.
- **Read-safety:** `openat2(RESOLVE_NO_SYMLINKS | RESOLVE_BENEATH)` against an
  `O_PATH` dirfd opened once per workspace; all per-path reads relative to it;
  lstat-ladder fallback. `path` and `from` are root-relative, slash-normalised,
  ownership-checked; escapes rejected, not scoped. The daemon-absent scoped
  fallback inherits the identical guard.
- **Windows:** `GetNamedPipeClientProcessId` → token → SID compare as the
  uid-equivalent. Until built: owner-only pipe ACL is the boundary; a `None`
  peer-pid MUST NOT be folded into `Spoofed` for these verbs. (OQ: prerequisite
  for named-pipe `validate_paths` GA.)

### Confinement mode

```
[workspace]
admission = "open" | "allowlist"     # operator-level config only (ANVIL_HOME/XDG, owner-only)
allow = [ "/abs/path", "/abs/prefix/*" ]   # exact + prefix entries
```

- `open` (default): first-touch auto-adopt within uid.
- `allowlist`: non-admitted roots refused with structured `workspace-not-admitted`;
  first-touch auto-adopt OFF; the primary check-in root is implicitly admitted,
  the allowlist governs additional roots.
- Managed via `anvil workspace allow|deny|list|mode`.
- Config load failure **fails closed and loud** (never silent fall-back to
  `open`).
- Policy guardrail, not an OS jail: governs Anvil-mediated paths (validation +
  enforcement-participating writes) only; cannot stop raw shell ops; hard
  boundary = separate OS user.

## 5. Coalescing (latest-state)

- Collapse only **true no-op duplicates** (identical `content_hash`).
- When distinct-hash saves coalesce, the response's `evaluated[]` reports the
  state actually validated, so no client renders a verdict against the wrong
  buffer.
- Save-time does **not** promise to evaluate every transient intermediate state;
  a violation written and reverted inside the debounce window is not separately
  reported. Audit anchors are the explicit/background full scan and commit-time
  enforcement.

## 6. Assurance lifecycle, observability, fallback

- Lifecycle: `clean → stale` on an uncertifiable delta; `stale → pending →
  running → clean` via the background scheduler.
- `reason` is non-optional for `stale`; `scan_started_at` present for `running`;
  daemon emits a structured INFO log on every transition.
- A background scan over a configurable timeout → `stale(reason: scan-timeout)`;
  on daemon restart, any `running` workspace becomes `stale`.
- Fallback (daemon absent): `workspace_assurance{state: unavailable, reason:
  daemon-absent}`, never `clean`; WARN on first fallback; `anvil status` reports
  "unprotected (daemon not running)", not a stale cached state.
- `clean` is documented as a same-uid liveness signal, not a tamper-proof
  attestation.

## 7. Gated correctness bar (Phase-2 hard blockers)

1. Exhaustive invalidation taxonomy (§2) incl. mandatory inode classification.
2. Cross-path diagnostic parity — golden corpus, identical finding sets across
   watch+daemon, watch+fallback, MCP+daemon, MCP+fallback, **order-normalised**
   by `(path, rule_id, span_start)`; shared config discovery off `workspace_root`;
   `workspace_assurance` carved out of parity.
3. `workspace_root` authorisation + read-safety (§4).

## 8. Resource model & SLO

- One per-host budget split into **two cooperating rayon pools**: a small
  interactive pool (validate_paths only) + a background pool; background full
  scans are chunked and check a cooperative cancel/yield flag between chunks.
- Per-workspace DoS caps: max parse file size; per-`WorktreeKey` in-flight-work
  admission token over the existing per-connection semaphore; directory-walk
  depth cap. (Symlink cycles already neutralised by §4.)
- Fallback processes cap their per-process pool at `cores/4`; RLB-002 benchmarks
  the daemon-absent ramp separately. True cross-process host-wide capping
  (cgroups) is a deferred OQ.
- **Concurrency SLO:** 4 agents + 1 active background scan keep interactive
  `validate_paths` p95 within the ADR-031 budget; WARN when an interactive
  request waits >80 ms pre-service; RLB-008 wires this as a CI gate.

## 9. Backing & persistence

- Backing starts as the per-`WorktreeKey` `SymbolGraph` cache (bounded LRU +
  generation-guard + unregister-hook; deltas via `anvil_kernel::graph::
  incremental`) — MLP2-067 folded in as sub-phase A. The GV2 hot-read slice
  (GV2-010/011/020/022) swaps in under the unchanged wire (sub-phase A′), blocked
  on the GV2 hot-/non-hot-path boundary gate.
- **Persistence (sub-phase B):** `ANVIL_PERSIST_GRAPH` defaults **off** in v1.
  Snapshots live under `ANVIL_HOME`/XDG state dir (reusing `anvil_home_prefix()`
  + the socket-dir resolver), owner-only 0700/0600, never in the repo tree, with
  a gitignore entry as belt-and-suspenders.
- **Warm-start restores indexes, never the verdict.** A restored workspace comes
  up `stale`/`pending`; a fast reconcile (cheap, indexes warm) re-establishes
  `clean`. This removes the snapshot-staleness race by construction — no
  `delta_sequence`/watermark needed.
- **Privacy line:** persist structural identity (symbol names, import/path
  identity, edges, content hashes) needed for boundary checks; **never** persist
  raw source bodies, snippets, comment text, or secret-shaped literals. Protected
  at rest by perms + machine-local location. The privacy gate is a concrete
  allow/deny checklist owned by GV2-021.
- **Crash-safety:** atomic write (tmp + fsync + rename); a corrupt / unreadable /
  version-mismatched snapshot → cold rebuild with a loud log, never refuse to
  start, never panic; staggered re-warm (concurrency cap ~2); snapshot embeds
  `format_version` and `backing_schema_version` (`interim-symbolgraph-v1` vs
  `gv2-hotindex-v1`); a backing swap bumps `backing_schema_version`, invalidating
  pre-swap snapshots (one cold rebuild).

## Appendix — implementation notes (NOT frozen by ADR-061)

These are current intent, tunable without an ADR revision (per the "demote impl
detail" council resolution):

- Interactive pool size (≈2–4 threads), background pool size, LRU capacity
  defaults, snapshot cadence, coalescing window, reverse-impact closure budget
  (≈64 files / a few re-export hops), background-scan timeout default (≈5 min),
  staggered re-warm concurrency (≈2).
- Numbers are validated by RLB-001/002/008 benchmarks against the §8 SLO, not
  frozen in the contract.

## Open questions

- Windows peer-SID check (prerequisite for named-pipe `validate_paths` GA).
- Cross-process host-wide CPU capping for daemon-absent fallback (cgroups?).
- Whether the reverse-impact closure budget should be adaptive to host size.
