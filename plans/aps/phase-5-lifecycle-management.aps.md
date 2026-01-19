# Phase 5: Lifecycle Management - APS Document

**Phase:** 5 of 7
**Duration:** 2 weeks (10 working days)
**Dependencies:** Phase 0 (Foundation), Phase 3 (Query & Retrieval)
**Status:** Not Started
**Owner:** TBD

---

## Phase Overview

### Purpose
Implement lifecycle management capabilities that keep Edda's knowledge fresh and relevant through deprecation, supersession, staleness detection, and aggressive forgetting strategies.

### Scope
This phase delivers the temporal dimension of Edda, ensuring that memories reflect current institutional knowledge while preserving historical context through proper supersession chains.

### Success Criteria
- ✅ Memory deprecation workflow operational
- ✅ Supersession chains tracked and queryable
- ✅ Staleness detection identifies outdated memories
- ✅ Aggressive forgetting removes obsolete knowledge
- ✅ Time-to-live (TTL) policies enforced
- ✅ CLI commands for lifecycle management
- ✅ 100% test coverage on lifecycle logic

---

## Epic Breakdown

### Epic 1: Memory Status Transitions
**Duration:** 2 days
**Priority:** P0 (Blocking)

#### Epic 1.1: Status State Machine
**Estimate:** 3 hours
**Owner:** TBD

**Description:**
Define and implement the memory status state machine with valid transitions.

**Acceptance Criteria:**
- 3 statuses: active, deprecated, superseded
- Valid transitions defined
- State machine validates transitions
- Transition reasons required
- Audit trail logs status changes

**Implementation:**

```typescript
// packages/edda-core/src/lifecycle/status-machine.ts

export enum MemoryStatus {
  ACTIVE = 'active',
  DEPRECATED = 'deprecated',
  SUPERSEDED = 'superseded',
}

export interface StatusTransition {
  from: MemoryStatus
  to: MemoryStatus
  reason: string
  transitioned_by: string
  transitioned_at: string
  superseded_by?: MemoryId  // If transitioning to superseded
}

export class MemoryStatusMachine {
  /**
   * Valid state transitions
   */
  private static readonly VALID_TRANSITIONS: Record<MemoryStatus, MemoryStatus[]> = {
    [MemoryStatus.ACTIVE]: [MemoryStatus.DEPRECATED, MemoryStatus.SUPERSEDED],
    [MemoryStatus.DEPRECATED]: [MemoryStatus.ACTIVE, MemoryStatus.SUPERSEDED],
    [MemoryStatus.SUPERSEDED]: [], // Terminal state (cannot transition out)
  }

  /**
   * Check if transition is valid
   */
  static canTransition(from: MemoryStatus, to: MemoryStatus): boolean {
    return this.VALID_TRANSITIONS[from].includes(to)
  }

  /**
   * Validate transition
   * @throws InvalidTransitionError if invalid
   */
  static validateTransition(
    from: MemoryStatus,
    to: MemoryStatus,
    superseded_by?: MemoryId,
  ): void {
    if (!this.canTransition(from, to)) {
      throw new InvalidTransitionError(
        `Cannot transition from ${from} to ${to}. Valid transitions: ${this.VALID_TRANSITIONS[from].join(', ')}`
      )
    }

    if (to === MemoryStatus.SUPERSEDED && !superseded_by) {
      throw new InvalidTransitionError('superseded_by required when transitioning to superseded')
    }
  }

  /**
   * Create transition record
   */
  static createTransition(
    from: MemoryStatus,
    to: MemoryStatus,
    principal: Principal,
    reason: string,
    superseded_by?: MemoryId,
  ): StatusTransition {
    this.validateTransition(from, to, superseded_by)

    return {
      from,
      to,
      reason,
      transitioned_by: principal.identifier,
      transitioned_at: new Date().toISOString(),
      superseded_by,
    }
  }
}

export class InvalidTransitionError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'InvalidTransitionError'
  }
}
```

**File Structure:**
```
packages/edda-core/src/lifecycle/
├── status-machine.ts
└── __tests__/
    └── status-machine.test.ts
```

**Tests:**
- Valid transitions allowed
- Invalid transitions rejected
- Superseded requires superseded_by
- Terminal state (superseded) cannot transition out

---

#### Epic 1.2: Lifecycle Service
**Estimate:** 5 hours
**Owner:** TBD

**Description:**
Implement lifecycle service for managing memory status transitions.

**Acceptance Criteria:**
- ILifecycleService interface implemented
- deprecateMemory() marks memory as deprecated
- supersed eMemory() creates supersession chain
- reactivateMemory() returns deprecated memory to active
- Updates memory storage and index
- Logs all transitions to audit trail

**Implementation:**

```typescript
// packages/edda-core/src/lifecycle/lifecycle-service.ts

export interface ILifecycleService {
  /**
   * Deprecate a memory (soft delete)
   */
  deprecateMemory(
    memoryId: MemoryId,
    principal: Principal,
    reason: string,
  ): Promise<MemoryObject>

  /**
   * Supersede a memory with a new one
   */
  supersedeMemory(
    oldMemoryId: MemoryId,
    newMemory: CreateMemoryData,
    principal: Principal,
  ): Promise<{ old: MemoryObject; new: MemoryObject }>

  /**
   * Reactivate a deprecated memory
   */
  reactivateMemory(
    memoryId: MemoryId,
    principal: Principal,
    reason: string,
  ): Promise<MemoryObject>

  /**
   * Get status history for a memory
   */
  getStatusHistory(memoryId: MemoryId): Promise<StatusTransition[]>

  /**
   * Find memories superseded by a given memory
   */
  findSupersededBy(memoryId: MemoryId): Promise<MemoryObject[]>
}

export class LifecycleService implements ILifecycleService {
  constructor(
    private memoryManager: IMemoryManager,
    private storage: IMemoryStorage,
    private authz: IAuthorizationService,
    private audit: IAuditTrailService,
  ) {}

  async deprecateMemory(
    memoryId: MemoryId,
    principal: Principal,
    reason: string,
  ): Promise<MemoryObject> {
    // Authorize
    await this.authz.authorizeMemoryOperation(principal, 'update')

    // Get memory
    const memory = await this.storage.fetch(memoryId)
    if (!memory) {
      throw new MemoryNotFoundError(memoryId)
    }

    // Validate transition
    const transition = MemoryStatusMachine.createTransition(
      memory.status,
      MemoryStatus.DEPRECATED,
      principal,
      reason,
    )

    // Update memory
    const updated: MemoryObject = {
      ...memory,
      status: MemoryStatus.DEPRECATED,
      lifecycle: {
        ...memory.lifecycle,
        deprecated_at: transition.transitioned_at,
        deprecation_reason: reason,
        status_history: [
          ...(memory.lifecycle.status_history || []),
          transition,
        ],
      },
      authority: {
        ...memory.authority,
        updated_at: new Date().toISOString(),
        modification_history: [
          ...memory.authority.modification_history,
          {
            modified_by: principal.identifier,
            modified_at: new Date().toISOString(),
            operation: 'update',
            reason: `Deprecated: ${reason}`,
          },
        ],
      },
    }

    // Store
    await this.storage.store(updated)

    // Audit
    await this.audit.log({
      actor: principal.identifier,
      action: AuditAction.MEMORY_DEPRECATE,
      resource_type: 'memory',
      resource_id: memoryId,
      outcome: 'success',
      metadata: { reason },
    })

    return updated
  }

  async supersedeMemory(
    oldMemoryId: MemoryId,
    newMemory: CreateMemoryData,
    principal: Principal,
  ): Promise<{ old: MemoryObject; new: MemoryObject }> {
    // Authorize
    await this.authz.authorizeMemoryOperation(principal, 'create')
    await this.authz.authorizeMemoryOperation(principal, 'update')

    // Get old memory
    const oldMem = await this.storage.fetch(oldMemoryId)
    if (!oldMem) {
      throw new MemoryNotFoundError(oldMemoryId)
    }

    // Create new memory with supersedes relationship
    const newMem = await this.memoryManager.create(principal, {
      ...newMemory,
      lifecycle: {
        ...(newMemory.lifecycle || {}),
        supersedes: [oldMemoryId],
      },
      relations: {
        ...(newMemory.relations || {}),
        related_to: [
          ...(newMemory.relations?.related_to || []),
          oldMemoryId,
        ],
      },
    })

    // Validate transition
    const transition = MemoryStatusMachine.createTransition(
      oldMem.status,
      MemoryStatus.SUPERSEDED,
      principal,
      `Superseded by ${newMem.id}`,
      newMem.id,
    )

    // Update old memory
    const updated: MemoryObject = {
      ...oldMem,
      status: MemoryStatus.SUPERSEDED,
      lifecycle: {
        ...oldMem.lifecycle,
        superseded_at: transition.transitioned_at,
        superseded_by: newMem.id,
        status_history: [
          ...(oldMem.lifecycle.status_history || []),
          transition,
        ],
      },
      authority: {
        ...oldMem.authority,
        updated_at: new Date().toISOString(),
      },
    }

    // Store old memory update
    await this.storage.store(updated)

    // Audit
    await this.audit.log({
      actor: principal.identifier,
      action: AuditAction.MEMORY_SUPERSEDE,
      resource_type: 'memory',
      resource_id: oldMemoryId,
      outcome: 'success',
      metadata: {
        superseded_by: newMem.id,
      },
    })

    return { old: updated, new: newMem }
  }

  async reactivateMemory(
    memoryId: MemoryId,
    principal: Principal,
    reason: string,
  ): Promise<MemoryObject> {
    // Authorize
    await this.authz.authorizeMemoryOperation(principal, 'update')

    // Get memory
    const memory = await this.storage.fetch(memoryId)
    if (!memory) {
      throw new MemoryNotFoundError(memoryId)
    }

    // Validate transition
    const transition = MemoryStatusMachine.createTransition(
      memory.status,
      MemoryStatus.ACTIVE,
      principal,
      reason,
    )

    // Update memory
    const updated: MemoryObject = {
      ...memory,
      status: MemoryStatus.ACTIVE,
      lifecycle: {
        ...memory.lifecycle,
        deprecated_at: undefined,
        deprecation_reason: undefined,
        status_history: [
          ...(memory.lifecycle.status_history || []),
          transition,
        ],
      },
      authority: {
        ...memory.authority,
        updated_at: new Date().toISOString(),
      },
    }

    // Store
    await this.storage.store(updated)

    // Audit
    await this.audit.log({
      actor: principal.identifier,
      action: AuditAction.MEMORY_REACTIVATE,
      resource_type: 'memory',
      resource_id: memoryId,
      outcome: 'success',
      metadata: { reason },
    })

    return updated
  }

  async getStatusHistory(memoryId: MemoryId): Promise<StatusTransition[]> {
    const memory = await this.storage.fetch(memoryId)
    if (!memory) {
      throw new MemoryNotFoundError(memoryId)
    }

    return memory.lifecycle.status_history || []
  }

  async findSupersededBy(memoryId: MemoryId): Promise<MemoryObject[]> {
    // Query for memories superseded by this one
    const result = await this.query.query({
      filters: {
        status: ['superseded'],
      },
    })

    return result.memories.filter(m =>
      m.lifecycle.superseded_by === memoryId
    )
  }
}
```

**Tests:**
- Deprecate active memory
- Cannot deprecate superseded memory
- Supersede creates new memory with proper relationships
- Old memory marked as superseded
- Reactivate deprecated memory
- Cannot reactivate superseded memory
- Status history tracked
- All transitions audited

---

### Epic 2: Staleness Detection
**Duration:** 2 days
**Priority:** P1 (Important)

#### Epic 2.1: Staleness Analyzer
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Implement staleness detection based on age, usage, and validation frequency.

**Acceptance Criteria:**
- StalenessCriteria configurable per memory type
- Analyzes: last_validated, age, usage_count, references
- Returns staleness_score (0.0 - 1.0)
- Identifies memories needing review

**Implementation:**

```typescript
// packages/edda-core/src/lifecycle/staleness-analyzer.ts

export interface StalenessCriteria {
  max_age_days?: number         // Days since created_at
  max_unvalidated_days?: number // Days since last_validated
  min_usage_count?: number      // Minimum expected usage
  require_references?: boolean  // Must be referenced by other memories
}

export interface StalenessResult {
  memory_id: MemoryId
  staleness_score: number       // 0.0 (fresh) to 1.0 (stale)
  reasons: string[]
  recommendations: string[]
  needs_review: boolean
}

export interface IStalenessAnalyzer {
  /**
   * Analyze staleness of a single memory
   */
  analyzeMemory(memory: MemoryObject): Promise<StalenessResult>

  /**
   * Find stale memories matching criteria
   */
  findStaleMemories(
    criteria: StalenessCriteria,
    threshold?: number,
  ): Promise<StalenessResult[]>

  /**
   * Get default criteria for memory type
   */
  getDefaultCriteria(type: MemoryType): StalenessCriteria
}

export class StalenessAnalyzer implements IStalenessAnalyzer {
  constructor(
    private storage: IMemoryStorage,
    private queryService: IQueryService,
  ) {}

  async analyzeMemory(memory: MemoryObject): Promise<StalenessResult> {
    const criteria = this.getDefaultCriteria(memory.type)
    const reasons: string[] = []
    let score = 0.0

    // Factor 1: Age since creation
    if (criteria.max_age_days) {
      const ageScore = this.calculateAgeScore(
        memory.authority.created_at,
        criteria.max_age_days,
      )
      score += ageScore * 0.3
      if (ageScore > 0.7) {
        reasons.push(`Created ${this.getDaysAgo(memory.authority.created_at)} days ago`)
      }
    }

    // Factor 2: Validation recency
    if (criteria.max_unvalidated_days) {
      const validationScore = this.calculateValidationScore(
        memory.lifecycle.last_validated || memory.authority.created_at,
        criteria.max_unvalidated_days,
      )
      score += validationScore * 0.4
      if (validationScore > 0.7) {
        reasons.push(`Not validated in ${this.getDaysAgo(memory.lifecycle.last_validated!)} days`)
      }
    }

    // Factor 3: Usage frequency
    if (criteria.min_usage_count) {
      const usageScore = this.calculateUsageScore(
        memory.lifecycle.usage_count || 0,
        criteria.min_usage_count,
      )
      score += usageScore * 0.2
      if (usageScore > 0.7) {
        reasons.push(`Low usage count: ${memory.lifecycle.usage_count || 0}`)
      }
    }

    // Factor 4: References
    if (criteria.require_references) {
      const hasReferences = await this.hasReferences(memory.id)
      if (!hasReferences) {
        score += 0.1
        reasons.push('Not referenced by other memories')
      }
    }

    // Generate recommendations
    const recommendations = this.generateRecommendations(score, reasons, memory)

    return {
      memory_id: memory.id,
      staleness_score: Math.min(score, 1.0),
      reasons,
      recommendations,
      needs_review: score > 0.7,
    }
  }

  async findStaleMemories(
    criteria: StalenessCriteria,
    threshold: number = 0.7,
  ): Promise<StalenessResult[]> {
    // Query all active memories
    const result = await this.queryService.query({
      filters: {
        status: ['active'],
      },
      pagination: {
        limit: 1000,
        offset: 0,
      },
    })

    // Analyze each
    const analyses = await Promise.all(
      result.memories.map(m => this.analyzeMemory(m))
    )

    // Filter by threshold
    return analyses.filter(a => a.staleness_score >= threshold)
  }

  getDefaultCriteria(type: MemoryType): StalenessCriteria {
    // Type-specific criteria
    const criteriaMap: Record<MemoryType, StalenessCriteria> = {
      decision: {
        max_age_days: 365,          // Decisions valid for 1 year
        max_unvalidated_days: 180,  // Review every 6 months
        min_usage_count: 5,
        require_references: false,
      },
      pattern: {
        max_age_days: 180,          // Patterns fresher (6 months)
        max_unvalidated_days: 90,   // Review quarterly
        min_usage_count: 10,
        require_references: true,   // Should be referenced
      },
      constraint: {
        max_age_days: 730,          // Constraints long-lived (2 years)
        max_unvalidated_days: 90,   // But review quarterly
        min_usage_count: 20,        // Should be enforced frequently
        require_references: false,
      },
      warning: {
        max_age_days: 90,           // Warnings short-lived (3 months)
        max_unvalidated_days: 30,   // Review monthly
        min_usage_count: 5,
        require_references: false,
      },
      doctrine: {
        max_age_days: 1095,         // Doctrines long-lived (3 years)
        max_unvalidated_days: 180,  // Review semi-annually
        min_usage_count: 50,        // Foundational knowledge
        require_references: true,
      },
      lesson: {
        max_age_days: 180,          // Lessons medium-term (6 months)
        max_unvalidated_days: 90,   // Review quarterly
        min_usage_count: 3,
        require_references: false,
      },
    }

    return criteriaMap[type]
  }

  private calculateAgeScore(created_at: string, max_days: number): number {
    const ageInDays = this.getDaysAgo(created_at)
    return Math.min(ageInDays / max_days, 1.0)
  }

  private calculateValidationScore(last_validated: string, max_days: number): number {
    const daysSinceValidation = this.getDaysAgo(last_validated)
    return Math.min(daysSinceValidation / max_days, 1.0)
  }

  private calculateUsageScore(usage_count: number, min_count: number): number {
    if (usage_count >= min_count) return 0.0
    return 1.0 - (usage_count / min_count)
  }

  private getDaysAgo(date: string): number {
    const now = Date.now()
    const then = new Date(date).getTime()
    return Math.floor((now - then) / (24 * 60 * 60 * 1000))
  }

  private async hasReferences(memoryId: MemoryId): Promise<boolean> {
    // Check if other memories reference this one
    const result = await this.queryService.query({
      filters: {
        status: ['active'],
      },
      pagination: {
        limit: 1,
        offset: 0,
      },
    })

    return result.memories.some(m =>
      m.relations.related_to?.includes(memoryId) ||
      m.lifecycle.supersedes?.includes(memoryId)
    )
  }

  private generateRecommendations(
    score: number,
    reasons: string[],
    memory: MemoryObject,
  ): string[] {
    const recommendations: string[] = []

    if (score > 0.9) {
      recommendations.push('Consider deprecating or superseding')
    } else if (score > 0.7) {
      recommendations.push('Schedule review with domain experts')
    }

    if (reasons.some(r => r.includes('Not validated'))) {
      recommendations.push('Mark as validated after review')
    }

    if (reasons.some(r => r.includes('Low usage'))) {
      recommendations.push('Verify relevance and applicability')
    }

    return recommendations
  }
}
```

**File Structure:**
```
packages/edda-core/src/lifecycle/
├── staleness-analyzer.ts
└── __tests__/
    └── staleness-analyzer.test.ts
```

**Tests:**
- Analyze fresh memory (low score)
- Analyze old unvalidated memory (high score)
- Analyze low-usage memory (medium score)
- Type-specific criteria applied
- Find stale memories above threshold
- Recommendations generated

---

#### Epic 2.2: Validation Workflow
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Implement validation workflow to mark memories as reviewed.

**Acceptance Criteria:**
- validateMemory() updates last_validated timestamp
- Validation notes optional
- Resets staleness score
- CLI command for validation

**Implementation:**

```typescript
// packages/edda-core/src/lifecycle/validation-service.ts

export interface IValidationService {
  /**
   * Mark memory as validated
   */
  validateMemory(
    memoryId: MemoryId,
    principal: Principal,
    notes?: string,
  ): Promise<MemoryObject>

  /**
   * Get memories needing validation
   */
  getMemoriesNeedingValidation(
    days?: number,
  ): Promise<MemoryObject[]>
}

export class ValidationService implements IValidationService {
  constructor(
    private storage: IMemoryStorage,
    private stalenessAnalyzer: IStalenessAnalyzer,
    private authz: IAuthorizationService,
    private audit: IAuditTrailService,
  ) {}

  async validateMemory(
    memoryId: MemoryId,
    principal: Principal,
    notes?: string,
  ): Promise<MemoryObject> {
    // Authorize
    await this.authz.authorizeMemoryOperation(principal, 'update')

    // Get memory
    const memory = await this.storage.fetch(memoryId)
    if (!memory) {
      throw new MemoryNotFoundError(memoryId)
    }

    // Update memory
    const updated: MemoryObject = {
      ...memory,
      lifecycle: {
        ...memory.lifecycle,
        last_validated: new Date().toISOString(),
        validation_notes: notes,
      },
      authority: {
        ...memory.authority,
        updated_at: new Date().toISOString(),
      },
    }

    // Store
    await this.storage.store(updated)

    // Audit
    await this.audit.log({
      actor: principal.identifier,
      action: AuditAction.MEMORY_VALIDATE,
      resource_type: 'memory',
      resource_id: memoryId,
      outcome: 'success',
      metadata: { notes },
    })

    return updated
  }

  async getMemoriesNeedingValidation(days: number = 90): Promise<MemoryObject[]> {
    const cutoff = new Date()
    cutoff.setDate(cutoff.getDate() - days)
    const cutoffISO = cutoff.toISOString()

    // Query memories not validated recently
    const result = await this.queryService.query({
      filters: {
        status: ['active'],
        // last_validated < cutoff OR null
      },
      pagination: {
        limit: 100,
        offset: 0,
      },
    })

    return result.memories.filter(m =>
      !m.lifecycle.last_validated ||
      m.lifecycle.last_validated < cutoffISO
    )
  }
}
```

**Tests:**
- Validate memory (updates timestamp)
- Validation notes saved
- Get memories needing validation (90 days)
- Authorization required

---

### Epic 3: Aggressive Forgetting
**Duration:** 2 days
**Priority:** P1 (Important)

#### Epic 3.1: TTL Policies
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Implement time-to-live policies for automatic deprecation.

**Acceptance Criteria:**
- TTL policy schema defined
- TTL policies stored in `.edda/policies/ttl/`
- Per-type, per-scope, per-tag TTL support
- Policies evaluated on schedule

**Implementation:**

```typescript
// packages/edda-core/src/lifecycle/ttl-policy.ts

export interface TTLPolicy {
  policy_id: string
  name: string
  description: string
  enabled: boolean

  // Matchers (AND logic)
  memory_type?: MemoryType
  scope_pattern?: string
  tags?: string[]

  // TTL duration
  ttl_days: number

  // Action on expiry
  expiry_action: 'deprecate' | 'delete' | 'notify'

  // Grace period before action
  grace_period_days?: number

  created_at: string
  updated_at: string
}

export interface ITTLPolicyService {
  /**
   * Create or update TTL policy
   */
  upsertPolicy(policy: TTLPolicy): Promise<void>

  /**
   * Get all policies
   */
  listPolicies(): Promise<TTLPolicy[]>

  /**
   * Find applicable policies for a memory
   */
  findApplicablePolicies(memory: MemoryObject): Promise<TTLPolicy[]>

  /**
   * Evaluate TTL policies (run on schedule)
   */
  evaluatePolicies(): Promise<TTLEvaluationResult>
}

export interface TTLEvaluationResult {
  evaluated_count: number
  expired_count: number
  actions_taken: Array<{
    memory_id: MemoryId
    policy_id: string
    action: 'deprecate' | 'delete' | 'notify'
  }>
}

export class TTLPolicyService implements ITTLPolicyService {
  constructor(
    private storage: IMemoryStorage,
    private lifecycleService: ILifecycleService,
    private gitStorage: IGitStorage,
  ) {}

  async upsertPolicy(policy: TTLPolicy): Promise<void> {
    const filePath = `.edda/policies/ttl/${policy.policy_id}.yaml`
    const content = yaml.stringify(policy)
    await this.gitStorage.write(filePath, content, `Update TTL policy ${policy.policy_id}`)
  }

  async listPolicies(): Promise<TTLPolicy[]> {
    const files = await this.gitStorage.list('.edda/policies/ttl/')
    const policies = await Promise.all(
      files.map(async file => {
        const content = await this.gitStorage.read(file)
        return yaml.parse(content) as TTLPolicy
      })
    )

    return policies.filter(p => p.enabled)
  }

  async findApplicablePolicies(memory: MemoryObject): Promise<TTLPolicy[]> {
    const allPolicies = await this.listPolicies()

    return allPolicies.filter(policy => {
      // Match type
      if (policy.memory_type && policy.memory_type !== memory.type) {
        return false
      }

      // Match scope
      if (policy.scope_pattern && !memory.scope.startsWith(policy.scope_pattern)) {
        return false
      }

      // Match tags
      if (policy.tags && !policy.tags.some(tag => memory.tags.includes(tag))) {
        return false
      }

      return true
    })
  }

  async evaluatePolicies(): Promise<TTLEvaluationResult> {
    const policies = await this.listPolicies()
    const actions: TTLEvaluationResult['actions_taken'] = []

    // Get all active memories
    const result = await this.queryService.query({
      filters: {
        status: ['active'],
      },
      pagination: {
        limit: 10000,
        offset: 0,
      },
    })

    for (const memory of result.memories) {
      const applicablePolicies = await this.findApplicablePolicies(memory)

      for (const policy of applicablePolicies) {
        if (this.isExpired(memory, policy)) {
          await this.executePolicy(memory, policy)
          actions.push({
            memory_id: memory.id,
            policy_id: policy.policy_id,
            action: policy.expiry_action,
          })
        }
      }
    }

    return {
      evaluated_count: result.memories.length,
      expired_count: actions.length,
      actions_taken: actions,
    }
  }

  private isExpired(memory: MemoryObject, policy: TTLPolicy): boolean {
    const ageInDays = this.getDaysAgo(memory.authority.created_at)
    const ttlWithGrace = policy.ttl_days + (policy.grace_period_days || 0)

    return ageInDays > ttlWithGrace
  }

  private async executePolicy(memory: MemoryObject, policy: TTLPolicy): Promise<void> {
    const systemPrincipal: Principal = {
      identifier: 'system',
      principal_type: 'system',
      roles: [AuthorityLevel.SYSTEM],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    }

    switch (policy.expiry_action) {
      case 'deprecate':
        await this.lifecycleService.deprecateMemory(
          memory.id,
          systemPrincipal,
          `Automatically deprecated by TTL policy: ${policy.name}`,
        )
        break

      case 'delete':
        await this.memoryManager.delete(systemPrincipal, memory.id)
        break

      case 'notify':
        // Send notification (implementation depends on notification system)
        console.log(`TTL expiry notification for memory ${memory.id}`)
        break
    }
  }

  private getDaysAgo(date: string): number {
    const now = Date.now()
    const then = new Date(date).getTime()
    return Math.floor((now - then) / (24 * 60 * 60 * 1000))
  }
}
```

**Example TTL Policies:**

```yaml
# .edda/policies/ttl/warnings-90d.yaml
policy_id: ttl-warnings-90d
name: Deprecate old warnings
description: Warnings older than 90 days are likely outdated
enabled: true
memory_type: warning
ttl_days: 90
grace_period_days: 7
expiry_action: deprecate
created_at: 2026-01-19T10:00:00Z
updated_at: 2026-01-19T10:00:00Z
```

```yaml
# .edda/policies/ttl/temp-scope-30d.yaml
policy_id: ttl-temp-30d
name: Delete temporary memories
description: Memories in temp/ scope deleted after 30 days
enabled: true
scope_pattern: temp/
ttl_days: 30
grace_period_days: 0
expiry_action: delete
created_at: 2026-01-19T10:00:00Z
updated_at: 2026-01-19T10:00:00Z
```

**File Structure:**
```
packages/edda-core/src/lifecycle/
├── ttl-policy.ts
└── __tests__/
    └── ttl-policy.test.ts

.edda/policies/ttl/
├── warnings-90d.yaml
└── temp-scope-30d.yaml
```

**Tests:**
- Create TTL policy
- Find applicable policies for memory
- Evaluate policies (deprecated expired memories)
- Grace period honored
- Different actions (deprecate, delete, notify)

---

#### Epic 3.2: Scheduled Lifecycle Jobs
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Implement scheduled jobs for staleness detection and TTL evaluation.

**Acceptance Criteria:**
- Cron-style scheduler for lifecycle jobs
- Daily staleness scan
- Weekly TTL policy evaluation
- Job results logged

**Implementation:**

```typescript
// packages/edda-core/src/lifecycle/lifecycle-scheduler.ts

export interface LifecycleJob {
  name: string
  schedule: string  // Cron expression
  handler: () => Promise<void>
}

export interface ILifecycleScheduler {
  /**
   * Start lifecycle jobs
   */
  start(): Promise<void>

  /**
   * Stop lifecycle jobs
   */
  stop(): Promise<void>

  /**
   * Run job immediately (for testing)
   */
  runJobNow(jobName: string): Promise<void>
}

export class LifecycleScheduler implements ILifecycleScheduler {
  private jobs: LifecycleJob[]
  private intervals: Map<string, NodeJS.Timeout> = new Map()

  constructor(
    private stalenessAnalyzer: IStalenessAnalyzer,
    private ttlPolicyService: ITTLPolicyService,
    private audit: IAuditTrailService,
  ) {
    this.jobs = [
      {
        name: 'staleness-scan',
        schedule: '0 2 * * *',  // Daily at 2am
        handler: () => this.runStalenessScan(),
      },
      {
        name: 'ttl-evaluation',
        schedule: '0 3 * * 0',  // Weekly on Sunday at 3am
        handler: () => this.runTTLEvaluation(),
      },
    ]
  }

  async start(): Promise<void> {
    for (const job of this.jobs) {
      // Parse cron and set interval (simplified)
      const interval = this.cronToInterval(job.schedule)
      const timer = setInterval(() => {
        job.handler().catch(err => {
          console.error(`Lifecycle job ${job.name} failed:`, err)
        })
      }, interval)

      this.intervals.set(job.name, timer)
    }

    console.log('Lifecycle scheduler started')
  }

  async stop(): Promise<void> {
    for (const [name, timer] of this.intervals) {
      clearInterval(timer)
    }
    this.intervals.clear()

    console.log('Lifecycle scheduler stopped')
  }

  async runJobNow(jobName: string): Promise<void> {
    const job = this.jobs.find(j => j.name === jobName)
    if (!job) {
      throw new Error(`Job ${jobName} not found`)
    }

    await job.handler()
  }

  private async runStalenessScan(): Promise<void> {
    console.log('Running staleness scan...')

    const staleMemories = await this.stalenessAnalyzer.findStaleMemories(
      {}, // Use default criteria
      0.7, // Threshold
    )

    await this.audit.log({
      actor: 'system',
      action: AuditAction.LIFECYCLE_STALENESS_SCAN,
      resource_type: 'lifecycle',
      resource_id: 'staleness-scan',
      outcome: 'success',
      metadata: {
        stale_count: staleMemories.length,
        memory_ids: staleMemories.map(m => m.memory_id),
      },
    })

    console.log(`Staleness scan complete: ${staleMemories.length} stale memories found`)
  }

  private async runTTLEvaluation(): Promise<void> {
    console.log('Running TTL policy evaluation...')

    const result = await this.ttlPolicyService.evaluatePolicies()

    await this.audit.log({
      actor: 'system',
      action: AuditAction.LIFECYCLE_TTL_EVALUATION,
      resource_type: 'lifecycle',
      resource_id: 'ttl-evaluation',
      outcome: 'success',
      metadata: {
        evaluated: result.evaluated_count,
        expired: result.expired_count,
        actions: result.actions_taken,
      },
    })

    console.log(`TTL evaluation complete: ${result.expired_count} memories processed`)
  }

  private cronToInterval(cron: string): number {
    // Simplified: map common cron expressions to intervals
    // In production, use a proper cron parser
    if (cron === '0 2 * * *') return 24 * 60 * 60 * 1000 // Daily
    if (cron === '0 3 * * 0') return 7 * 24 * 60 * 60 * 1000 // Weekly

    return 24 * 60 * 60 * 1000 // Default daily
  }
}
```

**Tests:**
- Start scheduler
- Run job immediately
- Staleness scan logs results
- TTL evaluation logs results
- Stop scheduler

---

### Epic 4: CLI Commands
**Duration:** 2 days
**Priority:** P1 (Important)

#### Epic 4.1: Lifecycle Management CLI
**Estimate:** 6 hours
**Owner:** TBD

**Description:**
Implement CLI commands for lifecycle management.

**Acceptance Criteria:**
- `anvil edda deprecate <memory-id> --reason "..."` - Deprecate memory
- `anvil edda supersede <old-id> --with <new-file>` - Supersede memory
- `anvil edda reactivate <memory-id>` - Reactivate deprecated memory
- `anvil edda validate <memory-id>` - Mark as validated
- `anvil edda staleness` - Show stale memories
- `anvil edda ttl list/create/evaluate` - Manage TTL policies

**Implementation:**

```typescript
// packages/anvil/src/commands/edda/lifecycle.ts

export const deprecateCommand: Command = {
  name: 'deprecate',
  description: 'Deprecate a memory',
  args: [
    { name: 'memory-id', required: true },
  ],
  options: [
    { name: 'reason', required: true },
  ],
  async execute(context, args, options) {
    const memoryId = args['memory-id']
    const reason = options.reason

    const updated = await context.edda.lifecycle.deprecateMemory(
      memoryId,
      context.currentPrincipal,
      reason,
    )

    console.log(`✅ Deprecated memory: ${memoryId}`)
    console.log(`   Reason: ${reason}`)
    console.log(`   Status: ${updated.status}`)
  },
}

export const supersedeCommand: Command = {
  name: 'supersede',
  description: 'Supersede a memory with a new one',
  args: [
    { name: 'old-id', required: true },
  ],
  options: [
    { name: 'with', required: true },  // Path to new memory YAML
  ],
  async execute(context, args, options) {
    const oldId = args['old-id']
    const newMemoryPath = options.with

    // Load new memory from file
    const newMemoryData = await loadMemoryFromFile(newMemoryPath)

    const result = await context.edda.lifecycle.supersedeMemory(
      oldId,
      newMemoryData,
      context.currentPrincipal,
    )

    console.log(`✅ Superseded memory:`)
    console.log(`   Old: ${result.old.id} (status: ${result.old.status})`)
    console.log(`   New: ${result.new.id} (status: ${result.new.status})`)
  },
}

export const reactivateCommand: Command = {
  name: 'reactivate',
  description: 'Reactivate a deprecated memory',
  args: [
    { name: 'memory-id', required: true },
  ],
  options: [
    { name: 'reason', required: true },
  ],
  async execute(context, args, options) {
    const memoryId = args['memory-id']
    const reason = options.reason

    const updated = await context.edda.lifecycle.reactivateMemory(
      memoryId,
      context.currentPrincipal,
      reason,
    )

    console.log(`✅ Reactivated memory: ${memoryId}`)
    console.log(`   Status: ${updated.status}`)
  },
}

export const validateCommand: Command = {
  name: 'validate',
  description: 'Mark memory as validated',
  args: [
    { name: 'memory-id', required: true },
  ],
  options: [
    { name: 'notes' },
  ],
  async execute(context, args, options) {
    const memoryId = args['memory-id']
    const notes = options.notes

    const updated = await context.edda.validation.validateMemory(
      memoryId,
      context.currentPrincipal,
      notes,
    )

    console.log(`✅ Validated memory: ${memoryId}`)
    console.log(`   Last validated: ${updated.lifecycle.last_validated}`)
  },
}

export const stalenessCommand: Command = {
  name: 'staleness',
  description: 'Show stale memories',
  options: [
    { name: 'threshold', default: 0.7 },
  ],
  async execute(context, args, options) {
    const staleMemories = await context.edda.staleness.findStaleMemories(
      {},
      options.threshold,
    )

    console.log(`Stale Memories (threshold: ${options.threshold}):\n`)
    console.log('MEMORY ID       | SCORE | REASONS')
    console.log('─'.repeat(80))

    for (const result of staleMemories) {
      console.log(
        `${result.memory_id.padEnd(15)} | ` +
        `${result.staleness_score.toFixed(2).padStart(5)} | ` +
        `${result.reasons.join('; ')}`
      )

      if (result.recommendations.length > 0) {
        console.log(`   Recommendations: ${result.recommendations.join(', ')}`)
      }
      console.log()
    }

    console.log(`\nTotal: ${staleMemories.length} memories need review`)
  },
}

export const ttlCommand: Command = {
  name: 'ttl',
  description: 'Manage TTL policies',
  subcommands: [
    {
      name: 'list',
      description: 'List all TTL policies',
      async execute(context) {
        const policies = await context.edda.ttl.listPolicies()

        console.log(`TTL Policies (${policies.length}):\n`)

        for (const policy of policies) {
          console.log(`${policy.name} (${policy.policy_id})`)
          console.log(`  Type: ${policy.memory_type || 'all'}`)
          console.log(`  TTL: ${policy.ttl_days} days`)
          console.log(`  Action: ${policy.expiry_action}`)
          console.log(`  Enabled: ${policy.enabled}`)
          console.log()
        }
      },
    },

    {
      name: 'evaluate',
      description: 'Evaluate TTL policies now',
      async execute(context) {
        console.log('Evaluating TTL policies...')

        const result = await context.edda.ttl.evaluatePolicies()

        console.log(`\n✅ Evaluation complete:`)
        console.log(`   Evaluated: ${result.evaluated_count} memories`)
        console.log(`   Expired: ${result.expired_count} memories`)
        console.log(`   Actions taken: ${result.actions_taken.length}`)

        if (result.actions_taken.length > 0) {
          console.log(`\nActions:`)
          for (const action of result.actions_taken) {
            console.log(`   ${action.memory_id}: ${action.action} (policy: ${action.policy_id})`)
          }
        }
      },
    },

    // Similar for create...
  ],
}
```

**Tests:**
- Deprecate memory command
- Supersede memory command
- Reactivate memory command
- Validate memory command
- Show staleness command
- TTL commands (list, evaluate)

---

### Epic 5: Integration & Testing
**Duration:** 2 days (end of phase)
**Priority:** P0 (Blocking)

#### Epic 5.1: Integration Tests
**Estimate:** 6 hours
**Owner:** TBD

**Description:**
End-to-end integration tests for lifecycle management.

**Test Scenarios:**

```typescript
describe('Lifecycle Management Integration', () => {
  it('should complete full supersession workflow', async () => {
    // Create original memory
    const original = await edda.memories.create(admin, {
      type: 'pattern',
      statement: 'Use callbacks for async operations',
      tags: ['javascript'],
    })

    // Supersede with new memory
    const result = await edda.lifecycle.supersedeMemory(
      original.id,
      {
        type: 'pattern',
        statement: 'Use async/await for async operations',
        tags: ['javascript'],
      },
      admin,
    )

    expect(result.old.status).toBe('superseded')
    expect(result.old.lifecycle.superseded_by).toBe(result.new.id)
    expect(result.new.lifecycle.supersedes).toContain(original.id)
  })

  it('should detect stale memories', async () => {
    // Create old memory (backdated)
    const oldMemory = await createMemory({
      type: 'warning',
      statement: 'Old warning',
      created_at: '2024-01-01T00:00:00Z',  // 1+ year old
    })

    // Analyze staleness
    const result = await edda.staleness.analyzeMemory(oldMemory)

    expect(result.staleness_score).toBeGreaterThan(0.7)
    expect(result.needs_review).toBe(true)
  })

  it('should apply TTL policies', async () => {
    // Create TTL policy
    const policy: TTLPolicy = {
      policy_id: 'test-ttl',
      name: 'Test TTL',
      description: 'Test policy',
      enabled: true,
      memory_type: 'warning',
      ttl_days: 30,
      expiry_action: 'deprecate',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    }
    await edda.ttl.upsertPolicy(policy)

    // Create old warning (backdated)
    const oldWarning = await createMemory({
      type: 'warning',
      statement: 'Old warning',
      created_at: '2025-01-01T00:00:00Z',  // 1+ month old
    })

    // Evaluate policies
    const result = await edda.ttl.evaluatePolicies()

    expect(result.expired_count).toBeGreaterThan(0)
    expect(result.actions_taken.some(a => a.memory_id === oldWarning.id)).toBe(true)

    // Verify memory deprecated
    const updated = await edda.memories.get(oldWarning.id)
    expect(updated.status).toBe('deprecated')
  })
})
```

**Tests:**
- Full supersession workflow
- Staleness detection works
- TTL policies applied correctly
- Validation resets staleness
- Status transitions validated
- 100% test coverage

---

## Timeline

### Week 1 (Days 1-5)
- **Day 1-2:** Epic 1 (Memory Status Transitions)
- **Day 3-4:** Epic 2 (Staleness Detection)
- **Day 5:** Epic 3 (Aggressive Forgetting) - Part 1

### Week 2 (Days 6-10)
- **Day 6:** Epic 3 (Aggressive Forgetting) - Part 2
- **Day 7-8:** Epic 4 (CLI Commands)
- **Day 9-10:** Epic 5 (Integration & Testing)

---

## Deliverables

### Package Structure
```
packages/edda-core/src/lifecycle/
├── status-machine.ts
├── lifecycle-service.ts
├── staleness-analyzer.ts
├── validation-service.ts
├── ttl-policy.ts
├── lifecycle-scheduler.ts
└── __tests__/
    ├── status-machine.test.ts
    ├── lifecycle-service.test.ts
    ├── staleness-analyzer.test.ts
    ├── validation-service.test.ts
    ├── ttl-policy.test.ts
    └── integration/
        └── lifecycle.integration.test.ts

packages/anvil/src/commands/edda/
├── deprecate.ts
├── supersede.ts
├── reactivate.ts
├── validate.ts
├── staleness.ts
└── ttl.ts
```

### Storage Structure
```
.edda/policies/ttl/
├── warnings-90d.yaml
├── temp-scope-30d.yaml
└── lessons-180d.yaml
```

### Documentation
- Lifecycle management guide
- TTL policy configuration guide
- Staleness criteria reference

### Tests
- Unit tests: 40+ tests
- Integration tests: 10+ scenarios
- Test coverage: 100%

---

## Success Metrics

### Functional
- ✅ Status transitions validated correctly
- ✅ Supersession chains tracked
- ✅ Staleness detection accurate
- ✅ TTL policies applied automatically
- ✅ CLI commands operational

### Quality
- ✅ 100% test coverage
- ✅ All edge cases handled
- ✅ Clear error messages
- ✅ Audit trail complete

---

## Risks & Mitigation

### Risk 1: Aggressive Forgetting Too Aggressive
**Probability:** Medium
**Impact:** High

**Mitigation:**
- Start with conservative TTL periods
- Grace periods before action
- Audit all TTL actions
- Admin override capability
- Notify before auto-deprecation

### Risk 2: Staleness Criteria Inaccurate
**Probability:** Medium
**Impact:** Medium

**Mitigation:**
- Type-specific criteria
- Tunable thresholds
- User feedback collection
- Regular criteria review

---

## Dependencies

### Upstream (Must Complete First)
- Phase 0: Foundation (memory storage)
- Phase 3: Query & Retrieval (memory queries)

### Downstream (Blocked By This Phase)
- Phase 7: Meta-Capabilities (lifecycle analytics)

---

## Next Steps

1. ✅ Complete Phase 0 (Foundation)
2. ✅ Complete Phase 3 (Query & Retrieval)
3. **Review this APS document** with team
4. **Define default TTL policies** for organization
5. **Assign owners** to epics and tasks
6. **Kick off Phase 5** implementation

---

**Document Version:** 1.0
**Last Updated:** 2026-01-19
**Status:** Ready for Review
**Estimated Completion:** 2 weeks after Phase 3 completion
