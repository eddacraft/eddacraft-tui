# Edda APS Planning: Open Questions

**Purpose:** Resolve architectural decisions before detailed APS planning
**Status:** For stakeholder review
**Date:** 2026-01-19

---

## Overview

Before proceeding with detailed APS planning for Phases 1-7, we need to resolve the following open questions. These decisions will impact implementation approach, timeline, and resource requirements.

**Priority:** HIGH - Required for Phase 1+ planning
**Decision Deadline:** Before Phase 0 completion (Week 2)

---

## Question 1: Storage Strategy

**Status:** ✅ RESOLVED

**Decision:** Git-backed YAML + SQLite Index for v1
**Rationale:** See `/docs/architecture/edda-storage-comparison.md`
**Impact:** Phase 0 can proceed as planned

---

## Question 2: Semantic Search

**Question:** Is semantic search mandatory for v1, or can it be optional/Phase 7?

### Options

**A) Mandatory (Phase 3)**
- **Pros:** Better UX, more powerful queries, competitive feature
- **Cons:** Requires embedding service (Ollama or OpenAI), adds complexity, external dependency
- **Timeline Impact:** +1 week to Phase 3
- **Cost:** External API costs (OpenAI) OR self-hosted infrastructure (Ollama)

**B) Optional (Phase 3)**
- **Pros:** Simpler, FTS5 may be sufficient, can add later
- **Cons:** Weaker search initially, may need to refactor later
- **Timeline Impact:** No change
- **Cost:** None for v1

**C) Phase 7 (Deferred)**
- **Pros:** Focus on core features first, validate need with real usage
- **Cons:** v1 shipped without semantic search
- **Timeline Impact:** -2 weeks from v1
- **Cost:** None for v1

### Recommendation: **Option B (Optional in Phase 3)**

**Rationale:**
1. **FTS5 is powerful** - SQLite's full-text search can handle most queries
2. **Low risk** - Can add semantic search without breaking changes
3. **Validate need** - Real usage data will inform whether semantic search is necessary
4. **Cost control** - Avoid external dependencies until proven valuable

**Implementation:**
```typescript
interface IQueryService {
  query(query: EddaQuery): Promise<EddaQueryResult>
  search(text: string): Promise<SemanticResult>  // FTS5 by default

  // Optional semantic search (if embedding service available)
  semanticSearch?(query: SemanticQuery): Promise<SemanticResult>
}
```

**Decision Required:** Product/Engineering Leadership
**Impact:** Phase 3 scope and timeline

---

## Question 3: Identity Provider Integration

**Question:** Which identity provider(s) should we integrate with for v1?

### Options

**A) GitHub OAuth only**
- **Pros:** Simplest, works for GitHub-hosted projects, quick implementation
- **Cons:** Limited to GitHub users, doesn't work for on-prem
- **Timeline:** No change (2 weeks for Phase 2)
- **Complexity:** Low

**B) Generic OIDC (OpenID Connect)**
- **Pros:** Works with multiple providers (GitHub, Google, Azure AD), flexible
- **Cons:** More complex, harder to test, configuration burden
- **Timeline:** +1 week to Phase 2 (3 weeks total)
- **Complexity:** Medium

**C) Both GitHub OAuth + OIDC**
- **Pros:** Best of both worlds, maximum flexibility
- **Cons:** Most complex, maintenance burden
- **Timeline:** +2 weeks to Phase 2 (4 weeks total)
- **Complexity:** High

**D) Simple JWT (no external provider)**
- **Pros:** No external dependencies, complete control
- **Cons:** No SSO, manual user management, security burden
- **Timeline:** -1 week from Phase 2 (1 week total)
- **Complexity:** Low but risky

### Recommendation: **Option A for v1, Option C for v2**

**Rationale:**
1. **Fastest to market** - GitHub OAuth is simple and sufficient for MVP
2. **Target audience** - Primary users are developers using GitHub
3. **Migration path** - Can add OIDC later without breaking changes
4. **Testing** - Easier to test with single provider

**Implementation:**
```typescript
// v1: GitHub OAuth only
interface IIdentityProvider {
  authenticate(token: string): Promise<Principal>
  getPrincipalInfo(identifier: string): Promise<PrincipalInfo>
}

class GitHubOAuthProvider implements IIdentityProvider { ... }

// v2: Add OIDC
class OIDCProvider implements IIdentityProvider { ... }
```

**Decision Required:** Product/Engineering Leadership
**Impact:** Phase 2 scope and timeline

---

## Question 4: REST API Priority

**Question:** Is REST API required for v1, or can we ship CLI-only?

### Options

**A) CLI-only for v1, API in v1.1**
- **Pros:** Faster to market (-2 weeks), focus on core features
- **Cons:** No programmatic access, limits integrations
- **Timeline:** 14 weeks total (skip Phase 6)
- **v1 Features:** CLI fully functional

**B) Basic read-only API in v1**
- **Pros:** Some programmatic access, reasonable compromise
- **Cons:** Limited utility without write APIs
- **Timeline:** 15 weeks (Phase 6 reduced to 1 week)
- **v1 Features:** CLI + read API

**C) Full API in v1**
- **Pros:** Complete programmatic access, enables integrations
- **Cons:** Delays v1 release
- **Timeline:** 16 weeks (Phase 6 as planned: 2 weeks)
- **v1 Features:** CLI + full REST API

### Recommendation: **Option A (CLI-only for v1)**

**Rationale:**
1. **Primary users are humans** - CLI serves human review workflow
2. **API can ship later** - v1.1 release with API (2-3 weeks after v1)
3. **Faster feedback** - Get core workflow validated before API
4. **Lower risk** - Less surface area to support initially

**Timeline Impact:**
- v1.0 ships Week 14 (CLI-only)
- v1.1 ships Week 17 (add REST API)

**Acceptance Criteria for v1.0:**
- ✅ All CLI commands functional
- ✅ Human promotion workflow complete
- ✅ Enforcement hooks working
- ⏸️ REST API deferred to v1.1

**Decision Required:** Product/Engineering Leadership
**Impact:** v1 release timeline and feature set

---

## Question 5: Multi-Tenancy

**Question:** Should Edda support multiple organizations in v1?

### Options

**A) Single-org only**
- **Pros:** Simpler data model, faster implementation, easier testing
- **Cons:** Can't support multi-org deployments
- **Timeline:** No change (16 weeks)
- **Scope:** `.edda/` directory per repository

**B) Multi-tenant from day one**
- **Pros:** Supports hosted service, org isolation
- **Cons:** Complex data model, slower implementation
- **Timeline:** +3 weeks (19 weeks total)
- **Scope:** Database per org OR scoped storage

### Recommendation: **Option A (Single-org for v1)**

**Rationale:**
1. **Target deployment** - Self-hosted per organization initially
2. **Simpler is better** - Validate core workflow before multi-tenancy
3. **Migration path** - Can add multi-tenancy in v2 if needed
4. **Lower risk** - Fewer failure modes to handle

**Assumptions:**
- One `.edda/` directory per repository
- One organization per Edda instance
- Future: hosted service would run multiple Edda instances

**Implementation Notes:**
```typescript
// v1: Single org (implicit)
interface MemoryObjectExtended {
  // No org_id field
}

// v2: Multi-tenant (if needed)
interface MemoryObjectExtended {
  org_id: string  // Added for multi-tenancy
}
```

**Decision Required:** Product/Engineering Leadership
**Impact:** Data model design and v1 scope

---

## Decision Matrix

| Question | Options | Recommended | Impact | Priority |
|----------|---------|-------------|--------|----------|
| **1. Storage** | Git+YAML vs JSONL vs PostgreSQL | ✅ Git+YAML | None (resolved) | HIGH |
| **2. Semantic Search** | Mandatory vs Optional vs Phase 7 | 🟡 Optional | +0 weeks | MEDIUM |
| **3. Identity Provider** | GitHub vs OIDC vs Both vs JWT | 🟡 GitHub for v1 | +0 weeks | HIGH |
| **4. REST API** | CLI-only vs Read-only vs Full | 🟡 CLI-only for v1 | -2 weeks | MEDIUM |
| **5. Multi-Tenancy** | Single-org vs Multi-tenant | 🟡 Single-org | +0 weeks | LOW |

**Timeline Impact of Recommendations:**
- v1.0: 14 weeks (CLI-only, no API)
- v1.1: 17 weeks (add REST API)
- Total: 17 weeks for complete v1

**vs. Original Estimate:**
- Original: 16 weeks (full feature v1)
- Recommended: 14 weeks (v1.0) + 3 weeks (v1.1) = 17 weeks total
- Delta: +1 week, but v1.0 ships 2 weeks earlier

---

## Recommendation: Phased Release Strategy

### v1.0 (Week 14) - "Core Workflow MVP"

**Features:**
- ✅ Memory storage (Git+YAML)
- ✅ Promotion pipeline with human review
- ✅ RBAC and agent trust
- ✅ Enforcement hooks (Anvil integration)
- ✅ Full CLI commands
- ✅ Export/import (basic)
- ⏸️ REST API (deferred)
- ⏸️ Semantic search (optional)

**Success Criteria:**
- Humans can review and approve memories via CLI
- Enforcement hooks prevent policy violations
- Agent trust scores improve proposal quality

**Target Audience:**
- Internal teams (dogfooding)
- Early adopters willing to use CLI

---

### v1.1 (Week 17) - "API & Integration"

**Features:**
- ✅ REST API (full CRUD)
- ✅ OpenAPI documentation
- ✅ Webhook support
- ✅ Enhanced export/import

**Success Criteria:**
- External tools can integrate via API
- Automated workflows can query Edda
- CI/CD pipelines can enforce policies

**Target Audience:**
- Teams wanting programmatic access
- Integration partners

---

### v2.0 (Future) - "Enterprise & Scale"

**Features:**
- ✅ Semantic search
- ✅ OIDC support (multi-provider)
- ✅ Multi-tenancy
- ✅ PostgreSQL backend (if needed)
- ✅ Knowledge graph
- ✅ Cultural drift detection

**Success Criteria:**
- Scales to 10K+ memories
- Supports enterprise SSO
- Advanced analytics and insights

---

## Next Steps

### Immediate (Before Phase 0 completion)
1. **Review this document** with stakeholders
2. **Make decisions** on questions 2-5
3. **Update Phase 1+ APS documents** based on decisions
4. **Communicate timeline** to team and stakeholders

### Decision Meeting Agenda
1. Review phased release strategy (v1.0 vs v1.1)
2. Decide: Semantic search (mandatory/optional/deferred)
3. Decide: Identity provider (GitHub/OIDC/both)
4. Decide: REST API priority (v1.0/v1.1)
5. Decide: Multi-tenancy (v1/v2)
6. Approve updated timeline

### After Decisions
1. Finalize Phase 1 APS document
2. Create Phase 2-6 APS documents
3. Set up project tracking (milestones, issues)
4. Begin Phase 0 implementation

---

## Appendix: Decision Impact Summary

### If All Recommendations Accepted

**v1.0 Timeline (MVP):**
```
Phase 0: Foundation (2 weeks)
Phase 1: Promotion (3 weeks)
Phase 2: Authority (2 weeks) - GitHub OAuth only
Phase 3: Query (2 weeks) - FTS5 only, semantic optional
Phase 4: Enforcement (3 weeks)
Phase 5: Lifecycle (2 weeks)
───────────────────────────────
Total: 14 weeks to v1.0
```

**v1.1 Timeline (API):**
```
Phase 6: Interop (2 weeks) - REST API
Phase 6.1: Webhooks (1 week)
───────────────────────────────
Total: 17 weeks to v1.1
```

**v2.0 Timeline (Enterprise):**
```
Phase 7: Meta (3 weeks) - Knowledge graph, drift
Phase 2.1: OIDC (1 week)
Phase 3.1: Semantic (2 weeks) - If validated as needed
Phase 5.1: Multi-tenant (3 weeks)
───────────────────────────────
Total: ~25 weeks to v2.0
```

### Resource Requirements

**v1.0 (14 weeks):**
- 2 Senior Engineers (full-time)
- 1 Mid-Level Engineer (full-time)
- 1 UX Designer (part-time, 25%)
- **Total:** 3.25 FTE

**v1.1 (3 weeks):**
- 1 Senior Engineer (full-time)
- 1 Mid-Level Engineer (part-time, 50%)
- **Total:** 1.5 FTE

---

**Document Owner:** Engineering Leadership
**Reviewers:** Product, Engineering, Design
**Decision Deadline:** Before Phase 0 completion (Week 2)
**Status:** Awaiting stakeholder review
