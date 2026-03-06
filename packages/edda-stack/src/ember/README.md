# Ember — Candidate Memory System

Ember is the interpretive layer between raw observation (Kindling) and durable
memory (Edda). It consumes high-volume, low-trust signals from Kindling and
produces low-volume, medium-trust candidate proposals for potential promotion to
Edda. Ember scores observations using heuristic rules, assigns expiry deadlines
to all proposals, and surfaces patterns that might be worth remembering —
without ever claiming authority over what gets kept.

> Ember has taste, but no power. It proposes memory. It does not create it.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Ember                               │
│                  (Candidate Memory Layer)                   │
└─────────────────────────────────────────────────────────────┘

  ┌─────────────┐     session_completed event
  │   Kindling  │ ─────────────────────────────┐
  │  (Camera)   │                              │
  └─────────────┘                              ▼
                                    ┌────────────────────┐
                                    │  ObservationHook   │
                                    │  (event listener)  │
                                    └────────┬───────────┘
                                             │
                                             ▼
  ┌─────────────┐     groups      ┌────────────────────┐
  │  Kindling   │ ──────────────► │ AggregatorService  │
  │  Port (API) │                 │ (groups obs.)      │
  └─────────────┘                 └────────┬───────────┘
                                           │
                                           ▼
                                ┌────────────────────────┐
                                │    EvaluatorService    │
                                │  (runs rules, scores)  │
                                │                        │
                                │  ┌──────────────────┐  │
                                │  │  repetition.rule │  │
                                │  │  escalation.rule │  │
                                │  │  resolution.rule │  │
                                │  │  convergence.rule│  │
                                │  │  surprise.rule   │  │
                                │  └──────────────────┘  │
                                └────────────┬───────────┘
                                             │ confidence ≥ min_confidence
                                             ▼
                                ┌────────────────────────┐
                                │   CandidateService     │
                                │   (orchestration API)  │
                                └────────────┬───────────┘
                                             │
                                             ▼
  ┌─────────────┐   promotes     ┌────────────────────────┐
  │    Edda     │ ◄─────────────  │    ProposalStore       │
  │  (Ledger)   │    (human)      │   (SQLite, TTL-based)  │
  └─────────────┘                └────────────────────────┘
                                             │
                                    DecayService runs
                                    on interval, expires
                                    unreviewed proposals
```

## File Structure

```
packages/edda-stack/src/ember/
├── candidate-service.ts    # High-level orchestration API (start here)
├── proposal-store.ts       # SQLite-backed IEmberPort implementation
├── aggregator-service.ts   # Groups Kindling observations into clusters
├── evaluator-service.ts    # Rule engine that scores observation groups
├── decay-service.ts        # Runs TTL expiry and proposal pruning
├── observation-hook.ts     # Subscribes to session_completed events
├── config.ts               # Zod schema for EmberConfig + defaults
├── rules/
│   ├── index.ts            # createDefaultRules() factory
│   ├── repetition.rule.ts  # Fires when a pattern recurs above threshold
│   ├── escalation.rule.ts  # Fires when severity increases within a window
│   ├── resolution.rule.ts  # Fires on failure-to-success transitions
│   ├── convergence.rule.ts # Fires when multiple sessions share a pattern
│   └── surprise.rule.ts    # Fires on anomalous kinds or unusual timing
└── README.md
```

## Key Concepts

### Proposals

A `CandidateProposal` is Ember's unit of output. It represents something Ember
thinks might be worth remembering. Proposals are:

- **Ephemeral** — they expire after `ttl_days` unless promoted
- **Heuristic** — confidence scores are not probabilities; they indicate
  relative strength of evidence
- **Traceable** — every proposal links back to source Kindling observation IDs
- **Typed** — one of six fixed types (see below)

**Proposal types:**

| Type         | Meaning                                         |
| ------------ | ----------------------------------------------- |
| `decision`   | A choice made with observable consequences      |
| `pattern`    | A recurring structure or behaviour              |
| `warning`    | A signal of potential problems                  |
| `lesson`     | A learning extracted from failure or success    |
| `anomaly`    | An unexpected deviation from expected behaviour |
| `constraint` | A discovered limitation or boundary             |

**Proposal lifecycle:**

```
active ──► promoted  (human promotes to Edda)
       ──► dismissed (human explicitly dismisses)
       ──► expired   (TTL elapsed without action)
```

Only `active` proposals are returned by `getActiveProposals()`. The decay cycle
runs `processExpiredProposals()` to transition stale proposals.

### Evaluation Rules

Rules implement the `EvaluationRule` interface and receive an `ObservationGroup`
and `EvaluationContext`. Each rule returns:

- `fired: boolean` — whether the rule triggered
- `contribution: number` — confidence contribution (0.0–1.0)
- `context?: Record<string, unknown>` — rule-specific diagnostics

`EvaluatorService` collects contributions from all fired rules, applies per-rule
weights, and computes a weighted confidence score. Rules that do not fire
contribute nothing.

### Decay

Every proposal has an `expires_at` timestamp derived from `ttl_days`.
`DecayService` provides:

- `processExpired()` — marks proposals whose `expires_at` has passed
- `pruneOld(days)` — hard-deletes resolved proposals older than N days
- `run()` — runs both in sequence (use for scheduled maintenance)

The default TTL is 30 days. Proposals are not silently deleted on expiry — they
are marked `expired` first, so queries can distinguish "never reviewed" from
"promoted".

### Aggregation

`AggregatorService` takes raw Kindling observations and groups them into
`ObservationGroup` clusters using three strategies:

- **By kind** — groups observations with the same `kind` field
- **By temporal proximity** — clusters observations within a 5-minute window
- **Repetition detection** — groups observations with the same kind + summary
  fingerprint that appear at or above threshold

Overlapping groups are merged. The final list is deduplicated and sorted by
count descending.

## Configuration Reference

Configuration lives under the `ember` key in `.anvilrc`. All fields have
defaults and can be omitted.

```json
{
  "ember": {
    "enabled": true,
    "database": ".anvil/ember.db",
    "decay": {
      "default_ttl_days": 30,
      "min_ttl_days": 7,
      "max_ttl_days": 90
    },
    "evaluation": {
      "min_confidence": 0.3,
      "repetition_threshold": 3,
      "escalation_window_hours": 24
    },
    "limits": {
      "max_candidates": 1000,
      "max_proposal_size_kb": 64
    }
  }
}
```

| Field                                | Default           | Description                              |
| ------------------------------------ | ----------------- | ---------------------------------------- |
| `ember.enabled`                      | `true`            | Enable or disable the Ember layer        |
| `ember.database`                     | `.anvil/ember.db` | Path to SQLite database file             |
| `decay.default_ttl_days`             | `30`              | Default proposal lifetime in days        |
| `decay.min_ttl_days`                 | `7`               | Minimum TTL (lower values are clamped)   |
| `decay.max_ttl_days`                 | `90`              | Maximum TTL (higher values are clamped)  |
| `evaluation.min_confidence`          | `0.3`             | Proposals below this score are discarded |
| `evaluation.repetition_threshold`    | `3`               | Observations required to fire repetition |
| `evaluation.escalation_window_hours` | `24`              | Window for escalation detection          |
| `limits.max_candidates`              | `1000`            | Hard cap on total stored proposals       |
| `limits.max_proposal_size_kb`        | `64`              | Maximum size of a single proposal        |

## API Reference: CandidateService

`CandidateService` is the main entry point for all Ember operations.

```typescript
import { CandidateService } from '@eddacraft/anvil-edda-stack/ember';
import { ProposalStore } from '@eddacraft/anvil-edda-stack/ember';

const store = new ProposalStore('.anvil/ember.db');
const service = new CandidateService({ store });
```

### Constructor

```typescript
new CandidateService(deps: CandidateServiceDeps)
```

| Parameter           | Type                  | Required | Description                                   |
| ------------------- | --------------------- | -------- | --------------------------------------------- |
| `deps.store`        | `IEmberPort`          | Yes      | Proposal storage implementation               |
| `deps.kindlingPort` | `IKindlingPort`       | No       | Kindling API for observation queries          |
| `deps.eventBus`     | `IStackEventBus`      | No       | Event bus for publish/subscribe               |
| `deps.config`       | `EmberServiceConfig`  | No       | Override defaults                             |
| `deps.aggregator`   | `CandidateAggregator` | No       | Custom aggregator (defaults to group-by-kind) |
| `deps.evaluator`    | `CandidateEvaluator`  | No       | Custom evaluator (defaults to count-based)    |

### Methods

#### `processSession(sessionId: string): Promise<CandidateProposal[]>`

Fetches observations for a session from Kindling, aggregates them into groups,
evaluates each group using the configured evaluator, and creates proposals for
groups that meet the confidence threshold. Returns proposals created in this
run.

Requires `deps.kindlingPort` to be set. Returns `[]` if not.

#### `createProposal(input: CreateProposalInput): Promise<CandidateProposal>`

Creates a proposal directly, bypassing observation processing. Enforces
`max_candidates` limit and clamps confidence and TTL to configured bounds.
Publishes a `proposal_created` event if `deps.eventBus` is set.

#### `getProposal(id: string): Promise<CandidateProposal | null>`

Returns a single proposal by ID, or `null` if not found.

#### `queryProposals(query: ProposalQuery): Promise<ProposalQueryResult>`

Queries proposals with optional filters:

| Filter            | Type                                           | Description                                |
| ----------------- | ---------------------------------------------- | ------------------------------------------ |
| `types`           | `ProposalType[]`                               | Filter to specific proposal types          |
| `statuses`        | `ProposalStatus[]`                             | Filter to specific statuses                |
| `min_confidence`  | `number`                                       | Minimum confidence threshold               |
| `created_after`   | `Timestamp`                                    | Created after this ISO timestamp           |
| `created_before`  | `Timestamp`                                    | Created before this ISO timestamp          |
| `include_expired` | `boolean`                                      | Include expired proposals (default: false) |
| `session_id`      | `string`                                       | Filter to proposals from a session         |
| `limit`           | `number`                                       | Page size (default: 100)                   |
| `offset`          | `number`                                       | Pagination offset                          |
| `sort_by`         | `'created_at' \| 'confidence' \| 'expires_at'` | Sort field                                 |
| `sort_order`      | `'asc' \| 'desc'`                              | Sort direction                             |

#### `getActiveProposals(): Promise<CandidateProposal[]>`

Returns all proposals with `status = 'active'` that have not yet expired.

#### `updateProposal(id: string, input: UpdateProposalInput): Promise<CandidateProposal | null>`

Updates `summary`, `rationale`, `confidence`, or `metadata` on a proposal.
Returns `null` if the proposal does not exist.

#### `promoteProposal(id: string, memoryId: string, resolvedBy: string): Promise<void>`

Marks a proposal as `promoted` and records the Edda memory ID it was promoted
to. This does not create the Edda memory — that is a separate step.

#### `dismissProposal(id: string, reason: string, resolvedBy: string): Promise<void>`

Marks a proposal as `dismissed` with a reason. Dismissed proposals are excluded
from active queries.

#### `runDecayCycle(): Promise<{ expired: number; pruned: number }>`

Expires all proposals past their TTL, then prunes resolved proposals older than
90 days. Returns counts of affected rows.

#### `getStats(): Promise<EmberStats>`

Returns aggregated statistics: total proposals, counts by status and type,
number expiring within 24 hours, average confidence, oldest active timestamp,
most recent proposal timestamp, and promotion rate.

#### `isAvailable(): Promise<boolean>`

Returns `true` if the underlying store is reachable. Use to check readiness
before starting a session.

## Built-in Rules

All five rules are included by default via `createDefaultRules()` from
`rules/index.ts`.

| Rule          | Weight | Detects                                               | Suggested Type |
| ------------- | -----: | ----------------------------------------------------- | -------------- |
| `repetition`  |    1.2 | Pattern recurs at or above `repetition_threshold`     | `pattern`      |
| `escalation`  |    1.1 | Severity increases within `escalation_window_hours`   | `warning`      |
| `resolution`  |    1.0 | Failure signal followed by success signal in group    | `lesson`       |
| `convergence` |    0.9 | Two or more sessions share the same observation group | `decision`     |
| `surprise`    |    1.0 | Anomalous observation kinds or bursty/sparse timing   | `anomaly`      |

Rules with higher weight have greater influence on the final confidence score.
`repetition` has the highest weight because frequency is the most reliable
heuristic signal available without AI.

## Examples

### Creating a ProposalStore and using CandidateService

```typescript
import { ProposalStore } from './proposal-store.js';
import { CandidateService } from './candidate-service.js';

// File-based store (persists across restarts)
const store = new ProposalStore('.anvil/ember.db');

// In-memory store (useful in tests)
const testStore = ProposalStore.createInMemory();

const service = new CandidateService({
  store,
  config: {
    evaluation: {
      min_confidence: 0.4, // Raise threshold to reduce noise
      repetition_threshold: 5, // Require more repetitions
      escalation_window_hours: 12,
    },
    decay: {
      default_ttl_days: 14, // Shorter window for faster review cycles
      min_ttl_days: 7,
      max_ttl_days: 30,
    },
    limits: {
      max_candidates: 500,
    },
  },
});

// Create a proposal manually
const proposal = await service.createProposal({
  type: 'warning',
  summary: 'Auth service returning 503 repeatedly',
  rationale: 'Four auth failures recorded in one session',
  confidence: 0.75,
  ttl_days: 7,
  provenance: {
    observation_ids: ['obs_abc', 'obs_def'],
    session_ids: ['ses_xyz'],
    earliest_observation: '2026-03-01T10:00:00Z',
    latest_observation: '2026-03-01T10:04:00Z',
  },
});

// Query active warnings
const result = await service.queryProposals({
  types: ['warning'],
  statuses: ['active'],
  min_confidence: 0.5,
  sort_by: 'confidence',
  sort_order: 'desc',
});

// Dismiss a noisy proposal
await service.dismissProposal(
  proposal.id,
  'Known flaky test environment',
  'alice'
);

// Run scheduled maintenance
const { expired, pruned } = await service.runDecayCycle();

// Always close the store when done
store.close();
```

### Processing a session automatically via ObservationHook

```typescript
import { ProposalStore } from './proposal-store.js';
import { CandidateService } from './candidate-service.js';
import { ObservationHook } from './observation-hook.js';
import { EvaluatorService } from './evaluator-service.js';
import { createDefaultRules } from './rules/index.js';

const store = new ProposalStore('.anvil/ember.db');
const evaluator = new EvaluatorService(createDefaultRules());

const service = new CandidateService({
  store,
  kindlingPort, // injected from your Kindling integration
  eventBus, // injected from your event bus
  evaluator: {
    evaluateGroup: async (group) => {
      const outcome = evaluator.evaluate(group, {
        existingProposals: await service.getActiveProposals(),
        config: {
          min_confidence: 0.3,
          repetition_threshold: 3,
          escalation_window_hours: 24,
        },
      });
      if (!outcome.meetsThreshold) return null;
      return {
        should_propose: true,
        confidence: outcome.confidence,
        type: outcome.suggestedType,
        summary: `${group.grouping_type} group — ${group.count} observations`,
        rationale: `Signals: ${outcome.signals.map((s) => s.rule).join(', ')}`,
        ttl_days: 30,
      };
    },
  },
});

const hook = new ObservationHook({ candidateService: service, eventBus });
hook.start(); // Ember now processes new sessions automatically

// Later, on shutdown:
hook.stop();
store.close();
```

### Writing a custom evaluation rule

```typescript
import type {
  EvaluationRule,
  EvaluationResult,
  EvaluationContext,
} from './evaluator-service.js';
import type { ObservationGroup } from './aggregator-service.js';

export class LongSessionRule implements EvaluationRule {
  readonly name = 'long_session';
  readonly description = 'Fires when a session spans more than 4 hours';
  readonly weight = 0.8;

  evaluate(
    group: ObservationGroup,
    _context: EvaluationContext
  ): EvaluationResult {
    const spanMs =
      new Date(group.latest).getTime() - new Date(group.earliest).getTime();
    const spanHours = spanMs / (60 * 60 * 1000);

    if (spanHours < 4) {
      return { fired: false, contribution: 0 };
    }

    return {
      fired: true,
      contribution: Math.min(1, spanHours / 8), // caps at 1.0 after 8 hours
      context: { span_hours: spanHours, suggested_type: 'pattern' },
    };
  }
}

// Register alongside the defaults
import { EvaluatorService } from './evaluator-service.js';
import { createDefaultRules } from './rules/index.js';

const evaluator = new EvaluatorService([
  ...createDefaultRules(),
  new LongSessionRule(),
]);
```

To remove a rule at runtime:

```typescript
evaluator.removeRule('surprise'); // by name
```

To replace a rule (same name, new implementation):

```typescript
evaluator.registerRule(new LongSessionRule()); // replaces existing if name matches
```

## Design Decisions

The following decisions from the APS plan govern this module:

**D-EMBER-001 — Heuristics-first, AI-optional.** Ember must work without any AI
dependency. Rules are deterministic functions. AI evaluation may be added in v2
as an optional layer above the rule engine.

**D-EMBER-002 — Decay by default.** All proposals expire. There is no option to
create a proposal with no TTL. This forces review and prevents unbounded
accumulation.

**D-EMBER-003 — SQLite for storage.** Ember data is ephemeral by design. SQLite
is local, fast, and can be deleted without consequence. There are no migration
guarantees between Ember schema versions.

**D-EMBER-004 — Proposal types are fixed (v1).** Six types cover the majority of
use cases. Extensible type systems add complexity that v1 does not justify.

**D-EMBER-005 — Confidence is heuristic, not truth.** A confidence score of 0.8
does not mean "80% chance this is correct". It means "strong signal relative to
other proposals". Ember proposes; it does not assert.
