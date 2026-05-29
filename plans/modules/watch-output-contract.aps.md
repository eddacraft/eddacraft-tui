# Watch Output Contract

| ID   | Owner  | Status | Progress |
| ---- | ------ | ------ | -------- |
| WOUT | @aneki | Done   | 6/6      |

**Last reviewed:** 2026-05-19 (final release sweep: PR #1554 is **Merged** in the
lifecycle narrative — schema `Status: Done` per `plans/aps-rules.md` §"Module
Schema Status Values"; WOUT rides `v0.7.0-beta` as developer-facing release
freight and advances to Released/Shipped when release evidence lands.)

Earlier 2026-05-14:

**Execution authorisation:** operator `@aneki` authorised this module for
implementation on 2026-05-14 via `/goal complete WOUT using /dev-workflow`.
Status advanced from Proposed → In Progress per `plans/aps-rules.md` rule 1
(operator approval for Proposed work recorded inline).

## Purpose

Make `anvil --json watch` safe for downstream consumers to pipe into their own
systems. The current implementation already emits one JSON object per watch
event on stdout, but the payload contract is not yet strong enough for external
integrations: `detail` is a debug-formatted string, stdout/stderr discipline is
partly implicit, and there is no documented versioning or compatibility policy.

WOUT turns the existing JSON-lines behaviour into a durable consumer surface:

- stdout is reserved for newline-delimited JSON event records in JSON mode
- every event has a versioned schema and structured payload
- stderr carries diagnostics, warnings, and human guidance that is not part of
  the data stream
- action dispatch behaviour is explicit so child processes cannot corrupt the
  event stream
- tests prove consumers can parse the stream from a real `anvil watch` process

## In Scope

- Versioned NDJSON event envelope for `anvil --json watch`
- Structured payloads for progress, snapshot, violation, error, and action
  result events
- stdout/stderr ownership rules for watch JSON mode
- Consumer-facing docs and examples for `jq`, shell pipelines, and simple
  long-running readers
- Compatibility fixtures that pin representative event lines
- Integration tests that spawn `anvil --json watch`, mutate a fixture workspace,
  and parse emitted lines

## Out of Scope

- Bidirectional stdin command/control protocol for `anvil watch`
- MCP protocol changes
- Daemon notification fan-out or hosted event streaming
- Graph v2 persistence
- Dashboard-specific rendering
- Non-watch command JSON contracts except where shared helpers are needed

## Interfaces

- **Depends on:**
  - `crates/anvil-cli/src/commands/watch.rs`
  - `crates/anvil-kernel/src/watch.rs`
  - `crates/anvil-kernel-types/src/*`
  - `crates/anvil-tui/src/surfaces/watch/*` for action result shape alignment
  - WATCHUX startup/fallback semantics
- **Exposes:**
  - a documented `anvil.watch.event.v1` NDJSON contract
  - fixture-backed JSON examples for consumers
  - parseable stdout behaviour for piped integrations

## Work Items

### WOUT-001: Watch JSON Contract Spec

- **Intent:** Define the consumer-facing contract before changing the event
  stream shape.
- **Expected Outcome:** A spec records the NDJSON framing rule, stdout/stderr
  ownership, schema versioning, event kinds, payload fields, compatibility
  policy, and intentional non-goals such as stdin control.
- **Files:** `docs/specs/watch-output-contract.md`,
  `docs/public/anvil/integrations/watch-output.md`
- **Validation:** `pnpm docs:check` — passed 2026-05-14 (7/7 surfaces)
- **Status:** Done
- **changeType:** docs
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: developer
  - type: added
  - text: `anvil --json watch` gains a documented NDJSON contract for
    downstream consumers.

### WOUT-002: Typed Watch Event Envelope

- **Intent:** Replace the current debug-string `detail` field with a structured,
  versioned payload that consumers can parse without depending on Rust debug
  formatting.
- **Expected Outcome:** Watch JSON mode emits `schemaVersion`, `timestamp`,
  `eventType`, and `payload` fields. Payload variants cover progress, snapshot,
  violation, error, and action result events with stable names and optional
  extension fields.
- **Files:** `crates/anvil-kernel-types/src/*`,
  `crates/anvil-cli/src/commands/watch.rs`
- **Validation:** `cargo test -p eddacraft-anvil commands::watch::tests::watch_event_serialises_to_json`
  and fixture assertions for each event variant — passed 2026-05-14 (1 + 8
  kernel-types serde tests; full `commands::watch::` module green)
- **Status:** Done
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor

### WOUT-003: JSON-Mode Stdout Discipline

- **Intent:** Make stdout safe to pipe by ensuring only NDJSON event records are
  written there while watch runs in JSON mode.
- **Expected Outcome:** Startup guidance, bare-exclude warnings, action child
  diagnostics, watcher setup errors, and shutdown notices are routed to stderr
  or encoded as explicit event records. Child `stdout` cannot interleave with
  watch event lines.
- **Files:** `crates/anvil-cli/src/commands/watch.rs`,
  `crates/anvil-cli/src/main.rs`
- **Validation:** Unit tests for warning routing plus an integration smoke that
  asserts every stdout line parses as JSON while stderr may contain human text
  — unit tests added 2026-05-14 (`watch_output_mode_*`,
  `warning_channel_for_advisory_*`, `child_stdio_policy_*`,
  `format_bare_exclude_warning_*`); integration smoke covered by WOUT-004
- **Status:** Done
- **changeType:** fix
- **releaseIntent:** candidate
- **releaseScope:** minor

### WOUT-004: Consumer Pipe Integration Harness

- **Intent:** Prove the contract from a consumer's point of view, not just by
  unit-testing serialisation helpers.
- **Expected Outcome:** Tests spawn `anvil --json watch` in a fixture workspace,
  apply a file change, read stdout as NDJSON, and assert the expected event
  sequence without relying on terminal behaviour.
- **Files:** `crates/anvil-cli/tests/watch_json_output.rs`,
  `crates/anvil-cli/tests/fixtures/watch-json/*`
- **Validation:** `cargo test -p eddacraft-anvil --test watch_json_output`
  — passed 2026-05-14 (2/2: initial snapshot via real spawn,
  bare-exclude warning routes to stderr)
- **Status:** Done
- **changeType:** internal
- **releaseIntent:** candidate
- **releaseScope:** minor

### WOUT-005: Compatibility Fixtures and Drift Guard

- **Intent:** Prevent accidental breaking changes to event names or required
  fields once the contract is published.
- **Expected Outcome:** Golden event fixtures live in the repo, schema checks
  reject missing required fields, and docs examples are generated from or checked
  against those fixtures.
- **Files:** `crates/anvil-cli/tests/fixtures/watch-json/*.jsonl`,
  `docs/public/anvil/integrations/watch-output.md`, optional schema under
  `packages/anvil/contracts` if the project chooses to publish one there
- **Validation:** `cargo test -p eddacraft-anvil --test watch_json_output` and
  `pnpm docs:check` — both green 2026-05-14 (8/8 watch_json_output tests
  including fixture+docs alignment; docs:check 7/7 surfaces). JSON Schema
  export remains deferred per Open Question 1.
- **Status:** Done
- **changeType:** internal
- **releaseIntent:** candidate
- **releaseScope:** minor

### WOUT-006: Consumer Documentation and Migration Note

- **Intent:** Show users how to consume the stream correctly and name the limits
  of the contract.
- **Expected Outcome:** Public docs include examples for `jq`, shell loops, and a
  small long-running reader; docs explain that stdout is NDJSON, stderr is
  diagnostics, records are append-only compatible within v1, and stdin control is
  not supported yet.
- **Files:** `docs/public/anvil/integrations/watch-output.md`,
  `docs/public/anvil/tutorials/ci.md`,
  `docs/public/anvil/releases/changelog.md`
- **Validation:** `pnpm docs:check` — passed 2026-05-14 (7/7 surfaces)
- **Status:** Done
- **changeType:** docs
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: developer
  - type: added
  - text: Consumers can pipe `anvil --json watch` as stable NDJSON.

## Sequencing

1. **Contract first:** WOUT-001 defines what consumers can rely on.
2. **Shape the stream:** WOUT-002 and WOUT-003 make stdout structurally safe.
3. **Prove the pipe:** WOUT-004 validates the spawned-process consumer path.
4. **Guard drift:** WOUT-005 pins fixtures and docs examples.
5. **Teach usage:** WOUT-006 publishes examples and migration notes.

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Consumers depend on current `detail` strings | Medium | Medium | Treat current shape as pre-contract; document the v1 migration clearly |
| Watch output blocks on slow consumers | Low | Medium | Keep stdout writes line-oriented and document reader back-pressure expectations |
| Child action output corrupts JSON mode | Medium | High | WOUT-003 owns child stdout suppression / stderr routing tests |
| Schema overfits current kernel payloads | Medium | Medium | Use explicit event variants with optional extension fields |

## Open Questions

1. Should `anvil.watch.event.v1` live only in Rust types, or also publish a JSON
   Schema under `packages/anvil/contracts` for non-Rust consumers?
2. Should JSON mode emit a terminal `shutdown` / `stopped` event on Ctrl-C, or is
   EOF the only stream termination signal for v1?
3. Should action child `stderr` be inherited in JSON mode, or should it become an
   explicit `action_output` event in a later version?
