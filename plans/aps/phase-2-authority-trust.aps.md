# Phase 2: Authority & Trust - APS Document

**Phase:** 2 of 7
**Duration:** 2 weeks (10 working days)
**Dependencies:** Phase 0 (Foundation), Phase 1 (Promotion Pipeline)
**Status:** Not Started
**Owner:** TBD

---

## Phase Overview

### Purpose
Implement Role-Based Access Control (RBAC), agent trust scoring, and audit trail systems to ensure proper authority validation for all memory operations and promotion decisions.

### Scope
This phase delivers the authority layer that controls who can create, modify, and delete memories, as well as the trust scoring system that adjusts agent confidence based on historical performance.

### Success Criteria
- ✅ 5-level RBAC system operational (system → readonly)
- ✅ Permission checks enforce authority policies on all operations
- ✅ Agent trust profiles track performance and adjust confidence
- ✅ Complete audit trail captures all authority decisions
- ✅ CLI commands for role/trust management working
- ✅ <10ms permission check overhead
- ✅ 100% test coverage on authority checks

---

## Epic Breakdown

### Epic 1: Principal & Role System
**Duration:** 2 days
**Priority:** P0 (Blocking)

#### Epic 1.1: Principal Schema & Storage
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Define and implement the Principal identity model with role assignments.

**Acceptance Criteria:**
- Principal interface matches edda-extended.ts contract
- Principals stored in `.edda/principals/`
- YAML format: `{principal-type}-{identifier}.yaml`
- GitHub OAuth principals supported
- Validation via Zod schema

**Implementation:**

```typescript
// packages/edda-core/src/authority/principal.ts

import { z } from 'zod'

export const PrincipalSchema = z.object({
  identifier: z.string().min(1).max(255),
  principal_type: z.enum(['user', 'agent', 'service', 'system']),
  display_name: z.string().optional(),
  roles: z.array(z.enum(['system', 'org_admin', 'team_lead', 'contributor', 'agent', 'readonly'])),
  metadata: z.record(z.any()).optional(),
  created_at: z.string().datetime(),
  updated_at: z.string().datetime(),
})

export type Principal = z.infer<typeof PrincipalSchema>

export interface IPrincipalRepository {
  /**
   * Get principal by identifier
   * @throws PrincipalNotFoundError if not found
   */
  get(identifier: string): Promise<Principal>

  /**
   * List all principals (optionally filtered by type)
   */
  list(filter?: { principal_type?: PrincipalType }): Promise<Principal[]>

  /**
   * Create or update principal
   */
  upsert(principal: Principal): Promise<void>

  /**
   * Delete principal
   */
  delete(identifier: string): Promise<void>

  /**
   * Assign role to principal
   */
  assignRole(identifier: string, role: AuthorityLevel): Promise<void>

  /**
   * Revoke role from principal
   */
  revokeRole(identifier: string, role: AuthorityLevel): Promise<void>
}
```

**File Structure:**
```
packages/edda-core/src/authority/
├── principal.ts          # Principal schema & repository interface
├── principal-repo.ts     # Git-backed YAML implementation
└── __tests__/
    └── principal-repo.test.ts
```

**Tests:**
- Create principal with valid data
- Reject invalid principal (missing required fields)
- List principals filtered by type
- Assign/revoke roles
- Update existing principal
- Delete principal

---

#### Epic 1.2: Role Hierarchy & Inheritance
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Implement 5-level role hierarchy with inheritance rules.

**Acceptance Criteria:**
- 5 authority levels: system > org_admin > team_lead > contributor > agent > readonly
- Higher roles inherit lower role permissions
- Role comparison utilities (isHigherRole, canActAs)
- Default role assignments for new principals

**Implementation:**

```typescript
// packages/edda-core/src/authority/roles.ts

export enum AuthorityLevel {
  SYSTEM = 'system',
  ORG_ADMIN = 'org_admin',
  TEAM_LEAD = 'team_lead',
  CONTRIBUTOR = 'contributor',
  AGENT = 'agent',
  READONLY = 'readonly',
}

const ROLE_HIERARCHY: Record<AuthorityLevel, number> = {
  [AuthorityLevel.SYSTEM]: 100,
  [AuthorityLevel.ORG_ADMIN]: 80,
  [AuthorityLevel.TEAM_LEAD]: 60,
  [AuthorityLevel.CONTRIBUTOR]: 40,
  [AuthorityLevel.AGENT]: 20,
  [AuthorityLevel.READONLY]: 0,
}

export class RoleHierarchy {
  /**
   * Check if roleA is higher than roleB in hierarchy
   */
  static isHigherRole(roleA: AuthorityLevel, roleB: AuthorityLevel): boolean {
    return ROLE_HIERARCHY[roleA] > ROLE_HIERARCHY[roleB]
  }

  /**
   * Check if principal with roleA can act as roleB
   * (roleA >= roleB in hierarchy)
   */
  static canActAs(roleA: AuthorityLevel, roleB: AuthorityLevel): boolean {
    return ROLE_HIERARCHY[roleA] >= ROLE_HIERARCHY[roleB]
  }

  /**
   * Get all roles that inherit from given role
   */
  static getInheritedRoles(role: AuthorityLevel): AuthorityLevel[] {
    const level = ROLE_HIERARCHY[role]
    return Object.entries(ROLE_HIERARCHY)
      .filter(([_, value]) => value <= level)
      .map(([key, _]) => key as AuthorityLevel)
  }

  /**
   * Get default role for principal type
   */
  static getDefaultRole(principalType: PrincipalType): AuthorityLevel {
    switch (principalType) {
      case 'system': return AuthorityLevel.SYSTEM
      case 'user': return AuthorityLevel.CONTRIBUTOR
      case 'agent': return AuthorityLevel.AGENT
      case 'service': return AuthorityLevel.READONLY
      default: return AuthorityLevel.READONLY
    }
  }
}
```

**Tests:**
- Role hierarchy ordering correct
- canActAs logic works (org_admin can act as contributor)
- Inherited roles calculated correctly
- Default roles assigned by principal type

---

### Epic 2: Permission System
**Duration:** 2 days
**Priority:** P0 (Blocking)

#### Epic 2.1: Permission Definitions
**Estimate:** 3 hours
**Owner:** TBD

**Description:**
Define all permission types and role-permission mappings.

**Acceptance Criteria:**
- Permission enum covers all operations (memory CRUD, role management, etc.)
- Role-permission matrix defined
- Permission checking utilities

**Implementation:**

```typescript
// packages/edda-core/src/authority/permissions.ts

export enum Permission {
  // Memory operations
  MEMORY_READ = 'memory:read',
  MEMORY_CREATE = 'memory:create',
  MEMORY_UPDATE = 'memory:update',
  MEMORY_DELETE = 'memory:delete',
  MEMORY_PROMOTE = 'memory:promote',

  // Promotion operations
  PROPOSAL_SUBMIT = 'proposal:submit',
  PROPOSAL_APPROVE = 'proposal:approve',
  PROPOSAL_REJECT = 'proposal:reject',

  // Role management
  ROLE_ASSIGN = 'role:assign',
  ROLE_REVOKE = 'role:revoke',
  PRINCIPAL_CREATE = 'principal:create',
  PRINCIPAL_DELETE = 'principal:delete',

  // Trust management
  TRUST_VIEW = 'trust:view',
  TRUST_ADJUST = 'trust:adjust',

  // Audit
  AUDIT_VIEW = 'audit:view',

  // Configuration
  CONFIG_UPDATE = 'config:update',
}

const ROLE_PERMISSIONS: Record<AuthorityLevel, Permission[]> = {
  [AuthorityLevel.SYSTEM]: [
    // System can do everything
    ...Object.values(Permission),
  ],

  [AuthorityLevel.ORG_ADMIN]: [
    Permission.MEMORY_READ,
    Permission.MEMORY_CREATE,
    Permission.MEMORY_UPDATE,
    Permission.MEMORY_DELETE,
    Permission.MEMORY_PROMOTE,
    Permission.PROPOSAL_APPROVE,
    Permission.PROPOSAL_REJECT,
    Permission.ROLE_ASSIGN,
    Permission.ROLE_REVOKE,
    Permission.PRINCIPAL_CREATE,
    Permission.PRINCIPAL_DELETE,
    Permission.TRUST_VIEW,
    Permission.TRUST_ADJUST,
    Permission.AUDIT_VIEW,
    Permission.CONFIG_UPDATE,
  ],

  [AuthorityLevel.TEAM_LEAD]: [
    Permission.MEMORY_READ,
    Permission.MEMORY_CREATE,
    Permission.MEMORY_UPDATE,
    Permission.PROPOSAL_APPROVE,
    Permission.PROPOSAL_REJECT,
    Permission.TRUST_VIEW,
    Permission.AUDIT_VIEW,
  ],

  [AuthorityLevel.CONTRIBUTOR]: [
    Permission.MEMORY_READ,
    Permission.MEMORY_CREATE,
    Permission.PROPOSAL_APPROVE,
    Permission.PROPOSAL_REJECT,
    Permission.AUDIT_VIEW,
  ],

  [AuthorityLevel.AGENT]: [
    Permission.PROPOSAL_SUBMIT,
    Permission.MEMORY_READ,
  ],

  [AuthorityLevel.READONLY]: [
    Permission.MEMORY_READ,
  ],
}

export class PermissionChecker {
  /**
   * Check if role has permission
   */
  static hasPermission(role: AuthorityLevel, permission: Permission): boolean {
    const permissions = ROLE_PERMISSIONS[role]
    return permissions.includes(permission)
  }

  /**
   * Check if any of principal's roles has permission
   */
  static principalHasPermission(principal: Principal, permission: Permission): boolean {
    return principal.roles.some(role => this.hasPermission(role, permission))
  }

  /**
   * Get all permissions for role
   */
  static getPermissions(role: AuthorityLevel): Permission[] {
    return ROLE_PERMISSIONS[role]
  }
}
```

**Tests:**
- All roles have correct permissions
- System role has all permissions
- Readonly role has only read permission
- principalHasPermission works with multiple roles

---

#### Epic 2.2: Authorization Service
**Estimate:** 5 hours
**Owner:** TBD

**Description:**
Implement the authorization service that enforces permission checks across all operations.

**Acceptance Criteria:**
- IAuthorizationService interface implemented
- authorize() method checks principal permissions
- authorizeMemoryOperation() validates memory-specific rules
- Throws UnauthorizedError on failure
- <10ms overhead per check

**Implementation:**

```typescript
// packages/edda-core/src/authority/authorization-service.ts

export class UnauthorizedError extends Error {
  constructor(
    public principal: string,
    public permission: Permission,
    public resource?: string,
  ) {
    super(`Principal ${principal} does not have permission ${permission}${resource ? ` on ${resource}` : ''}`)
    this.name = 'UnauthorizedError'
  }
}

export interface IAuthorizationService {
  /**
   * Check if principal has permission
   * @throws UnauthorizedError if not authorized
   */
  authorize(principal: Principal, permission: Permission): Promise<void>

  /**
   * Check if principal can perform memory operation
   * @throws UnauthorizedError if not authorized
   */
  authorizeMemoryOperation(
    principal: Principal,
    operation: 'create' | 'read' | 'update' | 'delete',
    memory?: MemoryObject,
  ): Promise<void>

  /**
   * Check if principal can approve promotion
   * @throws UnauthorizedError if not authorized
   */
  authorizePromotion(
    principal: Principal,
    proposal: PromotionRequest,
  ): Promise<void>

  /**
   * Check authorization without throwing (returns boolean)
   */
  isAuthorized(principal: Principal, permission: Permission): Promise<boolean>
}

export class AuthorizationService implements IAuthorizationService {
  constructor(
    private principalRepo: IPrincipalRepository,
  ) {}

  async authorize(principal: Principal, permission: Permission): Promise<void> {
    const hasPermission = PermissionChecker.principalHasPermission(principal, permission)
    if (!hasPermission) {
      throw new UnauthorizedError(principal.identifier, permission)
    }
  }

  async authorizeMemoryOperation(
    principal: Principal,
    operation: 'create' | 'read' | 'update' | 'delete',
    memory?: MemoryObject,
  ): Promise<void> {
    // Map operation to permission
    const permissionMap = {
      create: Permission.MEMORY_CREATE,
      read: Permission.MEMORY_READ,
      update: Permission.MEMORY_UPDATE,
      delete: Permission.MEMORY_DELETE,
    }

    const permission = permissionMap[operation]
    await this.authorize(principal, permission)

    // Additional checks for update/delete
    if ((operation === 'update' || operation === 'delete') && memory) {
      // Only author or higher role can modify
      if (memory.authority.author !== principal.identifier) {
        const authorPrincipal = await this.principalRepo.get(memory.authority.author)
        const canOverride = principal.roles.some(role =>
          authorPrincipal.roles.some(authorRole => RoleHierarchy.isHigherRole(role, authorRole))
        )

        if (!canOverride) {
          throw new UnauthorizedError(
            principal.identifier,
            permission,
            `memory ${memory.id} (author: ${memory.authority.author})`
          )
        }
      }
    }
  }

  async authorizePromotion(
    principal: Principal,
    proposal: PromotionRequest,
  ): Promise<void> {
    // Must have approval permission
    await this.authorize(principal, Permission.PROPOSAL_APPROVE)

    // Cannot approve own proposals
    if (proposal.proposed_by_agent && proposal.proposed_by_agent === principal.identifier) {
      throw new UnauthorizedError(
        principal.identifier,
        Permission.PROPOSAL_APPROVE,
        'cannot approve own proposal'
      )
    }
  }

  async isAuthorized(principal: Principal, permission: Permission): Promise<boolean> {
    try {
      await this.authorize(principal, permission)
      return true
    } catch (error) {
      if (error instanceof UnauthorizedError) {
        return false
      }
      throw error
    }
  }
}
```

**Tests:**
- Authorized operations succeed
- Unauthorized operations throw UnauthorizedError
- Memory author can update/delete own memories
- Higher roles can override lower role memories
- Cannot approve own proposals
- Performance: <10ms per check

---

### Epic 3: Authority Metadata
**Duration:** 1 day
**Priority:** P0 (Blocking)

#### Epic 3.1: Authority Metadata on Memories
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Attach authority metadata to all memory objects.

**Acceptance Criteria:**
- AuthorityMetadata interface implemented
- author, created_at, updated_at tracked
- reviewer, approved_at tracked for promoted memories
- Authority metadata immutable after creation

**Implementation:**

```typescript
// packages/edda-core/src/authority/metadata.ts

export interface AuthorityMetadata {
  author: string              // Principal identifier
  created_at: string          // ISO 8601
  updated_at: string          // ISO 8601
  reviewer?: string           // Principal who approved (for promoted memories)
  approved_at?: string        // ISO 8601
  modification_history: Array<{
    modified_by: string
    modified_at: string
    operation: 'create' | 'update' | 'delete'
    reason?: string
  }>
}

export class AuthorityMetadataBuilder {
  /**
   * Create authority metadata for new memory
   */
  static create(author: string): AuthorityMetadata {
    const now = new Date().toISOString()
    return {
      author,
      created_at: now,
      updated_at: now,
      modification_history: [{
        modified_by: author,
        modified_at: now,
        operation: 'create',
      }],
    }
  }

  /**
   * Add approval metadata
   */
  static approve(
    metadata: AuthorityMetadata,
    reviewer: string,
  ): AuthorityMetadata {
    return {
      ...metadata,
      reviewer,
      approved_at: new Date().toISOString(),
    }
  }

  /**
   * Record modification
   */
  static recordModification(
    metadata: AuthorityMetadata,
    modifiedBy: string,
    operation: 'update' | 'delete',
    reason?: string,
  ): AuthorityMetadata {
    const now = new Date().toISOString()
    return {
      ...metadata,
      updated_at: now,
      modification_history: [
        ...metadata.modification_history,
        {
          modified_by: modifiedBy,
          modified_at: now,
          operation,
          reason,
        },
      ],
    }
  }
}
```

**Tests:**
- Create authority metadata
- Approve memory (adds reviewer)
- Record modification (updates history)
- Modification history ordered by time

---

### Epic 4: Agent Trust Profiles
**Duration:** 2 days
**Priority:** P1 (Important)

#### Epic 4.1: Trust Profile Schema & Storage
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Implement agent trust profile storage and CRUD operations.

**Acceptance Criteria:**
- AgentTrustProfile interface implemented
- Stored in `.edda/trust/agent-{agent-id}.yaml`
- Tracks proposals submitted/approved/rejected
- Trust score calculated: approval_rate * 100
- Confidence adjustment: -0.2 to +0.2 based on trust

**Implementation:**

```typescript
// packages/edda-core/src/authority/trust-profile.ts

export const AgentTrustProfileSchema = z.object({
  agent_id: z.string(),
  trust_score: z.number().min(0).max(100),
  proposals_submitted: z.number().int().min(0),
  proposals_approved: z.number().int().min(0),
  proposals_rejected: z.number().int().min(0),
  approval_rate: z.number().min(0).max(1),
  confidence_adjustment: z.number().min(-0.2).max(0.2),
  last_updated: z.string().datetime(),
  performance_trend: z.enum(['improving', 'stable', 'declining']).optional(),
})

export type AgentTrustProfile = z.infer<typeof AgentTrustProfileSchema>

export interface ITrustProfileRepository {
  get(agentId: string): Promise<AgentTrustProfile>
  list(): Promise<AgentTrustProfile[]>
  update(profile: AgentTrustProfile): Promise<void>

  /**
   * Record proposal outcome and recalculate trust
   */
  recordProposalOutcome(
    agentId: string,
    outcome: 'approved' | 'rejected',
  ): Promise<AgentTrustProfile>
}

export class TrustCalculator {
  /**
   * Calculate trust score from approval rate
   * trust_score = approval_rate * 100
   */
  static calculateTrustScore(approvalRate: number): number {
    return Math.round(approvalRate * 100)
  }

  /**
   * Calculate confidence adjustment based on trust score
   * - Low trust (<50): -0.2 to -0.1
   * - Medium trust (50-80): -0.1 to +0.1
   * - High trust (>80): +0.1 to +0.2
   */
  static calculateConfidenceAdjustment(trustScore: number): number {
    if (trustScore < 50) {
      // Low trust: -0.2 to -0.1
      return -0.2 + (trustScore / 50) * 0.1
    } else if (trustScore <= 80) {
      // Medium trust: -0.1 to +0.1
      return -0.1 + ((trustScore - 50) / 30) * 0.2
    } else {
      // High trust: +0.1 to +0.2
      return 0.1 + ((trustScore - 80) / 20) * 0.1
    }
  }

  /**
   * Determine performance trend
   */
  static calculateTrend(
    currentRate: number,
    previousRate: number | undefined,
  ): 'improving' | 'stable' | 'declining' {
    if (!previousRate) return 'stable'
    const delta = currentRate - previousRate
    if (delta > 0.05) return 'improving'
    if (delta < -0.05) return 'declining'
    return 'stable'
  }
}
```

**File Structure:**
```
packages/edda-core/src/authority/
├── trust-profile.ts
├── trust-profile-repo.ts     # Git-backed YAML
├── trust-calculator.ts
└── __tests__/
    ├── trust-profile-repo.test.ts
    └── trust-calculator.test.ts
```

**Tests:**
- Create trust profile for new agent
- Record approved proposal (increases approval_rate)
- Record rejected proposal (decreases approval_rate)
- Trust score calculation correct
- Confidence adjustment ranges correct
- Performance trend detection

---

#### Epic 4.2: Trust Integration with Promotion
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Integrate trust scoring into promotion pipeline to adjust proposal confidence.

**Acceptance Criteria:**
- Promotion service applies confidence adjustment from trust profile
- Low-trust agents have confidence reduced
- High-trust agents have confidence increased
- Trust profile updated after approval/rejection

**Implementation:**

```typescript
// packages/edda-core/src/promotion/promotion-service.ts (enhancement)

export class PromotionService {
  constructor(
    private trustProfileRepo: ITrustProfileRepository,
    // ... other dependencies
  ) {}

  async createPromotionRequest(
    proposal: EmberProposal,
    agentId: string,
  ): Promise<PromotionRequest> {
    // Get agent trust profile
    let trustProfile: AgentTrustProfile
    try {
      trustProfile = await this.trustProfileRepo.get(agentId)
    } catch (error) {
      // Create new profile for first-time agent
      trustProfile = {
        agent_id: agentId,
        trust_score: 50, // Neutral starting score
        proposals_submitted: 0,
        proposals_approved: 0,
        proposals_rejected: 0,
        approval_rate: 0.5,
        confidence_adjustment: 0,
        last_updated: new Date().toISOString(),
      }
    }

    // Apply confidence adjustment
    const baseConfidence = this.mapConfidence(proposal.confidence)
    const adjustedConfidence = this.adjustConfidence(
      baseConfidence,
      trustProfile.confidence_adjustment,
    )

    // Create promotion request
    const request: PromotionRequest = {
      request_id: generateRequestId(),
      proposal_id: proposal.id,
      proposed_by_agent: agentId,
      proposed_memory: {
        // ... map proposal to memory
        confidence: adjustedConfidence,
      },
      trust_adjustment: trustProfile.confidence_adjustment,
      // ... rest of request
    }

    // Increment proposals_submitted
    trustProfile.proposals_submitted++
    await this.trustProfileRepo.update(trustProfile)

    return request
  }

  private adjustConfidence(
    confidence: EddaConfidenceLevel,
    adjustment: number,
  ): EddaConfidenceLevel {
    // Map to numeric, adjust, map back
    const confidenceMap: Record<EddaConfidenceLevel, number> = {
      'high': 0.9,
      'medium': 0.6,
      'low': 0.3,
    }

    const reverseMap: Array<[number, EddaConfidenceLevel]> = [
      [0.8, 'high'],
      [0.5, 'medium'],
      [0, 'low'],
    ]

    let numeric = confidenceMap[confidence]
    numeric = Math.max(0, Math.min(1, numeric + adjustment))

    for (const [threshold, level] of reverseMap) {
      if (numeric >= threshold) return level
    }
    return 'low'
  }

  async approvePromotion(
    requestId: string,
    reviewer: Principal,
  ): Promise<MemoryObject> {
    // ... existing approval logic

    // Update trust profile
    const request = await this.getPromotionRequest(requestId)
    if (request.proposed_by_agent) {
      await this.trustProfileRepo.recordProposalOutcome(
        request.proposed_by_agent,
        'approved',
      )
    }

    // ... return created memory
  }

  async rejectPromotion(
    requestId: string,
    reviewer: Principal,
    reason: string,
  ): Promise<void> {
    // ... existing rejection logic

    // Update trust profile
    const request = await this.getPromotionRequest(requestId)
    if (request.proposed_by_agent) {
      await this.trustProfileRepo.recordProposalOutcome(
        request.proposed_by_agent,
        'rejected',
      )
    }
  }
}
```

**Tests:**
- New agent starts with neutral trust (0 adjustment)
- High-trust agent gets confidence boost
- Low-trust agent gets confidence penalty
- Trust profile updated after approval
- Trust profile updated after rejection

---

### Epic 5: Audit Trail
**Duration:** 2 days
**Priority:** P1 (Important)

#### Epic 5.1: Audit Entry Schema & Storage
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Implement audit trail storage for all authority-related events.

**Acceptance Criteria:**
- AuditEntry interface implemented
- Stored in `.edda/audit/YYYY-MM-DD.jsonl` (JSONL for append-only)
- Captures: actor, action, resource, timestamp, outcome, metadata
- Indexed in SQLite for fast queries

**Implementation:**

```typescript
// packages/edda-core/src/authority/audit-trail.ts

export interface AuditEntry {
  entry_id: string
  timestamp: string           // ISO 8601
  actor: string               // Principal identifier
  action: AuditAction
  resource_type: 'memory' | 'proposal' | 'principal' | 'role' | 'trust'
  resource_id: string
  outcome: 'success' | 'failure' | 'unauthorized'
  error?: string              // If outcome is failure
  metadata?: Record<string, unknown>
}

export enum AuditAction {
  MEMORY_CREATE = 'memory.create',
  MEMORY_UPDATE = 'memory.update',
  MEMORY_DELETE = 'memory.delete',
  MEMORY_READ = 'memory.read',

  PROPOSAL_SUBMIT = 'proposal.submit',
  PROPOSAL_APPROVE = 'proposal.approve',
  PROPOSAL_REJECT = 'proposal.reject',

  ROLE_ASSIGN = 'role.assign',
  ROLE_REVOKE = 'role.revoke',

  PRINCIPAL_CREATE = 'principal.create',
  PRINCIPAL_DELETE = 'principal.delete',

  TRUST_ADJUST = 'trust.adjust',
}

export interface IAuditTrailService {
  /**
   * Log an audit entry
   */
  log(entry: Omit<AuditEntry, 'entry_id' | 'timestamp'>): Promise<void>

  /**
   * Query audit trail
   */
  query(filter: AuditFilter): Promise<AuditEntry[]>
}

export interface AuditFilter {
  actor?: string
  action?: AuditAction
  resource_type?: string
  resource_id?: string
  outcome?: 'success' | 'failure' | 'unauthorized'
  start_date?: string
  end_date?: string
  limit?: number
}

export class AuditTrailService implements IAuditTrailService {
  constructor(
    private storage: IAuditStorage,  // JSONL writer
    private index: IAuditIndex,      // SQLite indexer
  ) {}

  async log(entry: Omit<AuditEntry, 'entry_id' | 'timestamp'>): Promise<void> {
    const fullEntry: AuditEntry = {
      entry_id: generateEntryId(),
      timestamp: new Date().toISOString(),
      ...entry,
    }

    // Append to JSONL file
    await this.storage.append(fullEntry)

    // Index in SQLite
    await this.index.insert(fullEntry)
  }

  async query(filter: AuditFilter): Promise<AuditEntry[]> {
    // Query SQLite index, then fetch from JSONL
    return this.index.query(filter)
  }
}
```

**File Structure:**
```
packages/edda-core/src/authority/
├── audit-trail.ts
├── audit-storage.ts          # JSONL append-only storage
├── audit-index.ts            # SQLite indexer
└── __tests__/
    ├── audit-trail.test.ts
    └── audit-query.test.ts
```

**Storage Format:**
```
.edda/audit/
├── 2026-01-19.jsonl
├── 2026-01-20.jsonl
└── index.db                  # SQLite index
```

**SQLite Schema:**
```sql
CREATE TABLE audit_entries (
  entry_id TEXT PRIMARY KEY,
  timestamp TEXT NOT NULL,
  actor TEXT NOT NULL,
  action TEXT NOT NULL,
  resource_type TEXT NOT NULL,
  resource_id TEXT NOT NULL,
  outcome TEXT NOT NULL,
  error TEXT,
  metadata_json TEXT
);

CREATE INDEX idx_audit_actor ON audit_entries(actor);
CREATE INDEX idx_audit_action ON audit_entries(action);
CREATE INDEX idx_audit_resource ON audit_entries(resource_type, resource_id);
CREATE INDEX idx_audit_timestamp ON audit_entries(timestamp);
```

**Tests:**
- Log audit entry (written to JSONL and indexed)
- Query by actor
- Query by action
- Query by resource
- Query by date range
- Query with limit

---

#### Epic 5.2: Audit Integration
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Integrate audit logging into all authority-sensitive operations.

**Acceptance Criteria:**
- All authorization checks logged (success and failure)
- All memory operations logged
- All promotion actions logged
- All role changes logged
- Trust adjustments logged

**Implementation:**

```typescript
// Audit decorators for key services

export class AuditedAuthorizationService implements IAuthorizationService {
  constructor(
    private inner: IAuthorizationService,
    private audit: IAuditTrailService,
  ) {}

  async authorize(principal: Principal, permission: Permission): Promise<void> {
    try {
      await this.inner.authorize(principal, permission)
      await this.audit.log({
        actor: principal.identifier,
        action: AuditAction.PERMISSION_CHECK,
        resource_type: 'permission',
        resource_id: permission,
        outcome: 'success',
      })
    } catch (error) {
      await this.audit.log({
        actor: principal.identifier,
        action: AuditAction.PERMISSION_CHECK,
        resource_type: 'permission',
        resource_id: permission,
        outcome: error instanceof UnauthorizedError ? 'unauthorized' : 'failure',
        error: error.message,
      })
      throw error
    }
  }

  // Similar wrapping for other methods...
}

// Similar audit wrappers for:
// - MemoryManager (CRUD operations)
// - PromotionService (approve/reject)
// - PrincipalRepository (role changes)
// - TrustProfileRepository (trust adjustments)
```

**Tests:**
- Successful authorization logged
- Failed authorization logged
- Memory operations logged
- Promotion actions logged
- Role changes logged

---

### Epic 6: CLI Commands
**Duration:** 1 day
**Priority:** P1 (Important)

#### Epic 6.1: Role Management CLI
**Estimate:** 3 hours
**Owner:** TBD

**Description:**
Implement CLI commands for managing principals and roles.

**Acceptance Criteria:**
- `anvil edda roles list` - List all principals with roles
- `anvil edda roles assign <principal> <role>` - Assign role
- `anvil edda roles revoke <principal> <role>` - Revoke role
- `anvil edda principals create <identifier> --type <type>` - Create principal
- `anvil edda principals delete <identifier>` - Delete principal

**Implementation:**

```typescript
// packages/anvil/src/commands/edda/roles.ts

export const rolesCommand: Command = {
  name: 'roles',
  description: 'Manage Edda roles and principals',
  subcommands: [
    {
      name: 'list',
      description: 'List all principals with their roles',
      async execute(context) {
        const principals = await context.edda.principals.list()

        console.log('Principals and Roles:\n')
        for (const principal of principals) {
          console.log(`${principal.identifier} (${principal.principal_type})`)
          console.log(`  Roles: ${principal.roles.join(', ')}`)
          console.log(`  Created: ${principal.created_at}`)
          console.log()
        }
      },
    },

    {
      name: 'assign',
      description: 'Assign role to principal',
      args: [
        { name: 'principal', required: true },
        { name: 'role', required: true },
      ],
      async execute(context, args) {
        const { principal, role } = args

        // Check authorization
        await context.edda.authorization.authorize(
          context.currentPrincipal,
          Permission.ROLE_ASSIGN,
        )

        // Assign role
        await context.edda.principals.assignRole(principal, role as AuthorityLevel)

        console.log(`✅ Assigned role '${role}' to principal '${principal}'`)
      },
    },

    // Similar for revoke...
  ],
}

export const principalsCommand: Command = {
  name: 'principals',
  description: 'Manage Edda principals',
  subcommands: [
    {
      name: 'create',
      description: 'Create a new principal',
      args: [
        { name: 'identifier', required: true },
      ],
      options: [
        { name: 'type', required: true, choices: ['user', 'agent', 'service', 'system'] },
        { name: 'display-name', required: false },
      ],
      async execute(context, args, options) {
        const { identifier } = args
        const { type, 'display-name': displayName } = options

        // Check authorization
        await context.edda.authorization.authorize(
          context.currentPrincipal,
          Permission.PRINCIPAL_CREATE,
        )

        // Create principal
        const principal: Principal = {
          identifier,
          principal_type: type as PrincipalType,
          display_name: displayName,
          roles: [RoleHierarchy.getDefaultRole(type as PrincipalType)],
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        }

        await context.edda.principals.upsert(principal)

        console.log(`✅ Created principal '${identifier}' with role '${principal.roles[0]}'`)
      },
    },

    // Similar for delete...
  ],
}
```

**Tests:**
- List principals works
- Assign role requires permission
- Cannot assign invalid role
- Create principal with default role
- Delete principal requires permission

---

#### Epic 6.2: Trust Management CLI
**Estimate:** 3 hours
**Owner:** TBD

**Description:**
Implement CLI commands for viewing and managing agent trust.

**Acceptance Criteria:**
- `anvil edda trust list` - List all agent trust profiles
- `anvil edda trust show <agent-id>` - Show detailed trust profile
- `anvil edda trust adjust <agent-id> <adjustment>` - Manually adjust trust (admin only)

**Implementation:**

```typescript
// packages/anvil/src/commands/edda/trust.ts

export const trustCommand: Command = {
  name: 'trust',
  description: 'Manage agent trust profiles',
  subcommands: [
    {
      name: 'list',
      description: 'List all agent trust profiles',
      options: [
        { name: 'sort', choices: ['trust', 'proposals', 'rate'], default: 'trust' },
      ],
      async execute(context, args, options) {
        const profiles = await context.edda.trust.list()

        // Sort
        profiles.sort((a, b) => {
          if (options.sort === 'trust') return b.trust_score - a.trust_score
          if (options.sort === 'proposals') return b.proposals_submitted - a.proposals_submitted
          if (options.sort === 'rate') return b.approval_rate - a.approval_rate
          return 0
        })

        console.log('Agent Trust Profiles:\n')
        console.log('AGENT ID                  | TRUST | PROPOSALS | APPROVED | RATE  | ADJUSTMENT | TREND')
        console.log('─'.repeat(90))

        for (const profile of profiles) {
          const trendIcon = {
            improving: '↑',
            stable: '→',
            declining: '↓',
          }[profile.performance_trend || 'stable']

          console.log(
            `${profile.agent_id.padEnd(25)} | ` +
            `${String(profile.trust_score).padStart(5)} | ` +
            `${String(profile.proposals_submitted).padStart(9)} | ` +
            `${String(profile.proposals_approved).padStart(8)} | ` +
            `${(profile.approval_rate * 100).toFixed(1).padStart(5)}% | ` +
            `${profile.confidence_adjustment.toFixed(2).padStart(10)} | ` +
            `${trendIcon}`
          )
        }
      },
    },

    {
      name: 'show',
      description: 'Show detailed trust profile for agent',
      args: [
        { name: 'agent-id', required: true },
      ],
      async execute(context, args) {
        const profile = await context.edda.trust.get(args['agent-id'])

        console.log(`Agent Trust Profile: ${profile.agent_id}\n`)
        console.log(`Trust Score: ${profile.trust_score}/100`)
        console.log(`Approval Rate: ${(profile.approval_rate * 100).toFixed(1)}%`)
        console.log(`Confidence Adjustment: ${profile.confidence_adjustment > 0 ? '+' : ''}${profile.confidence_adjustment.toFixed(2)}`)
        console.log(`\nProposals:`)
        console.log(`  Submitted: ${profile.proposals_submitted}`)
        console.log(`  Approved:  ${profile.proposals_approved}`)
        console.log(`  Rejected:  ${profile.proposals_rejected}`)
        console.log(`\nPerformance Trend: ${profile.performance_trend || 'stable'}`)
        console.log(`Last Updated: ${profile.last_updated}`)
      },
    },

    {
      name: 'adjust',
      description: 'Manually adjust agent trust (admin only)',
      args: [
        { name: 'agent-id', required: true },
        { name: 'adjustment', required: true },
      ],
      async execute(context, args) {
        const { 'agent-id': agentId, adjustment } = args

        // Check authorization (only admin can manually adjust)
        await context.edda.authorization.authorize(
          context.currentPrincipal,
          Permission.TRUST_ADJUST,
        )

        const adjustmentValue = parseFloat(adjustment)
        if (adjustmentValue < -0.2 || adjustmentValue > 0.2) {
          throw new Error('Adjustment must be between -0.2 and +0.2')
        }

        const profile = await context.edda.trust.get(agentId)
        profile.confidence_adjustment = adjustmentValue
        profile.last_updated = new Date().toISOString()

        await context.edda.trust.update(profile)

        console.log(`✅ Adjusted trust for agent '${agentId}' to ${adjustmentValue > 0 ? '+' : ''}${adjustmentValue.toFixed(2)}`)
      },
    },
  ],
}
```

**Tests:**
- List trust profiles works
- Show individual profile
- Adjust trust requires permission
- Adjustment must be in valid range

---

#### Epic 6.3: Audit CLI
**Estimate:** 2 hours
**Owner:** TBD

**Description:**
Implement CLI commands for querying audit trail.

**Acceptance Criteria:**
- `anvil edda audit list` - List recent audit entries
- `anvil edda audit query --actor <principal>` - Query by actor
- `anvil edda audit query --action <action>` - Query by action
- `anvil edda audit query --resource <resource-id>` - Query by resource

**Implementation:**

```typescript
// packages/anvil/src/commands/edda/audit.ts

export const auditCommand: Command = {
  name: 'audit',
  description: 'Query Edda audit trail',
  subcommands: [
    {
      name: 'list',
      description: 'List recent audit entries',
      options: [
        { name: 'limit', default: 50 },
      ],
      async execute(context, args, options) {
        const entries = await context.edda.audit.query({
          limit: options.limit,
        })

        console.log(`Recent Audit Entries (${entries.length}):\n`)
        console.log('TIMESTAMP            | ACTOR           | ACTION              | RESOURCE          | OUTCOME')
        console.log('─'.repeat(100))

        for (const entry of entries) {
          const timestamp = new Date(entry.timestamp).toISOString().slice(0, 19).replace('T', ' ')
          const outcomeIcon = {
            success: '✅',
            failure: '❌',
            unauthorized: '🚫',
          }[entry.outcome]

          console.log(
            `${timestamp} | ` +
            `${entry.actor.padEnd(15).slice(0, 15)} | ` +
            `${entry.action.padEnd(19)} | ` +
            `${entry.resource_id.padEnd(17).slice(0, 17)} | ` +
            `${outcomeIcon} ${entry.outcome}`
          )
        }
      },
    },

    {
      name: 'query',
      description: 'Query audit trail with filters',
      options: [
        { name: 'actor', required: false },
        { name: 'action', required: false },
        { name: 'resource', required: false },
        { name: 'outcome', choices: ['success', 'failure', 'unauthorized'], required: false },
        { name: 'start-date', required: false },
        { name: 'end-date', required: false },
        { name: 'limit', default: 100 },
      ],
      async execute(context, args, options) {
        const filter: AuditFilter = {
          actor: options.actor,
          action: options.action,
          resource_id: options.resource,
          outcome: options.outcome,
          start_date: options['start-date'],
          end_date: options['end-date'],
          limit: options.limit,
        }

        const entries = await context.edda.audit.query(filter)

        console.log(`Audit Query Results (${entries.length}):\n`)

        // Display same as list...
      },
    },
  ],
}
```

**Tests:**
- List recent entries
- Query by actor
- Query by action
- Query by date range
- Query with limit

---

### Epic 7: Integration & Testing
**Duration:** 1 day
**Priority:** P0 (Blocking)

#### Epic 7.1: Integration Tests
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
End-to-end integration tests for authority & trust system.

**Acceptance Criteria:**
- Full promotion flow with authorization
- Trust score updates after approval/rejection
- Audit trail captures all events
- Unauthorized operations blocked
- 100% test coverage

**Test Scenarios:**

```typescript
// packages/edda-core/src/__tests__/integration/authority-trust.integration.test.ts

describe('Authority & Trust Integration', () => {
  it('should enforce authorization on memory operations', async () => {
    // Create principals
    const admin = await createPrincipal('admin', 'user', [AuthorityLevel.ORG_ADMIN])
    const readonly = await createPrincipal('readonly', 'user', [AuthorityLevel.READONLY])

    // Admin can create memory
    await edda.memories.create(admin, memoryData)

    // Readonly cannot create memory
    await expect(
      edda.memories.create(readonly, memoryData)
    ).rejects.toThrow(UnauthorizedError)
  })

  it('should track trust scores through promotion lifecycle', async () => {
    // Create agent with no history
    const agent = await createPrincipal('test-agent', 'agent', [AuthorityLevel.AGENT])
    const reviewer = await createPrincipal('reviewer', 'user', [AuthorityLevel.CONTRIBUTOR])

    // Initial trust: neutral (50)
    let trust = await edda.trust.get('test-agent')
    expect(trust.trust_score).toBe(50)
    expect(trust.confidence_adjustment).toBe(0)

    // Submit and approve 3 proposals
    for (let i = 0; i < 3; i++) {
      const request = await edda.promotion.createRequest(proposal, 'test-agent')
      await edda.promotion.approve(request.request_id, reviewer)
    }

    // Trust should improve
    trust = await edda.trust.get('test-agent')
    expect(trust.trust_score).toBeGreaterThan(50)
    expect(trust.confidence_adjustment).toBeGreaterThan(0)
    expect(trust.performance_trend).toBe('improving')

    // Reject 2 proposals
    for (let i = 0; i < 2; i++) {
      const request = await edda.promotion.createRequest(proposal, 'test-agent')
      await edda.promotion.reject(request.request_id, reviewer, 'Not good enough')
    }

    // Trust should decline
    trust = await edda.trust.get('test-agent')
    expect(trust.trust_score).toBeLessThan(70)
    expect(trust.performance_trend).toBe('declining')
  })

  it('should create comprehensive audit trail', async () => {
    const actor = await createPrincipal('actor', 'user', [AuthorityLevel.CONTRIBUTOR])

    // Perform various operations
    await edda.memories.create(actor, memoryData)
    await edda.promotion.approve(requestId, actor)
    await edda.roles.assignRole('agent-1', AuthorityLevel.AGENT)

    // Query audit trail
    const entries = await edda.audit.query({ actor: 'actor' })

    expect(entries).toHaveLength(3)
    expect(entries[0].action).toBe(AuditAction.MEMORY_CREATE)
    expect(entries[1].action).toBe(AuditAction.PROPOSAL_APPROVE)
    expect(entries[2].action).toBe(AuditAction.ROLE_ASSIGN)
    expect(entries.every(e => e.outcome === 'success')).toBe(true)
  })

  it('should prevent agents from approving own proposals', async () => {
    const agent = await createPrincipal('agent', 'agent', [
      AuthorityLevel.AGENT,
      AuthorityLevel.CONTRIBUTOR, // Has approval permission
    ])

    const request = await edda.promotion.createRequest(proposal, 'agent')

    // Cannot approve own proposal
    await expect(
      edda.promotion.approve(request.request_id, agent)
    ).rejects.toThrow(UnauthorizedError)
  })
})
```

**Tests:**
- Authorization enforcement works
- Trust scores update correctly
- Audit trail comprehensive
- Cannot approve own proposals
- Higher roles can override lower roles
- Performance: <10ms per authorization check

---

#### Epic 7.2: Performance & Load Testing
**Estimate:** 2 hours
**Owner:** TBD

**Description:**
Validate performance requirements for authority checks.

**Acceptance Criteria:**
- Authorization checks: <10ms
- Audit log write: <5ms
- Trust profile lookup: <5ms
- Can handle 1000 concurrent authorization checks

**Tests:**

```typescript
describe('Authority Performance', () => {
  it('should check authorization in <10ms', async () => {
    const iterations = 1000
    const start = performance.now()

    for (let i = 0; i < iterations; i++) {
      await authz.authorize(principal, Permission.MEMORY_READ)
    }

    const end = performance.now()
    const avgTime = (end - start) / iterations

    expect(avgTime).toBeLessThan(10)
  })

  it('should handle 1000 concurrent authorization checks', async () => {
    const promises = Array.from({ length: 1000 }, () =>
      authz.authorize(principal, Permission.MEMORY_READ)
    )

    await expect(Promise.all(promises)).resolves.not.toThrow()
  })
})
```

---

## Timeline

### Week 1 (Days 1-5)
- **Day 1-2:** Epic 1 (Principal & Role System)
- **Day 3-4:** Epic 2 (Permission System)
- **Day 5:** Epic 3 (Authority Metadata)

### Week 2 (Days 6-10)
- **Day 6-7:** Epic 4 (Agent Trust Profiles)
- **Day 8-9:** Epic 5 (Audit Trail)
- **Day 10:** Epic 6 (CLI Commands) + Epic 7 (Integration & Testing)

---

## Deliverables

### Package Structure
```
packages/edda-core/src/authority/
├── principal.ts
├── principal-repo.ts
├── roles.ts
├── permissions.ts
├── authorization-service.ts
├── metadata.ts
├── trust-profile.ts
├── trust-profile-repo.ts
├── trust-calculator.ts
├── audit-trail.ts
├── audit-storage.ts
├── audit-index.ts
└── __tests__/
    ├── principal-repo.test.ts
    ├── roles.test.ts
    ├── authorization-service.test.ts
    ├── trust-calculator.test.ts
    ├── audit-trail.test.ts
    └── integration/
        └── authority-trust.integration.test.ts

packages/anvil/src/commands/edda/
├── roles.ts
├── principals.ts
├── trust.ts
└── audit.ts
```

### Storage Structure
```
.edda/
├── principals/
│   ├── user-alice.yaml
│   ├── agent-gpt4.yaml
│   └── system-anvil.yaml
├── trust/
│   ├── agent-gpt4.yaml
│   └── agent-claude.yaml
├── audit/
│   ├── 2026-01-19.jsonl
│   ├── 2026-01-20.jsonl
│   └── index.db
```

### Documentation
- Authority & Trust Architecture (already exists: `docs/specs/edda-authority-trust.md`)
- API documentation for authority services
- CLI usage guide for role/trust management

### Tests
- Unit tests: 50+ tests
- Integration tests: 10+ scenarios
- Performance tests: <10ms authorization, <5ms audit
- Test coverage: 100%

---

## Success Metrics

### Functional
- ✅ All 5 authority levels working
- ✅ Permission checks enforce policies
- ✅ Trust scores adjust based on performance
- ✅ Audit trail captures all events
- ✅ CLI commands operational

### Performance
- ✅ Authorization checks: <10ms
- ✅ Audit log write: <5ms
- ✅ Trust profile lookup: <5ms
- ✅ 1000 concurrent checks supported

### Quality
- ✅ 100% test coverage
- ✅ All edge cases handled (own approvals, role hierarchy, etc.)
- ✅ Error messages clear and actionable
- ✅ Zero security vulnerabilities

---

## Risks & Mitigation

### Risk 1: Permission Model Too Complex
**Probability:** Medium
**Impact:** Medium

**Mitigation:**
- Start simple (5 roles, clear permission matrix)
- Can extend later if needed
- Document permission rationale

### Risk 2: Trust Scoring Unfair to New Agents
**Probability:** Medium
**Impact:** Low

**Mitigation:**
- New agents start at neutral (50) score
- Small confidence adjustment initially (±0.1)
- Can manually override trust scores (admin only)

### Risk 3: Audit Storage Grows Large
**Probability:** High
**Impact:** Low

**Mitigation:**
- JSONL files rotated daily
- Can archive old audit logs (>90 days)
- SQLite index keeps queries fast

---

## Dependencies

### Upstream (Must Complete First)
- Phase 0: Foundation (memory storage, Git layer)
- Phase 1: Promotion Pipeline (PromotionRequest schema)

### Downstream (Blocked By This Phase)
- Phase 4: Enforcement Hooks (requires authorization checks)
- Phase 6: Interop & Export (requires audit trail)

---

## Open Questions

### Q1: Identity Provider Integration (from OPEN-QUESTIONS.md)
**Status:** 🟡 Pending Stakeholder Decision
**Recommended:** GitHub OAuth for v1

**Impact on Phase 2:**
- GitHub OAuth: 2 weeks (as planned)
- OIDC: +1 week (3 weeks total)
- Both: +2 weeks (4 weeks total)

**Decision Required By:** Before Phase 2 starts

---

### Q2: Multi-Tenancy
**Status:** 🟡 Pending Stakeholder Decision
**Recommended:** Single-org for v1

**Impact on Phase 2:**
- Single-org: No impact (2 weeks)
- Multi-tenant: +1 week (add org_id to all schemas)

**Decision Required By:** Before Phase 2 starts

---

## Next Steps

1. ✅ Complete Phase 0 (Foundation)
2. ✅ Complete Phase 1 (Promotion Pipeline)
3. **Review this APS document** with team
4. **Resolve Open Questions** (identity provider, multi-tenancy)
5. **Assign owners** to epics and tasks
6. **Kick off Phase 2** implementation

---

**Document Version:** 1.0
**Last Updated:** 2026-01-19
**Status:** Ready for Review
**Estimated Completion:** 2 weeks after Phase 1 completion
