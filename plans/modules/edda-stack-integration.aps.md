<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Edda Stack Integration

| Scope | Owner | Priority | Status      |
| ----- | ----- | -------- | ----------- |
| STACK | —     | medium   | In Progress |

## Purpose

The Edda Stack is a three-layer architecture that governs how activity becomes
memory. This module defines the integration points, shared contracts, and
cross-cutting concerns for Kindling → Ember → Edda data flow.

**The Stack Philosophy:**

```
Kindling observes — captures without judgement
Ember reflects — meaning without authority
Edda remembers — memory with restraint
```

**Problem:** Without explicit integration design:

- Layer boundaries blur (Ember writes to Edda directly)
- Schema drift causes silent data loss
- Provenance chains break across transitions
- AI behaviours vary unpredictably per layer
- Testing requires standing up the full stack

**Solution:** Stack integration provides:

- **Shared contracts**: Common types, consistent provenance model
- **Clear boundaries**: Explicit handoff protocols between layers
- **Schema strategy**: Decision on shared vs independent schemas
- **Testing utilities**: Per-layer mocking, integration fixtures
- **Configuration coherence**: Stack-wide settings, dependency checks

**Governing Rules:**

1. Kindling cannot judge (facts only)
2. Ember cannot decide (proposals only)
3. Edda cannot speculate (curated truths only)
4. Each layer is intentionally limited
5. Meaning emerges only through their separation

## In Scope

**Shared Contracts:**

- Common identifier formats (UUIDs, content hashes)
- Timestamp conventions (ISO8601 everywhere)
- Provenance link schema (cross-layer references)
- Type mappings (Ember ProposalType → Edda MemoryType)
- Confidence scale definitions

**Layer Boundaries:**

- Kindling → Ember: Observation event hooks
- Ember → Edda: Promotion protocol (human-gated)
- Edda → Kindling: Provenance resolution

**Schema Strategy:**

- Contracts package for shared types
- Per-layer storage schemas (optimised for purpose)
- Version compatibility guarantees

**Cross-cutting Concerns:**

- Stack-wide configuration schema
- Dependency health checks (`anvil stack status`)
- Migration coordination (schema changes across layers)
- Telemetry and observability

**Testing Utilities:**

- Per-layer mocks for isolation testing
- Integration test fixtures
- Provenance chain validation tools

## Out of Scope (v1)

- ❌ Cross-workspace stack federation
- ❌ Real-time synchronisation between layers
- ❌ Automatic schema migration across layers
- ❌ Stack-level access control (authentication)
- ❌ Distributed deployment topology
- ❌ Performance optimisation (caching, indexing)

## Interfaces

**Depends on:**

- `kindling-integration` — Observation layer
- `ember` — Candidate memory layer
- `edda` — Canonical memory layer

**Exposes:**

- `@eddacraft/anvil-edda-stack/contracts` — Shared type definitions
- `@eddacraft/anvil-edda-stack/ports` — Layer interface definitions
- `@eddacraft/anvil-edda-stack/testing` — Test utilities and fixtures
- CLI commands: `anvil stack status`, `anvil stack validate`
- Configuration schema in `.anvilrc` (stack section)

**Configuration Example:**

```json
{
  "stack": {
    "kindling": { "enabled": true },
    "ember": { "enabled": true },
    "edda": { "enabled": true },
    "validation": {
      "check_provenance_integrity": true,
      "check_schema_compatibility": true
    }
  }
}
```

## Boundary Rules

- Layer modules MUST communicate only through defined ports
- Provenance links MUST be bidirectionally resolvable
- Schema changes MUST be coordinated with migration tooling
- Tests MUST not depend on other layers unless explicitly integration tests
- Configuration MUST cascade: stack → layer → feature

## Critical Decision: Schema Language

**The most dangerous decision is whether Ember and Edda share a schema language.**

### Option A: Shared Schema (Recommended for v1)

```
packages/edda-stack/
├── contracts/           # Shared Zod schemas
│   ├── common.ts        # IDs, timestamps, confidence
│   ├── provenance.ts    # Cross-layer links
│   ├── proposal.ts      # Ember proposal schema
│   ├── memory.ts        # Edda memory schema
│   └── mappings.ts      # Proposal → Memory conversion
```

**Advantages:**

- Single source of truth for types
- Guaranteed compatibility
- Easier testing
- Clear evolution path

**Disadvantages:**

- Coupling (changes affect both)
- May constrain layer-specific optimisation

### Option B: Independent Schemas with Mapping Layer

```
packages/edda-stack/
├── ember/contracts/     # Ember-specific schemas
├── edda/contracts/      # Edda-specific schemas
└── integration/
    └── mappers/         # Explicit conversion
```

**Advantages:**

- Layers can evolve independently
- Optimise per-layer

**Disadvantages:**

- Mapping maintenance burden
- Drift risk

### Recommended Approach

**Shared contracts for common types, independent storage for optimisation.**

```typescript
// Shared (contracts/)
export type ObservationId = string; // UUID
export type ProposalId = string;    // UUID
export type MemoryId = string;      // UUID
export type Timestamp = string;     // ISO8601

// Ember-specific (ember/)
interface EmberProposal {
  // Uses shared types
  source_refs: ObservationId[];
  // Plus ephemeral-optimised fields
  ttl_remaining_ms: number;
}

// Edda-specific (edda/)
interface EddaMemory {
  // Uses shared types
  provenance: { observation_ids: ObservationId[] };
  // Plus versioned fields
  schema_version: number;
  git_commit: string;
}
```

## Acceptance Criteria

- [ ] Shared contracts package defined with common types
- [ ] Provenance schema supports cross-layer resolution
- [ ] Type mappings defined for Proposal → Memory promotion
- [ ] `anvil stack status` shows health of all layers
- [ ] `anvil stack validate` checks provenance integrity
- [ ] Layer mocks available for isolated testing
- [ ] Integration test fixtures cover full stack flow
- [ ] Configuration schema validates layer dependencies
- [ ] Schema versioning documented with compatibility rules

## Risks & Mitigations

| Risk                          | Mitigation                                        |
| ----------------------------- | ------------------------------------------------- |
| Schema drift between layers   | Shared contracts, versioning, validation tests    |
| Provenance links break        | Integrity checks, graceful degradation            |
| Configuration complexity      | Sensible defaults, validation on load             |
| Testing requires full stack   | Layer mocks, clear isolation boundaries           |
| Migration coordination fails  | Explicit migration protocol, version gates        |

## Tasks

### Phase A: Contracts Foundation

#### STACK-001: Common identifier schemas

- **Intent:** Define shared ID formats used across all layers
- **Expected Outcome:** Zod schemas for ObservationId, ProposalId, MemoryId
- **Scope:** `packages/edda-stack/src/contracts/`
- **Non-scope:** Storage-specific ID generation
- **Files:**
  - `packages/edda-stack/src/contracts/identifiers.ts`
  - `packages/edda-stack/src/contracts/identifiers.test.ts`
- **Dependencies:** —
- **Validation:** `nx test edda-stack --testNamePattern="identifiers"`
- **Confidence:** high
- **Status:** Complete

#### STACK-002: Timestamp and temporal schemas

- **Intent:** Define shared timestamp conventions (ISO8601)
- **Expected Outcome:** Timestamp schemas with validation and utilities
- **Scope:** `packages/edda-stack/src/contracts/`
- **Non-scope:** Timezone handling beyond ISO8601
- **Files:**
  - `packages/edda-stack/src/contracts/temporal.ts`
  - `packages/edda-stack/src/contracts/temporal.test.ts`
- **Dependencies:** —
- **Validation:** `nx test edda-stack --testNamePattern="temporal"`
- **Confidence:** high
- **Status:** Complete

#### STACK-003: Confidence scale definitions

- **Intent:** Define confidence levels used across Ember and Edda
- **Expected Outcome:** Shared confidence schema with semantic definitions
- **Scope:** `packages/edda-stack/src/contracts/`
- **Non-scope:** Confidence computation logic
- **Files:**
  - `packages/edda-stack/src/contracts/confidence.ts`
  - `packages/edda-stack/src/contracts/confidence.test.ts`
- **Dependencies:** —
- **Validation:** `nx test edda-stack --testNamePattern="confidence"`
- **Confidence:** high
- **Status:** Complete

#### STACK-004: Provenance link schema

- **Intent:** Define cross-layer reference schema for provenance tracking
- **Expected Outcome:** ProvenanceLink schema with validation rules
- **Scope:** `packages/edda-stack/src/contracts/`
- **Non-scope:** Link resolution implementation
- **Files:**
  - `packages/edda-stack/src/contracts/provenance.ts`
  - `packages/edda-stack/src/contracts/provenance.test.ts`
- **Dependencies:** STACK-001
- **Validation:** `nx test edda-stack --testNamePattern="provenance"`
- **Confidence:** high
- **Status:** Complete

### Phase B: Type Mappings

#### STACK-005: Proposal → Memory type mapping

- **Intent:** Define explicit conversion rules between Ember and Edda types
- **Expected Outcome:** Mapping functions with validation for promotion
- **Scope:** `packages/edda-stack/src/contracts/`
- **Non-scope:** Promotion workflow (business logic)
- **Files:**
  - `packages/edda-stack/src/contracts/type-mappings.ts`
  - `packages/edda-stack/src/contracts/type-mappings.test.ts`
- **Dependencies:** EMBER-001, EDDA-001
- **Validation:** `nx test edda-stack --testNamePattern="type-mappings"`
- **Confidence:** high
- **Status:** Complete

#### STACK-006: Observation → Proposal type mapping

- **Intent:** Define conversion rules from Kindling observations to Ember proposals
- **Expected Outcome:** Mapping functions for observation aggregation
- **Scope:** `packages/edda-stack/src/contracts/`
- **Non-scope:** Aggregation logic
- **Files:**
  - `packages/edda-stack/src/contracts/observation-mappings.ts`
  - `packages/edda-stack/src/contracts/observation-mappings.test.ts`
- **Dependencies:** KINDLING-001, EMBER-001
- **Validation:** `nx test edda-stack --testNamePattern="observation-mappings"`
- **Confidence:** medium
- **Status:** Draft

### Phase C: Ports & Interfaces

#### STACK-007: Layer port definitions

- **Intent:** Define interface contracts for layer boundaries
- **Expected Outcome:** TypeScript interfaces for Kindling, Ember, Edda ports
- **Scope:** `packages/edda-stack/src/contracts/ports/`
- **Non-scope:** Implementation
- **Files:**
  - `packages/edda-stack/src/contracts/ports/kindling.port.ts`
  - `packages/edda-stack/src/contracts/ports/ember.port.ts`
  - `packages/edda-stack/src/contracts/ports/edda.port.ts`
  - `packages/edda-stack/src/contracts/ports/index.ts`
- **Dependencies:** STACK-001 through STACK-006
- **Validation:** TypeScript compilation passes
- **Confidence:** high
- **Status:** Complete

#### STACK-008: Event bus for layer communication

- **Intent:** Define event types for cross-layer communication
- **Expected Outcome:** Event schemas for observation, proposal, promotion events
- **Scope:** `packages/edda-stack/src/contracts/`
- **Non-scope:** Event bus implementation (async/sync choice)
- **Files:**
  - `packages/edda-stack/src/contracts/events.ts`
  - `packages/edda-stack/src/contracts/events.test.ts`
- **Dependencies:** STACK-004
- **Validation:** `nx test edda-stack --testNamePattern="events"`
- **Confidence:** medium
- **Status:** Complete

### Phase D: Testing Utilities

#### STACK-009: Layer mock factories

- **Intent:** Provide mock implementations for isolated testing
- **Expected Outcome:** Mock factories for Kindling, Ember, Edda services
- **Scope:** `packages/edda-stack/src/testing/`
- **Non-scope:** Full integration tests
- **Files:**
  - `packages/edda-stack/src/testing/mocks/kindling.mock.ts`
  - `packages/edda-stack/src/testing/mocks/ember.mock.ts`
  - `packages/edda-stack/src/testing/mocks/edda.mock.ts`
  - `packages/edda-stack/src/testing/index.ts`
- **Dependencies:** STACK-007
- **Validation:** Mocks can substitute for real implementations in tests
- **Confidence:** high
- **Status:** Complete

#### STACK-010: Integration test fixtures

- **Intent:** Provide fixtures for testing full stack flows
- **Expected Outcome:** Sample observations, proposals, memories for testing
- **Scope:** `packages/edda-stack/src/testing/`
- **Non-scope:** E2E tests (Playwright)
- **Files:**
  - `packages/edda-stack/src/testing/fixtures/proposals.ts`
  - `packages/edda-stack/src/testing/fixtures/memories.ts`
  - `packages/edda-stack/src/testing/fixtures/index.ts`
- **Dependencies:** STACK-009
- **Validation:** Fixtures pass schema validation
- **Confidence:** high
- **Status:** Complete

#### STACK-011: Provenance chain validator

- **Intent:** Tool to validate provenance integrity across layers
- **Expected Outcome:** Validator checks links resolve and form valid chains
- **Scope:** `packages/edda-stack/src/testing/`
- **Non-scope:** Production monitoring
- **Files:**
  - `packages/edda-stack/src/testing/validators/provenance-chain.ts`
  - `packages/edda-stack/src/testing/validators/provenance-chain.test.ts`
- **Dependencies:** STACK-004, STACK-010
- **Validation:** `nx test edda-stack --testNamePattern="provenance-chain"`
- **Confidence:** high
- **Status:** Complete

### Phase E: CLI & Configuration

#### STACK-012: Stack configuration schema

- **Intent:** Define stack-wide configuration with layer dependencies
- **Expected Outcome:** Zod schema for stack config, validation of dependencies
- **Scope:** `packages/edda-stack/src/`, `core/src/gate/gate-config.ts`
- **Non-scope:** TUI config editor
- **Files:**
  - `packages/edda-stack/src/config.ts`
  - `core/src/gate/gate-config.ts` (extend schema)
- **Dependencies:** KINDLING-002, EMBER-003, EDDA-005
- **Validation:** `nx test edda-stack --testNamePattern="stack.*config"`
- **Confidence:** high
- **Status:** Complete

#### STACK-013: CLI stack status command

- **Intent:** Add `anvil stack status` to show health of all layers
- **Expected Outcome:** CLI displays enabled/disabled, counts, health indicators
- **Scope:** `apps/anvil-cli/src/commands/`
- **Non-scope:** Detailed diagnostics per layer
- **Files:**
  - `apps/anvil-cli/src/commands/stack.ts`
  - `apps/anvil-cli/src/commands/stack.test.ts`
- **Dependencies:** STACK-012
- **Validation:** `anvil stack status`
- **Confidence:** high
- **Status:** Complete

#### STACK-014: CLI stack validate command

- **Intent:** Add `anvil stack validate` to check provenance integrity
- **Expected Outcome:** CLI validates cross-layer links, reports broken chains
- **Scope:** `apps/anvil-cli/src/commands/`
- **Non-scope:** Auto-repair
- **Files:**
  - `apps/anvil-cli/src/commands/stack.ts` (add validate subcommand)
- **Dependencies:** STACK-011, STACK-013
- **Validation:** `anvil stack validate`
- **Confidence:** high
- **Status:** Complete

### Phase F: Documentation

#### STACK-015: Stack architecture documentation

- **Intent:** Document the Edda Stack architecture and philosophy
- **Expected Outcome:** Architecture guide explaining layers, boundaries, data flow
- **Scope:** `docs/`, `packages/edda-stack/README.md`
- **Non-scope:** Implementation tutorials
- **Files:**
  - `docs/architecture/edda-stack.md`
  - `packages/edda-stack/README.md` (enhance existing)
- **Dependencies:** STACK-007
- **Validation:** Manual review
- **Confidence:** high
- **Status:** Complete

#### STACK-016: Migration guide

- **Intent:** Document how to migrate schema changes across the stack
- **Expected Outcome:** Guide for coordinated schema migrations
- **Scope:** `docs/`
- **Non-scope:** Automatic migration tooling
- **Files:**
  - `docs/guides/stack-migration.md`
- **Dependencies:** STACK-012
- **Validation:** Manual review
- **Confidence:** medium
- **Status:** Complete

### Phase G: Reconciliation

#### STACK-017: Path drift cleanup in APS plan files

- **Intent:** Align all APS file references with actual package layout
- **Expected Outcome:** All Scope/Files fields in STACK, EMBER, EDDA plans use
  correct `src/` paths; index and rules files updated
- **Scope:** `plans/`, `.claude/rules/`
- **Non-scope:** Code changes — plan metadata only
- **Files:**
  - `plans/modules/edda-stack-integration.aps.md`
  - `plans/modules/ember.aps.md`
  - `plans/modules/edda.aps.md`
  - `plans/index.aps.md`
  - `.claude/rules/aps-project.md`
- **Dependencies:** —
- **Validation:** All file paths in plan tasks resolve to real files
- **Confidence:** high
- **Status:** Complete

#### STACK-018: Retroactive evidence capture

- **Intent:** Record test results and implementation evidence for completed tasks
- **Expected Outcome:** Execution step files confirm STACK-001–016 pass criteria
- **Scope:** `plans/execution/`
- **Non-scope:** New implementation work
- **Files:**
  - `plans/execution/STACK-001.steps.md` through `STACK-016.steps.md`
- **Dependencies:** STACK-017
- **Validation:** Each steps file references passing test or artefact
- **Confidence:** high
- **Status:** Complete

#### STACK-019: Missing deliverable audit

- **Intent:** Identify STACK deliverables that exist in code but lack plan tasks
- **Expected Outcome:** New tasks or notes for undocumented artefacts (e.g.
  `ember-proposal.ts`, `edda-memory.ts` live in STACK tree but are owned by
  EMBER-001 / EDDA-001)
- **Scope:** `plans/modules/edda-stack-integration.aps.md`
- **Non-scope:** Creating the missing implementations
- **Files:**
  - `plans/modules/edda-stack-integration.aps.md`
- **Dependencies:** STACK-017
- **Validation:** Manual review — no orphaned artefacts
- **Confidence:** medium
- **Status:** Complete
- **Audit Result:** 44 non-test, non-index source files audited. 42 tracked by
  STACK, EMBER, EDDA, or EERB plan tasks. 2 untracked files identified (see
  "STACK-019 Audit: Untracked Source Files" below).

## Decisions

**D-STACK-001:** Shared contracts package (common types)

- **Rationale:** Single source of truth, guaranteed compatibility
- **Alternatives:** Per-layer schemas with mapping
- **Trade-offs:** Some coupling, but clearer evolution

**D-STACK-002:** Per-layer storage schemas (optimised)

- **Rationale:** Each layer has different performance characteristics
- **Alternatives:** Shared storage schema
- **Trade-offs:** Some duplication, but better optimisation

**D-STACK-003:** Event-based layer communication

- **Rationale:** Loose coupling, testability, clear boundaries
- **Alternatives:** Direct service calls
- **Trade-offs:** More indirection, but cleaner architecture

**D-STACK-004:** Provenance links are mandatory

- **Rationale:** Traceability is core to the stack's value proposition
- **Alternatives:** Optional provenance
- **Trade-offs:** More storage, but auditability

**D-STACK-005:** Schema versions are explicit

- **Rationale:** Enables migration tooling, backwards compatibility
- **Alternatives:** Implicit versioning via git
- **Trade-offs:** More complexity, but safer evolution

## Notes

**Package structure (complete):**

```
packages/edda-stack/
├── src/
│   ├── contracts/                # Shared type definitions
│   │   ├── identifiers.ts        # ID formats
│   │   ├── temporal.ts           # Timestamp conventions
│   │   ├── confidence.ts         # Confidence scales
│   │   ├── provenance.ts         # Cross-layer references
│   │   ├── type-mappings.ts      # Proposal → Memory
│   │   ├── events.ts             # Layer communication events
│   │   ├── ember-proposal.ts     # Ember proposal schema
│   │   ├── edda-memory.ts        # Edda memory schema
│   │   ├── ports/                # Interface definitions
│   │   │   ├── kindling.port.ts
│   │   │   ├── ember.port.ts
│   │   │   ├── edda.port.ts
│   │   │   └── index.ts
│   │   └── index.ts
│   ├── testing/                  # Test utilities
│   │   ├── mocks/
│   │   │   ├── kindling.mock.ts
│   │   │   ├── ember.mock.ts
│   │   │   └── edda.mock.ts
│   │   ├── fixtures/
│   │   │   ├── proposals.ts
│   │   │   ├── memories.ts
│   │   │   └── index.ts
│   │   ├── validators/
│   │   │   └── provenance-chain.ts
│   │   └── index.ts
│   ├── config.ts                 # Stack configuration
│   └── index.ts
├── ember/                        # Ember implementation (not yet built)
│   └── (see ember.aps.md)
├── edda/                         # Edda implementation (not yet built)
│   └── (see edda.aps.md)
├── README.md
└── package.json
```

**Data flow (single pass):**

```
1. Kindling records everything that happens
   └── Observations emitted (11 kinds)

2. Ember suggests what it might mean
   └── Candidates generated (6 types)
   └── Confidence scored (heuristics)
   └── Proposals decay unless promoted

3. Edda preserves what has earned permanence
   └── Human reviews candidate
   └── Promotion decision (with attribution)
   └── Memory created (versioned, auditable)
```

**Failure modes this architecture avoids:**

```
✗ Log-as-memory collapse
  → Kindling is logs, Edda is memory (explicit)

✗ AI hallucinations becoming "facts"
  → Promotion requires human decision

✗ Memory inflation
  → Ember decays, Edda requires justification

✗ Silent drift of institutional knowledge
  → Git-backed, versioned, evolution graph

✗ Agent feedback loops reinforcing errors
  → Read-only enforcement in Kindling
```

**Mental models (repeat to team):**

```
Kindling is a camera — it sees everything, understands nothing
Ember is a curator — it proposes, but cannot decide
Edda is a ledger — you never casually rewrite it

Ember is a queue that empties itself
Edda is a ledger that persists

Truth flows one way: Kindling → Ember → Edda
AI never writes back to Kindling
Promotion must be deliberate and attributable
```

**Future enhancements (v2+):**

- Parquet export for analytics (common schema across layers)
- Cross-workspace stack federation
- Real-time streaming between layers
- GraphQL API for stack queries
- TUI stack explorer (visualise data flow)
- AI-assisted promotion suggestions (human decides)
- Stack metrics dashboard (Prometheus/Grafana)

## STACK-019 Audit: Untracked Source Files

Full source tree audit of `packages/edda-stack/src/` (excluding test files and
barrel `index.ts` files). **44 non-test, non-index files examined; 42 tracked.**

### Untracked files

#### `contracts/edda-extended.ts` (~938 lines)

Extended Edda contracts covering governance, RBAC, enforcement hooks, knowledge
graph types, and additional memory schemas. Pure TypeScript interfaces — no
runtime code.

- **Conceptual owner:** EDDA module (extends the canonical memory layer)
- **Likely origin:** Emerged during EDDA implementation but never received a
  dedicated plan task
- **Recommendation:** No action required for v1. If these contracts are consumed
  by future features (governance, RBAC), create a task at that point. Currently
  unused outside the file itself.

#### `edda/store-interfaces.ts` (~41 lines)

Internal interfaces (`IMemoryStoreOperations`, `IVersionTracker`) extracted from
`memory-store.ts` for dependency inversion.

- **Conceptual owner:** EDDA-006 (memory store implementation)
- **Likely origin:** Implementation detail extracted during EDDA-006 work
- **Recommendation:** No action required. This is an internal refactoring
  artefact — a supporting file for EDDA-006, not a standalone deliverable.
