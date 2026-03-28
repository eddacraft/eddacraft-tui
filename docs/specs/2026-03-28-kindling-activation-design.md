# Kindling Activation — Design Spec

**Date:** 2026-03-28
**Status:** Approved
**Approach:** B — Full Vertical Slice (CLI wiring + Edda-Stack port)

## Summary

Activate the existing Kindling infrastructure so that every governance touchpoint
in Anvil emits structured observations to a local SQLite database. Surface this
data via `anvil status` and a dedicated `anvil kindling` subcommand. Implement
the Edda-Stack `IKindlingPort` so the memory pipeline can consume observations
when Ember intake is wired up.

## Design Constraints

- Use existing `@eddacraft/kindling-core` (v0.1.2), `kindling-store-sqlite`,
  and `kindling-provider-local` — all built and functional
- Use existing `kindling-integration` contracts, emitters, adapter, and bootstrap
- Per-project storage in `.anvil/kindling.db`
- Kindling enabled by default
- Fire-and-forget emission — Kindling failures never block CLI commands
- Any governance touchpoint gets observed: blocks, nudges, corrections, positive
  detections

## 1. Bootstrap & Lifecycle

A `KindlingContext` object is created during CLI startup:

1. Opens `.anvil/kindling.db` via existing `kindling-bootstrap.ts`
2. Constructs `SqliteKindlingStore` + `LocalFtsProvider` + `KindlingService`
3. Opens a session capsule with intent derived from the command name (e.g.
   `"gate run"`, `"watch start"`)
4. On command exit (success or failure), closes the capsule with a summary and
   shuts down the store
5. If Kindling is disabled in config (`enabled: false`), returns a no-op
   context — all emitters become silent
6. Pruning runs at session close via `pruneOldObservations()` — no separate
   scheduler needed

## 2. Emission Call Sites

### Gate evaluation (`anvil gate`, `anvil watch` per-cycle)

- `emitGateEvaluated` — full gate result: checks ran, pass/fail/warn per check,
  overall verdict, severity levels
- Captures blocks, nudges, and positive detections in one event

### Session lifecycle (all commands)

- `emitSessionStart` — command name, args, repo context
- `emitSessionEnd` — exit code, duration, summary

### Actions (`anvil fix`, suppressions, auto-corrections)

- `emitActionExecuted` — what Anvil did in response to a finding

### Constraints (suppression application, policy overrides)

- `emitConstraintApplied` — what was suppressed and why

### Errors

- `emitError` — unhandled or significant errors during governance operations

### Not in this phase

- Plan lifecycle emitters (`emitPlanCreated` etc.) — wire up when plan commands
  need observability

## 3. Edda-Stack IKindlingPort Implementation

New file: `packages/edda-stack/src/kindling/kindling-port-impl.ts`

Constructor takes a `KindlingService` instance (same one the CLI bootstrap
creates).

| Port Method                  | Delegates To                                         |
| ---------------------------- | ---------------------------------------------------- |
| `createObservation()`        | `service.appendObservation()`                        |
| `createObservationBatch()`   | Loop over `appendObservation()` in a transaction     |
| `getObservation()`           | `store.getObservationById()`                         |
| `queryObservations()`        | `store.queryObservations()` with scope/time filters  |
| `getSessionObservations()`   | `store.queryObservations({ sessionId })`             |
| `observationExists()`        | `getObservation() !== undefined`                     |
| `querySession()`             | `service.getCapsule()` + its observations            |
| `queryByPlan()`              | `store.queryObservations()` filtered by plan scope   |
| `getObservationsByTimeRange()` | `store.queryObservations(fromTs, toTs)`            |
| `getObservationsAsRefs()`    | Query + map to provenance reference format           |
| `isAvailable()`              | Check DB file exists and is readable                 |
| `countObservations()`        | Query with count                                     |
| `pruneObservations()`        | Retention-based deletion via store                   |

A thin mapping layer adapts between Edda-Stack's `Observation` type and Kindling
Core's.

`MockKindlingPort` goes in `packages/edda-stack/src/testing/mocks/` alongside
existing Ember and Edda mocks.

`getObservationsAsRefs()` is critical for the governance/provenance story — it
lets provenance chains link Edda memories back to raw operational events.

## 4. CLI Surface

### `anvil status` integration

Adds a "Kindling" section to existing status output:

```
Kindling      enabled · 2.4 MB · 847 observations
Last session  gate run · 12 min ago · 5 checks passed, 1 warning
```

Uses existing `getKindlingStatus()` from `kindling-integration/src/status.ts`.
If disabled or DB doesn't exist, shows `Kindling    disabled`.

### `anvil kindling` subcommand

- `anvil kindling sessions [--limit N]` — Lists recent session capsules
  (timestamp, command, duration, verdict). Default last 10.
- `anvil kindling query <session-id>` — All observations for a session in
  timeline order. Includes gate results, actions, errors.
- `anvil kindling export session <id>` — Writes to
  `.kindling/exports/session-<id>.json` (structured, committable).
- `anvil kindling export [--format json|jsonl] [--since <date>]` — Bulk export
  to stdout for piping/archival.

## 5. Configuration & Defaults

Lives in Edda-Stack config (`.anvil/config.yml` or equivalent):

```yaml
stack:
  kindling:
    enabled: true
    mode: "ephemeral"       # future: "persistent"
    capture:
      session_start: true
      session_end: true
      gate_evaluated: true
      action_executed: true
      constraint_applied: true
      human_input: true
      error: true
    retention:
      days: 90
    query:
      maxResults: 100
      maxPayloadBytes: 1048576
```

**Kindling defaults to enabled.** Users can disable with `enabled: false`.

**Pruning** runs at session close — piggybacks on normal CLI usage.

**Sensitive data** is redacted by `sensitive-data-validator.ts` in the service
layer before persistence.

### `.gitignore` defaults

- `.anvil/kindling.db*` — always ignored (binary SQLite)
- `.kindling/` — ignored by default

### Future: Persistence mode

- `mode: "ephemeral"` (default) — DB gitignored, pruned on retention schedule
- `mode: "persistent"` — exports auto-written to `.kindling/exports/` on session
  close, directory not gitignored, becomes committable audit trail
- `anvil kindling persist enable/disable` toggles the mode
- Not in first delivery — designed for, not built

## 6. Testing Strategy

### Unit tests

- `kindling-port-impl.test.ts` — all 14 port methods against in-memory SQLite
  (`:memory:`)
- Adapter mapping tests — Anvil kinds map correctly to Kindling Core kinds with
  provenance preserved
- Status integration test — output formats correctly with and without data

### Integration tests

- Full write-store-query cycle: bootstrap → emit → query back → verify integrity
- Session capsule lifecycle: open → emit gate + action + error → close → query
  timeline → verify ordering and completeness
- Retention pruning: insert old observations → prune → verify only recent data
  survives
- Sensitive data redaction: emit observation with API key → query back → verify
  redacted

### Mock for consumers

- `MockKindlingPort` in `packages/edda-stack/src/testing/mocks/` — returns
  canned data, records calls. Ember tests use this when observation intake is
  wired up.

## Dependencies

### Existing packages (no new deps)

- `@eddacraft/kindling-core` (0.1.2)
- `@eddacraft/kindling-store-sqlite` (0.1.2)
- `@eddacraft/kindling-provider-local` (0.1.2)
- `@eddacraft/anvil-kindling-integration` (0.1.0)
- `better-sqlite3` (already in edda-stack)

### Files to create

- `packages/edda-stack/src/kindling/kindling-port-impl.ts`
- `packages/edda-stack/src/kindling/kindling-port-impl.test.ts`
- `packages/edda-stack/src/testing/mocks/kindling.mock.ts`
- `apps/anvil-cli/src/commands/kindling.ts`
- `apps/anvil-cli/src/commands/kindling.test.ts`

### Files to modify

- `apps/anvil-cli/src/services/kindling-bootstrap.ts` — extend with lifecycle
  management
- `apps/anvil-cli/src/commands/gate.ts` — add emitter calls
- `apps/anvil-cli/src/commands/watch.ts` — add emitter calls (per-cycle gate
  evaluation)
- `apps/anvil-cli/src/commands/status.ts` — add Kindling section
- `packages/edda-stack/src/config.ts` — default Kindling to enabled
- `packages/edda-stack/src/contracts/ports/index.ts` — export port impl

## Out of Scope

- Ember observation intake (follow-on module)
- TUI Kindling surface (TUIDASH territory)
- `anvil kindling search` (FTS CLI surface)
- Cross-project aggregation
- Persistent mode toggle (`anvil kindling persist`)
- Plan lifecycle emitters
