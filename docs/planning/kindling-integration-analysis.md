# Kindling Integration Analysis

**Date**: 2026-01-01
**Status**: Research & Recommendation
**Author**: Claude Code

## Executive Summary

Kindling is a lightweight, local-first memory system for AI workflows that captures observations (tool calls, diffs, commands, errors) and organizes them into "capsules" with deterministic, provenance-aware retrieval. It aligns exceptionally well with Anvil's core principles and could enhance three critical areas: **evidence & provenance**, **cross-session continuity**, and **audit transparency**.

**Recommendation**: Integrate Kindling as Anvil's memory layer, focusing on:
1. Evidence bundle enhancement (immediate value)
2. Plan evolution tracking (high strategic value)
3. Gate check history and learning (medium-term value)

## What is Kindling?

### Core Purpose
Kindling provides "durable memory and contextual continuity to agentic and AI-assisted workflows" by capturing observations without enforcing governance or organizational truth.

### Key Capabilities

1. **Observation Capture** — Records tool calls, diffs, commands, errors, and messages
2. **Capsule Organization** — Groups observations into bounded units (sessions, workflow nodes)
3. **Provenance-Aware Retrieval** — Deterministic, scoped, explainable results
4. **Local-First Operation** — Embedded SQLite, no external services required

### Architecture (Planned)

```
kindling-core              → Domain model, retrieval orchestration
kindling-store-sqlite      → Persistent storage layer
kindling-provider-local    → Full-text search + recency ranking
kindling-adapter-opencode  → OpenCode integration (deprioritized for Anvil)
kindling-adapter-pocketflow → Workflow integration (deprioritized for Anvil)
kindling-cli               → Inspection, debugging, export/import
```

### Key Design Principles

- **Determinism**: Same input → same retrieval results
- **Explainability**: Transparent provenance for all retrieved data
- **Local-first**: No cloud dependencies, embedded SQLite
- **Conservative summarization**: Retain raw observations
- **Scope boundaries**: Explicitly excludes governance, access control, and cloud deployment

### Current Status

**Work in Progress** — Early-stage repository (10 commits, 1 star) with primarily planning documentation. Has a `plans/index.aps.md` file suggesting potential APS format usage.

---

## Anvil + Kindling: Alignment Analysis

### Shared Principles

| Principle | Anvil | Kindling | Alignment |
|-----------|-------|----------|-----------|
| **Determinism** | Hash-stable APS, reproducible gates | Deterministic retrieval, SQLite system of record | ✅ Perfect |
| **Transparency** | Evidence bundles, provenance tracking | Observation capture with provenance | ✅ Perfect |
| **Local-first** | CLI-based, no cloud requirement | Embedded SQLite, no external services | ✅ Perfect |
| **Safety** | Snapshot before apply, rollback capability | Conservative observation retention | ✅ Strong |
| **Interoperability** | Multi-format adapters (SpecKit, BMAD) | Adapter architecture (OpenCode, PocketFlow) | ✅ Strong |

### Strategic Fit

Kindling addresses a gap Anvil doesn't currently solve: **cross-session memory and context continuity**. Anvil is stateless—each validation, gate check, or export is independent. Kindling could provide:

1. **Historical context** for plan evolution
2. **Learning** from previous gate failures
3. **Audit trails** linking plan changes to outcomes
4. **Resume capability** for long-running development workflows

---

## Integration Opportunities

### 1. Evidence Bundle Enhancement ⭐ **Immediate Value**

**Problem**: Anvil's evidence bundles capture snapshots but don't maintain historical context across multiple plan iterations.

**Solution**: Use Kindling to:
- Capture all `anvil validate`, `anvil gate`, and `anvil export` operations as observations
- Store tool calls (ESLint, Vitest, coverage checks) with full provenance
- Link plan versions to gate results over time
- Enable queries like "When did this test start failing?" or "What changed between passing and failing gates?"

**Implementation**:
```typescript
// In core/src/provenance/evidence-bundle.ts
import { Kindling } from 'kindling-core';

export async function captureGateEvidence(
  planHash: string,
  gateResults: GateResult[]
): Promise<void> {
  const capsule = kindling.createCapsule({
    type: 'gate-execution',
    planHash,
    timestamp: new Date().toISOString(),
  });

  for (const result of gateResults) {
    capsule.observe({
      type: 'gate-check',
      check: result.checkName,
      status: result.status,
      output: result.output,
      provenance: result.provenance,
    });
  }

  await capsule.close();
}
```

**Benefits**:
- Richer evidence bundles with full historical context
- Answers questions like "Has this plan ever passed gates?"
- Supports compliance and audit requirements

---

### 2. Plan Evolution Tracking ⭐⭐ **High Strategic Value**

**Problem**: Anvil sees plans as immutable snapshots (hash-based). It doesn't track how plans evolve or why changes were made.

**Solution**: Use Kindling to:
- Capture every plan modification as an observation
- Link plan hashes to git commits, AI conversations, or manual edits
- Track adapter conversions (SpecKit → APS → BMAD)
- Enable queries like "What was the plan state when tests were passing?" or "Why was this step added?"

**Implementation**:
```typescript
// In cli/src/services/plan-loader.ts
import { Kindling } from 'kindling-core';

export async function trackPlanModification(
  planPath: string,
  previousHash: string,
  newHash: string,
  source: 'manual' | 'ai-generated' | 'adapter-conversion'
): Promise<void> {
  const capsule = kindling.getCapsule('plan-evolution');

  capsule.observe({
    type: 'plan-modified',
    path: planPath,
    previousHash,
    newHash,
    source,
    diff: await computePlanDiff(previousHash, newHash),
    timestamp: new Date().toISOString(),
  });
}
```

**Benefits**:
- Understand plan evolution over time
- Link plan changes to outcomes (gate results, deployments)
- Support "time-travel" debugging ("What did the plan look like when it worked?")
- Enable AI assistants to learn from plan history

---

### 3. Gate Check History & Learning ⭐ **Medium-Term Value**

**Problem**: Anvil's gates are stateless—each run is independent. Developers must remember previous failures and fixes.

**Solution**: Use Kindling to:
- Store all gate check results with full context
- Track recurring failures (e.g., "ESLint always fails on this pattern")
- Suggest fixes based on historical solutions
- Enable queries like "What fixed this coverage failure last time?"

**Implementation**:
```typescript
// In core/src/gate/gate-runner.ts
import { Kindling } from 'kindling-core';

export async function runGateWithHistory(
  plan: APSPlan,
  checks: GateCheck[]
): Promise<GateResult[]> {
  const history = await kindling.retrieve({
    capsule: 'gate-history',
    filter: { planHash: plan.hash },
    limit: 10,
  });

  const results: GateResult[] = [];

  for (const check of checks) {
    const result = await check.run(plan);

    // Capture observation
    await kindling.observe({
      type: 'gate-check-result',
      check: check.name,
      planHash: plan.hash,
      status: result.status,
      output: result.output,
      previousFailures: history.filter(h =>
        h.check === check.name && h.status === 'failed'
      ),
    });

    results.push(result);
  }

  return results;
}
```

**Benefits**:
- Avoid repeating the same fixes
- Surface patterns in gate failures
- Enable AI assistants to suggest contextual fixes
- Improve developer productivity

---

### 4. Cross-Session Continuity ⭐ **Strategic Enabler**

**Problem**: AI assistants working with Anvil lose context between sessions (e.g., "Why did we choose this architecture?").

**Solution**: Use Kindling to:
- Capture AI conversation context alongside plan modifications
- Store architectural decisions and rationale
- Enable resume workflows ("Continue working on authentication plan")
- Link plan changes to issue trackers, PRs, or design docs

**Implementation**:
```typescript
// In cli/src/services/ai-context.ts
import { Kindling } from 'kindling-core';

export async function captureAIDecision(
  planHash: string,
  decision: {
    question: string;
    answer: string;
    rationale: string;
    alternatives: string[];
  }
): Promise<void> {
  await kindling.observe({
    type: 'ai-decision',
    planHash,
    ...decision,
    timestamp: new Date().toISOString(),
  });
}

export async function retrievePlanContext(
  planHash: string
): Promise<AIContext> {
  const observations = await kindling.retrieve({
    capsule: 'plan-context',
    filter: { planHash },
  });

  return {
    decisions: observations.filter(o => o.type === 'ai-decision'),
    modifications: observations.filter(o => o.type === 'plan-modified'),
    gateResults: observations.filter(o => o.type === 'gate-check-result'),
  };
}
```

**Benefits**:
- AI assistants can resume work with full context
- Developers can understand "why" decisions were made
- Supports knowledge transfer and onboarding
- Enables better collaboration between human and AI

---

## Implementation Strategy

### Phase 1: Proof of Concept (1-2 weeks)

**Goal**: Validate Kindling integration with minimal changes

**Tasks**:
1. Add `kindling-core` and `kindling-store-sqlite` as dependencies
2. Create `core/src/memory/kindling-integration.ts` with basic capture
3. Integrate observation capture in `gate-runner.ts`
4. Add simple retrieval in CLI (`anvil gate --history`)
5. Write integration tests

**Success Criteria**:
- Gate results are captured to Kindling
- Can retrieve previous gate results for the same plan
- No performance degradation

---

### Phase 2: Evidence Bundle Integration (2-3 weeks)

**Goal**: Enhance evidence bundles with Kindling observations

**Tasks**:
1. Extend evidence bundle format to include Kindling capsule IDs
2. Capture all validation, gate, and export operations
3. Add `anvil evidence show <capsule-id>` command
4. Implement provenance linking (plan hash → observations)
5. Add documentation and examples

**Success Criteria**:
- Evidence bundles include links to Kindling observations
- Can retrieve full audit trail for any plan version
- CLI supports evidence inspection

---

### Phase 3: Plan Evolution Tracking (3-4 weeks)

**Goal**: Track plan modifications and link to outcomes

**Tasks**:
1. Add plan modification hooks in `plan-loader.ts`
2. Capture plan diffs and change rationale
3. Implement `anvil plan history <plan-path>` command
4. Add time-travel queries ("Show plan at commit X")
5. Integrate with git history

**Success Criteria**:
- All plan modifications are tracked
- Can query plan history and evolution
- Links plan changes to gate results

---

### Phase 4: Gate Learning & Suggestions (4-6 weeks)

**Goal**: Use historical data to improve gate feedback

**Tasks**:
1. Build gate history analysis
2. Detect recurring failure patterns
3. Implement suggestion engine
4. Add `anvil gate --suggest-fixes` flag
5. Train on historical gate results

**Success Criteria**:
- Suggests fixes based on historical solutions
- Identifies recurring failure patterns
- Improves developer productivity

---

## Technical Considerations

### Performance

**Concern**: Kindling adds I/O overhead (SQLite writes)

**Mitigation**:
- Use async observation capture (non-blocking)
- Batch observations before commit
- Configure SQLite for performance (WAL mode, tuned cache)
- Make Kindling optional (feature flag)

### Storage

**Concern**: SQLite database growth over time

**Mitigation**:
- Implement retention policies (configurable)
- Add `anvil memory prune` command
- Support export/archive for long-term storage
- Default to 30-day retention

### Privacy & Security

**Concern**: Observations might contain sensitive data

**Mitigation**:
- Implement redaction for secrets (reuse Anvil's secret detection)
- Allow users to disable observation capture
- Document what data is captured
- Support local-only operation (no cloud sync)

### Dependency Management

**Concern**: Adding Kindling as a dependency

**Mitigation**:
- Kindling is local-first (no external services)
- Small footprint (embedded SQLite)
- Apache 2.0 licence (compatible with Anvil)
- Early-stage project (contribute upstream improvements)

---

## Risks & Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Kindling is early-stage, may lack features | High | Medium | Contribute features upstream, fork if needed |
| Performance overhead from observations | Medium | Low | Async capture, feature flag, performance testing |
| Storage growth over time | Low | High | Retention policies, pruning, export/archive |
| Kindling development stalls | Medium | Medium | Fork and maintain if needed, small codebase |
| User privacy concerns | High | Low | Redaction, clear documentation, opt-in model |

---

## Alternative Approaches

### 1. Build Custom Memory System

**Pros**: Full control, Anvil-specific optimizations
**Cons**: Duplicates effort, distracts from core value, longer timeline
**Verdict**: ❌ Avoid—Kindling aligns well with needs

### 2. Use Existing Memory Systems (mem0, basic-memory)

**Pros**: More mature, larger communities
**Cons**: Cloud-first, not deterministic, governance-heavy
**Verdict**: ❌ Misaligned with Anvil's local-first, deterministic principles

### 3. SQLite Direct (No Abstraction)

**Pros**: Minimal dependencies, full control
**Cons**: No capsule abstraction, manual provenance tracking
**Verdict**: ⚠️ Possible fallback if Kindling doesn't mature

### 4. Kindling Integration (Recommended)

**Pros**: Aligns with principles, active development, good abstraction
**Cons**: Early-stage, dependency risk
**Verdict**: ✅ **Recommended**—Best fit for Anvil's needs

---

## Success Metrics

### Technical Metrics
- Evidence bundle completeness score (% of operations captured)
- Query response time (< 100ms for typical history queries)
- Storage growth rate (< 10MB per 1000 operations)
- Integration test coverage (> 80%)

### User Metrics
- Developer time saved (estimate from surveys)
- Adoption rate (% of projects using memory features)
- Gate fix success rate (% of suggestions that resolve issues)
- Cross-session resume success (% of sessions that successfully resume)

---

## Recommendations

### Immediate Actions (This Week)

1. ✅ Review Kindling repository architecture
2. ⏭️ Clone Kindling locally and explore codebase
3. ⏭️ Build proof-of-concept integration (gate observation capture)
4. ⏭️ Schedule review with stakeholders

### Short-Term (Next Month)

1. Implement Phase 1 (Proof of Concept)
2. Contribute feedback/improvements to Kindling upstream
3. Write integration tests
4. Document Kindling integration in Anvil architecture

### Medium-Term (Next Quarter)

1. Implement Phase 2 (Evidence Bundle Integration)
2. Implement Phase 3 (Plan Evolution Tracking)
3. Add CLI commands for memory inspection
4. Write user documentation and examples

### Long-Term (Next 6 Months)

1. Implement Phase 4 (Gate Learning & Suggestions)
2. Explore AI-assisted fix suggestions
3. Build analytics and insights features
4. Consider contributing Anvil-specific adapters to Kindling

---

## Conclusion

Kindling is a **strong strategic fit** for Anvil. It aligns with core principles (determinism, transparency, local-first) and addresses a genuine gap (cross-session memory and context continuity).

**Primary value areas**:
1. Enhanced evidence bundles with full audit trails
2. Plan evolution tracking and time-travel debugging
3. Gate check history and learning
4. Cross-session continuity for AI assistants

**Recommended approach**: Phased integration starting with proof-of-concept, then evidence bundles, plan tracking, and finally learning/suggestions.

**Key success factors**:
- Async observation capture (performance)
- Retention policies (storage)
- Redaction for secrets (privacy)
- Clear documentation (adoption)

**Next steps**: Clone Kindling, build PoC, validate integration approach.

---

## Appendix: Kindling Repository Details

**Repository**: https://github.com/EddaCraft/kindling
**Licence**: Apache 2.0
**Status**: Work in Progress (10 commits, 1 star)
**Language**: Likely TypeScript (based on ecosystem)

**Planned Packages**:
- kindling-core
- kindling-store-sqlite
- kindling-provider-local
- kindling-adapter-opencode (deprioritize)
- kindling-adapter-pocketflow (deprioritize)
- kindling-cli

**Key Files**:
- `plans/index.aps.md` — Planning document (possibly APS format)
- `docs/architecture.md` — Architecture specification
- `docs/data-model.md` — Data model definitions
- `docs/retrieval-contract.md` — Retrieval behaviour contract

**Boundary Conditions** (Explicitly Out of Scope):
- Governance workflows
- Multi-user access control
- Cloud deployment
- Truth assertion or memory curation

These boundaries align well with Anvil's focus on deterministic, local-first tooling.

---

**Sources**:
- [GitHub - EddaCraft/kindling](https://github.com/EddaCraft/kindling)
- [GitHub - basicmachines-co/basic-memory](https://github.com/basicmachines-co/basic-memory)
- [GitHub - mem0ai/mem0](https://github.com/mem0ai/mem0)
- [GitHub - steveyegge/beads](https://github.com/steveyegge/beads)
