# Edda Enforcement & Guidance Hooks Specification

**Version:** 1.0.0 **Status:** Draft **Related:**
`/docs/architecture/edda-system-architecture.md` (Section 5)

---

## Overview

Enforcement & Guidance Hooks make Edda **actionable** rather than archival.
They:

1. **Block** actions that violate established constraints
2. **Warn** about potential issues before they occur
3. **Suggest** alternatives based on patterns and lessons
4. **Guide** planning with contextual knowledge

**Core Principle:** Edda intervenes proactively, not reactively.

---

## 1. Hook Architecture

### 1.1 Hook Execution Flow

```
┌─────────────────────────────────────────────────────────┐
│                  Anvil Execution                         │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  Action Initiated                                        │
│       ↓                                                  │
│  ┌────▼─────────┐                                       │
│  │ Pre-Check    │ ←──── Edda Enforcement Hooks          │
│  │ (BLOCKING)   │                                        │
│  └────┬─────────┘                                       │
│       │                                                  │
│  ┌────▼─────────┐                                       │
│  │ Violations?  │                                        │
│  └────┬─────────┘                                       │
│       │                                                  │
│   ┌───┴───┐                                             │
│   │ Yes   │ No                                          │
│   ↓       ↓                                             │
│ Block   Allow                                           │
│ (with   (with warnings/suggestions)                     │
│ reason)                                                 │
│           ↓                                             │
│      Execute Action                                     │
│           ↓                                             │
│  ┌────▼─────────┐                                       │
│  │ Post-Check   │ ←──── Learn Signal (optional)         │
│  │ (AUDIT)      │                                        │
│  └──────────────┘                                       │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### 1.2 Hook Types

```typescript
type EnforcementHookType =
  | 'pre_execution' // Before action runs (can block)
  | 'validation' // During planning validation
  | 'guidance' // Soft suggestions (non-blocking)
  | 'post_execution' // After action completes (audit/learn)
  | 'approval_required'; // Human-in-loop gate
```

**Type Characteristics:**

| Type              | Blocking | Timing                 | Purpose                      |
| ----------------- | -------- | ---------------------- | ---------------------------- |
| pre_execution     | Yes      | Just before action     | Enforce constraints/policies |
| validation        | Yes      | During plan validation | Catch issues early           |
| guidance          | No       | Planning/execution     | Surface relevant knowledge   |
| post_execution    | No       | After action           | Learn from outcomes          |
| approval_required | Yes      | Before action          | Human oversight              |

---

## 2. Hook Definition Schema

### 2.1 Core Schema

```typescript
interface EnforcementHook {
  hook_id: HookId; // EDDA-HOOK-{ulid}
  type: EnforcementHookType;
  name: string; // Human-readable name
  description: string; // What this hook does

  // Trigger
  trigger: HookTrigger;

  // Applicable memories
  applicable_memories: MemoryMatcher;

  // Action to take
  action: HookAction;

  // Configuration
  enabled: boolean;
  priority: number; // Execution order (0-1000)

  // Metadata
  created_by: Principal;
  created_at: Timestamp;
  updated_at: Timestamp;
}
```

### 2.2 Hook Trigger

```typescript
interface HookTrigger {
  event: HookEvent;
  conditions?: TriggerCondition[];
}

type HookEvent =
  | 'plan_created'
  | 'action_about_to_execute'
  | 'file_about_to_change'
  | 'command_about_to_run'
  | 'gate_evaluated'
  | 'human_approval_requested';

interface TriggerCondition {
  field: string; // e.g., 'action.type', 'file.path'
  operator: '==' | '!=' | 'contains' | 'matches' | 'in';
  value: unknown;
}
```

**Example Triggers:**

```typescript
// Trigger on any file write
{
  event: 'file_about_to_change',
  conditions: []
}

// Trigger only on source file changes
{
  event: 'file_about_to_change',
  conditions: [
    { field: 'file.path', operator: 'matches', value: '^src/.*\\.(ts|js)$' }
  ]
}

// Trigger on database migrations
{
  event: 'action_about_to_execute',
  conditions: [
    { field: 'action.type', operator: '==', value: 'shell_command' },
    { field: 'action.command', operator: 'contains', value: 'migrate' }
  ]
}

// Trigger on specific gate
{
  event: 'gate_evaluated',
  conditions: [
    { field: 'gate.name', operator: '==', value: 'security_review' }
  ]
}
```

### 2.3 Memory Matcher

```typescript
interface MemoryMatcher {
  types?: MemoryType[]; // Filter by memory type
  tags?: string[]; // Filter by tags
  scope?: ScopeSpecifier; // Filter by scope
  enforcement_modes?: EnforcementMode[]; // Filter by mode
  confidence_min?: EddaConfidenceLevel; // Min confidence
}

type EnforcementMode = 'advisory' | 'warning' | 'blocking' | 'audit_only';
```

**Example Matchers:**

```typescript
// All blocking warnings
{
  types: ['warning'],
  enforcement_modes: ['blocking']
}

// Security-related decisions
{
  types: ['decision', 'doctrine'],
  tags: ['security']
}

// High-confidence constraints
{
  types: ['constraint'],
  confidence_min: 'high'
}

// Team-specific patterns
{
  types: ['pattern'],
  scope: { type: 'team', identifier: 'platform' }
}
```

### 2.4 Hook Action

```typescript
interface HookAction {
  mode: 'block' | 'warn' | 'suggest' | 'log' | 'require_approval';
  message_template: string; // Can reference {memory.statement}, etc.
  alternatives?: string[]; // Suggested alternatives
  approval_required_from?: AuthorityLevel[];
}
```

**Message Templates:**

Use `{variable}` syntax to inject context:

```typescript
// Variables available:
// - {memory.id}, {memory.statement}, {memory.type}
// - {action.type}, {action.details}
// - {file.path}
// - {principal.identifier}

'Blocked: {memory.statement}\n\nThis is a {memory.type} with {memory.confidence} confidence.\n\nRationale: {memory.context.why}';

'Warning: Detected pattern violation\nMemory: {memory.id}\n\nConsider: {alternatives}';
```

---

## 3. Hook Examples

### 3.1 Block Database Schema Changes Without Migration

```yaml
hook_id: EDDA-HOOK-db-migration
type: pre_execution
name: 'Require Migration for Schema Changes'
description: 'Block direct database schema changes without proper migration'

trigger:
  event: action_about_to_execute
  conditions:
    - field: action.type
      operator: ==
      value: shell_command
    - field: action.command
      operator: matches
      value: 'ALTER TABLE|DROP TABLE|CREATE TABLE'

applicable_memories:
  types:
    - constraint
  tags:
    - database
    - schema
  enforcement_modes:
    - blocking

action:
  mode: block
  message_template: |
    Blocked: Direct database schema modification

    Memory: {memory.statement}

    Schema changes must go through migration system:
    1. Create migration: npm run migration:create
    2. Review migration SQL
    3. Test in staging
    4. Deploy via CI/CD

    Rationale: {memory.context.why}

enabled: true
priority: 100
```

### 3.2 Warn About Deprecated Patterns

```yaml
hook_id: EDDA-HOOK-deprecated-pattern
type: pre_execution
name: 'Warn on Deprecated Patterns'
description: 'Surface warnings when deprecated patterns are detected'

trigger:
  event: file_about_to_change
  conditions:
    - field: file.path
      operator: matches
      value: "^src/.*\\.(ts|tsx)$"

applicable_memories:
  types:
    - pattern
    - warning
  tags:
    - deprecated
  enforcement_modes:
    - warning

action:
  mode: warn
  message_template: |
    ⚠️  Warning: Deprecated pattern detected

    Memory: {memory.statement}

    This pattern is deprecated. Consider:
    {alternatives}

    See: {memory.id}
  alternatives:
    - 'Use the new async/await pattern'
    - 'See examples in src/examples/modern-async.ts'

enabled: true
priority: 50
```

### 3.3 Suggest Best Practices During Planning

```yaml
hook_id: EDDA-HOOK-guidance-auth
type: guidance
name: 'Authentication Best Practices'
description: 'Surface auth-related guidance during planning'

trigger:
  event: plan_created
  conditions:
    - field: plan.intent
      operator: contains
      value: 'auth|login|session|token'

applicable_memories:
  types:
    - pattern
    - doctrine
    - lesson
  tags:
    - authentication
    - security
  confidence_min: medium

action:
  mode: suggest
  message_template: |
    💡 Guidance: Authentication implementation

    Relevant memories:
    {memory.statement}

    When to apply: {memory.context.when}

    See full context: anvil edda show {memory.id}

enabled: true
priority: 10
```

### 3.4 Require Approval for Production Deployments

```yaml
hook_id: EDDA-HOOK-prod-approval
type: approval_required
name: 'Production Deployment Approval'
description: 'Require team lead approval for production deployments'

trigger:
  event: action_about_to_execute
  conditions:
    - field: action.type
      operator: ==
      value: deployment
    - field: action.environment
      operator: ==
      value: production

applicable_memories:
  types:
    - doctrine
  tags:
    - deployment
    - production
  enforcement_modes:
    - blocking

action:
  mode: require_approval
  message_template: |
    Production deployment requires approval

    Policy: {memory.statement}

    Requesting approval from: team_lead
  approval_required_from:
    - team_lead
    - org_admin

enabled: true
priority: 200
```

### 3.5 Log Lessons Learned Post-Incident

```yaml
hook_id: EDDA-HOOK-incident-learn
type: post_execution
name: 'Capture Incident Lessons'
description: 'Prompt to create lesson memory after incident resolution'

trigger:
  event: action_completed
  conditions:
    - field: action.type
      operator: ==
      value: incident_resolution
    - field: action.severity
      operator: in
      value: [high, critical]

applicable_memories:
  types:
    - lesson
  tags:
    - incident

action:
  mode: suggest
  message_template: |
    📝 Suggestion: Document this incident

    Incident resolved: {action.incident_id}

    Consider creating a lesson memory:
    anvil edda create-lesson --incident={action.incident_id}

    This will help prevent similar incidents.

enabled: true
priority: 5
```

---

## 4. Hook Execution Engine

### 4.1 Execution Flow

```typescript
class HookExecutionEngine {
  /**
   * Execute hooks for an event
   */
  async executeHooks(
    event: HookEvent,
    context: ExecutionContext
  ): Promise<HookExecutionResult> {
    // 1. Find applicable hooks
    const hooks = await this.findApplicableHooks(event, context);

    // 2. Sort by priority (descending)
    hooks.sort((a, b) => b.priority - a.priority);

    // 3. Execute each hook
    const results: HookResult[] = [];
    for (const hook of hooks) {
      const result = await this.executeHook(hook, context);
      results.push(result);

      // Stop if blocking violation
      if (result.action === 'block') {
        return {
          allowed: false,
          violations: results.filter((r) => r.action === 'block'),
          warnings: results.filter((r) => r.action === 'warn'),
          suggestions: results.filter((r) => r.action === 'suggest'),
        };
      }
    }

    return {
      allowed: true,
      violations: [],
      warnings: results.filter((r) => r.action === 'warn'),
      suggestions: results.filter((r) => r.action === 'suggest'),
    };
  }

  private async findApplicableHooks(
    event: HookEvent,
    context: ExecutionContext
  ): Promise<EnforcementHook[]> {
    // Query hooks by event
    const hooks = await this.hookRegistry.getByEvent(event);

    // Filter by enabled
    return hooks.filter((hook) => {
      if (!hook.enabled) return false;

      // Check trigger conditions
      if (!this.checkTriggerConditions(hook.trigger, context)) {
        return false;
      }

      return true;
    });
  }

  private async executeHook(
    hook: EnforcementHook,
    context: ExecutionContext
  ): Promise<HookResult> {
    // 1. Find matching memories
    const memories = await this.findMatchingMemories(
      hook.applicable_memories,
      context
    );

    if (memories.length === 0) {
      return { action: 'allow' };
    }

    // 2. For each memory, evaluate enforcement
    for (const memory of memories) {
      const enforcement = this.evaluateEnforcement(memory, hook, context);

      if (enforcement.action === 'block') {
        return {
          action: 'block',
          memory,
          message: this.renderMessage(
            hook.action.message_template,
            memory,
            context
          ),
        };
      } else if (enforcement.action === 'warn') {
        return {
          action: 'warn',
          memory,
          message: this.renderMessage(
            hook.action.message_template,
            memory,
            context
          ),
        };
      } else if (enforcement.action === 'suggest') {
        return {
          action: 'suggest',
          memory,
          message: this.renderMessage(
            hook.action.message_template,
            memory,
            context
          ),
          alternatives: hook.action.alternatives,
        };
      }
    }

    return { action: 'allow' };
  }

  private evaluateEnforcement(
    memory: MemoryObjectExtended,
    hook: EnforcementHook,
    context: ExecutionContext
  ): EnforcementDecision {
    const mode = memory.enforcement.mode;

    // Check if principal can override
    const canOverride =
      context.principal &&
      this.canOverride(context.principal, memory.enforcement.override_requires);

    switch (hook.action.mode) {
      case 'block':
        return {
          action: canOverride ? 'warn' : 'block',
          can_override: canOverride,
        };

      case 'warn':
        return { action: 'warn' };

      case 'suggest':
        return { action: 'suggest' };

      case 'require_approval':
        return {
          action: this.hasApproval(context) ? 'allow' : 'block',
          approval_required_from: hook.action.approval_required_from,
        };

      default:
        return { action: 'allow' };
    }
  }

  private renderMessage(
    template: string,
    memory: MemoryObjectExtended,
    context: ExecutionContext
  ): string {
    return template
      .replace(/{memory\.id}/g, memory.id)
      .replace(/{memory\.statement}/g, memory.statement)
      .replace(/{memory\.type}/g, memory.type)
      .replace(/{memory\.confidence}/g, memory.confidence)
      .replace(/{memory\.context\.why}/g, memory.context.why)
      .replace(/{memory\.context\.when}/g, memory.context.when)
      .replace(/{action\.type}/g, context.action?.type || '')
      .replace(/{file\.path}/g, context.file?.path || '')
      .replace(/{principal\.identifier}/g, context.principal?.identifier || '');
    // Add more as needed
  }
}
```

### 4.2 Execution Context

```typescript
interface ExecutionContext {
  // Who
  principal?: Principal;

  // What
  action?: ActionContext;
  file?: FileContext;
  plan?: PlanContext;

  // Where
  scope?: ScopeSpecifier;

  // Session
  session_id?: string;
}

interface ActionContext {
  action_type: string;
  action_details: Record<string, unknown>;
  command?: string;
  environment?: string;
}

interface FileContext {
  path: string;
  operation: 'read' | 'write' | 'delete';
  content?: string;
}

interface PlanContext {
  plan_id: string;
  intent: string;
  technologies?: string[];
  phase?: string;
}
```

### 4.3 Execution Result

```typescript
interface HookExecutionResult {
  allowed: boolean;
  violations: HookResult[];
  warnings: HookResult[];
  suggestions: HookResult[];
}

interface HookResult {
  action: 'block' | 'warn' | 'suggest' | 'allow';
  memory?: MemoryObjectExtended;
  message?: string;
  alternatives?: string[];
  can_override?: boolean;
  approval_required_from?: AuthorityLevel[];
}
```

---

## 5. Anvil Integration Points

### 5.1 Gate System Integration

Anvil's gate system is the primary integration point.

```typescript
// In Anvil gate runner
async function evaluateGate(
  gate: Gate,
  context: GateContext
): Promise<GateResult> {
  // ... existing gate logic ...

  // Add Edda hook check
  const eddaCheck = await eddaHooks.executeHooks('gate_evaluated', {
    principal: context.principal,
    action: {
      action_type: 'gate_evaluation',
      action_details: { gate_name: gate.name },
    },
    scope: context.scope,
  });

  if (!eddaCheck.allowed) {
    return {
      passed: false,
      reason: 'Edda enforcement violation',
      violations: eddaCheck.violations,
      can_override: eddaCheck.violations.some((v) => v.can_override),
    };
  }

  // Return warnings/suggestions even if allowed
  return {
    passed: true,
    warnings: eddaCheck.warnings,
    suggestions: eddaCheck.suggestions,
  };
}
```

### 5.2 Pre-Action Hook

Before any tool execution:

```typescript
// In Anvil action executor
async function executeAction(
  action: Action,
  context: ActionContext
): Promise<ActionResult> {
  // Edda pre-execution check
  const eddaCheck = await eddaHooks.executeHooks('action_about_to_execute', {
    principal: context.principal,
    action: {
      action_type: action.type,
      action_details: action.parameters,
    },
    scope: context.scope,
    session_id: context.session_id,
  });

  if (!eddaCheck.allowed) {
    // Surface violations to user
    throw new ActionBlockedError({
      reason: 'Edda policy violation',
      violations: eddaCheck.violations,
      override_instructions: eddaCheck.violations
        .filter((v) => v.can_override)
        .map((v) => 'Use --force with justification'),
    });
  }

  // Log warnings/suggestions
  if (eddaCheck.warnings.length > 0) {
    logWarnings(eddaCheck.warnings);
  }

  if (eddaCheck.suggestions.length > 0) {
    logSuggestions(eddaCheck.suggestions);
  }

  // Proceed with action
  return await action.execute();
}
```

### 5.3 File Change Hook

Before writing files:

```typescript
// In Anvil file writer
async function writeFile(
  path: string,
  content: string,
  context: WriteContext
): Promise<void> {
  // Edda file change check
  const eddaCheck = await eddaHooks.executeHooks('file_about_to_change', {
    principal: context.principal,
    file: {
      path,
      operation: 'write',
      content,
    },
    scope: context.scope,
  });

  if (!eddaCheck.allowed) {
    throw new FileWriteBlockedError({
      path,
      violations: eddaCheck.violations,
    });
  }

  // Show warnings
  if (eddaCheck.warnings.length > 0) {
    console.warn(`Warnings for ${path}:`);
    eddaCheck.warnings.forEach((w) => console.warn(`  - ${w.message}`));
  }

  // Proceed with write
  await fs.writeFile(path, content);
}
```

### 5.4 Planning Guidance

During plan creation:

```typescript
// In Anvil planner
async function createPlan(
  intent: string,
  context: PlanningContext
): Promise<Plan> {
  // Get guidance from Edda
  const eddaGuidance = await eddaHooks.executeHooks('plan_created', {
    principal: context.principal,
    plan: {
      plan_id: generatePlanId(),
      intent,
      technologies: context.technologies,
    },
    scope: context.scope,
  });

  // Surface suggestions to planner (AI or human)
  if (eddaGuidance.suggestions.length > 0) {
    console.log('\n💡 Relevant guidance from Edda:');
    eddaGuidance.suggestions.forEach((s) => {
      console.log(`\n  ${s.message}`);
      if (s.alternatives) {
        console.log('  Alternatives:');
        s.alternatives.forEach((alt) => console.log(`    - ${alt}`));
      }
    });
  }

  // Proceed with planning
  return await generatePlan(intent, context);
}
```

---

## 6. Override Mechanism

### 6.1 Override Request

```typescript
interface OverrideRequest {
  violation_id: string;
  memory_id: MemoryId;
  requester: Principal;
  justification: string; // Required

  // Context
  action: ActionContext;
  original_check: HookExecutionResult;
}

interface OverrideDecision {
  approved: boolean;
  decided_by: Principal;
  decision_rationale: string;
  audit_entry_id: AuditId;

  // Follow-up
  requires_review?: boolean;
  review_deadline?: Timestamp;
}
```

### 6.2 Override Flow

```typescript
async function requestOverride(
  request: OverrideRequest
): Promise<OverrideDecision> {
  // 1. Check if override is allowed
  const memory = await edda.getMemory(request.memory_id)

  if (!memory.enforcement.override_requires) {
    return {
      approved: false,
      decided_by: { type: 'system', identifier: 'edda' },
      decision_rationale: 'This memory does not allow overrides',
      audit_entry_id: await auditLogger.log({ ... })
    }
  }

  // 2. Check if requester has authority
  const hasAuthority = memory.enforcement.override_requires.some(level =>
    authorityChecker.hasLevel(request.requester, level)
  )

  if (!hasAuthority) {
    // Escalate to human with required authority
    return await requestHumanApproval(request, memory.enforcement.override_requires)
  }

  // 3. Approved - log override
  const auditEntry = await auditLogger.log({
    operation: 'enforcement_overridden',
    target_id: memory.id,
    principal: request.requester,
    rationale: request.justification,
    changes: { original_violation: request.original_check }
  })

  // 4. Schedule review if override rate is high
  if (await isOverrideRateHigh(memory.id)) {
    await scheduleMemoryReview(memory.id, 'high_override_rate')
  }

  return {
    approved: true,
    decided_by: request.requester,
    decision_rationale: request.justification,
    audit_entry_id: auditEntry.audit_id,
    requires_review: await isOverrideRateHigh(memory.id)
  }
}
```

### 6.3 CLI Override

```bash
# Attempt action
anvil run deploy --env=production

# Blocked by Edda
❌ Blocked: Production deployments require approval
Memory: EDDA-M-doctrine-001
Policy: All production deployments must be reviewed by team lead

# Request override (if allowed)
anvil run deploy --env=production --force \
  --justification="Hotfix for critical security issue (CVE-2024-001)"

# Or request approval
anvil edda request-override \
  --memory=EDDA-M-doctrine-001 \
  --justification="Emergency hotfix" \
  --approver=user:alice
```

---

## 7. Performance Considerations

### 7.1 Hook Execution Performance

**Target:** <50ms overhead per action

**Optimizations:**

1. **Index hooks by event** - O(1) lookup
2. **Cache memory queries** - LRU cache for hot memories
3. **Parallel hook execution** - Execute non-dependent hooks in parallel
4. **Short-circuit on block** - Stop on first blocking violation
5. **Lazy memory loading** - Only load memory details if needed

```typescript
class OptimizedHookEngine {
  private hookIndex: Map<HookEvent, EnforcementHook[]>;
  private memoryCache: LRUCache<MemoryId, MemoryObjectExtended>;

  async executeHooks(
    event: HookEvent,
    context: ExecutionContext
  ): Promise<HookExecutionResult> {
    // Fast path: check cache
    const hooks = this.hookIndex.get(event) || [];
    if (hooks.length === 0) {
      return { allowed: true, violations: [], warnings: [], suggestions: [] };
    }

    // Execute hooks in parallel (up to first block)
    const results = await Promise.all(
      hooks.map((hook) => this.executeHook(hook, context))
    );

    // Short-circuit on block
    const blockingResult = results.find((r) => r.action === 'block');
    if (blockingResult) {
      return {
        allowed: false,
        violations: [blockingResult],
        warnings: [],
        suggestions: [],
      };
    }

    return {
      allowed: true,
      violations: [],
      warnings: results.filter((r) => r.action === 'warn'),
      suggestions: results.filter((r) => r.action === 'suggest'),
    };
  }
}
```

### 7.2 Memory Query Optimization

```typescript
// Instead of loading full memory objects:
const memories = await edda.queryMemories({
  types: hook.applicable_memories.types,
  tags: hook.applicable_memories.tags,
});

// Use lightweight matching:
const memoryIds = await edda.findMatchingMemoryIds({
  types: hook.applicable_memories.types,
  tags: hook.applicable_memories.tags,
});

// Load only if needed
if (memoryIds.length > 0) {
  const memories = await edda.getMemories(memoryIds);
}
```

---

## 8. Testing Strategy

### 8.1 Unit Tests

```typescript
describe('HookExecutionEngine', () => {
  test('executes hooks in priority order', async () => {
    const engine = new HookExecutionEngine()
    const hooks = [
      { priority: 10, ... },
      { priority: 100, ... },
      { priority: 50, ... }
    ]

    const result = await engine.executeHooks('action_about_to_execute', context)

    expect(executionOrder).toEqual([100, 50, 10])
  })

  test('short-circuits on blocking violation', async () => {
    // ... test that execution stops on first block
  })

  test('renders message templates correctly', () => {
    const message = engine.renderMessage(
      'Blocked: {memory.statement}',
      memory,
      context
    )

    expect(message).toContain(memory.statement)
  })
})
```

### 8.2 Integration Tests

```typescript
describe('Anvil-Edda Integration', () => {
  test('blocks action when hook returns violation', async () => {
    // Setup hook
    await edda.createHook({
      type: 'pre_execution',
      trigger: { event: 'action_about_to_execute' },
      applicable_memories: { types: ['constraint'] },
      action: { mode: 'block' },
    });

    // Attempt action
    const result = await anvil.executeAction(action);

    expect(result.blocked).toBe(true);
    expect(result.violations).toHaveLength(1);
  });

  test('surfaces warnings during file write', async () => {
    // ... test warning flow
  });
});
```

### 8.3 Performance Tests

```typescript
test('hook execution overhead <50ms', async () => {
  const start = Date.now();

  await engine.executeHooks('action_about_to_execute', context);

  const duration = Date.now() - start;
  expect(duration).toBeLessThan(50);
});
```

---

## 9. Implementation Checklist

### Phase 4.1: Hook Framework (3 days)

- [ ] Define `EnforcementHook` schema
- [ ] Implement hook storage (Git YAML)
- [ ] Hook registry with indexing
- [ ] Trigger condition evaluator
- [ ] Memory matcher
- [ ] Unit tests

### Phase 4.2: Pre-Execution Checks (2 days)

- [ ] Pre-execution check API
- [ ] Action context extraction
- [ ] Policy violation detection
- [ ] Constraint checking
- [ ] Integration tests

### Phase 4.3: Enforcement Modes (2 days)

- [ ] Advisory, warning, blocking modes
- [ ] Override mechanism
- [ ] Approval workflow
- [ ] Unit tests

### Phase 4.4: Anvil Integration (3 days)

- [ ] Gate system hooks
- [ ] Pre-action hooks
- [ ] File change hooks
- [ ] Planning guidance
- [ ] E2E tests

### Phase 4.5: Guidance System (2 days)

- [ ] Guidance request API
- [ ] Context-based retrieval
- [ ] Relevance scoring
- [ ] Suggestion generation
- [ ] Unit tests

### Phase 4.6: CLI Commands (1 day)

- [ ] `anvil edda hooks list`
- [ ] `anvil edda hooks create <config>`
- [ ] `anvil edda check <action>` (dry-run)
- [ ] `anvil edda request-override`
- [ ] E2E tests

---

## 10. Success Criteria

- [ ] Hooks can block/warn/suggest on actions
- [ ] Integrated with Anvil execution pipeline
- [ ] Override mechanism works with audit
- [ ] Guidance surfaces relevant memories during planning
- [ ] Performance: <50ms overhead per action
- [ ] No false blocks (incorrect violations)
- [ ] Warnings are actionable and helpful
- [ ] Suggestions are contextually relevant

---

**Next:** See `/plans/edda-phase-breakdown.md` Phase 4 for detailed task
breakdown
