# @eddacraft/anvil-kindling-integration

> Mechanical contracts and integration layer for Kindling memory in Anvil v1

This package provides the **read-only, queryable memory contract** between Anvil
and Kindling. It defines what Anvil records (observations), how to retrieve them
(queries), and enforces that user-supplied AI can read but never mutate the
system of record.

No embedded AI. No magic. Just mechanics.

## Governing Rule

> **Kindling is a system of record, not a reasoning engine.** Queries may
> retrieve facts; interpretation is the caller's responsibility.

Anvil enforces this mechanically:

- User-supplied AI may **read**, but may not **mutate, infer, or generalise**
  via Kindling
- All queries are **bounded, explicit, and evidence-preserving**
- No free-text search. No global scans. No cross-project reads.

## Architecture

```
                    +----------------+
                    |    Kindling    |
                    |  (SQLite DB)  |
                    +----------------+
                          ^   |
                    Write |   | Read
                     Only |   | Only
                          |   v
        +-----------------+-------------------+
        |                                     |
   +----v------+                      +-------v--------+
   |   Anvil   |                      |   User AI      |
   |   Emits   |                      |   (BYO-AI)     |
   |   Facts   |                      |   Queries +    |
   +----------+                       |   Interprets   |
                                      +----------------+

Truth flows one way: Anvil -> Kindling -> AI
AI never writes back to Kindling
```

## Package Structure

```
packages/kindling-integration/
  src/
    observation-contract.ts   # Write-only (11 observation kinds)
    query-contract.ts         # Read-only (4 query scopes)
    index.ts                  # Public exports
    kindling-service.ts       # Core service (emit + query)
    config.ts                 # Configuration schema
    query-service.ts          # Read-only query API
    query-limits.ts           # Anti-vacuum-cleaner enforcement
    sensitive-data-validator.ts  # Secret detection
    retention.ts              # Auto-pruning
    status.ts                 # Status utility (decoupled from CLI)
    malicious-ai.test.ts      # Read-only enforcement tests
    emitters/
      session-emitter.ts      # session_start / session_end
      gate-emitter.ts         # gate_evaluated
      action-emitter.ts       # action_executed
      plan-emitter.ts         # plan_created / edited / approved / rejected
      human-input-emitter.ts  # human_input
      constraint-emitter.ts   # constraint_applied
      error-emitter.ts        # error
  benchmarks/
    emission-overhead.bench.ts  # Performance validation
  scripts/
    generate-openapi.ts       # OpenAPI 3.1 spec generator
  CONTRACTS.md                # Contract summary
  README.md                   # This file
  openapi.json                # Generated OpenAPI spec
```

## Contracts Overview

### 1. Observation Contract (Write-Only)

**File:** [`src/observation-contract.ts`](./src/observation-contract.ts)

Defines the 11 observation kinds Anvil must emit to be "Kindling-complete":

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

**All observations are:**

- Immutable (write-once)
- Timestamped (ISO8601)
- Linked (session_id, plan_id, gate_id, action_id)
- Sanitised (no secrets, redacted commands)
- Facts only (no interpretation, no inference)

### 2. Query Contract (Read-Only)

**File:** [`src/query-contract.ts`](./src/query-contract.ts)

Defines 4 bounded query scopes:

| Scope     | Question                              | Required ID    |
| --------- | ------------------------------------- | -------------- |
| `session` | "What happened in this run?"          | `session_id`   |
| `plan`    | "What happened because of this plan?" | `plan_id`      |
| `gate`    | "Why did this gate pass/fail?"        | `gate_eval_id` |
| `action`  | "What exactly did this action do?"    | `action_id`    |

## Service Setup and Configuration

### Configuration

Kindling is opt-in and disabled by default:

```typescript
// In .anvilrc
{
  "kindling": {
    "enabled": true,
    "database_path": ".anvil/kindling.db",
    "retention": {
      "days": 90,
      "auto_prune": false
    },
    "capture": {
      "sessions": true,
      "plans": true,
      "gates": true,
      "actions": true,
      "constraints": true,
      "human_inputs": true,
      "errors": true
    },
    "query_limits": {
      "max_results": 100,
      "max_payload_bytes": 1048576
    }
  }
}
```

### Service Initialisation

```typescript
import { KindlingService } from '@eddacraft/anvil-kindling-integration';

const kindling = new KindlingService(store, config);
```

## Emitter Usage

Emitters are specialised helpers for each observation kind. They construct
properly-typed observations and emit them through the service.

### Session Emitter

```typescript
import { SessionStartObservation } from '@eddacraft/anvil-kindling-integration/observation';

// Emit at command entry
const obs: SessionStartObservation = {
  kind: 'session_start',
  session_id: '550e8400-e29b-41d4-a716-446655440000',
  timestamp: new Date().toISOString(),
  context: {
    working_directory: '/home/user/project',
    anvil_version: '1.0.0',
    command: 'anvil check',
    args: ['--watch'],
    environment: 'development',
  },
};
await kindling.emit(obs);
```

### Gate Emitter

```typescript
import { GateEvaluatedObservation } from '@eddacraft/anvil-kindling-integration/observation';

const obs: GateEvaluatedObservation = {
  kind: 'gate_evaluated',
  session_id: sessionId,
  timestamp: new Date().toISOString(),
  gate_eval_id: 'gate-eval-001',
  gate_id: 'architecture',
  inputs: {
    file_count: 12,
    changed_files: ['src/index.ts', 'src/config.ts'],
  },
  outcome: 'pass',
  rules_evaluated: ['no-circular-deps', 'layer-boundaries'],
  enforcement: 'blocking',
  duration_ms: 250,
};
await kindling.emit(obs);
```

### Validation Before Emission

Always validate observations before emitting:

```typescript
import {
  validateObservation,
  containsSensitiveData,
} from '@eddacraft/anvil-kindling-integration';

const validation = validateObservation(obs);
if (!validation.success) {
  console.error('Invalid observation:', validation.error);
  return;
}

const sensitiveCheck = containsSensitiveData(validation.data);
if (sensitiveCheck.hasSensitiveData) {
  console.error('Sensitive data detected:', sensitiveCheck.issues);
  return;
}

await kindling.emit(validation.data);
```

## Query API Usage

### Session Query

```typescript
import {
  SessionQuery,
  validateQueryRequest,
} from '@eddacraft/anvil-kindling-integration';

const query: SessionQuery = {
  scope: 'session',
  session_id: '550e8400-e29b-41d4-a716-446655440000',
  shape: 'timeline',
  format: 'json',
  max_results: 100,
};

const result = validateQueryRequest(query);
if (result.success) {
  const response = await kindling.query(result.data);
  console.log(`${response.metadata.result_count} observations`);
  console.log(`Truncated: ${response.metadata.truncated}`);
  for (const obs of response.observations) {
    console.log(`  [${obs.timestamp}] ${obs.kind}`);
  }
}
```

### Plan Query (Cross-Session)

```typescript
import { PlanQuery } from '@eddacraft/anvil-kindling-integration';

// This is the ONLY cross-session read allowed
const query: PlanQuery = {
  scope: 'plan',
  plan_id: 'plan-001',
  shape: 'entity',
  include_executions: true,
  include_versions: true,
};

const response = await kindling.query(query);
```

### Gate Query

```typescript
import { GateQuery } from '@eddacraft/anvil-kindling-integration';

const query: GateQuery = {
  scope: 'gate',
  gate_eval_id: 'gate-eval-456',
  shape: 'entity',
};

const response = await kindling.query(query);
// Returns: gate evaluation with rule IDs, inputs (sanitised), outcome
```

### Action Query

```typescript
import { ActionQuery } from '@eddacraft/anvil-kindling-integration';

const query: ActionQuery = {
  scope: 'action',
  action_id: 'action-789',
  shape: 'entity',
  include_approval_chain: true,
};

const response = await kindling.query(query);
// Returns: action details, redacted command, governance links
```

## CLI Command Mapping

All queries have CLI equivalents. The CLI is a **thin wrapper** over the same
query surface.

| CLI Command              | Query Scope | Query Type     |
| ------------------------ | ----------- | -------------- |
| `anvil run show <id>`    | session     | `SessionQuery` |
| `anvil plan trace <id>`  | plan        | `PlanQuery`    |
| `anvil gate show <id>`   | gate        | `GateQuery`    |
| `anvil action show <id>` | action      | `ActionQuery`  |

**Examples:**

```bash
# Session timeline (JSON)
anvil run show 550e8400-e29b-41d4-a716-446655440000 --json

# Plan trace with linked executions
anvil plan trace plan-001 --json

# Gate evaluation details
anvil gate show gate-eval-456 --json

# Action with approval chain
anvil action show action-789 --json
```

## Security Model

### Read-Only Enforcement

Operations that **MUST NOT** exist in the query API:

- `write()` / `update()` / `delete()`
- `annotate()` / `tag()`
- `learn()` / `embed()` / `infer()`

**If user AI wants memory, it must bring its own store.** The malicious AI test
suite (`src/malicious-ai.test.ts`) proves these boundaries hold.

### Sensitive Data Detection

Observations are validated before emission to catch:

- Passwords, tokens, API keys
- AWS credentials
- Private keys
- Email addresses (flagged as potentially sensitive)

```typescript
const check = containsSensitiveData(obs);
if (check.hasSensitiveData) {
  // Reject observation, log issues
  console.error(check.issues);
}
```

### Query Limits (Anti-Vacuum-Cleaner)

Every query enforces:

- `max_results`: Default 100, max 1000
- `max_payload_bytes`: Default 1MB, max 10MB
- Mandatory scoping (scope + explicit ID)
- No free-text search, no global scans

### Output Guarantees

Every Kindling response guarantees:

1. **Stable field names** -- no field names change between queries
2. **Explicit timestamps** -- every observation has ISO8601 timestamp
3. **Explicit links** -- provenance via typed links (caused_by, governed_by,
   approved_by)
4. **No hidden inference** -- payload contains only raw facts
5. **No reordered history** -- observations returned in recorded order

## Status Utility

Check Kindling integration status without coupling to any CLI framework:

```typescript
import {
  getKindlingStatus,
  formatKindlingStatus,
} from '@eddacraft/anvil-kindling-integration';

// Quick check (no store needed)
const status = await getKindlingStatus({ enabled: false });
// => { enabled: false }

// Full status with store
const status = await getKindlingStatus(
  { enabled: true, retention: { days: 90 } },
  myStore
);
console.log(formatKindlingStatus(status));
// Kindling: enabled
// Observations: 42
// Database size: 80 KB
// Retention: 90 days
// Last observation: 2026-02-15T10:30:00.000Z
```

## BYO-AI Integration Example

User brings their own AI (e.g., Claude via API):

```typescript
import {
  QueryRequest,
  QueryResponse,
} from '@eddacraft/anvil-kindling-integration';

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

## OpenAPI Spec Generation

Generate a machine-readable OpenAPI 3.1 spec from the query contracts:

```bash
npx tsx scripts/generate-openapi.ts
# Outputs: openapi.json
```

Use the spec for:

- **Client library generation** (TypeScript, Python, Go, Rust, etc.)
- **API documentation** (Swagger UI, Redoc)
- **Contract testing** (validate implementations against the spec)

```bash
# Generate TypeScript client
npx openapi-generator-cli generate -i openapi.json -g typescript-fetch -o ./clients/ts

# Generate Python client
openapi-generator-cli generate -i openapi.json -g python -o ./clients/python
```

## Performance

Observation emission must add < 50ms overhead (async, non-blocking). The
benchmark suite validates this:

```bash
pnpm bench --filter kindling-integration
```

With a no-op store, emission overhead is typically < 1ms per observation
(validation + sensitive data check).

## Running Tests

```bash
# All tests
pnpm test --filter kindling-integration

# Malicious AI test suite only
pnpm test --filter kindling-integration -- --testNamePattern="malicious-ai"

# Benchmarks
pnpm bench --filter kindling-integration
```

## Explicit Non-Goals (v1)

The following are **OUT OF SCOPE** for v1:

- Semantic search / similarity queries / embeddings
- Cross-plan discovery (except explicit plan_id lookup)
- Learned relevance / pattern detection
- Auto-summaries stored in Kindling
- AI-generated annotations stored in Kindling
- Real-time streaming dashboard
- Team-level memory sharing

**These belong to Edda / Ember, not Kindling v1.**

## Licence

Copyright (c) 2026 eddacraft, Inc. All rights reserved. See
[LICENSE](../../LICENSE) for details.

## See Also

- [CONTRACTS.md](./CONTRACTS.md) -- One-page contract summary
- [Kindling Integration Plan](../../plans/modules/kindling-integration.aps.md)
  -- APS module specification
- [Kindling Repository](https://github.com/eddacraft/kindling) -- Core Kindling
  implementation
