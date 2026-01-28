# Edda Component Dependency Map

**Version:** 1.0.0 **Purpose:** Visual and structural dependency mapping for
implementation planning **Related:**
`/docs/architecture/edda-system-architecture.md`,
`/plans/edda-phase-breakdown.md`

---

## Component Hierarchy

```
┌─────────────────────────────────────────────────────────────────┐
│                        Edda System                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │                  User Interfaces                        │    │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐            │    │
│  │  │   CLI    │  │ REST API │  │   Web    │  (Phase 6) │    │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘            │    │
│  └───────┼─────────────┼─────────────┼──────────────────┘    │
│          └─────────────┼─────────────┘                         │
│                        │                                        │
│  ┌────────────────────▼───────────────────────────────┐       │
│  │              Application Layer                      │       │
│  │                                                      │       │
│  │  ┌──────────────┐  ┌──────────────┐               │       │
│  │  │  Promotion   │  │ Enforcement  │  (Phases 1,4) │       │
│  │  │  Pipeline    │  │   Hooks      │               │       │
│  │  └──────┬───────┘  └──────┬───────┘               │       │
│  │         │                  │                        │       │
│  │  ┌──────▼──────────────────▼───────┐              │       │
│  │  │    Lifecycle Management    │  (Phase 5)    │   │       │
│  │  └────────────────┬───────────┘              │   │       │
│  └───────────────────┼──────────────────────────┘   │       │
│                      │                              │       │
│  ┌──────────────────▼────────────────────────────┐  │       │
│  │           Service Layer                        │  │       │
│  │                                                 │  │       │
│  │  ┌─────────────┐  ┌─────────────┐            │  │       │
│  │  │  Authority  │  │    Query    │  (Phases 2,3)│ │       │
│  │  │  & Trust    │  │  & Retrieval │            │  │       │
│  │  └──────┬──────┘  └──────┬──────┘            │  │       │
│  │         └─────────────────┼────────────────────  │       │
│  │                           │                        │       │
│  │  ┌────────────────────────▼──────────────────┐   │       │
│  │  │         Memory Manager (Core)             │   │       │
│  │  └────────────────────┬──────────────────────┘   │       │
│  └───────────────────────┼──────────────────────────┘       │
│                          │                                    │
│  ┌──────────────────────▼────────────────────────────┐      │
│  │             Storage Layer (Phase 0)               │      │
│  │                                                    │      │
│  │  ┌──────────────┐  ┌──────────────┐             │      │
│  │  │ Git Storage  │  │ SQLite Index │             │      │
│  │  │   (YAML)     │  │   (FTS5)     │             │      │
│  │  └──────────────┘  └──────────────┘             │      │
│  └────────────────────────────────────────────────────┘     │
│                                                              │
└──────────────────────────────────────────────────────────────┘

External Dependencies:
┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
│  Kindling│  │  Ember   │  │  Anvil   │  │ Identity │
│   Port   │  │   Port   │  │  Gates   │  │ Provider │
└──────────┘  └──────────┘  └──────────┘  └──────────┘
```

---

## Core Components

### 1. Storage Layer (Phase 0)

**Components:**

- Git Storage Adapter
- SQLite Index
- Storage Port Implementation

**Dependencies:**

- Git CLI
- SQLite library
- File system access

**Dependents:**

- Memory Manager
- All higher layers

**Interfaces:**

```typescript
interface IStorageAdapter {
  write(path: string, content: string): Promise<void>;
  read(path: string): Promise<string>;
  delete(path: string): Promise<void>;
  commit(message: string): Promise<string>;
}

interface IIndexAdapter {
  index(memory: MemoryObjectExtended): Promise<void>;
  query(query: EddaQuery): Promise<MemoryId[]>;
  search(text: string): Promise<MemoryId[]>;
}
```

---

### 2. Memory Manager (Phase 0)

**Components:**

- Memory Object Model
- CRUD Operations
- Validation Logic
- Version Management

**Dependencies:**

- Storage Layer
- Zod (validation)

**Dependents:**

- Query Service
- Promotion Pipeline
- Lifecycle Manager
- All higher layers

**Interfaces:**

```typescript
interface IMemoryManager {
  create(input: MemoryObjectInput): Promise<MemoryObjectExtended>;
  get(id: MemoryId): Promise<MemoryObjectExtended>;
  update(id: MemoryId, patch: MemoryObjectPatch): Promise<MemoryObjectExtended>;
  delete(id: MemoryId): Promise<void>;
  list(filter?: MemoryFilter): Promise<MemoryObjectExtended[]>;
}
```

---

### 3. Authority & Trust Service (Phase 2)

**Components:**

- Principal Registry
- Role Manager
- Permission Checker
- Agent Trust Tracker
- Audit Logger

**Dependencies:**

- Storage Layer (for principals, roles, audit logs)
- Memory Manager
- Identity Provider (external)

**Dependents:**

- All write operations (permission checks)
- Promotion Pipeline (trust scoring)
- Enforcement Hooks (authority checks)

**Interfaces:**

```typescript
interface IAuthorityService {
  // Principals
  resolvePrincipal(identifier: string): Promise<Principal>;
  registerPrincipal(principal: Principal): Promise<void>;

  // Roles
  assignRole(principal: Principal, role: Role): Promise<void>;
  revokeRole(principal: Principal, roleId: string): Promise<void>;

  // Permissions
  hasPermission(principal: Principal, permission: Permission): boolean;
  canAccessMemory(principal: Principal, memory: MemoryObjectExtended): boolean;

  // Trust
  getTrustProfile(agentId: string): Promise<AgentTrustProfile>;
  updateTrustProfile(agentId: string, event: TrustEvent): Promise<void>;

  // Audit
  audit(entry: AuditEntry): Promise<void>;
  queryAudit(query: AuditQuery): Promise<AuditEntry[]>;
}
```

---

### 4. Query & Retrieval Service (Phase 3)

**Components:**

- Query Builder
- Full-Text Search
- Semantic Search (optional)
- Conflict Detector
- Provenance Tracer

**Dependencies:**

- Memory Manager
- SQLite Index (for FTS)
- Embedding Service (optional, for semantic search)
- Ember Port (for provenance)
- Kindling Port (for provenance)

**Dependents:**

- CLI commands
- REST API
- Enforcement Hooks (memory matching)
- Promotion Pipeline (conflict detection)

**Interfaces:**

```typescript
interface IQueryService {
  // Basic queries
  query(query: EddaQuery): Promise<EddaQueryResult>;
  search(text: string): Promise<SemanticResult>;

  // Conflict detection
  detectConflicts(query: ConflictQuery): Promise<ConflictResult>;

  // Temporal
  queryTemporal(query: TemporalQuery): Promise<TemporalResult>;

  // Provenance
  traceProvenance(query: ProvenanceQuery): Promise<ProvenanceResult>;
}
```

---

### 5. Promotion Pipeline (Phase 1)

**Components:**

- Promotion Request Manager
- Type Mapper (Ember → Edda)
- Diff Generator
- Review Workflow
- Rejection Tracker

**Dependencies:**

- Memory Manager
- Authority Service (permissions)
- Query Service (conflict detection)
- Ember Port (proposals)
- Kindling Port (provenance validation)

**Dependents:**

- CLI review commands
- Agent proposal submissions

**Interfaces:**

```typescript
interface IPromotionService {
  // Requests
  createPromotionRequest(
    proposalId: ProposalId,
    requestedBy: Principal
  ): Promise<PromotionRequest>;

  // Review
  startReview(
    requestId: PromotionRequestId,
    reviewer: Principal
  ): Promise<void>;
  submitReview(
    requestId: PromotionRequestId,
    review: PromotionReview
  ): Promise<PromotionResult>;

  // Queries
  listPendingReviews(): Promise<PromotionRequest[]>;
  getPromotionDiff(requestId: PromotionRequestId): Promise<PromotionDiff>;

  // Rejection
  recordRejection(
    proposalId: ProposalId,
    rejection: RejectionRecord
  ): Promise<void>;
}
```

---

### 6. Enforcement Hooks (Phase 4)

**Components:**

- Hook Registry
- Hook Execution Engine
- Trigger Evaluator
- Memory Matcher
- Override Manager

**Dependencies:**

- Memory Manager
- Query Service (memory matching)
- Authority Service (override permissions)
- Anvil Gate System (integration)

**Dependents:**

- Anvil execution pipeline

**Interfaces:**

```typescript
interface IEnforcementService {
  // Hook management
  registerHook(hook: EnforcementHook): Promise<void>;
  updateHook(hookId: HookId, updates: Partial<EnforcementHook>): Promise<void>;
  deleteHook(hookId: HookId): Promise<void>;
  listHooks(filter?: HookFilter): Promise<EnforcementHook[]>;

  // Execution
  executeHooks(
    event: HookEvent,
    context: ExecutionContext
  ): Promise<HookExecutionResult>;

  // Override
  requestOverride(request: OverrideRequest): Promise<OverrideDecision>;

  // Guidance
  getGuidance(context: PlanningContext): Promise<GuidanceResponse>;
}
```

---

### 7. Lifecycle Manager (Phase 5)

**Components:**

- Deprecation Workflow
- Review Scheduler
- Supersession Handler
- Staleness Detector
- Forgetting Engine

**Dependencies:**

- Memory Manager
- Authority Service (permissions)
- Query Service (impact analysis)
- Enforcement Hooks (hook migration)

**Dependents:**

- CLI lifecycle commands
- Automated review triggers

**Interfaces:**

```typescript
interface ILifecycleService {
  // Deprecation
  proposeDeprecation(request: DeprecationRequest): Promise<void>;
  retireMemory(id: MemoryId, reason: string, by: Principal): Promise<void>;

  // Supersession
  supersedeMemory(request: SupersessionRequest): Promise<SupersessionResult>;

  // Review
  scheduleReview(id: MemoryId, policy: ReviewPolicy): Promise<void>;
  getReviewsDue(): Promise<ReviewSchedule[]>;

  // Staleness
  detectStaleness(): Promise<StalenessFactor[]>;
  getRetirementCandidates(): Promise<ForgettingReport>;
}
```

---

## Dependency Matrix

| Component              | Depends On                                       | Used By                               |
| ---------------------- | ------------------------------------------------ | ------------------------------------- |
| **Git Storage**        | Git CLI, FS                                      | Memory Manager                        |
| **SQLite Index**       | SQLite library                                   | Memory Manager, Query Service         |
| **Memory Manager**     | Storage Layer                                    | All higher layers                     |
| **Authority Service**  | Storage, Memory Manager, Identity Provider       | All write ops, Promotion, Enforcement |
| **Query Service**      | Memory Manager, SQLite, Ember/Kindling Ports     | CLI, API, Enforcement, Promotion      |
| **Promotion Pipeline** | Memory Manager, Authority, Query, Ember/Kindling | CLI, Agents                           |
| **Enforcement Hooks**  | Memory Manager, Query, Authority, Anvil          | Anvil execution                       |
| **Lifecycle Manager**  | Memory Manager, Authority, Query, Enforcement    | CLI, Scheduled jobs                   |
| **CLI**                | All services                                     | Users                                 |
| **REST API**           | All services                                     | External clients                      |

---

## Critical Paths

### Minimum Viable Edda (MVP)

```
Phase 0: Storage + Memory Manager
    ↓
Phase 1: Promotion Pipeline
    ↓
Phase 2: Authority Service
    ↓
Phase 4: Enforcement Hooks
    ↓
Phase 6: Export/Import
```

**Duration:** ~12 weeks **Deliverable:** Working Edda with promotion,
enforcement, and basic governance

### Full Feature Set

```
Phase 0: Storage + Memory Manager
    ↓
    ├─→ Phase 1: Promotion ──→ Phase 5: Lifecycle
    │                               ↓
    ├─→ Phase 2: Authority ─────────┤
    │                               ↓
    └─→ Phase 3: Query ──→ Phase 4: Enforcement
                               ↓
                          Phase 6: Interop
                               ↓
                          Phase 7: Meta (optional)
```

**Duration:** ~19 weeks (excluding Phase 7) **Deliverable:** Complete Edda
system with all planned features

---

## External Integration Points

### 1. Ember Port

**Purpose:** Source of promotion proposals

**Contract:**

```typescript
interface IEmberPort {
  getProposal(id: ProposalId): Promise<CandidateProposal>;
  getActiveProposals(): Promise<CandidateProposal[]>;
  markPromoted(id: ProposalId, memoryId: MemoryId): Promise<void>;
  markDismissed(id: ProposalId, reason: string): Promise<void>;
}
```

**Integration Points:**

- Promotion Pipeline: fetch proposals
- Promotion Pipeline: update proposal status
- Trust Service: track agent performance

---

### 2. Kindling Port

**Purpose:** Provenance validation

**Contract:**

```typescript
interface IKindlingPort {
  getObservation(id: ObservationId): Promise<Observation>;
  queryObservations(query: ObservationQuery): Promise<Observation[]>;
}
```

**Integration Points:**

- Promotion Pipeline: validate provenance chain
- Provenance Tracing: fetch source observations

---

### 3. Anvil Gate System

**Purpose:** Enforcement integration

**Contract:**

```typescript
interface IAnvilGateSystem {
  registerPreExecutionHook(hook: PreExecutionHook): void;
  registerFileChangeHook(hook: FileChangeHook): void;
  registerPlanningHook(hook: PlanningHook): void;
}

type PreExecutionHook = (
  action: Action,
  context: ActionContext
) => Promise<HookExecutionResult>;
```

**Integration Points:**

- Enforcement Hooks: pre-action checks
- Enforcement Hooks: file change checks
- Enforcement Hooks: planning guidance

---

### 4. Identity Provider

**Purpose:** Authentication and principal resolution

**Contract:**

```typescript
interface IIdentityProvider {
  authenticate(token: string): Promise<Principal>;
  resolvePrincipal(identifier: string): Promise<PrincipalInfo>;
  listTeamMembers(teamId: string): Promise<Principal[]>;
}
```

**Integration Points:**

- Authority Service: principal resolution
- Authority Service: team membership
- API: authentication

---

## Data Flow Diagrams

### Promotion Flow

```
Ember (Proposal)
    ↓
Promotion Request Created
    ↓
Human Review (CLI)
    ├→ Approve
    │    ↓
    │  Type Mapping
    │    ↓
    │  Conflict Check (Query Service)
    │    ↓
    │  Create Memory (Memory Manager)
    │    ↓
    │  Update Ember Status
    │    ↓
    │  Update Agent Trust (Authority Service)
    │    ↓
    │  Audit Log
    │    ↓
    │  DONE
    │
    └→ Reject
         ↓
       Record Rejection
         ↓
       Generate Feedback
         ↓
       Update Agent Trust
         ↓
       DONE
```

### Enforcement Flow

```
Anvil Action Initiated
    ↓
Extract Context
    ↓
Find Applicable Hooks (Enforcement Service)
    ↓
For Each Hook:
    ├→ Evaluate Trigger
    │    ↓
    │  Find Matching Memories (Query Service)
    │    ↓
    │  Check Enforcement Mode
    │    ↓
    │  Check Authority (if override)
    │    ↓
    │  Generate Result
    │
    └→ Aggregate Results
         ↓
    ┌────┴────┐
    │ Block?  │
    └────┬────┘
         │
     ┌───┴───┐
     │ Yes   │ No
     ↓       ↓
   BLOCK   ALLOW
           (with warnings)
```

### Query Flow

```
Query Request
    ↓
Build Query (Query Service)
    ↓
Check Permissions (Authority Service)
    ↓
┌───┴────┐
│ FTS?   │ Semantic?
└───┬────┘
    │
┌───┴────┐
│ SQLite │ Embedding Service
│ FTS5   │
└───┬────┘
    │
Retrieve Memory IDs
    ↓
Load Full Memories (Memory Manager)
    ↓
Filter by Visibility (Authority Service)
    ↓
Return Results
```

---

## Build Dependencies

### Package Structure

```
packages/
├── edda-stack/              # Contracts (existing)
│   └── src/contracts/
│       ├── edda-memory.ts
│       ├── edda-extended.ts  # NEW
│       └── ports/
│           └── edda.port.ts
│
├── edda-core/               # NEW: Phase 0
│   ├── src/
│   │   ├── storage/
│   │   │   ├── git-adapter.ts
│   │   │   └── sqlite-index.ts
│   │   ├── memory/
│   │   │   ├── memory-manager.ts
│   │   │   └── validation.ts
│   │   └── index.ts
│   └── package.json
│
├── edda-promotion/          # NEW: Phase 1
│   ├── src/
│   │   ├── promotion-service.ts
│   │   ├── type-mapper.ts
│   │   ├── diff-generator.ts
│   │   └── index.ts
│   └── package.json
│
├── edda-authority/          # NEW: Phase 2
│   ├── src/
│   │   ├── authority-service.ts
│   │   ├── trust-tracker.ts
│   │   ├── audit-logger.ts
│   │   └── index.ts
│   └── package.json
│
├── edda-query/              # NEW: Phase 3
│   ├── src/
│   │   ├── query-service.ts
│   │   ├── conflict-detector.ts
│   │   ├── provenance-tracer.ts
│   │   └── index.ts
│   └── package.json
│
├── edda-enforcement/        # NEW: Phase 4
│   ├── src/
│   │   ├── hook-engine.ts
│   │   ├── anvil-integration.ts
│   │   └── index.ts
│   └── package.json
│
├── edda-lifecycle/          # NEW: Phase 5
│   ├── src/
│   │   ├── lifecycle-service.ts
│   │   ├── staleness-detector.ts
│   │   └── index.ts
│   └── package.json
│
└── edda-api/                # NEW: Phase 6
    ├── src/
    │   ├── rest/
    │   │   └── server.ts
    │   ├── export/
    │   │   └── exporter.ts
    │   └── index.ts
    └── package.json
```

### Dependency Graph

```
edda-core
    ↓
    ├─→ edda-promotion ──→ edda-lifecycle
    │                           ↑
    ├─→ edda-authority ─────────┤
    │                           ↑
    └─→ edda-query ──────→ edda-enforcement
                                ↑
                                │
                           edda-api
```

---

## Testing Dependencies

### Test Infrastructure

```
packages/
└── edda-testing/            # Shared test utilities
    ├── src/
    │   ├── fixtures/
    │   │   ├── memories.ts
    │   │   ├── principals.ts
    │   │   ├── hooks.ts
    │   │   └── contexts.ts
    │   ├── mocks/
    │   │   ├── storage-mock.ts
    │   │   ├── identity-mock.ts
    │   │   └── anvil-mock.ts
    │   └── helpers/
    │       ├── test-db.ts
    │       └── git-repo.ts
    └── package.json
```

**Usage:**

```typescript
import { createTestMemory, createTestPrincipal } from '@anvil/edda-testing';
import { MockStorageAdapter } from '@anvil/edda-testing/mocks';

test('memory creation', async () => {
  const storage = new MockStorageAdapter();
  const manager = new MemoryManager(storage);

  const memory = await manager.create(createTestMemory());

  expect(memory.id).toMatch(/^EDDA-M-/);
});
```

---

## Deployment Dependencies

### Runtime Dependencies

**Required:**

- Node.js ≥18
- Git CLI
- SQLite3

**Optional:**

- Embedding service (Ollama or OpenAI API) for semantic search
- Redis (for distributed caching, future)
- PostgreSQL (alternative to SQLite for large deployments, future)

### Configuration Dependencies

**Required:**

- `.edda/` directory (git repository)
- Identity provider configuration
- Anvil integration enabled

**Optional:**

- Embedding service credentials
- Custom role definitions
- Custom enforcement hooks

---

## Parallel Development Opportunities

### Can Be Developed in Parallel

1. **Phase 0 + Contracts**
   - Storage adapters (one dev)
   - Memory validation (another dev)

2. **After Phase 0:**
   - Phase 1 (Promotion) + Phase 2 (Authority) - independent
   - Phase 3 (Query) - depends only on Phase 0

3. **After Phases 1, 2, 3:**
   - Phase 4 (Enforcement) + Phase 5 (Lifecycle) - mostly independent

### Must Be Sequential

1. Phase 0 must complete before others
2. Phase 6 (API) should wait for core features (1-5)
3. Phase 7 (Meta) requires full system

---

## Risk Dependencies

### High-Risk Dependencies

1. **Anvil Gate System** (Phase 4)
   - Risk: Requires Anvil core changes
   - Mitigation: Early coordination with Anvil team

2. **Identity Provider** (Phase 2)
   - Risk: Integration complexity varies by provider
   - Mitigation: Start with GitHub OAuth (simpler)

3. **Embedding Service** (Phase 3)
   - Risk: External service dependency, costs
   - Mitigation: Make optional, provide local alternative (Ollama)

### Medium-Risk Dependencies

1. **Git Performance** (Phase 0)
   - Risk: Large repositories may have slow commits
   - Mitigation: Batch commits, use git-fast-import for bulk

2. **SQLite Scalability** (Phase 0)
   - Risk: May not scale to 100k+ memories
   - Mitigation: Design with alternative DB in mind (PostgreSQL)

---

## Success Criteria by Component

| Component      | Success Metric      | Target           |
| -------------- | ------------------- | ---------------- |
| Storage        | Write latency       | <100ms p95       |
| Memory Manager | CRUD latency        | <50ms p95        |
| Authority      | Permission check    | <10ms p95        |
| Query          | Search latency      | <200ms p95       |
| Promotion      | Review UX           | <5min per review |
| Enforcement    | Hook overhead       | <50ms p95        |
| Lifecycle      | Staleness detection | >90% accuracy    |
| API            | Response time       | <500ms p95       |

---

**Next Steps:**

1. Review this dependency map with team
2. Identify resource allocation per component
3. Establish component ownership
4. Create detailed APS for each phase

---

**Document Owner:** Architecture Team **Last Updated:** 2026-01-19
