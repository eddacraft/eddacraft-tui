# Edda Storage Architecture: Comparative Analysis

**Purpose:** Evaluate storage strategies before APS planning
**Date:** 2026-01-19
**Decision Required:** Choose storage approach for Edda implementation

---

## Executive Summary

This document compares three storage approaches for Edda:
1. **Git-backed YAML + SQLite Index** (our proposed approach)
2. **Git-backed JSONL + SQLite Cache** (Beads-inspired approach)
3. **PostgreSQL + Git Snapshots** (enterprise alternative)

**Recommendation:** Proceed with **Git-backed YAML + SQLite Index** for Phase 0-1, with abstraction layer enabling future PostgreSQL migration if needed.

**Context:** Edda is the top layer of a three-layer memory stack (Kindling → Ember → Edda). Each layer has different storage requirements based on its role and data characteristics.

---

## Three-Layer Stack Architecture

Before diving into Edda's storage options, it's important to understand how Edda fits into the broader memory stack:

```
┌─────────────────────────────────────────────────────────────┐
│                    Anvil Runtime Activity                    │
└────────────────────────┬────────────────────────────────────┘
                         ↓
                    Observations
                         ↓
┌────────────────────────▼────────────────────────────────────┐
│ LAYER 1: KINDLING (Capture)                                 │
│ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│ Purpose:  Capture facts, no interpretation                   │
│ Storage:  SQLite + WAL mode + FTS5                          │
│ Volume:   High (1000s per session)                          │
│ Lifetime: Bounded (per session/capsule)                     │
│ Trust:    Facts only (what actually happened)               │
└────────────────────────┬────────────────────────────────────┘
                         ↓
                 Pattern Detection
                         ↓
┌────────────────────────▼────────────────────────────────────┐
│ LAYER 2: EMBER (Interpretation)                             │
│ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│ Purpose:  Propose candidate memories (heuristic)            │
│ Storage:  SQLite (ephemeral, TTL-based)                     │
│ Volume:   Medium (10s-100s active)                          │
│ Lifetime: Temporary (30 day TTL)                            │
│ Trust:    Heuristic (may be wrong)                          │
└────────────────────────┬────────────────────────────────────┘
                         ↓
                Human Approval
                         ↓
┌────────────────────────▼────────────────────────────────────┐
│ LAYER 3: EDDA (Memory) ← THIS DOCUMENT                      │
│ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│ Purpose:  Store curated institutional truth                 │
│ Storage:  Git+YAML + SQLite Index (PROPOSED)                │
│ Volume:   Low (100s-1000s total)                            │
│ Lifetime: Permanent (explicit retirement only)              │
│ Trust:    High (human-approved)                             │
└─────────────────────────────────────────────────────────────┘
```

### Why Different Storage for Each Layer?

| Layer | Storage | Why This Choice |
|-------|---------|----------------|
| **Kindling** | SQLite + FTS5 | • High-volume writes (1000s/session)<br>• Local-first, no external deps<br>• Bounded queries (session-scoped)<br>• Fast text search (FTS5)<br>• Disposable (old sessions pruned) |
| **Ember** | SQLite + TTL | • Ephemeral by design (30d TTL)<br>• Medium volume (candidates)<br>• Fast queries for review<br>• Disposable (expired proposals deleted)<br>• Same tech as Kindling (simplicity) |
| **Edda** | Git+YAML + Index | • **Durable** (never auto-deleted)<br>• **Auditable** (Git history)<br>• **Human-readable** (YAML for reviews)<br>• **Versioned** (Git commits)<br>• Low volume (infrequent writes)<br>• High trust (institutional truth) |

### Key Insight: Storage Strategy Reflects Trust Level

The storage durability increases as trust increases:

```
Kindling (SQLite)     → Disposable facts (can rebuild from logs)
     ↓
Ember (SQLite+TTL)    → Disposable candidates (can regenerate)
     ↓
Edda (Git+YAML)       → Permanent truth (cannot lose!)
```

**Edda is the only layer where data loss is unacceptable.** This is why Git-backed storage (with its inherent versioning, backup, and audit trail) is critical for Edda but not necessary for Kindling or Ember.

---

## Approach 1: Git-backed YAML + SQLite Index (Proposed)

### Architecture

```
┌─────────────────────────────────────────┐
│          Application Layer              │
└─────────────┬───────────────────────────┘
              │
    ┌─────────▼─────────┐
    │  Memory Manager   │
    └─────────┬─────────┘
              │
    ┌─────────┴─────────┐
    │                   │
┌───▼────────┐   ┌─────▼──────┐
│ Git Storage│   │ SQLite Index│
│  (YAML)    │   │   (FTS5)    │
│ PRIMARY    │   │  SECONDARY  │
└────────────┘   └─────────────┘
     │                  │
     └─────────┬────────┘
               │
         ┌─────▼─────┐
         │ File System│
         └───────────┘
```

### Data Flow

**Write Path:**
```
Memory Object Created
    ↓
Validate with Zod
    ↓
Serialize to YAML
    ↓
Write to .edda/memories/{type}/{id}.yaml
    ↓
Git commit
    ↓
Index in SQLite (async)
```

**Read Path:**
```
Query Request
    ↓
SQLite Index (find matching IDs)
    ↓
Load YAML files from disk
    ↓
Deserialize and return
```

### File Structure

```
.edda/
├── memories/
│   ├── decision/
│   │   ├── EDDA-M-decision-01h2...yaml
│   │   └── EDDA-M-decision-02h3...yaml
│   ├── pattern/
│   │   └── EDDA-M-pattern-01h4...yaml
│   └── ...
├── audit/
│   └── 2025-01.jsonl
├── principals/
│   ├── users.yaml
│   ├── agents.yaml
│   └── teams.yaml
├── hooks/
│   └── *.yaml
└── .index/
    └── memories.db (SQLite)
```

### Example Memory File (YAML)

```yaml
id: EDDA-M-decision-01h2xcf8k9m4n5p6
type: decision
status: active

statement: |
  All database schema changes must use Prisma migrations.
  Direct schema modifications via SQL are prohibited.

context:
  when: For all database schema modifications
  why: |
    Direct schema changes caused 3 production incidents in Q4 2024.
    Migrations provide audit trail and rollback capability.
  conditions:
    - Use: npx prisma migrate dev
    - Review migrations before deployment
    - Test rollback procedures
  tags:
    - database
    - schema
    - migrations
    - critical

confidence: high
confidence_rationale: Ratified by platform team after incident analysis

scope:
  type: project
  identifier: anvil

authority:
  owner:
    type: team
    identifier: team:platform
  reviewers:
    - type: human
      identifier: user:alice
  visibility: public

enforcement:
  mode: blocking
  hooks:
    - pre_execution
  override_requires:
    - org_admin

provenance:
  ember_source:
    proposal_id: EMBER-P-decision-01h1...
    proposal_type: decision
    confidence: 0.85
    created_at: "2025-01-14T15:00:00Z"
  kindling_sources:
    - observation_id: KINDLING-OBS-01h0...
      kind: error_recorded
      session_id: SESSION-042
  source_sessions:
    - SESSION-042
    - SESSION-043

attribution:
  promoted_by: user:alice
  promoted_at: "2025-01-15T10:00:00Z"
  reason: Post-incident policy after INC-2025-001

evolution:
  supersedes: []
  superseded_by: null

review_policy:
  strategy: time_based
  interval_days: 180
  last_reviewed_at: "2025-01-15T10:00:00Z"

created_at: "2025-01-15T10:00:00Z"
updated_at: "2025-01-15T10:00:00Z"

metadata:
  incident_id: INC-2025-001
  decision_maker:
    type: team
    identifier: team:platform
  alternatives_considered:
    - Manual SQL scripts
    - Flyway migrations
  consequences:
    expected:
      - Fewer production incidents
      - Better audit trail
      - Easier rollbacks
```

### Pros

✅ **Human-Readable:** YAML is easy to read/edit for debugging and manual fixes
✅ **Schema-Friendly:** YAML's structure matches our complex nested objects well
✅ **Git-Native:** Each memory is a file, clean diffs, easy to review in PRs
✅ **Type-Safe:** Zod validation ensures data integrity before write
✅ **Comments:** YAML supports comments (useful for internal notes)
✅ **Multi-Line:** Natural support for long text fields (statement, rationale)
✅ **Tooling:** Excellent editor support (syntax highlighting, validation)
✅ **Simplicity:** No complex synchronization logic needed

### Cons

❌ **Write Performance:** YAML serialization slower than JSON (~2x)
❌ **Parse Overhead:** YAML parsing more complex than JSON
❌ **File Size:** YAML is slightly more verbose than JSON
❌ **Merge Conflicts:** Complex nested YAML can have harder merge conflicts
❌ **No Atomic Append:** Must rewrite entire file for updates
❌ **Query Performance:** Must parse YAML to query (hence SQLite index needed)

### Performance Characteristics

| Operation | Latency | Notes |
|-----------|---------|-------|
| Write (single) | ~80ms | Serialize + Git commit |
| Read (single) | ~20ms | Direct file read + parse |
| Query (10 results) | ~150ms | SQLite index + 10 YAML parses |
| Full-text search | ~200ms | SQLite FTS5 |
| Bulk write (100) | ~5s | Batched git commits |

### Scalability Limits

- **Sweet Spot:** 100-5,000 memories
- **Acceptable:** 5,000-10,000 memories (query performance degrades)
- **Breaking Point:** >10,000 memories (Git operations slow, need PostgreSQL)

---

## Approach 2: Git-backed JSONL + SQLite Cache (Beads-Inspired)

### Architecture

```
┌─────────────────────────────────────────┐
│          Application Layer              │
└─────────────┬───────────────────────────┘
              │
    ┌─────────▼─────────┐
    │  Memory Manager   │
    └─────────┬─────────┘
              │
    ┌─────────┴─────────┐
    │                   │
┌───▼────────┐   ┌─────▼──────┐
│ Git Storage│◄─►│ SQLite Cache│
│  (JSONL)   │   │  (FAST)     │
│ CANONICAL  │   │ DISPOSABLE  │
└────────────┘   └─────────────┘
     │                  ▲
     └──────────────────┘
     Background Daemon (sync)
```

### Data Flow (Beads-Style)

**Write Path:**
```
Memory Object Created
    ↓
Validate with Zod
    ↓
Serialize to JSON (single line)
    ↓
Append to .edda/memories.jsonl
    ↓
Update SQLite cache (immediate)
    ↓
Background daemon commits to Git
```

**Read Path:**
```
Query Request
    ↓
SQLite Cache (full query)
    ↓
Return cached objects
    (no file I/O needed)
```

### File Structure

```
.edda/
├── memories.jsonl          # All memories, one per line
├── audit.jsonl             # Audit trail
├── principals.jsonl        # Users, agents, teams
├── hooks.jsonl             # Enforcement hooks
└── .cache/
    └── edda.db (SQLite)    # Full cache of all data
```

### Example Memory Line (JSONL)

```json
{"id":"EDDA-M-decision-01h2...","type":"decision","status":"active","statement":"All database schema changes must use Prisma migrations","context":{"when":"For all database modifications","why":"Direct changes caused incidents","conditions":["Use: npx prisma migrate dev"],"tags":["database","schema"]},"confidence":"high","scope":{"type":"project","identifier":"anvil"},"authority":{"owner":{"type":"team","identifier":"team:platform"},"visibility":"public"},"enforcement":{"mode":"blocking","hooks":["pre_execution"]},"created_at":"2025-01-15T10:00:00Z"}
```

### Pros

✅ **Append-Only:** JSONL supports atomic appends (no file rewrites)
✅ **Fast Writes:** JSON serialization faster than YAML (~50ms vs ~80ms)
✅ **Fast Queries:** SQLite cache eliminates file I/O for reads
✅ **Merge-Friendly:** Line-based format has better merge semantics
✅ **Incremental Sync:** Can sync only new lines (efficient)
✅ **Proven:** Beads demonstrates this works for agent systems
✅ **Cache Rebuild:** Can always rebuild SQLite from JSONL (idempotent)

### Cons

❌ **Not Human-Readable:** Dense JSON lines hard to read/debug
❌ **No Comments:** JSON doesn't support comments
❌ **Line Length:** Complex objects create very long lines (>1KB)
❌ **No Diff Clarity:** Git diffs show entire line changes, lose granularity
❌ **Update Complexity:** Updates require append of new version + mark old as deleted
❌ **Sync Complexity:** Background daemon adds failure modes (Beads has issues)
❌ **Eventual Consistency:** SQLite cache can diverge from JSONL during sync
❌ **Compaction Needed:** Deleted/updated entries accumulate, need periodic compaction

### Performance Characteristics

| Operation | Latency | Notes |
|-----------|---------|-------|
| Write (single) | ~50ms | Append + cache update |
| Read (single) | ~5ms | Cache only (no disk I/O) |
| Query (10 results) | ~20ms | Pure SQLite query |
| Full-text search | ~150ms | SQLite FTS5 |
| Bulk write (100) | ~2s | 100 appends + cache updates |
| Cache rebuild | ~30s | Parse all JSONL into SQLite |

### Scalability Limits

- **Sweet Spot:** 1,000-50,000 memories
- **Acceptable:** 50,000-100,000 memories (JSONL gets large)
- **Breaking Point:** >100,000 memories (file size issues, need sharding)

### Known Issues (from Beads)

> "The system uses git hooks, daemons, and intelligent multidirectional syncing... which has led to broken edge cases, such as issues getting resurrected after deletion, or even issues getting deleted or overwritten."

> "Under extreme concurrent load (100+ simultaneous operations), you may see 'database is locked' errors."

---

## Approach 3: PostgreSQL + Git Snapshots (Enterprise)

### Architecture

```
┌─────────────────────────────────────────┐
│          Application Layer              │
└─────────────┬───────────────────────────┘
              │
    ┌─────────▼─────────┐
    │  Memory Manager   │
    └─────────┬─────────┘
              │
    ┌─────────▼─────────┐
    │    PostgreSQL      │
    │   (PRIMARY DB)     │
    └─────────┬─────────┘
              │
    ┌─────────▼─────────┐
    │ Git Snapshots      │
    │ (nightly export)   │
    └───────────────────┘
```

### Data Model

**PostgreSQL Tables:**
```sql
CREATE TABLE memories (
  id VARCHAR(50) PRIMARY KEY,
  type VARCHAR(20) NOT NULL,
  status VARCHAR(20) NOT NULL,
  statement TEXT NOT NULL,
  context JSONB NOT NULL,
  confidence VARCHAR(10) NOT NULL,
  scope JSONB NOT NULL,
  authority JSONB NOT NULL,
  enforcement JSONB NOT NULL,
  provenance JSONB NOT NULL,
  attribution JSONB NOT NULL,
  evolution JSONB NOT NULL,
  review_policy JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL,
  metadata JSONB,

  -- Indexes
  INDEX idx_type (type),
  INDEX idx_status (status),
  INDEX idx_created_at (created_at),
  GIN INDEX idx_context (context),
  GIN INDEX idx_search (to_tsvector('english', statement || ' ' || context->>'why'))
);

CREATE TABLE memory_versions (
  version_id SERIAL PRIMARY KEY,
  memory_id VARCHAR(50) REFERENCES memories(id),
  version_number INT NOT NULL,
  snapshot JSONB NOT NULL,
  changed_by JSONB NOT NULL,
  changed_at TIMESTAMPTZ NOT NULL,
  change_reason TEXT
);

-- Full-text search
CREATE INDEX idx_memory_fts ON memories
  USING GIN (to_tsvector('english', statement || ' ' || context->>'why'));
```

### Pros

✅ **Scalability:** Handles millions of records easily
✅ **Performance:** <10ms queries, <50ms writes
✅ **ACID:** Full transactions, no sync issues
✅ **Powerful Queries:** Complex JOINs, aggregations, CTEs
✅ **Built-in FTS:** PostgreSQL has excellent full-text search
✅ **JSON Support:** Native JSONB for flexible schemas
✅ **Mature:** Battle-tested, extensive tooling
✅ **Replication:** Built-in replication for HA
✅ **Backup:** Point-in-time recovery

### Cons

❌ **Infrastructure:** Requires PostgreSQL server (not local)
❌ **Complexity:** More moving parts (connection pooling, migrations)
❌ **Not Git-Native:** Git snapshots are secondary, not primary
❌ **Deployment:** Need to manage DB servers, credentials, migrations
❌ **Cost:** Server costs for hosting (vs free local SQLite)
❌ **Latency:** Network round-trips (10-50ms) vs local disk (1-5ms)
❌ **Dev Experience:** Requires running PostgreSQL locally

### Performance Characteristics

| Operation | Latency | Notes |
|-----------|---------|-------|
| Write (single) | ~30ms | Network + DB write |
| Read (single) | ~10ms | Network + index lookup |
| Query (10 results) | ~50ms | Network + query execution |
| Full-text search | ~100ms | Network + FTS |
| Bulk write (100) | ~500ms | Batched INSERT |

### Scalability Limits

- **Sweet Spot:** 10,000-10,000,000 memories
- **Acceptable:** Virtually unlimited with proper indexing
- **Breaking Point:** Petabyte scale (but Edda won't reach this)

---

## Decision Matrix

### Feature Comparison

| Feature | Git+YAML | Git+JSONL | PostgreSQL |
|---------|----------|-----------|------------|
| **Human Readability** | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐ (with tools) |
| **Write Performance** | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Read Performance** | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Query Performance** | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Scalability** | ⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Git Integration** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐ |
| **Simplicity** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ |
| **Reliability** | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Merge Safety** | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Zero Infrastructure** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐ |
| **Dev Experience** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ |

### Use Case Alignment

**Edda's Requirements:**

1. **Human-in-the-Loop Review** → Favors Git+YAML (readability)
2. **Institutional Memory** → Favors Git+YAML (clarity, comments)
3. **Audit Trail** → All approaches work (Git or DB logs)
4. **Complex Objects** → Favors Git+YAML or PostgreSQL (structure)
5. **Low Ops Burden** → Favors Git+YAML (no server)
6. **<10K Memories (v1)** → Git+YAML sufficient
7. **Future: >10K Memories** → PostgreSQL needed

### Beads vs Edda Context

**Beads (Issue Tracking):**
- Many small objects (1000s of issues)
- Frequent writes (agents create issues constantly)
- Agent-primary (speed over readability)
- Append-only workflow (issues rarely updated)
- Multi-agent concurrency (high)

**Edda (Institutional Memory):**
- Fewer large objects (100s-1000s of memories)
- Infrequent writes (human-approved only)
- Human-primary (readability critical for review)
- Update workflow (memories evolve, get retired)
- Single-writer mostly (one human reviewing at a time)

**Conclusion:** Beads' JSONL approach is optimized for problems Edda doesn't have (high-frequency agent writes, extreme concurrency). Edda prioritizes readability and clarity for human reviewers.

---

## Recommendation

### Phase 0-1: Git-backed YAML + SQLite Index

**Rationale:**
1. **Human Review UX:** YAML's readability is critical for promotion pipeline (humans must review proposals and understand context)
2. **Simplicity:** No background daemon, no sync issues, straightforward implementation
3. **Git-Native:** Clean diffs for PR reviews, easy rollback, natural versioning
4. **Sufficient Performance:** <10K memories will perform well (<200ms queries)
5. **Low Risk:** Proven pattern, no novel synchronization logic
6. **Comments Support:** Useful for internal notes during development

### Phase 2+: Add Abstraction Layer

Implement storage port abstraction:

```typescript
interface IStorageBackend {
  write(id: string, data: MemoryObjectExtended): Promise<void>
  read(id: string): Promise<MemoryObjectExtended>
  query(query: EddaQuery): Promise<MemoryId[]>
  delete(id: string): Promise<void>
}

class GitYamlBackend implements IStorageBackend { ... }
class PostgreSQLBackend implements IStorageBackend { ... }
```

This allows migration to PostgreSQL if:
- Memory count exceeds 10,000
- Query performance degrades below acceptable levels
- Multi-tenancy requires database isolation
- Enterprise customers require hosted solution

### Migration Path (if needed)

```
Phase 0-6:  Git+YAML (v1 release)
            ↓
Phase 7:    Add abstraction layer
            ↓
Post-v1:    Implement PostgreSQL backend
            ↓
Post-v1:    Migration tool (YAML → PostgreSQL)
            ↓
v2:         PostgreSQL default, Git+YAML legacy mode
```

---

## Alternative: Hybrid Approach

If we're concerned about future scalability, consider:

### Hybrid Git+YAML + PostgreSQL (Optional)

**Architecture:**
- Primary: Git+YAML (human-readable, auditable)
- Secondary: PostgreSQL replica (fast queries, analytics)
- Sync: Background job exports YAML to PostgreSQL nightly

**Benefits:**
- Best of both worlds: readability + performance
- PostgreSQL used only for queries (not source of truth)
- Can delay PostgreSQL until actually needed

**Drawbacks:**
- More complexity (two storage systems)
- Eventual consistency (PostgreSQL lags Git)
- Higher operational burden

**Recommendation:** Defer this until proven need (likely post-v1).

---

## Comparison to Industry Practices

### Similar Systems

**Git-backed Configuration:**
- Kubernetes: YAML configs in Git (similar to our approach)
- Terraform: HCL configs in Git (similar philosophy)
- GitOps: All config in Git (Argo, Flux)

**Agent Memory Systems:**
- Beads: JSONL + SQLite (optimized for agents)
- LangChain Memory: Vector stores + SQLite
- AutoGPT: JSON files (simple but not scalable)

**Knowledge Management:**
- Confluence: PostgreSQL + Elasticsearch
- Notion: PostgreSQL + custom storage
- GitHub Issues: PostgreSQL (not Git-backed)

**Multi-Layer Memory Architectures:**
- **Kindling + Ember + Edda:** Three layers with different storage strategies
  - Layer 1 (Kindling): SQLite for high-volume capture
  - Layer 2 (Ember): SQLite+TTL for ephemeral candidates
  - Layer 3 (Edda): Git+YAML for permanent truth
- **LangChain:** Single-layer (immediate storage, no curation pipeline)
- **Beads:** Two-layer (JSONL + SQLite cache, no human review)

**Edda's Position:**
- Closer to **GitOps/IaC** than issue trackers (prioritizes human readability and auditability)
- Unique three-layer approach with **intentional storage heterogeneity**
- Each layer optimized for its trust level and data characteristics

---

## Risk Assessment

### Git+YAML Risks

**Medium Risk: Git Performance**
- Symptom: Slow commits/pushes with >5,000 memories
- Mitigation: Abstraction layer for PostgreSQL migration
- Monitoring: Track git operation latency

**Low Risk: YAML Parsing Performance**
- Symptom: Query latency >500ms
- Mitigation: SQLite index handles most queries
- Monitoring: Track p95 query latency

**Low Risk: Merge Conflicts**
- Symptom: Conflicts during concurrent edits
- Mitigation: Mostly single-writer (human reviews), rare concurrency
- Monitoring: Track conflict rate

### Git+JSONL Risks

**High Risk: Sync Reliability**
- Symptom: Cache divergence, data loss (Beads experienced this)
- Mitigation: Complex daemon logic, extensive testing
- Monitoring: Cache consistency checks

**Medium Risk: Human Review UX**
- Symptom: Reviewers struggle with dense JSON
- Mitigation: Build rich UI for viewing (more dev work)
- Monitoring: User feedback

**Low Risk: JSONL Compaction**
- Symptom: File grows indefinitely
- Mitigation: Periodic compaction job
- Monitoring: File size alerts

### PostgreSQL Risks

**High Risk: Operational Complexity**
- Symptom: Outages, connection issues, credential leaks
- Mitigation: Managed service (RDS, Cloud SQL), good ops practices
- Monitoring: Database health metrics

**Medium Risk: Dev Experience**
- Symptom: Devs must run PostgreSQL locally
- Mitigation: Docker Compose setup, seed data scripts
- Monitoring: Developer feedback

**Low Risk: Lock-In**
- Symptom: Hard to migrate away from PostgreSQL
- Mitigation: Abstraction layer, export utilities
- Monitoring: N/A

---

## Performance Benchmarks (Projected)

### Scenario: 1,000 Memories

| Operation | Git+YAML | Git+JSONL | PostgreSQL |
|-----------|----------|-----------|------------|
| Single write | 80ms | 50ms | 30ms |
| Single read | 20ms | 5ms | 10ms |
| Query (10 results) | 150ms | 20ms | 50ms |
| Full-text search | 200ms | 150ms | 100ms |
| Bulk write (100) | 5s | 2s | 500ms |

### Scenario: 10,000 Memories

| Operation | Git+YAML | Git+JSONL | PostgreSQL |
|-----------|----------|-----------|------------|
| Single write | 150ms | 80ms | 35ms |
| Single read | 25ms | 5ms | 12ms |
| Query (10 results) | 300ms | 30ms | 60ms |
| Full-text search | 500ms | 250ms | 120ms |
| Bulk write (100) | 12s | 4s | 600ms |

**Observation:** Git+YAML degrades at 10K scale, Git+JSONL remains acceptable, PostgreSQL scales linearly.

---

## Cost Analysis

### Development Cost

| Approach | Initial Dev | Maintenance | Migration Effort |
|----------|-------------|-------------|------------------|
| Git+YAML | 2 weeks | Low | Medium (to PostgreSQL) |
| Git+JSONL | 3 weeks | Medium (daemon) | Medium (to PostgreSQL) |
| PostgreSQL | 3 weeks | Medium (ops) | Low (already there) |

### Operational Cost

| Approach | Infrastructure | Personnel | Monitoring |
|----------|----------------|-----------|------------|
| Git+YAML | $0 (local) | Minimal | Basic |
| Git+JSONL | $0 (local) | Low | Medium (daemon) |
| PostgreSQL | $50-500/mo | Medium | High (DB ops) |

### Risk Cost

| Approach | Data Loss Risk | Downtime Risk | Migration Risk |
|----------|----------------|---------------|----------------|
| Git+YAML | Very Low | Very Low | Medium |
| Git+JSONL | Low | Low | Medium |
| PostgreSQL | Very Low | Medium | Low |

---

## Final Recommendation

**Proceed with Git-backed YAML + SQLite Index for v1.**

### Justification

1. **Aligns with Edda's Philosophy**
   - Human-in-the-loop requires human-readable formats
   - Institutional memory needs clarity and auditability
   - YAML supports comments for context

2. **Sufficient for v1 Scale**
   - Target: <1,000 memories in first year
   - Acceptable: up to 5,000 memories
   - Performance: <200ms queries, <100ms writes

3. **Low Risk, High Clarity**
   - No novel synchronization logic (no daemon)
   - Git-native versioning (no custom version management)
   - Simple debugging (view/edit YAML files directly)

4. **Good Dev Experience**
   - No infrastructure setup required
   - Works offline (local Git)
   - Easy testing (fixture YAML files)

5. **Migration Path Exists**
   - Abstraction layer in Phase 0
   - PostgreSQL backend in Phase 7 or post-v1
   - Export/import tooling in Phase 6

### Success Criteria

Track these metrics to validate the decision:
- Git operation latency (target: <100ms p95)
- Query performance (target: <200ms p95)
- Memory count (alert: >5,000)
- Merge conflict rate (target: <1% of commits)
- Human review satisfaction (qualitative feedback)

If metrics degrade beyond thresholds, initiate PostgreSQL migration.

---

## References

- [Beads GitHub Repository](https://github.com/steveyegge/beads)
- [Introducing Beads: A coding agent memory system](https://steve-yegge.medium.com/introducing-beads-a-coding-agent-memory-system-637d7d92514a)
- [Beads: A Git-Friendly Issue Tracker for AI Coding Agents](https://betterstack.com/community/guides/ai/beads-issue-tracker-ai-agents/)

---

**Decision:** Approved for Phase 0 implementation
**Reviewers:** [Pending]
**Date:** 2026-01-19
