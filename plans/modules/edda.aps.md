<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Edda — Canonical Memory System

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| EDDA  | —     | medium   | Draft  |

## Purpose

Edda is the canonical memory layer — a curated, low-volume, high-trust knowledge
store. It preserves only what has crossed a deliberate threshold and earned
permanence.

**Problem:** Without canonical memory:

- Institutional knowledge exists only in human heads or scattered docs
- Decisions are relitigated because context is lost
- Patterns are rediscovered repeatedly at cost
- AI systems lack authoritative grounding (hallucination risk)
- Lessons learned are forgotten between projects/teams

**Solution:** Edda provides:

- **Durable memory objects**: Explicit structure, provenance, intent
- **Reference and retrieval**: Stable identifiers, deterministic lookup
- **Meaning preservation**: Resists drift, makes change explicit
- **Promotion pathway**: Deliberate handoff from Ember candidates
- **Evolution graph**: Supersedes/superseded-by relationships

**Governing Rule:**

> Edda is not a log. It is not a transcript. It is not a database of everything.
> It is an institutional memory: decisions, lessons, patterns, constraints, truths.
> Edda forgets aggressively by default. That is a feature, not a bug.

**Technical Character:**

- Low-volume, high-trust
- Versioned, append-biased, auditable
- Migration-safe (breaking Edda is unacceptable)
- AI-assisted, not AI-authored
- Promotion must be deliberate and attributable

## In Scope

**Memory Object Types:**

- `decision` — A choice made with context and consequences
- `pattern` — A recurring structure worth codifying
- `constraint` — A boundary or limitation to respect
- `warning` — A persistent caution or known risk
- `doctrine` — An organisational principle or belief
- `lesson` — A learning from experience

**Memory Object Structure:**

- Stable, referable ID
- Type classification
- Statement (the remembered truth)
- Context (when/why/under what conditions)
- Provenance (links to Kindling + Ember artefacts)
- Confidence level (human-asserted, not inferred)
- Evolution graph (supersedes/superseded_by)
- Attribution (who promoted, when, why)

**Promotion Workflow:**

- Review Ember candidate
- Human decision to promote (explicit action)
- Attribution recorded (who, when, rationale)
- Provenance links preserved
- Memory object created (immutable once written)

**Query API:**

- Get by ID (deterministic lookup)
- List by type, recency, confidence
- Search by metadata (structured queries only)
- Evolution traversal (what superseded what)
- Provenance traversal (trace back to observations)

**Storage:**

- Git-backed markdown/JSON/YAML (versioned, auditable)
- Strongly versioned document store (alternative)
- Append-biased (mutations create new versions)
- Human-readable format preferred

**CLI Commands:**

- `anvil edda list [--type <type>]` — List memory objects
- `anvil edda show <id>` — Display memory object with provenance
- `anvil edda promote <candidate_id>` — Promote Ember candidate to Edda
- `anvil edda retire <id> --reason <reason>` — Mark memory as superseded
- `anvil edda trace <id>` — Show evolution and provenance graph

## Out of Scope (v1)

- ❌ AI-authored memory creation (AI assists, humans decide)
- ❌ Auto-promotion from Ember (always human-triggered)
- ❌ Semantic/embedding-based retrieval
- ❌ Cross-workspace memory sharing
- ❌ Real-time synchronisation
- ❌ Memory merging/deduplication
- ❌ Conflict resolution for concurrent edits
- ❌ Memory expiry (Edda memories are permanent unless superseded)

**These may be considered for v2+ or separate tooling.**

## Interfaces

**Depends on:**

- `ember` — Candidate memory proposals for promotion
- `kindling-integration` — Observation provenance links

**Exposes:**

- `MemoryService` — High-level API for memory management
- `MemoryStore` — Storage layer for memory objects
- `PromotionService` — Handles Ember → Edda promotion workflow
- `ProvenanceService` — Traces memory back to observations
- `EvolutionService` — Manages supersedes/superseded_by relationships
- CLI commands: `anvil edda list`, `anvil edda show`, `anvil edda promote`, etc.
- Configuration schema in `.anvilrc` (edda section)

**Configuration Example:**

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

## Boundary Rules

- EDDA must never create memory without human decision
- All mutations create new versions (append-only semantics)
- Provenance links must be preserved and queryable
- AI may summarise/suggest but never author final memory
- Breaking schema changes require migration tooling
- Confidence levels are human-asserted, not computed
- Superseded memories remain queryable (soft delete only)
- Attribution is mandatory (who promoted, when, why)

## Acceptance Criteria

- [ ] Memory Object schema defined (Zod) with all required fields
- [ ] 6 memory types supported (decision, pattern, constraint, warning, doctrine, lesson)
- [ ] Git-backed storage with YAML format
- [ ] Promotion workflow requires human decision + attribution
- [ ] `anvil edda list` shows memory objects with type/confidence
- [ ] `anvil edda show <id>` displays full memory with provenance
- [ ] `anvil edda promote <candidate_id>` creates memory from Ember
- [ ] `anvil edda retire <id>` marks memory as superseded
- [ ] `anvil edda trace <id>` shows evolution graph
- [ ] Provenance links resolve to Kindling observations
- [ ] All memory mutations are versioned (git commits)
- [ ] No AI-only memory creation (human in the loop)
- [ ] Schema changes require explicit migration

## Risks & Mitigations

| Risk                              | Mitigation                                                |
| --------------------------------- | --------------------------------------------------------- |
| Memory accumulates without review | Regular review prompts, usage metrics                     |
| Promotion without context         | Require reason, copy Ember rationale by default           |
| Schema evolution breaks data      | Strict versioning, migration tooling, backwards compat    |
| AI creates memory without human   | Enforcement tests, no auto-promote paths                  |
| Superseded memory confusion       | Clear UI distinction, evolution graph visualisation       |
| Provenance links break            | Validate links on query, graceful degradation             |
| Storage format portability        | YAML/JSON human-readable, git-standard tooling            |

## Tasks

### Phase A: Contracts & Schema

#### EDDA-001: Memory Object schema

- **Intent:** Define the core data model for canonical memory objects
- **Expected Outcome:** Zod schema for MemoryObject with all required fields
- **Scope:** `packages/edda-stack/contracts/`
- **Non-scope:** Storage implementation, promotion workflow
- **Files:**
  - `packages/edda-stack/contracts/edda-memory.ts`
  - `packages/edda-stack/contracts/edda-memory.test.ts`
- **Dependencies:** —
- **Validation:** `nx test edda-stack --testNamePattern="edda-memory"`
- **Confidence:** high
- **Status:** Draft

#### EDDA-002: Memory type definitions

- **Intent:** Define the 6 memory types with their specific fields
- **Expected Outcome:** Type-specific schemas for decision, pattern, constraint, warning, doctrine, lesson
- **Scope:** `packages/edda-stack/contracts/`
- **Non-scope:** Promotion rules for each type
- **Files:**
  - `packages/edda-stack/contracts/memory-types.ts`
  - `packages/edda-stack/contracts/memory-types.test.ts`
- **Dependencies:** EDDA-001
- **Validation:** `nx test edda-stack --testNamePattern="memory-types"`
- **Confidence:** high
- **Status:** Draft

#### EDDA-003: Provenance schema

- **Intent:** Define provenance links connecting memory to observations
- **Expected Outcome:** Schema for provenance with Kindling + Ember references
- **Scope:** `packages/edda-stack/contracts/`
- **Non-scope:** Link resolution implementation
- **Files:**
  - `packages/edda-stack/contracts/provenance.ts`
  - `packages/edda-stack/contracts/provenance.test.ts`
- **Dependencies:** EDDA-001, `kindling-integration`, `ember`
- **Validation:** `nx test edda-stack --testNamePattern="provenance"`
- **Confidence:** high
- **Status:** Draft

#### EDDA-004: Evolution graph schema

- **Intent:** Define supersedes/superseded_by relationship model
- **Expected Outcome:** Schema for evolution tracking between memory versions
- **Scope:** `packages/edda-stack/contracts/`
- **Non-scope:** Graph traversal implementation
- **Files:**
  - `packages/edda-stack/contracts/evolution.ts`
  - `packages/edda-stack/contracts/evolution.test.ts`
- **Dependencies:** EDDA-001
- **Validation:** `nx test edda-stack --testNamePattern="evolution"`
- **Confidence:** high
- **Status:** Draft

#### EDDA-005: Edda configuration schema

- **Intent:** Define configuration schema with storage and promotion settings
- **Expected Outcome:** Zod schema for edda config, integrated with .anvilrc
- **Scope:** `packages/edda-stack/edda/`, `core/src/gate/gate-config.ts`
- **Non-scope:** TUI config editor
- **Files:**
  - `packages/edda-stack/edda/config.ts`
  - `core/src/gate/gate-config.ts` (extend schema)
- **Dependencies:** EDDA-001
- **Validation:** `nx test edda-stack --testNamePattern="edda.*config"`
- **Confidence:** high
- **Status:** Draft

### Phase B: Storage

#### EDDA-006: Git-backed MemoryStore

- **Intent:** Implement git-backed storage for memory objects
- **Expected Outcome:** MemoryStore with CRUD operations, git versioning
- **Scope:** `packages/edda-stack/edda/`
- **Non-scope:** Alternative storage backends
- **Files:**
  - `packages/edda-stack/edda/memory-store.ts`
  - `packages/edda-stack/edda/memory-store.test.ts`
- **Dependencies:** EDDA-001, EDDA-005
- **Validation:** `nx test edda-stack --testNamePattern="MemoryStore"`
- **Confidence:** high
- **Status:** Draft

#### EDDA-007: YAML serialisation

- **Intent:** Implement human-readable YAML format for memory objects
- **Expected Outcome:** Serialise/deserialise memory to YAML with schema validation
- **Scope:** `packages/edda-stack/edda/`
- **Non-scope:** JSON/Markdown alternatives
- **Files:**
  - `packages/edda-stack/edda/serialisation.ts`
  - `packages/edda-stack/edda/serialisation.test.ts`
- **Dependencies:** EDDA-001
- **Validation:** `nx test edda-stack --testNamePattern="serialisation"`
- **Confidence:** high
- **Status:** Draft

#### EDDA-008: Version tracking

- **Intent:** Track all memory changes via git commits
- **Expected Outcome:** Every mutation creates git commit with attribution
- **Scope:** `packages/edda-stack/edda/`
- **Non-scope:** Conflict resolution, merge handling
- **Files:**
  - `packages/edda-stack/edda/version-tracker.ts`
  - `packages/edda-stack/edda/version-tracker.test.ts`
- **Dependencies:** EDDA-006
- **Validation:** `nx test edda-stack --testNamePattern="version-tracker"`
- **Confidence:** medium
- **Status:** Draft

### Phase C: Core Services

#### EDDA-009: PromotionService

- **Intent:** Handle Ember → Edda promotion workflow with human decision
- **Expected Outcome:** Promote candidate to memory with attribution and provenance
- **Scope:** `packages/edda-stack/edda/`
- **Non-scope:** Auto-promotion, batch promotion
- **Files:**
  - `packages/edda-stack/edda/promotion-service.ts`
  - `packages/edda-stack/edda/promotion-service.test.ts`
- **Dependencies:** EDDA-006, `ember`
- **Validation:** `nx test edda-stack --testNamePattern="PromotionService"`
- **Confidence:** high
- **Status:** Draft

#### EDDA-010: ProvenanceService

- **Intent:** Trace memory back to Kindling observations and Ember candidates
- **Expected Outcome:** Resolve provenance links, validate integrity
- **Scope:** `packages/edda-stack/edda/`
- **Non-scope:** Cross-workspace provenance
- **Files:**
  - `packages/edda-stack/edda/provenance-service.ts`
  - `packages/edda-stack/edda/provenance-service.test.ts`
- **Dependencies:** EDDA-003, `kindling-integration`, `ember`
- **Validation:** `nx test edda-stack --testNamePattern="ProvenanceService"`
- **Confidence:** medium
- **Status:** Draft

#### EDDA-011: EvolutionService

- **Intent:** Manage supersedes/superseded_by relationships
- **Expected Outcome:** Create evolution links, traverse graph, retire memories
- **Scope:** `packages/edda-stack/edda/`
- **Non-scope:** Automatic evolution detection
- **Files:**
  - `packages/edda-stack/edda/evolution-service.ts`
  - `packages/edda-stack/edda/evolution-service.test.ts`
- **Dependencies:** EDDA-004, EDDA-006
- **Validation:** `nx test edda-stack --testNamePattern="EvolutionService"`
- **Confidence:** high
- **Status:** Draft

#### EDDA-012: MemoryService (high-level API)

- **Intent:** Unified API for memory management orchestrating all services
- **Expected Outcome:** MemoryService provides complete memory lifecycle operations
- **Scope:** `packages/edda-stack/edda/`
- **Non-scope:** CLI implementation
- **Files:**
  - `packages/edda-stack/edda/memory-service.ts`
  - `packages/edda-stack/edda/memory-service.test.ts`
- **Dependencies:** EDDA-006, EDDA-009, EDDA-010, EDDA-011
- **Validation:** `nx test edda-stack --testNamePattern="MemoryService"`
- **Confidence:** high
- **Status:** Draft

### Phase D: CLI

#### EDDA-013: CLI list and show commands

- **Intent:** Add `anvil edda list` and `anvil edda show` CLI commands
- **Expected Outcome:** Users can browse and inspect memory objects
- **Scope:** `cli/src/commands/`
- **Non-scope:** TUI visualisation
- **Files:**
  - `cli/src/commands/edda.ts`
  - `cli/src/commands/edda.test.ts`
- **Dependencies:** EDDA-012
- **Validation:** `anvil edda list && anvil edda show <id>`
- **Confidence:** high
- **Status:** Draft

#### EDDA-014: CLI promote command

- **Intent:** Add `anvil edda promote` command for Ember → Edda workflow
- **Expected Outcome:** CLI guides user through promotion with attribution capture
- **Scope:** `cli/src/commands/`
- **Non-scope:** Batch promotion, auto-promotion
- **Files:**
  - `cli/src/commands/edda.ts` (add promote subcommand)
- **Dependencies:** EDDA-009, EDDA-013
- **Validation:** `anvil edda promote <candidate_id> --reason "reason"`
- **Confidence:** high
- **Status:** Draft

#### EDDA-015: CLI retire and trace commands

- **Intent:** Add `anvil edda retire` and `anvil edda trace` commands
- **Expected Outcome:** Users can retire memories and trace evolution/provenance
- **Scope:** `cli/src/commands/`
- **Non-scope:** Complex graph visualisation
- **Files:**
  - `cli/src/commands/edda.ts` (add retire, trace subcommands)
- **Dependencies:** EDDA-011, EDDA-013
- **Validation:** `anvil edda retire <id> && anvil edda trace <id>`
- **Confidence:** high
- **Status:** Draft

### Phase E: Integration & Validation

#### EDDA-016: Human-in-the-loop enforcement tests

- **Intent:** Prove that AI cannot create memory without human decision
- **Expected Outcome:** Tests that auto-creation, AI-only promotion fail
- **Scope:** `packages/edda-stack/edda/`
- **Non-scope:** AI integration (just validation)
- **Files:**
  - `packages/edda-stack/edda/human-in-loop.test.ts`
- **Dependencies:** EDDA-009, EDDA-012
- **Validation:** `nx test edda-stack --testNamePattern="human-in-loop"`
- **Confidence:** high
- **Status:** Draft

#### EDDA-017: Status integration

- **Intent:** Show Edda stats in `anvil status` output
- **Expected Outcome:** Status displays memory count by type, recent promotions
- **Scope:** `cli/src/commands/status.ts`
- **Non-scope:** Detailed memory browser
- **Files:**
  - `cli/src/commands/status.ts` (add edda section)
- **Dependencies:** EDDA-012
- **Validation:** `anvil status | grep -A5 "Edda"`
- **Confidence:** high
- **Status:** Draft

#### EDDA-018: Schema migration tooling

- **Intent:** Provide tools for migrating memory objects across schema versions
- **Expected Outcome:** Migration scripts validate and upgrade memory format
- **Scope:** `packages/edda-stack/edda/`
- **Non-scope:** Automatic migration on startup
- **Files:**
  - `packages/edda-stack/edda/migration/`
  - `packages/edda-stack/edda/migration/migrate.ts`
  - `packages/edda-stack/edda/migration/migrate.test.ts`
- **Dependencies:** EDDA-001, EDDA-006
- **Validation:** `nx test edda-stack --testNamePattern="migration"`
- **Confidence:** medium
- **Status:** Draft

#### EDDA-019: Documentation

- **Intent:** Document Edda architecture, CLI usage, promotion workflow
- **Expected Outcome:** User guide with examples for memory management
- **Scope:** `docs/`, `packages/edda-stack/edda/README.md`
- **Non-scope:** Video tutorials
- **Files:**
  - `docs/guides/edda-memory.md`
  - `packages/edda-stack/edda/README.md`
  - `packages/edda-stack/edda/examples/`
- **Dependencies:** EDDA-013, EDDA-014, EDDA-015
- **Validation:** Manual review of documentation completeness
- **Confidence:** high
- **Status:** Draft

## Decisions

**D-EDDA-001:** Git-backed storage (versioned, auditable)

- **Rationale:** Human-readable, built-in versioning, portable, auditable
- **Alternatives:** SQLite (faster queries), document store (scalability)
- **Trade-offs:** Slower queries, but perfect audit trail and simplicity

**D-EDDA-002:** YAML format (human-readable)

- **Rationale:** Humans can read/edit directly, git diffs are meaningful
- **Alternatives:** JSON (more universal), Markdown (more prose-friendly)
- **Trade-offs:** Less structured than JSON, but more readable

**D-EDDA-003:** Human decision required for all promotions

- **Rationale:** AI-assisted, not AI-authored; prevents hallucination creep
- **Alternatives:** Auto-promote high-confidence candidates
- **Trade-offs:** Slower, but trustworthy

**D-EDDA-004:** Attribution mandatory

- **Rationale:** Accountability — know who promoted what and why
- **Alternatives:** Optional attribution
- **Trade-offs:** More friction, but better governance

**D-EDDA-005:** Soft delete only (supersedes, not delete)

- **Rationale:** Audit trail preserved; can trace why something changed
- **Alternatives:** Hard delete with archive
- **Trade-offs:** Storage grows, but history is preserved

**D-EDDA-006:** Confidence is human-asserted

- **Rationale:** Humans judge truth, not algorithms
- **Alternatives:** Computed confidence from Ember
- **Trade-offs:** More work for humans, but more trustworthy

## Notes

**Package structure:**

```
packages/edda-stack/edda/
├── memory-service.ts         # High-level API
├── memory-store.ts           # Git-backed storage
├── promotion-service.ts      # Ember → Edda workflow
├── provenance-service.ts     # Link resolution
├── evolution-service.ts      # Supersedes graph
├── version-tracker.ts        # Git versioning
├── serialisation.ts          # YAML format
├── config.ts                 # Configuration schema
├── migration/                # Schema migration tools
│   └── migrate.ts
├── examples/                 # Usage examples
├── README.md
└── index.ts
```

**Memory Object (minimal fields):**

```typescript
interface MemoryObject {
  id: string; // Stable, referable ID
  type: MemoryType; // decision | pattern | constraint | warning | doctrine | lesson
  statement: string; // The remembered truth
  context: {
    // When/why/under what conditions
    when: string; // ISO8601 or description
    why: string; // Rationale
    conditions: string[]; // Applicability conditions
  };
  provenance: {
    // Links to observations
    ember_candidate_id?: string;
    kindling_observation_ids: string[];
    source_sessions: string[];
  };
  confidence: 'low' | 'medium' | 'high'; // Human-asserted
  attribution: {
    promoted_by: string; // Who
    promoted_at: string; // When (ISO8601)
    reason: string; // Why promoted
  };
  evolution: {
    supersedes?: string[]; // Memory IDs this replaces
    superseded_by?: string; // Memory ID that replaced this
    retired_at?: string; // When retired (ISO8601)
    retired_reason?: string; // Why retired
  };
  created_at: string; // ISO8601
  version: number; // Schema version
}
```

**Storage structure (.anvil/edda/):**

```
.anvil/edda/
├── memories/
│   ├── decision/
│   │   ├── mem-001.yaml
│   │   └── mem-002.yaml
│   ├── pattern/
│   │   └── mem-003.yaml
│   └── constraint/
│       └── mem-004.yaml
├── index.yaml              # Memory index for fast lookup
└── .git/                   # Git versioning
```

**Mental model:**

```
Ember is a queue — you empty it intentionally
Edda is a ledger — you never casually rewrite it

If you can't explain why something is in Edda, it doesn't belong.
```

**Failure modes this architecture avoids:**

- Log-as-memory collapse
- AI hallucinations becoming "facts"
- Memory inflation
- Silent drift of institutional knowledge
- Agent feedback loops reinforcing errors

**Schema sharing decision (critical):**

The question of whether Ember and Edda share a schema language is architecturally
significant. Current design assumes:

- **Shared contracts package** for common types (ProposalType ↔ MemoryType mapping)
- **Different storage schemas** (Ember: ephemeral SQLite, Edda: versioned YAML)
- **Explicit promotion mapping** (not automatic type conversion)

This can be revisited, but once decided, is hard to change.

**Future enhancements (v2+):**

- AI-assisted summarisation (draft statements, not final)
- Semantic search across memories
- Cross-workspace memory sharing (opt-in)
- Memory usage analytics
- TUI memory browser with evolution visualisation
- Integration with external knowledge bases
- Memory export to standard formats (ADR, RFC)
