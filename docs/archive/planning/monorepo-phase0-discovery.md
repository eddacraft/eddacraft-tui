# Monorepo Migration Phase 0: Discovery & Analysis

> **Date:** 2026-01-14 **Status:** Complete **Author:** Claude Code (autonomous
> agent) **Context:** Pre-Phase 1 discovery to establish accurate migration
> boundaries

---

## Executive Summary

This document captures the findings from Phase 0 discovery tasks (MONO-000a
through MONO-000d). The discovery revealed significant gaps between the
documented plan and actual codebase state:

| Finding                                                    | Impact                                |
| ---------------------------------------------------------- | ------------------------------------- |
| apps/ directory exists with placeholder scaffolds          | Plan assumed it needed creation       |
| edda-stack has 10,422 lines of production code             | Was marked "Out of Scope"             |
| kindling-integration exists (3 files)                      | Not mentioned in original plan        |
| packages/anvil, platform, shared, tooling are placeholders | Plan assumed they needed creation     |
| core/ has 41,580 lines across 17 subdirectories            | Original plan lacked detailed mapping |

---

## 1. Current State Inventory (MONO-000a)

### 1.1 Root Directory Structure

```
anvil.monorepo-migration/
├── apps/                    # EXISTS - placeholder scaffolds
├── archive (removed)        # Historical/deprecated code
├── cli (removed)            # CLI now under apps/anvil-cli
├── core (removed)           # Core now under packages/anvil
├── docs/                    # Internal documentation
├── packages/                # Mixed production/placeholder
├── packs (removed)          # Legacy packs root
├── plans/                   # APS planning specs
├── scripts/                 # Build utilities
├── tools/                   # Placeholder for generators
└── ui (removed)             # Legacy UI root
```

### 1.2 apps/ Directory (Placeholder Scaffolds)

All apps are placeholder scaffolds containing only README.md files:

| App             | Status      | Contents       |
| --------------- | ----------- | -------------- |
| apps/anvil-api/ | Placeholder | README.md only |
| apps/anvil-ui/  | Placeholder | README.md only |
| apps/docs-site/ | Placeholder | README.md only |
| apps/e2e/       | Placeholder | README.md only |
| apps/website/   | Placeholder | README.md only |

**Implication:** No actual migration needed for apps/ structure - scaffolds
exist.

### 1.3 packages/ Directory

| Package                        | Status         | Lines      | Notes                  |
| ------------------------------ | -------------- | ---------- | ---------------------- |
| packages/adapters/             | Production     | ~3k        | Bundled adapters       |
| packages/anvil/                | Placeholder    | 0          | README only            |
| packages/aps/                  | Production     | ~4k        | APS module             |
| packages/edda-stack/           | **Production** | **10,422** | Memory/proposal system |
| packages/eslint-plugin-anvil/  | Production     | ~1k        | ESLint plugin          |
| packages/kindling-integration/ | Production     | ~500       | Kindling contracts     |
| packages/platform/             | Placeholder    | 0          | README only            |
| packages/shared/               | Placeholder    | 0          | README only            |
| packages/tooling/              | Placeholder    | 0          | README only            |
| packages/vscode-extension/     | Production     | ~2k        | VS Code extension      |

### 1.4 core/ Source Code Inventory

Total: **41,580 lines** across 17 subdirectories

| Subdirectory  | Lines  | Test Files | Description                                  |
| ------------- | ------ | ---------- | -------------------------------------------- |
| gate/         | 16,757 | 5          | Gate runner, checks, OPA policy integration  |
| architecture/ | 6,469  | 4          | Layer detection, baseline, boundary analysis |
| drift/        | 3,060  | 3          | Snapshot capture, drift reporting            |
| export/       | 2,817  | 2          | llms.txt, MCP, export formats                |
| antipattern/  | 2,010  | 2          | Pattern detection and scanning               |
| cache/        | 1,744  | 2          | File-based caching layer                     |
| suppression/  | 1,595  | 2          | Warning suppression system                   |
| explain/      | 1,407  | 1          | Explain command support                      |
| provenance/   | 1,274  | 1          | Provenance tracking                          |
| watch/        | 1,168  | 1          | File system watching                         |
| validation/   | 917    | 2          | APS plan validation                          |
| schema/       | 651    | 1          | Zod schemas (APS, warning)                   |
| warnings/     | 564    | 1          | Warning ID system                            |
| crypto/       | 405    | 1          | Hashing utilities                            |
| types/        | 278    | 0          | Shared type definitions                      |
| utils/        | 204    | 1          | Generic utilities                            |
| **fixtures**/ | 0      | 0          | Test fixtures                                |

---

## 2. core/ Subdirectory Mapping (MONO-000b)

### 2.1 Target Package Architecture

```
packages/
└── anvil/
    ├── contracts/     # Schemas, types, events (zero dependencies)
    ├── ports/         # Interfaces only (depends on contracts)
    ├── core/          # Pure domain logic (depends on ports, contracts)
    ├── runtime/       # Orchestration + I/O (depends on core, ports)
    ├── policy/        # OPA/Rego wrappers (depends on contracts)
    └── sdk/           # Client SDK (depends on contracts, ports)
```

### 2.2 Mapping Table

| Source (core/src/) | Target Package                   | Lines | Rationale                    |
| ------------------ | -------------------------------- | ----- | ---------------------------- |
| schema/            | @eddacraft/anvil-contracts       | 651   | Pure Zod schemas, zero I/O   |
| types/             | @eddacraft/anvil-contracts       | 278   | Type definitions only        |
| validation/        | @eddacraft/anvil-contracts       | 917   | Schema validation (no I/O)   |
| crypto/            | @eddacraft/anvil-platform/crypto | 405   | Hashing (crypto module I/O)  |
| utils/             | @eddacraft/anvil-shared/util     | 204   | Generic utilities            |
| provenance/        | @eddacraft/anvil-core            | 1,274 | Pure domain logic            |
| warnings/          | @eddacraft/anvil-core            | 564   | Pure domain logic            |
| antipattern/       | @eddacraft/anvil-core            | 2,010 | Pure domain logic            |
| suppression/       | @eddacraft/anvil-core            | 1,595 | Pure domain logic            |
| explain/           | @eddacraft/anvil-core            | 1,407 | Pure domain logic            |
| architecture/      | @eddacraft/anvil-core            | 6,469 | Architecture analysis domain |
| drift/             | @eddacraft/anvil-core            | 3,060 | Drift detection domain       |
| cache/             | @eddacraft/anvil-runtime         | 1,744 | File system I/O              |
| watch/             | @eddacraft/anvil-runtime         | 1,168 | File watching I/O            |
| export/            | @eddacraft/anvil-runtime         | 2,817 | File writing I/O             |

### 2.3 gate/ Subdirectory Split (16,757 lines)

The gate/ subdirectory requires splitting across multiple packages:

```
core/src/gate/
├── check.interface.ts      → @eddacraft/anvil-ports (interface definition)
├── gate-runner.ts          → @eddacraft/anvil-runtime (orchestration)
├── gate-config.ts          → @eddacraft/anvil-runtime (configuration loading)
├── integration.test.ts     → @eddacraft/anvil-runtime (tests)
├── checks/                  → @eddacraft/anvil-runtime (check implementations)
│   ├── eslint.check.ts
│   ├── coverage.check.ts
│   ├── secret.check.ts
│   ├── policy.check.ts
│   ├── architecture.check.ts
│   └── command-safety.check.ts
├── config/                  → @eddacraft/anvil-runtime (config utilities)
├── formatters/              → @eddacraft/anvil-runtime (output formatting)
├── parsers/                 → @eddacraft/anvil-runtime (output parsing)
├── rules/                   → @eddacraft/anvil-runtime (rule definitions)
└── policy/                  → @eddacraft/anvil-policy (OPA integration)
    ├── opa-binary-manager.ts
    ├── opa-executor.ts
    ├── policy-loader.ts
    ├── bundle-manager.ts
    └── bundle-verifier.ts
```

### 2.4 Line Count Summary by Target

| Target Package                   | Total Lines | Percentage |
| -------------------------------- | ----------- | ---------- |
| @eddacraft/anvil-contracts       | 1,846       | 4.4%       |
| @eddacraft/anvil-ports           | ~200        | 0.5%       |
| @eddacraft/anvil-core            | 14,379      | 34.6%      |
| @eddacraft/anvil-runtime         | 12,500      | 30.1%      |
| @eddacraft/anvil-policy          | 4,046       | 9.7%       |
| @eddacraft/anvil-platform/crypto | 405         | 1.0%       |
| @eddacraft/anvil-shared/util     | 204         | 0.5%       |
| **Unassigned/Split**             | **8,000**   | **19.2%**  |

---

## 3. edda-stack Structure (MONO-000c)

### 3.1 Package Overview

**Location:** `packages/edda-stack/` **Total Lines:** 10,422 **Status:**
Production code (not a placeholder)

### 3.2 Directory Structure

```
packages/edda-stack/
├── src/
│   ├── index.ts              # Package entry point
│   ├── config.ts             # Stack configuration
│   ├── config.test.ts        # Configuration tests
│   ├── contracts/            # 4,000+ lines
│   │   ├── index.ts          # Re-exports all contracts
│   │   ├── identifiers.ts    # UUID schemas, ID creators
│   │   ├── temporal.ts       # Timestamp/duration schemas
│   │   ├── confidence.ts     # Confidence level schemas
│   │   ├── provenance.ts     # Provenance chain schemas
│   │   ├── ember-proposal.ts # Ember proposal schemas
│   │   ├── edda-memory.ts    # Edda memory schemas
│   │   ├── events.ts         # Stack event schemas
│   │   ├── type-mappings.ts  # Cross-layer mappings
│   │   └── ports/            # Interface definitions
│   │       ├── kindling.port.ts
│   │       ├── ember.port.ts
│   │       └── edda.port.ts
│   └── testing/              # 3,500+ lines
│       ├── index.ts          # Testing utilities entry
│       ├── mocks/            # Port mocks
│       │   ├── kindling.mock.ts
│       │   ├── ember.mock.ts
│       │   └── edda.mock.ts
│       ├── fixtures/         # Test data factories
│       │   ├── proposals.ts
│       │   └── memories.ts
│       └── validators/       # Provenance validators
│           └── provenance-chain.ts
├── package.json
├── tsconfig.json
└── vitest.config.ts
```

### 3.3 Contract Schemas Provided

| Schema               | Purpose                                | Export Path                   |
| -------------------- | -------------------------------------- | ----------------------------- |
| IdentifierSchemas    | UUID, ContentHash, ObservationId, etc. | `@eddacraft/anvil-edda-stack` |
| TemporalSchemas      | Timestamp, Duration, TimeRange, TTL    | `@eddacraft/anvil-edda-stack` |
| ConfidenceSchemas    | EmberConfidence, EddaConfidenceLevel   | `@eddacraft/anvil-edda-stack` |
| ProvenanceSchemas    | KindlingRef, EmberRef, ProvenanceChain | `@eddacraft/anvil-edda-stack` |
| EmberProposalSchemas | ProposalType, CandidateProposal        | `@eddacraft/anvil-edda-stack` |
| EddaMemorySchemas    | MemoryType, MemoryObject, Evolution    | `@eddacraft/anvil-edda-stack` |
| EventSchemas         | StackEvent, EventType, EventHandler    | `@eddacraft/anvil-edda-stack` |

### 3.4 Port Interfaces Provided

| Interface     | Purpose           | Methods                                             |
| ------------- | ----------------- | --------------------------------------------------- |
| IKindlingPort | Observation layer | recordObservation, queryObservations, querySessions |
| IEmberPort    | Proposal layer    | createProposal, queryProposals, updateProposal      |
| IEddaPort     | Memory layer      | promoteProposal, queryMemories, updateMemory        |

### 3.5 Testing Utilities Provided

| Utility                 | Purpose                        |
| ----------------------- | ------------------------------ |
| createMockKindlingPort  | Creates mock IKindlingPort     |
| createMockEmberPort     | Creates mock IEmberPort        |
| createMockEddaPort      | Creates mock IEddaPort         |
| createProposalFixture   | Factory for test proposals     |
| createMemoryFixture     | Factory for test memories      |
| validateProvenanceChain | Validates provenance integrity |

### 3.6 Integration with Anvil Packages

**Current State:** edda-stack is self-contained with no dependencies on core/.

**Target State Integration:**

```
                    ┌─────────────────────┐
                    │ @eddacraft/anvil-edda-stack   │
                    │ ├── contracts/      │
                    │ └── testing/        │
                    └──────────┬──────────┘
                               │
                    ┌──────────┴──────────┐
                    │                     │
           ┌────────▼────────┐   ┌────────▼────────┐
           │ @eddacraft/anvil-runtime  │   │ @eddacraft/anvil-cli      │
           │ (uses ports)    │   │ (uses testing)  │
           └────────┬────────┘   └─────────────────┘
                    │
           ┌────────▼────────┐
           │ @eddacraft/anvil-ports    │
           │ (re-exports     │
           │  stack ports)   │
           └─────────────────┘
```

**Recommended Actions:**

1. Keep edda-stack as standalone package (no merge into anvil/)
2. Re-export relevant ports from @eddacraft/anvil-ports for consistency
3. Use edda-stack/testing in CLI E2E tests

---

## 4. kindling-integration Disposition (MONO-000d)

### 4.1 Package Overview

**Location:** `packages/kindling-integration/` **Total Lines:** ~500 **Files:**
3

### 4.2 Current Contents

```
packages/kindling-integration/
└── src/
    ├── index.ts                # Re-exports
    ├── observation-contract.ts # 11 observation kinds
    └── query-contract.ts       # 4 query scopes
```

### 4.3 Observation Contract (observation-contract.ts)

Defines 11 observation kinds:

| Kind               | Schema                             | Purpose                     |
| ------------------ | ---------------------------------- | --------------------------- |
| session_start      | SessionStartObservationSchema      | Opens session capsule       |
| session_end        | SessionEndObservationSchema        | Closes session with outcome |
| plan_created       | PlanCreatedObservationSchema       | New plan authored           |
| plan_edited        | PlanEditedObservationSchema        | Plan modified               |
| plan_approved      | PlanApprovedObservationSchema      | Plan approved               |
| plan_rejected      | PlanRejectedObservationSchema      | Plan rejected               |
| action_executed    | ActionExecutedObservationSchema    | Action completed            |
| gate_evaluated     | GateEvaluatedObservationSchema     | Gate check result           |
| constraint_applied | ConstraintAppliedObservationSchema | Constraint enforced         |
| human_input        | HumanInputObservationSchema        | Human intervention          |
| error              | ErrorObservationSchema             | Error recorded              |

### 4.4 Query Contract (query-contract.ts)

Defines 4 query scopes:

| Scope   | Schema             | Purpose          |
| ------- | ------------------ | ---------------- |
| session | SessionQuerySchema | Query by session |
| plan    | PlanQuerySchema    | Query by plan    |
| gate    | GateQuerySchema    | Query by gate    |
| action  | ActionQuerySchema  | Query by action  |

### 4.5 Relationship to edda-stack

**Overlap Analysis:**

| Aspect              | kindling-integration | edda-stack              |
| ------------------- | -------------------- | ----------------------- |
| Observation schemas | Yes (11 kinds)       | No                      |
| Query schemas       | Yes (4 scopes)       | No                      |
| IKindlingPort       | No                   | Yes                     |
| Kindling types      | No                   | Yes (KindlingRef, etc.) |

**Complementary, Not Duplicate:**

- kindling-integration defines WHAT observations look like
- edda-stack defines HOW to interact with Kindling layer

### 4.6 Disposition Decision

**Decision: Merge kindling-integration INTO edda-stack**

**Rationale:**

1. Both packages are part of Kindling/Ember/Edda architecture
2. Observation schemas should be co-located with IKindlingPort
3. Reduces package count and import complexity
4. Eliminates potential circular dependency risk

**Migration Path:**

```bash
# Step 1: Create target directory
mkdir -p packages/edda-stack/src/contracts/kindling

# Step 2: Move files
mv packages/kindling-integration/src/observation-contract.ts \
   packages/edda-stack/src/contracts/kindling/
mv packages/kindling-integration/src/query-contract.ts \
   packages/edda-stack/src/contracts/kindling/

# Step 3: Create index.ts for kindling/ submodule
# Step 4: Update edda-stack/src/contracts/index.ts to re-export
# Step 5: Update any imports in the codebase
# Step 6: Delete packages/kindling-integration/
```

**New Export Structure:**

```typescript
// @eddacraft/anvil-edda-stack
export * from './contracts/kindling/observation-contract.js';
export * from './contracts/kindling/query-contract.js';
export * from './contracts/ports/kindling.port.js';
// ... rest of edda-stack exports
```

---

## 5. Dependency Graph

### 5.1 Current State (Simplified)

```
                    ┌─────────────┐
                    │    cli/     │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
              ▼            ▼            ▼
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │ adapters │ │   core   │ │   aps    │
        └────┬─────┘ └──────────┘ └────┬─────┘
             │                         │
             └────────────┬────────────┘
                          ▼
                    (no deps)
```

### 5.2 Target State (Layered)

```
                         ┌──────────────┐
                         │ apps/cli     │
                         └──────┬───────┘
                                │
            ┌───────────────────┼───────────────────┐
            │                   │                   │
            ▼                   ▼                   ▼
    ┌───────────────┐   ┌───────────────┐   ┌───────────────┐
    │ @eddacraft/anvil-runtime│   │ @eddacraft/anvil-policy │   │ adapters/*    │
    └───────┬───────┘   └───────┬───────┘   └───────┬───────┘
            │                   │                   │
            └─────────┬─────────┴─────────┬─────────┘
                      │                   │
                      ▼                   │
              ┌───────────────┐           │
              │  @eddacraft/anvil-core  │           │
              └───────┬───────┘           │
                      │                   │
                      ▼                   │
              ┌───────────────┐           │
              │  @eddacraft/anvil-ports │◄──────────┘
              └───────┬───────┘
                      │
                      ▼
              ┌───────────────┐
              │@eddacraft/anvil-contracts│
              └───────────────┘

    ┌─────────────────────────────────────────────────────────┐
    │                   @eddacraft/anvil-edda-stack                     │
    │  (standalone, provides contracts + testing utilities)   │
    └─────────────────────────────────────────────────────────┘
```

### 5.3 Dependency Rules

| Package                     | May Depend On                  | Must Not Depend On        |
| --------------------------- | ------------------------------ | ------------------------- |
| @eddacraft/anvil-contracts  | None                           | Everything                |
| @eddacraft/anvil-ports      | contracts                      | core, runtime, policy     |
| @eddacraft/anvil-core       | contracts, ports               | runtime, policy, adapters |
| @eddacraft/anvil-policy     | contracts                      | core, runtime, adapters   |
| @eddacraft/anvil-runtime    | contracts, ports, core, policy | apps                      |
| @eddacraft/anvil-edda-stack | None (standalone)              | anvil/\* packages         |
| apps/\*                     | Any package                    | Other apps                |

---

## 6. Risk Assessment

### 6.1 Identified Risks

| Risk                                      | Likelihood | Impact | Mitigation                     |
| ----------------------------------------- | ---------- | ------ | ------------------------------ |
| gate/ split creates circular deps         | Medium     | High   | Careful interface extraction   |
| edda-stack integration breaks tests       | Low        | Medium | edda-stack is self-contained   |
| kindling-integration merge breaks imports | Medium     | Low    | Update all 3 import sites      |
| Placeholder directories cause confusion   | Low        | Low    | Document actual vs placeholder |

### 6.2 Open Questions Resolved

| Question                                        | Resolution                             |
| ----------------------------------------------- | -------------------------------------- |
| Is edda-stack in scope?                         | Yes, migration only (not new features) |
| What to do with kindling-integration?           | Merge into edda-stack                  |
| Are placeholder directories migration blockers? | No, scaffolds already exist            |

---

## 7. Recommendations

### 7.1 Immediate Actions

1. **Update plan document** - Mark Phase 0 tasks complete
2. **Update Current State** - Reflect actual codebase state
3. **Proceed to Phase 1** - With accurate boundaries

### 7.2 Phase 1 Considerations

Based on discovery:

1. **Nx generators** should handle gate/ split specially
2. **Codemod** must account for edda-stack exports
3. **kindling-integration merge** should happen early in Phase 2

### 7.3 Testing Strategy

- Run full test suite after any file moves
- edda-stack tests are self-contained (10 test files)
- core/ tests may need path updates after split

---

## Appendix A: File Counts

| Directory                          | .ts Files | .test.ts Files | Total Lines |
| ---------------------------------- | --------- | -------------- | ----------- |
| core/src/                          | 68        | 35             | 41,580      |
| packages/edda-stack/src/           | 21        | 10             | 10,422      |
| packages/kindling-integration/src/ | 3         | 0              | ~500        |
| cli/src/                           | ~40       | ~15            | ~6,000      |

## Appendix B: Related Documents

- [Monorepo Migration Plan](../modules/monorepo-migration.aps.md)
- [Monorepo Cleanup Impact Assessment](./monorepo-cleanup-impact-assessment.md)
- [Edda Stack Documentation](../../packages/edda-stack/README.md) (if exists)

---

_Discovery completed: 2026-01-14_ _Next phase: MONO-001 (Nx generators)_
