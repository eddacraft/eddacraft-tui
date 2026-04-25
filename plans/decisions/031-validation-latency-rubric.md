# ADR-031: Single Latency Rubric for Save-Time and Mid-Edit Validation

## Status

Proposed

## Date

2026-04-26

## Context

Anvil's intercept daemon serves two structurally different validation
flows that today carry three different, mutually inconsistent latency
budgets across the planning record:

- **`intercept-daemon.aps.md` INTD-014** — the JSON-RPC conformance +
  round-trip benchmark task references a "warm daemon" budget without
  pinning percentiles, fixture corpus, or the boundary being measured.
  It points at a "< 100ms p95 warm" save-time number borrowed from the
  editor-driver design spec without an evidence base.
- **`surface-drivers.aps.md` DRVR-002** — the editor-driver protocol
  task names "< 50ms daemon-side / < 100ms total save-to-diagnostic
  p95" as a §3.4 exit criterion, flags that those numbers extrapolate
  from in-process KERN benchmarks, and parks an unowned harness as
  S7.
- **`realtime-ai-validation.aps.md` RTAI** — the mid-edit module
  declares "< 50ms p95 daemon-side / < 100ms total round-trip" as a
  *stricter* mid-edit budget, drops it onto the same wire as
  save-time, and acknowledges in its Risks section that the numbers
  come from halving the save-time budget with no mid-edit evidence.

These three statements describe two genuinely different paths:

- **Save-time path.** Triggered by `didSave` (or filesystem watcher in
  daemon-only flows). Driver requests a scan; daemon reads file from
  disk, evaluates rules, returns diagnostics. Measured from save event
  on the surface to diagnostic delivered back to the surface. Lower
  call rate, larger payloads possible (whole-file content), tolerant
  of disk I/O.
- **Mid-edit path.** Triggered by `didChange` (LSP) or pre-write tool
  intercept (MCP). Driver supplies unsaved buffer content in the
  request; daemon evaluates rules without disk I/O; returns
  diagnostics. Measured from typed character (or proposed-write
  intercept) to diagnostic delivered. Higher call rate (every typing
  burst after debounce), no disk I/O, tighter human-perception budget
  ("real-time" as agent feedback).

Council A surfaced the inconsistency as a launch-blocker: three
modules invent their own numbers, none of them measure the same thing
end-to-end, and downstream work items pin to whichever variant their
author copied first. Until one rubric exists, the launch story for
real-time AI-output validation cannot honestly claim a latency
contract.

This ADR defines that single rubric. INTD-014, DRVR-002, RTAI-002, and
RTAI-006 will be re-pointed at this document instead of carrying their
own numbers.

## Decision

Adopt one validation latency rubric covering both modes. The rubric
specifies (a) the boundaries being measured, (b) the budgets at p50,
p95, p99, (c) the measurement methodology, (d) the fixture corpus,
and (e) the fallback if measured numbers exceed budget.

### Boundaries and what they measure

Two end-to-end boundaries, four component boundaries, all named
explicitly so downstream work items reference one definition.

#### End-to-end boundaries

| Boundary | Mode | Start event | End event |
| --- | --- | --- | --- |
| `e2e.save` | Save-time | Surface emits save event (`didSave` on the editor driver, fs-watcher event in daemon-only mode) | Diagnostic delivered to surface (`publishDiagnostics` received, or MCP tool response composed) |
| `e2e.midEdit` | Mid-edit | Typed character lands in surface buffer (`didChange` fired) **or** MCP pre-write tool call enters the driver | Diagnostic delivered to surface (`publishDiagnostics` received with `phase: "midEdit"` marker, or MCP tool response composed) |

Driver-side debounce is **excluded** from the budget — debounce is a
cooperative quieting interval, not latency. The clock starts on the
event that survives debounce. RTAI-004's 80ms debounce is therefore
not a tax on the mid-edit budget.

#### Component boundaries

Each component boundary is independently measurable so the rubric can
diagnose where budget is being spent without re-instrumenting.

| Boundary | What it measures |
| --- | --- |
| `comp.transport` | Driver `request()` call to daemon `accept()` of the JSON-RPC frame, plus the symmetric return path (response written by daemon to response acked by driver). UDS / named-pipe round-trip only — excludes JSON parsing on either end. |
| `comp.daemonRpc` | Daemon JSON-RPC dispatch: frame parsed, method resolved, params validated, ready to enter rule pipeline. |
| `comp.ruleEval` | Rule pipeline: content acquired (read from disk for save-time, taken from request for mid-edit), binary check + size cap applied, configured rules executed against content, diagnostics built. |
| `comp.driverFraming` | Surface-side: time spent in `DriverClient` framing, NDJSON serialise/deserialise, debouncer admission check (excluding the debounce interval itself). |

For any e2e measurement, `e2e ≈ 2*comp.transport + comp.daemonRpc +
comp.ruleEval + 2*comp.driverFraming` (plus surface-specific overhead
the driver cannot avoid — e.g. VSCode diagnostics-collection redraw).
The rubric budgets the four components additively so a regression in
one is locatable without bisecting the e2e number.

### Budgets

Budgets are stated at p50, p95, and p99 against the canonical fixture
corpus (defined below) on a warm daemon. Cold-start is **out of
scope** for this rubric — daemon cold-start is a separate launch
concern handled by INTD-001.

#### End-to-end budgets

| Boundary | p50 | p95 | p99 |
| --- | --- | --- | --- |
| `e2e.save` | 50 ms | 120 ms | 250 ms |
| `e2e.midEdit` | 35 ms | 80 ms | 150 ms |

Save-time p95 is loosened to 120 ms (from the legacy "< 100 ms"
phrasing) because the disk-read step in INTD-005 is genuinely
variable on consumer SSDs at the cap fixture, and a budget that
ignores that variance fails honestly under load. Mid-edit p95 stays
tight at 80 ms because no disk I/O is on the path; this is the
human-perception budget for "feedback while I'm typing".

#### Component budgets (warm, near-cap fixture)

| Boundary | p50 | p95 | p99 |
| --- | --- | --- | --- |
| `comp.transport` | 1.0 ms | 3.0 ms | 8.0 ms |
| `comp.daemonRpc` | 0.5 ms | 1.5 ms | 4.0 ms |
| `comp.ruleEval` (save-time, with disk read) | 30 ms | 80 ms | 180 ms |
| `comp.ruleEval` (mid-edit, content-from-request) | 20 ms | 50 ms | 110 ms |
| `comp.driverFraming` | 1.0 ms | 3.0 ms | 8.0 ms |

These component budgets sum, with headroom, to the e2e budgets above.
The headroom is intentional — surface-side overhead (VSCode
diagnostics redraw, MCP tool-response composition) eats the rest and
varies per consumer.

### Measurement methodology

One harness, one fixture corpus, one tool, used identically for
INTD-014, DRVR-002, RTAI-003, and any future budget claim.

#### Tool

- **Daemon-side and component-level:** `criterion` benches in
  `crates/anvil-intercept/benches/` — `ipc_roundtrip.rs` (existing,
  expanded under INTD-014) and `midedit_roundtrip.rs` (new under
  RTAI-003). Component boundaries instrumented via `tracing` spans
  with histograms exported through the existing observability lane.
- **End-to-end:** a node test harness in
  `packages/anvil-driver-client/test/latency/` that drives the
  `DriverClient` API against a real daemon binary, records timings
  with `process.hrtime.bigint()`, and emits the same percentile
  layout as the criterion side. The harness lives next to
  `DriverClient` because that is the boundary the surface actually
  experiences.

#### Fixture corpus

The canonical corpus is named `latency-corpus-v1` and lives at
`crates/anvil-intercept/tests/fixtures/latency-corpus-v1/`. It
contains:

- `small.txt` — 2 KB, plain text, 1 secret-detection match.
- `medium.ts` — 50 KB, typical TypeScript module, 2
  antipattern matches.
- `large.ts` — 500 KB, generated module, 5 mixed matches.
- `near-cap.ts` — 1 MB minus 1 byte (the INTD-005 content cap), 10
  mixed matches. Exercises the size-cap edge.
- `binary-blob.bin` — 100 KB binary, exercises the binary-detection
  short-circuit.
- `empty.txt` — 0 bytes, exercises the empty-content path.
- `unicode-heavy.md` — 100 KB of mixed UTF-8 with combining
  characters, exercises grapheme-cluster handling in
  reasoning-pattern rules once they exist.

Each fixture is committed and immutable — versioning is in the
directory name (`latency-corpus-v1`). A future corpus revision is a
new directory, not a mutation, so historical bench numbers stay
comparable.

#### Active rule set

Benches run with the **default v1 rule set** active:
`secret-detection`, `antipattern`, `path-deny`, `regex-content`. The
reasoning-pattern rules (AI-001..AI-007) are **not** part of the
canonical corpus run because they live in a crate that does not yet
exist (RTVS open question 3); when they land they extend the corpus
with their own bench rather than replacing the rubric.

#### Run conditions

- **Warm daemon.** The harness performs 100 throwaway iterations
  before the recorded run to amortise JIT-ish effects (rule registry
  cache warmup, PGID lookup cache, etc.).
- **CI host class.** Numbers are recorded against `ubuntu-latest`
  on the workspace CI runner. macOS and Windows are recorded as
  separate datasets — the rubric numbers above are
  Linux-CI-runner numbers; deviation thresholds for the other
  platforms are tracked, not enforced.
- **No concurrent IPC traffic.** The harness is the only client of
  the daemon during measurement.
- **Reported as percentiles, never as averages.** Averages are
  forbidden in the rubric — they hide tail behaviour, which is
  exactly what the budget targets.

#### Regression policy

CI fails a benchmark run if:

- Any p95 number for the run regresses by more than 15 % vs the
  recorded baseline for that boundary on that platform, or
- Any p99 number regresses by more than 25 %, or
- Any e2e p95 exceeds its absolute budget above (regardless of
  baseline drift).

Baseline numbers are committed to
`crates/anvil-intercept/benches/baselines/`. Updating a baseline
requires an explicit commit message tagged `bench-baseline:` so
re-baselining is auditable.

## Open questions

1. **RTAI-001 spike is the validator for the mid-edit numbers.** The
   `e2e.midEdit` p95 budget of 80 ms is the rubric's claim *before*
   measurement. RTAI-001 measures the actual numbers on
   `latency-corpus-v1`. If the measured p95 exceeds 80 ms by more
   than the regression tolerance, the rubric does **not** loosen the
   number to fit reality; instead the **save-time framing replaces
   the mid-edit framing**:
   - Mid-edit reverts to a degraded mode where it runs a reduced
     rule subset (secret-detection only — the rule whose mid-edit
     value-density is highest) on a tighter debounce, and the
     "real-time validation" capability claim is downgraded to
     "save-time validation with mid-edit secret peek".
   - The rest of the mid-edit feature remains gated behind a
     feature flag until the rule pipeline is fast enough to honour
     the rubric.

   This fallback is the rubric's pre-committed answer to
   "what if the numbers don't hold" — it stops the rubric from
   becoming aspirational fiction.
2. **Cold-start rubric.** Daemon cold-start latency is out of scope
   here. INTD-001 owns it. The rubric assumes a warm daemon; if a
   cold-start budget becomes a launch concern, it gets its own ADR
   rather than being bolted onto this one.
3. **Surface-specific overhead.** The headroom between the sum of
   component budgets and the e2e budgets absorbs surface-specific
   redraw / response-composition cost. If a particular surface
   consistently eats more headroom than budgeted (e.g. VSCode
   diagnostics-collection redraw at scale), that surface owns the
   investigation — the daemon-side rubric does not loosen.
4. **Multi-rule scaling.** The rubric is calibrated for the v1
   default rule set. As the catalogue grows (reasoning-pattern
   rules, structural rules), per-rule cost has to be measured and
   either fits inside `comp.ruleEval` or the rubric is revised. The
   regression policy catches drift; new rules failing the budget
   are the rule's problem to optimise, not the rubric's to widen.
5. **Plain-prose budgets in module docs.** Once this ADR is
   Accepted, INTD-014 / DRVR-002 / RTAI-002 / RTAI-006 should
   delete their inline numbers and link here. That cleanup is
   tracked as a follow-up item in each module's update pass — the
   ADR's existence does not by itself rewrite the modules.

## Consequences

- **Positive:**
  - One source of truth for validation latency. Three modules stop
    inventing their own numbers; downstream specs cite ADR-031.
  - Component boundaries make regressions diagnosable without
    re-instrumentation.
  - Fixture corpus is versioned and immutable, so historical bench
    numbers stay comparable.
  - p50/p95/p99 framing kills the "the average is fine" failure
    mode that hides tail-latency regressions.
  - Pre-committed fallback (RTAI-001 spike → save-time framing if
    mid-edit fails) prevents the rubric from becoming aspirational.
- **Negative:**
  - The save-time p95 budget loosens from "< 100 ms" (the legacy
    phrasing in the editor-driver design) to 120 ms. Marketing /
    docs that quoted the tighter number need updating.
  - Two benches and one node harness is more measurement
    infrastructure than the modules currently have. INTD-014 and
    RTAI-003 absorb that cost; DRVR-002 references both.
  - The rubric is calibrated against a single CI host class (Linux
    runner). macOS / Windows numbers are tracked, not enforced.
    A regression visible only on Windows is allowed by the rubric
    but flagged for follow-up.
- **Risks:**
  - **Measured mid-edit numbers don't fit the budget.** Pre-empted
    by Open Question 1's fallback. The rubric's reaction to bad
    numbers is documented; it is not "raise the budget".
  - **Fixture corpus doesn't represent real workloads.** The corpus
    is a synthetic anchor. If real users routinely operate above
    `near-cap.ts` or with rule mixes the bench doesn't cover, the
    corpus needs revision (`latency-corpus-v2`). The directory-name
    versioning makes that auditable.
  - **Component sums don't actually add to e2e.** The "headroom"
    absorbs surface redraw cost; if it doesn't, either a component
    is mis-budgeted or the surface is doing more work than
    expected. The rubric mandates that the e2e harness records
    component spans alongside the e2e number so this is visible.
- **Mitigations:**
  - INTD-014, DRVR-002, RTAI-002, RTAI-003, RTAI-006 cite this ADR
    rather than copying numbers. Numerical drift is a single-file
    edit in this ADR plus a baseline re-commit.
  - The fallback in Open Question 1 is the launch-blocker safety
    valve: if reality doesn't fit, capability claims downgrade
    before the rubric does.

## References

- Related ADRs: [ADR-015](./015-intercept-loop-enforcement.md)
  (intercept loop authority — AD-4 IPC transport, AD-6 rule
  integration), [ADR-030](./030-surface-drivers-supersede-napi-cutover.md)
  (drivers-on-daemon architecture).
- APS modules:
  - [intercept-daemon](../modules/intercept-daemon.aps.md) —
    INTD-014 references this ADR for save-time round-trip
    measurement; it owns the JSON-RPC conformance work that the
    bench rides on.
  - [surface-drivers](../modules/surface-drivers.aps.md) —
    DRVR-002 references this ADR in its §3.4 exit criterion and
    drops the S7 harness ask in favour of pointing at this
    rubric.
  - [realtime-ai-validation](../modules/realtime-ai-validation.aps.md)
    — RTAI-002 / RTAI-003 / RTAI-006 reference this ADR for the
    mid-edit budgets, fixture corpus, and the fallback in Open
    Question 1.
- Design specs:
  [editor-and-mcp-driver-design](../specs/anvil-driver-framework/editor-and-mcp-driver-design.md)
  §3.4 (the prior save-time number whose evidence base this ADR
  supplies).
