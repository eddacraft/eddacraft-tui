# Phase 3: Query & Retrieval - APS Document

**Phase:** 3 of 7
**Duration:** 2 weeks (10 working days)
**Dependencies:** Phase 0 (Foundation), Phase 2 (Authority & Trust)
**Status:** Not Started
**Owner:** TBD

---

## Phase Overview

### Purpose
Implement comprehensive query and retrieval capabilities for Edda memories, including full-text search, structured queries, provenance tracing, and optional semantic search.

### Scope
This phase delivers the query layer that allows users and agents to efficiently find relevant memories using text search, structured filters, and relationship traversal.

### Success Criteria
- ✅ SQLite FTS5 full-text search operational
- ✅ Structured query DSL working (filters, sorting, pagination)
- ✅ Provenance chain tracing functional
- ✅ Memory relationship queries (supersedes, related_to)
- ✅ CLI commands for search and query
- ✅ <50ms query response time (95th percentile)
- ✅ Optional semantic search interface ready (implementation optional)
- ✅ 100% test coverage on query logic

---

## Epic Breakdown

### Epic 1: SQLite FTS5 Full-Text Search
**Duration:** 2 days
**Priority:** P0 (Blocking)

#### Epic 1.1: FTS5 Schema & Indexing
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Set up SQLite FTS5 virtual table for full-text search across memory content.

**Acceptance Criteria:**
- FTS5 table indexes: statement, context.reasoning, tags
- Tokenizer configured for code/technical terms
- Index synchronized on memory create/update/delete
- Supports phrase queries, AND/OR/NOT operators
- Ranking by relevance (BM25)

**Implementation:**

```typescript
// packages/edda-core/src/query/fts-indexer.ts

export interface IFTSIndexer {
  /**
   * Initialize FTS5 table
   */
  initialize(): Promise<void>

  /**
   * Index a memory for full-text search
   */
  index(memory: MemoryObject): Promise<void>

  /**
   * Remove memory from index
   */
  remove(memoryId: MemoryId): Promise<void>

  /**
   * Full-text search
   */
  search(query: string, options?: FTSSearchOptions): Promise<SearchResult[]>

  /**
   * Rebuild entire index
   */
  rebuild(): Promise<void>
}

export interface FTSSearchOptions {
  limit?: number
  offset?: number
  minScore?: number
}

export interface SearchResult {
  memory_id: MemoryId
  rank: number           // BM25 score
  snippet: string        // Highlighted excerpt
}

export class FTS5Indexer implements IFTSIndexer {
  constructor(private db: Database) {}

  async initialize(): Promise<void> {
    // Create FTS5 virtual table
    await this.db.exec(`
      CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
        memory_id UNINDEXED,
        statement,
        reasoning,
        tags,
        tokenize = 'porter unicode61 remove_diacritics 2'
      );
    `)

    // Create triggers to keep FTS in sync
    await this.db.exec(`
      -- Trigger on insert (handled by explicit index() call)
      -- Trigger on update
      -- Trigger on delete (handled by explicit remove() call)
    `)
  }

  async index(memory: MemoryObject): Promise<void> {
    const stmt = await this.db.prepare(`
      INSERT INTO memory_fts (memory_id, statement, reasoning, tags)
      VALUES (?, ?, ?, ?)
      ON CONFLICT(memory_id) DO UPDATE SET
        statement = excluded.statement,
        reasoning = excluded.reasoning,
        tags = excluded.tags
    `)

    await stmt.run(
      memory.id,
      memory.statement,
      memory.context.reasoning || '',
      memory.tags.join(' '),
    )
  }

  async remove(memoryId: MemoryId): Promise<void> {
    await this.db.run(`
      DELETE FROM memory_fts WHERE memory_id = ?
    `, memoryId)
  }

  async search(query: string, options: FTSSearchOptions = {}): Promise<SearchResult[]> {
    const { limit = 50, offset = 0, minScore = 0 } = options

    // FTS5 query with ranking
    const stmt = await this.db.prepare(`
      SELECT
        memory_id,
        rank AS rank,
        snippet(memory_fts, 1, '<mark>', '</mark>', '...', 32) AS snippet
      FROM memory_fts
      WHERE memory_fts MATCH ?
      AND rank >= ?
      ORDER BY rank DESC
      LIMIT ? OFFSET ?
    `)

    const rows = await stmt.all(query, -minScore, limit, offset)

    return rows.map(row => ({
      memory_id: row.memory_id,
      rank: -row.rank, // FTS5 ranks are negative
      snippet: row.snippet,
    }))
  }

  async rebuild(): Promise<void> {
    // Delete all entries
    await this.db.run('DELETE FROM memory_fts')

    // Re-index all memories
    // (Assumes access to memory repository)
    const memories = await this.memoryRepo.listAll()
    for (const memory of memories) {
      await this.index(memory)
    }
  }
}
```

**SQLite FTS5 Configuration:**
```sql
-- Virtual table with porter stemming and unicode support
CREATE VIRTUAL TABLE memory_fts USING fts5(
  memory_id UNINDEXED,        -- Not indexed for search, just stored
  statement,                  -- Primary searchable field
  reasoning,                  -- Context reasoning
  tags,                       -- Space-separated tags
  tokenize = 'porter unicode61 remove_diacritics 2'
);

-- Example queries:
-- Simple: "authentication"
-- Phrase: "user authentication"
-- Boolean: "authentication AND (oauth OR jwt)"
-- NOT: "authentication NOT deprecated"
```

**File Structure:**
```
packages/edda-core/src/query/
├── fts-indexer.ts
├── __tests__/
│   └── fts-indexer.test.ts
```

**Tests:**
- Initialize FTS5 table
- Index memory (searchable immediately)
- Search with simple query
- Search with boolean operators (AND, OR, NOT)
- Search with phrase query ("exact match")
- Ranking by relevance
- Pagination (limit, offset)
- Minimum score filtering
- Remove from index
- Rebuild entire index

---

#### Epic 1.2: FTS Integration with Memory Manager
**Estimate:** 2 hours
**Owner:** TBD

**Description:**
Hook FTS indexer into memory CRUD operations to keep index synchronized.

**Acceptance Criteria:**
- Memory create triggers index()
- Memory update triggers index()
- Memory delete triggers remove()
- Index stays consistent with storage
- Failed index operations don't fail memory operations

**Implementation:**

```typescript
// packages/edda-core/src/memory/memory-manager.ts (enhancement)

export class MemoryManager implements IMemoryManager {
  constructor(
    private storage: IMemoryStorage,
    private index: IMemoryIndex,
    private ftsIndexer: IFTSIndexer,  // NEW
    private authz: IAuthorizationService,
    private audit: IAuditTrailService,
  ) {}

  async create(principal: Principal, data: CreateMemoryData): Promise<MemoryObject> {
    // ... existing authorization and validation

    const memory = this.buildMemory(data, principal)

    // Store in Git + SQLite index
    await this.storage.store(memory)
    await this.index.insert(memory)

    // Index for full-text search
    try {
      await this.ftsIndexer.index(memory)
    } catch (error) {
      // Log but don't fail the operation
      console.error(`Failed to index memory ${memory.id} for FTS:`, error)
    }

    // ... audit logging

    return memory
  }

  async update(principal: Principal, memoryId: MemoryId, updates: UpdateMemoryData): Promise<MemoryObject> {
    // ... existing authorization and update logic

    // Update FTS index
    try {
      await this.ftsIndexer.index(updatedMemory)
    } catch (error) {
      console.error(`Failed to update FTS index for memory ${memoryId}:`, error)
    }

    return updatedMemory
  }

  async delete(principal: Principal, memoryId: MemoryId): Promise<void> {
    // ... existing authorization and deletion

    // Remove from FTS index
    try {
      await this.ftsIndexer.remove(memoryId)
    } catch (error) {
      console.error(`Failed to remove memory ${memoryId} from FTS index:`, error)
    }
  }
}
```

**Tests:**
- Create memory → searchable immediately
- Update memory → index updated
- Delete memory → removed from search results
- FTS index failure doesn't break memory operations

---

### Epic 2: Structured Query DSL
**Duration:** 3 days
**Priority:** P0 (Blocking)

#### Epic 2.1: Query Schema & Parser
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Define structured query DSL for filtering, sorting, and paginating memories.

**Acceptance Criteria:**
- EddaQuery interface matches edda-extended.ts
- Supports filters: type, status, confidence, scope, tags, date_range
- Supports sorting: created_at, updated_at, confidence
- Supports pagination: limit, offset
- Query validation via Zod

**Implementation:**

```typescript
// packages/edda-core/src/query/query-schema.ts

export const EddaQuerySchema = z.object({
  // Filters
  filters: z.object({
    type: z.array(z.enum(['decision', 'pattern', 'warning', 'constraint', 'doctrine', 'lesson'])).optional(),
    status: z.array(z.enum(['active', 'deprecated', 'superseded'])).optional(),
    confidence: z.array(z.enum(['high', 'medium', 'low'])).optional(),
    scope: z.string().optional(),  // ScopeSpecifier pattern
    tags: z.array(z.string()).optional(),
    author: z.string().optional(),
    created_after: z.string().datetime().optional(),
    created_before: z.string().datetime().optional(),
    updated_after: z.string().datetime().optional(),
    updated_before: z.string().datetime().optional(),
  }).optional(),

  // Full-text search
  search: z.object({
    query: z.string(),
    fields: z.array(z.enum(['statement', 'reasoning', 'tags', 'all'])).optional(),
  }).optional(),

  // Sorting
  sort: z.object({
    field: z.enum(['created_at', 'updated_at', 'confidence', 'relevance']),
    direction: z.enum(['asc', 'desc']),
  }).optional(),

  // Pagination
  pagination: z.object({
    limit: z.number().int().min(1).max(1000).default(50),
    offset: z.number().int().min(0).default(0),
  }).optional(),

  // Include related data
  include: z.object({
    provenance: z.boolean().optional(),
    related_memories: z.boolean().optional(),
    superseded_by: z.boolean().optional(),
  }).optional(),
})

export type EddaQuery = z.infer<typeof EddaQuerySchema>

export interface EddaQueryResult {
  memories: MemoryObject[]
  total_count: number
  page_info: {
    has_next_page: boolean
    has_previous_page: boolean
    limit: number
    offset: number
  }
  query_metadata: {
    execution_time_ms: number
    indexed_search: boolean
  }
}
```

**Examples:**

```typescript
// Find all active warnings about authentication
const query1: EddaQuery = {
  filters: {
    type: ['warning'],
    status: ['active'],
  },
  search: {
    query: 'authentication',
  },
}

// Find high-confidence decisions from last 30 days
const query2: EddaQuery = {
  filters: {
    type: ['decision'],
    confidence: ['high'],
    created_after: new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString(),
  },
  sort: {
    field: 'created_at',
    direction: 'desc',
  },
  pagination: {
    limit: 20,
    offset: 0,
  },
}

// Find memories with specific tags
const query3: EddaQuery = {
  filters: {
    tags: ['security', 'oauth'],
  },
}
```

**Tests:**
- Parse valid query
- Reject invalid query (bad dates, invalid enum values)
- Default values applied (limit, offset)
- Query serialization (to/from JSON)

---

#### Epic 2.2: Query Executor
**Estimate:** 6 hours
**Owner:** TBD

**Description:**
Implement query executor that translates EddaQuery to SQL and executes against SQLite index.

**Acceptance Criteria:**
- IQueryService interface implemented
- Filters translated to SQL WHERE clauses
- FTS search integrated when search field present
- Sorting and pagination work correctly
- Returns EddaQueryResult with metadata
- <50ms execution time (95th percentile)

**Implementation:**

```typescript
// packages/edda-core/src/query/query-service.ts

export interface IQueryService {
  /**
   * Execute structured query
   */
  query(query: EddaQuery): Promise<EddaQueryResult>

  /**
   * Simple full-text search (convenience method)
   */
  search(text: string, limit?: number): Promise<MemoryObject[]>

  /**
   * Count memories matching filters (without fetching)
   */
  count(filters: EddaQuery['filters']): Promise<number>
}

export class QueryService implements IQueryService {
  constructor(
    private index: IMemoryIndex,
    private ftsIndexer: IFTSIndexer,
    private storage: IMemoryStorage,
  ) {}

  async query(query: EddaQuery): Promise<EddaQueryResult> {
    const startTime = performance.now()

    // Validate query
    const validated = EddaQuerySchema.parse(query)

    let memoryIds: MemoryId[]
    let indexedSearch = false

    // Step 1: Get memory IDs (from FTS or index)
    if (validated.search) {
      // Use FTS5 for full-text search
      const searchResults = await this.ftsIndexer.search(
        validated.search.query,
        {
          limit: (validated.pagination?.limit || 50) * 2, // Over-fetch for filtering
        },
      )
      memoryIds = searchResults.map(r => r.memory_id)
      indexedSearch = true
    } else {
      // Use SQLite index for structured filters
      memoryIds = await this.queryIndex(validated)
      indexedSearch = true
    }

    // Step 2: Fetch full memory objects from storage
    const memories = await this.storage.fetchMany(memoryIds)

    // Step 3: Apply additional filters if needed
    let filtered = this.applyFilters(memories, validated.filters)

    // Step 4: Sort
    if (validated.sort) {
      filtered = this.applySorting(filtered, validated.sort)
    }

    // Step 5: Paginate
    const total_count = filtered.length
    const limit = validated.pagination?.limit || 50
    const offset = validated.pagination?.offset || 0
    const paginated = filtered.slice(offset, offset + limit)

    // Step 6: Include related data if requested
    if (validated.include) {
      await this.includeRelatedData(paginated, validated.include)
    }

    const executionTime = performance.now() - startTime

    return {
      memories: paginated,
      total_count,
      page_info: {
        has_next_page: offset + limit < total_count,
        has_previous_page: offset > 0,
        limit,
        offset,
      },
      query_metadata: {
        execution_time_ms: Math.round(executionTime),
        indexed_search: indexedSearch,
      },
    }
  }

  private async queryIndex(query: EddaQuery): Promise<MemoryId[]> {
    // Build SQL query from filters
    const conditions: string[] = []
    const params: any[] = []

    if (query.filters?.type) {
      conditions.push(`type IN (${query.filters.type.map(() => '?').join(',')})`)
      params.push(...query.filters.type)
    }

    if (query.filters?.status) {
      conditions.push(`status IN (${query.filters.status.map(() => '?').join(',')})`)
      params.push(...query.filters.status)
    }

    if (query.filters?.confidence) {
      conditions.push(`confidence IN (${query.filters.confidence.map(() => '?').join(',')})`)
      params.push(...query.filters.confidence)
    }

    if (query.filters?.scope) {
      conditions.push(`scope LIKE ?`)
      params.push(`${query.filters.scope}%`)
    }

    if (query.filters?.tags) {
      // Tags stored as JSON array in SQLite
      for (const tag of query.filters.tags) {
        conditions.push(`EXISTS (SELECT 1 FROM json_each(tags) WHERE value = ?)`)
        params.push(tag)
      }
    }

    if (query.filters?.author) {
      conditions.push(`author = ?`)
      params.push(query.filters.author)
    }

    if (query.filters?.created_after) {
      conditions.push(`created_at >= ?`)
      params.push(query.filters.created_after)
    }

    if (query.filters?.created_before) {
      conditions.push(`created_at <= ?`)
      params.push(query.filters.created_before)
    }

    // Build final SQL
    const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : ''
    const sql = `
      SELECT memory_id
      FROM memory_index
      ${whereClause}
    `

    const rows = await this.index.db.all(sql, ...params)
    return rows.map(row => row.memory_id)
  }

  private applyFilters(memories: MemoryObject[], filters?: EddaQuery['filters']): MemoryObject[] {
    if (!filters) return memories

    return memories.filter(memory => {
      // Additional in-memory filtering if needed
      // (Most filtering should happen in SQL for performance)
      return true
    })
  }

  private applySorting(memories: MemoryObject[], sort: EddaQuery['sort']): MemoryObject[] {
    if (!sort) return memories

    const { field, direction } = sort

    return [...memories].sort((a, b) => {
      let aVal: any
      let bVal: any

      if (field === 'created_at') {
        aVal = new Date(a.authority.created_at).getTime()
        bVal = new Date(b.authority.created_at).getTime()
      } else if (field === 'updated_at') {
        aVal = new Date(a.authority.updated_at).getTime()
        bVal = new Date(b.authority.updated_at).getTime()
      } else if (field === 'confidence') {
        const confidenceOrder = { high: 3, medium: 2, low: 1 }
        aVal = confidenceOrder[a.confidence]
        bVal = confidenceOrder[b.confidence]
      }

      if (direction === 'asc') {
        return aVal < bVal ? -1 : aVal > bVal ? 1 : 0
      } else {
        return aVal > bVal ? -1 : aVal < bVal ? 1 : 0
      }
    })
  }

  private async includeRelatedData(
    memories: MemoryObject[],
    include: EddaQuery['include'],
  ): Promise<void> {
    // Fetch related data and attach to memories
    // (Implementation depends on storage layer)
  }

  async search(text: string, limit: number = 50): Promise<MemoryObject[]> {
    const result = await this.query({
      search: { query: text },
      pagination: { limit, offset: 0 },
    })
    return result.memories
  }

  async count(filters: EddaQuery['filters']): Promise<number> {
    const memoryIds = await this.queryIndex({ filters })
    return memoryIds.length
  }
}
```

**File Structure:**
```
packages/edda-core/src/query/
├── query-schema.ts
├── query-service.ts
├── __tests__/
│   ├── query-schema.test.ts
│   └── query-service.test.ts
```

**Tests:**
- Query with single filter (type)
- Query with multiple filters (type + status)
- Query with date range
- Query with tags
- Query with search + filters
- Sorting (ascending/descending)
- Pagination
- Count without fetching
- Performance: <50ms for 1000 memories

---

#### Epic 2.3: Query Optimization
**Estimate:** 2 hours
**Owner:** TBD

**Description:**
Optimize query performance with proper indexing and caching.

**Acceptance Criteria:**
- SQLite indexes on commonly queried fields
- Query plan analysis shows index usage
- 95th percentile query time <50ms
- Memory-efficient result streaming for large result sets

**Implementation:**

```sql
-- Additional SQLite indexes for query performance

-- Composite index for type + status queries
CREATE INDEX idx_memory_type_status ON memory_index(type, status);

-- Index for date range queries
CREATE INDEX idx_memory_created_at ON memory_index(created_at);
CREATE INDEX idx_memory_updated_at ON memory_index(updated_at);

-- Index for author queries
CREATE INDEX idx_memory_author ON memory_index(author);

-- Index for confidence queries
CREATE INDEX idx_memory_confidence ON memory_index(confidence);

-- Index for scope prefix matching
CREATE INDEX idx_memory_scope ON memory_index(scope);
```

```typescript
// Query plan analysis utility
export class QueryAnalyzer {
  async analyzeQuery(query: EddaQuery): Promise<QueryPlan> {
    const sql = this.buildSQL(query)
    const plan = await this.db.all(`EXPLAIN QUERY PLAN ${sql}`)

    return {
      uses_index: plan.some(row => row.detail.includes('USING INDEX')),
      estimated_rows: this.estimateRows(plan),
      suggestions: this.generateSuggestions(plan),
    }
  }
}
```

**Tests:**
- Index usage verified (EXPLAIN QUERY PLAN)
- Common queries use indexes
- Performance regression tests

---

### Epic 3: Provenance Tracing
**Duration:** 2 days
**Priority:** P1 (Important)

#### Epic 3.1: Provenance Query Service
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Implement provenance chain traversal to trace memories back to Kindling observations.

**Acceptance Criteria:**
- traceProvenance() returns full chain: Kindling → Ember → Edda
- Forward and backward traversal
- Handles missing links gracefully
- Returns structured provenance tree

**Implementation:**

```typescript
// packages/edda-core/src/query/provenance-service.ts

export interface ProvenanceNode {
  layer: 'kindling' | 'ember' | 'edda'
  id: string
  type: string
  timestamp: string
  author?: string
  confidence?: string
  children?: ProvenanceNode[]
}

export interface IProvenanceService {
  /**
   * Trace provenance chain for a memory
   * Returns full tree from Kindling to Edda
   */
  traceProvenance(memoryId: MemoryId): Promise<ProvenanceNode>

  /**
   * Find all memories derived from a Kindling observation
   */
  findDerivedMemories(kindlingId: string): Promise<MemoryObject[]>

  /**
   * Find all Ember proposals for a memory
   */
  findProposals(memoryId: MemoryId): Promise<EmberProposal[]>
}

export class ProvenanceService implements IProvenanceService {
  constructor(
    private memoryStorage: IMemoryStorage,
    private emberPort: IEmberPort,      // Port to query Ember
    private kindlingPort: IKindlingPort, // Port to query Kindling
  ) {}

  async traceProvenance(memoryId: MemoryId): Promise<ProvenanceNode> {
    // Get Edda memory
    const memory = await this.memoryStorage.fetch(memoryId)
    if (!memory) {
      throw new Error(`Memory ${memoryId} not found`)
    }

    const eddaNode: ProvenanceNode = {
      layer: 'edda',
      id: memory.id,
      type: memory.type,
      timestamp: memory.authority.created_at,
      author: memory.authority.author,
      confidence: memory.confidence,
      children: [],
    }

    // Trace back to Ember proposals
    if (memory.provenance.ember_proposal_id) {
      try {
        const emberProposal = await this.emberPort.getProposal(memory.provenance.ember_proposal_id)
        const emberNode = await this.traceEmberProvenance(emberProposal)
        eddaNode.children = [emberNode]
      } catch (error) {
        console.warn(`Could not fetch Ember proposal ${memory.provenance.ember_proposal_id}:`, error)
      }
    }

    return eddaNode
  }

  private async traceEmberProvenance(proposal: EmberProposal): Promise<ProvenanceNode> {
    const emberNode: ProvenanceNode = {
      layer: 'ember',
      id: proposal.id,
      type: 'proposal',
      timestamp: proposal.created_at,
      author: proposal.proposed_by_agent,
      confidence: String(proposal.confidence),
      children: [],
    }

    // Trace back to Kindling observations
    for (const kindlingId of proposal.source_observations) {
      try {
        const observation = await this.kindlingPort.getObservation(kindlingId)
        emberNode.children.push({
          layer: 'kindling',
          id: observation.id,
          type: observation.type,
          timestamp: observation.timestamp,
          author: observation.agent_id,
        })
      } catch (error) {
        console.warn(`Could not fetch Kindling observation ${kindlingId}:`, error)
      }
    }

    return emberNode
  }

  async findDerivedMemories(kindlingId: string): Promise<MemoryObject[]> {
    // Query memories where provenance includes this Kindling ID
    // (Requires indexing provenance.source_observations)
    const result = await this.query({
      filters: {
        // Custom filter for provenance
      },
    })
    return result.memories
  }

  async findProposals(memoryId: MemoryId): Promise<EmberProposal[]> {
    const memory = await this.memoryStorage.fetch(memoryId)
    if (!memory || !memory.provenance.ember_proposal_id) {
      return []
    }

    const proposal = await this.emberPort.getProposal(memory.provenance.ember_proposal_id)
    return [proposal]
  }
}
```

**File Structure:**
```
packages/edda-core/src/query/
├── provenance-service.ts
├── __tests__/
│   └── provenance-service.test.ts
```

**Tests:**
- Trace full provenance chain (Kindling → Ember → Edda)
- Handle missing Ember proposal
- Handle missing Kindling observation
- Find derived memories from Kindling ID
- Find proposals for memory

---

#### Epic 3.2: Provenance CLI
**Estimate:** 2 hours
**Owner:** TBD

**Description:**
Add CLI command to visualize provenance chains.

**Acceptance Criteria:**
- `anvil edda trace <memory-id>` - Show provenance tree
- Tree visualization with ASCII art
- Shows all layers and timestamps

**Implementation:**

```typescript
// packages/anvil/src/commands/edda/trace.ts

export const traceCommand: Command = {
  name: 'trace',
  description: 'Trace provenance chain for a memory',
  args: [
    { name: 'memory-id', required: true },
  ],
  async execute(context, args) {
    const memoryId = args['memory-id']

    console.log(`Tracing provenance for memory: ${memoryId}\n`)

    const tree = await context.edda.provenance.traceProvenance(memoryId)

    // Render tree with ASCII art
    renderProvenanceTree(tree, 0)
  },
}

function renderProvenanceTree(node: ProvenanceNode, depth: number) {
  const indent = '  '.repeat(depth)
  const layerColor = {
    edda: '\x1b[32m',      // Green
    ember: '\x1b[33m',     // Yellow
    kindling: '\x1b[34m',  // Blue
  }[node.layer]
  const reset = '\x1b[0m'

  console.log(
    `${indent}${layerColor}[${node.layer.toUpperCase()}]${reset} ` +
    `${node.id} (${node.type}) ` +
    `${node.author ? `by ${node.author}` : ''} ` +
    `at ${new Date(node.timestamp).toISOString()}`
  )

  if (node.children && node.children.length > 0) {
    for (const child of node.children) {
      renderProvenanceTree(child, depth + 1)
    }
  }
}
```

**Example Output:**
```
Tracing provenance for memory: MEM-2026-001

[EDDA] MEM-2026-001 (decision) by alice at 2026-01-19T10:30:00Z
  [EMBER] PROP-123 (proposal) by agent-gpt4 at 2026-01-19T10:15:00Z
    [KINDLING] OBS-456 (log_entry) by agent-gpt4 at 2026-01-19T10:00:00Z
    [KINDLING] OBS-457 (tool_use) by agent-gpt4 at 2026-01-19T10:05:00Z
```

**Tests:**
- Render single-layer tree
- Render multi-layer tree
- Handle missing children

---

### Epic 4: Memory Relationships
**Duration:** 1 day
**Priority:** P1 (Important)

#### Epic 4.1: Relationship Queries
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Query memories by relationships (supersedes, related_to, conflicts_with).

**Acceptance Criteria:**
- findSuperseded(memoryId) - Find what this memory supersedes
- findSupersededBy(memoryId) - Find what supersedes this memory
- findRelated(memoryId) - Find related memories
- Transitive closure for supersession chain

**Implementation:**

```typescript
// packages/edda-core/src/query/relationship-service.ts

export interface IRelationshipService {
  /**
   * Find memories superseded by this memory
   */
  findSuperseded(memoryId: MemoryId): Promise<MemoryObject[]>

  /**
   * Find memory that supersedes this one (if any)
   */
  findSupersededBy(memoryId: MemoryId): Promise<MemoryObject | null>

  /**
   * Find related memories
   */
  findRelated(memoryId: MemoryId): Promise<MemoryObject[]>

  /**
   * Find full supersession chain (transitive)
   */
  findSupersessionChain(memoryId: MemoryId): Promise<MemoryObject[]>
}

export class RelationshipService implements IRelationshipService {
  constructor(private storage: IMemoryStorage) {}

  async findSuperseded(memoryId: MemoryId): Promise<MemoryObject[]> {
    const memory = await this.storage.fetch(memoryId)
    if (!memory || !memory.lifecycle.supersedes) {
      return []
    }

    const superseded = await Promise.all(
      memory.lifecycle.supersedes.map(id => this.storage.fetch(id))
    )

    return superseded.filter(m => m !== null) as MemoryObject[]
  }

  async findSupersededBy(memoryId: MemoryId): Promise<MemoryObject | null> {
    // Query index for memories where supersedes includes memoryId
    // (Requires index on lifecycle.supersedes)
    const sql = `
      SELECT memory_id
      FROM memory_index
      WHERE json_array_contains(supersedes_json, ?)
      LIMIT 1
    `
    const row = await this.index.db.get(sql, memoryId)
    if (!row) return null

    return await this.storage.fetch(row.memory_id)
  }

  async findRelated(memoryId: MemoryId): Promise<MemoryObject[]> {
    const memory = await this.storage.fetch(memoryId)
    if (!memory || !memory.relations.related_to) {
      return []
    }

    const related = await Promise.all(
      memory.relations.related_to.map(id => this.storage.fetch(id))
    )

    return related.filter(m => m !== null) as MemoryObject[]
  }

  async findSupersessionChain(memoryId: MemoryId): Promise<MemoryObject[]> {
    const chain: MemoryObject[] = []
    let currentId: MemoryId | null = memoryId

    // Follow supersedes chain backwards
    while (currentId) {
      const memory = await this.storage.fetch(currentId)
      if (!memory) break

      chain.push(memory)

      // Move to next in chain
      const supersededBy = await this.findSupersededBy(currentId)
      currentId = supersededBy?.id || null
    }

    return chain
  }
}
```

**Tests:**
- Find superseded memories
- Find superseding memory
- Find related memories
- Transitive supersession chain
- Handle circular references gracefully

---

### Epic 5: Optional Semantic Search Interface
**Duration:** 1 day
**Priority:** P2 (Optional)

#### Epic 5.1: Semantic Search Port
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Define port interface for optional semantic search (implementation in Phase 7 or later).

**Acceptance Criteria:**
- ISemanticSearchService interface defined
- EmbeddingProvider port (Ollama, OpenAI, etc.)
- Query structure for semantic queries
- Falls back to FTS5 if not available

**Implementation:**

```typescript
// packages/edda-core/src/query/semantic-search-port.ts

export interface IEmbeddingProvider {
  /**
   * Generate embedding vector for text
   */
  embed(text: string): Promise<number[]>

  /**
   * Check if provider is available
   */
  isAvailable(): Promise<boolean>

  /**
   * Get embedding dimension
   */
  getDimension(): number
}

export interface ISemanticSearchService {
  /**
   * Semantic similarity search
   * Returns memories most similar to query
   */
  search(query: string, options?: SemanticSearchOptions): Promise<SemanticSearchResult[]>

  /**
   * Find similar memories to a given memory
   */
  findSimilar(memoryId: MemoryId, limit?: number): Promise<MemoryObject[]>

  /**
   * Check if semantic search is available
   */
  isAvailable(): Promise<boolean>
}

export interface SemanticSearchOptions {
  limit?: number
  minSimilarity?: number  // 0.0 - 1.0
  filters?: EddaQuery['filters']
}

export interface SemanticSearchResult {
  memory: MemoryObject
  similarity: number  // Cosine similarity 0.0 - 1.0
}

// Stub implementation that falls back to FTS5
export class SemanticSearchFallback implements ISemanticSearchService {
  constructor(private ftsSearch: IQueryService) {}

  async search(query: string, options?: SemanticSearchOptions): Promise<SemanticSearchResult[]> {
    // Fall back to FTS5
    console.warn('Semantic search not available, falling back to FTS5')
    const memories = await this.ftsSearch.search(query, options?.limit)

    return memories.map(memory => ({
      memory,
      similarity: 0.5, // Unknown similarity
    }))
  }

  async findSimilar(memoryId: MemoryId, limit?: number): Promise<MemoryObject[]> {
    console.warn('Semantic similarity not available')
    return []
  }

  async isAvailable(): Promise<boolean> {
    return false
  }
}
```

**File Structure:**
```
packages/edda-core/src/query/
├── semantic-search-port.ts
├── semantic-search-fallback.ts
└── __tests__/
    └── semantic-search-fallback.test.ts
```

**Tests:**
- Interface contract defined
- Fallback implementation uses FTS5
- isAvailable() returns false for fallback

**Note:** Full semantic search implementation deferred based on stakeholder decision (OPEN-QUESTIONS.md #2).

---

### Epic 6: CLI Commands
**Duration:** 1 day
**Priority:** P0 (Blocking)

#### Epic 6.1: Search & Query CLI
**Estimate:** 6 hours
**Owner:** TBD

**Description:**
Implement comprehensive search and query CLI commands.

**Acceptance Criteria:**
- `anvil edda search <query>` - Full-text search
- `anvil edda list --type <type> --status <status>` - Structured query
- `anvil edda show <memory-id> --include-provenance` - Show with relationships
- `anvil edda related <memory-id>` - Find related memories

**Implementation:**

```typescript
// packages/anvil/src/commands/edda/search.ts

export const searchCommand: Command = {
  name: 'search',
  description: 'Full-text search across memories',
  args: [
    { name: 'query', required: true, variadic: true },
  ],
  options: [
    { name: 'limit', default: 20 },
    { name: 'type', multiple: true },
    { name: 'status', multiple: true },
  ],
  async execute(context, args, options) {
    const query = args.query.join(' ')

    const result = await context.edda.query.query({
      search: { query },
      filters: {
        type: options.type,
        status: options.status,
      },
      pagination: {
        limit: options.limit,
        offset: 0,
      },
    })

    console.log(`Found ${result.total_count} memories (showing ${result.memories.length}):\n`)

    for (const memory of result.memories) {
      console.log(`${memory.id} [${memory.type}] ${memory.confidence}`)
      console.log(`  ${memory.statement.slice(0, 100)}...`)
      console.log(`  Tags: ${memory.tags.join(', ')}`)
      console.log()
    }

    console.log(`Query executed in ${result.query_metadata.execution_time_ms}ms`)
  },
}

export const listCommand: Command = {
  name: 'list',
  description: 'List memories with structured filters',
  options: [
    { name: 'type', multiple: true },
    { name: 'status', multiple: true },
    { name: 'confidence', multiple: true },
    { name: 'author' },
    { name: 'tags', multiple: true },
    { name: 'created-after' },
    { name: 'created-before' },
    { name: 'sort', default: 'created_at', choices: ['created_at', 'updated_at', 'confidence'] },
    { name: 'limit', default: 50 },
  ],
  async execute(context, args, options) {
    const result = await context.edda.query.query({
      filters: {
        type: options.type,
        status: options.status,
        confidence: options.confidence,
        author: options.author,
        tags: options.tags,
        created_after: options['created-after'],
        created_before: options['created-before'],
      },
      sort: {
        field: options.sort,
        direction: 'desc',
      },
      pagination: {
        limit: options.limit,
        offset: 0,
      },
    })

    console.log(`Found ${result.total_count} memories (showing ${result.memories.length}):\n`)
    console.log('ID              | TYPE       | STATUS     | CONFIDENCE | CREATED')
    console.log('─'.repeat(80))

    for (const memory of result.memories) {
      console.log(
        `${memory.id.padEnd(15)} | ` +
        `${memory.type.padEnd(10)} | ` +
        `${memory.status.padEnd(10)} | ` +
        `${memory.confidence.padEnd(10)} | ` +
        `${new Date(memory.authority.created_at).toISOString().slice(0, 10)}`
      )
    }
  },
}

export const relatedCommand: Command = {
  name: 'related',
  description: 'Find related memories',
  args: [
    { name: 'memory-id', required: true },
  ],
  async execute(context, args) {
    const memoryId = args['memory-id']

    const related = await context.edda.relationships.findRelated(memoryId)
    const superseded = await context.edda.relationships.findSuperseded(memoryId)
    const supersededBy = await context.edda.relationships.findSupersededBy(memoryId)

    console.log(`Related memories for: ${memoryId}\n`)

    if (related.length > 0) {
      console.log('Related:')
      for (const memory of related) {
        console.log(`  - ${memory.id}: ${memory.statement.slice(0, 60)}...`)
      }
      console.log()
    }

    if (superseded.length > 0) {
      console.log('Supersedes:')
      for (const memory of superseded) {
        console.log(`  - ${memory.id}: ${memory.statement.slice(0, 60)}...`)
      }
      console.log()
    }

    if (supersededBy) {
      console.log(`Superseded by: ${supersededBy.id}`)
    }
  },
}
```

**Tests:**
- Search command executes
- List with filters works
- Show with provenance
- Related memories command

---

### Epic 7: Integration & Testing
**Duration:** 1 day (end of phase)
**Priority:** P0 (Blocking)

#### Epic 7.1: Integration Tests
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
End-to-end integration tests for query system.

**Test Scenarios:**

```typescript
describe('Query & Retrieval Integration', () => {
  it('should find memories via full-text search', async () => {
    // Create test memories
    await createMemory({ statement: 'Always use OAuth for authentication' })
    await createMemory({ statement: 'Never use basic auth' })
    await createMemory({ statement: 'Prefer JWT tokens' })

    // Search
    const results = await edda.query.search('authentication')

    expect(results.length).toBeGreaterThan(0)
    expect(results.some(m => m.statement.includes('OAuth'))).toBe(true)
  })

  it('should filter by type and status', async () => {
    const results = await edda.query.query({
      filters: {
        type: ['warning'],
        status: ['active'],
      },
    })

    expect(results.memories.every(m => m.type === 'warning')).toBe(true)
    expect(results.memories.every(m => m.status === 'active')).toBe(true)
  })

  it('should trace full provenance chain', async () => {
    const memory = await createMemoryWithProvenance()

    const provenance = await edda.provenance.traceProvenance(memory.id)

    expect(provenance.layer).toBe('edda')
    expect(provenance.children).toHaveLength(1)
    expect(provenance.children[0].layer).toBe('ember')
    expect(provenance.children[0].children.length).toBeGreaterThan(0)
  })

  it('should perform queries in <50ms', async () => {
    // Seed 1000 memories
    await seedMemories(1000)

    const start = performance.now()
    await edda.query.query({
      filters: { type: ['decision'] },
      pagination: { limit: 50 },
    })
    const end = performance.now()

    expect(end - start).toBeLessThan(50)
  })
})
```

**Tests:**
- Full-text search works
- Structured queries work
- Provenance tracing works
- Relationship queries work
- Performance: <50ms for common queries
- 100% test coverage

---

## Timeline

### Week 1 (Days 1-5)
- **Day 1-2:** Epic 1 (FTS5 Indexing)
- **Day 3-5:** Epic 2 (Structured Query DSL)

### Week 2 (Days 6-10)
- **Day 6-7:** Epic 3 (Provenance Tracing)
- **Day 8:** Epic 4 (Memory Relationships)
- **Day 9:** Epic 5 (Semantic Search Port) + Epic 6 (CLI Commands)
- **Day 10:** Epic 7 (Integration & Testing)

---

## Deliverables

### Package Structure
```
packages/edda-core/src/query/
├── fts-indexer.ts
├── query-schema.ts
├── query-service.ts
├── provenance-service.ts
├── relationship-service.ts
├── semantic-search-port.ts
├── semantic-search-fallback.ts
└── __tests__/
    ├── fts-indexer.test.ts
    ├── query-service.test.ts
    ├── provenance-service.test.ts
    ├── relationship-service.test.ts
    └── integration/
        └── query-retrieval.integration.test.ts

packages/anvil/src/commands/edda/
├── search.ts
├── list.ts
├── trace.ts
└── related.ts
```

### SQLite Schema Enhancements
```sql
-- FTS5 virtual table
CREATE VIRTUAL TABLE memory_fts USING fts5(...);

-- Additional indexes for query performance
CREATE INDEX idx_memory_type_status ON memory_index(type, status);
CREATE INDEX idx_memory_created_at ON memory_index(created_at);
CREATE INDEX idx_memory_author ON memory_index(author);
```

### Documentation
- Query DSL specification
- FTS5 query syntax guide
- API documentation for query services
- CLI usage examples

### Tests
- Unit tests: 40+ tests
- Integration tests: 10+ scenarios
- Performance tests: <50ms query time
- Test coverage: 100%

---

## Success Metrics

### Functional
- ✅ Full-text search finds relevant memories
- ✅ Structured queries filter correctly
- ✅ Provenance tracing complete
- ✅ Relationship queries work
- ✅ CLI commands operational

### Performance
- ✅ Query response time: <50ms (95th percentile)
- ✅ FTS5 indexing: <5ms per memory
- ✅ Handles 10,000+ memories efficiently

### Quality
- ✅ 100% test coverage
- ✅ All edge cases handled
- ✅ Graceful degradation when semantic search unavailable
- ✅ Clear error messages

---

## Risks & Mitigation

### Risk 1: FTS5 Not Sufficient for Technical Content
**Probability:** Low
**Impact:** Medium

**Mitigation:**
- FTS5 with porter stemming works well for code/tech terms
- Semantic search port ready if needed later
- Can tune FTS5 tokenizer if needed

### Risk 2: Performance Degradation at Scale
**Probability:** Medium
**Impact:** Medium

**Mitigation:**
- Proper SQLite indexing on all query fields
- Query plan analysis during development
- Performance regression tests
- Can add caching layer if needed

### Risk 3: Provenance Links Break (Ember/Kindling data deleted)
**Probability:** Medium
**Impact:** Low

**Mitigation:**
- Graceful handling of missing provenance links
- Log warnings but don't fail queries
- Document retention policies for Ember/Kindling

---

## Dependencies

### Upstream (Must Complete First)
- Phase 0: Foundation (memory storage, SQLite index)
- Phase 2: Authority & Trust (authorization for queries)

### Downstream (Blocked By This Phase)
- Phase 4: Enforcement Hooks (uses query service)
- Phase 7: Meta-Capabilities (uses query for analytics)

---

## Open Questions

### Q1: Semantic Search Priority (from OPEN-QUESTIONS.md)
**Status:** 🟡 Pending Stakeholder Decision
**Recommended:** Optional in Phase 3 (port defined, implementation deferred)

**Impact on Phase 3:**
- Port only: 2 weeks (as planned)
- Full implementation: +2 weeks (4 weeks total)

**Decision Required By:** Before Phase 3 starts

---

## Next Steps

1. ✅ Complete Phase 0 (Foundation)
2. ✅ Complete Phase 1 (Promotion Pipeline)
3. ✅ Complete Phase 2 (Authority & Trust)
4. **Review this APS document** with team
5. **Decide on semantic search** (port only vs full implementation)
6. **Assign owners** to epics and tasks
7. **Kick off Phase 3** implementation

---

**Document Version:** 1.0
**Last Updated:** 2026-01-19
**Status:** Ready for Review
**Estimated Completion:** 2 weeks after Phase 2 completion
