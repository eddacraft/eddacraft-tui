<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Edda Stack Integration

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| STACK | —     | medium   | Draft  |

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
- **Scope:** `packages/edda-stack/contracts/`
- **Non-scope:** Storage-specific ID generation
- **Files:**
  - `packages/edda-stack/contracts/identifiers.ts`
  - `packages/edda-stack/contracts/identifiers.test.ts`
- **Dependencies:** —
- **Validation:** `nx test edda-stack --testNamePattern="identifiers"`
- **Confidence:** high
- **Status:** Draft

#### STACK-002: Timestamp and temporal schemas

- **Intent:** Define shared timestamp conventions (ISO8601)
- **Expected Outcome:** Timestamp schemas with validation and utilities
- **Scope:** `packages/edda-stack/contracts/`
- **Non-scope:** Timezone handling beyond ISO8601
- **Files:**
  - `packages/edda-stack/contracts/temporal.ts`
  - `packages/edda-stack/contracts/temporal.test.ts`
- **Dependencies:** —
- **Validation:** `nx test edda-stack --testNamePattern="temporal"`
- **Confidence:** high
- **Status:** Draft

#### STACK-003: Confidence scale definitions

- **Intent:** Define confidence levels used across Ember and Edda
- **Expected Outcome:** Shared confidence schema with semantic definitions
- **Scope:** `packages/edda-stack/contracts/`
- **Non-scope:** Confidence computation logic
- **Files:**
  - `packages/edda-stack/contracts/confidence.ts`
  - `packages/edda-stack/contracts/confidence.test.ts`
- **Dependencies:** —
- **Validation:** `nx test edda-stack --testNamePattern="confidence"`
- **Confidence:** high
- **Status:** Draft

#### STACK-004: Provenance link schema

- **Intent:** Define cross-layer reference schema for provenance tracking
- **Expected Outcome:** ProvenanceLink schema with validation rules
- **Scope:** `packages/edda-stack/contracts/`
- **Non-scope:** Link resolution implementation
- **Files:**
  - `packages/edda-stack/contracts/provenance-link.ts`
  - `packages/edda-stack/contracts/provenance-link.test.ts`
- **Dependencies:** STACK-001
- **Validation:** `nx test edda-stack --testNamePattern="provenance-link"`
- **Confidence:** high
- **Status:** Draft

### Phase B: Type Mappings

#### STACK-005: Proposal → Memory type mapping

- **Intent:** Define explicit conversion rules between Ember and Edda types
- **Expected Outcome:** Mapping functions with validation for promotion
- **Scope:** `packages/edda-stack/contracts/`
- **Non-scope:** Promotion workflow (business logic)
- **Files:**
  - `packages/edda-stack/contracts/type-mappings.ts`
  - `packages/edda-stack/contracts/type-mappings.test.ts`
- **Dependencies:** EMBER-001, EDDA-001
- **Validation:** `nx test edda-stack --testNamePattern="type-mappings"`
- **Confidence:** high
- **Status:** Draft

#### STACK-006: Observation → Proposal type mapping

- **Intent:** Define conversion rules from Kindling observations to Ember proposals
- **Expected Outcome:** Mapping functions for observation aggregation
- **Scope:** `packages/edda-stack/contracts/`
- **Non-scope:** Aggregation logic
- **Files:**
  - `packages/edda-stack/contracts/observation-mappings.ts`
  - `packages/edda-stack/contracts/observation-mappings.test.ts`
- **Dependencies:** KINDLING-001, EMBER-001
- **Validation:** `nx test edda-stack --testNamePattern="observation-mappings"`
- **Confidence:** medium
- **Status:** Draft

### Phase C: Ports & Interfaces

#### STACK-007: Layer port definitions

- **Intent:** Define interface contracts for layer boundaries
- **Expected Outcome:** TypeScript interfaces for Kindling, Ember, Edda ports
- **Scope:** `packages/edda-stack/ports/`
- **Non-scope:** Implementation
- **Files:**
  - `packages/edda-stack/ports/kindling.port.ts`
  - `packages/edda-stack/ports/ember.port.ts`
  - `packages/edda-stack/ports/edda.port.ts`
  - `packages/edda-stack/ports/index.ts`
- **Dependencies:** STACK-001 through STACK-006
- **Validation:** TypeScript compilation passes
- **Confidence:** high
- **Status:** Draft

#### STACK-008: Event bus for layer communication

- **Intent:** Define event types for cross-layer communication
- **Expected Outcome:** Event schemas for observation, proposal, promotion events
- **Scope:** `packages/edda-stack/contracts/`
- **Non-scope:** Event bus implementation (async/sync choice)
- **Files:**
  - `packages/edda-stack/contracts/events.ts`
  - `packages/edda-stack/contracts/events.test.ts`
- **Dependencies:** STACK-004
- **Validation:** `nx test edda-stack --testNamePattern="events"`
- **Confidence:** medium
- **Status:** Draft

### Phase D: Testing Utilities

#### STACK-009: Layer mock factories

- **Intent:** Provide mock implementations for isolated testing
- **Expected Outcome:** Mock factories for Kindling, Ember, Edda services
- **Scope:** `packages/edda-stack/testing/`
- **Non-scope:** Full integration tests
- **Files:**
  - `packages/edda-stack/testing/mocks/kindling.mock.ts`
  - `packages/edda-stack/testing/mocks/ember.mock.ts`
  - `packages/edda-stack/testing/mocks/edda.mock.ts`
  - `packages/edda-stack/testing/index.ts`
- **Dependencies:** STACK-007
- **Validation:** Mocks can substitute for real implementations in tests
- **Confidence:** high
- **Status:** Draft

#### STACK-010: Integration test fixtures

- **Intent:** Provide fixtures for testing full stack flows
- **Expected Outcome:** Sample observations, proposals, memories for testing
- **Scope:** `packages/edda-stack/testing/`
- **Non-scope:** E2E tests (Playwright)
- **Files:**
  - `packages/edda-stack/testing/fixtures/observations.ts`
  - `packages/edda-stack/testing/fixtures/proposals.ts`
  - `packages/edda-stack/testing/fixtures/memories.ts`
  - `packages/edda-stack/testing/fixtures/chains.ts` (full provenance chains)
- **Dependencies:** STACK-009
- **Validation:** Fixtures pass schema validation
- **Confidence:** high
- **Status:** Draft

#### STACK-011: Provenance chain validator

- **Intent:** Tool to validate provenance integrity across layers
- **Expected Outcome:** Validator checks links resolve and form valid chains
- **Scope:** `packages/edda-stack/testing/`
- **Non-scope:** Production monitoring
- **Files:**
  - `packages/edda-stack/testing/validators/provenance-chain.ts`
  - `packages/edda-stack/testing/validators/provenance-chain.test.ts`
- **Dependencies:** STACK-004, STACK-010
- **Validation:** `nx test edda-stack --testNamePattern="provenance-chain"`
- **Confidence:** high
- **Status:** Draft

### Phase E: CLI & Configuration

#### STACK-012: Stack configuration schema

- **Intent:** Define stack-wide configuration with layer dependencies
- **Expected Outcome:** Zod schema for stack config, validation of dependencies
- **Scope:** `packages/edda-stack/`, `core/src/gate/gate-config.ts`
- **Non-scope:** TUI config editor
- **Files:**
  - `packages/edda-stack/config.ts`
  - `core/src/gate/gate-config.ts` (extend schema)
- **Dependencies:** KINDLING-002, EMBER-003, EDDA-005
- **Validation:** `nx test edda-stack --testNamePattern="stack.*config"`
- **Confidence:** high
- **Status:** Draft

#### STACK-013: CLI stack status command

- **Intent:** Add `anvil stack status` to show health of all layers
- **Expected Outcome:** CLI displays enabled/disabled, counts, health indicators
- **Scope:** `cli/src/commands/`
- **Non-scope:** Detailed diagnostics per layer
- **Files:**
  - `cli/src/commands/stack.ts`
  - `cli/src/commands/stack.test.ts`
- **Dependencies:** STACK-012
- **Validation:** `anvil stack status`
- **Confidence:** high
- **Status:** Draft

#### STACK-014: CLI stack validate command

- **Intent:** Add `anvil stack validate` to check provenance integrity
- **Expected Outcome:** CLI validates cross-layer links, reports broken chains
- **Scope:** `cli/src/commands/`
- **Non-scope:** Auto-repair
- **Files:**
  - `cli/src/commands/stack.ts` (add validate subcommand)
- **Dependencies:** STACK-011, STACK-013
- **Validation:** `anvil stack validate`
- **Confidence:** high
- **Status:** Draft

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
- **Status:** Draft

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
- **Status:** Draft

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
├── contracts/                    # Shared type definitions
│   ├── identifiers.ts            # ID formats
│   ├── temporal.ts               # Timestamp conventions
│   ├── confidence.ts             # Confidence scales
│   ├── provenance-link.ts        # Cross-layer references
│   ├── type-mappings.ts          # Proposal → Memory
│   ├── observation-mappings.ts   # Observation → Proposal
│   ├── events.ts                 # Layer communication events
│   ├── ember-proposal.ts         # Re-export from ember
│   ├── edda-memory.ts            # Re-export from edda
│   └── index.ts
├── ports/                        # Interface definitions
│   ├── kindling.port.ts
│   ├── ember.port.ts
│   ├── edda.port.ts
│   └── index.ts
├── ember/                        # Ember implementation
│   └── (see ember.aps.md)
├── edda/                         # Edda implementation
│   └── (see edda.aps.md)
├── testing/                      # Test utilities
│   ├── mocks/
│   │   ├── kindling.mock.ts
│   │   ├── ember.mock.ts
│   │   └── edda.mock.ts
│   ├── fixtures/
│   │   ├── observations.ts
│   │   ├── proposals.ts
│   │   ├── memories.ts
│   │   └── chains.ts
│   ├── validators/
│   │   └── provenance-chain.ts
│   └── index.ts
├── config.ts                     # Stack configuration
├── README.md
└── index.ts
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
