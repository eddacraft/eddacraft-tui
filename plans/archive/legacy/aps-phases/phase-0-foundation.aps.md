# APS: Edda Phase 0 - Foundation

**Phase:** 0 (Foundation)
**Duration:** 2 weeks (10 working days)
**Status:** Planning
**Owner:** [TBD]
**Dependencies:** None (foundational phase)

---

## Overview

Phase 0 establishes the core storage and memory management infrastructure for Edda. This is the foundational layer upon which all other phases depend. The deliverables are a working Git-backed storage system, memory object CRUD operations, and a SQLite index for fast queries.

---

## Goals

### Primary Goal
Build a reliable, tested storage foundation that can handle:
- Memory object persistence (Git-backed YAML)
- Fast queries (SQLite FTS5 index)
- Version management
- Validation and type safety

### Success Criteria
- ✅ Can create, read, update, delete memories
- ✅ Git commits show clean history per operation
- ✅ Query performance <100ms for typical queries
- ✅ All 6 memory types supported and validated
- ✅ 90%+ test coverage
- ✅ Zero data loss or corruption in tests

---

## Architecture Context

```
Phase 0 Deliverables:

┌─────────────────────────────────────────┐
│     Edda Core (NEW)                     │
├─────────────────────────────────────────┤
│                                          │
│  ┌──────────────────────────────────┐  │
│  │   Memory Manager                  │  │
│  │   - CRUD operations               │  │
│  │   - Validation (Zod)              │  │
│  │   - ID generation                 │  │
│  └──────────┬───────────────────────┘  │
│             │                            │
│    ┌────────┴────────┐                  │
│    │                 │                  │
│ ┌──▼─────────┐  ┌───▼────────┐        │
│ │ Git Storage│  │SQLite Index│        │
│ │  (YAML)    │  │  (FTS5)    │        │
│ └────────────┘  └────────────┘        │
│                                          │
└─────────────────────────────────────────┘
```

---

## Epics & Tasks

### Epic 1: Project Setup (1 day)

**Goal:** Create package structure and tooling

#### Task 1.1: Create `edda-core` Package
**Estimate:** 2 hours
**Owner:** [TBD]

**Description:**
Create new package in monorepo with TypeScript configuration, build setup, and dependencies.

**Acceptance Criteria:**
- [ ] Package created at `packages/edda-core/`
- [ ] `package.json` configured with correct dependencies
- [ ] TypeScript configuration (`tsconfig.json`)
- [ ] Build script produces `dist/` output
- [ ] Can import `@anvil/edda-stack` contracts
- [ ] Vitest configured for testing
- [ ] ESLint and Prettier configured

**Dependencies:**
- Zod (validation)
- YAML (js-yaml)
- better-sqlite3
- ulid (ID generation)

**Files Created:**
```
packages/edda-core/
├── package.json
├── tsconfig.json
├── vitest.config.ts
├── .eslintrc.json
├── src/
│   └── index.ts
└── tests/
    └── setup.ts
```

---

#### Task 1.2: Set Up Test Infrastructure
**Estimate:** 2 hours
**Owner:** [TBD]

**Description:**
Configure test environment with fixtures, mocks, and utilities.

**Acceptance Criteria:**
- [ ] Vitest runs successfully
- [ ] Test fixtures for all 6 memory types
- [ ] Temporary directory setup/cleanup utilities
- [ ] Git test repository helper
- [ ] SQLite in-memory database helper
- [ ] Code coverage reporting configured

**Files Created:**
```
packages/edda-core/tests/
├── setup.ts
├── fixtures/
│   ├── memories.ts
│   └── test-data.ts
├── helpers/
│   ├── git-repo.ts
│   ├── temp-dir.ts
│   └── test-db.ts
└── unit/
└── integration/
```

---

#### Task 1.3: Define Schema with Zod
**Estimate:** 2 hours
**Owner:** [TBD]

**Description:**
Create Zod schemas for validation, extending the base contracts from `edda-stack`.

**Acceptance Criteria:**
- [ ] Zod schema for `MemoryObjectExtended`
- [ ] Type-specific metadata schemas (Decision, Pattern, Warning, etc.)
- [ ] Scope, Authority, Enforcement schemas
- [ ] Schema validation tests (valid + invalid inputs)
- [ ] Error messages are clear and actionable

**Files Created:**
```
packages/edda-core/src/
├── schemas/
│   ├── memory.schema.ts
│   ├── authority.schema.ts
│   ├── enforcement.schema.ts
│   └── metadata.schema.ts
└── validation/
    └── validator.ts
```

**Example:**
```typescript
import { z } from 'zod'
import type { MemoryType } from '@anvil/edda-stack'

export const MemoryObjectSchema = z.object({
  id: z.string().regex(/^EDDA-M-[a-z]+-[0-9a-z]+$/),
  type: z.enum(['decision', 'pattern', 'warning', 'constraint', 'doctrine', 'lesson']),
  status: z.enum(['active', 'superseded', 'retired']),
  statement: z.string().min(1).max(2000),
  // ... rest of schema
})
```

---

### Epic 2: Git Storage Layer (3 days)

**Goal:** Implement Git-backed YAML storage

#### Task 2.1: Git Storage Adapter - Core
**Estimate:** 1 day
**Owner:** [TBD]

**Description:**
Implement Git storage adapter for writing/reading YAML files.

**Acceptance Criteria:**
- [ ] `GitStorageAdapter` class implements `IStorageAdapter` interface
- [ ] Can write YAML file to `.edda/memories/{type}/{id}.yaml`
- [ ] Can read YAML file and deserialize
- [ ] Can delete YAML file
- [ ] Creates parent directories automatically
- [ ] Handles file not found errors gracefully
- [ ] Unit tests for all operations

**Interface:**
```typescript
interface IStorageAdapter {
  write(memory: MemoryObjectExtended): Promise<void>
  read(id: MemoryId): Promise<MemoryObjectExtended>
  delete(id: MemoryId): Promise<void>
  exists(id: MemoryId): Promise<boolean>
  list(filter?: { type?: MemoryType }): Promise<MemoryId[]>
}
```

**Files Created:**
```
packages/edda-core/src/storage/
├── git-storage-adapter.ts
├── storage-adapter.interface.ts
└── yaml-serializer.ts
```

**Test Coverage:**
- Write memory → verify file exists
- Read memory → verify content correct
- Delete memory → verify file removed
- Non-existent memory → throws NotFoundError
- Invalid YAML → throws ValidationError

---

#### Task 2.2: Git Operations Integration
**Estimate:** 1 day
**Owner:** [TBD]

**Description:**
Add Git commit/push operations for atomicity and audit trail.

**Acceptance Criteria:**
- [ ] Each write operation creates a Git commit
- [ ] Commit message includes memory ID and operation type
- [ ] Can initialize `.edda` directory as Git repo
- [ ] Can check if repo is initialized
- [ ] Handles Git errors gracefully
- [ ] Integration tests with real Git operations
- [ ] Performance: commit operation <100ms

**Commit Message Format:**
```
edda: create memory EDDA-M-decision-01h2...

Type: decision
Statement: Use TypeScript for backend
Promoted by: user:alice
```

**Files Created:**
```
packages/edda-core/src/storage/
├── git-operations.ts
└── commit-formatter.ts
```

**Test Coverage:**
- Write → commit created
- Update → new commit created
- Delete → commit shows deletion
- Multiple operations → clean history
- Git error → graceful failure

---

#### Task 2.3: File Locking & Concurrency
**Estimate:** 0.5 day
**Owner:** [TBD]

**Description:**
Implement file locking to prevent concurrent write conflicts.

**Acceptance Criteria:**
- [ ] Lock acquired before write operation
- [ ] Lock released after write (even on error)
- [ ] Concurrent writes are serialized
- [ ] Lock timeout after 5 seconds
- [ ] Unit tests for concurrent scenarios
- [ ] Performance: lock overhead <10ms

**Files Created:**
```
packages/edda-core/src/storage/
└── file-lock.ts
```

**Test Coverage:**
- Two concurrent writes → serialized
- Lock timeout → throws error
- Lock released on error → next write succeeds

---

#### Task 2.4: YAML Serialization & Formatting
**Estimate:** 0.5 day
**Owner:** [TBD]

**Description:**
Create clean, readable YAML output with consistent formatting.

**Acceptance Criteria:**
- [ ] YAML output is human-readable
- [ ] Multi-line strings use `|` syntax
- [ ] Arrays formatted consistently
- [ ] No unnecessary quotes
- [ ] Preserves comment placeholders
- [ ] Round-trip test: memory → YAML → memory (identical)

**Files Created:**
```
packages/edda-core/src/storage/
├── yaml-serializer.ts
└── yaml-formatter.ts
```

**Example Output:**
```yaml
id: EDDA-M-decision-01h2...
type: decision
status: active

statement: |
  All database schema changes must use Prisma migrations.
  Direct schema modifications via SQL are prohibited.

context:
  when: For all database schema modifications
  why: |
    Direct schema changes caused 3 incidents.
    Migrations provide audit trail and rollback.
  tags:
    - database
    - schema
    - migrations
```

---

### Epic 3: SQLite Index Layer (2 days)

**Goal:** Implement fast query index

#### Task 3.1: SQLite Schema Design
**Estimate:** 0.5 day
**Owner:** [TBD]

**Description:**
Design SQLite schema for indexing memories.

**Acceptance Criteria:**
- [ ] Table schema defined for memories
- [ ] Indexes on: type, status, created_at, updated_at
- [ ] FTS5 table for full-text search
- [ ] Migration script for schema creation
- [ ] Schema documentation

**Schema:**
```sql
CREATE TABLE memories (
  id TEXT PRIMARY KEY,
  type TEXT NOT NULL,
  status TEXT NOT NULL,
  statement TEXT NOT NULL,
  tags TEXT, -- JSON array
  scope_type TEXT,
  scope_identifier TEXT,
  confidence TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,

  -- For fast queries
  CHECK (type IN ('decision', 'pattern', 'warning', 'constraint', 'doctrine', 'lesson')),
  CHECK (status IN ('active', 'superseded', 'retired'))
);

CREATE INDEX idx_type ON memories(type);
CREATE INDEX idx_status ON memories(status);
CREATE INDEX idx_created_at ON memories(created_at DESC);
CREATE INDEX idx_type_status ON memories(type, status);

-- Full-text search
CREATE VIRTUAL TABLE memories_fts USING fts5(
  id UNINDEXED,
  statement,
  context_why,
  context_when,
  tags,
  content='memories'
);
```

**Files Created:**
```
packages/edda-core/src/index/
├── schema.sql
└── migrations.ts
```

---

#### Task 3.2: Index Adapter Implementation
**Estimate:** 1 day
**Owner:** [TBD]

**Description:**
Implement SQLite adapter for indexing and querying.

**Acceptance Criteria:**
- [ ] `SqliteIndexAdapter` class implements `IIndexAdapter` interface
- [ ] Can index memory on creation
- [ ] Can update index on memory update
- [ ] Can remove from index on deletion
- [ ] Can query by filters (type, status, tags)
- [ ] Can perform full-text search
- [ ] Can paginate results
- [ ] Unit tests for all operations
- [ ] Performance: typical query <50ms

**Interface:**
```typescript
interface IIndexAdapter {
  index(memory: MemoryObjectExtended): Promise<void>
  remove(id: MemoryId): Promise<void>
  query(query: EddaQuery): Promise<MemoryId[]>
  search(text: string, limit?: number): Promise<MemoryId[]>
  count(filter?: Partial<EddaQuery>): Promise<number>
  rebuild(memories: MemoryObjectExtended[]): Promise<void>
}
```

**Files Created:**
```
packages/edda-core/src/index/
├── sqlite-index-adapter.ts
├── index-adapter.interface.ts
└── query-builder.ts
```

**Test Coverage:**
- Index memory → appears in queries
- Update memory → index updated
- Delete memory → removed from index
- Query by type → returns correct IDs
- Full-text search → returns relevant IDs
- Pagination → correct offsets

---

#### Task 3.3: Index Synchronization
**Estimate:** 0.5 day
**Owner:** [TBD]

**Description:**
Ensure index stays in sync with Git storage.

**Acceptance Criteria:**
- [ ] Index automatically updated after write
- [ ] Rebuild index from Git storage (idempotent)
- [ ] Detect index drift (count mismatch)
- [ ] Recovery mechanism for corruption
- [ ] Unit tests for sync scenarios

**Files Created:**
```
packages/edda-core/src/index/
└── index-sync.ts
```

**Test Coverage:**
- Write memory → index updated
- Rebuild index → matches Git storage
- Corrupted index → rebuild succeeds
- Index count → matches file count

---

### Epic 4: Memory Manager (2 days)

**Goal:** Implement CRUD operations with validation

#### Task 4.1: Memory Manager - Create
**Estimate:** 0.5 day
**Owner:** [TBD]

**Description:**
Implement memory creation with validation and ID generation.

**Acceptance Criteria:**
- [ ] Validates input with Zod schema
- [ ] Generates unique ID (EDDA-M-{type}-{ulid})
- [ ] Sets timestamps (created_at, updated_at)
- [ ] Writes to Git storage
- [ ] Indexes in SQLite
- [ ] Returns created memory
- [ ] Unit tests for all scenarios
- [ ] Error handling for validation failures

**Interface:**
```typescript
class MemoryManager {
  async create(input: MemoryObjectInput): Promise<MemoryObjectExtended>
}
```

**Files Created:**
```
packages/edda-core/src/memory/
├── memory-manager.ts
├── id-generator.ts
└── timestamp-utils.ts
```

**Test Coverage:**
- Valid input → memory created
- Invalid input → validation error
- Duplicate ID → conflict error
- All 6 types → created successfully

---

#### Task 4.2: Memory Manager - Read & Query
**Estimate:** 0.5 day
**Owner:** [TBD]

**Description:**
Implement read and query operations.

**Acceptance Criteria:**
- [ ] Get memory by ID
- [ ] List memories with filters
- [ ] Query with pagination
- [ ] Full-text search
- [ ] Handle not found gracefully
- [ ] Unit tests for all queries

**Interface:**
```typescript
class MemoryManager {
  async get(id: MemoryId): Promise<MemoryObjectExtended>
  async list(filter?: MemoryFilter): Promise<MemoryObjectExtended[]>
  async query(query: EddaQuery): Promise<EddaQueryResult>
  async search(text: string): Promise<MemoryObjectExtended[]>
}
```

**Test Coverage:**
- Get existing memory → returns memory
- Get non-existent → throws NotFoundError
- List by type → returns correct memories
- Search → returns relevant results
- Pagination → correct pages

---

#### Task 4.3: Memory Manager - Update & Delete
**Estimate:** 0.5 day
**Owner:** [TBD]

**Description:**
Implement update and delete operations with versioning.

**Acceptance Criteria:**
- [ ] Update memory (creates new version)
- [ ] Delete memory (soft delete)
- [ ] Update timestamp on modify
- [ ] Validate updates
- [ ] Git commit per operation
- [ ] Index updated
- [ ] Unit tests for all scenarios

**Interface:**
```typescript
class MemoryManager {
  async update(id: MemoryId, patch: MemoryObjectPatch): Promise<MemoryObjectExtended>
  async delete(id: MemoryId): Promise<void>
}
```

**Test Coverage:**
- Update memory → new version created
- Delete memory → marked deleted
- Update non-existent → throws error
- Concurrent update → handled gracefully

---

#### Task 4.4: Error Handling & Logging
**Estimate:** 0.5 day
**Owner:** [TBD]

**Description:**
Implement comprehensive error handling and logging.

**Acceptance Criteria:**
- [ ] Custom error types (NotFoundError, ValidationError, etc.)
- [ ] Structured logging (debug, info, warn, error)
- [ ] Error context includes memory ID
- [ ] No sensitive data in logs
- [ ] Unit tests for error scenarios

**Error Types:**
```typescript
class NotFoundError extends Error
class ValidationError extends Error
class ConflictError extends Error
class StorageError extends Error
```

**Files Created:**
```
packages/edda-core/src/
├── errors/
│   ├── not-found.error.ts
│   ├── validation.error.ts
│   ├── conflict.error.ts
│   └── storage.error.ts
└── logging/
    └── logger.ts
```

---

### Epic 5: Integration & Testing (2 days)

**Goal:** End-to-end testing and documentation

#### Task 5.1: Integration Tests
**Estimate:** 1 day
**Owner:** [TBD]

**Description:**
Write comprehensive integration tests covering full workflows.

**Acceptance Criteria:**
- [ ] Create → Read → Update → Delete workflow
- [ ] Concurrent operations
- [ ] Large dataset (1000 memories)
- [ ] Query performance tests
- [ ] Git history verification
- [ ] Index consistency tests
- [ ] All tests pass reliably

**Test Scenarios:**
```typescript
describe('Integration: Full CRUD Workflow', () => {
  test('create memory → appears in queries', ...)
  test('update memory → new version in Git', ...)
  test('delete memory → removed from index', ...)
  test('1000 memories → query performance <100ms', ...)
})
```

**Files Created:**
```
packages/edda-core/tests/integration/
├── crud-workflow.test.ts
├── concurrent-operations.test.ts
├── performance.test.ts
└── git-history.test.ts
```

---

#### Task 5.2: Documentation
**Estimate:** 0.5 day
**Owner:** [TBD]

**Description:**
Write developer documentation and API reference.

**Acceptance Criteria:**
- [ ] README with quick start guide
- [ ] API reference (JSDoc exported)
- [ ] Architecture diagrams
- [ ] Usage examples
- [ ] Contributing guidelines
- [ ] Troubleshooting guide

**Files Created:**
```
packages/edda-core/
├── README.md
├── docs/
│   ├── api-reference.md
│   ├── architecture.md
│   ├── usage-examples.md
│   └── troubleshooting.md
└── examples/
    └── basic-usage.ts
```

---

#### Task 5.3: Performance Benchmarks
**Estimate:** 0.5 day
**Owner:** [TBD]

**Description:**
Create performance benchmarks and validate targets.

**Acceptance Criteria:**
- [ ] Write operation: <100ms p95
- [ ] Read operation: <50ms p95
- [ ] Query (10 results): <100ms p95
- [ ] Full-text search: <200ms p95
- [ ] Benchmark suite runs in CI
- [ ] Performance regression detection

**Benchmarks:**
```typescript
describe('Performance Benchmarks', () => {
  bench('write memory', ...)
  bench('read memory', ...)
  bench('query 10 memories', ...)
  bench('search across 1000 memories', ...)
})
```

**Files Created:**
```
packages/edda-core/benchmarks/
├── write.bench.ts
├── read.bench.ts
├── query.bench.ts
└── search.bench.ts
```

---

## Dependencies

### External
- Node.js ≥18
- Git CLI (for storage)
- SQLite3 (bundled with better-sqlite3)

### Internal (Monorepo)
- `@anvil/edda-stack` (contracts)

### NPM Packages
- `zod` - Schema validation
- `js-yaml` - YAML serialization
- `better-sqlite3` - SQLite bindings
- `ulid` - ID generation
- `vitest` - Testing
- `proper-lockfile` - File locking

---

## Risks & Mitigations

### Risk 1: Git Performance
**Severity:** Medium
**Impact:** Slow commits with many memories
**Probability:** Medium (likely at scale)

**Mitigation:**
- Benchmark early with 1000 memories
- Design abstraction layer for PostgreSQL migration
- Batch commits where possible

**Contingency:**
If Git performance is unacceptable:
- Option A: Optimize Git operations (sparse checkout, etc.)
- Option B: Accelerate PostgreSQL backend development
- Option C: Reduce commit frequency (batch writes)

---

### Risk 2: YAML Parsing Overhead
**Severity:** Low
**Impact:** Slower read operations
**Probability:** Low (SQLite index mitigates)

**Mitigation:**
- SQLite index handles most queries (no YAML parsing)
- Cache frequently accessed memories
- Profile and optimize hot paths

---

### Risk 3: Index Drift
**Severity:** Medium
**Impact:** Query results don't match Git storage
**Probability:** Low

**Mitigation:**
- Atomic operations (write → index together)
- Rebuild index from Git (idempotent recovery)
- Automated consistency checks in tests

---

## Success Metrics

### Functional
- ✅ All 6 memory types can be created
- ✅ CRUD operations work correctly
- ✅ Queries return correct results
- ✅ Git history is clean and auditable

### Performance
- ✅ Write: <100ms p95
- ✅ Read: <50ms p95
- ✅ Query: <100ms p95
- ✅ Search: <200ms p95

### Quality
- ✅ 90%+ test coverage
- ✅ Zero critical bugs in integration tests
- ✅ All benchmarks pass
- ✅ Documentation complete

---

## Deliverables

### Code
- [ ] `packages/edda-core` package
- [ ] Git storage adapter
- [ ] SQLite index adapter
- [ ] Memory manager with CRUD
- [ ] Zod validation schemas
- [ ] 50+ unit tests
- [ ] 10+ integration tests
- [ ] Performance benchmarks

### Documentation
- [ ] README with quick start
- [ ] API reference
- [ ] Architecture diagrams
- [ ] Usage examples
- [ ] Troubleshooting guide

### Infrastructure
- [ ] CI pipeline for tests
- [ ] Code coverage reporting
- [ ] Performance benchmark tracking

---

## Timeline

### Week 1 (Days 1-5)

**Day 1: Setup**
- Task 1.1: Create package (2h)
- Task 1.2: Test infrastructure (2h)
- Task 1.3: Zod schemas (2h)

**Day 2-3: Git Storage**
- Task 2.1: Git storage adapter (1d)
- Task 2.2: Git operations (1d)

**Day 4: Git Storage (cont.)**
- Task 2.3: File locking (0.5d)
- Task 2.4: YAML serialization (0.5d)

**Day 5: SQLite Index**
- Task 3.1: SQLite schema (0.5d)
- Task 3.2: Index adapter (start)

### Week 2 (Days 6-10)

**Day 6: SQLite Index (cont.)**
- Task 3.2: Index adapter (finish 0.5d)
- Task 3.3: Index sync (0.5d)

**Day 7-8: Memory Manager**
- Task 4.1: Create operation (0.5d)
- Task 4.2: Read & query (0.5d)
- Task 4.3: Update & delete (0.5d)
- Task 4.4: Error handling (0.5d)

**Day 9-10: Integration & Testing**
- Task 5.1: Integration tests (1d)
- Task 5.2: Documentation (0.5d)
- Task 5.3: Benchmarks (0.5d)

---

## Sign-Off

### Definition of Done
- [ ] All tasks completed
- [ ] All tests passing (unit + integration)
- [ ] Code coverage ≥90%
- [ ] Performance benchmarks meet targets
- [ ] Documentation complete
- [ ] Code reviewed and approved
- [ ] Merged to main branch

### Approval
- [ ] Tech Lead: _______________
- [ ] Engineering Manager: _______________
- [ ] Date: _______________

---

## Next Phase

After Phase 0 completion, proceed to:
**Phase 1: Promotion Pipeline** (3 weeks)
- Ember → Edda promotion workflow
- Human review interface
- Type mapping and diff generation

See: `/plans/aps/phase-1-promotion.aps.md` (to be created)

---

**Document Version:** 1.0.0
**Last Updated:** 2026-01-19
**Status:** Ready for approval
