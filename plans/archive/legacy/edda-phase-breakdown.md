# Edda Implementation Phase Breakdown

**Purpose:** Detailed task breakdown for APS planning
**Related:** `/docs/architecture/edda-system-architecture.md`
**Status:** Draft for APS Planning

---

## Overview

This document breaks down the Edda system implementation into 7 phases (0-6), with Phase 7 being optional for v1. Each phase includes:
- Concrete tasks with effort estimates
- Dependencies and blockers
- Success criteria and test requirements
- Risk assessment

**Total Estimated Duration:** 16-19 weeks (excluding Phase 7)
**Critical Path:** Phase 0 → 1 → 2 → 4 → 6 (12-14 weeks minimum)

---

## Phase 0: Foundation (Weeks 1-2)

**Goal:** Core data models, storage, and CRUD operations

### Tasks

#### 0.1 Memory Storage Layer (3 days)
- [ ] Implement `GitStorageAdapter` for YAML/JSON persistence
- [ ] Create repository structure (`.edda/memories/`, `.edda/versions/`, `.edda/audit/`)
- [ ] Implement atomic write operations with git commits
- [ ] Add file locking for concurrent access
- [ ] Test: parallel writes, conflict resolution

**Blockers:** None
**Risk:** Medium (git operations can be complex)

#### 0.2 Memory Object Implementation (2 days)
- [ ] Implement `MemoryObjectExtended` class
- [ ] Add validation using Zod schemas
- [ ] Implement ID generation (`EDDA-M-{type}-{ulid}`)
- [ ] Add type-specific metadata helpers
- [ ] Test: all 6 memory types, validation edge cases

**Blockers:** None
**Risk:** Low

#### 0.3 CRUD Operations (2 days)
- [ ] Implement `create`, `read`, `update`, `delete` operations
- [ ] Add query filtering (by type, status, scope, tags)
- [ ] Implement pagination
- [ ] Add error handling and logging
- [ ] Test: CRUD operations, error cases, pagination

**Blockers:** 0.1, 0.2
**Risk:** Low

#### 0.4 Index Layer (SQLite) (2 days)
- [ ] Create SQLite schema for fast queries
- [ ] Implement indexing on create/update/delete
- [ ] Add FTS5 full-text search
- [ ] Sync mechanism (git → SQLite)
- [ ] Test: query performance, index consistency

**Blockers:** 0.1, 0.2
**Risk:** Low

#### 0.5 Core Port Implementation (1 day)
- [ ] Extend `IEddaPort` with new operations
- [ ] Implement port adapter
- [ ] Wire up storage + index layers
- [ ] Test: port contract compliance

**Blockers:** 0.3, 0.4
**Risk:** Low

### Deliverables
- `packages/edda-core` package
- Git storage adapter
- SQLite index layer
- Extended port implementation
- 90% test coverage

### Success Criteria
- [ ] Can create, read, update, delete memories
- [ ] Git history shows clean commits per operation
- [ ] Query performance <100ms for typical queries
- [ ] All 6 memory types supported
- [ ] Validation catches all invalid inputs

---

## Phase 1: Promotion Pipeline (Weeks 3-5)

**Goal:** Ember → Edda promotion workflow with human review

### Tasks

#### 1.1 Promotion Request System (2 days)
- [ ] Implement `PromotionRequest` model
- [ ] Add state machine (awaiting → under_review → approved/rejected)
- [ ] Storage for promotion requests (git + SQLite)
- [ ] Lifecycle operations (create, review, resolve)
- [ ] Test: state transitions, invalid transitions

**Blockers:** Phase 0 complete
**Risk:** Low

#### 1.2 Type Mapping (Ember → Edda) (1 day)
- [ ] Implement proposal type → memory type mapping
- [ ] Confidence transformation (numeric → semantic)
- [ ] Scope inference logic
- [ ] Enforcement mode recommendation
- [ ] Test: all proposal types, edge cases

**Blockers:** 1.1, Ember port available
**Risk:** Low

#### 1.3 Diff Generation (2 days)
- [ ] Implement `PromotionDiff` generator
- [ ] Show proposal → memory transformation
- [ ] Detect conflicts with existing memories
- [ ] Generate human-readable summary
- [ ] Test: various proposal types, conflicts

**Blockers:** 1.2
**Risk:** Medium (conflict detection is complex)

#### 1.4 Review Interface (CLI) (3 days)
- [ ] `anvil edda proposals` - list pending reviews
- [ ] `anvil edda promote <id>` - start review flow
- [ ] Interactive diff display (colored, formatted)
- [ ] Modification editor (optional edits during review)
- [ ] Approval/rejection workflow with rationale
- [ ] Test: full review workflows, edge cases

**Blockers:** 1.3
**Risk:** Medium (UX is critical)

#### 1.5 Provenance Validation (1 day)
- [ ] Implement provenance chain validator
- [ ] Check Kindling → Ember → Edda integrity
- [ ] Validate observation references exist
- [ ] Test: valid chains, broken chains, missing refs

**Blockers:** 1.1, Kindling/Ember ports
**Risk:** Low

#### 1.6 Rejection Recording (1 day)
- [ ] Implement `RejectionRecord` storage
- [ ] Capture rejection rationale and category
- [ ] Generate feedback for Ember (confidence adjustment)
- [ ] Test: rejection flows, feedback generation

**Blockers:** 1.1
**Risk:** Low

#### 1.7 Versioning System (2 days)
- [ ] Implement `MemoryVersion` tracking
- [ ] Store version snapshots
- [ ] Diff generation between versions
- [ ] Version history queries
- [ ] Test: multiple updates, version retrieval

**Blockers:** Phase 0 complete
**Risk:** Low

#### 1.8 Integration Testing (1 day)
- [ ] End-to-end promotion flows
- [ ] Performance testing (promote 100 proposals)
- [ ] Error recovery testing
- [ ] Test: real-world scenarios

**Blockers:** 1.4, 1.5, 1.6, 1.7
**Risk:** Low

### Deliverables
- Promotion request system
- CLI review interface (`anvil edda proposals`, `promote`, `reject`)
- Type mapping logic
- Provenance validation
- Rejection tracking
- Version history

### Success Criteria
- [ ] Can promote Ember proposal to Edda memory
- [ ] Review UI is intuitive and informative
- [ ] Provenance chain is validated and preserved
- [ ] Rejections create learning signals for Ember
- [ ] All operations are versioned and auditable

---

## Phase 2: Authority & Trust (Weeks 6-7)

**Goal:** RBAC, permissions, audit trail

### Tasks

#### 2.1 Principal & Role System (2 days)
- [ ] Implement `Principal` model
- [ ] Define default roles (org_admin, team_lead, contributor, agent)
- [ ] Role storage and management
- [ ] Principal resolution (string → Principal)
- [ ] Test: role creation, assignment, resolution

**Blockers:** Phase 0 complete
**Risk:** Low

#### 2.2 Permission System (2 days)
- [ ] Implement permission checks
- [ ] RBAC enforcement middleware
- [ ] Scope-based access control
- [ ] Permission inheritance
- [ ] Test: all permission types, denial cases

**Blockers:** 2.1
**Risk:** Medium (security-critical)

#### 2.3 Authority Metadata (1 day)
- [ ] Add authority fields to memories
- [ ] Owner and reviewer assignment
- [ ] Visibility enforcement (public/team/private)
- [ ] Test: access control, visibility rules

**Blockers:** 2.2
**Risk:** Low

#### 2.4 Agent Trust Profiles (2 days)
- [ ] Implement `AgentTrustProfile` storage
- [ ] Track proposal success rates
- [ ] Calculate trust scores
- [ ] Confidence adjustment based on trust
- [ ] Test: trust calculation, score updates

**Blockers:** 2.1, Phase 1 complete
**Risk:** Low

#### 2.5 Audit Trail (2 days)
- [ ] Implement `AuditEntry` model
- [ ] Automatic audit logging for all operations
- [ ] Audit query interface
- [ ] Retention policy (configurable)
- [ ] Test: all operations logged, query correctness

**Blockers:** 2.1
**Risk:** Low

#### 2.6 CLI Authority Commands (1 day)
- [ ] `anvil edda roles list`
- [ ] `anvil edda roles assign <principal> <role>`
- [ ] `anvil edda audit <filters>`
- [ ] `anvil edda trust <agent-id>`
- [ ] Test: CLI workflows

**Blockers:** 2.2, 2.5
**Risk:** Low

### Deliverables
- RBAC system
- Agent trust profiles
- Audit trail
- CLI authority management
- Security middleware

### Success Criteria
- [ ] Role-based access control enforced
- [ ] Agents have trust scores based on performance
- [ ] All operations are audited
- [ ] Visibility rules are enforced
- [ ] Cannot bypass permission checks

---

## Phase 3: Query & Retrieval (Weeks 8-9)

**Goal:** Finding and understanding memories

### Tasks

#### 3.1 Query Interface (2 days)
- [ ] Implement `EddaQuery` builder
- [ ] Filter by type, status, scope, tags, confidence
- [ ] Temporal filters (created_after, etc.)
- [ ] Pagination with cursors
- [ ] Test: all filter combinations, pagination

**Blockers:** Phase 0 complete
**Risk:** Low

#### 3.2 Full-Text Search (1 day)
- [ ] Implement FTS5 integration
- [ ] Search statement + context fields
- [ ] Ranking by relevance
- [ ] Highlighting matches
- [ ] Test: various queries, ranking quality

**Blockers:** Phase 0 (0.4 SQLite index)
**Risk:** Low

#### 3.3 Semantic Search (Optional) (3 days)
- [ ] Integrate embedding service (Ollama/OpenAI)
- [ ] Generate embeddings on memory creation
- [ ] Vector similarity search
- [ ] Hybrid search (keyword + semantic)
- [ ] Test: semantic relevance, performance

**Blockers:** 3.2
**Risk:** High (external dependency, performance)

#### 3.4 Conflict Detection (2 days)
- [ ] Implement conflict detector
- [ ] Detect contradictions (statement analysis)
- [ ] Detect duplications (similarity check)
- [ ] Detect supersession candidates
- [ ] Test: known conflicts, edge cases

**Blockers:** 3.2
**Risk:** Medium (requires heuristics/ML)

#### 3.5 Temporal Queries (1 day)
- [ ] "What was true at time T" queries
- [ ] Version snapshot retrieval
- [ ] Time-range queries
- [ ] Test: historical queries, edge cases

**Blockers:** Phase 1 (1.7 versioning)
**Risk:** Low

#### 3.6 Provenance Tracing (2 days)
- [ ] Implement provenance query
- [ ] Trace memory → proposal → observations
- [ ] Generate provenance graph
- [ ] Evolution chain visualization
- [ ] Test: full chain tracing, visualization

**Blockers:** Phase 1 (1.5 provenance)
**Risk:** Low

#### 3.7 CLI Query Commands (1 day)
- [ ] `anvil edda list <filters>`
- [ ] `anvil edda search "<query>"`
- [ ] `anvil edda show <id>`
- [ ] `anvil edda trace <id>`
- [ ] `anvil edda conflicts`
- [ ] Test: CLI workflows

**Blockers:** 3.1-3.6
**Risk:** Low

### Deliverables
- Query interface with filters
- Full-text search
- Semantic search (optional)
- Conflict detection
- Temporal queries
- Provenance tracing
- CLI query commands

### Success Criteria
- [ ] Can find memories by any attribute
- [ ] Full-text search returns relevant results
- [ ] Conflict detection identifies contradictions
- [ ] Can query historical state
- [ ] Provenance chain is traceable
- [ ] Query performance <200ms (typical)

---

## Phase 4: Enforcement Hooks (Weeks 10-12)

**Goal:** Edda guides and blocks actions

### Tasks

#### 4.1 Hook Framework (3 days)
- [ ] Implement `EnforcementHook` model
- [ ] Hook registration and storage
- [ ] Trigger condition evaluation
- [ ] Memory matcher (find applicable memories)
- [ ] Hook execution engine
- [ ] Test: hook lifecycle, trigger matching

**Blockers:** Phase 0 complete
**Risk:** Medium (complex logic)

#### 4.2 Pre-Execution Checks (2 days)
- [ ] Implement pre-execution check API
- [ ] Action context extraction
- [ ] Policy violation detection
- [ ] Constraint checking
- [ ] Test: various action types, violations

**Blockers:** 4.1, Phase 2 (authority)
**Risk:** Low

#### 4.3 Enforcement Modes (2 days)
- [ ] Implement advisory, warning, blocking, audit_only modes
- [ ] Override mechanism (with justification)
- [ ] Approval-required workflow
- [ ] Test: all modes, overrides

**Blockers:** 4.2
**Risk:** Low

#### 4.4 Anvil Integration Points (3 days)
- [ ] Hook into Anvil gate system
- [ ] Pre-action hooks (before tool execution)
- [ ] File change hooks (before write)
- [ ] Command hooks (before shell execution)
- [ ] Test: integration with Anvil workflows

**Blockers:** 4.3, Anvil gate system
**Risk:** High (requires Anvil changes)

#### 4.5 Guidance System (2 days)
- [ ] Implement guidance request API
- [ ] Context-based memory retrieval
- [ ] Relevance scoring
- [ ] Suggestion generation
- [ ] Test: various contexts, relevance quality

**Blockers:** Phase 3 (query system)
**Risk:** Low

#### 4.6 CLI Enforcement Commands (1 day)
- [ ] `anvil edda hooks list`
- [ ] `anvil edda hooks create <config>`
- [ ] `anvil edda check <action>` (dry-run)
- [ ] Test: CLI workflows

**Blockers:** 4.1-4.5
**Risk:** Low

### Deliverables
- Hook framework
- Pre-execution checks
- Enforcement modes (advisory/warning/blocking)
- Anvil integration
- Guidance system
- CLI commands

### Success Criteria
- [ ] Hooks can block/warn/advise on actions
- [ ] Integrated with Anvil execution pipeline
- [ ] Override mechanism works with audit
- [ ] Guidance surfaces relevant memories
- [ ] Performance: <50ms overhead per action

---

## Phase 5: Lifecycle Management (Weeks 13-14)

**Goal:** Change management and decay

### Tasks

#### 5.1 Deprecation Workflow (2 days)
- [ ] Implement `DeprecationRequest` model
- [ ] Deprecation proposal and approval
- [ ] Timeline tracking (deprecation → retirement)
- [ ] Impact assessment
- [ ] Test: deprecation workflows

**Blockers:** Phase 1, 2 (promotion, authority)
**Risk:** Low

#### 5.2 Review Scheduling (2 days)
- [ ] Implement `ReviewSchedule` system
- [ ] Time-based review triggers
- [ ] Event-based review triggers (violations, etc.)
- [ ] Review notifications
- [ ] Test: schedule triggers, notifications

**Blockers:** Phase 0 complete
**Risk:** Low

#### 5.3 Supersession Handling (2 days)
- [ ] Implement supersession workflow
- [ ] Atomic old→retired + new→active
- [ ] Reference updates (memories referencing old)
- [ ] Enforcement hook migration
- [ ] Test: supersession flows, reference updates

**Blockers:** Phase 1 (versioning), Phase 4 (hooks)
**Risk:** Medium (atomic updates critical)

#### 5.4 Staleness Detection (1 day)
- [ ] Implement staleness scoring
- [ ] Factor calculation (time, usage, contradictions)
- [ ] Automatic staleness reports
- [ ] Test: staleness calculation, thresholds

**Blockers:** Phase 3 (queries), Phase 4 (usage tracking)
**Risk:** Low

#### 5.5 Forgetting Engine (1 day)
- [ ] Implement retention policies
- [ ] Automatic retirement candidates
- [ ] Protected memories (never auto-retire)
- [ ] Human approval for auto-retirement
- [ ] Test: forgetting logic, protections

**Blockers:** 5.1, 5.4
**Risk:** Low

#### 5.6 Historical Queries (1 day)
- [ ] Extend query system for retired memories
- [ ] "Active during" temporal queries
- [ ] Retirement reason display
- [ ] Test: historical query correctness

**Blockers:** Phase 3 (queries), 5.1
**Risk:** Low

#### 5.7 CLI Lifecycle Commands (1 day)
- [ ] `anvil edda retire <id> --reason="..."`
- [ ] `anvil edda supersede <old-id> <new-id>`
- [ ] `anvil edda review-due`
- [ ] `anvil edda stale`
- [ ] Test: CLI workflows

**Blockers:** 5.1-5.6
**Risk:** Low

### Deliverables
- Deprecation workflow
- Review scheduling
- Supersession handling
- Staleness detection
- Forgetting engine
- CLI lifecycle commands

### Success Criteria
- [ ] Can deprecate and retire memories
- [ ] Reviews are scheduled and triggered
- [ ] Supersession preserves lineage
- [ ] Stale memories are identified
- [ ] Forgetting is controlled and auditable

---

## Phase 6: Interop & Export (Weeks 15-16)

**Goal:** Edda as platform - APIs, exports, integrations

### Tasks

#### 6.1 REST API (3 days)
- [ ] Implement Express server
- [ ] All CRUD endpoints
- [ ] Query endpoints (search, filter)
- [ ] Promotion endpoints
- [ ] Lifecycle endpoints
- [ ] Test: API contract compliance, security

**Blockers:** All previous phases complete
**Risk:** Low

#### 6.2 OpenAPI Specification (1 day)
- [ ] Generate OpenAPI 3.0 spec
- [ ] API documentation site
- [ ] Request/response examples
- [ ] Test: spec validity

**Blockers:** 6.1
**Risk:** Low

#### 6.3 Export/Import (2 days)
- [ ] Implement memory export (JSON/YAML)
- [ ] Metadata preservation
- [ ] Import with validation
- [ ] Conflict resolution on import
- [ ] Test: round-trip export/import

**Blockers:** Phase 0, 3 (storage, queries)
**Risk:** Low

#### 6.4 Markdown Rendering (1 day)
- [ ] Implement memory → markdown converter
- [ ] Structured sections (statement, context, provenance)
- [ ] ADR-style formatting
- [ ] Test: all memory types, formatting quality

**Blockers:** Phase 0 complete
**Risk:** Low

#### 6.5 Tool Projections (2 days)
- [ ] Anvil projection (gates, constraints)
- [ ] CI projection (pre-commit, PR rules)
- [ ] Documentation projection (ADRs, guides)
- [ ] Test: projection correctness

**Blockers:** Phase 4 (hooks), 6.4 (markdown)
**Risk:** Medium (custom per tool)

#### 6.6 CLI Export Commands (1 day)
- [ ] `anvil edda export --format=json|yaml|markdown`
- [ ] `anvil edda import <file>`
- [ ] `anvil edda stats`
- [ ] Test: CLI workflows

**Blockers:** 6.3, 6.4
**Risk:** Low

### Deliverables
- REST API server
- OpenAPI documentation
- Export/import utilities
- Markdown rendering
- Tool projections
- CLI export commands

### Success Criteria
- [ ] API is documented and functional
- [ ] Can export/import all memories
- [ ] Markdown output is human-readable
- [ ] Projections work with target tools
- [ ] API performance: p95 <500ms

---

## Phase 7: Meta-Capabilities (Weeks 17-19) [OPTIONAL for v1]

**Goal:** Organisational intelligence

### Tasks

#### 7.1 Contradiction Detection (2 days)
- [ ] Implement semantic contradiction detector
- [ ] Detect direct contradictions (same scope, opposing statements)
- [ ] Detect implicit contradictions (inferred)
- [ ] Resolution suggestions
- [ ] Test: known contradictions, false positives

**Blockers:** Phase 3 (semantic search)
**Risk:** High (requires ML/semantic understanding)

#### 7.2 Knowledge Graph (3 days)
- [ ] Build graph from memory relationships
- [ ] Calculate graph metrics (centrality, clustering)
- [ ] Identify critical paths
- [ ] Graph visualization (Graphviz/D3)
- [ ] Test: graph correctness, metrics

**Blockers:** Phase 1, 3, 5 (versioning, queries, supersession)
**Risk:** Medium (complex graph algorithms)

#### 7.3 Impact Analysis (2 days)
- [ ] Implement impact analyser
- [ ] Trace dependencies (what depends on this memory)
- [ ] Calculate risk scores
- [ ] Generate recommendations
- [ ] Test: various change scenarios, risk assessment

**Blockers:** 7.2 (knowledge graph)
**Risk:** Medium

#### 7.4 Learning Signals (2 days)
- [ ] Track rejection patterns
- [ ] Analyse proposal quality trends
- [ ] Generate tuning recommendations (for Ember)
- [ ] Quality dashboard
- [ ] Test: pattern detection, recommendations

**Blockers:** Phase 1 (rejections), Phase 2 (agent trust)
**Risk:** Low

#### 7.5 Cultural Drift Detection (3 days)
- [ ] Implement drift detector
- [ ] Compare policy (Edda) vs practice (observations)
- [ ] Detect violations, overrides, ignored policies
- [ ] Generate drift reports
- [ ] Test: known drift scenarios

**Blockers:** Phase 4 (enforcement), Kindling observations
**Risk:** High (requires pattern recognition)

#### 7.6 CLI Meta Commands (1 day)
- [ ] `anvil edda analyse <memory-id>` (impact analysis)
- [ ] `anvil edda graph [--cluster] [--critical-paths]`
- [ ] `anvil edda contradictions`
- [ ] `anvil edda drift-report`
- [ ] `anvil edda learning-signals`
- [ ] Test: CLI workflows

**Blockers:** 7.1-7.5
**Risk:** Low

### Deliverables
- Contradiction detection
- Knowledge graph
- Impact analysis
- Learning signals tracker
- Cultural drift detection
- CLI analysis commands

### Success Criteria
- [ ] Contradictions are detected with <10% false positive rate
- [ ] Knowledge graph accurately represents relationships
- [ ] Impact analysis identifies downstream effects
- [ ] Learning signals improve Ember over time
- [ ] Cultural drift is measurable

---

## Cross-Phase Concerns

### Testing Strategy

**Unit Tests (all phases):**
- 80%+ coverage for core logic
- Test framework: Vitest
- Mock external dependencies

**Integration Tests (per phase):**
- Cross-component workflows
- Database transactions
- Error handling

**E2E Tests (Phase 6+):**
- Full user workflows via CLI
- Performance benchmarks
- Data consistency checks

**Load Tests (Phase 6):**
- 1000 memories, query performance
- Concurrent access (10 users)
- Large export/import

### Documentation

**Per Phase:**
- Architecture decision records (ADRs)
- API documentation (JSDoc)
- User guides (CLI usage)
- Developer guides (extension points)

**Final (Phase 6):**
- Comprehensive user manual
- API reference
- Deployment guide
- Troubleshooting guide

### Performance Targets

| Operation | Target | Measurement |
|-----------|--------|-------------|
| Memory creation | <100ms | p95 |
| Memory query | <200ms | p95 |
| Promotion review | <500ms | p95 |
| Pre-execution check | <50ms | p95 |
| Full-text search | <300ms | p95 |
| Semantic search | <1000ms | p95 |
| Export (1000 memories) | <5s | p95 |
| API response | <500ms | p95 |

### Security Requirements

**All Phases:**
- Input validation (Zod schemas)
- SQL injection prevention (parameterised queries)
- Path traversal prevention (file operations)
- Audit logging (all mutations)

**Phase 2+:**
- Authentication integration
- Authorization checks
- Role-based access control
- API rate limiting (Phase 6)

---

## Risk Assessment

### High-Risk Items

1. **Anvil Integration (Phase 4.4)** - Requires changes to Anvil core
   - **Mitigation:** Early prototype, design review with Anvil team

2. **Semantic Search (Phase 3.3)** - External dependency, performance
   - **Mitigation:** Optional feature, can skip for v1

3. **Contradiction Detection (Phase 7.1)** - Complex ML problem
   - **Mitigation:** Optional for v1, use heuristics initially

4. **Cultural Drift (Phase 7.5)** - Requires sophisticated pattern recognition
   - **Mitigation:** Optional for v1, start with simple metrics

### Medium-Risk Items

1. **Conflict Detection (Phase 3.4)** - Heuristics may have false positives
   - **Mitigation:** Tune thresholds, allow user feedback

2. **Supersession (Phase 5.3)** - Atomicity critical
   - **Mitigation:** Transaction support, thorough testing

3. **Tool Projections (Phase 6.5)** - Each tool has unique requirements
   - **Mitigation:** Start with Anvil only, expand later

---

## Dependencies

### External Dependencies

- **Git:** Version control for storage
- **SQLite:** Index layer, FTS5
- **Node.js:** Runtime (v18+)
- **Zod:** Schema validation
- **Vitest:** Testing
- **Commander.js:** CLI framework

### Optional Dependencies

- **Ollama/OpenAI:** Semantic search (Phase 3.3)
- **Graphviz:** Graph visualization (Phase 7.2)
- **Express:** REST API (Phase 6.1)

### Internal Dependencies

- **Anvil Gate System:** Enforcement hooks (Phase 4.4)
- **Ember Port:** Promotion pipeline (Phase 1)
- **Kindling Port:** Provenance validation (Phase 1.5)

---

## Resource Requirements

### Development Team

**Minimum Team:**
- 1 Senior Engineer (architecture, critical path)
- 1 Mid-Level Engineer (features, testing)
- 1 Part-Time UX Designer (CLI/API design)

**Optimal Team:**
- 1 Tech Lead (architecture, reviews)
- 2 Senior Engineers (phases in parallel)
- 1 Mid-Level Engineer (testing, docs)
- 1 Part-Time UX Designer

### Infrastructure

**Development:**
- Git repository
- CI/CD pipeline (GitHub Actions)
- Test environment
- Documentation site hosting

**Production (v1):**
- Git storage (local or remote)
- SQLite (local)
- Optional: API server (cloud)

---

## Success Metrics

### Phase-Level Metrics

**Phase 0:** Test coverage >80%, all CRUD operations functional
**Phase 1:** Promotion approval rate >60%, review time <5 min
**Phase 2:** Permission checks enforced 100%, audit coverage 100%
**Phase 3:** Query performance <200ms p95, search relevance >80%
**Phase 4:** Hook execution <50ms p95, no false blocks
**Phase 5:** Staleness detection accuracy >85%, zero accidental deletions
**Phase 6:** API uptime >99%, export round-trip success 100%
**Phase 7:** Contradiction detection F1 >0.8, drift detection useful

### Product-Level Metrics

**Adoption:**
- Memories created per week
- Teams actively using Edda
- Agent proposal volume

**Quality:**
- Promotion approval rate (target: >70%)
- Contradiction rate (target: <5%)
- Staleness score distribution

**Impact:**
- Violations prevented by enforcement
- Onboarding time reduction
- Decision consistency

---

## Next Steps

1. **Review & Prioritize** - Stakeholder review, adjust phases
2. **Resource Allocation** - Assign team, secure budget
3. **Detailed APS** - Create APS documents for approved phases
4. **Kickoff** - Phase 0 development start
5. **Weekly Checkpoints** - Progress reviews, risk assessment

---

**Document Owner:** Architecture Team
**Last Updated:** 2026-01-19
**Next Review:** After Phase 0 completion
