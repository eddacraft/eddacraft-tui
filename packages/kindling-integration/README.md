# @anvil/kindling-integration

> Mechanical contracts for Kindling memory integration in Anvil v1

This package defines the **read-only, queryable memory contract** between Anvil
and Kindling. No embedded AI. No magic. Just mechanics.

## Governing Rule

> **Kindling is a system of record, not a reasoning engine.** Queries may
> retrieve facts; interpretation is the caller's responsibility.

Anvil enforces this mechanically:

- User-supplied AI may **read**, but may not **mutate, infer, or generalise**
  via Kindling
- All queries are **bounded, explicit, and evidence-preserving**
- No free-text search. No global scans. No cross-project reads.

## Two Contracts

### 1. Observation Contract (Write-Only)

**File:** [`src/observation-contract.ts`](./src/observation-contract.ts)

Defines what Anvil must emit to be "Kindling-complete". **11 observation
kinds:**

| Kind                 | Purpose                   | When Emitted                    |
| -------------------- | ------------------------- | ------------------------------- |
| `session_start`      | Session recording spine   | Every Anvil run starts          |
| `session_end`        | Session outcome + summary | Every Anvil run completes       |
| `plan_created`       | Plan lifecycle tracking   | New plan authored               |
| `plan_edited`        | Plan version history      | Plan modified                   |
| `plan_approved`      | Human approval            | User approves plan              |
| `plan_rejected`      | Human rejection           | User rejects plan               |
| `action_executed`    | Action provenance         | Command/tool/file operation     |
| `gate_evaluated`     | Gate check result         | Every gate evaluation           |
| `constraint_applied` | Decision constraint       | Action prevented by rule/policy |
| `human_input`        | Human decision            | Approval/override/rejection     |
| `error`              | Failure history           | Command/tool/execution error    |

**Key properties:**

- Immutable (write-once)
- Timestamped (ISO8601)
- Linked (session_id, plan_id, gate_id, action_id)
- Sanitised (no secrets, redacted commands)
- Facts only (no interpretation, no inference)

**Example:**

```typescript
import { SessionStartObservation } from '@anvil/kindling-integration/observation';

const obs: SessionStartObservation = {
  kind: 'session_start',
  session_id: '123e4567-e89b-12d3-a456-426614174000',
  timestamp: '2025-01-11T10:00:00.000Z',
  context: {
    working_directory: '/home/user/project',
    anvil_version: '1.0.0',
    command: 'anvil check',
    args: ['--watch'],
    environment: 'development',
  },
};
```

### 2. Query Contract (Read-Only)

**File:** [`src/query-contract.ts`](./src/query-contract.ts)

Defines how to retrieve observations. **4 query scopes:**

#### A. Session Scope

> "What happened in this run?"

```typescript
import { SessionQuery } from '@anvil/kindling-integration/query';

const query: SessionQuery = {
  scope: 'session',
  session_id: '123e4567-e89b-12d3-a456-426614174000',
  shape: 'timeline',
  format: 'json',
};
```

**CLI equivalent:**

```bash
anvil run show <run_id> --json
```

#### B. Plan Scope

> "What happened because of this plan?"

```typescript
const query: PlanQuery = {
  scope: 'plan',
  plan_id: 'plan-001',
  shape: 'entity',
  include_executions: true,
};
```

**CLI equivalent:**

```bash
anvil plan trace <plan_id> --json
```

**Note:** This is the **only cross-session read** allowed, via explicit
`plan_id`.

#### C. Gate Scope

> "Why did this gate pass/fail?"

```typescript
const query: GateQuery = {
  scope: 'gate',
  gate_eval_id: 'gate-eval-456',
  shape: 'entity',
};
```

**CLI equivalent:**

```bash
anvil gate show <gate_eval_id> --json
```

#### D. Action Scope

> "What exactly did this action do?"

```typescript
const query: ActionQuery = {
  scope: 'action',
  action_id: 'action-789',
  shape: 'entity',
  include_approval_chain: true,
};
```

**CLI equivalent:**

```bash
anvil action show <action_id> --json
```

### Query Characteristics (Mandatory)

Every query must specify:

- **scope**: `session | plan | gate | action`
- **identifier**: Concrete ID(s) for the scope
- **time bounds**: Implicit via session, explicit otherwise
- **result shape**: `timeline | list | entity`
- **format**: `json | text`

### Query Limits (Anti-Vacuum-Cleaner)

To prevent "AI vacuum cleaners":

- `max_results`: Default 100, max 1000
- `max_payload_bytes`: Default 1MB, max 10MB
- Mandatory scoping (no global queries)
- Optional rate limits per user/session

## Output Guarantees (LLM-Safe)

Every Kindling response guarantees:

1. **Stable field names** — No field names change between queries
2. **Explicit timestamps** — Every observation has ISO8601 timestamp
3. **Explicit links** — Provenance via typed links (`caused_by`, `governed_by`,
   `approved_by`)
4. **No hidden inference** — Payload contains only raw facts
5. **No reordered history** — Observations returned in recorded order

**This makes Kindling LLM-safe by construction.**

AI can:

- ✅ Narrate events
- ✅ Summarise outcomes
- ✅ Explain facts

But AI will always be explaining **facts, not ghosts**.

## Read-Only Enforcement

Operations that **MUST NOT** exist in the query API:

❌ `write()` ❌ `update()` ❌ `delete()` ❌ `annotate()` ❌ `tag()` ❌ `learn()`
❌ `embed()` ❌ `infer()`

**If user AI wants memory, it must bring its own store.**

## Explicit Non-Goals (v1)

The following are **OUT OF SCOPE** for v1:

❌ Semantic search ❌ Similarity queries ❌ Embeddings ❌ Cross-plan discovery
❌ Learned relevance ❌ Auto-summaries (stored in Kindling) ❌ AI-generated
annotations stored in Kindling

**These belong to Edda / Ember, not Kindling v1.**

## CLI Symmetry (Human-First, AI-Compatible)

All queries have CLI equivalents. The CLI is a **thin wrapper** over the same
query surface. That symmetry is intentional.

| CLI Command              | Query Scope | Returns                           |
| ------------------------ | ----------- | --------------------------------- |
| `anvil run show <id>`    | session     | Timeline of session observations  |
| `anvil plan trace <id>`  | plan        | Plan metadata + linked executions |
| `anvil gate show <id>`   | gate        | Gate evaluation details           |
| `anvil action show <id>` | action      | Action execution details          |

## Integration Points (Where to Emit)

| Observation Kind     | Integration Point                                          |
| -------------------- | ---------------------------------------------------------- |
| `session_start/end`  | `cli/src/commands/*.ts` (every command entry/exit)         |
| `gate_evaluated`     | `core/src/gate/gate-runner.ts` (GateRunner.run completion) |
| `action_executed`    | Anywhere Anvil executes commands (via child_process)       |
| `plan_*`             | `core/src/aps/` (plan parsing, validation, execution)      |
| `human_input`        | `cli/src/tui/` (TUI confirmation prompts)                  |
| `constraint_applied` | `core/src/gate/` (when gate blocks action)                 |
| `error`              | All try/catch blocks that handle failures                  |

## Mental Model (Repeat to Team)

```
Kindling records
Anvil orchestrates
Users (or their AI) interpret

Truth never moves
```

That separation is your long-term moat.

## Example: BYO-AI Integration

User brings their own AI (e.g., Claude via API):

```typescript
import { QueryRequest, QueryResponse } from '@anvil/kindling-integration';

// AI wants to explain why gate failed
async function explainGateFailure(gateEvalId: string): Promise<string> {
  // 1. Query Kindling (read-only, bounded)
  const query: QueryRequest = {
    scope: 'gate',
    gate_eval_id: gateEvalId,
    shape: 'entity',
    format: 'json',
  };

  const response: QueryResponse = await kindling.query(query);

  // 2. AI interprets facts (using external AI service)
  const prompt = `
    Explain why this gate failed based on these facts:
    ${JSON.stringify(response.observations, null, 2)}
  `;

  const explanation = await callExternalAI(prompt);

  // 3. AI stores interpretation in its own memory (NOT in Kindling)
  await userAI.memory.store({
    type: 'gate_failure_explanation',
    gate_eval_id: gateEvalId,
    explanation,
    generated_at: new Date().toISOString(),
  });

  return explanation;
}
```

**Key points:**

- AI reads from Kindling (bounded query)
- AI interprets facts (external reasoning)
- AI stores conclusions in **its own memory** (not Kindling)
- Kindling remains immutable system of record

## Usage

```typescript
// Emit observations (write-only)
import {
  validateObservation,
  SessionStartObservation,
} from '@anvil/kindling-integration/observation';

const obs: SessionStartObservation = {
  /* ... */
};
const result = validateObservation(obs);
if (result.success) {
  await kindling.emit(result.data);
}

// Query observations (read-only)
import {
  validateQueryRequest,
  SessionQuery,
} from '@anvil/kindling-integration/query';

const query: SessionQuery = {
  /* ... */
};
const result = validateQueryRequest(query);
if (result.success) {
  const response = await kindling.query(result.data);
  console.log(response.observations);
}
```

## Licence

Copyright (c) 2026 EddaCraft. All rights reserved. See [LICENSE](../../LICENSE)
for details.

## See Also

- [Kindling Integration Plan](../../plans/modules/kindling-integration.aps.md) —
  APS module specification
- [Kindling Repository](https://github.com/EddaCraft/kindling) — Core Kindling
  implementation
