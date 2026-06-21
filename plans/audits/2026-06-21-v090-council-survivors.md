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

- [ ] **091a CE-3 sensitive-path egress deny-list** — _high, CONFIRMED, cut-blocker._
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
- [ ] **091b workspace_root size-cap + NUL validation** — _medium (CE-6 gap)._
      `save_time.rs:986`. `workspace_root` taken as raw `String` →
      `canonicalize` uncapped/unchecked; NUL yields `-32603 Internal` instead of
      the contract `InvalidQuery`. **Fix:** validate NUL + 512B cap in the
      GctxDispatch impls before `PathBuf::from`; return structured
      `InvalidQuery`.
- [ ] **091c O(n) Vec::insert under cache Mutex in `collect_impact_with_budget`**
      — _medium (ADR-031 lock budget)._ `lib.rs:531-544`. Sorted insert under
      `with_graphs` shifts up to `MAX_AFFECTED_SYMBOLS=20_000` per symbol.
      **Fix:** collect unsorted (O(1) push, bounded by cap) under the lock,
      sort/dedupe after release in `project_impact` (mirror
      `collect_candidates`/`project`), or bounded max-heap.
- [ ] **091d CE-6 per-session byte ceiling for `graph://` reads** — _medium._
      `crates/anvil-cli/src/mcp/resources/mod.rs`. Page size (200) capped but no
      per-session byte accumulator; an assistant can reassemble the whole graph
      across many `resources/read` round-trips. **Fix:** per-MCP-session
      read-credit counter (~8 MiB); deduct serialized size; structured
      `quota_exceeded` on exhaustion. (If deferred: document as a tracked
      Phase-1 limitation.)

## CIB-092 — Persistence / warm-start wire-integrity & observability

Crate(s): `anvil-graph-cache` (snapshot/io), `anvil-intercept` (save_time, full_scan_executor).

- [ ] **092a ADR-069 §6 golden wire-bytes fixture** — _high, CONFIRMED._
      `snapshot.rs:820-865`. Shipped tests compare two calls from the same
      binary; writer+reader drift together so a postcard/field/codec change
      slips through with no `SNAPSHOT_BACKING_SCHEMA_VERSION` bump. **Fix:** pin
      `to_bytes()` of the standard fixture against a committed `&[u8]`/`.bin` so
      any wire change fails CI.
- [ ] **092b ADR-069 §10 metric counters** (`snapshot_load_result` /
      `snapshot_write_result`) — _high, CONFIRMED, graduation-blocker (not the
      default-off cut)._ `save_time.rs:383`. **Fix:** one increment per
      `load_snapshot` (labelled by `SnapshotReadError` variant) and per write
      (ok/error) via the existing `SaveTimeState` `TelemetryEmitter` fanout.
- [ ] **092c ADR-069 §10 orphan `.snap` startup sweep** — _low (corrected from
      high), CONFIRMED._ `save_time.rs:1463-1465`. `sweep_snapshot_temps_on_start`
      removes only `*.tmp`; a worktree deleted while the daemon was down leaves
      its `.snap` forever. **Fix:** `sweep_stale_snapshots_on_start(registered_roots)`.
- [ ] **092d openat2 RESOLVE_NO_SYMLINKS|RESOLVE_BENEATH discipline (ADR-069 §4)**
      — _medium._ `snapshot_io.rs:152-157,210-213`. Path-based open with
      leaf-only `O_NOFOLLOW`; ADR-069 §4 mandates `openat` relative to an
      `O_PATH` fd (repo already ships `open_workspace_dirfd`/`read_under_openat2`
      in `path_safety.rs`). **Fix:** anchor temp create + read to the validated
      dirfd.
- [ ] **092e `sha2` undeclared dep on lean `anvil-graph-cache`** — _medium
      (ADR-064/069 dep budget)._ `Cargo.toml:42`. Sole use is
      `snapshot_filename` hashing the workspace-root path (unsalted, PV-12) —
      no crypto benefit. **Fix:** replace with FNV-1a/SipHash (or crate-internal
      crc32), or add to workspace deps with justification.
- [ ] **092f ADR-069 §3 verdict-gate end-to-end test** — _medium (coverage)._
      `save_time.rs:651-911`. Live path is safe but untested: assert
      `validate_paths` serves `Stale` (not `Clean`/`Certified`) in the
      restore→reconcile window.
- [ ] **092g fsync_dir-failed-after-rename mislabels persistence-failed** —
      _medium._ `snapshot_io.rs:162`. After `fs::rename` succeeds a failing
      `fsync_dir(dir)?` reports "persistence failed" for a durably-published
      file. **Fix:** treat rename-succeeded/fsync_dir-failed as semi-success.
- [ ] **092h ADR-035 Notification on persist write failure** — _medium._
      `save_time.rs:383`, `full_scan_executor.rs:634`. With persistence enabled a
      write failure should raise an ADR-035 Notification, not only `warn!`.
      **Fix:** emit via existing `TelemetryBroadcaster`/`NotificationEnvelope`
      when `persistence_enabled()`.

## CIB-093 — GV2 substrate hot-path & trust correctness

Crate(s): `anvil-graph-cache` (trust/certify/incremental/snapshot), `anvil-intercept/src/kernel_cache.rs`.

- [ ] **093a Extend `PRIVILEGED_MODULES`** — _medium (false-certify)._
      `trust.rs:16`. Add spawn/exec/sandbox-escape Node built-ins
      (`worker_threads`, `vm`, `v8`, `dns`, `tls`, `dgram`) — the load-bearing
      privilege gate currently certifies them CLEAN.
- [ ] **093b Memoise reexport-privilege on certify hot path** — _medium._
      `certify.rs:314` re-walks Reexports BFS per-file every ContentModify after
      `annotate_trust` already stamped `trust_level`. **Fix:** read `trust_level`
      off the file's symbols, or memoise the set onto GraphDelta/warm-cache.
- [ ] **093c `known_files` via O(files) index not O(symbols) scan** — _medium._
      `incremental.rs:686-693, 626-637` use `node_weights()` scan; sibling
      `re_resolve_calls_tracked` (~906) already uses `file_names()`. **Fix:**
      swap both to `file_names()`.
- [ ] **093d `annotate_trust` latency signal** — _medium._
      `kernel_cache.rs:537`. Whole-graph O(N) pass under the Mutex on every save
      with no tracing span/WARN threshold; outside the GV2-025 gate. **Fix:**
      add a debug span + WARN threshold.
- [ ] **093e Independent `SNAPSHOT_BACKING_SCHEMA_VERSION`** — _medium._
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
