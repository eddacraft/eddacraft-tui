<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Kindling Memory Integration

| Scope    | Owner | Priority | Status |
| -------- | ----- | -------- | ------ |
| KINDLING | —     | medium   | Draft  |

## Purpose

Enable Anvil to capture, organise, and retrieve execution observations using Kindling's local-first memory system. Transform ephemeral gate check results, command outputs, and file changes into queryable context that helps developers understand patterns, debug issues, and learn from past executions.

**Problem:** Without persistent memory:

- Gate check failures are transient — once cleared, insights are lost
- Developers repeatedly encounter the same architecture violations without seeing trends
- Context from previous sessions isn't available when debugging similar issues
- Learning from past executions requires manual log archaeology

**Solution:** Kindling integration provides:

- Automatic capture of gate observations into bounded capsules
- FTS-powered retrieval of relevant past executions
- Provenance linking observations to tasks, checks, and validation outcomes
- Drift pattern discovery through historical query analysis

## In Scope

- Observation capture from gate check executions
- Capsule lifecycle management (open/append/close)
- Integration with GateRunner progress callbacks
- CLI commands for memory inspection and querying
- Configuration for opt-in enablement and capsule strategies
- Retrieval API for context-aware suggestions
- Observation types: check results, warnings, errors, file changes, command outputs

## Out of Scope

- Kindling core modifications (use as-is)
- Real-time streaming dashboard (v2)
- Team-level memory sharing (privacy-first, local only)
- Memory synchronisation across machines
- AI model fine-tuning from observations
- Automatic fix suggestions based on historical patterns (v2)

## Interfaces

**Depends on:**

- `save-time-trust` — GateRunner, check execution infrastructure
- `drift-reporting` — Snapshot data structures, comparison patterns
- Kindling packages: `@kindling/core`, `@kindling/store-sqlite`, `@kindling/provider-local`

**Exposes:**

- `MemoryService` — High-level API for observation capture and retrieval
- `anvil memory query <query>` — Search historical observations
- `anvil memory sessions` — List recent execution capsules
- `anvil memory show <session-id>` — Display capsule contents
- `anvil memory stats` — Memory database statistics
- `ObservationCapture` — Hook for gate runner integration
- Configuration schema for `.anvil/config.yaml` memory section

**Configuration Example:**

```yaml
memory:
  enabled: true
  database: .anvil/memory.db
  capsule_strategy: session # session | task | continuous
  auto_capture:
    gate_checks: true
    warnings: true
    errors: true
    file_changes: false # opt-in for privacy
  retention:
    days: 90
    max_capsules: 1000
```

## Boundary Rules

- KINDLING must not modify gate check behaviour or results
- Observation capture must be asynchronous and non-blocking
- Memory operations must fail gracefully if disabled or unavailable
- Privacy-sensitive data (secrets, credentials) must never be captured
- Database location must be workspace-local (no global state)

## Acceptance Criteria

- [ ] Gate check results captured as observations when memory enabled
- [ ] Capsules automatically created/managed per execution session
- [ ] `anvil memory query` returns relevant past observations with explanations
- [ ] < 50ms observation capture overhead per gate check
- [ ] Memory disabled by default (explicit opt-in required)
- [ ] Query results include provenance (task ID, check name, timestamp)
- [ ] Capsule summaries generated on close with key findings
- [ ] Memory database stored in `.anvil/` (git-ignored)

## Risks & Mitigations

| Risk                                    | Mitigation                                          |
| --------------------------------------- | --------------------------------------------------- |
| Observation capture adds latency       | Async recording, batching, SQLite WAL mode          |
| Database grows unbounded               | Configurable retention policy, auto-pruning         |
| Sensitive data captured accidentally   | Explicit allowlist for observation types            |
| Kindling version conflicts             | Pin Kindling versions, vendor if needed             |
| Query performance degrades             | FTS5 indexing, query result limits                  |
| Users forget memory is enabled         | Include stats in `anvil status`, show capture count |

## Tasks

### Phase A: Core Integration

#### KINDLING-001: Memory service foundation

- **Intent:** Create MemoryService wrapper around Kindling core APIs
- **Expected Outcome:** Service initialises Kindling store, manages capsule lifecycle
- **Scope:** `packages/kindling-integration/src/`
- **Non-scope:** Gate runner integration, CLI commands
- **Files:**
  - `packages/kindling-integration/src/memory-service.ts`
  - `packages/kindling-integration/src/config.ts`
  - `packages/kindling-integration/src/memory-service.test.ts`
- **Dependencies:** —
- **Validation:** `nx test kindling-integration --testNamePattern="MemoryService"`
- **Confidence:** high
- **Status:** Draft

#### KINDLING-002: Observation schema mapping

- **Intent:** Map Anvil types (GateResult, Warning, CheckContext) to Kindling observations
- **Expected Outcome:** Type-safe converters for all observable events
- **Scope:** `packages/kindling-integration/src/`
- **Non-scope:** Capture logic
- **Files:**
  - `packages/kindling-integration/src/observation-mapper.ts`
  - `packages/kindling-integration/src/observation-mapper.test.ts`
- **Dependencies:** KINDLING-001
- **Validation:** `nx test kindling-integration --testNamePattern="ObservationMapper"`
- **Confidence:** high
- **Status:** Draft

#### KINDLING-003: Gate runner observation hook

- **Intent:** Integrate observation capture into GateRunner progress callbacks
- **Expected Outcome:** GateRunner automatically records observations when memory enabled
- **Scope:** `core/src/gate/`, `packages/kindling-integration/src/`
- **Non-scope:** CLI commands
- **Files:**
  - `packages/kindling-integration/src/gate-observer.ts`
  - `core/src/gate/gate-runner.ts` (add observer hook)
  - `packages/kindling-integration/src/gate-observer.test.ts`
- **Dependencies:** KINDLING-001, KINDLING-002
- **Validation:** `nx test kindling-integration --testNamePattern="GateObserver" && nx test core --testNamePattern="gate-runner"`
- **Confidence:** medium
- **Status:** Draft

### Phase B: Capsule Management

#### KINDLING-004: Capsule lifecycle orchestration

- **Intent:** Manage capsule open/close lifecycle for execution sessions
- **Expected Outcome:** Capsules created per session with auto-close and summary generation
- **Scope:** `packages/kindling-integration/src/`
- **Non-scope:** Observation capture
- **Files:**
  - `packages/kindling-integration/src/capsule-manager.ts`
  - `packages/kindling-integration/src/capsule-manager.test.ts`
- **Dependencies:** KINDLING-001
- **Validation:** `nx test kindling-integration --testNamePattern="CapsuleManager"`
- **Confidence:** high
- **Status:** Draft

#### KINDLING-005: Session detection strategy

- **Intent:** Detect execution sessions from CLI invocations and watch mode
- **Expected Outcome:** Sessions bounded by CLI command lifecycle or watch intervals
- **Scope:** `packages/kindling-integration/src/`, `cli/src/commands/`
- **Non-scope:** Capsule storage
- **Files:**
  - `packages/kindling-integration/src/session-detector.ts`
  - `cli/src/commands/check.ts` (add session boundary markers)
  - `cli/src/commands/watch.ts` (add session boundary markers)
- **Dependencies:** KINDLING-004
- **Validation:** `nx test kindling-integration --testNamePattern="SessionDetector"`
- **Confidence:** medium
- **Status:** Draft

### Phase C: Retrieval & Query

#### KINDLING-006: Memory query service

- **Intent:** Expose Kindling retrieval API through Anvil-specific query interface
- **Expected Outcome:** Query service with scope filtering, ranking, and explanation
- **Scope:** `packages/kindling-integration/src/`
- **Non-scope:** CLI implementation
- **Files:**
  - `packages/kindling-integration/src/query-service.ts`
  - `packages/kindling-integration/src/query-service.test.ts`
- **Dependencies:** KINDLING-001
- **Validation:** `nx test kindling-integration --testNamePattern="QueryService"`
- **Confidence:** high
- **Status:** Draft

#### KINDLING-007: CLI memory commands

- **Intent:** Add `anvil memory` command group for inspection and querying
- **Expected Outcome:** Working `anvil memory query|sessions|show|stats` commands
- **Scope:** `cli/src/commands/`
- **Non-scope:** TUI visualisation
- **Files:**
  - `cli/src/commands/memory.ts`
  - `cli/src/commands/memory.test.ts`
- **Dependencies:** KINDLING-006
- **Validation:** `anvil memory --help && anvil memory query "architecture violations"`
- **Confidence:** high
- **Status:** Draft

### Phase D: Configuration & Privacy

#### KINDLING-008: Configuration schema and validation

- **Intent:** Define memory configuration schema with sensible defaults
- **Expected Outcome:** Memory config section in gate.yaml with Zod validation
- **Scope:** `core/src/gate/`, `packages/kindling-integration/src/`
- **Non-scope:** Config UI
- **Files:**
  - `packages/kindling-integration/src/config-schema.ts`
  - `core/src/gate/gate-config.ts` (extend schema)
- **Dependencies:** —
- **Validation:** `nx test kindling-integration --testNamePattern="config-schema"`
- **Confidence:** high
- **Status:** Draft

#### KINDLING-009: Privacy filtering and retention

- **Intent:** Filter sensitive data from observations and implement retention policies
- **Expected Outcome:** Secrets never captured, auto-pruning of old capsules
- **Scope:** `packages/kindling-integration/src/`
- **Non-scope:** Encryption
- **Files:**
  - `packages/kindling-integration/src/privacy-filter.ts`
  - `packages/kindling-integration/src/retention-policy.ts`
  - `packages/kindling-integration/src/privacy-filter.test.ts`
- **Dependencies:** KINDLING-002
- **Validation:** `nx test kindling-integration --testNamePattern="privacy"`
- **Confidence:** high
- **Status:** Draft

### Phase E: Integration & Polish

#### KINDLING-010: Status dashboard integration

- **Intent:** Show memory stats in `anvil status` output
- **Expected Outcome:** Status displays capsule count, query availability, DB size
- **Scope:** `cli/src/commands/`
- **Non-scope:** Detailed visualisation
- **Files:**
  - `cli/src/commands/status.ts` (add memory section)
- **Dependencies:** KINDLING-001, KINDLING-007
- **Validation:** `anvil status` shows memory section when enabled
- **Confidence:** high
- **Status:** Draft

#### KINDLING-011: Performance benchmarking

- **Intent:** Validate observation capture overhead meets acceptance criteria
- **Expected Outcome:** Benchmark suite showing < 50ms capture overhead
- **Scope:** `packages/kindling-integration/`
- **Non-scope:** Optimisation (only measurement)
- **Files:**
  - `packages/kindling-integration/benchmarks/capture-overhead.bench.ts`
- **Dependencies:** KINDLING-003
- **Validation:** `pnpm bench --filter kindling-integration`
- **Confidence:** medium
- **Status:** Draft

#### KINDLING-012: Documentation and examples

- **Intent:** Document memory configuration, CLI usage, and query patterns
- **Expected Outcome:** User guide section and example queries
- **Scope:** `docs/`, `packages/kindling-integration/README.md`
- **Non-scope:** Video tutorials
- **Files:**
  - `docs/MEMORY_GUIDE.md`
  - `packages/kindling-integration/README.md`
  - `packages/kindling-integration/examples/`
- **Dependencies:** KINDLING-007
- **Validation:** Manual review of documentation completeness
- **Confidence:** high
- **Status:** Draft

## Decisions

**D-KINDLING-001:** Separate package, not core integration

- **Rationale:** Keeps Kindling as optional dependency, reduces blast radius
- **Alternatives:** Integrate directly into @anvil/core
- **Trade-offs:** Additional package overhead, but better separation of concerns

**D-KINDLING-002:** Opt-in by default

- **Rationale:** Privacy-first, users must explicitly enable memory capture
- **Alternatives:** Opt-out with .gitignore
- **Trade-offs:** Lower adoption, but respects user agency

**D-KINDLING-003:** Session-based capsules, not task-based

- **Rationale:** Aligns with CLI execution model, simpler lifecycle
- **Alternatives:** Task-based (requires APS plan parsing)
- **Trade-offs:** Coarser grouping, but works planless-first

**D-KINDLING-004:** SQLite local storage, no remote sync

- **Rationale:** Privacy-first, no external dependencies, fast queries
- **Alternatives:** Remote backend for team sharing
- **Trade-offs:** No team collaboration, but preserves privacy

**D-KINDLING-005:** Async observation capture only

- **Rationale:** Must not add latency to gate execution
- **Alternatives:** Synchronous with timeout
- **Trade-offs:** Potential observation loss on crash, but acceptable

## Notes

**Package structure:**

```
packages/kindling-integration/
├── src/
│   ├── memory-service.ts       # High-level API
│   ├── observation-mapper.ts   # Type converters
│   ├── gate-observer.ts        # GateRunner hook
│   ├── capsule-manager.ts      # Lifecycle orchestration
│   ├── session-detector.ts     # Session boundary detection
│   ├── query-service.ts        # Retrieval API
│   ├── config-schema.ts        # Configuration
│   ├── privacy-filter.ts       # Sensitive data filtering
│   └── retention-policy.ts     # Auto-pruning
├── benchmarks/
│   └── capture-overhead.bench.ts
├── examples/
│   └── query-patterns.md
└── README.md
```

**Example usage:**

```typescript
// Automatic capture during gate execution
const result = await gateRunner.run(config, options);
// Observations automatically captured if memory.enabled = true

// Query past observations
const service = new MemoryService(config);
const results = await service.query('architecture violations in src/api', {
  scope: { repoId: 'anvil-001' },
  limit: 10,
});

// Results include:
// - Ranked observations with explanations
// - Provenance (session ID, check name, timestamp)
// - Summary context from capsules
```

**CLI examples:**

```bash
# Enable memory in project
anvil init --with-memory

# Query historical observations
anvil memory query "coverage failures"

# List recent sessions
anvil memory sessions --last 10

# Show session details
anvil memory show abc123

# Memory statistics
anvil memory stats

# Prune old data
anvil memory prune --older-than 90d
```

**Integration with drift reporting:**

Kindling memory complements drift snapshots:

- **Snapshots** capture point-in-time architecture state (coarse-grained)
- **Memory** captures observation stream with provenance (fine-grained)
- Together enable: "Show me all times this boundary was violated in the last month"

**Future enhancements:**

- Pattern detection: "This warning appears every Monday" (recurrence analysis)
- Context injection: Pre-populate AI tools with relevant historical context
- Team memory: Opt-in sharing of anonymised observation patterns
- Trend visualisation: TUI dashboard showing capsule timelines
