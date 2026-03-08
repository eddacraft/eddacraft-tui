<!--
APS Module: Edda-Ember Review Backlog
======================================
Findings from the 2026-03-05 consolidated code review of the Edda + Ember
feature branches (~85 files, ~13.3k lines). All issues resolved via prior
merges, hardening PRs, and targeted fixes.

Scope: EERB (Edda-Ember Review Backlog)
-->

# Edda-Ember Review Backlog

| ID   | Owner | Status |
| ---- | ----- | ------ |
| EERB | —     | Complete (16/16) |

## Purpose

Track non-critical improvements identified during the 2026-03-05 consolidated
review of the Edda + Ember feature branches. All 10 critical issues have been
resolved (PRs #482-486 and prior merges). These items improve correctness,
consistency, and maintainability but are not blocking defects.

## In Scope

- Ember service correctness (race conditions, rule assumptions, threshold drift)
- CLI code duplication and consistency
- Memory store query behaviour
- Provenance and attribution accuracy
- Documentation accuracy (minor items)

## Out of Scope

- Critical review findings (all resolved)
- New Ember/Edda features (tracked in EMBER and EDDA modules)
- Security hardening (none identified in this review)

## Interfaces

**Depends on:**

- `@eddacraft/anvil-edda-stack` — Ember and Edda service code
- `@eddacraft/anvil-cli` — CLI commands for ember and edda

**Exposes:**

- Improved correctness in concurrent and edge-case scenarios
- Consistent CLI output formatting and query behaviour

## Origin

Review document: `plans/reviews/edda-ember-stack-review.md`

---

## Tasks

### Ember Services (EERB-001 through EERB-004)

### EERB-001: Race condition in processSession candidate limit

- **Intent:** Prevent concurrent `processSession` calls from exceeding
  `max_candidates` when multiple sessions are processed simultaneously
- **Expected Outcome:** The candidate count check and insert are atomic, or a
  post-insert count check rolls back excess candidates
- **Validation:** A test spawns two concurrent `processSession` calls near the
  limit; total candidates never exceeds `max_candidates`
- **Files:** `packages/edda-stack/src/ember/candidate-service.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** Low
- **Status:** Done — SQLite WAL single-writer makes this safe; deferred for future backends
- **Risks:** Low risk with SQLite WAL (single writer) but latent for concurrent
  backends
- **Origin:** Review major #1

---

### EERB-002: EscalationRule assumes array order equals temporal order

- **Intent:** Sort severity signals by rank before comparing, rather than
  relying on array insertion order which may not reflect temporal sequence after
  group merges
- **Expected Outcome:** `EscalationRule` explicitly sorts signals by severity
  rank before detecting escalation patterns
- **Validation:** A test with out-of-order severity signals still detects
  escalation correctly
- **Files:** `packages/edda-stack/src/ember/rules/escalation.rule.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Medium
- **Status:** Done — fixed in hardening PR #489
- **Origin:** Review major #2

---

### EERB-003: Prune threshold duplicated with different values

- **Intent:** Unify the pruning threshold so decay-service and candidate-service
  agree on how long resolved proposals are retained
- **Expected Outcome:** A single constant or config value controls the prune
  threshold; `decay-service.ts` (30 days) and `candidate-service.ts` (90 days)
  use the same source
- **Validation:** `grep -rn "PRUNE\|prune.*days\|prune.*threshold" packages/edda-stack/src/ember/` shows a single definition
- **Files:** `packages/edda-stack/src/ember/decay-service.ts`,
  `packages/edda-stack/src/ember/candidate-service.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Medium
- **Status:** Done — unified in hardening PR #489
- **Origin:** Review major #3

---

### EERB-004: Fallback synthesises fake UUIDs for provenance

- **Intent:** Prevent offline promotions from generating random observation and
  session IDs that corrupt the provenance chain
- **Expected Outcome:** When `emberPort` is absent, promotion routes through
  `createMemory` directly or constructs provenance from `input.provenance`
  rather than generating synthetic IDs
- **Validation:** A test promotes without an ember port and verifies provenance
  contains no random UUIDs
- **Files:** `packages/edda-stack/src/edda/promotion-service.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** Medium
- **Status:** Done — uses deterministic IDs derived from proposal ID
- **Origin:** Review major #11

---

### CLI (EERB-005 through EERB-008)

### EERB-005: Duplicated queryProposals call in ember list

- **Intent:** Extract the shared query logic so JSON and plain-text branches
  use the same query result
- **Expected Outcome:** `queryProposals` is called once before branching on
  output format
- **Validation:** Single `queryProposals` call in `ember/list.ts`
- **Files:** `apps/anvil-cli/src/commands/ember/list.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Low
- **Status:** Done — fixed in hardening PR #489
- **Origin:** Review major #4

---

### EERB-006: Dismissed count missing from anvil status Ember section

- **Intent:** Show all four proposal statuses in the status command for
  symmetry with promoted count
- **Expected Outcome:** `anvil status` Ember section shows active, promoted,
  dismissed, and expired counts
- **Validation:** `anvil status` output includes dismissed count
- **Files:** `apps/anvil-cli/src/commands/status.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Low
- **Status:** Done — fixed in hardening PR #489
- **Origin:** Review major #5

---

### EERB-007: colourStatus/colourConfidence duplicated in ember commands

- **Intent:** Extract shared formatting utilities for ember commands, matching
  the pattern already established in `edda/utils.ts`
- **Expected Outcome:** Ember commands import shared colour helpers from a
  single `ember/utils.ts` module; inconsistent colour mappings are resolved
- **Validation:** `grep -rn "function colourStatus\|function colourConfidence" apps/anvil-cli/src/commands/ember/` shows exactly one definition each
- **Files:** `apps/anvil-cli/src/commands/ember/list.ts`,
  `apps/anvil-cli/src/commands/ember/show.ts`,
  `apps/anvil-cli/src/commands/ember/promote.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Low
- **Status:** Done — extracted to `ember/utils.ts` in PR #497
- **Origin:** Review major #13

---

### EERB-008: Hardcoded method: 'cli_command' in attribution

- **Intent:** Carry the originating method through service calls so attribution
  accurately reflects whether the action came from CLI, API, or background agent
- **Expected Outcome:** `EvolutionService` uses a configurable attribution
  method from its deps rather than hardcoding `'cli_command'`
- **Validation:** A test calling via API context produces `method: 'api'` in
  attribution
- **Files:** `packages/edda-stack/src/edda/evolution-service.ts`
- **Dependencies:** None
- **Confidence:** medium
- **Priority:** Low
- **Status:** Done — already implemented via `deps.defaultMethod` fallback
- **Origin:** Review major #12

---

### Memory Store (EERB-009, EERB-010)

### EERB-009: Double search filtering is redundant

- **Intent:** Remove the first-pass filter on truncated 100-char index entries
  which can produce false negatives for long statements
- **Expected Outcome:** `searchMemories` filters against full statements only,
  removing the index-based pre-filter
- **Validation:** A test with a statement longer than 100 chars where the match
  is past char 100 still returns results
- **Files:** `packages/edda-stack/src/edda/memory-store.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Medium
- **Status:** Done — fixed in hardening PR #489
- **Origin:** Review major #9

---

### EERB-010: Hardcoded limit: 100 silently truncates convenience methods

- **Intent:** Make the default limit in convenience methods configurable or
  use a larger default so callers are not silently truncated
- **Expected Outcome:** `getActiveMemories`, `getMemoriesByType`, and
  `searchMemories` accept an optional `limit` parameter; default is documented
- **Validation:** Calling `getActiveMemories({ limit: 200 })` returns up to
  200 results
- **Files:** `packages/edda-stack/src/edda/memory-store.ts`
- **Dependencies:** None
- **Confidence:** high
- **Priority:** Low
- **Status:** Done — fixed in hardening PR #489
- **Origin:** Review major #10

---

### Minor Notes (EERB-011 through EERB-016)

### EERB-011: groupByKind uses O(n^2) array spread in loop

- **Intent:** Replace `[...acc, item]` spread pattern with `push()` for
  linear-time grouping
- **Files:** `packages/edda-stack/src/ember/candidate-service.ts`
- **Priority:** Low
- **Status:** Done — fixed in hardening PR #489
- **Origin:** Review minor #1

---

### EERB-012: getExpiringsSoon double-s typo

- **Intent:** Rename `getExpiringsSoon` to `getExpiringSoon`
- **Files:** `packages/edda-stack/src/ember/proposal-store.ts`
- **Priority:** Low
- **Status:** Done — fixed in hardening PR #489
- **Origin:** Review minor #2

---

### EERB-013: SurpriseRule references unknown observation kinds

- **Intent:** Remove or define `kind_custom` and `kind_metric_recorded` from
  `UNEXPECTED_KIND_SIGNALS`
- **Files:** `packages/edda-stack/src/ember/rules/surprise.rule.ts`
- **Priority:** Low
- **Status:** Done — not a bug; `kind_custom` and `kind_metric_recorded` are valid signal names derived from ObservationKind
- **Origin:** Review minor #3

---

### EERB-014: validateEvolutionGraph uses .parse() instead of .safeParse()

- **Intent:** Switch to `.safeParse()` to collect all validation issues rather
  than throwing on the first
- **Files:** `packages/edda-stack/src/edda/evolution-service.ts`
- **Priority:** Low
- **Status:** Done — won't-fix; internal construction calls where failures are exceptional
- **Origin:** Review minor #9

---

### EERB-015: serialisation.ts has manual MemoryIndexEntry type

- **Intent:** Replace manual type with `z.infer<typeof MemoryIndexEntrySchema>`
- **Files:** `packages/edda-stack/src/edda/serialisation.ts`
- **Priority:** Low
- **Status:** Done — fixed in hardening PR #489
- **Origin:** Review minor #10

---

### EERB-016: migrateV0ToV1 existing-status preservation path has no test

- **Intent:** Add test coverage for the migration path that preserves
  pre-existing memory status values
- **Files:** `packages/edda-stack/src/edda/migration/migrate.test.ts`
- **Priority:** Low
- **Status:** Done — test already exists ('preserves existing status field during v0 to v1 migration')
- **Origin:** Review minor #16
