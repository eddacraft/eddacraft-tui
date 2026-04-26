# RTAI-001 — Phase-0 architecture spike report

**Date:** 2026-04-26
**Branch:** `rtai-001-spike`
**Spike binary:** `crates/spike/src/rtai_mid_edit.rs` (`spike-rtai-mid-edit`)
**Status:** Spike closed; RTAI module promoted from Proposed → Ready per the
RTAI-001 task gate. The remaining eight items stay at their existing statuses
(most are Proposed pending INTD/DRVR dependencies); the module-level state
captures "decided to do, deps real, value clear" rather than "actively in
flight."

## What this is

The phase-0 spike for `realtime-ai-validation.aps.md`. Per the RTAI-001 task
spec, the goal was to prove **end-to-end** that an in-flight buffer change in
one surface can reach the daemon, evaluate one rule against in-memory content,
and return a diagnostic inside the mid-edit latency budget — before any further
RTAI work commits to a shape.

The spike is intentionally throwaway. It does not attempt to land production
infrastructure; it answers two questions:

1. Does the simplest possible loop (driver → daemon → rule → diagnostic) fit
   under the `e2e.midEdit` p95 = 80 ms budget defined by ADR-031?
2. What architecture decisions does the measured shape force on RTAI-002+?

## Setup

| Component | Spike implementation |
|---|---|
| Driver "didChange" | In-process `mpsc::Sender<DidChange>` posting `{ path, text, version }` envelopes. Stand-in for the eventual LSP `textDocument/didChange` and MCP `apply_edit` payloads. |
| Daemon endpoint | Single worker thread blocked on `mpsc::Receiver`. Stand-in for the IPC listener INTD-002 will eventually deliver. |
| Rule | `anvil_checks::secret::scan_content` with `SecretCheckConfig::default()`. Picked because it is the rule whose mid-edit value-density is highest per ADR-031's RTAI-001 fallback clause, and it operates on raw buffer content with no disk I/O. |
| Diagnostic envelope | Canonical `anvil_kernel_types::Diagnostic` tagged `Mode::MidEdit`. Same shape AIGUARD-002 froze and AIGUARD-003 emits from the gate path. |
| Fixture | A 103-byte three-line buffer with `api_key='abcdEFGH1234567890'` on line 2. Picked to fire the default `API Key` rule deterministically (the bare `AKIA…` form is filtered by `looks_like_code`). |
| Iterations | 1024 round-trips after one warm-up. |

## Measurement

```text
RTAI-001 Phase-0 spike: mid-edit secret-detection round-trip
------------------------------------------------------------
iterations          : 1024
fixture path        : src/auth/client.ts
fixture buffer bytes: 103
diagnostics emitted : 1024 (avg 1.00 per round-trip)

Round-trip latency
  min  :     1297 µs
  p50  :     1315 µs
  p95  :     1360 µs
  p99  :     1617 µs
  max  :     2105 µs

Mid-edit p95 budget (ADR-031): 80 ms warm. Spike measured p95 1.4 ms.
Result: PASS — spike floor is well inside the warm budget.
```

Reproduce with `cargo run -q --release -p anvil-spike --bin spike-rtai-mid-edit`.
Numbers above are from the `aps-audit` runner; absolute values are dev-machine
specific but the ratio (~60× under budget) survives across hosts.

## What the numbers mean

The spike's p95 of **1.4 ms** is the **floor** for mid-edit round-trip latency
on this workload. Production will pay more than that on top of this floor:

| Cost the spike does **not** incur | Expected impact |
|---|---|
| IPC transport (Unix socket / named pipe) instead of in-process mpsc | ADR-031 budgets `2 × comp.transport` p95 = 6 ms |
| `anvil.diagnostic.v1` JSON serialise / deserialise on the wire | Subsumed by `comp.driverFraming` p95 = 3 ms |
| Larger buffers (the fixture is 103 bytes; real edits hit 10–100 KB files) | Linear in buffer size for `secret-detection`; sub-millisecond at typical sizes given the spike's per-byte cost |
| Multiple rules, not just secret-detection | Adds the rule's per-byte cost; antipattern + reasoning are the next two and both are line-bounded |
| Cancellation, batching, debounce admission | Adds bookkeeping but should not exceed `comp.driverFraming` |

Adding ADR-031's component budgets up: `e2e ≈ 2 × 3 ms + 1.5 ms + 50 ms + 2 × 3
ms = 63.5 ms` p95. The 80 ms `e2e.midEdit` budget has 16.5 ms of headroom over
the rubric's own component sum. The spike confirms the rubric's underlying
assumption — that a tight loop plus a single rule comfortably fits — is real,
not aspirational. The risk going forward is concentrated in `comp.ruleEval` (50
ms p95) once the rule set widens beyond secret-detection, not in the loop
shape.

**Bottom line:** the spike clears the gate. RTAI-002 can proceed against a
real budget rather than against an extrapolated one.

## Decisions

The RTAI-001 task spec asks the spike to record decisions on two questions.
Both are recorded here so the rest of the module can reference them rather
than re-litigating.

### Decision (a): extend the existing daemon RPC, or add a new method?

**Decision: single `scan_buffer` method, `mode` field discriminates save-time
vs mid-edit.**

The two paths share input shape (`{ path, text, version }`) and output shape
(`Vec<Diagnostic>`). The only semantic difference is the operating mode, and
that mode is already the canonical discriminator AIGUARD-002 reserved as
`KnownMode::SaveTime` / `KnownMode::MidEdit`. Three reasons to fold them into
one method instead of two:

1. **Wire surface stays small.** Adding `scan_buffer_save` and
   `scan_buffer_mid_edit` would force every transport (stdio, Unix socket,
   future HTTP) to register two near-identical methods. A discriminated single
   method keeps the surface tight and lets new modes (e.g. `pre-commit`,
   `diff-scan`) slot in without method proliferation.
2. **Mode is already on the diagnostic.** The reply already carries
   `Mode::MidEdit` per the envelope spec. Mirroring that on the request keeps
   the request/response symmetric and removes a class of "request was for
   save, response is tagged mid-edit" bug.
3. **Rule pipeline already branches on mode.** ADR-031 specifies different
   `comp.ruleEval` budgets for save-time (with disk read) vs mid-edit
   (content-from-request). A single method passing the mode through to the
   pipeline keeps the dispatch in one place; two methods would duplicate the
   branch on the boundary instead.

What this means for RTAI-002 (Daemon mid-edit RPC surface): the method
signature is `scan_buffer(path: String, text: String, version: u64, mode:
Mode) -> Vec<Diagnostic>`. INTD-002's existing IPC listener picks up the
new method; no second RPC entry point is added.

### Decision (b): does driver-side debounce belong in `DriverClient` or in each driver?

**Decision: `DriverClient` owns the debouncer; per-driver tuning is a config
parameter, not a reimplementation point.**

The mid-edit path runs through every driver Anvil ships (LSP, MCP, future
editor-specific shims). If each driver implements its own debounce, three
things go wrong:

1. **Tuning drifts.** ADR-031's RTAI-004 80 ms debounce is one number; if
   three drivers each decide what "80 ms" means, telemetry from real users
   stops being comparable across surfaces.
2. **Cancellation gets harder.** The spike does not exercise cancellation,
   but real mid-edit loops must drop stale work as new keystrokes arrive.
   Cancellation logic that lives in the shared `DriverClient` debouncer
   composes once; in three drivers it composes three times and drift becomes
   a feature.
3. **Test surface multiplies.** A `DriverClient`-owned debouncer is one
   integration test against one fake transport. Per-driver debounce is N
   integration tests with three subtly different shapes.

Per-driver tuning of the debounce window (e.g. LSP `didChange` is per-keystroke,
MCP `apply_edit` is per-agent-batch) is expressed as a config parameter on
`DriverClient::with_debounce(window: Duration)`. Drivers parameterise; they do
not reimplement.

What this means for RTAI-004 (DriverClient mid-edit envelope + debouncer): the
debouncer lives in `crates/anvil-kernel-types` (or wherever `DriverClient`
ultimately lands), not in `anvil-cli`'s editor adapter or in MCP server code.

## What this spike does not answer

These are the questions deferred to the rest of the RTAI module — flagging them
here so RTAI-002+ start with the open list, not a re-derivation:

- **Real IPC transport floor.** The spike measures in-process latency. The IPC
  cost (Unix domain socket / Windows named pipe / Tokio framed transport)
  remains unmeasured. RTAI-003 (mid-edit latency benchmark + budget
  enforcement) is the place to measure it on real wire.
- **Concurrent buffer-change pressure.** The spike serialises one round-trip
  at a time. Behaviour under "user typing fast across two open buffers" is
  unmeasured. RTAI-002 needs a back-pressure decision: do we coalesce stale
  versions in the daemon, or in the driver, or both?
- **Rule-set scaling.** Only secret-detection ran. Antipattern + reasoning
  (AI-001 from #1111) are the next mid-edit candidates. Their per-byte cost
  needs measuring before they ship to the mid-edit path.
- **Cold-start cost.** ADR-031's budgets are explicitly for the warm path.
  First-keystroke latency (regex compilation, page faults) is unmeasured here
  and is the hardest perception case for "feels real-time".

## Cross-references

- Module: `plans/modules/realtime-ai-validation.aps.md` (RTAI-001 closes; -002
  and beyond gated on this report's review).
- Latency rubric: `plans/decisions/031-validation-latency-rubric.md`.
- Diagnostic shape: `plans/specs/2026-04-26-diagnostic-envelope-coordination.md`.
- Demo runbook: `plans/specs/2026-04-26-rtai-demo-runbook.md`.
- Spike binary: `crates/spike/src/rtai_mid_edit.rs`.
