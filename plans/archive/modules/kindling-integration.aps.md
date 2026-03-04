<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Kindling Memory Integration (v1)

| Scope    | Owner | Priority | Status   |
| -------- | ----- | -------- | -------- |
| KINDLING | —     | medium   | Complete |

## Purpose

Make Anvil a credible system of record by integrating Kindling's local-first memory for write-only observation capture and read-only bounded queries. No embedded AI. No magic. Just mechanical facts.

**Problem:** Without persistent memory:

- "What happened?" requires log archaeology or memory recall
- Plan → execution → outcome linkage is lost between sessions
- Gate failures are transient — no audit trail for governance
- Human approvals and overrides vanish after the fact
- AI tools operate without factual grounding (hallucination risk)

**Solution:** Kindling integration provides:

- **System of record**: Every Anvil run writes immutable observations (11 kinds)
- **Bounded queries**: Retrieve facts by session/plan/gate/action (4 scopes)
- **Read-only enforcement**: User AI can read, but never mutate/annotate/infer
- **Accountability**: Human inputs recorded as first-class events
- **Provenance**: Explicit links (caused_by, governed_by, approved_by)

**Governing Rule:**
> Kindling is a system of record, not a reasoning engine.
> Queries may retrieve facts; interpretation is the caller's responsibility.

## In Scope

**Observation Emission (Write-Only):**
- Session recording (session_start, session_end) — every run
- PlanSpec lifecycle (plan_created, plan_edited, plan_approved, plan_rejected)
- Action provenance (action_executed) — commands, tools, file operations
- Gate evaluation (gate_evaluated) — outcomes with rule IDs
- Decision constraints (constraint_applied) — prevented actions
- Human inputs (human_input) — approvals, overrides, rejections
- Error history (error) — failures as data, not noise

**Query API (Read-Only):**
- Session scope: "What happened in this run?"
- Plan scope: "What happened because of this plan?" (only cross-session read)
- Gate scope: "Why did this gate pass/fail?"
- Action scope: "What exactly did this action do?"
- Mandatory constraints: scope + ID (no free-text search)
- Anti-vacuum-cleaner limits (max_results, max_payload_bytes)

**CLI Commands:**
- `anvil run show <run_id> [--json]` — Session timeline
- `anvil plan trace <plan_id> [--json]` — Plan + linked executions
- `anvil gate show <gate_eval_id> [--json]` — Gate evaluation details
- `anvil action show <action_id> [--json]` — Action execution details

**Configuration:**
- Opt-in enablement (disabled by default for privacy)
- Database location (.anvil/kindling.db, git-ignored)
- Retention policy (auto-pruning old observations)
- Observation kind filters (e.g., capture gates but not file changes)

## Out of Scope (v1)

- ❌ Semantic search / FTS free-text queries
- ❌ Similarity queries / embeddings
- ❌ Cross-plan discovery (except explicit plan_id lookup)
- ❌ Pattern detection / trend analysis
- ❌ Auto-summaries stored in Kindling
- ❌ AI-generated annotations written to Kindling
- ❌ Write/update/delete/annotate/tag/learn APIs
- ❌ Real-time streaming dashboard
- ❌ Team-level memory sharing
- ❌ Memory synchronisation across machines

**These belong to Edda/Ember or v2+, not Kindling v1.**

## Interfaces

**Depends on:**

- `save-time-trust` — GateRunner, check execution infrastructure
- `drift-reporting` — Snapshot data structures (for consistency)
- Kindling packages: `@kindling/core`, `@kindling/store-sqlite`, `@kindling/provider-local`
- Contracts: `@eddacraft/anvil-kindling-integration` (observation + query schemas)

**Exposes:**

- `KindlingService` — High-level API for emit + query
- `ObservationEmitter` — Hook for gate runner, CLI commands, plan lifecycle
- `QueryService` — Read-only bounded query API
- CLI commands: `anvil run show`, `anvil plan trace`, `anvil gate show`, `anvil action show`
- Configuration schema in `.anvilrc` (kindling section)

**Configuration Example:**

```json
{
  "kindling": {
    "enabled": true,
    "database": ".anvil/kindling.db",
    "retention": {
      "days": 90,
      "max_observations": 10000
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

## Boundary Rules

- KINDLING must not modify gate check behaviour or results
- Observation capture must be asynchronous and non-blocking (< 50ms overhead)
- Memory operations must fail gracefully if disabled or unavailable
- Privacy-sensitive data (secrets, credentials) must never be captured
- Database location must be workspace-local (no global state)
- **Read-only enforcement**: No write/update/delete operations in query API
- **Bounded queries only**: Must specify scope + ID (no global scans)

## Acceptance Criteria

- [ ] All 11 observation kinds emitted when memory enabled
- [ ] Session capsule created/closed for every Anvil run
- [ ] `anvil run show <id>` returns timeline of session observations
- [ ] `anvil plan trace <id>` returns plan metadata + linked runs
- [ ] `anvil gate show <id>` returns gate evaluation with rule IDs
- [ ] `anvil action show <id>` returns action details with provenance
- [ ] < 50ms observation emission overhead (async, non-blocking)
- [ ] Memory disabled by default (explicit opt-in required)
- [ ] Query API rejects invalid scopes (no free-text, no global queries)
- [ ] Query API respects limits (max_results, max_payload_bytes)
- [ ] Observations include provenance links (session_id, plan_id, etc.)
- [ ] Sensitive data validation prevents secrets in observations
- [ ] Memory database stored in `.anvil/` (git-ignored)
- [ ] No write/update/delete operations exposed in query API

## Risks & Mitigations

| Risk                                    | Mitigation                                          |
| --------------------------------------- | --------------------------------------------------- |
| Observation capture adds latency       | Async emission, batching, SQLite WAL mode           |
| Database grows unbounded               | Configurable retention policy, auto-pruning         |
| Sensitive data captured accidentally   | Validation on emit, explicit capture filters        |
| Kindling version conflicts             | Pin Kindling versions, vendor if needed             |
| Query API bypassed for "convenience"   | No alternate read paths, enforce in KindlingService |
| AI writes annotations to Kindling      | No write APIs exposed, read-only enforcement tests  |
| Users forget memory is enabled         | Show stats in `anvil status`, capture count         |

## Tasks

### Phase A: Foundation & Contracts

#### KINDLING-001: Kindling service wrapper

- **Intent:** Create KindlingService that wraps Kindling core APIs with read-only enforcement
- **Expected Outcome:** Service initialises Kindling store, enforces write-only emit / read-only query contract
- **Scope:** `packages/kindling-integration/src/`
- **Non-scope:** Observation emission hooks, CLI commands
- **Files:**
  - `packages/kindling-integration/src/kindling-service.ts`
  - `packages/kindling-integration/src/kindling-service.test.ts`
- **Dependencies:** —
- **Validation:** `nx test kindling-integration --testNamePattern="KindlingService"`
- **Confidence:** high
- **Status:** Complete (2026-02-15)

#### KINDLING-002: Configuration schema and loading

- **Intent:** Define kindling config schema with sensible defaults, integrate with .anvilrc
- **Expected Outcome:** Kindling config loaded from .anvilrc with Zod validation
- **Scope:** `core/src/gate/gate-config.ts`, `packages/kindling-integration/src/`
- **Non-scope:** TUI config editor
- **Files:**
  - `packages/kindling-integration/src/config.ts`
  - `core/src/gate/gate-config.ts` (extend schema)
  - `packages/kindling-integration/src/config.test.ts`
- **Dependencies:** KINDLING-001
- **Validation:** `nx test kindling-integration --testNamePattern="config"`
- **Confidence:** high
- **Status:** Complete (2026-02-15)

### Phase B: Observation Emission (Write-Only)

#### KINDLING-003: Session observation hooks

- **Intent:** Emit session_start at command entry, session_end at command exit
- **Expected Outcome:** Every CLI command creates session capsule with start/end observations
- **Scope:** `cli/src/commands/`, `packages/kindling-integration/src/`
- **Non-scope:** Gate/plan/action observations
- **Files:**
  - `packages/kindling-integration/src/emitters/session-emitter.ts`
  - `cli/src/commands/check.ts` (add session hooks)
  - `cli/src/commands/watch.ts` (add session hooks)
  - `cli/src/commands/gate.ts` (add session hooks)
- **Dependencies:** KINDLING-001, KINDLING-002
- **Validation:** `anvil check && verify session observation in .anvil/kindling.db`
- **Confidence:** high
- **Status:** Complete (2026-02-15)

#### KINDLING-004: Gate evaluation observations

- **Intent:** Emit gate_evaluated after every gate check with rule IDs and outcome
- **Expected Outcome:** GateRunner emits observations with sanitised inputs, rules, enforcement
- **Scope:** `core/src/gate/`, `packages/kindling-integration/src/`
- **Non-scope:** Individual check instrumentation (just GateRunner aggregate)
- **Files:**
  - `packages/kindling-integration/src/emitters/gate-emitter.ts`
  - `core/src/gate/gate-runner.ts` (add observation hook)
  - `packages/kindling-integration/src/emitters/gate-emitter.test.ts`
- **Dependencies:** KINDLING-003
- **Validation:** `nx test kindling-integration --testNamePattern="gate-emitter" && anvil check --verify-observations`
- **Confidence:** high
- **Status:** Complete (2026-02-15)

#### KINDLING-005: Action execution observations

- **Intent:** Emit action_executed for observable actions (commands, file writes, diff apply)
- **Expected Outcome:** Actions recorded with redacted details, provenance, outcomes
- **Scope:** Anywhere Anvil executes observable actions
- **Non-scope:** Passive reads (use Read tool, not action_executed)
- **Files:**
  - `packages/kindling-integration/src/emitters/action-emitter.ts`
  - `core/src/gate/gate-runner.ts` (if gates execute actions)
  - Integration points TBD (depends on action execution patterns)
- **Dependencies:** KINDLING-003
- **Validation:** `nx test kindling-integration --testNamePattern="action-emitter"`
- **Confidence:** medium
- **Status:** Complete (2026-02-15)

#### KINDLING-006: Plan lifecycle observations

- **Intent:** Emit plan_created/edited/approved/rejected for all plan events
- **Expected Outcome:** Plan history recorded with versions, hashes, human decisions
- **Scope:** `core/src/aps/`, `cli/src/commands/plan.ts` (if exists)
- **Non-scope:** Plan execution (that's actions + gates)
- **Files:**
  - `packages/kindling-integration/src/emitters/plan-emitter.ts`
  - `core/src/aps/` (add hooks for plan lifecycle)
  - `packages/kindling-integration/src/emitters/plan-emitter.test.ts`
- **Dependencies:** KINDLING-003
- **Validation:** `nx test kindling-integration --testNamePattern="plan-emitter"`
- **Confidence:** medium
- **Status:** Complete (2026-02-15)

#### KINDLING-007: Human input and constraint observations

- **Intent:** Emit human_input (approvals/overrides) and constraint_applied (prevented actions)
- **Expected Outcome:** Human decisions and governance constraints recorded as first-class events
- **Scope:** `cli/src/tui/`, gate enforcement points
- **Non-scope:** Automated decisions (not "human input")
- **Files:**
  - `packages/kindling-integration/src/emitters/human-input-emitter.ts`
  - `packages/kindling-integration/src/emitters/constraint-emitter.ts`
  - TUI confirmation hooks (TBD based on TUI structure)
- **Dependencies:** KINDLING-003
- **Validation:** `nx test kindling-integration --testNamePattern="human|constraint"`
- **Confidence:** medium
- **Status:** Complete (2026-02-15)

#### KINDLING-008: Error observations

- **Intent:** Emit error for every failure (command failures, tool errors, aborted executions)
- **Expected Outcome:** Failures recorded as data with context, recoverability, partial state
- **Scope:** All try/catch blocks, process error handlers
- **Non-scope:** Expected errors (validation failures that are intentional)
- **Files:**
  - `packages/kindling-integration/src/emitters/error-emitter.ts`
  - Global error handlers in CLI
  - `packages/kindling-integration/src/emitters/error-emitter.test.ts`
- **Dependencies:** KINDLING-003
- **Validation:** `nx test kindling-integration --testNamePattern="error-emitter"`
- **Confidence:** high
- **Status:** Complete (2026-02-15)

### Phase C: Query API (Read-Only)

#### KINDLING-009: Query service with scope enforcement

- **Intent:** Implement bounded query API with 4 scopes (session, plan, gate, action)
- **Expected Outcome:** Query service enforces scope + ID, rejects invalid queries
- **Scope:** `packages/kindling-integration/src/`
- **Non-scope:** CLI implementation (just API)
- **Files:**
  - `packages/kindling-integration/src/query-service.ts`
  - `packages/kindling-integration/src/query-service.test.ts`
- **Dependencies:** KINDLING-001
- **Validation:** `nx test kindling-integration --testNamePattern="QueryService"`
- **Confidence:** high
- **Status:** Complete (2026-02-15)

#### KINDLING-010: Query limits and throttling

- **Intent:** Enforce max_results, max_payload_bytes, reject global/free-text queries
- **Expected Outcome:** Query API prevents "AI vacuum cleaner" patterns
- **Scope:** `packages/kindling-integration/src/`
- **Non-scope:** Rate limiting (can be added later)
- **Files:**
  - `packages/kindling-integration/src/query-limits.ts`
  - Update `query-service.ts` with limit enforcement
  - `packages/kindling-integration/src/query-limits.test.ts`
- **Dependencies:** KINDLING-009
- **Validation:** `nx test kindling-integration --testNamePattern="query-limits"`
- **Confidence:** high
- **Status:** Complete (2026-02-15)

#### KINDLING-011: Malicious AI test suite

- **Intent:** Prove read-only enforcement with tests for invalid operations
- **Expected Outcome:** Tests that write/annotate/embed/global-query operations fail
- **Scope:** `packages/kindling-integration/src/`
- **Non-scope:** Actual implementation (just validation tests)
- **Files:**
  - `packages/kindling-integration/src/malicious-ai.test.ts`
- **Dependencies:** KINDLING-009, KINDLING-010
- **Validation:** `nx test kindling-integration --testNamePattern="malicious-ai"`
- **Confidence:** high
- **Status:** Complete (2026-02-15)

### Phase D: CLI Commands

#### KINDLING-012: Session query command (run show)

- **Intent:** Add `anvil run show <run_id>` command for session timeline
- **Expected Outcome:** CLI displays session observations in timeline format
- **Scope:** `cli/src/commands/`
- **Non-scope:** TUI visualisation (just CLI table/JSON output)
- **Files:**
  - `cli/src/commands/run.ts`
  - `cli/src/commands/run.test.ts`
- **Dependencies:** KINDLING-009
- **Validation:** `anvil run show <id> --json | jq '.observations | length'`
- **Confidence:** high
- **Status:** Complete (2026-02-15)

#### KINDLING-013: Plan, gate, action query commands

- **Intent:** Add `anvil plan trace`, `anvil gate show`, `anvil action show` commands
- **Expected Outcome:** CLI commands for all 4 query scopes with --json support
- **Scope:** `cli/src/commands/`
- **Non-scope:** Complex visualisation
- **Files:**
  - `cli/src/commands/plan.ts` (add trace subcommand)
  - `cli/src/commands/gate.ts` (add show subcommand)
  - `cli/src/commands/action.ts` (new file)
- **Dependencies:** KINDLING-009, KINDLING-012
- **Validation:** `anvil plan trace <id> && anvil gate show <id> && anvil action show <id>`
- **Confidence:** high
- **Status:** Complete (2026-02-15)

### Phase E: Integration & Polish

#### KINDLING-014: Status integration

- **Intent:** Show Kindling stats in `anvil status` output
- **Expected Outcome:** Status displays enabled/disabled, observation count, DB size
- **Scope:** `cli/src/commands/status.ts`
- **Non-scope:** Detailed memory browser
- **Files:**
  - `cli/src/commands/status.ts` (add kindling section)
- **Dependencies:** KINDLING-001
- **Validation:** `anvil status | grep -A5 "Kindling"`
- **Confidence:** high
- **Status:** Complete (2026-02-15)

#### KINDLING-015: Sensitive data validation

- **Intent:** Validate observations don't contain secrets before emission
- **Expected Outcome:** Sensitive data detector catches passwords/tokens/keys in observations
- **Scope:** `packages/kindling-integration/src/`
- **Non-scope:** Perfect detection (heuristic-based)
- **Files:**
  - `packages/kindling-integration/src/sensitive-data-validator.ts`
  - Update emitters to validate before emit
  - `packages/kindling-integration/src/sensitive-data-validator.test.ts`
- **Dependencies:** KINDLING-003 through KINDLING-008
- **Validation:** `nx test kindling-integration --testNamePattern="sensitive-data"`
- **Confidence:** high
- **Status:** Complete (2026-02-15)

#### KINDLING-016: Retention and pruning

- **Intent:** Implement auto-pruning of old observations based on retention policy
- **Expected Outcome:** Background task prunes observations older than config.retention.days
- **Scope:** `packages/kindling-integration/src/`
- **Non-scope:** Manual pruning CLI (can be added later)
- **Files:**
  - `packages/kindling-integration/src/retention.ts`
  - `packages/kindling-integration/src/retention.test.ts`
- **Dependencies:** KINDLING-001, KINDLING-002
- **Validation:** `nx test kindling-integration --testNamePattern="retention"`
- **Confidence:** medium
- **Status:** Complete (2026-02-15)

#### KINDLING-017: Performance benchmarking

- **Intent:** Validate observation emission overhead meets < 50ms acceptance criteria
- **Expected Outcome:** Benchmark suite showing async emission is non-blocking
- **Scope:** `packages/kindling-integration/`
- **Non-scope:** Optimisation (only measurement)
- **Files:**
  - `packages/kindling-integration/benchmarks/emission-overhead.bench.ts`
- **Dependencies:** KINDLING-003 through KINDLING-008
- **Validation:** `pnpm bench --filter kindling-integration`
- **Confidence:** medium
- **Status:** Complete (2026-02-15)

#### KINDLING-018: Documentation and examples

- **Intent:** Document Kindling integration, CLI usage, BYO-AI patterns
- **Expected Outcome:** User guide with examples for query API and CLI commands
- **Scope:** `docs/`, `packages/kindling-integration/README.md` (already exists)
- **Non-scope:** Video tutorials
- **Files:**
  - `docs/guides/kindling-memory.md`
  - `packages/kindling-integration/examples/byo-ai.ts`
  - Update `packages/kindling-integration/README.md` with real examples
- **Dependencies:** KINDLING-012, KINDLING-013
- **Validation:** Manual review of documentation completeness
- **Confidence:** high
- **Status:** Complete (2026-02-15)

#### KINDLING-019: OpenAPI spec generation

- **Intent:** Generate machine-readable OpenAPI 3.1 spec from Zod schemas for codegen
- **Expected Outcome:** OpenAPI spec file enables client library generation in any language
- **Scope:** `packages/kindling-integration/`
- **Non-scope:** Client library implementation (just spec)
- **Files:**
  - `packages/kindling-integration/scripts/generate-openapi.ts`
  - `packages/kindling-integration/openapi.json` (generated)
  - `packages/kindling-integration/openapi.yaml` (generated)
- **Dependencies:** KINDLING-001, KINDLING-009
- **Validation:** `pnpm run generate:openapi && validate openapi.json against OpenAPI 3.1 schema`
- **Confidence:** high
- **Status:** Complete (2026-02-15)

## Decisions

**D-KINDLING-001:** Read-only enforcement at API level, not just documentation

- **Rationale:** Prevent "convenient" bypasses that break the contract
- **Alternatives:** Trust developers to not write alternate read paths
- **Trade-offs:** More rigorous, but prevents future violations

**D-KINDLING-002:** Opt-in by default (privacy-first)

- **Rationale:** Users must explicitly enable observation capture
- **Alternatives:** Opt-out with .gitignore
- **Trade-offs:** Lower adoption, but respects user agency and privacy

**D-KINDLING-003:** Session-based capsules (not task-based)

- **Rationale:** Aligns with CLI execution model (one run = one session)
- **Alternatives:** Task-based capsules (requires APS plan parsing)
- **Trade-offs:** Coarser grouping, but works planless-first

**D-KINDLING-004:** SQLite local storage (no remote sync)

- **Rationale:** Privacy-first, no external dependencies, fast queries
- **Alternatives:** Remote backend for team sharing
- **Trade-offs:** No team collaboration, but preserves privacy

**D-KINDLING-005:** Async observation emission only

- **Rationale:** Must not add latency to gate execution (< 50ms)
- **Alternatives:** Synchronous with timeout
- **Trade-offs:** Potential observation loss on crash, but acceptable for v1

**D-KINDLING-006:** No semantic search in v1

- **Rationale:** Bounded queries enforce system-of-record contract
- **Alternatives:** Add FTS5 free-text search
- **Trade-offs:** Less "convenient" but prevents AI vacuum cleaner patterns

**D-KINDLING-007:** CLI symmetry (thin wrapper over query API)

- **Rationale:** Human and AI use same query surface (no hidden paths)
- **Alternatives:** Separate CLI and programmatic APIs
- **Trade-offs:** More consistency, easier to audit

## Notes

**Package structure:**

```
packages/kindling-integration/
├── src/
│   ├── kindling-service.ts      # Core service (emit + query)
│   ├── config.ts                # Configuration schema
│   ├── query-service.ts         # Read-only query API
│   ├── query-limits.ts          # Anti-vacuum-cleaner enforcement
│   ├── sensitive-data-validator.ts  # Secret detection
│   ├── retention.ts             # Auto-pruning
│   ├── emitters/
│   │   ├── session-emitter.ts   # session_start/end
│   │   ├── gate-emitter.ts      # gate_evaluated
│   │   ├── action-emitter.ts    # action_executed
│   │   ├── plan-emitter.ts      # plan_created/edited/approved/rejected
│   │   ├── human-input-emitter.ts   # human_input
│   │   ├── constraint-emitter.ts    # constraint_applied
│   │   └── error-emitter.ts     # error
│   ├── observation-contract.ts  # (already exists)
│   ├── query-contract.ts        # (already exists)
│   └── index.ts                 # (already exists)
├── benchmarks/
│   └── emission-overhead.bench.ts
├── examples/
│   └── byo-ai.ts                # BYO-AI integration example
├── scripts/
│   └── generate-openapi.ts      # OpenAPI spec generator
├── CONTRACTS.md                 # (already exists)
├── README.md                    # (already exists)
├── openapi.json                 # (generated)
└── openapi.yaml                 # (generated)
```

**CLI command structure:**

```
anvil run show <run_id> [--json]
  → SessionQuery { scope: 'session', session_id: run_id }

anvil plan trace <plan_id> [--json]
  → PlanQuery { scope: 'plan', plan_id: plan_id }

anvil gate show <gate_eval_id> [--json]
  → GateQuery { scope: 'gate', gate_eval_id: gate_eval_id }

anvil action show <action_id> [--json]
  → ActionQuery { scope: 'action', action_id: action_id }
```

**Integration points (where to emit):**

| Observation Kind | Integration Point |
|------------------|-------------------|
| session_start/end | `cli/src/commands/*.ts` — every command entry/exit |
| gate_evaluated | `core/src/gate/gate-runner.ts` — after gate.run() |
| action_executed | Anywhere Anvil executes commands via child_process |
| plan_* | `core/src/aps/` — plan lifecycle events |
| human_input | `cli/src/tui/` — confirmation prompts, approval flags |
| constraint_applied | `core/src/gate/` — when gate blocks action |
| error | All try/catch blocks handling failures |

**BYO-AI example usage:**

```typescript
import { KindlingService, SessionQuery } from '@eddacraft/anvil-kindling-integration';

// User brings their own AI (e.g., Claude via API)
async function explainGateFailure(runId: string) {
  // 1. Query Kindling (read-only, bounded)
  const kindling = new KindlingService(config);
  const response = await kindling.query({
    scope: 'session',
    session_id: runId,
    shape: 'timeline',
  });

  // 2. AI interprets facts (external AI service)
  const observations = response.observations.filter(
    obs => obs.kind === 'gate_evaluated' && obs.payload.outcome === 'fail'
  );

  const prompt = `Explain these gate failures:\n${JSON.stringify(observations, null, 2)}`;
  const explanation = await callExternalAI(prompt);

  // 3. AI stores interpretation in its own memory (NOT in Kindling)
  await userAI.memory.store({
    type: 'gate_failure_explanation',
    run_id: runId,
    explanation,
    generated_at: new Date().toISOString(),
  });

  return explanation;
}
```

**Mental model (repeat to team):**

```
Kindling records
Anvil orchestrates
Users (or their AI) interpret

Truth never moves
```

**OpenAPI spec generation (KINDLING-019):**

The Zod schemas in `observation-contract.ts` and `query-contract.ts` can be automatically converted to OpenAPI 3.1 spec using `@asteasolutions/zod-to-openapi`. This enables:

- **Client library generation**: Use OpenAPI generators for TypeScript, Python, Go, Rust, etc.
- **API documentation**: Auto-generated interactive docs (Swagger UI, Redoc)
- **Contract testing**: Validate implementations against the spec
- **IDE autocomplete**: Import spec into tools like Postman, Insomnia

Example command:
```bash
pnpm run generate:openapi
# Outputs: openapi.json, openapi.yaml
```

Client generation example:
```bash
# TypeScript client
npx openapi-generator-cli generate -i openapi.yaml -g typescript-fetch -o ./clients/ts

# Python client
openapi-generator-cli generate -i openapi.yaml -g python -o ./clients/python
```

This is crucial for BYO-AI integration — users can generate type-safe clients in their language of choice.

**Future enhancements (v2+):**

- Semantic search (FTS5 free-text)
- Pattern detection ("this warning appears every Monday")
- Context injection (pre-populate AI tools with relevant observations)
- Team memory (opt-in sharing of anonymised observations)
- TUI memory browser (timeline visualisation)
- Trend analysis dashboard
