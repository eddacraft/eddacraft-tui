<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Ember — Candidate Memory System

| Scope | Owner | Priority | Status   |
| ----- | ----- | -------- | -------- |
| EMBER | —     | medium   | Complete |

## Purpose

Ember is the interpretive layer between raw observation (Kindling) and durable
memory (Edda). It consumes high-volume, low-trust signals and produces
low-volume, medium-trust proposals for potential memorisation.

**Problem:** Without an interpretive layer:

- Raw observations accumulate without meaning extraction
- AI systems can't distinguish signal from noise
- Pattern recognition requires re-processing entire observation history
- Memory promotion happens ad-hoc, without evaluation criteria
- Hallucinations can enter institutional memory unchallenged

**Solution:** Ember provides:

- **Candidate generation**: Proposes what might be worth remembering
- **Pattern detection**: Correlates observations across time, agents, sessions
- **Decay by default**: Proposals expire unless promoted (prevents bloat)
- **Heuristic evaluation**: Scores memorability without claiming truth
- **Provenance links**: Every proposal traces back to Kindling facts

**Governing Rule:**

> Ember has taste, but no power. It proposes memory. It does not create it.

**Technical Character:**

- Write-heavy, ephemeral by default
- Allowed to be wrong (probabilistic thinking)
- May use AI but must not depend on it
- Breaking Ember data is acceptable (unlike Edda)

## In Scope

**Candidate Memory Proposals:**

- Aggregation of related Kindling observations
- Pattern detection (repetition, deviation, resolution, escalation)
- Confidence scoring via rules or heuristics
- Expiry/decay semantics for unpromoted proposals
- Rationale capture (why this was proposed)

**Proposal Types:**

- `decision` — A choice made with consequences
- `pattern` — A recurring structure or behaviour
- `warning` — A signal of potential problems
- `lesson` — A learning from failure or success
- `anomaly` — An unexpected deviation
- `constraint` — A discovered limitation or boundary

**Evaluation Signals:**

- Repetition (same pattern appears N times)
- Convergence (multiple agents reach same conclusion)
- Escalation (severity increasing over time)
- Surprise (deviation from expected patterns)
- Resolution (problem → solution linkage)
- Cross-agent interaction (coordination signals)
- Human intervention (indicates significance)

**Query API:**

- List candidates by type, recency, confidence
- Get candidate details with linked observations
- Filter by source session, agent, time range
- Expiry status queries (near-decay candidates)

**Storage:**

- Local SQLite/DuckDB (fast, cheap, disposable)
- Append-friendly with TTL semantics
- No strong consistency requirements
- No backwards compatibility guarantees needed

## Out of Scope (v1)

- ❌ AI-powered evaluation (heuristics only for v1)
- ❌ Semantic similarity detection (no embeddings)
- ❌ Cross-workspace candidate sharing
- ❌ Real-time streaming of candidates
- ❌ Promotion to Edda (separate concern)
- ❌ User-facing candidate management UI
- ❌ Training or learning from feedback
- ❌ Custom evaluation rule authoring

**These belong to Edda, future Ember versions, or separate tooling.**

## Interfaces

**Depends on:**

- `kindling-integration` — Observation data source (Kindling observations)
- `@kindling/core` — Kindling primitives for querying observations

**Exposes:**

- `CandidateService` — High-level API for proposal generation and query
- `AggregatorService` — Groups related Kindling observations
- `EvaluatorService` — Scores candidates using rules/heuristics
- `ProposalStore` — Storage layer for candidate memory proposals
- `DecayService` — Handles expiry and cleanup of stale proposals
- CLI commands: `anvil ember list`, `anvil ember show`, `anvil ember promote`
- Configuration schema in `.anvilrc` (ember section)

**Configuration Example:**

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

## Boundary Rules

- EMBER must never create Edda memory objects directly
- Proposals are suggestions, not assertions
- All proposals must link to source Kindling observations
- Confidence scores are heuristic, not authoritative
- AI outputs (if used) must be advisory and explainable
- Ember must degrade gracefully to heuristics if AI unavailable
- Database schema may break between versions (ephemeral by design)
- No proposal survives indefinitely without promotion

## Acceptance Criteria

- [x] Candidate Memory Proposal schema defined (Zod)
- [x] 6 proposal types supported (decision, pattern, warning, lesson, anomaly, constraint)
- [x] Aggregator groups Kindling observations by correlation rules
- [x] Evaluator produces confidence scores from heuristics
- [x] Proposals expire after configurable TTL
- [x] DecayService removes expired proposals on schedule
- [x] Query API supports type, recency, confidence filters
- [x] Proposals include provenance links to Kindling IDs
- [x] `anvil ember list` shows active candidates
- [x] `anvil ember show <id>` displays candidate with linked observations
- [x] Storage uses SQLite with TTL semantics
- [x] No runtime AI dependencies (heuristics only for v1)

## Risks & Mitigations

| Risk                               | Mitigation                                               |
| ---------------------------------- | -------------------------------------------------------- |
| Proposals accumulate without decay | Aggressive default TTL, DecayService runs on every start |
| Low-quality proposals add noise    | Configurable min_confidence threshold                    |
| Heuristics miss important patterns | Design for extensibility; add AI evaluation in v2        |
| Ember/Kindling schema drift        | Version observation references, validate on load         |
| Ember confused with Edda           | Clear naming, different storage locations, docs          |
| Performance impact from evaluation | Async evaluation, batch processing, sampling             |

## Tasks

### Phase A: Contracts & Foundation

#### EMBER-001: Candidate Memory Proposal schema

- **Intent:** Define the core data model for candidate memory proposals
- **Expected Outcome:** Zod schema for CandidateMemoryProposal with all fields
- **Scope:** `packages/edda-stack/src/contracts/`
- **Non-scope:** Storage implementation, evaluation logic
- **Files:**
  - `packages/edda-stack/src/contracts/ember-proposal.ts`
  - `packages/edda-stack/src/contracts/contracts.test.ts`
- **Dependencies:** —
- **Validation:** `nx test edda-stack --testNamePattern="Ember Proposals"`
- **Confidence:** high
- **Status:** Complete

#### EMBER-002: Proposal type definitions

- **Intent:** Define the 6 proposal types with their specific fields
- **Expected Outcome:** Type-specific schemas for decision, pattern, warning, lesson, anomaly, constraint
- **Scope:** `packages/edda-stack/src/contracts/`
- **Non-scope:** Evaluation rules for each type
- **Files:**
  - `packages/edda-stack/src/contracts/proposal-types.ts`
  - `packages/edda-stack/src/contracts/proposal-types.test.ts`
- **Dependencies:** EMBER-001
- **Validation:** `nx test edda-stack --testNamePattern="proposal-types"`
- **Confidence:** high
- **Status:** Complete

#### EMBER-003: Ember configuration schema

- **Intent:** Define configuration schema with sensible defaults
- **Expected Outcome:** Zod schema for ember config, integrated with .anvilrc
- **Scope:** `packages/edda-stack/src/ember/`, `packages/anvil/runtime/src/gate/gate-config.ts`
- **Non-scope:** TUI config editor
- **Files:**
  - `packages/edda-stack/src/ember/config.ts`
  - `packages/anvil/runtime/src/gate/gate-config.ts` (extend schema)
- **Dependencies:** EMBER-001
- **Validation:** `nx test edda-stack --testNamePattern="ember.*config"`
- **Confidence:** high
- **Status:** Complete

### Phase B: Storage & Decay

#### EMBER-004: ProposalStore implementation

- **Intent:** Implement SQLite-backed storage for candidate proposals
- **Expected Outcome:** ProposalStore with create, read, query, delete operations
- **Scope:** `packages/edda-stack/src/ember/`
- **Non-scope:** Decay scheduling, aggregation logic
- **Files:**
  - `packages/edda-stack/src/ember/proposal-store.ts`
  - `packages/edda-stack/src/ember/proposal-store.test.ts`
- **Dependencies:** EMBER-001, EMBER-003
- **Validation:** `nx test edda-stack --testNamePattern="ProposalStore"`
- **Confidence:** high
- **Status:** Complete

#### EMBER-005: DecayService implementation

- **Intent:** Implement expiry and cleanup of stale proposals
- **Expected Outcome:** DecayService removes proposals past TTL on schedule
- **Scope:** `packages/edda-stack/src/ember/`
- **Non-scope:** Custom decay rules per proposal type
- **Files:**
  - `packages/edda-stack/src/ember/decay-service.ts`
  - `packages/edda-stack/src/ember/decay-service.test.ts`
- **Dependencies:** EMBER-004
- **Validation:** `nx test edda-stack --testNamePattern="DecayService"`
- **Confidence:** high
- **Status:** Complete

### Phase C: Aggregation & Evaluation

#### EMBER-006: AggregatorService foundation

- **Intent:** Group related Kindling observations for candidate generation
- **Expected Outcome:** AggregatorService correlates observations by session, time, pattern
- **Scope:** `packages/edda-stack/src/ember/`
- **Non-scope:** Advanced correlation (cross-session, semantic)
- **Files:**
  - `packages/edda-stack/src/ember/aggregator-service.ts`
  - `packages/edda-stack/src/ember/aggregator-service.test.ts`
- **Dependencies:** `kindling-integration`, EMBER-004
- **Validation:** `nx test edda-stack --testNamePattern="AggregatorService"`
- **Confidence:** medium
- **Status:** Complete

#### EMBER-007: Evaluation rules engine

- **Intent:** Implement heuristic evaluation for pattern detection
- **Expected Outcome:** EvaluatorService produces confidence scores from rules
- **Scope:** `packages/edda-stack/src/ember/`
- **Non-scope:** AI-powered evaluation, learning
- **Files:**
  - `packages/edda-stack/src/ember/evaluator-service.ts`
  - `packages/edda-stack/src/ember/rules/`
  - `packages/edda-stack/src/ember/evaluator-service.test.ts`
- **Dependencies:** EMBER-006
- **Validation:** `nx test edda-stack --testNamePattern="EvaluatorService"`
- **Confidence:** medium
- **Status:** Complete

#### EMBER-008: Built-in evaluation rules

- **Intent:** Implement default rules for repetition, escalation, resolution
- **Expected Outcome:** Core heuristics detect common patterns
- **Scope:** `packages/edda-stack/src/ember/rules/`
- **Non-scope:** Custom rule authoring, AI rules
- **Files:**
  - `packages/edda-stack/src/ember/rules/repetition.rule.ts`
  - `packages/edda-stack/src/ember/rules/escalation.rule.ts`
  - `packages/edda-stack/src/ember/rules/resolution.rule.ts`
  - `packages/edda-stack/src/ember/rules/convergence.rule.ts`
  - `packages/edda-stack/src/ember/rules/surprise.rule.ts`
- **Dependencies:** EMBER-007
- **Validation:** `nx test edda-stack --testNamePattern="rules"`
- **Confidence:** medium
- **Status:** Complete

### Phase D: Service Integration

#### EMBER-009: CandidateService (high-level API)

- **Intent:** Unified API for candidate generation, query, and management
- **Expected Outcome:** CandidateService orchestrates aggregation, evaluation, storage
- **Scope:** `packages/edda-stack/src/ember/`
- **Non-scope:** CLI commands, promotion to Edda
- **Files:**
  - `packages/edda-stack/src/ember/candidate-service.ts`
  - `packages/edda-stack/src/ember/candidate-service.test.ts`
- **Dependencies:** EMBER-004, EMBER-005, EMBER-006, EMBER-007
- **Validation:** `nx test edda-stack --testNamePattern="CandidateService"`
- **Confidence:** high
- **Status:** Complete

#### EMBER-010: Kindling observation hooks

- **Intent:** Trigger candidate evaluation when new Kindling observations arrive
- **Expected Outcome:** Ember processes new observations automatically
- **Scope:** `packages/edda-stack/src/ember/`, `packages/kindling-integration/`
- **Non-scope:** Real-time streaming, webhooks
- **Files:**
  - `packages/edda-stack/src/ember/observation-hook.ts`
  - `packages/edda-stack/src/ember/observation-hook.test.ts`
- **Dependencies:** EMBER-009, `kindling-integration`
- **Validation:** `nx test edda-stack --testNamePattern="observation-hook"`
- **Confidence:** medium
- **Status:** Complete

### Phase E: CLI & Query

#### EMBER-011: CLI ember commands

- **Intent:** Add `anvil ember list`, `anvil ember show` CLI commands
- **Expected Outcome:** Users can view and explore candidate proposals
- **Scope:** `apps/anvil-cli/src/commands/`
- **Non-scope:** TUI visualisation, promotion workflow
- **Files:**
  - `apps/anvil-cli/src/commands/ember/index.ts`
  - `apps/anvil-cli/src/commands/ember/ember.test.ts`
- **Dependencies:** EMBER-009
- **Validation:** `anvil ember list && anvil ember show <id>`
- **Confidence:** high
- **Status:** Complete

#### EMBER-012: Query API implementation

- **Intent:** Implement query API for filtering candidates
- **Expected Outcome:** Query by type, recency, confidence, expiry status
- **Scope:** `packages/edda-stack/src/ember/`
- **Non-scope:** Free-text search, semantic queries
- **Files:**
  - `packages/edda-stack/src/ember/query-api.ts`
  - `packages/edda-stack/src/ember/query-api.test.ts`
- **Dependencies:** EMBER-004, EMBER-009
- **Validation:** `nx test edda-stack --testNamePattern="query-api"`
- **Confidence:** high
- **Status:** Complete

### Phase F: Integration & Polish

#### EMBER-013: Status integration

- **Intent:** Show Ember stats in `anvil status` output
- **Expected Outcome:** Status displays candidate count, near-expiry warnings, decay stats
- **Scope:** `apps/anvil-cli/src/commands/status.ts`
- **Non-scope:** Detailed candidate browser
- **Files:**
  - `apps/anvil-cli/src/commands/status.ts` (add ember section)
- **Dependencies:** EMBER-009
- **Validation:** `anvil status | grep -A5 "Ember"`
- **Confidence:** high
- **Status:** Complete

#### EMBER-014: Documentation and examples

- **Intent:** Document Ember architecture, CLI usage, integration patterns
- **Expected Outcome:** User guide with examples for candidate review workflow
- **Scope:** `docs/`, `packages/edda-stack/src/ember/README.md`
- **Non-scope:** Video tutorials
- **Files:**
  - `docs/guides/ember-candidates.md`
  - `packages/edda-stack/src/ember/README.md`
  - `packages/edda-stack/src/ember/`
- **Dependencies:** EMBER-011, EMBER-012
- **Validation:** Manual review of documentation completeness
- **Confidence:** high
- **Status:** Complete

## Decisions

**D-EMBER-001:** Heuristics-first, AI-optional

- **Rationale:** Must function without AI dependencies; AI is advisory
- **Alternatives:** AI-first with heuristic fallback
- **Trade-offs:** Simpler v1, but may miss subtle patterns

**D-EMBER-002:** Decay by default (proposals expire)

- **Rationale:** Prevents unbounded growth; forces promotion decision
- **Alternatives:** Manual cleanup, no expiry
- **Trade-offs:** Candidates may be lost if not reviewed

**D-EMBER-003:** SQLite for storage (ephemeral-friendly)

- **Rationale:** Fast, local, no migration concerns, can be recreated
- **Alternatives:** DuckDB (analytics-optimised), JSON files
- **Trade-offs:** Less sophisticated querying than DuckDB

**D-EMBER-004:** Proposal types are fixed (v1)

- **Rationale:** Reduces complexity; covers most use cases
- **Alternatives:** Extensible type system
- **Trade-offs:** May need new types added in v2

**D-EMBER-005:** Confidence is heuristic, not truth

- **Rationale:** Ember proposes, doesn't assert; clear semantics
- **Alternatives:** Confidence as probability
- **Trade-offs:** Less precise, but more honest

## Notes

**Package structure:**

```
packages/edda-stack/src/ember/
├── candidate-service.ts      # High-level API
├── proposal-store.ts         # SQLite storage
├── aggregator-service.ts     # Observation grouping
├── evaluator-service.ts      # Confidence scoring
├── decay-service.ts          # TTL enforcement
├── observation-hook.ts       # Kindling integration
├── query-api.ts              # Query interface
├── config.ts                 # Configuration schema
├── rules/                    # Built-in evaluation rules
│   ├── repetition.rule.ts
│   ├── escalation.rule.ts
│   ├── resolution.rule.ts
│   ├── convergence.rule.ts
│   └── surprise.rule.ts
├── README.md
└── index.ts
```

**Candidate Memory Proposal (minimal fields):**

```typescript
interface CandidateMemoryProposal {
  id: string; // Unique proposal ID
  type: ProposalType; // decision | pattern | warning | lesson | anomaly | constraint
  source_refs: string[]; // Kindling observation IDs
  confidence: number; // 0.0 - 1.0 heuristic score
  rationale: string; // Why this was proposed
  summary: string; // Brief description of candidate
  created_at: string; // ISO8601 timestamp
  expires_at: string; // ISO8601 timestamp (TTL)
  metadata: Record<string, unknown>; // Type-specific data
}
```

**Mental model:**

```
Kindling records everything (facts)
Ember proposes what might matter (candidates)
Edda preserves what we choose to keep (memory)

Ember is a queue that empties itself.
Edda is a ledger that persists.
```

**Future enhancements (v2+):**

- AI-powered evaluation (LLM scoring, semantic similarity)
- Cross-session candidate correlation
- Custom rule authoring
- Feedback loop (promoted → better heuristics)
- TUI candidate browser
- Batch promotion workflow
- Prometheus metrics export
