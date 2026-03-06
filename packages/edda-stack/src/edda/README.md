# Edda — Canonical Memory System

Edda is the final layer of the Kindling · Ember · Edda stack. It stores only
what a human has deliberately chosen to keep: decisions, patterns, constraints,
warnings, doctrines, and lessons. Every memory has an author, a reason, a
confidence level, and a provenance chain tracing back to the Ember proposal and
Kindling observations that originated it.

> Edda is not a log. It is not a transcript. It is not a database of everything.
> It is an institutional memory: decisions, lessons, patterns, constraints,
> truths. Edda forgets aggressively by default. That is a feature, not a bug.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                          Edda                               │
│                  (Canonical Memory Layer)                   │
└─────────────────────────────────────────────────────────────┘

  ┌─────────────┐   promote <ember-id>
  │    Ember    │ ─────────────────────────────┐
  │  (Proposals)│   (human decision required)  │
  └─────────────┘                              ▼
                                   ┌────────────────────────┐
                                   │   PromotionService     │
                                   │   (validates input,    │
                                   │    creates memory)     │
                                   └────────────┬───────────┘
                                                │
                              ┌─────────────────┼──────────────────┐
                              ▼                 ▼                  ▼
                   ┌──────────────────┐  ┌────────────┐  ┌──────────────────┐
                   │ ProvenanceService│  │MemoryStore │  │ EvolutionService │
                   │ (links back to   │  │(git-backed │  │ (supersedes,     │
                   │  Kindling,Ember) │  │  YAML)     │  │  retire, trace)  │
                   └──────────────────┘  └────────────┘  └──────────────────┘
                                                │
                                                ▼
                                   ┌────────────────────────┐
                                   │     MemoryService      │
                                   │  (orchestration API)   │
                                   └────────────────────────┘
```

## File Structure

```
packages/edda-stack/src/edda/
├── memory-service.ts      # High-level orchestration API (start here)
├── memory-store.ts        # Git-backed YAML storage implementation
├── promotion-service.ts   # Validates and creates memories from proposals
├── provenance-service.ts  # Resolves and validates provenance chains
├── evolution-service.ts   # Supersede, retire, and trace memory history
├── version-tracker.ts     # Git commit integration for audit trail
├── serialisation.ts       # YAML serialise/deserialise for memory files
├── store-interfaces.ts    # IMemoryStoreOperations, IVersionTracker
├── config.ts              # Zod schema for EddaConfig + defaults
└── README.md
```

## Storage Format

Edda stores memories as YAML files under `.anvil/edda/`, organised by type. The
directory is intended to be tracked in version control, giving every change a
permanent, auditable git history.

```
.anvil/edda/
├── memories/
│   ├── decision/
│   ├── pattern/
│   ├── constraint/
│   ├── warning/
│   ├── doctrine/
│   └── lesson/
├── index.yaml
└── .git/
```

Each memory is a single file named `<memory-id>.yaml`. The `index.yaml` contains
lightweight index entries for fast querying without loading every file. Memory
files are the source of truth; the index is derived from them.

### Memory file example

```yaml
id: mem_a1b2c3d4
type: decision
status: active
schema_version: '1.0'
statement: >
  All cross-package imports use published package names, never relative paths.
context:
  why: >
    Relative paths across package boundaries break under pnpm workspace hoisting
    and produce runtime errors.
  when: '2026-03-01T14:23:00Z'
  scope: monorepo
  conditions: []
  tags:
    - imports
    - monorepo
    - esm
confidence: high
confidence_rationale:
  Confirmed by multiple build failures; rule enforced by ESLint.
attribution:
  actor: alice
  timestamp: '2026-03-01T14:25:00Z'
  method: cli_command
  reason: >
    Pattern appeared in three Ember proposals; confirmed during onboarding
    incident review.
provenance:
  source_sessions:
    - ses_xyz001
  kindling_sources:
    - observation_id: obs_abc001
    - observation_id: obs_abc002
  ember_source:
    proposal_id: prop_ember_001
    proposal_type: pattern
    confidence: 0.82
    created_at: '2026-02-28T09:00:00Z'
evolution:
  supersedes: []
created_at: '2026-03-01T14:25:00Z'
```

## Key Concepts

### Memory Types

Edda supports six fixed memory types. The type is chosen at promotion time and
determines where the file is stored and how the memory is interpreted.

| Type         | Meaning                                              |
| ------------ | ---------------------------------------------------- |
| `decision`   | A choice made with observable, lasting consequences  |
| `pattern`    | A recurring structure or behaviour worth encoding    |
| `constraint` | A discovered limitation or hard boundary             |
| `warning`    | A signal of potential problems or known failure mode |
| `doctrine`   | A principle or rule that governs how work is done    |
| `lesson`     | A learning extracted from failure or success         |

Types are fixed in v1. The set covers the majority of institutional memory needs
without introducing an extensible type system that would complicate querying and
display.

### Memory Lifecycle

```
active ──► superseded  (replaced by a newer memory)
       ──► retired     (withdrawn without replacement)
```

Memories are never hard-deleted. Retirement and supersession are soft
operations: the file remains on disk and in the index with its status updated.
This preserves the full audit trail and makes rollback possible.

### Confidence

Confidence in Edda is human-asserted, not computed. When promoting a proposal,
the operator assigns one of three levels:

| Level    | Meaning                                                 |
| -------- | ------------------------------------------------------- |
| `low`    | Plausible but unverified; treat as a working hypothesis |
| `medium` | Supported by evidence; likely correct in scope          |
| `high`   | Confirmed by multiple sources or direct human review    |

Ember's numeric confidence score (0.0–1.0) informs the decision but does not
become the Edda confidence level automatically. A human assigns confidence
explicitly at promotion time.

### Provenance

Every memory links back to its origin:

- `kindling_sources` — the raw Kindling observation IDs that triggered the Ember
  proposal
- `source_sessions` — the session IDs during which the observations occurred
- `ember_source` — the Ember proposal that was promoted, including its type,
  numeric confidence, and creation timestamp

Provenance is immutable. It records where a memory came from, not what it
currently says. If a memory is updated, its provenance still refers to the
original promotion event.

### Evolution

The `evolution` field tracks how a memory changes over time:

- `supersedes` — list of memory IDs this memory replaces
- `superseded_by` — set when this memory is replaced by a newer one
- `retired_at`, `retired_reason`, `retired_by` — set when the memory is
  explicitly withdrawn

Use `EvolutionService.getEvolutionChain(id)` to walk the full history of a
memory, from its first version to the current one. Use `getLatestVersion(id)` to
resolve any memory ID to the current active version.

### Attribution

Attribution is mandatory on every write operation. All methods that mutate state
require an `actor` (the person or system making the change) and a `reason`. This
is enforced at the service layer, not just at the schema level.

## Service Layer

### MemoryService

`MemoryService` is the main entry point for all Edda operations. It delegates to
the specialised services below and is the only surface that should be used from
outside this module.

```typescript
import { MemoryService } from '@eddacraft/anvil-edda-stack/edda';
```

#### Constructor

```typescript
new MemoryService(deps: MemoryServiceDeps)
```

| Parameter                | Type                     | Required | Description                   |
| ------------------------ | ------------------------ | -------- | ----------------------------- |
| `deps.store`             | `IMemoryStoreOperations` | Yes      | Storage implementation        |
| `deps.promotionService`  | `PromotionService`       | Yes      | Handles promotion validation  |
| `deps.provenanceService` | `ProvenanceService`      | Yes      | Handles provenance resolution |
| `deps.evolutionService`  | `EvolutionService`       | Yes      | Handles supersede and retire  |
| `deps.versionTracker`    | `IVersionTracker`        | No       | Git commit integration        |

#### Key methods

| Method                     | Description                                        |
| -------------------------- | -------------------------------------------------- |
| `promoteProposal(input)`   | Promotes an Ember proposal to an Edda memory       |
| `createMemory(input)`      | Creates a memory directly, without a proposal      |
| `updateMemory(id,input)`   | Updates statement, context, or confidence          |
| `retireMemory(id,input)`   | Soft-deletes a memory with a reason                |
| `supersedeMemory(old,new)` | Replaces a memory and links the evolution chain    |
| `getMemory(id)`            | Fetches a single memory by ID                      |
| `queryMemories(query)`     | Queries with type, status, confidence, tag filters |
| `getActiveMemories()`      | Returns all active memories                        |
| `getEvolutionChain(id)`    | Returns the full version history of a memory       |
| `getLatestVersion(id)`     | Resolves a memory ID to the current active version |
| `resolveProvenance(chain)` | Validates a provenance chain against live data     |
| `getStats()`               | Returns counts by status, type, and confidence     |
| `exportMemories()`         | Returns all memories as a serialisable array       |
| `importMemories(arr)`      | Bulk-imports memories (use for migrations only)    |

### PromotionService

Validates promotion input, checks Ember confidence thresholds, creates the
`MemoryObject`, and marks the source proposal as `promoted` in Ember.

Key constraints enforced by `PromotionService`:

- A reason is required unless `promotion.require_reason` is `false`
- Attribution (`promoted_by`) is required unless `promotion.require_attribution`
  is `false`
- Ember confidence must meet or exceed `promotion.min_ember_confidence` (default
  `0.5`)

### ProvenanceService

Validates and resolves provenance chains. Given a `ProvenanceChain`, it checks
that the referenced Ember proposal still exists and that all required fields are
present. Returns a `ProvenanceResolutionResult` with counts of resolved vs.
missing links and any warnings.

### EvolutionService

Manages memory history. Provides:

- `supersedeMemory(oldId, newInput)` — creates a new memory, retires the old one
  with `status: superseded`, and links the two via `supersedes` /
  `superseded_by`
- `retireMemory(id, input)` — marks a memory as `retired` with a reason and the
  actor who retired it
- `getEvolutionChain(id)` — walks the chain from the oldest ancestor to the
  newest descendant
- `getLatestVersion(id)` — follows `superseded_by` links to find the current
  active version

### MemoryStore

File-system implementation of `IMemoryStoreOperations`. Reads and writes YAML
files under the configured storage path. Maintains `index.yaml` for fast queries
without loading every memory file.

`MemoryStore` does not interact with git directly. All git integration is
handled by `VersionTracker` (injected as `IVersionTracker`). This separation
means the store can be used without git in tests.

## Configuration Reference

Configuration lives under the `edda` key in `.anvilrc`. All fields have defaults
and can be omitted.

```json
{
  "edda": {
    "enabled": true,
    "storage": {
      "type": "git",
      "path": ".anvil/edda/",
      "format": "yaml"
    },
    "promotion": {
      "require_reason": true,
      "require_attribution": true,
      "min_ember_confidence": 0.5
    },
    "limits": {
      "max_statement_length": 2000,
      "max_context_length": 10000
    }
  }
}
```

| Field                            | Default        | Description                             |
| -------------------------------- | -------------- | --------------------------------------- |
| `edda.enabled`                   | `true`         | Enable or disable the Edda layer        |
| `storage.type`                   | `"git"`        | Storage backend (only `git` in v1)      |
| `storage.path`                   | `.anvil/edda/` | Path to the memory store directory      |
| `storage.format`                 | `"yaml"`       | File format (only `yaml` in v1)         |
| `promotion.require_reason`       | `true`         | Require a reason on every promotion     |
| `promotion.require_attribution`  | `true`         | Require an actor on every promotion     |
| `promotion.min_ember_confidence` | `0.5`          | Minimum Ember confidence to allow       |
| `limits.max_statement_length`    | `2000`         | Maximum characters in a statement       |
| `limits.max_context_length`      | `10000`        | Maximum characters in the context block |

## Schema Version

The current schema version is `1.0`. It is recorded on every memory file in the
`schema_version` field. The constant `MEMORY_SCHEMA_VERSION` is exported from
`contracts/index.ts`.

Schema version changes require a migration pass over all memory files. A
`migration/` subdirectory exists in this module for future migration scripts.
There are no migrations in v1.

## Boundary Rules

The following constraints are non-negotiable and enforced by the service layer:

1. **Human-in-the-loop.** No memory is created without a human actor. Ember
   cannot promote itself.
2. **Attribution mandatory.** Every write carries an actor string. This is
   required, not optional.
3. **Confidence human-asserted.** Ember numeric confidence informs the decision;
   it does not become the Edda confidence level automatically.
4. **Soft delete only.** Memories are never hard-deleted. Retire or supersede
   instead.
5. **Provenance immutable.** Once recorded, provenance links cannot be updated.
   They are part of the audit trail.

## Design Decisions

The following decisions from the APS plan govern this module:

**D-EDDA-001 — Git-backed YAML storage.** Edda memories must be diffable,
mergeable, and auditable. YAML under a git-tracked directory provides this
without a database dependency. The trade-off is query performance, which is
acceptable given low expected volume.

**D-EDDA-002 — Human-in-the-loop promotion.** No path exists from Ember to Edda
that does not require an explicit human action. This is the core safety
guarantee of the stack.

**D-EDDA-003 — Soft delete only.** Memories must be retirable but never
erasable. Hard deletion would break provenance chains and undermine the audit
trail. Retired memories remain on disk indefinitely.

**D-EDDA-004 — Memory types are fixed (v1).** Six types cover the majority of
institutional memory use cases. Extensible type systems add querying complexity
that v1 does not justify.

**D-EDDA-005 — Confidence is human-asserted, not computed.** Ember confidence is
probabilistic and heuristic. Edda confidence must reflect human judgement. The
two are related but not equivalent.
