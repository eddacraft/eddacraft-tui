# Edda System Architecture - Document Index

**Version:** 1.0.0
**Status:** Ready for APS Planning
**Date:** 2026-01-19

---

## Purpose

This index organizes all Edda architecture and planning documents for the APS planning process. The documents provide comprehensive specifications for implementing Edda as the authoritative memory layer for Anvil.

---

## Document Structure

```
docs/architecture/
├── EDDA-ARCHITECTURE-INDEX.md         ← You are here
├── edda-system-architecture.md        ← Master architecture
├── edda-component-dependencies.md     ← Component relationships
│
docs/specs/
├── edda-authority-trust.md            ← Authority & trust model
├── edda-enforcement-hooks.md          ← Enforcement system
├── edda-api-contracts.md              ← API specifications
│
packages/edda-stack/src/contracts/
├── edda-extended.ts                   ← TypeScript contracts
│
plans/
└── edda-phase-breakdown.md            ← Implementation phases
```

---

## Reading Guide

### For Product/Business Stakeholders

Start with:
1. **[edda-system-architecture.md](./edda-system-architecture.md)** - Executive summary and system overview
   - Read sections: Executive Summary, System Overview, Implementation Phases
2. **[edda-phase-breakdown.md](../../plans/edda-phase-breakdown.md)** - Timelines and resource requirements
   - Focus on: Phase durations, success metrics, risk assessment

**Key Questions Answered:**
- What is Edda and why do we need it?
- How long will it take to build?
- What are the risks?
- What will it cost (in terms of resources)?

---

### For Engineering Leadership

Read in order:
1. **[edda-system-architecture.md](./edda-system-architecture.md)** - Full system design
2. **[edda-component-dependencies.md](./edda-component-dependencies.md)** - Technical dependencies
3. **[edda-phase-breakdown.md](../../plans/edda-phase-breakdown.md)** - Detailed task breakdown

**Key Questions Answered:**
- What are the technical risks and dependencies?
- Which phases can run in parallel?
- What external integrations are required?
- What's the critical path to MVP?

---

### For Implementation Teams

Your roadmap:
1. **[edda-component-dependencies.md](./edda-component-dependencies.md)** - Understand component relationships
2. **Phase-specific specs:**
   - Phase 0-1: [edda-system-architecture.md](./edda-system-architecture.md) sections 1-2
   - Phase 2: [edda-authority-trust.md](../specs/edda-authority-trust.md)
   - Phase 3: [edda-system-architecture.md](./edda-system-architecture.md) section 4
   - Phase 4: [edda-enforcement-hooks.md](../specs/edda-enforcement-hooks.md)
   - Phase 5: [edda-system-architecture.md](./edda-system-architecture.md) section 6
   - Phase 6: [edda-api-contracts.md](../specs/edda-api-contracts.md)
3. **[edda-extended.ts](../../packages/edda-stack/src/contracts/edda-extended.ts)** - TypeScript contracts

**Key Artifacts:**
- Component interfaces
- Data schemas
- API contracts
- Test requirements
- Success criteria

---

## Document Summaries

### 1. Master Architecture Document

**File:** [docs/architecture/edda-system-architecture.md](./edda-system-architecture.md)

**Contents:**
- Executive summary and philosophy
- 10 capability domains mapped to components:
  1. Memory Objects (typed knowledge schema)
  2. Promotion Pipeline (Ember → Edda workflow)
  3. Authority & Trust (RBAC, agent trust)
  4. Query & Retrieval (semantic search, provenance)
  5. Enforcement Hooks (pre-execution checks, guidance)
  6. Change Management (deprecation, staleness, forgetting)
  7. Agent Interaction (proposal, citation, feedback)
  8. Interop & Export (API, projections)
  9. UX & Experience (CLI, visual cues, narrative views)
  10. Meta-Capabilities (contradiction detection, knowledge graph)

**Key Sections:**
- Memory object schema with type-specific metadata
- Promotion workflow (human-in-the-loop)
- Enforcement modes (advisory, warning, blocking)
- Phase breakdown (0-7) with dependencies

**Length:** ~300 lines of specification

---

### 2. Component Dependencies Map

**File:** [docs/architecture/edda-component-dependencies.md](./edda-component-dependencies.md)

**Contents:**
- Visual component hierarchy
- Dependency matrix (what depends on what)
- Critical paths (MVP vs full feature set)
- External integration points
- Data flow diagrams
- Package structure
- Parallel development opportunities
- Risk dependencies

**Key Diagrams:**
- Component hierarchy (7 layers)
- Dependency graph with build order
- Critical path visualization
- Data flow for promotion, enforcement, query

**Use Cases:**
- Determine build order
- Identify blocking dependencies
- Plan parallel work streams
- Assess integration risks

---

### 3. Authority & Trust Specification

**File:** [docs/specs/edda-authority-trust.md](../specs/edda-authority-trust.md)

**Contents:**
- Principal system (human, agent, team, system)
- Authority levels (5 tiers: system → readonly)
- Role-based access control (RBAC)
- Permission types (10 permissions)
- Agent trust profiles (scoring, adjustment)
- Audit trail (immutable logging)
- Security considerations

**Key Features:**
- Default roles (org_admin, team_lead, contributor, agent)
- Permission checker implementation
- Trust score calculation algorithm
- Audit log format (JSONL)
- Rate limiting for agents

**Implementation Checklist:**
- 6 sub-phases with 2-day tasks
- Unit test requirements
- Integration test scenarios
- Security test cases

---

### 4. Enforcement Hooks Specification

**File:** [docs/specs/edda-enforcement-hooks.md](../specs/edda-enforcement-hooks.md)

**Contents:**
- Hook architecture (5 hook types)
- Hook definition schema
- Execution engine design
- Anvil integration points
- Override mechanism
- Performance optimization (<50ms target)

**Key Features:**
- Pre-execution checks (blocking)
- Contextual guidance (non-blocking)
- Memory matching logic
- Message templating
- Hook examples (5 real-world scenarios)

**Integration:**
- Gate system hooks
- Pre-action hooks
- File change hooks
- Planning guidance

**Performance:**
- Hook indexing by event
- Memory query caching
- Parallel execution
- Short-circuit on block

---

### 5. API Contracts Specification

**File:** [docs/specs/edda-api-contracts.md](../specs/edda-api-contracts.md)

**Contents:**
- REST API (30+ endpoints)
- CLI commands (40+ commands)
- TypeScript port interfaces
- External integration contracts
- Webhook support (future)

**API Categories:**
- Memory operations (8 endpoints)
- Promotion workflow (4 endpoints)
- Enforcement hooks (4 endpoints)
- Authority management (4 endpoints)
- Export/import (3 endpoints)
- Stats and health (2 endpoints)

**CLI Commands:**
- `anvil edda list/show/search/trace`
- `anvil edda proposals/promote/reject`
- `anvil edda hooks list/create/check`
- `anvil edda roles/trust/audit`
- `anvil edda export/import/stats`

**Integration Contracts:**
- Anvil gate system interface
- Identity provider interface
- Embedding service interface (optional)

---

### 6. Implementation Phase Breakdown

**File:** [plans/edda-phase-breakdown.md](../../plans/edda-phase-breakdown.md)

**Contents:**
- 7 phases (0-6, plus optional 7)
- Detailed task lists per phase
- Effort estimates (days per task)
- Dependencies and blockers
- Success criteria and KPIs
- Risk assessment (high/medium/low)

**Phase Summary:**
- **Phase 0:** Foundation (2 weeks) - Storage + Memory Manager
- **Phase 1:** Promotion Pipeline (3 weeks) - Ember → Edda workflow
- **Phase 2:** Authority & Trust (2 weeks) - RBAC + agent trust
- **Phase 3:** Query & Retrieval (2 weeks) - Search + provenance
- **Phase 4:** Enforcement Hooks (3 weeks) - Anvil integration
- **Phase 5:** Lifecycle (2 weeks) - Deprecation + staleness
- **Phase 6:** Interop (2 weeks) - API + export
- **Phase 7:** Meta (3 weeks, optional) - Knowledge graph + drift detection

**Critical Path:** 0 → 1 → 2 → 4 → 6 (12 weeks MVP)
**Full Path:** 0 → 1 → 2 → 3 → 4 → 5 → 6 (16 weeks)

---

### 7. TypeScript Contracts

**File:** [packages/edda-stack/src/contracts/edda-extended.ts](../../packages/edda-stack/src/contracts/edda-extended.ts)

**Contents:**
- Extended memory object interfaces
- Promotion pipeline types
- Authority and trust types
- Enforcement hook types
- Lifecycle management types
- Query and provenance types
- Meta-capability types

**Key Exports:**
- `MemoryObjectExtended` - Full memory with governance
- `PromotionRequest` - Promotion workflow state
- `EnforcementHook` - Hook definition
- `AgentTrustProfile` - Trust scoring
- `AuditEntry` - Audit trail
- `EddaQuery` - Query builder
- All supporting types and enums

**Use:**
```typescript
import type {
  MemoryObjectExtended,
  PromotionRequest,
  EnforcementHook
} from '@anvil/edda-stack/contracts/edda-extended'
```

---

## Implementation Roadmap

### MVP Timeline (12 weeks)

```
Week 1-2:   Phase 0 - Foundation
Week 3-5:   Phase 1 - Promotion
Week 6-7:   Phase 2 - Authority
Week 8-10:  Phase 4 - Enforcement
Week 11-12: Phase 6 - Interop (minimal)
```

**Deliverables:**
- Working memory storage (Git + SQLite)
- Promotion pipeline with human review
- RBAC and agent trust
- Enforcement hooks integrated with Anvil
- Basic CLI and export/import

**Not Included:**
- Advanced query (semantic search)
- Full lifecycle management
- Meta-capabilities
- Full REST API

---

### Full Feature Timeline (16-19 weeks)

```
Week 1-2:   Phase 0 - Foundation
Week 3-5:   Phase 1 - Promotion
Week 6-7:   Phase 2 - Authority
Week 8-9:   Phase 3 - Query
Week 10-12: Phase 4 - Enforcement
Week 13-14: Phase 5 - Lifecycle
Week 15-16: Phase 6 - Interop
Week 17-19: Phase 7 - Meta (optional)
```

**Deliverables:**
- Everything in MVP plus:
- Semantic search with conflict detection
- Full lifecycle management (deprecation, staleness)
- Complete REST API with OpenAPI docs
- Knowledge graph and contradiction detection
- Cultural drift detection

---

## Resource Requirements

### Minimum Team (MVP)
- 1 Senior Engineer (critical path)
- 1 Mid-Level Engineer (parallel tasks, testing)
- 1 Part-Time UX Designer (CLI/API design)

**Total:** 2.25 FTE

### Optimal Team (Full Feature)
- 1 Tech Lead (architecture, reviews)
- 2 Senior Engineers (parallel phases)
- 1 Mid-Level Engineer (testing, docs)
- 1 Part-Time UX Designer

**Total:** 4.5 FTE

---

## Key Architectural Decisions

### 1. Storage: Git-backed YAML + SQLite Index

**Rationale:**
- Git provides versioning and audit trail naturally
- YAML is human-readable for debugging
- SQLite provides fast queries without external dependencies

**Trade-offs:**
- Git may be slow for very large repositories (>10k memories)
- Mitigation: designed for migration to PostgreSQL if needed

---

### 2. Human-in-the-Loop Promotion

**Rationale:**
- Edda must be trustworthy (AI alone cannot decide institutional truth)
- Friction is a feature (prevents knowledge pollution)
- Humans provide nuance and context

**Trade-offs:**
- Slower than automatic promotion
- Mitigation: agent trust scores reduce review burden over time

---

### 3. Pre-Execution Enforcement

**Rationale:**
- Catch violations before damage is done
- Provide contextual guidance at the right time
- Integrate seamlessly with Anvil workflow

**Trade-offs:**
- Adds latency to action execution (<50ms target)
- Mitigation: aggressive caching, parallel hook execution

---

### 4. Aggressive Forgetting by Default

**Rationale:**
- Knowledge rots, stale memories are worse than no memories
- Forces continuous validation and review
- Prevents Edda from becoming a dumping ground

**Trade-offs:**
- Risk of losing valuable memories
- Mitigation: staleness detection with human review before deletion

---

## Open Questions for APS Planning

These questions should be resolved before detailed APS:

### 1. Storage Strategy
**Question:** Is Git-backed storage sufficient for v1, or should we design for PostgreSQL from the start?

**Options:**
- A) Git-only (simpler, good for <1000 memories)
- B) Design abstraction for future PostgreSQL migration
- C) PostgreSQL from day one

**Recommendation:** Option B (Git with abstraction layer)

---

### 2. Semantic Search
**Question:** Is semantic search mandatory for v1, or can it be optional/Phase 7?

**Options:**
- A) Mandatory (better UX, requires embedding service)
- B) Optional (simpler, FTS5 may be sufficient)
- C) Phase 7 (defer until proven need)

**Recommendation:** Option B (optional, enable if available)

---

### 3. Identity Provider
**Question:** Which identity provider(s) should we integrate with for v1?

**Options:**
- A) GitHub OAuth only (simplest for GitHub-hosted projects)
- B) Generic OIDC (works with multiple providers)
- C) Both GitHub OAuth + OIDC

**Recommendation:** Option A for MVP, Option C for full release

---

### 4. REST API Priority
**Question:** Is REST API required for v1, or can we ship CLI-only?

**Options:**
- A) CLI-only v1, API in v2 (faster to market)
- B) Basic API in v1 (read-only endpoints)
- C) Full API in v1 (delays release by 2 weeks)

**Recommendation:** Option A (CLI-only MVP, API in Phase 6)

---

### 5. Multi-Tenancy
**Question:** Should Edda support multiple organizations in v1?

**Options:**
- A) Single-org only (simpler)
- B) Multi-tenant from day one (more complex)

**Recommendation:** Option A (single-org for v1)

---

## Success Metrics

### Adoption Metrics
- Memories created per week
- Teams actively using Edda (>50% target)
- Agent proposals submitted per day
- Human reviews completed per week

### Quality Metrics
- Promotion approval rate (target: >70%)
- Memory contradiction rate (target: <5%)
- Staleness detection accuracy (target: >85%)
- False positive rate for enforcement (target: <10%)

### Impact Metrics
- Policy violations prevented by enforcement
- Incidents avoided through warnings
- Onboarding time reduction (target: -30%)
- Decision consistency score (qualitative)

---

## Risk Mitigation

### High-Risk Items
1. **Anvil Integration** - Requires core changes to Anvil
   - **Mitigation:** Early prototype in week 1-2, design review with Anvil team

2. **Semantic Search** - External dependency, performance unknown
   - **Mitigation:** Make optional, provide fallback to FTS5

3. **Git Performance** - May not scale to large repos
   - **Mitigation:** Design abstraction layer, benchmark early (Phase 0)

### Medium-Risk Items
1. **Conflict Detection** - Heuristics may have false positives
   - **Mitigation:** Tune thresholds, gather user feedback, iterate

2. **Agent Trust** - Trust scores may not reflect reality
   - **Mitigation:** Manual override, regular calibration, human feedback loops

---

## Next Steps

1. **Stakeholder Review** (Week 0)
   - Review this architecture package
   - Resolve open questions
   - Approve phases and timeline

2. **Resource Allocation** (Week 0)
   - Assign team members
   - Secure budget
   - Set up infrastructure (repos, CI/CD)

3. **Phase 0 Kickoff** (Week 1)
   - Create `edda-core` package
   - Implement Git storage adapter
   - Set up testing infrastructure

4. **Weekly Checkpoints**
   - Monday: Plan week, review blockers
   - Friday: Demo progress, update risks

5. **Phase Retrospectives**
   - After each phase completion
   - Adjust timeline/approach as needed

---

## Contributing

### For Document Updates

When updating architecture:
1. Update the relevant document (architecture, specs, phase breakdown)
2. Update this index if structure changes
3. Update `edda-extended.ts` if contracts change
4. Tag document with new version and date

### For New Documents

When adding new documents:
1. Add to appropriate directory (architecture, specs, plans)
2. Add entry to this index with summary
3. Update "Reading Guide" if relevant
4. Cross-reference from related documents

---

## Version History

| Version | Date | Changes | Author |
|---------|------|---------|--------|
| 1.0.0 | 2026-01-19 | Initial architecture package for APS planning | Architecture Team |

---

## Document Ownership

| Document | Owner | Reviewers |
|----------|-------|-----------|
| edda-system-architecture.md | Tech Lead | Eng Leadership, Product |
| edda-component-dependencies.md | Tech Lead | Senior Engineers |
| edda-authority-trust.md | Security Engineer | Tech Lead, Eng Leadership |
| edda-enforcement-hooks.md | Platform Engineer | Tech Lead, Anvil Team |
| edda-api-contracts.md | API Engineer | Tech Lead, Frontend Team |
| edda-phase-breakdown.md | Engineering Manager | Tech Lead, Product |
| edda-extended.ts | Tech Lead | All Engineers |

---

## Contact

**Questions about architecture?**
- Tech Lead: [TBD]
- Edda Project Channel: [TBD]
- Design Docs: This directory

**Ready for APS Planning?**
- Review checklist: ✅ All documents reviewed
- Open questions: ✅ Resolved
- Resource allocation: ⏳ Pending
- Timeline approval: ⏳ Pending

---

**This architecture package is ready for APS planning. All core design decisions are documented and implementation is well-scoped.**
