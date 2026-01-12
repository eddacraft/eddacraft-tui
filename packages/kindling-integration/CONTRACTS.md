# Kindling Integration Contracts — Summary

This document provides a one-page summary of the Kindling v1 contracts.

## Mental Model

```
┌─────────────────────────────────────────────────────────────┐
│                   Anvil v1 × Kindling                        │
│                  (Read-Only, Queryable)                      │
└─────────────────────────────────────────────────────────────┘

                    ┌──────────────┐
                    │   Kindling   │
                    │  (SQLite DB) │
                    └──────────────┘
                          ▲  │
                    Write │  │ Read
                     Only │  │ Only
                          │  ▼
        ┌─────────────────┴──────────────────┐
        │                                     │
   ┌────▼─────┐                      ┌───────▼──────┐
   │  Anvil   │                      │  User AI     │
   │  Emits   │                      │  (BYO-AI)    │
   │  Facts   │                      │  Queries +   │
   └──────────┘                      │  Interprets  │
                                     └──────────────┘

Truth flows one way: Anvil → Kindling → AI
AI never writes back to Kindling
```

## Two Surfaces

### 1. Observation Contract (Write-Only)

**File:** `src/observation-contract.ts`

**What:** 11 observation kinds Anvil must emit

| #   | Kind                 | Purpose                     |
| --- | -------------------- | --------------------------- |
| 1   | `session_start`      | Session begins (spine)      |
| 2   | `session_end`        | Session completes (outcome) |
| 3   | `plan_created`       | Plan authored               |
| 4   | `plan_edited`        | Plan modified               |
| 5   | `plan_approved`      | Human approves              |
| 6   | `plan_rejected`      | Human rejects               |
| 7   | `action_executed`    | Command/file operation      |
| 8   | `gate_evaluated`     | Gate check result           |
| 9   | `constraint_applied` | Action prevented            |
| 10  | `human_input`        | User decision               |
| 11  | `error`              | Failure recorded            |

**Properties:**

- Immutable (write-once)
- Timestamped (ISO8601)
- Linked (session_id, plan_id, etc.)
- Sanitised (no secrets)
- Facts only (no inference)

### 2. Query Contract (Read-Only)

**File:** `src/query-contract.ts`

**What:** 4 query scopes for bounded reads

| Scope     | Question                              | Returns                  |
| --------- | ------------------------------------- | ------------------------ |
| `session` | "What happened in this run?"          | Timeline of observations |
| `plan`    | "What happened because of this plan?" | Plan + linked executions |
| `gate`    | "Why did this gate pass/fail?"        | Gate evaluation details  |
| `action`  | "What exactly did this action do?"    | Action execution details |

**Mandatory constraints:**

- scope + identifier (no free-text search)
- max_results (default 100, max 1000)
- max_payload_bytes (default 1MB, max 10MB)

**Output guarantees:**

1. Stable field names
2. Explicit timestamps
3. Explicit links (`caused_by`, `governed_by`, `approved_by`)
4. No hidden inference
5. No reordered history

## Read-Only Enforcement

**Operations that MUST NOT exist:**

❌ write, update, delete, annotate, tag, learn, embed, infer

**If AI wants memory, it brings its own store.**

## CLI Symmetry

```bash
anvil run show <id>       →  SessionQuery
anvil plan trace <id>     →  PlanQuery
anvil gate show <id>      →  GateQuery
anvil action show <id>    →  ActionQuery
```

CLI is a thin wrapper over the same query API.

## Explicit Non-Goals (v1)

❌ Semantic search ❌ Similarity queries ❌ Embeddings ❌ Cross-plan discovery
❌ Learned relevance ❌ Auto-summaries stored in Kindling ❌ AI annotations
stored in Kindling

**These belong to Edda / Ember, not Kindling v1.**

## Integration Checklist

To make Anvil "Kindling-complete":

- [ ] Emit `session_start` at command entry
- [ ] Emit `session_end` at command exit
- [ ] Emit `gate_evaluated` after every gate check
- [ ] Emit `action_executed` for every observable action
- [ ] Emit `error` for every failure
- [ ] Emit `human_input` for every approval/override
- [ ] Emit `constraint_applied` when actions prevented
- [ ] Emit `plan_*` for all plan lifecycle events
- [ ] Implement bounded query API (4 scopes)
- [ ] Implement CLI commands (`run show`, `plan trace`, etc.)
- [ ] Validate no write/update/delete operations exposed
- [ ] Test query limits (max_results, max_payload_bytes)

## Next Steps (Recommended)

### Option A: Malicious AI Test Suite

Create tests that prove read-only enforcement:

```typescript
// These should FAIL
await kindling.annotate(obs); // No such method
await kindling.embed(obs); // No such method
await kindling.query({ scope: 'global' }); // Invalid scope
await kindling.query({ free_text: 'violations' }); // No free-text
```

### Option B: OpenAPI / TypeSpec Schema

Generate machine-readable API spec from Zod schemas:

```bash
pnpm add -D zod-to-openapi
# Generate OpenAPI 3.1 spec from query-contract.ts
```

### Option C: Implementation Plan

Use these contracts as the single source of truth when implementing:

1. Create `KindlingService` wrapper around Kindling core
2. Hook into GateRunner, CLI commands, etc. to emit observations
3. Implement query API with read-only enforcement
4. Add CLI commands as thin wrappers
5. Write malicious AI tests to prove boundaries

## Files in This Package

```
packages/kindling-integration/
├── src/
│   ├── observation-contract.ts  # Write-only (11 observation kinds)
│   ├── query-contract.ts        # Read-only (4 query scopes)
│   └── index.ts                 # Public exports
├── CONTRACTS.md                 # This file (summary)
├── README.md                    # Usage guide
├── package.json                 # Package config
└── tsconfig.json                # TypeScript config
```

## Governing Rule (Repeat to Team)

> **Kindling is a system of record, not a reasoning engine.** Queries may
> retrieve facts; interpretation is the caller's responsibility.

```
Kindling records
Anvil orchestrates
Users (or their AI) interpret

Truth never moves
```

That separation is your long-term moat.
