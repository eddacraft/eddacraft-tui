# Edda Stack Architecture

| Type | Authority | Owner | Status | Freshness                                        |
| ---- | --------- | ----- | ------ | ------------------------------------------------ |
| Spec | Derived   | EDDA  | Live   | Metadata backfilled 2026-05-27 during DOCGOV-011 |

| Upstream                                                    | Downstream                                       |
| ----------------------------------------------------------- | ------------------------------------------------ |
| Edda Stack public docs, Kindling and Ember package surfaces | Public Edda Stack docs, memory architecture docs |

> A three-layer architecture that governs how activity becomes memory.

> **Implementation status:** This describes the design contract. The
> `packages/edda-stack` TypeScript surface is a **partial implementation**
> (Edda + Ember present; Kindling capture via `packages/kindling-integration`)
> and is **retiring** as operational memory moves to the Rust Kindling path.
> [`overview.md`](overview.md) states the same partial/retiring status.

## Overview

The Edda Stack separates concerns that most systems collapse:

- **Observation** (what happened) - Kindling
- **Interpretation** (what might matter) - Ember
- **Memory** (what we know) - Edda

This separation prevents the common failure mode where logs become memory,
memory becomes noisy, and noise becomes institutional truth.

## Philosophy

```
Kindling observes — captures without judgement
Ember reflects — meaning without authority
Edda remembers — memory with restraint
```

**Each layer is intentionally limited:**

1. Kindling cannot judge (facts only)
2. Ember cannot decide (proposals only)
3. Edda cannot speculate (curated truths only)

**Meaning emerges only through their separation.**

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                        Edda Stack                           │
│                   (Memory Architecture)                     │
└─────────────────────────────────────────────────────────────┘

                    ┌──────────────┐
                    │    Edda      │  ← Canonical memory (high-trust)
                    │  (Ledger)    │     Git-backed, versioned, auditable
                    └──────┬───────┘
                           │ Promotion (human decision)
                    ┌──────▼───────┐
                    │    Ember     │  ← Candidate memory (medium-trust)
                    │   (Queue)    │     SQLite, ephemeral, decays
                    └──────┬───────┘
                           │ Aggregation + Evaluation
                    ┌──────▼───────┐
                    │   Kindling   │  ← Observations (facts only)
                    │  (Camera)    │     Write-only, read-only query
                    └──────────────┘
```

## Data Flow

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

**Truth flows one way:** Kindling → Ember → Edda

## Layer Details

### Kindling — The Camera

**Role:** Capture without judgement

Kindling is the sensory layer. It observes what happens and records it
faithfully, without interpretation.

| Aspect          | Description                                        |
| --------------- | -------------------------------------------------- |
| **Question**    | "What occurred?"                                   |
| **Input**       | Agent activity, tool usage, communications, errors |
| **Output**      | Structured observations (11 kinds)                 |
| **Storage**     | SQLite, local                                      |
| **AI Usage**    | None - deliberately naïve                          |
| **Trust Level** | Facts only (no interpretation)                     |

**Observation Kinds:**

1. `gate_evaluated` - Rule evaluation results
2. `action_executed` - Tool/command execution
3. `decision_made` - Agent decisions
4. `error_occurred` - Error events
5. `conversation_turn` - Conversation events
6. `file_modified` - File system changes
7. `context_switched` - Context changes
8. `resource_accessed` - External resource access
9. `assertion_made` - Agent assertions
10. `feedback_received` - User feedback
11. `session_event` - Session lifecycle

### Ember — The Curator

**Role:** Meaning without authority

Ember is the interpretive layer. It sits between raw observation and durable
memory, looking for candidate meaning.

| Aspect          | Description                      |
| --------------- | -------------------------------- |
| **Question**    | "Might this matter later?"       |
| **Input**       | Observations from Kindling       |
| **Output**      | Candidate proposals (6 types)    |
| **Storage**     | SQLite, ephemeral                |
| **AI Usage**    | Optional (must not depend on it) |
| **Trust Level** | Medium (proposals may be wrong)  |

**Proposal Types:**

1. `decision` - Potential decisions to codify
2. `pattern` - Recurring patterns observed
3. `warning` - Potential issues to remember
4. `lesson` - Lessons learned
5. `anomaly` - Unexpected behaviours
6. `constraint` - Constraints discovered

**Key Characteristics:**

- Ephemeral by default (TTL-based decay)
- Allowed to be wrong (probabilistic thinking)
- Breaking Ember data is acceptable
- Has taste, but no power

### Edda — The Ledger

**Role:** Memory with restraint

Edda is the canonical memory layer. It stores only what has crossed a deliberate
threshold.

| Aspect          | Description                                  |
| --------------- | -------------------------------------------- |
| **Question**    | "What do we know to be true enough to keep?" |
| **Input**       | Promoted proposals from Ember                |
| **Output**      | Canonical memories                           |
| **Storage**     | Git-backed YAML/JSON                         |
| **AI Usage**    | AI-assisted, not AI-authored                 |
| **Trust Level** | High (curated truths)                        |

**Memory Types:**

1. `decision` - Codified decisions
2. `pattern` - Established patterns
3. `constraint` - Known constraints
4. `warning` - Persistent warnings
5. `doctrine` - Guiding principles
6. `lesson` - Lessons learned

**Key Characteristics:**

- Low-volume, high-trust
- Versioned, append-biased, auditable
- Forgets aggressively by default
- Breaking Edda is unacceptable

## Shared Contracts

The `@eddacraft/anvil-edda-stack/contracts` package defines shared types:

### Identifiers

```typescript
type ObservationId = string; // UUID
type ProposalId = string; // UUID
type MemoryId = string; // UUID
type SessionId = string; // UUID
```

### Timestamps

```typescript
type Timestamp = string; // ISO8601 format
```

### Confidence Levels

**Ember (numeric):** 0.0 - 1.0 scale

**Edda (semantic):**

- `high` - Explicitly decided, well-established
- `medium` - Observed pattern, reasonable confidence
- `low` - Emerging pattern, tentative
- `inferred` - AI-derived, requires validation

### Provenance Chain

```typescript
interface ProvenanceChain {
  ember_source?: {
    proposal_id: ProposalId;
    proposal_type: ProposalType;
    confidence: number;
    created_at: Timestamp;
  };
  kindling_sources: KindlingSource[];
  source_sessions: SessionId[];
}
```

## Port Interfaces

Each layer exposes a port interface for interaction:

### IKindlingPort

```typescript
interface IKindlingPort {
  // Query observations
  getObservation(id: ObservationId): Promise<Observation | null>;
  queryObservations(query: ObservationQuery): Promise<ObservationResult>;
  getObservationsBySession(sessionId: SessionId): Promise<Observation[]>;

  // Status
  isAvailable(): Promise<boolean>;
  getStats(): Promise<KindlingStats>;
}
```

### IEmberPort

```typescript
interface IEmberPort {
  // Proposal management
  createProposal(input: CreateProposalInput): Promise<CandidateProposal>;
  getProposal(id: ProposalId): Promise<CandidateProposal | null>;
  queryProposals(query: ProposalQuery): Promise<ProposalQueryResult>;

  // Lifecycle
  extendProposalTTL(id: ProposalId, ms: number): Promise<void>;
  dismissProposal(id: ProposalId, reason: string): Promise<void>;
  markAsPromoted(id: ProposalId, memoryId: MemoryId): Promise<void>;

  // Status
  isAvailable(): Promise<boolean>;
  getStats(): Promise<EmberStats>;
}
```

### IEddaPort

```typescript
interface IEddaPort {
  // Memory management
  promoteProposal(input: PromoteProposalInput): Promise<MemoryObject>;
  createMemory(input: CreateMemoryInput): Promise<MemoryObject>;
  updateMemory(
    id: MemoryId,
    input: UpdateMemoryInput
  ): Promise<MemoryObject | null>;
  retireMemory(
    id: MemoryId,
    input: RetireMemoryInput
  ): Promise<MemoryObject | null>;

  // Queries
  getMemory(id: MemoryId): Promise<MemoryObject | null>;
  queryMemories(query: MemoryQuery): Promise<MemoryQueryResult>;
  getActiveMemories(): Promise<MemoryObject[]>;

  // Evolution
  supersedeMemory(
    oldId: MemoryId,
    newInput: CreateMemoryInput
  ): Promise<{ old: MemoryObject; new: MemoryObject }>;
  getEvolutionChain(id: MemoryId): Promise<MemoryObject[]>;

  // Provenance
  resolveProvenance(
    chain: ProvenanceChain
  ): Promise<ProvenanceResolutionResult>;

  // Status
  isAvailable(): Promise<boolean>;
  getStats(): Promise<EddaStats>;
}
```

## Configuration

Stack configuration lives in the Anvil project config file — `.anvilrc`
(migrating to the `.anvil.<ext>` form), the same config surface the rest of
Anvil reads under the project's `.anvil/` configuration area:

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

### CLI Commands

```bash
# Show status of all layers
anvil stack status

# Validate configuration and provenance
anvil stack validate
```

## Layer Dependencies

```
edda → ember → kindling
```

- Edda requires Ember (for promotion workflow)
- Ember requires Kindling (for observation aggregation)
- Kindling is standalone

When enabling layers, enable from bottom up:

1. Enable Kindling first
2. Then enable Ember
3. Finally enable Edda

## Failure Modes Avoided

| Anti-pattern                            | How the Stack Prevents It                              |
| --------------------------------------- | ------------------------------------------------------ |
| Log-as-memory collapse                  | Kindling is logs, Edda is memory (explicit separation) |
| AI hallucinations becoming "facts"      | Promotion requires human decision                      |
| Memory inflation                        | Ember decays, Edda requires justification              |
| Silent drift of institutional knowledge | Git-backed, versioned, evolution graph                 |
| Agent feedback loops reinforcing errors | Read-only enforcement in Kindling                      |

## Testing Utilities

The `@eddacraft/anvil-edda-stack/testing` package provides:

### Mocks

```typescript
import {
  createMockKindlingPort,
  createMockEmberPort,
  createMockEddaPort,
} from '@eddacraft/anvil-edda-stack/testing';

// Create isolated mocks for testing
const kindling = createMockKindlingPort();
const ember = createMockEmberPort();
const edda = createMockEddaPort();
```

### Fixtures

```typescript
import {
  createValidDecisionProposal,
  createValidPatternMemory,
  createEvolutionChain,
} from '@eddacraft/anvil-edda-stack/testing';

// Create valid test data
const proposal = createValidDecisionProposal();
const memory = createValidPatternMemory();
const chain = createEvolutionChain(3); // 3-level chain
```

### Validators

```typescript
import { validateProvenanceChain } from '@eddacraft/anvil-edda-stack/testing';

// Validate provenance integrity
const result = validateProvenanceChain(chain, { kindling, ember });
```

## Mental Models

```
Kindling is a camera — it sees everything, understands nothing
Ember is a curator — it proposes, but cannot decide
Edda is a ledger — you never casually rewrite it

Ember is a queue that empties itself
Edda is a ledger that persists

If you can't explain why something is in Edda, it doesn't belong.
```

## Related Documentation

- [Kindling Integration](../../plans/archive/modules/kindling-integration.aps.md)
- [Ember System](../../plans/archive/modules/ember.aps.md)
- [Edda System](../../plans/archive/modules/edda.aps.md)
- [Stack Integration](../../plans/archive/modules/edda-stack-integration.aps.md)
