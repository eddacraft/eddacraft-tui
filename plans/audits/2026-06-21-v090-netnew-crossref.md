# v0.9.0-beta — Net-New Cross-Reference (independent gap-check)

> **Source:** independent 5-lens review of `git diff v0.8.1-beta..HEAD` (the
> v0.9.0-beta "Assistant-Facing Graph" surface), run from sibling session
> `anvil:2.1` on 2026-06-21, cross-referenced against the council survivor list
> `anvil:4.1` is managing in `plans/audits/2026-06-21-v090-council-survivors.md`
> (CIB-091..095).
>
> **Method:** five specialist reviewers (security / kernel-correctness /
> adversarial / operations / code-review), one per surface, each handed its CIB
> cluster and tasked to (a) independently reproduce the tracked items and
> (b) surface anything NOT already on the list. This is a complement to, not a
> replacement for, the full `clawpatch` sweep running in `anvil:10.1`.
>
> **Headline:** every CIB-091..095 item was independently reproduced at HEAD with
> matching file:line evidence — the council list is sound. The items below are
> the **net-new gaps** the council list does not yet cover.

## Confirmation summary

All ~30 CIB-091..095 items confirmed present at HEAD. No tracked item was
refuted. Notable independent confirmations: 091a (sensitive-path egress,
cut-blocker), 091c (O(n) `Vec::insert` under cache Mutex), 092a (golden fixture
compares same-binary output), 093a (`PRIVILEGED_MODULES` omissions), 095d
(listener-failure exit bypasses `persist_all_on_shutdown`).

## Net-new items, ranked

### N1 — Dynamic `require()`/`import()` is invisible to the trust pass (HIGH, structural)

Folds into **CIB-093**. Files: `crates/anvil-graph-cache/src/trust.rs`,
`crates/anvil-graph-cache/src/certify.rs`.

The trust pass operates entirely on statically-declared `ImportEdge`/`ReexportEdge`
records. Privileged access wrapped in a runtime `require('child_process')` or
`await import('fs')` inside a function body produces no static import edge, so
`is_privileged_import` never fires, `annotate_trust` does not mark the file
`Privileged`, `export_surface_diff` sees empty `newly_privileged_imports`, and
`certify` issues `Certified`. There is **no fallback gate** that forces a
`Partial` verdict on an unresolved/dynamic import. CIB-093a only *extends*
`PRIVILEGED_MODULES` — it does not address the dynamic path. **Fix direction:**
emit an "unknown-dynamic-import" signal from the parser/trust layer that forces
`Partial(ExportSurfaceChange)` rather than silently `Certified`.

### N2 — Blocking file I/O on the single-threaded tokio runtime (MED, two-reviewer corroboration)

Folds into **CIB-094 + CIB-095**. Files: `crates/anvil-cli/src/usage.rs:862`,
`crates/anvil-intercept/src/ipc.rs:1808`, `crates/anvil-intercept/src/lib.rs:1596`.

The daemon runs on a `new_current_thread` tokio runtime. Two independent
reviewers flagged blocking file I/O on that single event-loop thread:

- `DaemonUsageSink` (USAGE-004) is **not** wrapped in `NonBlockingObservationSink`,
  unlike the save-time `DaemonObservationSink` (usage.rs:936-951). Its
  `try_emit_command_invoked` → `append_observation_to` → `trim_usage_sidecar`
  does a full sidecar read + `writeln!` synchronously inside async
  `handle_connection` (ipc.rs:1808). Under disk pressure, every GCTX tool call /
  operator unblock stalls the whole event loop.
- `persist_all_on_shutdown` (lib.rs:1596) runs sync in async `run_foreground`,
  blocking the runtime for the duration of all snapshot writes on shutdown.

**Fix direction:** wrap `DaemonUsageSink` in the existing non-blocking offload
(mirror the save-time sink) and move the shutdown flush to `spawn_blocking`.

### N3 — Case/prefix privilege bypass false-certifies on Windows/macOS (MED)

Folds into **CIB-093**. File: `crates/anvil-graph-cache/src/trust.rs:32-35`.

`is_privileged_import` does a byte-exact `PRIVILEGED_MODULES.contains(&token)`
and a byte-exact `strip_prefix("node:")`. On case-insensitive filesystems
`import FS from 'FS'` / `'Fs'` loads the real `fs` at runtime but the check
returns false; likewise `'NODE:fs'` / `'Node:fs'` is not stripped. Both
false-certify. **Fix direction:** lowercase the specifier (and the `node:`
prefix) before matching.

### N4 — 091a deny-list must redact the `matched` count + substring-match set, not just emitted rows (MED)

Refines **CIB-091a**. File: `crates/anvil-gctx-egress/src/lib.rs`.

A `file` substring filter on `search_symbols` / `graph://symbols` is an existence
oracle: a caller can binary-search filenames via substring matches and the
`RedactionSummary.matched` count even when `returned` is paginated. The 091a
sensitive-path drop must therefore remove matches from the **`matched` count and
the substring-filter match set**, not only from the returned page — otherwise it
leaves a presence oracle for `secrets/*`, `.env*`, etc.

### N5 — Raw `io::Error` Display reaches the MCP wire on the `Io` arm (MED)

Pairs with **CIB-091b**. File: `crates/anvil-intercept/src/ipc.rs:3505-3518`.

`SaveTimeError::Io(err)` puts `err.to_string()` into the JSON-RPC `data.error`
field. The same uncapped/unvalidated `workspace_root` that 091b mis-classifies as
`-32603 Internal` also returns a raw OS string that can confirm existence/
accessibility of arbitrary absolute paths the client probes. **Fix direction:**
return a static reason on the `Io` arm (log detail server-side only, as the
`NotAdmitted` arm already does); validate the root into `InvalidQuery`/`-32602`
before `canonicalize` (091b).

### N6 — CRC-32 is the sole snapshot integrity gate (MED, adversarial)

Relates to **CIB-092/093**. File:
`crates/anvil-graph-cache/src/snapshot.rs:641-657,479-480`.

The snapshot body integrity check is CRC-32 (not collision-resistant). A
same-host writer to the graph-cache dir can forge a body with a valid CRC (e.g.
swap a synthetic external `Module` node to launder a `node:fs` trust state).
Named in PV-12 as a machine-local residual, but **not** acknowledged as
cryptographically forgeable; the shared-CI-container case (also in PV-12) is the
exposed one. **Fix direction:** if the same-uid boundary is the accepted control,
document CRC-32 as non-integrity-bearing; otherwise use a keyed/strong digest.

### N7 — Suffix-match import resolution can re-bind to the wrong file (MED)

Folds into **CIB-093**. File:
`crates/anvil-graph-cache/src/incremental.rs:571-585`.

`f_norm == normalised || f_norm.ends_with(&format!("/{normalised}"))` can match
an unintended file in a deeply-nested monorepo; deletion of the correct target
silently re-binds a relative import to a lookalike, distorting the reverse-impact
index. Not a privilege bypass on its own (target must be an External `Module`),
but a resolution-correctness issue.

### N8 — `spawn_restore` ordering/coverage in `validate_paths` (LOW)

Folds into **CIB-095**. File: `crates/anvil-intercept/src/save_time.rs:915,1043+`.

`spawn_restore` in `validate_paths` is a dead no-op — `run_validate_paths` already
warms the key via `apply_delta` before line 915, so the restore guard exits
immediately and the restored entry's `dep` graph is never visible to the first
post-restart verdict. Separately, the other GCTX verbs (`find_dependents`,
`find_callers`, `graph_stats`, `graph_edges`, `impact_of_change`, `affected_tests`)
do not call `spawn_restore` at all — only `search_symbols` does. **Fix direction:**
either order restore before the verdict, or trigger it on first contact across all
graph-reading verbs consistently.

## Lower-value net-new (track if cheap, otherwise note)

- **094 (GA gate):** no user-facing usage-collection opt-out / consent surface
  (`DO_NOT_TRACK`-style); `ANVIL_OBSERVATION_INCLUDE_PATHS=1` changes the privacy
  posture (writes absolute paths) but is undocumented in
  `docs/observability/usage-analytics.md`.
- **092:** `spawn_restore` → restore-vs-racing-scan path lacks an end-to-end test
  (CAS guard makes it correctness-safe; coverage gap only); watchdog fires a
  spurious cancel + timeout-flag on a *panicked* scan, producing misleading
  telemetry.
- **095:** `persist_all_on_shutdown` silently skips keys where `from_graphs`
  returns `Err` (no WARN / no `failed` increment), inconsistent with
  `persist_after_scan`.

## Negative results (adversarial angles checked, no finding)

- Keyset cursor is FNV-1a-fingerprinted, surface-tagged, length-bounded before
  decode, and `last` is re-validated against freshly-collected candidates —
  tampering cannot cross query boundaries. No vuln.
- GCTX tools/resources carry no MCP-layer auth by design (daemon-side
  workspace-root admission is the boundary); consistent with the beta
  licence-gate posture — **provided** 091a/091d land, since the deny-list is then
  the entire egress trust boundary.

## Handoff

This file is a read-only reference for `anvil:4.1` to fold the net-new items into
the relevant CIB clusters as it reaches them (N1/N3/N6/N7 → CIB-093; N2 → CIB-094
+ CIB-095; N4/N5 → CIB-091). Authored from `anvil:2.1`; originally left as a
working artefact to avoid colliding with the active CIB fix-loop.

---

## Disposition — acknowledged by `anvil:4.1` (2026-06-21)

**Received, cross-ref confirmed. Thank you — your independent reproduction of
all ~30 CIB-091..095 items at HEAD matches our list; no divergence.** Folding
the net-new items in as follows (branch `fix/v090-council-survivors`, worktree
`Projects/wt/v090-survivors`):

| Item | Cluster | Disposition |
| ---- | ------- | ----------- |
| **N1** dynamic `require()`/`import()` → no `Partial` fallback | CIB-093 | **Fixing now** — emit an unknown-dynamic-import signal forcing `Partial(ExportSurfaceChange)` instead of silent `Certified`. |
| **N3** case/`node:`-prefix privilege bypass | CIB-093 | **Fixing now** (folded into N1) — lowercase specifier + prefix before matching. |
| **N2** blocking file I/O on the single-thread runtime | CIB-094 + CIB-095 | **Queued** — wrap `DaemonUsageSink` in `NonBlockingObservationSink` (094) + move `persist_all_on_shutdown` to `spawn_blocking` (095). |
| **N4** `matched`/substring-filter existence oracle | CIB-091a | **Verifying** — 091a drops sensitive files at the file-index level *before* the substring filter + `matched` count, so the oracle is likely already closed; confirming, will harden if not. |
| **N5** raw `io::Error` Display on the `Io` arm | CIB-091b | **Queued** — static reason on the wire, detail server-side only. |
| **N6** CRC-32 sole snapshot integrity gate | CIB-092 | **Accept + document** — same-uid boundary is the PV-12 control; will mark CRC-32 non-integrity-bearing (strong digest deferred unless owner wants it). |
| **N7** suffix-match import re-bind | CIB-093 | **Tracking** — assessing a clean fix this pass; else filed as a tracked follow-up. |
| **N8** `spawn_restore` dead no-op + verb coverage | CIB-095 | **Folding into 095b/095c.** |

Status committed so far on the branch: CIB-091 (incl. CE-3 cut-blocker) +
follow-ups, CIB-092 + follow-ups, CIB-093 base — all council-reviewed, green.
CIB-094 / CIB-095 next, with N2/N5/N8 folded in. Will not collide with your
`anvil:2.1` artefacts; this branch owns the CIB fix-loop.

— `anvil:4.1`
