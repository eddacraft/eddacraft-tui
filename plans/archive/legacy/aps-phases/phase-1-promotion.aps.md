# APS: Edda Phase 1 - Promotion Pipeline

**Phase:** 1 (Promotion Pipeline)
**Duration:** 3 weeks (15 working days)
**Status:** Planning
**Owner:** [TBD]
**Dependencies:** Phase 0 (Foundation) must be complete

---

## Overview

Phase 1 implements the Ember → Edda promotion workflow, enabling human-in-the-loop approval of candidate memories. This is the core value proposition of Edda: transforming AI-generated proposals into human-approved institutional truth.

---

## Goals

### Primary Goal
Build a complete promotion pipeline that:
- Fetches proposals from Ember
- Transforms proposal → memory (type mapping)
- Generates diffs for human review
- Captures approval/rejection with rationale
- Updates Ember with promotion status
- Tracks agent trust based on outcomes

### Success Criteria
- ✅ Can fetch active proposals from Ember
- ✅ Promotion requests created with transformation
- ✅ Human review UI (CLI) is intuitive and informative
- ✅ Approval creates memory with full provenance
- ✅ Rejection records rationale for learning
- ✅ Agent trust scores update after each review
- ✅ Average review time <5 minutes

---

## Architecture Context

```
Phase 1 Components:

┌─────────────────────────────────────────────────────────┐
│                    Ember Port                            │
│              (proposals, status updates)                 │
└────────────────┬────────────────────────────────────────┘
                 ↓
┌────────────────▼────────────────────────────────────────┐
│ Phase 1: Promotion Service (NEW)                        │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  ┌──────────────────┐  ┌──────────────────┐            │
│  │ Promotion Request│  │   Type Mapper    │            │
│  │   Manager        │  │ (Ember → Edda)   │            │
│  └────────┬─────────┘  └────────┬─────────┘            │
│           │                      │                       │
│  ┌────────▼──────────────────────▼─────────┐            │
│  │         Diff Generator                   │            │
│  │  (show proposal → memory changes)        │            │
│  └────────┬─────────────────────────────────┘            │
│           │                                               │
│  ┌────────▼──────────────────────────────┐              │
│  │      Review Workflow                  │              │
│  │  (approve, reject, needs revision)    │              │
│  └────────┬──────────────────────────────┘              │
│           │                                               │
│  ┌────────▼──────────────────────────────┐              │
│  │    Rejection Tracker                  │              │
│  │  (learning signals for Ember)         │              │
│  └───────────────────────────────────────┘              │
│                                                          │
└────────────────┬─────────────────────────────────────────┘
                 ↓
┌────────────────▼────────────────────────────────────────┐
│          Edda Core (Phase 0)                            │
│     (Memory Manager, Storage, Index)                    │
└─────────────────────────────────────────────────────────┘
```

---

## Epics & Tasks

### Epic 1: Ember Integration (2 days)

**Goal:** Connect to Ember and fetch proposals

#### Task 1.1: Ember Port Implementation
**Estimate:** 1 day
**Owner:** [TBD]

**Description:**
Implement Ember port interface to fetch and update proposals.

**Acceptance Criteria:**
- [ ] `EmberPortClient` implements `IEmberPort` interface
- [ ] Can fetch proposal by ID
- [ ] Can list active proposals (confidence > threshold)
- [ ] Can mark proposal as promoted (with memory ID)
- [ ] Can mark proposal as dismissed (with reason)
- [ ] Handles Ember API errors gracefully
- [ ] Unit tests with mocked Ember responses

**Interface:**
```typescript
interface IEmberPort {
  getProposal(id: ProposalId): Promise<CandidateProposal>
  getActiveProposals(options?: {
    minConfidence?: number
    types?: ProposalType[]
    limit?: number
  }): Promise<CandidateProposal[]>
  markPromoted(
    id: ProposalId,
    memoryId: MemoryId,
    resolvedBy: Principal
  ): Promise<void>
  markDismissed(
    id: ProposalId,
    reason: string,
    category: RejectionCategory,
    resolvedBy: Principal
  ): Promise<void>
}
```

**Files Created:**
```
packages/edda-promotion/src/
├── ember/
│   ├── ember-port-client.ts
│   └── ember-api-adapter.ts
└── __tests__/
    └── ember-port-client.test.ts
```

**Test Coverage:**
- Fetch proposal → returns valid proposal
- Fetch non-existent → throws NotFoundError
- List active → returns proposals above threshold
- Mark promoted → updates Ember status
- Mark dismissed → records rejection

---

#### Task 1.2: Kindling Integration for Provenance
**Estimate:** 0.5 day
**Owner:** [TBD]

**Description:**
Integrate with Kindling to validate provenance chains.

**Acceptance Criteria:**
- [ ] Can fetch observations by ID
- [ ] Can validate provenance chain (observations exist)
- [ ] Handles missing observations gracefully
- [ ] Unit tests for provenance validation

**Files Created:**
```
packages/edda-promotion/src/
├── provenance/
│   ├── provenance-validator.ts
│   └── kindling-client.ts
```

**Test Coverage:**
- Valid provenance → validation passes
- Missing observation → validation fails with detail
- Partial provenance → warns but allows

---

#### Task 1.3: Configuration & Setup
**Estimate:** 0.5 day
**Owner:** [TBD]

**Description:**
Configure promotion service with thresholds and policies.

**Acceptance Criteria:**
- [ ] Configuration schema defined
- [ ] Can set confidence thresholds per type
- [ ] Can enable/disable auto-promotion requests
- [ ] Configuration validated on load
- [ ] Unit tests for configuration

**Configuration:**
```typescript
interface PromotionConfig {
  confidenceThresholds: Record<ProposalType, number>
  autoRequestPromotion: boolean
  requireProvenanceValidation: boolean
  defaultReviewers: Principal[]
}

// Example:
{
  confidenceThresholds: {
    decision: 0.85,
    pattern: 0.75,
    warning: 0.70,
    lesson: 0.80,
    anomaly: 0.65,
    constraint: 0.90
  },
  autoRequestPromotion: true,
  requireProvenanceValidation: true,
  defaultReviewers: []
}
```

**Files Created:**
```
packages/edda-promotion/src/
├── config/
│   ├── promotion-config.ts
│   └── config-schema.ts
```

---

### Epic 2: Type Mapping & Transformation (3 days)

**Goal:** Transform Ember proposals into Edda memories

#### Task 2.1: Type Mapping Rules
**Estimate:** 1 day
**Owner:** [TBD]

**Description:**
Implement type mapping from Ember proposals to Edda memories.

**Acceptance Criteria:**
- [ ] All 6 proposal types map to memory types
- [ ] Confidence transformation (numeric → semantic)
- [ ] Statement extraction and refinement
- [ ] Context inference from proposal
- [ ] Scope inference from observations
- [ ] Unit tests for all type mappings

**Mapping Rules:**
```typescript
type TypeMapping = {
  [K in ProposalType]: {
    memoryType: MemoryType
    confidenceMapping: (numeric: number) => EddaConfidenceLevel
    statementTransform: (proposal: CandidateProposal) => string
    contextInference: (proposal: CandidateProposal) => MemoryContext
  }
}

// Example:
decision: {
  memoryType: 'decision',
  confidenceMapping: (n) => n > 0.85 ? 'high' : n > 0.7 ? 'medium' : 'low',
  statementTransform: (p) => p.summary, // Use summary as-is
  contextInference: (p) => ({
    when: inferWhen(p),
    why: p.rationale,
    conditions: inferConditions(p),
    tags: extractTags(p)
  })
}
```

**Files Created:**
```
packages/edda-promotion/src/
├── mapping/
│   ├── type-mapper.ts
│   ├── confidence-mapper.ts
│   ├── statement-transformer.ts
│   └── context-inferrer.ts
```

**Test Coverage:**
- Each proposal type → correct memory type
- Confidence 0.9 → 'high'
- Confidence 0.75 → 'medium'
- Confidence 0.6 → 'low'
- Statement extracted correctly
- Context inferred from rationale

---

#### Task 2.2: Metadata Transformation
**Estimate:** 1 day
**Owner:** [TBD]

**Description:**
Transform type-specific metadata from proposals.

**Acceptance Criteria:**
- [ ] Decision metadata extracted (alternatives, consequences)
- [ ] Pattern metadata extracted (applies_to, examples)
- [ ] Warning metadata extracted (severity, incident_refs)
- [ ] Constraint metadata extracted (type, workaround)
- [ ] Doctrine metadata extracted (category, ratified_by)
- [ ] Lesson metadata extracted (incident, cost, preventable)
- [ ] Unit tests for all metadata types

**Files Created:**
```
packages/edda-promotion/src/
├── mapping/
│   └── metadata-transformer.ts
```

**Test Coverage:**
- Decision proposal → DecisionMetadata
- Pattern proposal → PatternMetadata
- All fields correctly mapped
- Missing fields → defaults applied

---

#### Task 2.3: Scope & Authority Inference
**Estimate:** 1 day
**Owner:** [TBD]

**Description:**
Infer scope and authority settings from proposal context.

**Acceptance Criteria:**
- [ ] Scope inferred from observation sessions
- [ ] Authority owner set to proposing agent's team
- [ ] Visibility defaults to 'public' (configurable)
- [ ] Enforcement mode recommended based on type
- [ ] Unit tests for inference logic

**Inference Logic:**
```typescript
function inferScope(proposal: CandidateProposal): ScopeSpecifier {
  // If all observations from same repo → project scope
  // If observations span repos → global scope
  // If specific team mentioned in rationale → team scope

  const sessions = proposal.provenance.source_sessions
  const repos = getReposFromSessions(sessions)

  if (repos.length === 1) {
    return { type: 'project', identifier: repos[0] }
  } else {
    return { type: 'global' }
  }
}

function inferEnforcementMode(type: MemoryType): EnforcementMode {
  switch (type) {
    case 'constraint': return 'blocking'
    case 'warning': return 'warning'
    case 'decision': return 'warning'
    case 'pattern': return 'advisory'
    case 'doctrine': return 'blocking'
    case 'lesson': return 'advisory'
  }
}
```

**Files Created:**
```
packages/edda-promotion/src/
├── inference/
│   ├── scope-inferrer.ts
│   ├── authority-inferrer.ts
│   └── enforcement-recommender.ts
```

---

### Epic 3: Promotion Request System (3 days)

**Goal:** Manage promotion request lifecycle

#### Task 3.1: Promotion Request Manager
**Estimate:** 1.5 days
**Owner:** [TBD]

**Description:**
Implement promotion request creation and management.

**Acceptance Criteria:**
- [ ] Can create promotion request from proposal
- [ ] Generates proposed memory (transformation applied)
- [ ] Stores promotion request (Git YAML + SQLite)
- [ ] Can query pending requests
- [ ] Can update request status
- [ ] Unit tests for lifecycle

**Interface:**
```typescript
class PromotionRequestManager {
  async createRequest(
    proposalId: ProposalId,
    requestedBy: Principal
  ): Promise<PromotionRequest>

  async getRequest(id: PromotionRequestId): Promise<PromotionRequest>

  async listPending(filters?: {
    priority?: 'low' | 'normal' | 'high'
    types?: MemoryType[]
  }): Promise<PromotionRequest[]>

  async startReview(
    id: PromotionRequestId,
    reviewer: Principal
  ): Promise<void>

  async submitReview(
    id: PromotionRequestId,
    review: PromotionReview
  ): Promise<PromotionResult>
}
```

**Files Created:**
```
packages/edda-promotion/src/
├── request/
│   ├── promotion-request-manager.ts
│   ├── request-storage.ts
│   └── request-state-machine.ts
```

**Test Coverage:**
- Create request → stores in Git + index
- List pending → returns awaiting_review only
- Start review → status becomes under_review
- Submit review → creates memory or rejects

---

#### Task 3.2: Automatic Request Trigger
**Estimate:** 0.5 day
**Owner:** [TBD]

**Description:**
Automatically create promotion requests for high-confidence proposals.

**Acceptance Criteria:**
- [ ] Polls Ember for new high-confidence proposals
- [ ] Creates promotion request automatically
- [ ] Respects confidence thresholds per type
- [ ] Deduplicates (no duplicate requests)
- [ ] Unit tests for trigger logic

**Files Created:**
```
packages/edda-promotion/src/
├── triggers/
│   ├── auto-promotion-trigger.ts
│   └── deduplicator.ts
```

**Test Coverage:**
- High-confidence proposal → request created
- Low-confidence proposal → ignored
- Duplicate proposal → request not created
- Multiple proposals → all processed

---

#### Task 3.3: Priority Assignment
**Estimate:** 1 day
**Owner:** [TBD]

**Description:**
Assign priority to promotion requests based on signals.

**Acceptance Criteria:**
- [ ] Priority based on confidence, type, age
- [ ] High priority: confidence >0.9 OR critical warnings
- [ ] Normal priority: most proposals
- [ ] Low priority: confidence <0.7
- [ ] Can manually override priority
- [ ] Unit tests for priority logic

**Priority Logic:**
```typescript
function calculatePriority(proposal: CandidateProposal): Priority {
  if (proposal.confidence >= 0.9) return 'high'
  if (proposal.type === 'warning' && proposal.metadata.severity === 'critical') return 'high'
  if (proposal.confidence < 0.7) return 'low'
  return 'normal'
}
```

**Files Created:**
```
packages/edda-promotion/src/
├── priority/
│   └── priority-calculator.ts
```

---

### Epic 4: Diff Generation (2 days)

**Goal:** Show clear proposal → memory diffs for review

#### Task 4.1: Diff Generator Core
**Estimate:** 1 day
**Owner:** [TBD]

**Description:**
Generate structured diff showing transformation.

**Acceptance Criteria:**
- [ ] Shows proposal summary and rationale
- [ ] Shows proposed memory (after transformation)
- [ ] Highlights transformations applied
- [ ] Shows confidence mapping
- [ ] Shows scope inference
- [ ] Unit tests for diff generation

**Diff Structure:**
```typescript
interface PromotionDiff {
  proposal: {
    id: ProposalId
    type: ProposalType
    confidence: number
    summary: string
    rationale: string
  }

  memory: MemoryObjectExtended

  transformations: {
    typeMapping: string
    confidenceMapping: string
    scopeInference: string
    enforcementRecommendation: string
  }

  conflicts: ConflictDetection[]
  provenanceSummary: string
}
```

**Files Created:**
```
packages/edda-promotion/src/
├── diff/
│   ├── diff-generator.ts
│   └── transformation-summary.ts
```

**Test Coverage:**
- Generate diff → all sections present
- Transformation explanations clear
- Conflicts detected and shown

---

#### Task 4.2: Conflict Detection
**Estimate:** 1 day
**Owner:** [TBD]

**Description:**
Detect conflicts with existing memories.

**Acceptance Criteria:**
- [ ] Detect contradictions (opposing statements)
- [ ] Detect duplications (similar memories)
- [ ] Detect supersession candidates
- [ ] Severity scoring (low/medium/high)
- [ ] Explanation of conflict
- [ ] Unit tests for all conflict types

**Conflict Types:**
```typescript
type ConflictType = 'contradiction' | 'duplication' | 'supersession'

interface ConflictDetection {
  memoryId: MemoryId
  conflictType: ConflictType
  severity: 'low' | 'medium' | 'high'
  explanation: string
  recommendation?: string
}

// Example:
{
  memoryId: 'EDDA-M-pattern-01h1...',
  conflictType: 'duplication',
  severity: 'medium',
  explanation: 'Similar pattern exists: "Modern async patterns"',
  recommendation: 'Consider merging or clarifying differences'
}
```

**Files Created:**
```
packages/edda-promotion/src/
├── conflict/
│   ├── conflict-detector.ts
│   ├── contradiction-detector.ts
│   ├── duplication-detector.ts
│   └── supersession-detector.ts
```

**Test Coverage:**
- Contradicting statements → conflict detected
- Duplicate pattern → duplication detected
- Supersession candidate → detected with recommendation

---

### Epic 5: Review Workflow (3 days)

**Goal:** Human review interface via CLI

#### Task 5.1: Review CLI - List Command
**Estimate:** 0.5 day
**Owner:** [TBD]

**Description:**
CLI command to list pending promotion requests.

**Acceptance Criteria:**
- [ ] `anvil edda proposals` lists pending requests
- [ ] Shows ID, type, confidence, priority, age
- [ ] Filters by priority, type
- [ ] Sorts by priority (high first), then age
- [ ] Pagination support
- [ ] Colored output (priority-based)

**CLI Output:**
```
$ anvil edda proposals

Pending Promotion Requests (3)

[HIGH] EDDA-PR-01h6... decision (conf: 0.92) - 2 days ago
  "Use TypeScript for all new backend services"
  Proposed by: agent:anvil

[NORMAL] EDDA-PR-01h7... pattern (conf: 0.78) - 5 hours ago
  "Use async/await for async operations"
  Proposed by: agent:anvil

[NORMAL] EDDA-PR-01h8... warning (conf: 0.73) - 1 day ago
  "Avoid direct database schema changes"
  Proposed by: agent:anvil

Use: anvil edda promote <id> to review
```

**Files Created:**
```
apps/anvil-cli/src/commands/edda/
├── proposals.ts
└── proposals-formatter.ts
```

---

#### Task 5.2: Review CLI - Promote Command
**Estimate:** 1.5 days
**Owner:** [TBD]

**Description:**
Interactive CLI command for reviewing and promoting.

**Acceptance Criteria:**
- [ ] `anvil edda promote <id>` starts review
- [ ] Shows full diff (proposal vs proposed memory)
- [ ] Shows conflicts (if any)
- [ ] Shows provenance summary
- [ ] Prompts: approve, reject, edit, cancel
- [ ] Approve → creates memory, updates Ember
- [ ] Reject → records rejection with rationale
- [ ] Edit → allows modifications before approval

**CLI Flow:**
```
$ anvil edda promote EDDA-PR-01h6...

┌────────────────────────────────────────────────┐
│ Promotion Review: EDDA-PR-01h6...              │
└────────────────────────────────────────────────┘

PROPOSAL (from Ember)
Type: decision
Confidence: 0.92 (high confidence)
Proposed by: agent:anvil
Created: 2 days ago

Summary:
  Use TypeScript for all new backend services

Rationale:
  Based on 15 incidents where Python type errors caused
  production issues. TypeScript's static typing would
  reduce these by ~40% based on analysis.

Evidence:
  - 15 observations from 5 sessions
  - Pattern reinforced across multiple projects
  - Team expressed preference in discussions

────────────────────────────────────────────────

PROPOSED MEMORY (after transformation)

Type: decision
Statement:
  All new backend services will be written in TypeScript
  rather than Python.

Context:
  When: For new services started after 2025-01-01
  Why: Type safety reduces runtime errors by ~40%
  Conditions:
    - Applies to new services only (no forced migration)
    - Existing Python services can remain
    - Excludes ML/data science components

Scope: project:anvil
Confidence: high
Enforcement: warning

────────────────────────────────────────────────

TRANSFORMATIONS APPLIED

Type Mapping: decision → decision (no change)
Confidence: 0.92 → high (>0.85 threshold)
Scope: Inferred from session data (all in anvil project)
Enforcement: Recommended 'warning' for decisions

────────────────────────────────────────────────

CONFLICTS DETECTED: None

────────────────────────────────────────────────

PROVENANCE

Source Proposal: EMBER-P-decision-01h5...
Observations: 15 total
  - KINDLING-OBS-01h0... (error_recorded)
  - KINDLING-OBS-01h1... (error_recorded)
  - ... 13 more

Sessions: SESSION-042, SESSION-043, SESSION-044

────────────────────────────────────────────────

What would you like to do?

  [A]pprove  - Promote to Edda memory
  [R]eject   - Dismiss this proposal
  [E]dit     - Modify before approval
  [D]etails  - Show full proposal JSON
  [C]ancel   - Return without action

>
```

**Files Created:**
```
apps/anvil-cli/src/commands/edda/
├── promote.ts
├── promote-formatter.ts
└── interactive-prompt.ts
```

---

#### Task 5.3: Review CLI - Rejection Flow
**Estimate:** 1 day
**Owner:** [TBD]

**Description:**
Handle rejection with rationale capture.

**Acceptance Criteria:**
- [ ] Prompts for rejection category
- [ ] Prompts for detailed explanation
- [ ] Optionally suggest similar accepted examples
- [ ] Records rejection in Edda
- [ ] Updates Ember with rejection
- [ ] Generates feedback for Ember (confidence adjustment)

**Rejection Flow:**
```
> R (reject)

Why are you rejecting this proposal?

  [1] Insufficient evidence
  [2] Incorrect interpretation
  [3] Duplicate of existing memory
  [4] Out of scope
  [5] Not valuable enough
  [6] Conflicts with existing
  [7] Needs more observation

> 1

Provide detailed explanation (for learning):
> Only 15 observations across 5 sessions. Need at least
> 25 observations across 10+ sessions to establish this
> as a pattern. Current evidence is too thin.

Would you like to suggest improvements? (y/n)
> y

Suggestions:
> - Gather more observations (target: 25+)
> - Include observations from more diverse contexts
> - Interview team members for their preferences

────────────────────────────────────────────────

Rejection recorded.

- Ember will adjust confidence for similar proposals
- Agent trust score updated: 0.78 → 0.73 (-0.05)
- Feedback sent to Ember for learning

$
```

**Files Created:**
```
apps/anvil-cli/src/commands/edda/
├── reject.ts
└── rejection-prompt.ts
```

---

### Epic 6: Rejection Tracking & Learning (2 days)

**Goal:** Capture rejection signals for Ember improvement

#### Task 6.1: Rejection Recorder
**Estimate:** 1 day
**Owner:** [TBD]

**Description:**
Record rejection with structured learning signals.

**Acceptance Criteria:**
- [ ] Stores rejection record (Git YAML)
- [ ] Captures category, explanation, suggestions
- [ ] Marks false_positive flag
- [ ] Tracks duplicate_of if duplicate
- [ ] Generates Ember feedback
- [ ] Unit tests for all rejection types

**Rejection Record:**
```yaml
rejection_id: EDDA-REJ-01h8...
proposal_id: EMBER-P-decision-01h5...
rejected_by:
  type: human
  identifier: user:alice
rejected_at: "2025-01-16T10:30:00Z"

reason_category: insufficient_evidence
explanation: |
  Only 15 observations across 5 sessions. Need at least
  25 observations to establish pattern confidence.

# Learning signals
false_positive: false
insufficient_evidence: true
duplicate_of: null
policy_violation: null

# Feedback for Ember
ember_adjustment:
  adjust_confidence_by: -0.15
  adjust_factors:
    - factor: repetition
      new_weight: 0.8
  rationale: Require more observations for decision proposals
```

**Files Created:**
```
packages/edda-promotion/src/
├── rejection/
│   ├── rejection-recorder.ts
│   ├── learning-signal-generator.ts
│   └── ember-feedback-generator.ts
```

---

#### Task 6.2: Agent Trust Update
**Estimate:** 1 day
**Owner:** [TBD]

**Description:**
Update agent trust profiles based on promotion outcomes.

**Acceptance Criteria:**
- [ ] Increments proposals_approved on approval
- [ ] Increments proposals_rejected on rejection
- [ ] Recalculates trust_score
- [ ] Updates confidence_adjustment (-0.2 to +0.2)
- [ ] Updates requires_human_review flag
- [ ] Unit tests for trust calculation

**Trust Update Logic:**
```typescript
function updateTrustAfterPromotion(
  agentId: string,
  outcome: 'approved' | 'rejected',
  rejection?: RejectionRecord
): AgentTrustProfile {
  const profile = getTrustProfile(agentId)

  if (outcome === 'approved') {
    profile.proposals_approved++
    profile.factors.find(f => f.factor === 'historical_accuracy')!.current_value += 0.05
  } else {
    profile.proposals_rejected++
    profile.factors.find(f => f.factor === 'historical_accuracy')!.current_value -= 0.10

    if (rejection?.category === 'insufficient_evidence') {
      profile.factors.find(f => f.factor === 'reasoning_quality')!.current_value -= 0.05
    }
  }

  profile.approval_rate = profile.proposals_approved /
    (profile.proposals_approved + profile.proposals_rejected)

  profile.trust_score = calculateTrustScore(profile)
  profile.confidence_adjustment = (profile.trust_score - 0.5) * 0.4
  profile.requires_human_review = profile.trust_score < 0.7

  return profile
}
```

**Files Created:**
```
packages/edda-promotion/src/
├── trust/
│   ├── trust-updater.ts
│   └── trust-calculator.ts
```

---

### Epic 7: Integration & Testing (2 days)

**Goal:** End-to-end testing and documentation

#### Task 7.1: Integration Tests
**Estimate:** 1 day
**Owner:** [TBD]

**Description:**
Comprehensive end-to-end promotion workflows.

**Test Scenarios:**
```typescript
describe('Integration: Promotion Workflow', () => {
  test('full approval flow: proposal → request → review → memory', ...)
  test('rejection flow: proposal → request → review → rejection', ...)
  test('conflict detection during review', ...)
  test('agent trust updates after promotion', ...)
  test('provenance chain preserved in memory', ...)
  test('concurrent reviews handled correctly', ...)
})
```

**Files Created:**
```
packages/edda-promotion/tests/integration/
├── approval-flow.test.ts
├── rejection-flow.test.ts
├── conflict-detection.test.ts
└── trust-updates.test.ts
```

---

#### Task 7.2: Documentation
**Estimate:** 0.5 day
**Owner:** [TBD]

**Description:**
User and developer documentation.

**Acceptance Criteria:**
- [ ] User guide: How to review promotions
- [ ] CLI reference: All commands documented
- [ ] Developer guide: Extending promotion logic
- [ ] Architecture diagrams
- [ ] Troubleshooting guide

**Files Created:**
```
packages/edda-promotion/
├── README.md
└── docs/
    ├── user-guide.md
    ├── cli-reference.md
    ├── developer-guide.md
    └── architecture.md
```

---

#### Task 7.3: Performance & UX Testing
**Estimate:** 0.5 day
**Owner:** [TBD]

**Description:**
Validate performance and user experience.

**Acceptance Criteria:**
- [ ] Review workflow completes in <5 minutes
- [ ] CLI is responsive and intuitive
- [ ] Diff display is clear and informative
- [ ] Error messages are helpful
- [ ] User feedback collected and addressed

**Metrics:**
- Average review time: <5 minutes
- CLI responsiveness: <1s per action
- User satisfaction: qualitative feedback

---

## Dependencies

### External
- Ember service running and accessible
- Kindling service (for provenance validation)

### Internal
- Phase 0 complete (Memory Manager, Storage)
- Edda Core functional

### New Packages
- `@anvil/edda-promotion` (new)

---

## Risks & Mitigations

### Risk 1: Ember Integration Complexity
**Severity:** Medium
**Impact:** Cannot fetch proposals
**Probability:** Medium

**Mitigation:**
- Mock Ember API for development
- Stub Ember port for testing
- Gradual integration (start with mocked data)

---

### Risk 2: Poor Review UX
**Severity:** High
**Impact:** Humans won't use the system
**Probability:** Medium

**Mitigation:**
- Early UX testing with internal users
- Iterate on CLI design based on feedback
- Provide clear, actionable information in diffs

---

### Risk 3: Conflict Detection Inaccuracy
**Severity:** Medium
**Impact:** False positives/negatives
**Probability:** Medium

**Mitigation:**
- Start with simple heuristics
- Tune thresholds based on real usage
- Allow human override of conflict detection

---

## Success Metrics

### Functional
- ✅ Can promote proposals to memories
- ✅ Rejections capture useful learning signals
- ✅ Agent trust scores improve proposal quality over time

### Performance
- ✅ Review time <5 minutes average
- ✅ CLI responsive (<1s per action)
- ✅ Promotion request creation <500ms

### Quality
- ✅ 85%+ test coverage
- ✅ No critical bugs in integration tests
- ✅ User satisfaction (qualitative)

---

## Timeline

### Week 1 (Days 1-5)

**Day 1-2: Ember Integration**
- Task 1.1: Ember port (1d)
- Task 1.2: Kindling provenance (0.5d)
- Task 1.3: Configuration (0.5d)

**Day 3-5: Type Mapping**
- Task 2.1: Type mapping rules (1d)
- Task 2.2: Metadata transformation (1d)
- Task 2.3: Scope & authority inference (1d)

### Week 2 (Days 6-10)

**Day 6-8: Promotion Requests**
- Task 3.1: Request manager (1.5d)
- Task 3.2: Auto trigger (0.5d)
- Task 3.3: Priority assignment (1d)

**Day 9-10: Diff Generation**
- Task 4.1: Diff generator (1d)
- Task 4.2: Conflict detection (1d)

### Week 3 (Days 11-15)

**Day 11-13: Review Workflow**
- Task 5.1: List CLI (0.5d)
- Task 5.2: Promote CLI (1.5d)
- Task 5.3: Rejection flow (1d)

**Day 14-15: Learning & Testing**
- Task 6.1: Rejection recorder (1d)
- Task 6.2: Trust updates (1d)
- Task 7.1: Integration tests (1d)
- Task 7.2: Documentation (0.5d)
- Task 7.3: UX testing (0.5d)

---

## Deliverables

### Code
- [ ] `@anvil/edda-promotion` package
- [ ] Ember port client
- [ ] Type mapper and transformers
- [ ] Promotion request manager
- [ ] Diff generator with conflict detection
- [ ] CLI commands (proposals, promote, reject)
- [ ] Rejection tracker
- [ ] Agent trust updater
- [ ] 40+ unit tests
- [ ] 10+ integration tests

### Documentation
- [ ] User guide (how to review)
- [ ] CLI reference
- [ ] Developer guide
- [ ] Architecture diagrams

### Infrastructure
- [ ] CI pipeline for promotion package
- [ ] Integration with Ember (mocked initially)

---

## Sign-Off

### Definition of Done
- [ ] All tasks completed
- [ ] All tests passing (unit + integration)
- [ ] Code coverage ≥85%
- [ ] Review UX validated with users
- [ ] Documentation complete
- [ ] Code reviewed and approved
- [ ] Merged to main branch

### Approval
- [ ] Tech Lead: _______________
- [ ] Engineering Manager: _______________
- [ ] UX Designer: _______________ (CLI review)
- [ ] Date: _______________

---

## Next Phase

After Phase 1 completion, proceed to:
**Phase 2: Authority & Trust** (2 weeks)
- RBAC implementation
- Permission checks
- Agent trust profiles
- Audit trail

See: `/plans/aps/phase-2-authority.aps.md`

---

**Document Version:** 1.0.0
**Last Updated:** 2026-01-19
**Status:** Ready for approval (pending Phase 0 completion)
