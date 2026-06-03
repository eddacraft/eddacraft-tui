# ADR-061: Save-time governance is daemon-mediated delta validation

## Status

Accepted

Accepted 2026-06-01 via Planning Council `plan-5768ae0c` (architect,
pragmatic-lead, adversarial-reviewer, security-analyst, operations-reviewer).
Proposed 2026-05-31. The council ran a five-persona review of the locked
contract shape; all five returned objections that are resolved in the Decision
below. The full field-level contract is specified in
[`plans/specs/2026-06-01-daemon-save-time-validation-contract.md`](../specs/2026-06-01-daemon-save-time-validation-contract.md);
this ADR records the decision, the wire-freeze commitment, and the consequences.

**Acceptance conditions (must hold before the gated work they name):**

- This ADR locks the *save-time product contract and the wire shape*. It does
  **not** by itself close the GV2 `graph-v2-foundation` Ready gate
  ("hot-/non-hot-path boundary agreed with INTD and DRVR owners"). Phase 3 (the
  GV2 hot-read backing) MUST NOT freeze any hot-read API field until that gate
  is closed by a separate INTD + DRVR + GV2 contract review.
- The dependent **Proposed** ADRs — ADR-015 (intercept-loop enforcement),
  ADR-030 (surface drivers), ADR-031 (validation-latency rubric) — are ratified
  in parallel; they are referenced constraints, not blockers for sub-phase A
  implementation, which proceeds behind the existing daemon-absent fallback.

## Date

2026-05-31 (Proposed) · 2026-06-01 (Accepted)

## Context

Anvil's product claim is real-time governance: as an agent edits a repo, Anvil
validates the change at the point of change. Today `anvil watch` delivers that
in the most expensive way possible. On every debounced save it builds and spawns
a **cold `anvil check --all` child process**
(`crates/anvil-cli/src/commands/watch.rs:377` default action →
`:433` `build_action_command` → `:442` `Command::new` → `:463` `--all`). That
child cold-rebuilds the whole-repo graph and rayon-scans every file, then exits —
discarding all warm state.

A beta tester reported Anvil being CPU-expensive on their machine. A load probe
(`benchmarks/prototypes/anvil-load-probe.py`, measuring process-tree CPU under
sustained file churn across a ramp of concurrent agents) reproduced it: a single
agent's watch consumes ~7 cores during save storms, and ~2 concurrent agents
saturate a 16-core box. An `--action none` control measured 0.03 cores — proving
the cost is the per-save reaction (the whole-repo scan), not the watcher, the
file events, or idle daemon overhead. The existing `watch_resource_budget`
benchmark reports ~0% CPU because it measures the idle path (no file events, parent
pid only, static corpus) — it cannot see this failure mode.

Two structural multipliers make it worse:

- **No warm state is reused.** The kernel maintains incremental
  `SymbolGraph`/`DependencyGraph` state, but it lives in the watch process and is
  thrown away; `check --all` rebuilds cold each save.
- **Rayon is capped per-process, not per-host**
  (`crates/anvil-rayon-init/src/lib.rs:74`, `(cores/2).max(1)`), so N Anvil
  processes oversubscribe the machine.

Crucially, the daemon needed to fix this **already exists**. `anvil intercept`
is a permissioned, concurrent, warm-state daemon
(`crates/anvil-intercept/src/lib.rs:964` `run_foreground`,
`crates/anvil-intercept/src/ipc.rs:780` serve loop with semaphore-gated
admission at `:831`) that already validates in-flight buffers via the
`anvil/scan_buffer` JSON-RPC method
(`crates/anvil-intercept-proto/src/protocol.rs:83`, dispatch
`crates/anvil-intercept/src/ipc.rs:2220` `handle_scan_buffer_jsonrpc`). And the
check logic is already a
scoped, in-process library call —
`run_antipattern_check(files, config, workspace_root)`
(`crates/anvil-checks/src/antipattern/check.rs:95`) takes an arbitrary file list
and returns diagnostics synchronously, no subprocess required. The gap is that
`watch` (and `anvil mcp serve`) do not use any of it.

The tactical relief — scoping the per-save check to changed paths instead of
`--all` — **already shipped** as RLB-007 (Merged via PR #2184, 2026-05-31),
proving ~6.55 → ~0.08 cores on the changed-path path. This ADR is therefore not
the bug fix; it is the architecture that turns that scoping into a coordinated,
warm, multi-agent-safe save-time path, and locks the product contract so the
daemon, watch, MCP, and Graph V2 work (modules INTD, DRVR, RLB, GV2) sequence
behind one decision rather than each inventing a save-time model.

## Decision

**Save-time governance is delta validation mediated by the intercept daemon over
warm graph state. A whole-repo scan is never the default reaction to a single
save.** The daemon is scoped per ADR-036 — one daemon per `(uid, os)` execution
scope, serving every workspace under it; "warm model", work budget, and
assurance state below are **per-workspace state held inside that one daemon**,
not one daemon per repo.

The numbered decisions below incorporate the Planning Council resolutions. Field-
level detail (request/response shapes, enums, the invalidation taxonomy table)
lives in the contract spec; this section records the binding decisions.

### 1. The per-save hot path drops `check --all`

The default save-time action in `watch.rs` stops spawning a cold whole-repo
child. Whole-repo scans become explicit (`anvil check`), background, or
CI-driven — never the per-save reaction.

### 2. The intercept daemon is the save-time validation authority — one verb, verdict-shaped

The daemon gains three JSON-RPC methods alongside the existing
`anvil/scan_buffer`, on the same transport, framing, and diagnostic envelope:

- `anvil/validate_paths` — the single save-time verb, the generalisation of the
  existing buffer-validation path. It takes a set of changed on-disk paths (each
  carrying a daemon-classified change descriptor — see §5) and returns the
  existing diagnostic envelope plus `workspace_assurance` and a per-path
  `evaluated{path, content_hash}` echo.
- `anvil/workspace_status` — report per-workspace assurance state.
- `anvil/request_full_scan` — enqueue an explicit/background full scan. This is
  the **only** way to trigger a full scan; `validate_paths` has no `force_full`
  mode (it must never become the heavy work it is meant to outrank).

**The wire is verdict-shaped and frozen against backing change.** It commits to
a verdict + assurance vocabulary, never to a graph representation. The only
backing-derived value that crosses the wire is an **opaque monotonic
`generation`** token meaning "the assurance basis is unchanged since you last
asked"; there is no `graph_version` on the wire (it could not survive the
backing swap in §4). Any field that would require a consumer to model graph
structure is forbidden from this contract by construction. Diagnostics carry
rule id / path / line-col / severity / catalog message and **do not echo raw
source spans or snippets by default**, so the verdict-shaped claim is real and
the RPC is not a same-uid source read-oracle. Client-supplied `content_hash` /
`mtime` are advisory cache hints only; the daemon derives every verdict from
bytes it read itself (§7). `validate_paths` is the productionised form of
MLP2-067's narrow `kernel.evaluate`; MLP2-067 is folded into sub-phase A of this
work rather than shipped as a separate verb (see §9, Sequencing).

### 3. Watch, MCP, and intercept are thin clients of the one daemon

`anvil watch` sends classified changed paths to `validate_paths`;
`anvil mcp serve`'s `anvil_validate_write` re-points from its own in-process
scan to the daemon so a repo running watch + MCP keeps one warm model
(per-workspace) and never double-scans. MCP stays a *projection/client* of the
authority, consistent with the Graph V2 rule that MCP is not the control plane.

### 4. The daemon owns the warm backing and the work budget

The warm model starts as the existing per-`WorktreeKey` `SymbolGraph` cache
pattern (bounded LRU + generation-guard + unregister-hook, deltas applied via
`anvil_kernel::graph::incremental`) and is later replaced — **under the
unchanged wire** — by the GV2 hot-read slice (GV2-010 per-file extract + GV2-011
warm boundary/known-edge/dependency indexes + GV2-020 registry + GV2-022 bounded
hot-read API). Certifiability (§6) is a GV2-022 capability that reads **resident
hot indexes only** — no parse, resolve, or transitive traversal on the hot path.

**Resource model.** The one per-host budget is split into **two cooperating
rayon pools**: a small interactive pool (validate_paths only, never starved) and
a background pool; background full scans are chunked and check a cooperative
cancel/yield flag between chunks so they hand cores back under interactive load.
This replaces the "one pool, interactive preempts" framing, which rayon cannot
implement. Per-workspace DoS caps protect the shared process: a max parse file
size, a per-`WorktreeKey` in-flight-work admission token layered over the
existing per-connection semaphore, and a directory-walk depth cap. (Symlink
cycles are already neutralised by the §7 read-safety guard.) Specific pool
sizes, LRU capacities, snapshot cadence, and the coalescing window are
implementation detail recorded in the spec appendix, not frozen here.

### 5. Change classification and the invalidation taxonomy

The daemon maintains a per-path `(inode, mtime, size)` identity table and
classifies raw `notify` events into canonical change-classes — `ContentModify`,
`Create`, `Delete`, `Rename(from→to)` — rather than trusting the OS event type.
This makes **atomic-save editors** (vim/JetBrains write-temp-then-rename, which
arrive as `Modify(Name)` today) classify correctly: a same-path inode flip is
`ContentModify`, not a rename, so it neither triggers a needless full scan nor
gets mis-certified. Case-only renames on case-insensitive filesystems are
handled by a startup case-sensitivity probe + case-normalised keys; hardlink /
no-event drift is caught by a `stat`-on-validate comparison. Inode-based change
classification is **mandatory** in the gated correctness bar (§8).

The `Clean → Stale` invalidation taxonomy is **default-deny by change-class**:
any class not provably certifiable from resident hot indexes forces `Stale`. The
exhaustive `StaleReason` set is frozen as a wire-visible enum in the spec —
`cross-file-resolution-needed`, `deleted`, `renamed`, `symlink-retarget`,
`config-boundary-policy-edit`, `gitignore-scope-change`, `impact-set-overflow`,
`warm-state-evicted`, `scan-timeout`, `daemon-absent`, and an explicit
`unknown-class` fallthrough — projected directly from §6's classification. The
certifiable content-modify case is the one class that carries no reason and
stays `Clean`.

### 6. Certifiability is a bounded reverse-impact closure, not a forward-only check

`validate_paths` returns `coverage: certified | partial`, where `certified` iff
the workspace is `Clean`.

**Coverage is family-scoped (B2, council review 2026-06-01).** The response
carries `check_families: ["antipattern"]`, and `certified` attests **antipattern
cleanliness only** — not whole-repo structural-policy assurance. The four
structural policy checks (`CrossLayerViolation` / `NewDependencyIntroduction` /
`PublicApiExpansion` / `PrivilegeExpansion`) are **not run on the save-time
`validate_paths` hot path** — only `run_antipattern_check` is. Whole-repo
structural policy remains enforced by `anvil gate`; the embedded structural
pipeline (`run_embedded`) has no production caller today. Forcing those checks
onto the hot path would reintroduce the CPU regression this ADR exists to remove,
so we narrow the *claim* (label the family) rather than widen the *work*. A
correctly-labelled antipattern-only verdict is sound; an unlabelled one would be
a false attestation. The certifiable/partial decision (within the antipattern
family):

- A `ContentModify` with **no export-surface change** (read from the
  `update_file` `GraphDelta`) is self-contained → validate that file only →
  `certified`.
  **Conservative default (B4, council review 2026-06-01):** Sub-phase A has no
  stable-identity export-diff primitive. `update_file` removes a file's symbols
  and re-adds them (`incremental.rs:74-87`), so every save shows full churn, and
  `symbol_baseline_key = file::kind::name` (`incremental.rs:27-29`) conflates
  identity with position; GV2-002 (stable identity) is Draft. The export-surface
  decision is therefore made by the **`GraphDelta.previously_public` set-diff**,
  and any modify that touches a public/privileged symbol **defaults to
  `partial`/`Stale`** until a real export-diff helper lands — a body-only change
  with no public-symbol delta is the only modify that stays `certified` on the
  self-only path. This is conservatively safe (a rename reads as delete+add =
  surface change). The export fast-path graduates from conservative-partial only
  once GV2-002 stable identity exists; no dedicated `export_surface_changed()`
  helper is mandated for Sub-phase A. See
  `plans/reviews/2026-06-01-daemon-graph-council-verdict.md` (B4).
- An export-surface change, a `Delete`, or a `Rename` can make an **unchanged
  importer** illegal. The affected set is exactly `dependents_of(file)` — the
  1-hop importer set, read from the daemon's `DependencyGraph.reverse` index.
  **`GraphDelta.removed_edges` is always empty** (`update_file` emits
  `Vec::new()` at `incremental.rs:150`; `remove_file` via `..Default::default()`
  at `incremental.rs:291-298`), so importer discovery uses `dependents_of`
  **exclusively** — never `delta.removed_edges` (B4).
  **Correction (council review 2026-06-01):** this reverse index is **net-new**.
  The `DependencyGraph` type exists (`crates/anvil-kernel/src/graph/dependency.rs`)
  but no production path builds it today — `add_dependency`/`dependents_of` have
  zero non-test callers, and the daemon cache holds only `SymbolGraph`. Sub-phase A
  **must build and incrementally maintain** this index; it is not a free "existing
  / O(1)" read. See `plans/reviews/2026-06-01-daemon-graph-council-verdict.md` (B1).
  The daemon validates the file **plus that bounded reverse closure** inline
  (re-exports recurse, bounded by budget) and stays `certified` if clean.
- If the impact closure exceeds budget → `coverage: partial` →
  `Stale(reason: impact-set-overflow)` → background full scan reconciles.

This is deliberately more precise than "any file with importers is never
certified", which would force a full scan on almost every edit and discard the
CPU win. It closes the reverse-dependency soundness hole without transitive
traversal on the hot path.

### 7. Authorisation, read-safety, and confinement

- **Trust boundary:** SO_PEERCRED uid == daemon uid is the real and only trust
  boundary. Within one uid there is **no** cross-workspace boundary to enforce
  (the uid already has filesystem access to all its repos); the ADR states this
  plainly rather than implying a guarantee it cannot make.
- **Authority:** `validate_paths` / `workspace_status` / `request_full_scan` reuse
  the **SO_PEERCRED handshake/transport** that `scan_buffer` already requires, but
  wiring `auth.rs` `validate_workspace_roots` into authorisation is **net-new (B7),
  not "reuse"** — that API ships with unit tests yet has **no production caller today**
  (DRVR-001 Wave 2 left it unwired; zero call sites outside `auth.rs`). `validate_paths`
  is the first verb to wire it and the first to read arbitrary on-disk paths, so this
  authorisation path (with the read-safety guard below) is load-bearing. A connection
  carries a **growable set** of workspace roots; additions are auto-granted within the
  uid (default `open` mode — see the open-mode blast-radius note in the contract §4).
  The `/proc/<pid>/cwd` check from earlier drafts is **dropped entirely** — it is the
  wrong gate (it breaks editors/agents/MCP whose cwd is elsewhere) and adds no
  security within a uid.
- **Read-safety:** authorised reads use `openat2(RESOLVE_NO_SYMLINKS |
  RESOLVE_BENEATH)` against an `O_PATH` dirfd opened once per workspace, with all
  per-path reads relative to that fd (lstat-ladder fallback where `openat2` is
  unavailable). This closes the canonicalise-then-open TOCTOU/symlink class. The
  daemon-absent scoped fallback inherits the identical guard. Every `path` and
  `renamed.from` is workspace-root-relative, slash-normalised, and
  ownership-checked against the dirfd; escapes are rejected, not scoped.
- **Windows:** the uid-equivalent is `GetNamedPipeClientProcessId` →
  `OpenProcessToken` → `GetTokenInformation(TokenUser)` SID comparison. Until
  that lands, the owner-only pipe ACL is the boundary and a `None` peer-pid MUST
  NOT be folded into a `Spoofed` verdict for these verbs. (Open question:
  Windows peer-SID check is a prerequisite for named-pipe `validate_paths` GA.)
- **Confinement mode (opt-in):** an operator may set workspace admission to
  `allowlist` (default `open`). In `allowlist` mode the daemon refuses any
  non-admitted root with a structured `workspace-not-admitted` code and disables
  first-touch auto-adopt; the primary check-in root is implicitly admitted, the
  allowlist governs additional roots. The allowlist and mode live in
  **operator-level** config (under `ANVIL_HOME`/XDG, owner-only) — never in a
  repo's `.anvil.yaml`, so the agent being confined cannot grant itself access —
  and are managed via `anvil workspace allow|deny|list|mode`, supporting exact
  and prefix entries. Config load failure **fails closed and loud** (no silent
  fall-back to `open`), per the operator-config no-silent-defaults rule.
- **Placement (ops/security major, council review 2026-06-01).** The confinement
  config loader lives in **`anvil-intercept` (`confinement.rs`)** and resolves the
  operator-level config dir by **reusing the daemon's own `anvil_home_prefix()`**
  (`crates/anvil-intercept/src/lib.rs`) — the same `ANVIL_HOME`/XDG resolver the
  socket-dir resolver (`ipc.rs` `resolve_socket_dir`) already uses. The council's
  premise that "the `ANVIL_HOME` resolver lives in `anvil-cli`, so confinement is
  a wrong-direction dep" is **corrected by the code**: `anvil-cli` depends on
  `anvil-intercept` (not the reverse), and the daemon already resolves
  `ANVIL_HOME` itself (`anvil_home_prefix` deliberately mirrors the CLI's
  `install_root::resolve_install_root_from`). So **no cross-crate dependency and
  no new ADR are required** — confinement stays where the plan places it, reusing
  the existing daemon-side resolver. The allowlist must be read **only** through
  that operator-home resolver, **never** from a repo `.anvil.yaml` (the confined
  agent must not be able to grant itself access). See
  `plans/reviews/2026-06-01-daemon-graph-council-verdict.md` (§4, item 8).
  **This is a policy guardrail for well-behaved agent tooling, not an OS jail:**
  it constrains everything that goes through Anvil (validation and the
  enforcement-participating write gate), but cannot stop raw shell file
  operations, and within one uid a determined process could edit the config. The
  hard boundary remains running the agent under a separate OS user.

### 8. Gated correctness bar (before Phase 2 ships)

Three correctness gates are hard blockers for Phase 2 merge:

1. **Exhaustive invalidation taxonomy** (§5), including mandatory inode-based
   change classification.
2. **Cross-path diagnostic parity** — a golden-corpus test asserting identical
   **antipattern-family** finding sets (B2: scoped to
   `check_families: ["antipattern"]`, matching the `coverage: certified` claim)
   across the antipattern surfaces, with shared config discovery off
   `workspace_root`. Because warm (incremental) and cold (full-walk) paths order
   findings differently, parity is defined as **order-normalised** by
   `(path, rule_id, span_start)` — both paths sort before constructing the
   envelope; byte-identical raw output is not a goal. `workspace_assurance` is
   explicitly carved out of parity (it is structurally incomparable between
   daemon and fallback).
   **Surface scope (reconciled 2026-06-04, DSV-009):** the antipattern family runs
   only on the save-time `validate_paths` surfaces — **`watch+daemon` and
   `watch+fallback`** (`anvil check`); the gate
   (`crates/anvil-intercept/tests/diagnostic_parity.rs`) covers those two. MCP
   `anvil_validate_write` deliberately stays on the `scan_buffer` verb (secret +
   launch-reasoning / boundary family, `default_rule_registry()`) per DSV-007, so it
   has no antipattern findings; its `daemon`↔`embedded` parity is gated separately on
   that family. The earlier "across … MCP+daemon, MCP+fallback" antipattern framing
   pre-dated that DSV-007 decision and is corrected here (see contract §7.2).
3. **`workspace_root` authorisation + read-safety** (§7).

### 9. Assurance lifecycle, observability, and fallback

**Initial state (B6, council review 2026-06-01):** a freshly connected,
reconnected, or cold-cache-key workspace begins at
`Stale(reason: cross-file-resolution-needed)` — **never `Clean`** (nothing has
been certified yet). The `watch` client auto-issues `request_full_scan` on
connect and reconnect, so a workspace reaches `Clean` without operator action; in
Sub-phase A that connect-time scan is the only path out of the initial `Stale`
(the standing background scheduler is Sub-phase B). Without a defined initial
state, "`certified` iff `Clean`" would make `validate_paths` return `partial` on
every call until a client manually scanned.

Full lifecycle:
`(connect) → Stale(cross-file-resolution-needed) → Pending → Running → Clean`;
thereafter `Clean → Stale` on an uncertifiable delta (§5/§6) and
`Stale → Pending → Running → Clean` via the background scheduler.
`workspace_status` carries a **non-optional `reason`** for `Stale`, a
`scan_started_at` for `Running`.

**Observability of transitions (ops major, council review 2026-06-01).** Every
assurance state change is surfaced via the **ADR-035 Notification envelope** (the
user-visible-state-change pipe), not a bare INFO line — the earlier "structured
INFO log" wording named no fields and was unmonitorable. The daemon emits a
`NotificationEnvelope` (`telemetry.rs`) routed through the existing `Fanout::route`
(subject to the same cross-session `redact_envelope` guard), reusing the existing
**`FenceState`-transition envelope shape** (`envelope_for_fence_transition` is the
precedent): `notification.class = FenceState` (assurance is a workspace
fence/protection state), `priority` (`normal` for `→clean`/`→running`, `high` for
`→stale`/`→unavailable`), `grouping.transition = {from, to}` (the assurance state
names), `grouping.key = "intercept:assurance:<workspace_root>"`, and
`notification.{title,message}` naming the reason in human text (e.g. message
`"stale: cross-file-resolution-needed"`). The **precise machine-readable fields**
— `reason` (`StaleReason`), the opaque `generation`, and `scan_started_at` — are
emitted as named fields on a **mirrored `tracing` structured event** (free-form,
no wire-schema constraint), so transitions stay greppable in daemon logs with no
dashboard subscriber attached. The current `NotificationContext` is `{file,
source}` only; carrying `reason`/`generation`/`scan_started_at` as *machine-
readable envelope* fields (beyond the message text) would be a `NotificationContext`
schema extension in `anvil-kernel-types` **plus** a matching `redact_envelope`
update (so the new fields cross the cross-session boundary safely) — recorded as a
Task 9 prerequisite, not assumed to exist. A background scan exceeding a
configurable timeout transitions to `Stale(reason: scan-timeout)`; on daemon
restart, any `Running` workspace becomes `Stale` (the in-flight scan did not
complete). When the daemon is absent, the fallback returns
`workspace_assurance{state: unavailable, reason: daemon-absent}` (never `Clean`),
logs a WARN on first fallback, and `anvil status` reports "unprotected (daemon
not running)" rather than a stale cached state. `Clean` is documented as a
same-uid liveness signal, **not** a tamper-proof integrity attestation.

A **concurrency SLO** gates the design: 4 agents + 1 active background scan must
keep interactive `validate_paths` p95 within the ADR-031 budget; a WARN logs
when an interactive request waits >80 ms before service; RLB-008 wires this as a
CI gate.

### Sequencing

The work ships across the `v0.8.0-beta` minor in sub-phases; the wire shape is
byte-identical across sub-phases A and B.

- **Phase 1 — shipped (RLB-007, PR #2184).** Scope the per-save check to changed
  paths. Immediate CPU relief.
- **Sub-phase A:** the frozen `validate_paths` wire + watch client + MCP
  re-point, backed by the **interim SymbolGraph cache** (MLP2-067 folded in),
  **rebuild-on-restart** (no persistence). Gated by §8. (INTD + RLB-001..005 +
  DRVR.)
- **Sub-phase A′:** warm Graph V2 hot-read slice swaps under the wire
  (GV2-010/011/020/022). Blocked on the GV2 hot-/non-hot-path boundary gate.
- **Sub-phase B:** workspace-assurance background scheduler + GV2-021
  persistence/warm-start. Persistence defaults **off** (`ANVIL_PERSIST_GRAPH`
  opt-in in v1) and **restores warm indexes, never the verdict** — a restored
  workspace comes up `Stale`/`pending` and a fast reconcile re-establishes
  `Clean`, which eliminates the snapshot-staleness race by construction.
- Phase budgets gated by the process-tree CPU bench (RLB-001/008) + the
  `validate_paths` warm-read latency case on `ipc_roundtrip` (ADR-031).

## Rationale

The save-time path was doing whole-repo, cold, uncoordinated work per save. The
fix is not "make `check --all` faster" — it is "stop doing it on every save",
then coordinate the cheap delta path through the one warm, permissioned process
that already exists. Because the daemon, the warm graph maintainer (the
`SymbolGraph` cache — the **reverse-dependency `DependencyGraph` index is net-new**,
built and maintained by sub-phase A; B1), and a scoped in-process check library
already exist, the architectural work is wiring, a classification layer, the
net-new reverse index, and a warm-state slice
— not new runtime machinery. Holding the wire verdict-shaped (the generalisation
of `scan_buffer`/`kernel.evaluate`) lets the SymbolGraph backing ship first and
the GV2 slice swap in later without breaking a single consumer.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Daemon-mediated delta validation (chosen)** | Reuses the existing daemon, protocol, `SymbolGraph` maintainer, and in-process check library (the reverse-dependency `DependencyGraph` index is net-new — B1); one warm model and one work budget across watch/MCP/intercept; fixes per-process rayon oversubscription; verdict-shaped wire frozen against the GV2 backing swap | Requires warm graph state in the daemon, the net-new reverse index, and re-pointing MCP; assurance + confinement are new surface |
| **Only scope the per-save check to changed paths** | Tiny, no daemon/GV2 dependency; large immediate CPU drop | Still cold-spawns per save; no shared warm state; doesn't fix cross-process oversubscription or multi-agent coordination. Shipped as Phase 1 (RLB-007), not the end state |
| **Two graph-cache designs (MLP2-067 `kernel.evaluate` + ADR-061 hot-read)** | Each independently scoped | Two competing daemon graph-cache contracts that drift. Resolved by folding MLP2-067 into sub-phase A as the interim backing under the one `validate_paths` wire |
| **One shared rayon pool, "interactive preempts background"** | Simplest to describe | Rayon has no preemption; not implementable. Replaced by two cooperating pools + chunked-yield background scans |
| **Persist-and-trust `Clean` across restart** | Warm-start is "instant clean" | Snapshot-staleness race (delta after snapshot, then crash → false Clean). Replaced by "restore indexes, re-derive verdict" |
| **`/proc/<pid>/cwd` authorisation** | Intuitive "you must be in the repo" | Wrong gate: breaks editors/agents/MCP whose cwd differs; adds no security within a uid; not implemented. Dropped for handshake + manifest adoption |
| **New standalone validation daemon** | Clean-slate design | Re-implements transport, framing, concurrency, and warm state the intercept daemon already ships; two daemons to run |

## Consequences

- **Positive:** Per-save CPU drops from whole-repo to proportional-to-change;
  concurrent agents share one warm model and a bounded budget; one diagnostic
  envelope across watch/MCP/intercept; per-host rayon (within the daemon) removes
  oversubscription; the reverse-dependency closure catches importer violations
  without transitive hot-path work; the verdict-shaped wire freezes once and
  survives the GV2 backing swap; an opt-in confinement mode lets operators box
  roaming agents.
- **Negative:** The daemon gains warm graph state, change-classification state,
  and a new method surface; MCP's validation path changes; workspace-assurance
  and confinement are new concepts to surface in CLI/TUI; full-repo assurance is
  no longer implicit on every save and must be scheduled/requested.
- **Risks & honest limits:** (a) the daemon's per-host budgeting only holds for
  work routed through it — daemon-absent fallback processes retain their own
  per-process pools (capped lower at `cores/4`) and will oversubscribe during
  mixed-rollout states; true cross-process host-wide capping (cgroups) is a
  deferred open question. (b) Latest-state coalescing means a violation written
  and reverted inside the debounce window is not separately reported; save-time
  is "validate the state at the point of change", and the audit anchors are the
  explicit/background full scan and commit-time enforcement, not a per-save log.
  (c) Confinement is a policy guardrail, not an OS jail (see §7). (d) `Clean` is
  a same-uid liveness signal, not a cross-trust attestation.
- **Mitigations:** the process-tree CPU bench + the `ipc_roundtrip` warm-read
  latency budget (ADR-031) keep the hot path honest and fail on regression; the
  §8 gated correctness bar (taxonomy + parity + auth) blocks Phase 2 on
  soundness; mandatory daemon-absent fallback (scoped, never `--all`, exit 0);
  default-deny invalidation taxonomy with inode-based classification; fail-closed
  confinement config loading; persistence default-off with restore-indexes-not-
  verdict semantics.

## References

- Planning Council: `plan-5768ae0c` (2026-06-01); input artifact
  `ANVIL-DAEMON-DESIGN-RESPONSE.md` (PR #2188)
- Contract spec:
  [`plans/specs/2026-06-01-daemon-save-time-validation-contract.md`](../specs/2026-06-01-daemon-save-time-validation-contract.md)
- Related ADRs: ADR-015 (intercept-loop enforcement), ADR-031 (validation
  latency rubric), ADR-036 (daemon scope and boundaries), ADR-030 (surface
  drivers on the daemon), ADR-001 (planless-first), ADR-002 (warnings over
  blocks)
- APS modules: RLB-001/002/005/007/008 (resource-load-benchmarking),
  GV2-010/011/020/021/022 (graph-v2-foundation), MLP2-067 (folded into sub-phase
  A), INTD (intercept daemon), DRVR (surface-drivers)
- Evidence: process-tree load probe `benchmarks/prototypes/anvil-load-probe.py`
  (in-repo); CPU field report and tester-diagnostics tracking in issue #2156
