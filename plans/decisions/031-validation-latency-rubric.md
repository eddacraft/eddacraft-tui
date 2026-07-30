# ADR-031: Validation Latency Measurement Rubric

## Status

Proposed

## Date

2026-04-26

## Context

Anvil's intercept daemon is becoming the shared validation point for multiple
surfaces:

- save-time editor diagnostics
- filesystem-watch validation
- editor mid-edit diagnostics from unsaved buffers
- MCP / agent pre-write validation before proposed content reaches disk
- future drivers that should reuse the same daemon, rule registry, diagnostic
  envelope, and measurement language

The planning record currently carries inconsistent latency language across
INTD, DRVR, and RTAI. Some tasks quote daemon-side numbers, some quote
save-to-diagnostic numbers, some include debounce, and some extrapolate from
in-process benchmarks. That makes the requirement hard to verify and encourages
each new intercept use case to invent its own measurement scheme.

The problem is not that every validation path needs a bespoke budget. The
problem is that every validation path needs to state:

1. which mode it is measuring,
2. which timing boundary it is claiming,
3. which content source and rule set were used, and
4. whether the path is interactive enough to carry a latency SLO.

This ADR defines that shared rubric. INTD-014, DRVR-002, RTAI-002, RTAI-003,
RTAI-006, and future intercept modes cite this document instead of copying local
latency definitions.

## Decision

Adopt one validation latency measurement rubric for all intercept-backed
validation paths.

The rubric defines:

- canonical validation modes,
- canonical timing boundaries,
- required measurement dimensions,
- current interactive p95 SLOs, and
- rules for adding future modes without duplicating scanner or driver patterns.

Detailed benchmark implementation, exact fixture files, baseline storage, and
CI wiring stay in the owning work items. This ADR is the source of truth for
what those tasks must measure and how they must label the result.

### Shared validation shape

All intercept validation paths follow the same logical shape:

```text
surface trigger
  -> content acquired or supplied
  -> driver sends validation request
  -> daemon evaluates configured rules
  -> daemon returns diagnostics or structured error
  -> surface renders diagnostics or composes a tool response
```

New paths should reuse this shape before adding a new protocol, scanner, or
surface-specific validation pipeline. A path is a new mode only when it differs
in content source, trigger semantics, or enforcement semantics.

### Validation modes

| Mode | Trigger | Content source | Enforcement semantics | Budget class |
| --- | --- | --- | --- | --- |
| `save` | Editor `didSave`, explicit save command, or equivalent save event | On-disk file content unless the driver already has an authoritative saved buffer | Diagnostics and configured save-time enforcement | Interactive save-time |
| `midEdit` | Editor `didChange` after driver-side debounce | Unsaved editor buffer | Advisory diagnostics; cannot refuse the user's keystroke | Interactive buffer |
| `preWrite` | Agent / MCP tool proposes a write before it reaches disk | Proposed tool-call content | May warn or refuse the write according to project config | Interactive buffer |
| `watch` | Filesystem watcher observes a changed file outside an explicit driver request | On-disk file content | Background diagnostics / enforcement according to daemon policy | Background |

Future modes must declare the same four fields. If a future path uses the same
trigger, content source, and enforcement semantics as an existing mode, it must
reuse the existing mode label and add a `surface` dimension instead of creating a
new mode.

### Timing boundaries

All measurements must use one of these boundaries.

| Boundary | Start | End | Purpose |
| --- | --- | --- | --- |
| `validation.service` | Daemon has accepted a complete validation request | Daemon has produced the response payload | Measures the daemon validation service: request decode, content handling, rule evaluation, and response construction. |
| `validation.roundtrip` | Driver sends the validation request | Driver receives the validation response | Measures what an interactive driver experiences over the real transport. This is the primary SLO boundary. |
| `validation.visible` | User- or agent-observable trigger occurs | Diagnostic is visible, or MCP/tool response is composed | Measures surface UX. Report when useful, but each surface owns this number. It is not the global daemon SLO. |

Driver-side debounce is not part of `validation.roundtrip` or
`validation.service`. Debounce is a cooperative quieting interval before a
request exists. If a surface wants to measure from raw keystroke to visible
diagnostic, it reports `validation.visible` and records the debounce window as a
dimension.

Implementations may record spans such as transport, decode, content read, rule
evaluation, serialisation, and render time. Those spans are diagnostic evidence,
not separate product requirements unless a specific follow-up task makes them
one.

### Required dimensions

Every recorded measurement must include these dimensions:

| Dimension | Examples |
| --- | --- |
| `mode` | `save`, `midEdit`, `preWrite`, `watch` |
| `boundary` | `validation.service`, `validation.roundtrip`, `validation.visible` |
| `surface` | `vscode`, `mcp`, `daemon-watch`, `fake-driver`, `cli-harness` |
| `contentSource` | `disk`, `buffer`, `tool-proposal` |
| `ruleSet` | `default-v1`, `secret-only`, `reasoning-ai-001`, `custom` |
| `fixtureCorpus` | `latency-corpus-v1`, `user-sample`, `synthetic-spike` |
| `contentSize` | bytes, plus any configured size-cap marker |
| `platform` | Linux, macOS, Windows, CI runner class where relevant |
| `daemonState` | `warm`, `cold`, `restarted` |
| `driverProtocol` | protocol or contract version when a driver is involved |
| `debounceMs` | present for debounced surfaces; `0` or omitted for non-debounced paths |

Measurements without these dimensions are exploratory notes, not evidence for a
latency claim.

### Interactive SLOs

The shared requirement is p95 latency on a warm daemon. p50 and p99 must be
reported, but p95 is the pass/fail SLO unless an owning module explicitly adds a
stricter gate.

| Budget class | Applies to | `validation.service` p95 | `validation.roundtrip` p95 |
| --- | --- | --- | --- |
| Interactive buffer | `midEdit`, `preWrite` | 50 ms | 80 ms |
| Interactive save-time | `save` | 80 ms | 120 ms |
| Background | `watch` and non-interactive batch paths | Report only | Report only |

Cold-start measurements are reported separately and must not be mixed into warm
percentiles. If cold-start becomes a launch-blocking product requirement, it gets
its own ADR or module task rather than being hidden inside this rubric.

Background validation still matters, but it is not judged by the interactive
latency SLO. Background paths should instead be evaluated by throughput,
fairness, and whether they starve interactive requests.

### Corpus and harness requirements

The owning benchmark tasks must provide a versioned fixture corpus and at least
two harness perspectives:

1. a daemon/service harness for `validation.service`, and
2. a driver/transport harness for `validation.roundtrip` where a real driver or
   `DriverClient` path exists.

The canonical corpus should be versioned by directory or identifier, for
example `latency-corpus-v1`, and should cover at minimum:

- empty content,
- small representative content,
- medium representative source content,
- near-cap content at the configured validation size limit,
- binary or binary-like content that should short-circuit content rules, and
- Unicode-heavy content.

Exact fixture files, benchmark commands, and baseline locations are owned by
INTD-014 / RTAI-003 and related implementation tasks. Future corpus revisions
must be additive or versioned so historical measurements remain comparable.

### Regression policy

CI and release gates should use this rule:

- Interactive `validation.roundtrip` p95 over the canonical warm corpus must not
  exceed the SLO for its budget class.
- `validation.service` p95 must be recorded for the same run so regressions can
  be attributed to daemon work versus driver / transport work.
- p50 and p99 are reported for context and tail-risk review.
- Baseline-relative regression thresholds may be stricter than the SLO, but they
  are implementation policy owned by the benchmark task.

This keeps the product requirement simple while still allowing INTD, DRVR, and
RTAI to add stricter local regression gates when useful.

### Rules for future intercept paths

Any future intercept-backed validation path must answer these questions before
it gets a bespoke implementation:

1. Can it reuse an existing mode label with a new `surface` value?
2. Does it use the shared diagnostic envelope and rule registry?
3. Does it need `validation.service`, `validation.roundtrip`,
   `validation.visible`, or all three?
4. Is it interactive enough to inherit an existing SLO class?
5. If it needs a new SLO class, what user- or agent-perception constraint makes
   the existing classes wrong?

The default answer should be reuse. New modes and new budgets require evidence,
not taste.

## Consequences

- **Positive:**
  - INTD, DRVR, RTAI, and future intercept users share one latency vocabulary.
  - Measurement claims become comparable because mode, boundary, surface,
    content source, rule set, corpus, and daemon state are explicit.
  - Save-time, mid-edit, MCP pre-write, and watcher validation reuse the same
    daemon/rule/diagnostic pattern instead of growing separate pipelines.
  - The SLO stays simple: warm p95 at the round-trip boundary for interactive
    paths.
- **Negative:**
  - Existing module text that quotes local numbers must be rewritten to cite this
    ADR.
  - Some detail from the old ADR moves into implementation tasks, so reviewers
    must check INTD-014 / RTAI-003 for exact fixture and harness coverage.
  - `validation.visible` is intentionally surface-owned; it will vary more than
    daemon-owned measurements.
- **Risks:**
  - A surface may report a good `validation.roundtrip` while still feeling slow
    because debounce or rendering dominates `validation.visible`. Mitigation:
    surface tasks must report `validation.visible` when making UX claims.
  - A future mode may be named unnecessarily. Mitigation: the future-mode rules
    require reuse unless trigger, content source, or enforcement semantics differ.
  - Background validation could starve interactive work while still being outside
    the SLO. Mitigation: daemon scheduling and rate-limit tasks must test
    interactive fairness separately from this latency rubric.

## Follow-up Work

- INTD-014 should use `validation.service` and `validation.roundtrip` when it
  adds JSON-RPC conformance and latency benchmarks.
- DRVR-002 should delete local save-time latency language and cite this ADR's
  `save` mode and interactive save-time SLO.
- RTAI-002 / RTAI-003 should cite this ADR's `midEdit` mode, interactive buffer
  SLO, and required dimensions.
- RTAI-006 should cite this ADR's `preWrite` mode instead of treating MCP
  pre-write validation as an unnamed variant of mid-edit.
- Watcher/background validation tasks should report measurements but should not
  claim the interactive SLO unless they become user-blocking.

## References

- Related ADRs: [ADR-015](./015-intercept-loop-enforcement.md) (intercept loop
  authority and daemon enforcement), [ADR-030](./030-surface-drivers-supersede-napi-cutover.md)
  (drivers-on-daemon architecture).
- APS modules:
  - [intercept-daemon](../archive/modules/intercept-daemon.aps.md) — INTD-014 owns the
    daemon JSON-RPC conformance and latency benchmark implementation.
  - [surface-drivers](../archive/modules/surface-drivers.aps.md) — DRVR-002 owns the
    editor-driver protocol and should cite this ADR for save-time latency
    vocabulary.
  - [realtime-ai-validation](../modules/realtime-ai-validation.aps.md) —
    RTAI-002 / RTAI-003 / RTAI-006 own mid-edit and pre-write implementation and
    benchmarking.
- Design specs:
  [editor-and-mcp-driver-design](../specs/anvil-driver-framework/editor-and-mcp-driver-design.md)
  §3.4 contains the legacy local save-time number this ADR replaces.
