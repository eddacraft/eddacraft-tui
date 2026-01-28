# Edda Authority & Trust Model Specification

**Version:** 1.0.0 **Status:** Draft **Related:**
`/docs/architecture/edda-system-architecture.md` (Section 3)

---

## Overview

The Authority & Trust Model defines:

1. **Who** can perform operations on Edda memories
2. **How** trust is established and maintained
3. **Why** certain operations require higher authority
4. **When** trust scores are adjusted

**Core Principle:** Edda must be harder to write than to read. Authority creates
this asymmetry.

---

## 1. Principal System

### 1.1 Principal Types

```typescript
interface Principal {
  type: 'human' | 'agent' | 'team' | 'system';
  identifier: string;
}
```

**Examples:**

- `{ type: 'human', identifier: 'user:alice' }`
- `{ type: 'agent', identifier: 'agent:anvil' }`
- `{ type: 'team', identifier: 'team:platform' }`
- `{ type: 'system', identifier: 'system:edda' }`

### 1.2 Principal Resolution

**String Format:** `{type}:{identifier}`

```typescript
function resolvePrincipal(principalString: string): Principal {
  const [type, identifier] = principalString.split(':');

  if (!['human', 'agent', 'team', 'system'].includes(type)) {
    throw new Error(`Invalid principal type: ${type}`);
  }

  return { type: type as Principal['type'], identifier };
}
```

**Validation Rules:**

- Human: must exist in identity system (e.g., GitHub user)
- Agent: must be registered in agent registry
- Team: must exist in organisation structure
- System: reserved for Edda internal operations

### 1.3 Principal Storage

**Location:** `.edda/principals/`

```yaml
# .edda/principals/users.yaml
users:
  - identifier: user:alice
    name: Alice Smith
    email: alice@example.com
    roles:
      - org:admin
    created_at: '2025-01-15T10:00:00Z'

# .edda/principals/agents.yaml
agents:
  - identifier: agent:anvil
    name: Anvil AI Assistant
    version: '1.0.0'
    roles:
      - agent
    trust_profile_id: TP-anvil-001
    created_at: '2025-01-15T10:00:00Z'

# .edda/principals/teams.yaml
teams:
  - identifier: team:platform
    name: Platform Team
    members:
      - user:alice
      - user:bob
    leads:
      - user:alice
    created_at: '2025-01-15T10:00:00Z'
```

---

## 2. Authority Levels

### 2.1 Level Hierarchy

```
system          [Highest - Edda internal operations]
    ↓
org_admin       [Organisation-wide administration]
    ↓
team_lead       [Team/domain leadership]
    ↓
contributor     [Regular contributors]
    ↓
agent           [AI agents - propose only]
    ↓
readonly        [Lowest - read-only access]
```

### 2.2 Level Definitions

```typescript
type AuthorityLevel =
  | 'system' // Edda itself (automated processes)
  | 'org_admin' // Full administrative access
  | 'team_lead' // Team/domain admin (scoped)
  | 'contributor' // Regular developer
  | 'agent' // AI agent
  | 'readonly'; // Read-only access

interface AuthorityPolicy {
  level: AuthorityLevel;
  permissions: Permission[];
  constraints?: AuthorityConstraint[];
}
```

### 2.3 Default Authority Policies

```typescript
const DEFAULT_POLICIES: Record<AuthorityLevel, AuthorityPolicy> = {
  system: {
    level: 'system',
    permissions: [
      'read_all',
      'create_memory_direct',
      'update_memory',
      'retire_memory',
      'configure_enforcement',
      'manage_authority',
      'review_promotions',
    ],
    constraints: [],
  },

  org_admin: {
    level: 'org_admin',
    permissions: [
      'read_all',
      'review_promotions',
      'create_memory_direct',
      'update_memory',
      'retire_memory',
      'configure_enforcement',
      'manage_authority',
    ],
    constraints: [
      {
        type: 'approval_required',
        details: { for: ['retire_memory'], approvers: 2 },
      },
    ],
  },

  team_lead: {
    level: 'team_lead',
    permissions: [
      'read_all',
      'read_team',
      'review_promotions',
      'update_memory',
      'retire_memory',
      'configure_enforcement',
    ],
    constraints: [
      {
        type: 'scope_limited',
        details: { scope_type: 'team' },
      },
    ],
  },

  contributor: {
    level: 'contributor',
    permissions: ['read_public', 'read_team', 'propose_memory'],
    constraints: [],
  },

  agent: {
    level: 'agent',
    permissions: ['read_public', 'propose_memory'],
    constraints: [
      {
        type: 'quota_limited',
        details: { max_proposals_per_day: 50 },
      },
      {
        type: 'approval_required',
        details: { for: ['propose_memory'], confidence_threshold: 0.7 },
      },
    ],
  },

  readonly: {
    level: 'readonly',
    permissions: ['read_public'],
    constraints: [],
  },
};
```

---

## 3. Role-Based Access Control (RBAC)

### 3.1 Role Schema

```typescript
interface Role {
  role_id: string; // Unique identifier
  name: string; // Human-readable name
  authority_level: AuthorityLevel;
  permissions: Permission[];
  scope_restriction?: ScopeSpecifier;
  principals: Principal[];
}
```

### 3.2 Predefined Roles

```typescript
const PREDEFINED_ROLES: Role[] = [
  {
    role_id: 'org:admin',
    name: 'Organisation Administrator',
    authority_level: 'org_admin',
    permissions: [
      'read_all',
      'review_promotions',
      'create_memory_direct',
      'update_memory',
      'retire_memory',
      'configure_enforcement',
      'manage_authority',
    ],
    principals: [],
  },

  {
    role_id: 'team:lead',
    name: 'Team Lead',
    authority_level: 'team_lead',
    permissions: [
      'read_all',
      'read_team',
      'review_promotions',
      'update_memory',
      'retire_memory',
      'configure_enforcement',
    ],
    scope_restriction: {
      type: 'team',
      identifier: '{team_id}', // Replaced at runtime
    },
    principals: [],
  },

  {
    role_id: 'contributor',
    name: 'Contributor',
    authority_level: 'contributor',
    permissions: ['read_public', 'read_team', 'propose_memory'],
    principals: [],
  },

  {
    role_id: 'agent:trusted',
    name: 'Trusted AI Agent',
    authority_level: 'agent',
    permissions: ['read_public', 'read_team', 'propose_memory'],
    principals: [],
  },

  {
    role_id: 'agent:untrusted',
    name: 'Untrusted AI Agent',
    authority_level: 'agent',
    permissions: ['read_public', 'propose_memory'],
    principals: [],
  },
];
```

### 3.3 Custom Roles

Organisations can define custom roles:

```yaml
# .edda/roles/custom.yaml
custom_roles:
  - role_id: 'security:reviewer'
    name: 'Security Reviewer'
    authority_level: team_lead
    permissions:
      - read_all
      - review_promotions
      - configure_enforcement
    scope_restriction:
      type: domain
      identifier: security
    principals:
      - user:charlie
      - user:dana
    created_at: '2025-01-15T10:00:00Z'
```

### 3.4 Role Assignment

```typescript
interface RoleAssignment {
  assignment_id: string;
  role_id: string;
  principal: Principal;
  assigned_by: Principal;
  assigned_at: Timestamp;
  expires_at?: Timestamp;
  conditions?: AssignmentCondition[];
}

interface AssignmentCondition {
  type: 'time_limited' | 'scope_limited' | 'approval_required';
  details: Record<string, unknown>;
}
```

**Example:**

```yaml
# .edda/assignments/user-alice.yaml
assignments:
  - assignment_id: ASGN-001
    role_id: org:admin
    principal:
      type: human
      identifier: user:alice
    assigned_by:
      type: human
      identifier: user:founder
    assigned_at: '2025-01-15T10:00:00Z'
```

---

## 4. Permission System

### 4.1 Permission Types

```typescript
type Permission =
  // Read permissions
  | 'read_public' // Read public memories
  | 'read_team' // Read team-scoped memories
  | 'read_all' // Read all memories (including private)

  // Write permissions
  | 'propose_memory' // Create promotion requests
  | 'create_memory_direct' // Create memories without promotion
  | 'update_memory' // Modify existing memories
  | 'retire_memory' // Mark memories as retired

  // Governance permissions
  | 'review_promotions' // Approve/reject promotion requests
  | 'configure_enforcement' // Set enforcement policies
  | 'manage_authority'; // Grant/revoke roles and permissions
```

### 4.2 Permission Checks

```typescript
interface PermissionChecker {
  /**
   * Check if principal has permission
   */
  hasPermission(
    principal: Principal,
    permission: Permission,
    context?: PermissionContext
  ): boolean;

  /**
   * Check if principal can access memory
   */
  canAccessMemory(principal: Principal, memory: MemoryObjectExtended): boolean;

  /**
   * Get all permissions for principal
   */
  getPermissions(principal: Principal): Permission[];

  /**
   * Get authority level for principal
   */
  getAuthorityLevel(principal: Principal): AuthorityLevel;
}

interface PermissionContext {
  memory?: MemoryObjectExtended;
  scope?: ScopeSpecifier;
  operation?: string;
}
```

**Implementation:**

```typescript
class DefaultPermissionChecker implements PermissionChecker {
  hasPermission(
    principal: Principal,
    permission: Permission,
    context?: PermissionContext
  ): boolean {
    // 1. Get principal's roles
    const roles = this.getRolesForPrincipal(principal);

    // 2. Check if any role grants permission
    for (const role of roles) {
      if (role.permissions.includes(permission)) {
        // 3. Check scope constraints
        if (context?.scope && role.scope_restriction) {
          if (!this.scopeMatches(context.scope, role.scope_restriction)) {
            continue;
          }
        }

        // 4. Check other constraints
        if (this.checkConstraints(role, context)) {
          return true;
        }
      }
    }

    return false;
  }

  canAccessMemory(principal: Principal, memory: MemoryObjectExtended): boolean {
    // Public memories
    if (memory.authority.visibility === 'public') {
      return this.hasPermission(principal, 'read_public');
    }

    // Team memories
    if (memory.authority.visibility === 'team') {
      // Check if principal is in same team as memory owner
      if (this.isSameTeam(principal, memory.authority.owner)) {
        return this.hasPermission(principal, 'read_team');
      }
    }

    // Private memories
    if (memory.authority.visibility === 'private') {
      // Only owner, reviewers, or admins can access
      if (this.isPrincipalMatch(principal, memory.authority.owner)) {
        return true;
      }
      if (
        memory.authority.reviewers.some((r) =>
          this.isPrincipalMatch(principal, r)
        )
      ) {
        return true;
      }
      return this.hasPermission(principal, 'read_all');
    }

    return false;
  }

  private checkConstraints(role: Role, context?: PermissionContext): boolean {
    const policy = this.getPolicyForLevel(role.authority_level);

    for (const constraint of policy.constraints || []) {
      switch (constraint.type) {
        case 'scope_limited':
          if (context?.scope) {
            if (!this.scopeMatches(context.scope, role.scope_restriction)) {
              return false;
            }
          }
          break;

        case 'approval_required':
          // Check if operation requires approval
          if (
            context?.operation &&
            constraint.details.for?.includes(context.operation)
          ) {
            // Would need to check approval status
            return false; // Simplified for spec
          }
          break;

        case 'quota_limited':
          // Check if quota exceeded
          if (this.isQuotaExceeded(role, constraint.details)) {
            return false;
          }
          break;
      }
    }

    return true;
  }
}
```

### 4.3 Permission Middleware

For API endpoints:

```typescript
function requirePermission(permission: Permission) {
  return (req: Request, res: Response, next: NextFunction) => {
    const principal = req.user; // Extracted from auth token

    if (!permissionChecker.hasPermission(principal, permission)) {
      return res.status(403).json({
        error: 'Forbidden',
        message: `Principal ${principal.identifier} lacks permission: ${permission}`,
      });
    }

    next();
  };
}

// Usage
app.post(
  '/memories',
  authenticate,
  requirePermission('create_memory_direct'),
  createMemoryHandler
);
```

---

## 5. Agent Trust System

### 5.1 Trust Profile

```typescript
interface AgentTrustProfile {
  profile_id: string; // TP-{agent-id}-{version}
  agent_id: string;
  trust_score: number; // 0.0 (no trust) - 1.0 (full trust)

  // Historical performance
  proposals_submitted: number;
  proposals_approved: number;
  proposals_rejected: number;
  approval_rate: number; // Auto-calculated

  // Trust factors
  factors: TrustFactor[];

  // Permissions
  can_propose: boolean;
  confidence_adjustment: number; // -0.2 to +0.2
  requires_human_review: boolean;

  // Metadata
  created_at: Timestamp;
  last_updated: Timestamp;
}

interface TrustFactor {
  factor:
    | 'historical_accuracy'
    | 'source_quality'
    | 'reasoning_quality'
    | 'domain_expertise';
  weight: number; // 0.0 - 1.0
  current_value: number; // 0.0 - 1.0
  rationale: string;
}
```

### 5.2 Trust Score Calculation

```typescript
function calculateTrustScore(profile: AgentTrustProfile): number {
  // Base score from approval rate
  const baseScore = profile.approval_rate;

  // Weighted factor score
  const factorScore =
    profile.factors.reduce((sum, factor) => {
      return sum + factor.current_value * factor.weight;
    }, 0) / profile.factors.reduce((sum, f) => sum + f.weight, 0);

  // Combine (60% base, 40% factors)
  const combinedScore = baseScore * 0.6 + factorScore * 0.4;

  // Apply penalties
  let finalScore = combinedScore;

  // Penalty for low proposal volume (need data to trust)
  if (profile.proposals_submitted < 10) {
    finalScore *= 0.5;
  }

  // Penalty for recent rejections (sliding window)
  const recentRejectionRate = getRecentRejectionRate(profile.agent_id, 30); // Last 30 days
  if (recentRejectionRate > 0.5) {
    finalScore *= 0.8;
  }

  return Math.max(0, Math.min(1, finalScore));
}
```

### 5.3 Trust Score Updates

**When to update:**

1. After each promotion review (approved/rejected)
2. When human feedback is received
3. When contradiction is detected in agent-proposed memory
4. Periodic recalculation (weekly)

```typescript
function updateTrustProfile(
  agentId: string,
  event: TrustEvent
): AgentTrustProfile {
  const profile = getTrustProfile(agentId);

  switch (event.type) {
    case 'promotion_approved':
      profile.proposals_approved++;
      profile.factors.find(
        (f) => f.factor === 'historical_accuracy'
      )!.current_value += 0.05;
      break;

    case 'promotion_rejected':
      profile.proposals_rejected++;
      profile.factors.find(
        (f) => f.factor === 'historical_accuracy'
      )!.current_value -= 0.1;

      // Specific penalties based on rejection category
      if (event.details.category === 'insufficient_evidence') {
        profile.factors.find(
          (f) => f.factor === 'reasoning_quality'
        )!.current_value -= 0.05;
      }
      break;

    case 'human_feedback':
      const feedbackScore = event.details.score; // 0-1
      profile.factors.find(
        (f) => f.factor === 'reasoning_quality'
      )!.current_value =
        (profile.factors.find((f) => f.factor === 'reasoning_quality')!
          .current_value +
          feedbackScore) /
        2;
      break;

    case 'contradiction_detected':
      profile.factors.find(
        (f) => f.factor === 'historical_accuracy'
      )!.current_value -= 0.15;
      break;
  }

  // Recalculate overall score
  profile.trust_score = calculateTrustScore(profile);
  profile.approval_rate =
    profile.proposals_approved /
    (profile.proposals_approved + profile.proposals_rejected);

  // Update permissions based on score
  profile.can_propose = profile.trust_score > 0.3;
  profile.requires_human_review = profile.trust_score < 0.7;
  profile.confidence_adjustment = (profile.trust_score - 0.5) * 0.4; // -0.2 to +0.2

  profile.last_updated = new Date().toISOString();

  saveTrustProfile(profile);
  createAuditEntry({
    operation: 'trust_profile_updated',
    target_id: profile.profile_id,
    changes: event,
  });

  return profile;
}
```

### 5.4 Trust-Based Confidence Adjustment

When agents submit proposals, their confidence is adjusted based on trust:

```typescript
function adjustProposalConfidence(
  proposal: AgentProposal,
  trustProfile: AgentTrustProfile
): number {
  const originalConfidence = proposal.agent_confidence;
  const adjustment = trustProfile.confidence_adjustment;

  const adjustedConfidence = Math.max(
    0,
    Math.min(1, originalConfidence + adjustment)
  );

  return adjustedConfidence;
}
```

**Example:**

- Agent proposes with confidence 0.8
- Trust profile has adjustment -0.1 (due to recent rejections)
- Final confidence: 0.7 (used for promotion threshold checks)

---

## 6. Audit Trail

### 6.1 Audit Entry Schema

```typescript
interface AuditEntry {
  audit_id: AuditId; // EDDA-AUDIT-{ulid}
  timestamp: Timestamp;

  // Who
  principal: Principal;
  authority_level: AuthorityLevel;

  // What
  operation: AuditOperation;
  target_type: 'memory' | 'promotion' | 'authority' | 'config';
  target_id: string;

  // Details
  changes?: Record<string, unknown>;
  rationale?: string;

  // Context
  session_id?: string;
  ip_address?: string;
  user_agent?: string;
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
  | 'memory_queried' // Optional for sensitive memories
  | 'trust_profile_updated';
```

### 6.2 Audit Logging

**Automatic Logging:** All write operations are automatically audited.

```typescript
class AuditedEddaPort implements IEddaPortExtended {
  private auditLogger: AuditLogger;
  private innerPort: IEddaPortExtended;

  async createMemory(
    input: MemoryObjectInput,
    principal: Principal
  ): Promise<MemoryObjectExtended> {
    const memory = await this.innerPort.createMemory(input, principal);

    await this.auditLogger.log({
      audit_id: generateAuditId(),
      timestamp: new Date().toISOString(),
      principal,
      authority_level: getAuthorityLevel(principal),
      operation: 'memory_created',
      target_type: 'memory',
      target_id: memory.id,
      changes: { created: input },
      rationale: input.attribution?.reason,
    });

    return memory;
  }

  // Similar for other write operations...
}
```

### 6.3 Audit Storage

**Location:** `.edda/audit/`

**Format:** JSONL (JSON Lines) for efficient append

```jsonl
{"audit_id":"EDDA-AUDIT-01h...","timestamp":"2025-01-15T10:30:00Z","principal":{"type":"human","identifier":"user:alice"},"authority_level":"org_admin","operation":"memory_created","target_type":"memory","target_id":"EDDA-M-decision-02h...","changes":{"created":{...}},"rationale":"Post-incident decision"}
{"audit_id":"EDDA-AUDIT-01h...","timestamp":"2025-01-15T11:00:00Z","principal":{"type":"agent","identifier":"agent:anvil"},"authority_level":"agent","operation":"promotion_requested","target_type":"promotion","target_id":"EDDA-PR-03h...","rationale":"High confidence pattern detected"}
```

### 6.4 Audit Queries

```typescript
interface AuditQuery {
  // Filters
  principal?: Principal;
  operation?: AuditOperation;
  target_type?: string;
  target_id?: string;

  // Time range
  from?: Timestamp;
  to?: Timestamp;

  // Pagination
  limit?: number;
  offset?: number;
}

interface AuditQueryResult {
  entries: AuditEntry[];
  total_count: number;
  page_info: PageInfo;
}
```

### 6.5 Audit Retention

**Default Policy:**

- Keep all audit logs for 2 years
- Compress logs older than 90 days
- Archive logs older than 1 year (off-disk)

**Compliance:**

- Audit logs are immutable (append-only)
- Audit log modifications are themselves audited
- Support export for compliance reporting

---

## 7. Security Considerations

### 7.1 Authentication

Edda relies on external authentication:

- GitHub OAuth (for GitHub integration)
- LDAP/Active Directory (for enterprise)
- API keys (for programmatic access)

**Token Format:**

```
Authorization: Bearer <jwt-token>
```

**JWT Claims:**

```json
{
  "sub": "user:alice",
  "type": "human",
  "roles": ["org:admin"],
  "exp": 1705329600
}
```

### 7.2 Authorization

All operations check permissions before execution:

```typescript
function ensureAuthorized(
  principal: Principal,
  permission: Permission,
  context?: PermissionContext
): void {
  if (!permissionChecker.hasPermission(principal, permission, context)) {
    throw new UnauthorizedError(
      `Principal ${principal.identifier} lacks permission: ${permission}`
    );
  }
}
```

### 7.3 Sensitive Data

**Memory Visibility:**

- `public`: Anyone with `read_public`
- `team`: Team members with `read_team`
- `private`: Owner, reviewers, or `read_all` permission

**Audit Log Access:**

- Only org_admin and system can query audit logs
- Filtered by scope for team_lead

### 7.4 Rate Limiting

**Per-agent quotas:**

```typescript
interface RateLimit {
  agent_id: string;
  proposals_per_day: number;
  proposals_per_hour: number;
  current_count_day: number;
  current_count_hour: number;
  reset_at_day: Timestamp;
  reset_at_hour: Timestamp;
}
```

**Enforcement:**

```typescript
function checkRateLimit(agentId: string): void {
  const limit = getRateLimit(agentId);

  if (limit.current_count_hour >= limit.proposals_per_hour) {
    throw new RateLimitError('Hourly proposal limit exceeded');
  }

  if (limit.current_count_day >= limit.proposals_per_day) {
    throw new RateLimitError('Daily proposal limit exceeded');
  }

  // Increment counters
  incrementRateLimit(agentId);
}
```

---

## 8. Implementation Checklist

### Phase 2.1: Principal & Role System (2 days)

- [ ] Define `Principal` model
- [ ] Implement principal storage (YAML)
- [ ] Implement principal resolution
- [ ] Define default roles
- [ ] Role storage and management
- [ ] Unit tests

### Phase 2.2: Permission System (2 days)

- [ ] Define `Permission` enum
- [ ] Implement `PermissionChecker`
- [ ] Scope-based access control
- [ ] Permission middleware
- [ ] Unit tests

### Phase 2.3: Authority Metadata (1 day)

- [ ] Add authority fields to `MemoryObjectExtended`
- [ ] Owner and reviewer assignment
- [ ] Visibility enforcement in queries
- [ ] Integration tests

### Phase 2.4: Agent Trust Profiles (2 days)

- [ ] Define `AgentTrustProfile` schema
- [ ] Implement trust score calculation
- [ ] Trust score update logic
- [ ] Confidence adjustment
- [ ] Unit tests

### Phase 2.5: Audit Trail (2 days)

- [ ] Define `AuditEntry` schema
- [ ] Implement audit logger
- [ ] Audited port wrapper
- [ ] Audit query interface
- [ ] Storage (JSONL)
- [ ] Unit tests

### Phase 2.6: CLI Commands (1 day)

- [ ] `anvil edda roles list`
- [ ] `anvil edda roles assign <principal> <role>`
- [ ] `anvil edda audit [--principal=...] [--operation=...]`
- [ ] `anvil edda trust <agent-id>`
- [ ] E2E tests

---

## 9. Testing Strategy

### Unit Tests

- Principal resolution
- Permission checking (all combinations)
- Trust score calculation
- Role assignment
- Audit logging

### Integration Tests

- Full RBAC workflow
- Agent trust updates after promotion
- Audit trail for operations
- Scope-based access control

### Security Tests

- Unauthorised access attempts
- Permission escalation attempts
- Rate limit enforcement
- Audit log immutability

---

## 10. Success Criteria

- [ ] All operations check permissions before execution
- [ ] Agents cannot bypass human review
- [ ] Trust scores reflect proposal quality
- [ ] Audit trail captures all write operations
- [ ] Scope-based access control works correctly
- [ ] Rate limits prevent abuse
- [ ] CLI commands are intuitive

---

**Next:** See `/plans/edda-phase-breakdown.md` Phase 2 for detailed task
breakdown
