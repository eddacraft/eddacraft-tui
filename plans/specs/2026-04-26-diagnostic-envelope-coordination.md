# Diagnostic Envelope Coordination

## Purpose

Coordination spec for the **single canonical diagnostic shape** that four
in-flight work items each name and risk diverging on:

- **AIGUARD-002** — JSON diagnostic schema for AI tools consuming
  `anvil gate --profile ai`. Owns the canonical type at
  `crates/anvil-kernel-types/src/diagnostics.rs`.
- **RTAI-007** — mid-edit telemetry mirror (broadcast form: daemon →
  many telemetry subscribers).
- **INTD-013** — daemon control envelope mirroring enforcement decisions
  onto telemetry.
- **DRVR-002** — editor-driver and MCP-driver protocol over JSON-RPC.

If each ships its own envelope, downstream consumers (editors, AI tools,
CI) branch. Council A and Council E both flagged envelope coordination as
a launch-blocker prerequisite. This spec pins the inner `Diagnostic`
shape that all four reuse, and disambiguates the three **outer wrapper
shapes** (return-value, broadcast, control) that wrap the same payload
on different transports.

It complements (does not replace):

- `plans/specs/2026-04-22-notification-telemetry-stream-contract.md`
  — telemetry-lane envelope (`anvil.notification.v1`).
- `plans/specs/2026-04-22-intercept-notification-integration.md`
  — intercept control-to-notification mapping.
- `plans/decisions/ADR-031` (drafting in parallel) — latency rubric
  for save-time vs mid-edit vs gate paths. Referenced where budgets
  appear.

## Principle

> One **`Diagnostic`** payload. Many envelopes.

A diagnostic is a *finding produced by a rule* — a cross-layer import,
a leaked secret, a reasoning-pattern violation. Diagnostics are the
same logical thing whether they arrive at an AI tool via
`anvil gate --profile ai` (CLI return value), at a telemetry
subscriber via the daemon (broadcast), or at an editor via JSON-RPC
notification (control). What differs is the **outer envelope**:

- Return-value form (AIGUARD-002): one CLI invocation → one bounded
  response, exit code, single consumer.
- Broadcast form (RTAI-007): mid-edit decision → fan-out to N
  subscribers on the telemetry lane.
- Control form (INTD-013, DRVR-002): daemon → one driver, ack-driven,
  bounded round-trip.

The inner shape is identical. The outer shape is dictated by transport
and consumer expectations.

## Canonical Inner Shape: `Diagnostic`

### Location

`crates/anvil-kernel-types/src/diagnostics.rs`. AIGUARD-002 already
targets this path; whichever module lands first publishes the type and
the others import it.

### Fields

```json
{
  "schema_version": "anvil.diagnostic.v1",
  "id": "diag_01HW8K6Q4P0X7N9TJ4YA3S0V",
  "severity": "error",
  "summary": "Hardcoded API key detected",
  "location": {
    "file": "src/api/client.ts",
    "line": 42,
    "column": 18,
    "end_line": 42,
    "end_column": 47
  },
  "category": "secret",
  "source": {
    "rule_id": "secret-aws-access-key",
    "source_module": "anvil-checks::secrets"
  },
  "remediation_hint": "Move to environment variable; see docs/guides/secrets.md",
  "mode": "save-time"
}
```

| Field | Type | Required | Purpose |
| --- | --- | :---: | --- |
| `schema_version` | string | yes | Inner-shape version. Current value: `anvil.diagnostic.v1`. Distinct from any outer envelope `schema` field. |
| `id` | string | yes | ULID minted by the producing rule run. Stable across the same rule firing on the same file revision; new id when content or rule changes. |
| `severity` | enum | yes | One of `info`, `warning`, `error`. Distinct from the *control decision* (`allow`/`warn`/`block`/`interrupt`) — severity is the rule's view; control decisions are the daemon's view. |
| `summary` | string | yes | One-line human-readable headline. ≤ 200 chars. |
| `location` | object | yes | File anchor. `file` is required; `line`/`column` are 1-based; `end_line`/`end_column` optional. For deleted-file or path-only rules, `line` may be `null`. |
| `category` | enum | yes | Coarse grouping for routing/filtering: `secret`, `antipattern`, `boundary`, `policy`, `reasoning`, `command-safety`, `architecture`, `other`. New values require a spec amendment (see Versioning). |
| `source` | object | yes | Provenance. `rule_id` uniquely identifies the rule across Anvil; `source_module` is the crate or sub-module that produced it (e.g. `anvil-checks::secrets`). |
| `remediation_hint` | string | optional | Free-text actionable hint. Omit when no useful guidance exists rather than emitting a generic placeholder. |
| `mode` | enum | yes | Mode discriminator: `save-time`, `mid-edit`, `pre-write`, `gate`, `watch`. See "Mode Discriminator Semantics" below. |

### Rust shape

```rust
pub struct Diagnostic {
    pub schema_version: SchemaVersion,    // const `anvil.diagnostic.v1`
    pub id: DiagnosticId,                 // ULID newtype
    pub severity: Severity,               // Info | Warning | Error
    pub summary: String,
    pub location: Location,
    pub category: Category,
    pub source: DiagnosticSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation_hint: Option<String>,
    pub mode: Mode,                       // SaveTime | MidEdit | PreWrite | Gate | Watch
}
```

`Severity` and the control-decision enum from `notifications.rs` are
**not** the same type and must not be cross-cast. INTD-013 maps
diagnostic severity → control decision per project enforcement
configuration; that mapping is the daemon's job, not the
diagnostic's.

## Mode Discriminator Semantics

`mode` is the single field consumers branch on to know which path
produced the diagnostic and what the consumer expectation is.

| Mode | When it fires | Producer | Consumer expectation |
| --- | --- | --- | --- |
| `save-time` | After file write hits disk; daemon scans; diagnostic emitted via DRVR-002 protocol. | Intercept daemon (post-write). | Editor renders persistent diagnostic anchored to on-disk content. Suppression UI applies. |
| `mid-edit` | Before file write; editor driver sends an in-flight buffer after `didChange` debounce. | Intercept daemon (didChange). | Editor renders ephemeral advisory diagnostic with `phase: midEdit` marker. Latency maps to ADR-031 `mode = midEdit`. |
| `pre-write` | Before an agent / MCP write reaches disk; driver sends proposed tool-call content. | Intercept daemon (pre-write tool interception). | MCP driver may warn or block the tool call per project config. Latency maps to ADR-031 `mode = preWrite`. |
| `gate` | `anvil gate` invocation (any profile, including `--profile ai`). | CLI gate command. | Single consumer reads structured JSON return value, decides exit code consequences. No persistent rendering required. |
| `watch` | File-system watch loop emits without an attached driver session. | Watcher (LAUNCH module). | Streaming consumer (TUI, dashboard) renders transient notification. May coexist with `save-time` from a driver-attached session. |

A diagnostic produced inside one path MUST set `mode` to that path's
value. A consumer that doesn't recognise a mode value MUST surface the
diagnostic anyway, defaulting to "treat as informational" rather than
silently dropping. This is the same forward-compat rule the telemetry
contract uses for unknown `class` values.

## Outer Wrapper Shapes

The same `Diagnostic` payload travels in three distinct outer
envelopes depending on direction and transport.

### 1. Return-value form (AIGUARD-002)

CLI invocation produces a bounded response. One consumer (the AI tool
or CI). Read once.

```json
{
  "schema": "anvil.gate-result.v1",
  "exit_code": 1,
  "summary": {
    "total": 3,
    "by_severity": { "error": 1, "warning": 2, "info": 0 },
    "by_category": { "secret": 1, "antipattern": 2 }
  },
  "diagnostics": [
    { /* canonical Diagnostic, mode: "gate" */ },
    { /* canonical Diagnostic, mode: "gate" */ },
    { /* canonical Diagnostic, mode: "gate" */ }
  ]
}
```

- Owner: `crates/anvil-cli/src/commands/gate.rs` (output::json).
- Bounded: a single response, complete on stdout close.
- All `diagnostics[].mode` values MUST be `gate`.
- `summary` is a convenience for AI tools / CI; it derives from
  `diagnostics[]` and MUST agree with it.

### 2. Broadcast form (RTAI-007, INTD-013 telemetry mirror)

Daemon emits one event per decision. Many subscribers (TUI,
dashboard, observability sink). Lossy.

The outer envelope is `anvil.notification.v1` (already pinned in
`2026-04-22-notification-telemetry-stream-contract.md`). The
diagnostic rides inside it via a new optional `diagnostics` array
on the notification:

```json
{
  "schema": "anvil.notification.v1",
  "producer_instance_id": "pi_01HW...",
  "seq": 142,
  "timestamp": "2026-04-26T10:00:00.000Z",
  "correlation": {
    "session_id": "sess_01HW...",
    "worktree": "feat/x",
    "source": "intercept"
  },
  "notification": {
    "class": "finding",
    "priority": "high",
    "title": "src/api/client.ts",
    "message": "Hardcoded API key detected"
  },
  "mirror": {
    "decision": "warn",
    "driver": "intercept-daemon-v1",
    "ack_required": false,
    "path": "midEdit"
  },
  "diagnostics": [
    { /* canonical Diagnostic, mode: "mid-edit" or "save-time" */ }
  ]
}
```

- Owner: `crates/anvil-intercept/src/telemetry.rs`.
- `mirror.path` (added by RTAI-007) takes the values
  `saveTime` | `midEdit` to let subscribers split paths without
  parsing diagnostic bodies. `path` is **outer-envelope** metadata;
  `diagnostics[].mode` is **inner-shape** metadata. They are
  redundant by design (mode is per-diagnostic, path is
  per-event) and MUST agree when both are present.
- Lossy-tolerant: subscribers MAY drop events under backpressure
  per the telemetry contract. Producers SHOULD keep events with
  non-empty `diagnostics` above `info`/`progress` events when
  shedding load.
- Cross-session redaction (INTD-015) applies to diagnostic bodies
  the same way it applies to notification context: subscribers not
  authorised for the originating session see redacted excerpts
  (rule_id + hash of file path) instead of full `summary` /
  `location.file`.

### 3. Control form (DRVR-002, INTD-013 control lane)

Daemon ↔ driver, JSON-RPC 2.0 over NDJSON. Single consumer per
message. Ack-driven.

Diagnostics arrive at editors via a JSON-RPC notification:

```json
{
  "jsonrpc": "2.0",
  "method": "anvil/publishDiagnostics",
  "params": {
    "uri": "file:///workspace/src/api/client.ts",
    "version": 17,
    "diagnostics": [
      { /* canonical Diagnostic, mode: "save-time" or "mid-edit" */ }
    ]
  }
}
```

And as the response payload of the mid-edit RPC:

```json
{
  "jsonrpc": "2.0",
  "id": "req-42",
  "result": {
    "version": 17,
    "diagnostics": [
      { /* canonical Diagnostic, mode: "mid-edit" */ }
    ],
    "truncated": false
  }
}
```

- Owner: `plans/specs/anvil-driver-framework/editor-and-mcp-driver-design.md`
  (DRVR-002 protocol definition).
- The JSON-RPC envelope (`jsonrpc`, `method`, `id`, `params`,
  `result`, `error`) is pinned by JSON-RPC 2.0 conformance
  (INTD-014). This spec does not re-litigate it.
- Errors-as-first-class (RTAI-008) belongs at the JSON-RPC envelope
  level (`error: { code, message, data }`), not inside the
  diagnostics array. An empty `diagnostics: []` means "rule set ran,
  found nothing"; a structured `error` means "the run itself
  failed".
- The mid-edit response echoes `version` so clients can drop stale
  replies, and sets `truncated` when the daemon capped the diagnostic
  set for a single scan.
- The mapping from `Diagnostic.severity` to control decision
  (`allow`/`warn`/`block`/`interrupt`) is performed by INTD-013 per
  the project's enforcement config. Drivers must not infer it from
  severity alone.

## Versioning

The inner `Diagnostic` shape and the three outer envelopes evolve
**independently**. Each outer envelope already has its own version
field; `Diagnostic` carries `schema_version` (`anvil.diagnostic.v1`).

Rules for the inner shape:

- **Stays on `anvil.diagnostic.v1`:**
  - Adding a new optional field with `skip_serializing_if = "Option::is_none"`.
  - Adding a new value to `category` provided it is declared in this
    spec's Category list before producers emit it.
  - Adding a new value to `mode` provided it is declared in the
    Mode Discriminator Semantics table before producers emit it.
- **Bumps to `anvil.diagnostic.v2`:**
  - Removing or renaming a field.
  - Changing a field type.
  - Removing or renaming a `severity`, `category`, or `mode` value.
  - Tightening a previously optional field to required.
- Producers MAY emit both versions during a transition window;
  consumers MUST accept either when both are declared in
  `schema_version`.

Backwards-compat for AI tools is the load-bearing constraint. The
`anvil gate --profile ai` JSON contract is what external AI tools
parse; a v2 bump there breaks every consumer. We minimise bumps by
adding rather than rewriting; we accept that
`anvil.gate-result.v1` and `anvil.diagnostic.v1` may drift in
version once if forced.

Subscribers MUST treat unknown `severity` values as `warning`,
unknown `category` values as `other`, and unknown `mode` values as
"render and pass through" rather than dropping. Implemented by the
`#[serde(other)] Unknown` arm on `Severity`/`Category` and the
`Known | Unknown(String)` shape on `Mode` in
`anvil_kernel_types::diagnostics` (ADR-096); an unrecognised value
deserialises to the tolerant variant instead of failing the whole
`Diagnostic` parse.

## Coordination Rules

This spec exists because four work items name the same envelope.
Concretely:

1. **AIGUARD-002 lands the canonical type.** It already targets
   `crates/anvil-kernel-types/src/diagnostics.rs`. Whichever of the
   four work items reaches implementation first publishes the
   `Diagnostic` type and the `anvil.diagnostic.v1` schema version
   constant.
2. **RTAI-007 imports, does not redefine.** The mid-edit telemetry
   mirror MUST consume `kernel_types::Diagnostic` and add only the
   outer-envelope `mirror.path = "midEdit"` discriminator. RTAI-007
   does not introduce a parallel diagnostic struct.
3. **INTD-013 imports, does not redefine.** Telemetry mirror events
   that carry diagnostics use the same type. The control-decision
   mapping (`allow`/`warn`/`block`/`interrupt`) lives in INTD-013
   and is **not** a field on `Diagnostic` — it's an outer-envelope
   field on the notification mirror.
4. **DRVR-002 imports, does not redefine.** The editor-driver
   protocol's `anvil/publishDiagnostics` and `scan_buffer`
   methods use the same payload type. The shared TS/Rust contracts
   package generates TS bindings from the canonical Rust struct;
   editors and the MCP driver consume those bindings, not bespoke
   shapes.
5. **Cross-module test fixture.** AIGUARD-002 ships the round-trip
   serde test; RTAI-008 (errors-as-first-class contract test) MUST
   include a fixture exercising the inner shape's round-trip
   parity across all three outer envelopes. CI fails if any of the
   four consumers drift.

## Out of Scope

- Defining every event the daemon emits. This spec covers the
  diagnostic shape only; health events, fence transitions, progress
  events, and session-lifecycle events stay in their owning specs.
- JSON-RPC version, error code allocation, batch behaviour. Owned
  by INTD-014 (conformance) and per-module concerns.
- Suppression UX, code actions, fix payloads. Diagnostics carry
  `remediation_hint`; the decision to act on it is the consumer's.
- Reasoning-pattern catalogue (AI-001..AI-007). Those are rule
  implementations that emit diagnostics; how the catalogue is
  organised lives in anvil-checks, not here.
- Latency budgets per mode. Owned by ADR-031 (drafting in
  parallel). This spec references that ADR for budget numbers but
  does not pin them.

## Open Questions

1. **Should `category` be open-ended or closed?** ~~Closed (current
   draft) gives subscribers a fixed routing table; open lets new
   rule families ship without spec amendments. The closed list
   mirrors how `NotificationClass` works today, which has held up
   under change. Defaulting to closed; reopen if rule-family churn
   proves it wrong.~~ **Resolved (ADR-096, #2243): open at the
   consumer.** `Category` (and `Severity`) carry a `#[serde(other)]
   Unknown` arm, so an unrecognised value deserialises to `Unknown`
   and is routed as `other`/treated as `warning` rather than failing
   the parse — implementing the MUST above. The Category list here
   remains the declared set producers emit from; the tolerance is a
   consumer-side forward-compat guarantee, not a licence to emit
   undeclared categories.
2. **Should `id` be a ULID or a hash?** ULID gives global
   uniqueness and trivial sort-by-time; a content hash
   (rule_id + file + line + content fragment) gives natural
   deduplication. AIGUARD-002 picks one; this spec is agnostic so
   long as the choice is documented and stable.
3. **Does `mode = "watch"` justify its own outer envelope?** The
   watch loop today emits via the telemetry envelope, same as
   intercept. Keeping mode as a discriminator field rather than
   spawning a fourth wrapper keeps consumers simple. Revisit if
   LAUNCH lands a watch-specific consumer that demands more.

## Recommendation

Adopt `anvil.diagnostic.v1` as the canonical inner shape, owned by
`crates/anvil-kernel-types/src/diagnostics.rs` per AIGUARD-002.
Wire RTAI-007, INTD-013, and DRVR-002 to import and reuse the type;
no module re-defines `Diagnostic`. Whichever of the four lands
first ships the struct; the others add their respective outer
envelopes around it without touching the inner shape.
