# Edda System Architecture

**Status:** Draft for APS Planning
**Version:** 0.1.0
**Date:** 2026-01-19
**Purpose:** Comprehensive system architecture for Edda - the authoritative memory layer

---

## Executive Summary

Edda is the curated, authoritative memory layer that decides what becomes institutional truth. Unlike Kindling (capture) and Ember (interpretation), Edda stores typed, intentional knowledge with explicit provenance and human-in-the-loop approval.

**Core Philosophy:**
- Kindling captures → Ember suggests → Edda decides
- Harder to write than to read (friction is a feature)
- Nothing enters without intent
- Knowledge must be typed, scoped, and versioned

---

## System Overview

### Layer Positioning

```
┌─────────────────────────────────────────────────────────────┐
│                      Anvil Ecosystem                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Runtime Activity → Observations (Kindling)                 │
│                           ↓                                  │
│                    Candidates (Ember)                        │
│                           ↓                                  │
│              ┌────────────────────────┐                     │
│              │     Edda (Memory)      │                     │
│              │  Authoritative Truth   │                     │
│              └────────────────────────┘                     │
│                           ↓                                  │
│              Enforcement & Guidance Hooks                    │
│              (Policy, Pre-execution checks)                  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Trust Asymmetry

| Layer    | Trust Level | Write Barrier | Decay Strategy |
|----------|-------------|---------------|----------------|
| Kindling | Facts only  | None (write-only) | Time-based pruning |
| Ember    | Heuristic   | Confidence threshold | TTL expiry (30d) |
| Edda     | High        | Human approval | Explicit retirement |

---

## 1. Memory Objects (EDDA-CORE-001)

### 1.1 Typed Knowledge Schema

Edda stores **typed, intentional knowledge** - not notes, not logs, not chat history.

#### First-Class Memory Types

```typescript
type MemoryType =
  | 'decision'        // What was chosen, when, by whom, and why
  | 'pattern'         // Approved architectural patterns
  | 'warning'         // "Do not do X because Y happened"
  | 'constraint'      // Known limitations and boundaries
  | 'doctrine'        // Organisational principles and standards
  | 'lesson'          // Learnings from success/failure

// Future consideration (Phase 2+):
// | 'exception'       // Sanctioned deviations with expiry
// | 'historical'      // "This existed but no longer applies"
// | 'standard'        // Codified organisational rules
```

#### Memory Object Schema (EDDA-001 Extended)

```typescript
interface MemoryObject {
  // Identity & Classification
  id: MemoryId                    // EDDA-M-{type}-{ulid}
  type: MemoryType
  status: MemoryStatus            // 'active' | 'superseded' | 'retired'

  // Core Content
  statement: string               // 1-2000 chars: The remembered truth
  context: MemoryContext

  // Trust & Authority
  confidence: EddaConfidenceLevel // 'high' | 'medium' | 'low' | 'inferred'
  confidence_rationale?: string
  authority: AuthorityMetadata    // NEW: Who has power over this

  // Scope & Applicability
  scope: ScopeSpecifier           // NEW: Where this applies
  enforcement: EnforcementPolicy  // NEW: How to enforce

  // Provenance & Lineage
  provenance: ProvenanceChain     // Kindling → Ember → Edda
  attribution: Attribution
  evolution: EvolutionMetadata

  // Review & Decay
  review_policy: ReviewPolicy     // NEW: When to re-validate

  // Timestamps
  created_at: Timestamp
  updated_at: Timestamp

  // Extensibility
  metadata: Record<string, unknown>
}

interface MemoryContext {
  when: string                    // Temporal or conditional context
  why: string                     // Rationale for remembering
  conditions: string[]            // Applicability conditions
  scope?: string                  // Deprecated: use top-level scope
  tags: string[]                  // Categorisation
}

interface ScopeSpecifier {
  type: 'global' | 'workspace' | 'team' | 'project' | 'domain'
  identifier?: string             // e.g., team:platform, project:anvil
  exclusions?: string[]           // Explicit out-of-scope areas
}

interface EnforcementPolicy {
  mode: 'advisory' | 'warning' | 'blocking' | 'audit_only'
  hooks: EnforcementHook[]        // Which hooks enforce this
  override_requires?: AuthorityLevel[]  // Who can override
}

interface AuthorityMetadata {
  owner: Principal                // Who owns this memory
  reviewers: Principal[]          // Who can modify
  visibility: 'public' | 'team' | 'private'
}

interface Principal {
  type: 'human' | 'agent' | 'team' | 'system'
  identifier: string              // user:alice, team:platform, agent:anvil
}

interface ReviewPolicy {
  strategy: 'none' | 'time_based' | 'event_triggered' | 'usage_based'
  interval_days?: number          // For time_based
  trigger_events?: ReviewTrigger[] // For event_triggered
  usage_threshold?: number        // For usage_based
  last_reviewed_at?: Timestamp
}

type ReviewTrigger =
  | 'supersession_proposed'       // Someone wants to replace this
  | 'violation_threshold'         // Too many violations
  | 'contradiction_detected'      // Conflicts with new memory
  | 'staleness_detected'          // Not used in N days
```

### 1.2 Type-Specific Metadata

Each memory type has specialized metadata:

```typescript
// Decision Memory
interface DecisionMetadata {
  alternatives_considered: string[]
  consequences: { expected: string[], observed?: string[] }
  decision_maker: Principal
  irreversible: boolean
}

// Pattern Memory
interface PatternMetadata {
  applies_to: string[]            // Technologies, scenarios
  examples: CodeReference[]
  anti_patterns?: string[]
}

// Warning Memory
interface WarningMetadata {
  severity: 'low' | 'medium' | 'high' | 'critical'
  incident_references?: string[]  // Links to what went wrong
  mitigation?: string
}

// Constraint Memory
interface ConstraintMetadata {
  constraint_type: 'technical' | 'policy' | 'resource' | 'regulatory'
  workaround?: string
  expiry_condition?: string       // "Until migration complete"
}

// Doctrine Memory
interface DoctrineMetadata {
  principle_category: 'engineering' | 'security' | 'operations' | 'cultural'
  ratified_by?: string            // Team/leadership
  ratified_at?: Timestamp
}

// Lesson Memory
interface LessonMetadata {
  incident_id?: string
  cost?: { time?: string, money?: string, reputation?: string }
  preventable: boolean
}
```

### 1.3 Validation Rules

**Strict Enforcement (fail on violation):**
- Memory ID must match type: `EDDA-M-decision-*` for type=decision
- Statement must be 1-2000 characters
- Status transitions must be valid: active → superseded/retired only
- Provenance chain must be valid (if from Ember)
- Confidence + rationale required for non-high confidence
- Authority owner must exist
- Scope must be valid and non-empty

**Soft Warnings (log but allow):**
- Tags should follow convention (lowercase, kebab-case)
- Review policy should be set for active memories
- Enforcement mode should match memory type (warnings → warning/blocking)

---

## 2. Promotion Pipeline (EDDA-PIPELINE-001)

### 2.1 Promotion Workflow

```
Ember Proposal (active, confidence > threshold)
         ↓
    [Human Review]
         ↓
    ┌────┴────┐
    │         │
 Approve   Reject
    │         │
    ↓         ↓
  Edda    Dismissed
 Memory   (w/ rationale)
```

#### Promotion States

```typescript
type PromotionStatus =
  | 'awaiting_review'     // Queued for human review
  | 'under_review'        // Human is reviewing
  | 'approved'            // Will be promoted
  | 'rejected'            // Will not be promoted
  | 'needs_revision'      // Sent back to proposer

interface PromotionRequest {
  id: PromotionRequestId       // EDDA-PR-{ulid}
  proposal_id: ProposalId      // Source Ember proposal
  status: PromotionStatus

  // Transformation
  proposed_memory: MemoryObject // What will be created
  transformation_notes?: string // How proposal → memory

  // Review
  reviewer?: Principal
  review_started_at?: Timestamp
  review_completed_at?: Timestamp
  decision_rationale?: string

  // Metadata
  requested_by: Principal      // Agent or human who initiated
  requested_at: Timestamp
  priority: 'low' | 'normal' | 'high'
}
```

### 2.2 Promotion Triggers

**Automatic (Agent-Initiated):**
1. Ember proposal reaches high confidence (>0.85)
2. Proposal reinforced by multiple sessions
3. Pattern observed N times (configurable)
4. Critical warning detected

**Manual (Human-Initiated):**
1. Human reviews Ember dashboard
2. Explicit `anvil edda promote <proposal-id>` command
3. Incident post-mortem creates lessons

### 2.3 Review Interface

```typescript
interface PromotionReview {
  // Decision
  decision: 'approve' | 'reject' | 'revise'
  rationale: string                  // Required for all decisions

  // Modifications (for approve/revise)
  modifications?: MemoryObjectPatch

  // Reviewer context
  reviewer: Principal
  reviewed_at: Timestamp

  // Additional signals
  consulted_with?: Principal[]       // Who else was consulted
  related_memories?: MemoryId[]      // Conflicts or dependencies
}

interface MemoryObjectPatch {
  statement?: string
  context?: Partial<MemoryContext>
  confidence?: EddaConfidenceLevel
  confidence_rationale?: string
  authority?: Partial<AuthorityMetadata>
  scope?: Partial<ScopeSpecifier>
  enforcement?: Partial<EnforcementPolicy>
  review_policy?: Partial<ReviewPolicy>
}
```

### 2.4 Diff-Based Review

Reviews show structured diffs:

```typescript
interface PromotionDiff {
  proposal: CandidateProposal        // Original Ember proposal
  memory: MemoryObject               // Proposed memory

  transformations: {
    type_mapping: string             // proposal.type → memory.type
    confidence_mapping: string       // numeric → semantic
    scope_inference: string          // How scope was determined
    enforcement_recommendation: string
  }

  conflicts: ConflictDetection[]     // Existing memories this conflicts with
  provenance_summary: string         // Human-readable chain
}

interface ConflictDetection {
  memory_id: MemoryId
  conflict_type: 'contradiction' | 'duplication' | 'supersession'
  severity: 'low' | 'medium' | 'high'
  explanation: string
}
```

### 2.5 Rejection Signals

Rejections create valuable training data:

```typescript
interface RejectionRecord {
  rejection_id: RejectionId          // EDDA-REJ-{ulid}
  proposal_id: ProposalId
  rejected_by: Principal
  rejected_at: Timestamp

  reason_category: RejectionCategory
  explanation: string

  // Learning signals
  false_positive: boolean            // Ember wrongly elevated
  insufficient_evidence: boolean
  duplicate_of?: MemoryId
  policy_violation?: string

  // Feedback loop
  ember_adjustment?: EmberFeedback   // How to tune Ember confidence
}

type RejectionCategory =
  | 'insufficient_evidence'
  | 'incorrect_interpretation'
  | 'duplicate'
  | 'out_of_scope'
  | 'not_valuable'
  | 'conflicts_with_existing'
  | 'needs_more_observation'
```

### 2.6 Versioning & Lineage

Every memory mutation creates a new version:

```typescript
interface MemoryVersion {
  version: number                    // Monotonic
  memory_id: MemoryId               // Stable across versions
  snapshot: MemoryObject            // Full state at this version
  change_type: 'created' | 'updated' | 'superseded' | 'retired'
  changed_by: Principal
  changed_at: Timestamp
  change_reason: string
  diff?: MemoryObjectPatch          // What changed (for updates)
}

interface EvolutionChain {
  root_memory_id: MemoryId          // Original
  versions: MemoryVersion[]
  current_version: number
  supersession_tree?: SupersessionNode[]
}

interface SupersessionNode {
  memory_id: MemoryId
  supersedes: MemoryId[]            // Parents
  superseded_by?: MemoryId          // Child (if retired)
  active: boolean
}
```

---

## 3. Authority & Trust Model (EDDA-AUTH-001)

### 3.1 Authority Levels

```typescript
type AuthorityLevel =
  | 'system'          // System-generated (highest trust)
  | 'org_admin'       // Organisation administrator
  | 'team_lead'       // Team or domain lead
  | 'contributor'     // Regular contributor
  | 'agent'           // AI agent (propose only)
  | 'readonly'        // Read-only access

interface AuthorityPolicy {
  level: AuthorityLevel
  permissions: Permission[]
  constraints?: AuthorityConstraint[]
}

type Permission =
  | 'read_public'           // Read public memories
  | 'read_team'             // Read team-scoped memories
  | 'read_all'              // Read all memories
  | 'propose_memory'        // Initiate promotion requests
  | 'review_promotions'     // Approve/reject promotions
  | 'create_memory_direct'  // Create without promotion pipeline
  | 'update_memory'         // Modify existing memories
  | 'retire_memory'         // Mark memories as retired
  | 'configure_enforcement' // Set enforcement policies
  | 'manage_authority'      // Grant/revoke permissions

interface AuthorityConstraint {
  type: 'scope_limited' | 'type_limited' | 'quota_limited' | 'approval_required'
  details: Record<string, unknown>
}
```

### 3.2 Role-Based Access Control (RBAC)

```typescript
interface Role {
  role_id: string               // team:platform:lead
  name: string
  authority_level: AuthorityLevel
  permissions: Permission[]

  // Scope constraints
  scope_restriction?: ScopeSpecifier

  // Members
  principals: Principal[]
}

// Predefined Roles
const DefaultRoles: Role[] = [
  {
    role_id: 'org:admin',
    name: 'Organisation Administrator',
    authority_level: 'org_admin',
    permissions: ['read_all', 'review_promotions', 'create_memory_direct',
                  'update_memory', 'retire_memory', 'configure_enforcement',
                  'manage_authority'],
    principals: []
  },
  {
    role_id: 'team:lead',
    name: 'Team Lead',
    authority_level: 'team_lead',
    permissions: ['read_all', 'review_promotions', 'update_memory',
                  'retire_memory', 'configure_enforcement'],
    scope_restriction: { type: 'team', identifier: '{team_id}' },
    principals: []
  },
  {
    role_id: 'contributor',
    name: 'Contributor',
    authority_level: 'contributor',
    permissions: ['read_public', 'read_team', 'propose_memory'],
    principals: []
  },
  {
    role_id: 'agent',
    name: 'AI Agent',
    authority_level: 'agent',
    permissions: ['read_public', 'propose_memory'],
    principals: []
  }
]
```

### 3.3 Trust Weighting

Agents have trust scores that influence their proposals:

```typescript
interface AgentTrustProfile {
  agent_id: string              // agent:anvil, agent:copilot
  trust_score: number           // 0.0 - 1.0

  // Historical performance
  proposals_submitted: number
  proposals_approved: number
  proposals_rejected: number
  approval_rate: number         // Auto-calculated

  // Trust factors
  factors: TrustFactor[]

  // Permissions
  can_propose: boolean
  confidence_adjustment: number // -0.2 to +0.2 applied to proposals
  requires_human_review: boolean

  last_updated: Timestamp
}

interface TrustFactor {
  factor: 'historical_accuracy' | 'source_quality' | 'reasoning_quality' | 'domain_expertise'
  weight: number                // 0.0 - 1.0
  rationale: string
}
```

### 3.4 Audit Trail

Every operation is logged:

```typescript
interface AuditEntry {
  audit_id: string              // EDDA-AUDIT-{ulid}
  timestamp: Timestamp

  // Who
  principal: Principal
  authority_level: AuthorityLevel

  // What
  operation: AuditOperation
  target_type: 'memory' | 'promotion' | 'authority' | 'config'
  target_id: string

  // Details
  changes?: Record<string, unknown>
  rationale?: string

  // Context
  session_id?: string
  ip_address?: string
}

type AuditOperation =
  | 'memory_created'
  | 'memory_updated'
  | 'memory_retired'
  | 'promotion_approved'
  | 'promotion_rejected'
  | 'authority_granted'
  | 'authority_revoked'
  | 'enforcement_configured'
  | 'memory_queried'            // Optional: for sensitive memories
```

---

## 4. Query & Retrieval (EDDA-QUERY-001)

### 4.1 Query Interface

```typescript
interface EddaQuery {
  // Type & Status Filters
  types?: MemoryType[]
  statuses?: MemoryStatus[]

  // Scope Filters
  scope?: ScopeSpecifier
  tags?: string[]

  // Confidence Filters
  min_confidence?: EddaConfidenceLevel

  // Authority Filters
  owner?: Principal
  visibility?: ('public' | 'team' | 'private')[]

  // Temporal Filters
  created_after?: Timestamp
  created_before?: Timestamp
  updated_after?: Timestamp

  // Text Search
  search_text?: string          // Full-text search in statement + context

  // Pagination
  limit?: number
  offset?: number
  sort_by?: 'created_at' | 'updated_at' | 'confidence' | 'relevance'
  sort_order?: 'asc' | 'desc'
}

interface EddaQueryResult {
  memories: MemoryObject[]
  total_count: number
  page_info: PageInfo

  // Aggregations (optional)
  facets?: {
    by_type?: Record<MemoryType, number>
    by_status?: Record<MemoryStatus, number>
    by_confidence?: Record<EddaConfidenceLevel, number>
  }
}
```

### 4.2 Semantic Retrieval

Beyond exact queries, Edda supports semantic search:

```typescript
interface SemanticQuery {
  query: string                 // Natural language query
  scope?: ScopeSpecifier
  limit?: number

  // Filters (applied after semantic ranking)
  filters?: Partial<EddaQuery>
}

interface SemanticResult extends EddaQueryResult {
  memories: MemoryObjectWithRelevance[]
}

interface MemoryObjectWithRelevance {
  memory: MemoryObject
  relevance_score: number       // 0.0 - 1.0
  match_explanation: string     // Why this was returned
}
```

### 4.3 Conflict Detection

Query for potential conflicts:

```typescript
interface ConflictQuery {
  memory_id?: MemoryId          // Check against specific memory
  statement?: string            // Check against proposed statement
  scope?: ScopeSpecifier

  conflict_types?: ('contradiction' | 'duplication' | 'supersession')[]
}

interface ConflictResult {
  conflicts: ConflictDetection[]
  confidence: number            // 0.0 - 1.0 (how sure about conflicts)
}
```

### 4.4 Temporal Queries

"What was true at time T?":

```typescript
interface TemporalQuery {
  as_of: Timestamp              // Point in time
  memory_id?: MemoryId          // Specific memory
  query?: EddaQuery             // Or general query
}

interface TemporalResult {
  memories: MemoryObject[]      // State as of that time
  snapshot_info: {
    requested_time: Timestamp
    actual_time: Timestamp      // Closest available
    version_numbers: Record<MemoryId, number>
  }
}
```

### 4.5 Provenance Queries

Trace memory lineage:

```typescript
interface ProvenanceQuery {
  memory_id: MemoryId
  include_kindling?: boolean    // Trace all the way to observations
  include_versions?: boolean    // Include all versions
}

interface ProvenanceResult {
  memory: MemoryObject
  chain: ProvenanceChain

  // Full lineage
  ember_proposal?: CandidateProposal
  kindling_observations?: Observation[]

  // Evolution
  versions?: MemoryVersion[]
  supersession_chain?: MemoryObject[]

  // Visual representation
  graph?: ProvenanceGraph
}

interface ProvenanceGraph {
  nodes: ProvenanceNode[]
  edges: ProvenanceEdge[]
}

interface ProvenanceNode {
  id: string
  type: 'observation' | 'proposal' | 'memory' | 'version'
  label: string
  metadata: Record<string, unknown>
}

interface ProvenanceEdge {
  from: string
  to: string
  relationship: 'observed' | 'proposed' | 'promoted' | 'superseded' | 'versioned'
}
```

### 4.6 Explain-Why Responses

Every query result includes provenance:

```typescript
interface ExplainableResult {
  memory: MemoryObject
  explanation: {
    why_returned: string        // Why this matched the query
    confidence_basis: string    // Why this confidence level
    authority_basis: string     // Who vouches for this
    provenance_summary: string  // Where this came from
    last_validated: Timestamp   // When last reviewed
  }
}
```

---

## 5. Enforcement & Guidance Hooks (EDDA-ENFORCE-001)

### 5.1 Enforcement Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Anvil Execution                       │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  Pre-Execution      During Execution      Post-Execution│
│      ↓                    ↓                     ↓        │
│  ┌────────┐          ┌────────┐          ┌────────┐    │
│  │ Policy │          │Guidance│          │ Learn  │    │
│  │ Check  │          │Surfacing│         │ Signal │    │
│  └────┬───┘          └────┬───┘          └────┬───┘    │
│       │                   │                    │        │
│       └───────────────────┼────────────────────┘        │
│                           │                             │
│                      ┌────▼────┐                        │
│                      │  Edda   │                        │
│                      │Memories │                        │
│                      └─────────┘                        │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### 5.2 Hook Types

```typescript
type EnforcementHookType =
  | 'pre_execution'         // Before action runs
  | 'validation'            // During planning/validation
  | 'guidance'              // Soft warnings/suggestions
  | 'post_execution'        // After action completes
  | 'approval_required'     // Human-in-loop gate

interface EnforcementHook {
  hook_id: string           // EDDA-HOOK-{ulid}
  type: EnforcementHookType
  name: string
  description: string

  // What triggers this hook
  trigger: HookTrigger

  // What memories apply
  applicable_memories: MemoryMatcher

  // What action to take
  action: HookAction

  // Configuration
  enabled: boolean
  priority: number          // Execution order
}

interface HookTrigger {
  event: HookEvent
  conditions?: TriggerCondition[]
}

type HookEvent =
  | 'plan_created'
  | 'action_about_to_execute'
  | 'file_about_to_change'
  | 'command_about_to_run'
  | 'gate_evaluated'
  | 'human_approval_requested'

interface TriggerCondition {
  field: string             // e.g., 'action.type', 'file.path'
  operator: '==' | '!=' | 'contains' | 'matches'
  value: unknown
}

interface MemoryMatcher {
  types?: MemoryType[]
  tags?: string[]
  scope?: ScopeSpecifier
  enforcement_modes?: ('advisory' | 'warning' | 'blocking')[]
}

interface HookAction {
  mode: 'block' | 'warn' | 'suggest' | 'log' | 'require_approval'
  message_template: string  // Can reference {memory.statement}, etc.

  // For suggestions
  alternatives?: string[]

  // For approvals
  approval_required_from?: AuthorityLevel[]
}
```

### 5.3 Pre-Execution Checks

Before executing actions, check against Edda:

```typescript
interface PreExecutionCheck {
  // Context
  action: ActionContext
  plan?: PlanContext

  // Check request
  check_type: 'policy' | 'constraint' | 'warning'

  // Results
  result: CheckResult
}

interface ActionContext {
  action_type: string       // 'file_write', 'shell_command', etc.
  action_details: Record<string, unknown>
  scope: ScopeSpecifier
}

interface CheckResult {
  allowed: boolean
  violations: Violation[]
  warnings: Warning[]
  suggestions: Suggestion[]
}

interface Violation {
  memory_id: MemoryId
  memory: MemoryObject
  violation_type: 'hard_constraint' | 'policy_violation' | 'blocked_pattern'
  message: string
  can_override: boolean
  override_requires?: AuthorityLevel[]
}

interface Warning {
  memory_id: MemoryId
  memory: MemoryObject
  severity: 'low' | 'medium' | 'high'
  message: string
  recommendation?: string
}

interface Suggestion {
  memory_id: MemoryId
  memory: MemoryObject
  suggestion_type: 'alternative_approach' | 'best_practice' | 'reference'
  message: string
}
```

### 5.4 Contextual Guidance

Surface relevant knowledge during planning:

```typescript
interface GuidanceRequest {
  context: PlanningContext
  limit?: number
}

interface PlanningContext {
  intent: string            // What user is trying to do
  scope: ScopeSpecifier
  technologies?: string[]
  current_phase?: 'planning' | 'implementing' | 'testing'
}

interface GuidanceResponse {
  relevant_memories: RelevantMemory[]
  patterns_to_consider: MemoryObject[]
  warnings_to_avoid: MemoryObject[]
  lessons_learned: MemoryObject[]
}

interface RelevantMemory {
  memory: MemoryObject
  relevance_score: number
  why_relevant: string
  when_to_apply: string
}
```

### 5.5 Soft vs Hard Enforcement

```typescript
interface EnforcementDecision {
  memory: MemoryObject
  enforcement_mode: 'advisory' | 'warning' | 'blocking' | 'audit_only'

  // Decision logic
  should_enforce: boolean
  enforcement_action: EnforcementAction
}

type EnforcementAction =
  | { type: 'allow', log: boolean }
  | { type: 'warn', message: string, proceed_allowed: boolean }
  | { type: 'block', message: string, override_path?: string }
  | { type: 'require_approval', approvers: Principal[], message: string }

// Example enforcement logic
function determineEnforcement(memory: MemoryObject, context: ActionContext): EnforcementDecision {
  switch (memory.enforcement.mode) {
    case 'advisory':
      return { allow: true, log: true }

    case 'warning':
      return { warn: constructWarningMessage(memory), proceed_allowed: true }

    case 'blocking':
      const canOverride = hasAuthority(context.principal, memory.enforcement.override_requires)
      return {
        block: constructBlockMessage(memory),
        override_path: canOverride ? 'Use --force with justification' : undefined
      }

    case 'audit_only':
      return { allow: true, log: true, audit: true }
  }
}
```

### 5.6 Override Mechanism

```typescript
interface OverrideRequest {
  violation_id: string
  memory_id: MemoryId
  requester: Principal
  justification: string

  // Context
  action: ActionContext
  original_check: CheckResult
}

interface OverrideDecision {
  approved: boolean
  decided_by: Principal
  decision_rationale: string

  // Audit
  audit_entry_id: string

  // Follow-up
  requires_review?: boolean
  review_deadline?: Timestamp
}
```

---

## 6. Change Management & Decay (EDDA-LIFECYCLE-001)

### 6.1 Deprecation Workflow

```typescript
interface DeprecationRequest {
  memory_id: MemoryId
  reason: DeprecationReason
  proposed_by: Principal

  // Migration
  superseded_by?: MemoryId
  migration_guide?: string

  // Timeline
  deprecation_date: Timestamp
  retirement_date: Timestamp   // When fully removed

  // Impact
  estimated_impact: ImpactAssessment
}

type DeprecationReason =
  | 'superseded'            // Newer, better knowledge exists
  | 'obsolete'              // No longer relevant
  | 'incorrect'             // Was wrong
  | 'consolidated'          // Merged into another memory

interface ImpactAssessment {
  affected_systems: string[]
  affected_teams: string[]
  dependent_memories: MemoryId[]
  enforcement_hooks_count: number
  estimated_effort: 'low' | 'medium' | 'high'
}
```

### 6.2 Expiry & Review Timers

```typescript
interface ReviewSchedule {
  memory_id: MemoryId
  review_policy: ReviewPolicy

  // Schedule
  next_review_due: Timestamp
  review_history: ReviewEvent[]

  // Staleness
  staleness_score: number       // 0.0 (fresh) - 1.0 (stale)
  staleness_factors: StalenessFactor[]
}

interface ReviewEvent {
  reviewed_at: Timestamp
  reviewed_by: Principal
  outcome: ReviewOutcome
  notes: string
}

type ReviewOutcome =
  | 'reaffirmed'            // Still valid
  | 'updated'               // Modified
  | 'extended_review'       // Pushed next review
  | 'deprecated'            // Marked for retirement

interface StalenessFactor {
  factor: 'time_since_creation' | 'time_since_last_use' | 'contradicted_by_new_data' | 'unused'
  weight: number
  contribution: number      // To staleness score
}
```

### 6.3 Supersession Tracking

```typescript
interface SupersessionRequest {
  old_memory_id: MemoryId
  new_memory: MemoryObjectInput

  supersession_type: 'replacement' | 'refinement' | 'consolidation'
  relationship: string      // How new relates to old

  // Transition
  transition_plan?: string
  backward_compatibility?: boolean
  cutover_date?: Timestamp
}

interface SupersessionResult {
  old_memory: MemoryObject  // Now status=superseded
  new_memory: MemoryObject  // Status=active

  // Linkage
  evolution_link: EvolutionMetadata

  // Impact
  updated_references: ReferenceUpdate[]
  enforcement_migrations: EnforcementMigration[]
}

interface ReferenceUpdate {
  referencing_memory_id: MemoryId
  field: string
  old_value: MemoryId
  new_value: MemoryId
}

interface EnforcementMigration {
  hook_id: string
  old_memory_id: MemoryId
  new_memory_id: MemoryId
  requires_reconfiguration: boolean
}
```

### 6.4 Historical Visibility

Retired memories remain queryable:

```typescript
interface HistoricalQuery extends EddaQuery {
  include_retired?: boolean
  include_superseded?: boolean

  // Temporal range
  active_during?: {
    start: Timestamp
    end: Timestamp
  }
}

interface HistoricalResult extends EddaQueryResult {
  memories: HistoricalMemoryObject[]
}

interface HistoricalMemoryObject extends MemoryObject {
  // Additional context
  was_active: boolean
  active_period?: {
    start: Timestamp
    end: Timestamp
  }

  // Why no longer active
  retirement_info?: {
    retired_at: Timestamp
    retired_by: Principal
    reason: string
    superseded_by?: MemoryId
  }
}
```

### 6.5 Aggressive Forgetting

Edda forgets by default - memories must justify existence:

```typescript
interface RetentionPolicy {
  // Automatic retirement
  auto_retire_after_days?: number
  auto_retire_if_unused_days?: number

  // Protection
  protected_types?: MemoryType[]        // Never auto-retire
  protected_tags?: string[]

  // Review requirements
  require_review_every_days?: number
  retire_if_not_reviewed?: boolean
}

interface ForgettingReport {
  candidates_for_retirement: MemoryObject[]
  reasons: RetirementCandidate[]

  // Recommendations
  safe_to_retire: MemoryId[]
  requires_human_review: MemoryId[]
  protected: MemoryId[]
}

interface RetirementCandidate {
  memory_id: MemoryId
  reason: 'unused' | 'expired_review' | 'contradicted' | 'obsolete'
  last_used?: Timestamp
  contradiction_count?: number
  review_overdue_by_days?: number
}
```

---

## 7. Agent Interaction Contract (EDDA-AGENT-001)

### 7.1 Agent Permissions

```typescript
interface AgentCapabilities {
  agent_id: string

  // Read permissions
  can_read_public: boolean
  can_read_team_scoped: boolean
  can_read_private: boolean

  // Write permissions
  can_propose_memory: boolean
  can_annotate: boolean
  can_ratify: boolean          // Always false for v1
  can_delete: boolean          // Always false for v1

  // Scope restrictions
  scope_access: ScopeSpecifier[]

  // Trust
  trust_profile: AgentTrustProfile
}
```

### 7.2 Proposal Creation

Agents create proposals, never memories directly:

```typescript
interface AgentProposal {
  // Agent context
  agent_id: string
  agent_session_id: string

  // Proposal content
  proposed_memory: MemoryObjectInput

  // Evidence
  evidence: Evidence[]
  reasoning: string             // Why agent thinks this is valuable

  // Confidence
  agent_confidence: number      // 0.0 - 1.0
  confidence_factors: ConfidenceFactor[]
}

interface Evidence {
  source_type: 'observation' | 'pattern_match' | 'semantic_similarity' | 'human_feedback'
  source_id: string
  weight: number                // How much this contributes to confidence
  summary: string
}

interface ConfidenceFactor {
  factor: string                // 'repetition', 'consensus', 'source_quality'
  value: number
  weight: number
}
```

### 7.3 Agent Citation

Agents must cite Edda when acting on memories:

```typescript
interface AgentAction {
  action_id: string
  agent_id: string

  // What action
  action_type: string
  action_details: Record<string, unknown>

  // Why (Edda-backed)
  cited_memories: MemoryCitation[]
  reasoning: string
}

interface MemoryCitation {
  memory_id: MemoryId
  memory: MemoryObject

  // How this memory informed the action
  influence_type: 'constraint' | 'guidance' | 'pattern' | 'requirement'
  application: string           // How agent applied this memory
}
```

### 7.4 Reasoned Feedback on Rejection

When proposals are rejected, agents receive structured feedback:

```typescript
interface RejectionFeedback {
  proposal_id: ProposalId
  rejection: RejectionRecord

  // Feedback for learning
  what_was_wrong: string
  how_to_improve: string[]
  similar_accepted_examples?: MemoryId[]

  // Trust adjustment
  trust_impact: number          // -0.1 to 0 (rejection lowers trust)
  new_trust_score: number
}
```

### 7.5 Agent Query Interface

Agents query Edda with structured requests:

```typescript
interface AgentQueryRequest {
  agent_id: string

  // Query
  query_type: 'guidance' | 'constraint_check' | 'pattern_lookup' | 'decision_history'
  query: EddaQuery | SemanticQuery | ConflictQuery

  // Context
  context?: {
    current_task?: string
    scope?: ScopeSpecifier
    technologies?: string[]
  }
}

interface AgentQueryResponse extends EddaQueryResult {
  // Additional agent-specific data
  how_to_use: string[]          // Actionable guidance
  related_patterns?: MemoryId[]
  warnings?: MemoryId[]

  // Citation instructions
  cite_as: string               // How to cite in action logs
}
```

---

## 8. Interop & Export (EDDA-INTEROP-001)

### 8.1 API Access

```typescript
// REST API endpoints
interface EddaAPI {
  // Read
  'GET /memories': (query: EddaQuery) => EddaQueryResult
  'GET /memories/:id': (id: MemoryId) => MemoryObject
  'GET /memories/:id/provenance': (id: MemoryId) => ProvenanceResult
  'GET /memories/:id/history': (id: MemoryId) => MemoryVersion[]

  // Search
  'POST /memories/search': (query: SemanticQuery) => SemanticResult
  'POST /memories/conflicts': (query: ConflictQuery) => ConflictResult

  // Promotion (write)
  'POST /promotions': (request: PromotionRequest) => PromotionRequest
  'POST /promotions/:id/review': (id: string, review: PromotionReview) => PromotionResult

  // Lifecycle
  'PATCH /memories/:id': (id: MemoryId, patch: MemoryObjectPatch) => MemoryObject
  'POST /memories/:id/retire': (id: MemoryId, reason: string) => MemoryObject
  'POST /memories/:id/supersede': (id: MemoryId, request: SupersessionRequest) => SupersessionResult

  // Export
  'GET /export': (query?: EddaQuery) => MemoryExport
  'POST /import': (data: MemoryExport) => ImportResult

  // Meta
  'GET /stats': () => EddaStats
  'GET /health': () => HealthCheck
}
```

### 8.2 Snapshot Exports

```typescript
interface MemoryExport {
  // Metadata
  export_id: string
  exported_at: Timestamp
  exported_by: Principal

  // Data
  memories: MemoryObject[]

  // Provenance (optional)
  include_provenance: boolean
  provenance_chains?: ProvenanceChain[]

  // Version info
  schema_version: string
  edda_version: string
}

interface ImportResult {
  imported_count: number
  skipped_count: number
  errors: ImportError[]

  created_memory_ids: MemoryId[]
  skipped_memory_ids: MemoryId[]
}

interface ImportError {
  memory_id?: MemoryId
  error_type: 'validation_failed' | 'duplicate' | 'conflict' | 'permission_denied'
  message: string
}
```

### 8.3 Human-Readable Views

```typescript
interface MemoryMarkdownExport {
  memory: MemoryObject

  // Formatted output
  markdown: string              // Full markdown representation

  // Structure
  sections: {
    header: string              // # Decision: Use TypeScript for backend
    statement: string           // ## Statement
    context: string             // ## Context (when, why, conditions)
    provenance: string          // ## Provenance
    evolution: string           // ## Evolution (supersedes/superseded by)
    metadata: string            // ## Additional Details
  }
}

// Example output:
/*
# Decision: Use TypeScript for backend

**Status:** Active
**Confidence:** High
**Scope:** project:anvil
**Owner:** team:platform

## Statement

All new backend services in Anvil will be written in TypeScript rather than Python.

## Context

**When:** For all new services started after 2025-01-01

**Why:** Type safety reduces runtime errors by ~40% based on our incident analysis. Team expertise is stronger in TypeScript.

**Conditions:**
- Applies to new services only (no forced migration)
- Existing Python services can remain unless major refactor
- Excludes ML/data science components (Python preferred)

## Provenance

**Promoted by:** user:alice (2025-01-15)
**From Ember proposal:** EMBER-P-decision-01h...
**Based on observations:** 23 incidents, 15 successful TypeScript projects

## Evolution

**Supersedes:** None
**Superseded by:** None

## Metadata

```json
{
  "decision_maker": "team:platform",
  "alternatives_considered": ["Python", "Go", "Rust"],
  "consequences": {
    "expected": ["Reduced runtime errors", "Faster onboarding"],
    "observed": []
  }
}
```
*/
```

### 8.4 Tool-Specific Projections

```typescript
// Anvil-specific projection
interface AnvilProjection {
  // Enforcement hooks
  gates: GateDefinition[]
  constraints: ConstraintDefinition[]

  // Guidance
  patterns: PatternDefinition[]
  warnings: WarningDefinition[]
}

// GitHub Actions projection
interface CIProjection {
  // Pre-commit hooks
  pre_commit_checks: CheckDefinition[]

  // PR rules
  pr_requirements: PRRequirement[]
}

// Documentation site projection
interface DocsProjection {
  // Architecture decision records (ADRs)
  adrs: ADRDocument[]

  // Best practices
  guides: GuideDocument[]
}
```

---

## 9. UX & Experience (EDDA-UX-001)

### 9.1 CLI Interface

```bash
# Query & Read
anvil edda list [--type=decision] [--status=active] [--scope=team:platform]
anvil edda show <memory-id>
anvil edda search "how should we handle auth?"
anvil edda trace <memory-id>    # Show provenance

# Promotion
anvil edda proposals            # List pending promotions
anvil edda promote <proposal-id> [--with-changes]
anvil edda reject <proposal-id> --reason="..."

# Lifecycle
anvil edda update <memory-id>
anvil edda retire <memory-id> --reason="..."
anvil edda supersede <old-id> <new-id>

# Governance
anvil edda conflicts            # Show conflicting memories
anvil edda review-due           # Show memories needing review
anvil edda stale                # Show stale memories

# Export
anvil edda export [--format=json|yaml|markdown] [--output=file]
anvil edda import <file>

# Stats
anvil edda stats
anvil edda audit [--user=alice] [--days=30]
```

### 9.2 Visual Cues

Terminal output uses formatting to indicate confidence & status:

```
[ACTIVE] [HIGH CONFIDENCE] Decision: Use TypeScript for backend
└─ Scope: project:anvil
└─ Owner: team:platform
└─ Created: 2025-01-15 (42 days ago)
└─ Last reviewed: 2025-01-15

[SUPERSEDED] [MEDIUM CONFIDENCE] Decision: Use Python for backend
└─ Superseded by: EDDA-M-decision-02h...
└─ Retired: 2025-01-15

[RETIRED] [LOW CONFIDENCE] Pattern: Manual deployment process
└─ Retired reason: Automated with GitHub Actions
└─ Historical: 2023-06-10 → 2024-11-20
```

### 9.3 Narrative Views

Show "how we got here":

```typescript
interface NarrativeView {
  memory_id: MemoryId
  memory: MemoryObject

  // Story
  narrative: NarrativeBlock[]
}

interface NarrativeBlock {
  timestamp: Timestamp
  event_type: 'observed' | 'proposed' | 'discussed' | 'promoted' | 'updated' | 'superseded'
  actor: Principal
  description: string
  context?: Record<string, unknown>
}

// Example narrative:
/*
Timeline for: Decision: Use TypeScript for backend

2024-12-10: Pattern observed (agent:anvil)
  └─ Noticed 5 incidents caused by Python type issues

2024-12-15: Pattern reinforced (agent:anvil)
  └─ 3 more incidents in same category

2024-12-20: Proposal created (agent:anvil, confidence: 0.78)
  └─ "Consider TypeScript for new backend services"

2025-01-05: Proposal discussed (user:alice, user:bob)
  └─ Team meeting: reviewed pros/cons

2025-01-10: Proposal revised (user:alice)
  └─ Added exclusion for ML components

2025-01-15: Memory promoted (user:alice)
  └─ Ratified by platform team

2025-02-20: Memory reviewed (user:alice)
  └─ Reaffirmed after successful adoption
*/
```

### 9.4 Low-Ceremony Updates

Small amendments don't require full promotion:

```bash
# Quick updates for minor changes
anvil edda tag <memory-id> +security +critical
anvil edda scope <memory-id> --add-exclusion="test-utils/*"
anvil edda extend-review <memory-id> --days=90

# But substantial changes require review
anvil edda update <memory-id> --statement="..." # Triggers review if significant
```

### 9.5 Explicit Friction for Big Decisions

```typescript
interface FrictionMechanism {
  change_type: 'minor' | 'major' | 'critical'

  // Minor: tag, scope tweak, review extension
  minor: {
    approvals_required: 0
    confirmation_required: false
  }

  // Major: statement change, enforcement change
  major: {
    approvals_required: 1
    confirmation_required: true
    confirmation_prompt: "This will affect N enforcement hooks. Continue?"
  }

  // Critical: retirement, supersession
  critical: {
    approvals_required: 2
    confirmation_required: true
    confirmation_prompt: "This will retire an active memory. Type memory ID to confirm:"
    cooldown_period_minutes: 5  // Must wait before confirming
  }
}
```

---

## 10. Meta-Capabilities (EDDA-META-001)

### 10.1 Contradiction Detection

```typescript
interface ContradictionDetector {
  // Detection
  detectContradictions(scope?: ScopeSpecifier): ContradictionReport

  // Analysis
  analyzeContradiction(id1: MemoryId, id2: MemoryId): ContradictionAnalysis
}

interface ContradictionReport {
  contradictions: Contradiction[]
  severity_breakdown: Record<'low' | 'medium' | 'high' | 'critical', number>
}

interface Contradiction {
  memory_a: MemoryObject
  memory_b: MemoryObject

  contradiction_type: 'direct' | 'implicit' | 'conditional'
  severity: 'low' | 'medium' | 'high' | 'critical'

  explanation: string
  resolution_suggestions: ResolutionStrategy[]
}

type ResolutionStrategy =
  | { type: 'supersede', supersede_id: MemoryId, keep_id: MemoryId }
  | { type: 'scope_restriction', narrow_scope_of: MemoryId }
  | { type: 'add_condition', add_to: MemoryId, condition: string }
  | { type: 'merge', into_new: MemoryObjectInput }
```

### 10.2 Knowledge Graph

```typescript
interface KnowledgeGraph {
  nodes: KnowledgeNode[]
  edges: KnowledgeEdge[]

  // Views
  clusters: KnowledgeCluster[]
  critical_paths: CriticalPath[]
}

interface KnowledgeNode {
  id: MemoryId
  memory: MemoryObject

  // Graph metrics
  in_degree: number             // How many memories reference this
  out_degree: number            // How many this references
  centrality: number            // Importance score

  // Clustering
  cluster_id?: string
  tags: string[]
}

interface KnowledgeEdge {
  from: MemoryId
  to: MemoryId
  relationship: EdgeRelationship
  weight: number                // Strength of relationship
}

type EdgeRelationship =
  | 'supersedes'                // Evolution
  | 'depends_on'                // Dependency
  | 'contradicts'               // Conflict
  | 'supports'                  // Reinforcement
  | 'related_to'                // Weak association

interface KnowledgeCluster {
  cluster_id: string
  name: string                  // Auto-generated or manual
  memories: MemoryId[]
  centroid_tags: string[]       // Most common tags
  size: number
}

interface CriticalPath {
  path: MemoryId[]
  path_type: 'decision_chain' | 'evolution_chain' | 'dependency_chain'
  importance: number
}
```

### 10.3 Impact Analysis

```typescript
interface ImpactAnalyzer {
  // Analyze potential change
  analyzeImpact(
    memory_id: MemoryId,
    proposed_change: MemoryObjectPatch
  ): ImpactAnalysis

  // Simulate retirement
  simulateRetirement(memory_id: MemoryId): RetirementImpact
}

interface ImpactAnalysis {
  memory: MemoryObject
  proposed_change: MemoryObjectPatch

  // Affected entities
  affected_memories: MemoryId[]         // Memories that reference this
  affected_hooks: EnforcementHook[]     // Hooks that use this
  affected_agents: string[]             // Agents that cite this

  // Downstream impacts
  propagation: {
    direct_dependencies: MemoryId[]
    indirect_dependencies: MemoryId[]
    total_affected: number
  }

  // Risk assessment
  risk_level: 'low' | 'medium' | 'high' | 'critical'
  risk_factors: RiskFactor[]

  // Recommendations
  recommended_actions: string[]
}

interface RiskFactor {
  factor: string
  severity: 'low' | 'medium' | 'high' | 'critical'
  explanation: string
}
```

### 10.4 Learning Signals

Track what keeps getting proposed but rejected:

```typescript
interface LearningSignalTracker {
  // Track patterns
  trackRejectionPatterns(): RejectionPatternReport

  // Analyze proposals
  analyzeProposalQuality(): ProposalQualityReport
}

interface RejectionPatternReport {
  patterns: RejectionPattern[]

  // Insights
  most_common_rejection_reasons: Array<{ reason: RejectionCategory, count: number }>
  agents_with_high_rejection_rate: Array<{ agent_id: string, rate: number }>

  // Recommendations
  tuning_recommendations: TuningRecommendation[]
}

interface RejectionPattern {
  pattern: string               // e.g., "Low confidence proposals about X"
  occurrences: number
  example_proposal_ids: ProposalId[]

  // What's wrong
  common_issues: string[]

  // How to fix
  improvements: string[]
}

interface TuningRecommendation {
  target: 'agent' | 'ember' | 'promotion_threshold'
  recommendation: string
  expected_impact: string
  confidence: number
}

interface ProposalQualityReport {
  total_proposals: number
  approval_rate: number

  // By type
  by_type: Record<MemoryType, { submitted: number, approved: number, rate: number }>

  // Quality indicators
  average_confidence: number
  average_evidence_count: number

  // Trends
  trend_over_time: Array<{ month: string, approval_rate: number }>
}
```

### 10.5 Cultural Drift Detection

Detect when practice diverges from policy:

```typescript
interface CulturalDriftDetector {
  detectDrift(scope?: ScopeSpecifier): DriftReport
}

interface DriftReport {
  drifts: CulturalDrift[]

  // Summary
  total_drift_score: number     // 0.0 (aligned) - 1.0 (diverged)
  high_priority_drifts: CulturalDrift[]
}

interface CulturalDrift {
  memory: MemoryObject
  drift_type: DriftType
  severity: 'low' | 'medium' | 'high' | 'critical'

  // Evidence
  policy: string                // What Edda says
  practice: string              // What's actually happening
  evidence: DriftEvidence[]

  // Impact
  violation_count: number
  override_count: number
  affected_teams: string[]

  // Recommendations
  recommended_action: 'update_policy' | 'enforce_policy' | 'retire_policy' | 'add_exception'
  rationale: string
}

type DriftType =
  | 'policy_ignored'            // Memory exists but violated frequently
  | 'policy_outdated'           // Practice evolved beyond policy
  | 'policy_too_strict'         // Constant overrides
  | 'policy_forgotten'          // Memory not cited/used

interface DriftEvidence {
  evidence_type: 'violation' | 'override' | 'proposal_rejected' | 'observation'
  timestamp: Timestamp
  source_id: string
  description: string
}
```

---

## Implementation Phases

### Phase 0: Foundation (Weeks 1-2)
**Goal:** Core data models and storage

**Components:**
- MemoryObject schema implementation
- Git-backed storage layer
- Basic CRUD operations
- Schema validation

**Deliverables:**
- `packages/edda-core` with memory storage
- Unit tests for all schemas
- Git storage adapter

**Dependencies:**
- Existing `edda-stack` contracts
- Git CLI available

---

### Phase 1: Promotion Pipeline (Weeks 3-5)
**Goal:** Ember → Edda promotion workflow

**Components:**
- PromotionRequest lifecycle
- Human review interface (CLI)
- Approval/rejection workflow
- Provenance chain validation
- Type mapping (Ember → Edda)

**Deliverables:**
- `anvil edda proposals` command
- `anvil edda promote/reject` commands
- Promotion state machine
- Review notification system

**Dependencies:**
- Phase 0 complete
- Ember port implementation
- CLI framework

---

### Phase 2: Authority & Trust (Weeks 6-7)
**Goal:** Who can do what

**Components:**
- RBAC implementation
- Authority policies
- Agent trust profiles
- Audit trail
- Permission checks

**Deliverables:**
- Authority configuration system
- Audit log storage
- Permission middleware
- CLI commands for authority management

**Dependencies:**
- Phase 1 complete
- User/agent identity system

---

### Phase 3: Query & Retrieval (Weeks 8-9)
**Goal:** Finding and understanding memories

**Components:**
- Query interface implementation
- Semantic search integration
- Conflict detection
- Provenance tracing
- Temporal queries

**Deliverables:**
- `anvil edda list/search/show` commands
- Query API
- Conflict detector
- Provenance visualisation

**Dependencies:**
- Phase 0 complete
- Embedding service (optional for semantic)

---

### Phase 4: Enforcement Hooks (Weeks 10-12)
**Goal:** Edda guides and blocks

**Components:**
- Pre-execution checks
- Hook registration system
- Enforcement policies
- Override mechanism
- Contextual guidance

**Deliverables:**
- Hook framework
- Anvil integration points
- Enforcement evaluator
- Override workflow

**Dependencies:**
- Phase 0, 2, 3 complete
- Anvil gate system

---

### Phase 5: Lifecycle Management (Weeks 13-14)
**Goal:** Change and decay

**Components:**
- Deprecation workflow
- Review scheduling
- Supersession handling
- Staleness detection
- Historical queries

**Deliverables:**
- `anvil edda retire/supersede` commands
- Review scheduler
- Staleness analyser
- Forgetting engine

**Dependencies:**
- Phase 1, 3 complete

---

### Phase 6: Interop & Export (Weeks 15-16)
**Goal:** Edda as platform

**Components:**
- REST API
- Export/import
- Markdown rendering
- Tool projections (Anvil, CI, docs)

**Deliverables:**
- HTTP API server
- Export utilities
- Projection system
- API documentation

**Dependencies:**
- All core phases complete

---

### Phase 7: Meta-Capabilities (Weeks 17-19) [Optional for v1]
**Goal:** Organisational intelligence

**Components:**
- Contradiction detection
- Knowledge graph
- Impact analysis
- Learning signals
- Cultural drift detection

**Deliverables:**
- Analysis CLI commands
- Graph visualisation
- Impact simulator
- Learning dashboard

**Dependencies:**
- Full system operational

---

## Component Dependency Map

```
Phase 0 (Foundation)
    ↓
    ├─→ Phase 1 (Promotion) ─→ Phase 5 (Lifecycle)
    │       ↓                           ↓
    ├─→ Phase 2 (Authority) ────────────┤
    │       ↓                           ↓
    └─→ Phase 3 (Query) ────────────→ Phase 6 (Interop)
            ↓                           ↑
         Phase 4 (Enforcement) ─────────┘
            ↓
         Phase 7 (Meta) [Optional]
```

**Critical Path:** 0 → 1 → 2 → 4 → 6 (Minimum viable Edda)
**Full Feature:** 0 → 1 → 2 → 3 → 4 → 5 → 6 → 7

---

## Technical Stack

### Storage
- **Primary:** Git-backed YAML/JSON (versioning, auditability)
- **Index:** SQLite (fast queries, local)
- **Cache:** In-memory (LRU for hot memories)

### API
- **CLI:** Commander.js (existing Anvil pattern)
- **HTTP:** Express + OpenAPI (optional, Phase 6)
- **Events:** EventEmitter (in-process for v1)

### Search
- **Text:** SQLite FTS5 (good enough for v1)
- **Semantic:** Optional embedding service (Ollama/OpenAI)

### Testing
- **Unit:** Vitest (existing setup)
- **Integration:** Test fixtures (already built)
- **E2E:** CLI test harness

---

## Success Metrics

### Adoption Metrics
- Memories created per week
- Promotion approval rate
- Agent proposal quality (approval rate trend)
- Team engagement (memories created by team)

### Quality Metrics
- Contradiction detection rate
- Staleness score distribution
- Review completion rate
- Override frequency

### Impact Metrics
- Violations prevented (enforcement hooks)
- Incidents avoided (warnings surfaced)
- Onboarding time reduction (knowledge accessible)
- Decision consistency (similar scenarios → similar decisions)

---

## Open Questions for APS Planning

1. **Storage:** Git-backed sufficient or need distributed storage?
2. **Semantic Search:** Mandatory for v1 or optional?
3. **Multi-tenancy:** Single org vs multi-org support?
4. **Real-time:** EventBus in-process or need message queue?
5. **UI:** CLI-only for v1 or web dashboard needed?
6. **Embedding Model:** Self-hosted (Ollama) or API (OpenAI)?
7. **Authority:** Integrate with existing auth (GitHub, LDAP) or standalone?
8. **Compliance:** GDPR/audit requirements for enterprise?

---

## Next Steps

1. **Review this architecture** with stakeholders
2. **Prioritise phases** based on business needs
3. **Resolve open questions** before detailed APS
4. **Create detailed APS documents** for approved phases
5. **Set up development environment** and CI/CD
6. **Begin Phase 0 implementation**

---

**Document Status:** Draft for review
**Next Review:** After stakeholder feedback
**Owner:** Architecture team
