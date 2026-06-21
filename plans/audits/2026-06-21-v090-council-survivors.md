# v0.9.0-beta Release Council — Surviving Fix Items

> **Source:** council-per-surface review of `git diff v0.8.1-beta..HEAD` (the
> `v0.9.0-beta` "Assistant-Facing Graph" surface), 2026-06-21. Five surfaces ×
> four specialist lenses (security / kernel-correctness / adversarial /
> operations), per-surface synthesis, adversarial verification of every
> must-fix (refute-by-default), then a converged release gate.
>
> **Converged gate: BLOCK.** Two must-fixes were refuted in verification and
> dropped (the `gctx.egress` manifest flag — a documented Phase-2 deferral; and
> a severity-corrected item). This document captures only the items that
> **survived the skeptic** plus the council-recommended should-fixes, grouped
> into the CIB clusters that track their remediation (CIB-091..095).

## Cut-criteria carry-over (verify at tag time, not code work)

- ADR-031 latency gate (GV2-025) — **at-risk**: two GV2 mediums run in
  `apply_delta` outside the gated hot-read ops. Confirm green on current `main`.
- Full `Cross` matrix incl. Windows — **unknown**: PR-green Test is ubuntu-only.
  Run `gh workflow run rust.yml --ref <branch>`.
- `release-readiness.yml` on the source SHA — **unknown**: must be run + green.
- `ACKNOWLEDGEMENTS` fresh — **at-risk**: GCALL/GCTX/USAGE/DSV crates landed
  since `v0.8.1-beta`; regen if any new Rust/transitive dep.

---

## CIB-091 — GCTX egress hardening (cut-blocker)

Crate(s): `anvil-gctx-egress`, `anvil-cli/src/mcp`, `anvil-intercept/src/save_time.rs`.

- [x] **091a CE-3 sensitive-path egress deny-list** — _high, CONFIRMED, cut-blocker._
      `crates/anvil-gctx-egress/src/lib.rs`. The substrate scans with
      `standard_filters(false)`, so `secrets/.env.production` etc. are in the
      graph; their identity-only paths are emitted in `graph://symbols` file
      fields, `graph://edges` from/to.file, `anvil_find_dependents`,
      `anvil_impact_of_change`. **Fix:** add `is_sensitive_egress_path`
      (`.env*`, `*.pem`, `*.key`, `id_rsa*`, `*.p12`, `.git/`, `secrets/`,
      `.aws/`, `.ssh/`, `.gnupg/`) checked in all six `collect_*` projection
      paths; drop matches before sealing the DTO; count drops in
      `RedactionSummary.omitted_sensitive_paths`; structural test asserting
      those paths never appear in any projection. (PV-9 APPROVE-WITH-CONDITIONS.)
- [x] **091b workspace_root size-cap + NUL validation** — _medium (CE-6 gap)._
      `save_time.rs:986`. `workspace_root` taken as raw `String` →
      `canonicalize` uncapped/unchecked; NUL yields `-32603 Internal` instead of
      the contract `InvalidQuery`. **Fix:** validate NUL + 512B cap in the
      GctxDispatch impls before `PathBuf::from`; return structured
      `InvalidQuery`.
- [x] **091c O(n) Vec::insert under cache Mutex in `collect_impact_with_budget`**
      — _medium (ADR-031 lock budget)._ `lib.rs:531-544`. Sorted insert under
      `with_graphs` shifts up to `MAX_AFFECTED_SYMBOLS=20_000` per symbol.
      **Fix:** collect unsorted (O(1) push, bounded by cap) under the lock,
      sort/dedupe after release in `project_impact` (mirror
      `collect_candidates`/`project`), or bounded max-heap.
- [x] **091d CE-6 per-session byte ceiling for `graph://` reads** — _medium._
      `crates/anvil-cli/src/mcp/resources/mod.rs`. Page size (200) capped but no
      per-session byte accumulator; an assistant can reassemble the whole graph
      across many `resources/read` round-trips. **Fix:** per-MCP-session
      read-credit counter (~8 MiB); deduct serialized size; structured
      `quota_exceeded` on exhaustion. (If deferred: document as a tracked
      Phase-1 limitation.)
      - _Council survivor follow-up:_ the byte ceiling now also covers the GCTX
        **tool-call** surface (`anvil_search_symbols`, `find_dependents`,
        `find_callers`, `impact_of_change`, `affected_tests`). Those carry the
        same identity data and were previously bounded only by per-page
        pagination caps, so an assistant could reassemble the graph via
        `tools/call` past the resource cap. The dispatch
        (`commands/mcp.rs::tools_call_response`) now charges each GCTX tool's
        successful payload against the **same** `GRAPH_EGRESS_SPENT` credit
        (`charges_graph_egress` flag on `ToolDefinition`) and returns the same
        `quota_exceeded` result on exhaustion.

## CIB-092 — Persistence / warm-start wire-integrity & observability

Crate(s): `anvil-graph-cache` (snapshot/io), `anvil-intercept` (save_time, full_scan_executor).

- [x] **092a ADR-069 §6 golden wire-bytes fixture** — _high, CONFIRMED._
      `snapshot.rs:820-865`. Shipped tests compare two calls from the same
      binary; writer+reader drift together so a postcard/field/codec change
      slips through with no `SNAPSHOT_BACKING_SCHEMA_VERSION` bump. **Fix:** pin
      `to_bytes()` of the standard fixture against a committed `&[u8]`/`.bin` so
      any wire change fails CI.
- [x] **092b ADR-069 §10 metric counters** (`snapshot_load_result` /
      `snapshot_write_result`) — _high, CONFIRMED, graduation-blocker (not the
      default-off cut)._ `save_time.rs:383`. **Fix:** one increment per
      `load_snapshot` (labelled by `SnapshotReadError` variant) and per write
      (ok/error) via the existing `SaveTimeState` `TelemetryEmitter` fanout.
- [x] **092c ADR-069 §10 orphan `.snap` startup sweep** — _low (corrected from
      high), CONFIRMED._ `save_time.rs:1463-1465`. `sweep_snapshot_temps_on_start`
      removes only `*.tmp`; a worktree deleted while the daemon was down leaves
      its `.snap` forever. **Fix:** `sweep_stale_snapshots_on_start(registered_roots)`.
      - _Council survivor follow-up (item 3 — daemon wiring DEFERRED, honestly
        tracked):_ the function is **provided + empty-guarded** (a runtime guard
        returns 0 without deleting when the registered-root set is empty, and each
        root is canonicalized before hashing — see 092 council items 2 below), but
        it is **not wired into the daemon**. There is no safe call site: cold boot
        (`lib.rs:1465`, beside `sweep_snapshot_temps_on_start`) runs with an empty
        session registry, and the only available faithful set —
        `SessionRegistry::active_sessions()` — lists *currently-sessioned*
        worktrees, NOT *warm worktrees with a valid persisted snapshot*. Using it as
        the keep-set would reclaim the snapshot of a warm-but-idle worktree (no live
        session right now) — defeating warm-start for exactly the reattach case the
        snapshot exists for. A faithful registered-set source (a "warm worktrees
        seen this run" set distinct from "currently-sessioned") is a larger
        lifecycle change. **Daemon wiring deferred to a follow-up needing that
        faithful registered-set source.** The empty-guard is the safety net that
        lets a future caller wire it without a cold-boot wipe.
- [~] **092d openat2 RESOLVE_NO_SYMLINKS|RESOLVE_BENEATH discipline (ADR-069 §4)**
      — _medium._ `snapshot_io.rs:152-157,210-213`. Path-based open with
      leaf-only `O_NOFOLLOW`; ADR-069 §4 mandates `openat` relative to an
      `O_PATH` fd (repo already ships `open_workspace_dirfd`/`read_under_openat2`
      in `path_safety.rs`). **Fix:** anchor temp create + read to the validated
      dirfd.
- [x] **092e `sha2` undeclared dep on lean `anvil-graph-cache`** — _medium
      (ADR-064/069 dep budget)._ `Cargo.toml:42`. Sole use is
      `snapshot_filename` hashing the workspace-root path (unsalted, PV-12) —
      no crypto benefit. **Fix:** replace with FNV-1a/SipHash (or crate-internal
      crc32), or add to workspace deps with justification.
- [x] **092f ADR-069 §3 verdict-gate end-to-end test** — _medium (coverage)._
      `save_time.rs:651-911`. Live path is safe but untested: assert
      `validate_paths` serves `Stale` (not `Clean`/`Certified`) in the
      restore→reconcile window.
- [x] **092g fsync_dir-failed-after-rename mislabels persistence-failed** —
      _medium._ `snapshot_io.rs:162`. After `fs::rename` succeeds a failing
      `fsync_dir(dir)?` reports "persistence failed" for a durably-published
      file. **Fix:** treat rename-succeeded/fsync_dir-failed as semi-success.
- [x] **092h ADR-035 Notification on persist write failure** — _medium._
      `save_time.rs:383`, `full_scan_executor.rs:634`. With persistence enabled a
      write failure should raise an ADR-035 Notification, not only `warn!`.
      **Fix:** emit via existing `TelemetryBroadcaster`/`NotificationEnvelope`
      when `persistence_enabled()`.
      - _Council survivor follow-up (item 4 — honest downgrade, NOT a delivered
        notification):_ the persist-failure envelope is built with
        `TelemetryCorrelation::default()` (no `originating_session_id`), and the
        INTD-015 fanout (`fanout.rs::decide`) **hard-denies** any envelope whose
        `originating_session_id` is absent — to **every** subscriber, including the
        owner (the owner check `is_authorised(subscriber, originator)` itself needs
        an originator). A daemon-internal shutdown/background write has no
        originating session, so this envelope is **never delivered to anyone**. We
        searched `fanout.rs` / `broadcaster.rs` / `telemetry.rs` for a daemon-local
        health sink or any delivery path that bypasses the session-deny: **there is
        none** — every delivery routes through `Fanout::route`, and the session-deny
        is a deliberate INTD-015 invariant (the whole point of moving the
        access-control check daemon-side). So the **real, user-visible operator
        signal is the `tracing::warn!` per failed write PLUS the new cumulative
        snapshot-metrics shutdown `info!` log (item 1)** — not a delivered
        notification. The envelope is **kept** only because it is the correct shape
        for a *future* session-correlated producer / in-process subscriber (it costs
        nothing; the broadcaster simply drops it today); the code comments on
        `notify_persist_write_failure` / `PersistFailureNotifier::notify` already
        state this same-uid-local limitation plainly. **No overclaim:** 092h does
        not deliver a notification to an operator today.

## CIB-093 — GV2 substrate hot-path & trust correctness

Crate(s): `anvil-graph-cache` (trust/certify/incremental/snapshot), `anvil-intercept/src/kernel_cache.rs`.

- [x] **093a Extend `PRIVILEGED_MODULES`** — _medium (false-certify)._
      `trust.rs:16`. Add spawn/exec/sandbox-escape Node built-ins
      (`worker_threads`, `vm`, `v8`, `dns`, `tls`, `dgram`) — the load-bearing
      privilege gate currently certifies them CLEAN.
- [x] **093b Memoise reexport-privilege on certify hot path** — _medium._
      `certify.rs:314` re-walks Reexports BFS per-file every ContentModify after
      `annotate_trust` already stamped `trust_level`. **Fix:** read `trust_level`
      off the file's symbols, or memoise the set onto GraphDelta/warm-cache.
- [x] **093c `known_files` via O(files) index not O(symbols) scan** — _medium._
      `incremental.rs:686-693, 626-637` use `node_weights()` scan; sibling
      `re_resolve_calls_tracked` (~906) already uses `file_names()`. **Fix:**
      swap both to `file_names()`.
- [x] **093d `annotate_trust` latency signal** — _medium._
      `kernel_cache.rs:537`. Whole-graph O(N) pass under the Mutex on every save
      with no tracing span/WARN threshold; outside the GV2-025 gate. **Fix:**
      add a debug span + WARN threshold.
- [x] **093e Independent `SNAPSHOT_BACKING_SCHEMA_VERSION`** — _medium._
      `snapshot.rs:110` aliases it to `GRAPH_DELTA_SCHEMA_VERSION`; a delta-wire
      bump that leaves the DTO layout unchanged forces spurious cold rebuilds.
      **Fix:** give it its own independent const.

## CIB-094 — USAGE producer controls & robustness

Crate(s): `anvil-cli` (usage.rs, usage_views.rs, main.rs, intercept.rs), docs.

- [ ] **094a Operator kill-switch for CLI `command.invoked` producer** —
      _medium._ `record_invocation` (`main.rs:1102`→`usage.rs:605`) fires
      unconditionally; `ANVIL_INTERCEPT_DISABLE_OBSERVATION` guards daemon DPO
      only. **Fix:** add `ANVIL_USAGE_DISABLE=1` (or consult the existing knob).
- [ ] **094b Non-UTF-8 byte defeats retention trimming** — _medium._
      `trim_usage_sidecar_at` (`usage.rs:405`) uses `read_to_string`, returns
      silently on non-UTF-8 → file grows past the 64 MiB cap. **Fix:**
      `BufReader::lines()` skipping `InvalidData`; add a mid-file non-UTF-8 trim
      test.
- [ ] **094c Conformance test coverage** — _medium._
      `tests/usage_observation.rs:212` samples only 4 of ~40 commands but claims
      full coverage. **Fix:** iterate `registered_command_names()`, or correct
      the dossier claim.
- [ ] **094d Daemon-down non-dry-run Unblock → zero rows** — _medium._
      `intercept.rs:36-38` suppresses the CLI row; if the daemon is down no
      daemon row is written either. **Fix:** check daemon connectivity before
      suppressing, or emit the CLI row on IPC failure.
- [ ] **094e Retention/operator-control docs** — _medium._
      `docs/observability/usage-analytics.md:111-115` says "no built-in
      rotation" though a 7-day/64 MiB trim is live; three env knobs are
      code-comment-only. **Fix:** document the trim + knobs in docs + namespace
      registry.

## CIB-095 — Intercept hot-path follow-through

Crate(s): `anvil-intercept` (save_time, kernel_cache, lib, full_scan_executor).

- [ ] **095a `search_symbols` UNC-path filter parity** — _medium._
      `save_time.rs:1896-1908` omits the `starts_with(['/','\\'])` check that
      sibling verbs apply via `invalid_relative_path_reason` (1714). **Fix:**
      route through the shared helper.
- [ ] **095b restore→reconcile cannot emit Certified with empty `all_imports`**
      — _medium (confirm + guard)._ `kernel_cache.rs:537` + `save_time.rs:684`.
      Assurance machine is `Stale` in the window, but the certify verdict isn't
      forcibly blocked at the API layer. **Fix:** confirm non-certifiable; add a
      guard/test.
- [ ] **095c Scoped scan opt-out** — _medium (document + lever)._
      `save_time.rs:939-942,1003`. `workspace_status`/GCTX first-contact now
      spawn background scans on cold keys; only lever is `ANVIL_WATCH_DAEMON=0`.
      **Fix:** document the behaviour change; consider
      `ANVIL_WATCH_DAEMON_SCAN=0`.
- [ ] **095d Warm-graph snapshot lost on listener-failure exit** — _medium._
      `lib.rs:1572-1596`. `listener_handle.join()` error returns `Ok(())` at
      1576, bypassing `persist_all_on_shutdown()`. **Fix:** persist before the
      early return / common cleanup path.
- [ ] **095e Watchdog OS thread per scan job** — _low._
      `full_scan_executor.rs:282` spawns a raw thread for a `recv_timeout`.
      **Fix:** timer on the existing cancel channel / shared watchdog.
- [ ] **095f Shutdown snapshot-write-failure counter** — _low._
      `save_time.rs:384`. `persist_all_on_shutdown` emits only `warn!`/`info!`.
      **Fix:** add a `dropped_snapshot_writes` observation (pairs with 092b).

---

## Execution

Each cluster lands as its own dev-workflow unit (TDD → fresh Council → commit),
honouring the single-purpose boundary. CE-3 (091a) is the release-gating item
and lands first. Status here is the source of truth; tick boxes as each lands.

---

## Net-new items (sibling `anvil:2.1` cross-ref, 2026-06-21)

Source: `plans/audits/2026-06-21-v090-netnew-crossref.md` (all CIB-091..095
independently reproduced at HEAD; these are net-new gaps).

- [x] **N1 dynamic `require()`/`import()` invisible to the trust pass** — _HIGH,
      structural → CIB-093._ Literal `require('fs')`/`import('fs')` now flow
      through `is_privileged_import`; a computed/unresolved dynamic import sets
      `has_unresolved_dynamic_import`, threaded parser→GraphDelta→certify, forcing
      `Partial(ExportSurfaceChange)` instead of silent `Certified`.
- [x] **N3 case/`node:`-prefix privilege bypass** — _MED → CIB-093._
      `privileged_module_token` lowercases + strips case-insensitive `node:`
      before the `PRIVILEGED_MODULES` lookup (`'FS'`, `'NODE:fs'` now caught).
- [x] **N4 `matched`/substring-filter existence oracle** — _MED → CIB-091a._
      Verified already closed: sensitive files are dropped at the file-index
      level BEFORE the substring filter and the `matched` count (collect_candidates).
- [ ] **N2 blocking file I/O on the single-thread runtime** — _MED → CIB-094 (usage
      sink non-blocking) + CIB-095 (`persist_all_on_shutdown` → `spawn_blocking`)._
- [ ] **N5 raw `io::Error` Display on the `Io` arm reaches the wire** — _MED →
      CIB-091b follow-up: static reason, detail server-side._
- [ ] **N6 CRC-32 sole snapshot integrity gate** — _MED → CIB-092: accept under
      same-uid boundary, document CRC-32 as non-integrity-bearing._
- [ ] **N7 suffix-match import re-bind** — _MED → CIB-093 follow-up / tracked._
- [ ] **N8 `spawn_restore` dead no-op + verb coverage** — _LOW → CIB-095._
