# Phase 4: Enforcement & Guidance Hooks - APS Document

**Phase:** 4 of 7
**Duration:** 3 weeks (15 working days)
**Dependencies:** Phase 0 (Foundation), Phase 2 (Authority & Trust), Phase 3 (Query & Retrieval)
**Status:** Not Started
**Owner:** TBD

---

## Phase Overview

### Purpose
Implement the enforcement and guidance hook system that makes Edda actionable by blocking constraint violations, warning about risks, and providing contextual guidance during Anvil execution.

### Scope
This phase delivers the pre-execution checking system that integrates with Anvil to enforce Edda memories as executable policies, transforming Edda from passive knowledge storage into active institutional oversight.

### Success Criteria
- ✅ Pre-execution hooks block constraint violations
- ✅ Guidance hooks provide non-blocking suggestions
- ✅ Hook matching system identifies applicable memories
- ✅ Anvil integration working (pre-action checks)
- ✅ <50ms overhead per action check
- ✅ CLI commands for hook management
- ✅ 100% test coverage on enforcement logic

---

## Epic Breakdown

### Epic 1: Hook Schema & Storage
**Duration:** 2 days
**Priority:** P0 (Blocking)

#### Epic 1.1: Hook Definition Schema
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Define and implement the enforcement hook schema with triggers, matchers, and actions.

**Acceptance Criteria:**
- EnforcementHook interface matches edda-enforcement-hooks.md spec
- 5 hook types supported (pre_execution, validation, guidance, post_execution, approval_required)
- Hook triggers support multiple patterns (action_type, tool_name, context_pattern)
- Memory matchers support type, tags, scope filtering
- Actions support block, warn, suggest, require_approval
- Zod schema validation

**Implementation:**

```typescript
// packages/edda-core/src/enforcement/hook-schema.ts

import { z } from 'zod'

export enum EnforcementHookType {
  PRE_EXECUTION = 'pre_execution',
  VALIDATION = 'validation',
  GUIDANCE = 'guidance',
  POST_EXECUTION = 'post_execution',
  APPROVAL_REQUIRED = 'approval_required',
}

export enum BlockingSeverity {
  BLOCK = 'block',        // Hard block (cannot proceed)
  WARN = 'warn',          // Soft warning (can proceed)
  SUGGEST = 'suggest',    // Informational only
}

export const HookTriggerSchema = z.object({
  // Action patterns
  action_types: z.array(z.string()).optional(),  // ['tool_use', 'file_edit', 'bash_command']
  tool_names: z.array(z.string()).optional(),    // ['bash', 'kubectl', 'git']
  action_pattern: z.string().optional(),         // Regex for action description

  // Context patterns
  file_patterns: z.array(z.string()).optional(), // ['**/*.prod.yaml', 'db/migrations/**']
  scope_patterns: z.array(z.string()).optional(), // ['production/*', 'security/*']

  // Conditions
  require_all: z.boolean().default(false),       // AND vs OR matching
})

export type HookTrigger = z.infer<typeof HookTriggerSchema>

export const MemoryMatcherSchema = z.object({
  memory_types: z.array(z.enum(['decision', 'pattern', 'warning', 'constraint', 'doctrine', 'lesson'])).optional(),
  statuses: z.array(z.enum(['active', 'deprecated', 'superseded'])).optional(),
  confidence_min: z.enum(['low', 'medium', 'high']).optional(),
  tags: z.array(z.string()).optional(),
  scopes: z.array(z.string()).optional(),  // Scope prefixes
  text_query: z.string().optional(),       // FTS5 query
})

export type MemoryMatcher = z.infer<typeof MemoryMatcherSchema>

export const HookActionSchema = z.object({
  severity: z.enum(['block', 'warn', 'suggest']),
  message_template: z.string(),            // Template with {{variables}}
  include_memories: z.boolean().default(true),
  include_guidance: z.boolean().default(true),
  require_override: z.boolean().default(false), // Can admin override?
})

export type HookAction = z.infer<typeof HookActionSchema>

export const EnforcementHookSchema = z.object({
  hook_id: z.string().regex(/^EDDA-HOOK-/),
  type: z.nativeEnum(EnforcementHookType),
  name: z.string().min(1).max(200),
  description: z.string().max(2000),
  enabled: z.boolean().default(true),
  priority: z.number().int().min(0).max(100).default(50),

  trigger: HookTriggerSchema,
  applicable_memories: MemoryMatcherSchema,
  action: HookActionSchema,

  metadata: z.record(z.any()).optional(),
  created_at: z.string().datetime(),
  updated_at: z.string().datetime(),
  created_by: z.string(),
})

export type EnforcementHook = z.infer<typeof EnforcementHookSchema>
```

**File Structure:**
```
packages/edda-core/src/enforcement/
├── hook-schema.ts
└── __tests__/
    └── hook-schema.test.ts
```

**Tests:**
- Validate hook with all fields
- Reject invalid hook (missing required fields)
- Trigger pattern validation
- Memory matcher validation
- Action validation

---

#### Epic 1.2: Hook Repository
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Implement Git-backed YAML storage for enforcement hooks.

**Acceptance Criteria:**
- Hooks stored in `.edda/hooks/`
- YAML format: `{hook-type}-{name-slug}.yaml`
- CRUD operations (create, get, list, update, delete)
- SQLite index for fast querying by type/trigger

**Implementation:**

```typescript
// packages/edda-core/src/enforcement/hook-repository.ts

export interface IHookRepository {
  /**
   * Get hook by ID
   */
  get(hookId: HookId): Promise<EnforcementHook>

  /**
   * List all hooks (optionally filtered)
   */
  list(filter?: HookFilter): Promise<EnforcementHook[]>

  /**
   * Create or update hook
   */
  upsert(hook: EnforcementHook): Promise<void>

  /**
   * Delete hook
   */
  delete(hookId: HookId): Promise<void>

  /**
   * Find hooks by trigger pattern (for matching)
   */
  findByTrigger(triggerContext: TriggerContext): Promise<EnforcementHook[]>
}

export interface HookFilter {
  type?: EnforcementHookType
  enabled?: boolean
  priority_min?: number
}

export interface TriggerContext {
  action_type?: string
  tool_name?: string
  action_description?: string
  file_path?: string
  scope?: string
}

export class HookRepository implements IHookRepository {
  constructor(
    private gitStorage: IGitStorage,
    private index: IHookIndex,
  ) {}

  async get(hookId: HookId): Promise<EnforcementHook> {
    const filePath = `.edda/hooks/${hookId}.yaml`
    const content = await this.gitStorage.read(filePath)
    const parsed = yaml.parse(content)
    return EnforcementHookSchema.parse(parsed)
  }

  async list(filter?: HookFilter): Promise<EnforcementHook[]> {
    const hookIds = await this.index.query(filter)
    const hooks = await Promise.all(hookIds.map(id => this.get(id)))

    return hooks
      .filter(h => !filter?.enabled || h.enabled === filter.enabled)
      .sort((a, b) => b.priority - a.priority)
  }

  async upsert(hook: EnforcementHook): Promise<void> {
    // Validate
    EnforcementHookSchema.parse(hook)

    // Write to Git
    const filePath = `.edda/hooks/${hook.hook_id}.yaml`
    const content = yaml.stringify(hook)
    await this.gitStorage.write(filePath, content, `Update hook ${hook.hook_id}`)

    // Index
    await this.index.upsert(hook)
  }

  async delete(hookId: HookId): Promise<void> {
    const filePath = `.edda/hooks/${hookId}.yaml`
    await this.gitStorage.delete(filePath, `Delete hook ${hookId}`)
    await this.index.delete(hookId)
  }

  async findByTrigger(triggerContext: TriggerContext): Promise<EnforcementHook[]> {
    // Query index for hooks that might match
    const candidates = await this.index.findCandidates(triggerContext)

    // Load full hooks and filter
    const hooks = await Promise.all(candidates.map(id => this.get(id)))

    return hooks
      .filter(h => h.enabled)
      .filter(h => this.matchesTrigger(h.trigger, triggerContext))
      .sort((a, b) => b.priority - a.priority)
  }

  private matchesTrigger(trigger: HookTrigger, context: TriggerContext): boolean {
    const matches: boolean[] = []

    if (trigger.action_types && context.action_type) {
      matches.push(trigger.action_types.includes(context.action_type))
    }

    if (trigger.tool_names && context.tool_name) {
      matches.push(trigger.tool_names.includes(context.tool_name))
    }

    if (trigger.action_pattern && context.action_description) {
      const regex = new RegExp(trigger.action_pattern, 'i')
      matches.push(regex.test(context.action_description))
    }

    if (trigger.file_patterns && context.file_path) {
      matches.push(trigger.file_patterns.some(pattern =>
        minimatch(context.file_path!, pattern)
      ))
    }

    if (trigger.scope_patterns && context.scope) {
      matches.push(trigger.scope_patterns.some(pattern =>
        context.scope!.startsWith(pattern.replace('*', ''))
      ))
    }

    // AND vs OR logic
    if (trigger.require_all) {
      return matches.length > 0 && matches.every(m => m)
    } else {
      return matches.some(m => m)
    }
  }
}
```

**SQLite Index Schema:**
```sql
CREATE TABLE hook_index (
  hook_id TEXT PRIMARY KEY,
  type TEXT NOT NULL,
  enabled INTEGER NOT NULL,
  priority INTEGER NOT NULL,
  action_types_json TEXT,  -- JSON array
  tool_names_json TEXT,     -- JSON array
  created_at TEXT NOT NULL
);

CREATE INDEX idx_hook_type ON hook_index(type, enabled);
CREATE INDEX idx_hook_priority ON hook_index(priority DESC);
```

**Storage Structure:**
```
.edda/hooks/
├── EDDA-HOOK-01HQZX.yaml   # Pre-execution: no production kubectl
├── EDDA-HOOK-02ABCD.yaml   # Validation: require tests
├── EDDA-HOOK-03EFGH.yaml   # Guidance: suggest alternatives
```

**File Structure:**
```
packages/edda-core/src/enforcement/
├── hook-repository.ts
├── hook-index.ts
└── __tests__/
    ├── hook-repository.test.ts
    └── hook-index.test.ts
```

**Tests:**
- Create hook (stored in Git + indexed)
- Get hook by ID
- List hooks filtered by type
- Update hook
- Delete hook
- Find hooks by trigger (action_type, tool_name, file_pattern)
- Trigger matching logic (AND vs OR)

---

### Epic 2: Hook Matching Engine
**Duration:** 3 days
**Priority:** P0 (Blocking)

#### Epic 2.1: Memory Matcher
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Implement memory matching logic to find applicable memories for a given context.

**Acceptance Criteria:**
- findApplicableMemories() queries based on MemoryMatcher
- Supports type, status, confidence, tags, scope, text query
- Returns memories ranked by relevance
- Uses query service from Phase 3

**Implementation:**

```typescript
// packages/edda-core/src/enforcement/memory-matcher.ts

export interface IMemoryMatcher {
  /**
   * Find memories applicable to the given context
   */
  findApplicableMemories(
    matcher: MemoryMatcher,
    context: EnforcementContext,
  ): Promise<MemoryObject[]>
}

export interface EnforcementContext {
  action_type?: string
  tool_name?: string
  action_description?: string
  file_path?: string
  scope?: string
  tags?: string[]
}

export class MemoryMatcher implements IMemoryMatcher {
  constructor(
    private queryService: IQueryService,
  ) {}

  async findApplicableMemories(
    matcher: MemoryMatcher,
    context: EnforcementContext,
  ): Promise<MemoryObject[]> {
    // Build query from matcher
    const query: EddaQuery = {
      filters: {
        type: matcher.memory_types,
        status: matcher.statuses || ['active'], // Default to active only
        confidence: this.getConfidenceFilter(matcher.confidence_min),
        tags: matcher.tags,
        scope: this.buildScopePattern(matcher.scopes, context.scope),
      },
      search: matcher.text_query ? {
        query: this.buildSearchQuery(matcher.text_query, context),
      } : undefined,
      sort: {
        field: 'relevance',
        direction: 'desc',
      },
      pagination: {
        limit: 50, // Max applicable memories per hook
        offset: 0,
      },
    }

    const result = await this.queryService.query(query)
    return result.memories
  }

  private getConfidenceFilter(min?: EddaConfidenceLevel): EddaConfidenceLevel[] | undefined {
    if (!min) return undefined

    const levels: EddaConfidenceLevel[] = ['high', 'medium', 'low']
    const minIndex = levels.indexOf(min)

    return levels.slice(0, minIndex + 1)
  }

  private buildScopePattern(
    matcherScopes: string[] | undefined,
    contextScope: string | undefined,
  ): string | undefined {
    if (!matcherScopes || matcherScopes.length === 0) return contextScope

    // Match memories where scope matches context AND matcher patterns
    if (contextScope) {
      return contextScope
    }

    // Use first matcher scope as filter
    return matcherScopes[0]
  }

  private buildSearchQuery(textQuery: string, context: EnforcementContext): string {
    // Augment text query with context
    let query = textQuery

    if (context.tool_name) {
      query += ` OR ${context.tool_name}`
    }

    if (context.tags && context.tags.length > 0) {
      query += ` OR ${context.tags.join(' OR ')}`
    }

    return query
  }
}
```

**Tests:**
- Find memories by type
- Find memories by tags
- Find memories by scope
- Find memories by text query
- Confidence filtering (high only, medium+high, all)
- Context augmentation (adds tool_name to query)

---

#### Epic 2.2: Hook Evaluator
**Estimate:** 6 hours
**Owner:** TBD

**Description:**
Evaluate hooks against an action context to determine if they apply and what action to take.

**Acceptance Criteria:**
- evaluateHooks() takes action context, returns enforcement results
- Checks all applicable hooks in priority order
- Returns blocking violations, warnings, suggestions
- Includes matched memories and guidance
- <50ms evaluation time

**Implementation:**

```typescript
// packages/edda-core/src/enforcement/hook-evaluator.ts

export interface EnforcementResult {
  allowed: boolean
  blocking_violations: Violation[]
  warnings: Warning[]
  suggestions: Suggestion[]
  evaluation_time_ms: number
}

export interface Violation {
  hook: EnforcementHook
  memories: MemoryObject[]
  message: string
  can_override: boolean
}

export interface Warning {
  hook: EnforcementHook
  memories: MemoryObject[]
  message: string
}

export interface Suggestion {
  hook: EnforcementHook
  memories: MemoryObject[]
  message: string
  alternatives?: string[]
}

export interface IHookEvaluator {
  /**
   * Evaluate all applicable hooks for an action
   */
  evaluateHooks(context: EnforcementContext): Promise<EnforcementResult>

  /**
   * Check specific hook against context
   */
  evaluateHook(hook: EnforcementHook, context: EnforcementContext): Promise<HookResult | null>
}

export interface HookResult {
  hook: EnforcementHook
  applies: boolean
  memories: MemoryObject[]
  rendered_message: string
}

export class HookEvaluator implements IHookEvaluator {
  constructor(
    private hookRepo: IHookRepository,
    private memoryMatcher: IMemoryMatcher,
  ) {}

  async evaluateHooks(context: EnforcementContext): Promise<EnforcementResult> {
    const startTime = performance.now()

    // Find applicable hooks
    const hooks = await this.hookRepo.findByTrigger(context)

    // Evaluate each hook
    const results = await Promise.all(
      hooks.map(hook => this.evaluateHook(hook, context))
    )

    // Filter out non-applicable
    const applicable = results.filter(r => r !== null && r.applies) as HookResult[]

    // Categorize by severity
    const blocking_violations: Violation[] = []
    const warnings: Warning[] = []
    const suggestions: Suggestion[] = []

    for (const result of applicable) {
      if (result.hook.action.severity === 'block') {
        blocking_violations.push({
          hook: result.hook,
          memories: result.memories,
          message: result.rendered_message,
          can_override: result.hook.action.require_override || false,
        })
      } else if (result.hook.action.severity === 'warn') {
        warnings.push({
          hook: result.hook,
          memories: result.memories,
          message: result.rendered_message,
        })
      } else if (result.hook.action.severity === 'suggest') {
        suggestions.push({
          hook: result.hook,
          memories: result.memories,
          message: result.rendered_message,
          alternatives: this.extractAlternatives(result.memories),
        })
      }
    }

    const evaluationTime = performance.now() - startTime

    return {
      allowed: blocking_violations.length === 0,
      blocking_violations,
      warnings,
      suggestions,
      evaluation_time_ms: Math.round(evaluationTime),
    }
  }

  async evaluateHook(
    hook: EnforcementHook,
    context: EnforcementContext,
  ): Promise<HookResult | null> {
    // Find applicable memories
    const memories = await this.memoryMatcher.findApplicableMemories(
      hook.applicable_memories,
      context,
    )

    if (memories.length === 0) {
      return null // Hook doesn't apply
    }

    // Render message template
    const message = this.renderMessage(hook.action.message_template, {
      context,
      memories,
      hook,
    })

    return {
      hook,
      applies: true,
      memories,
      rendered_message: message,
    }
  }

  private renderMessage(
    template: string,
    data: { context: EnforcementContext; memories: MemoryObject[]; hook: EnforcementHook },
  ): string {
    let message = template

    // Replace {{variables}}
    message = message.replace(/\{\{action\}\}/g, data.context.action_description || 'this action')
    message = message.replace(/\{\{tool\}\}/g, data.context.tool_name || 'this tool')
    message = message.replace(/\{\{file\}\}/g, data.context.file_path || '')
    message = message.replace(/\{\{memory_count\}\}/g, String(data.memories.length))

    return message
  }

  private extractAlternatives(memories: MemoryObject[]): string[] {
    // Extract alternatives from memory statements
    // (Look for patterns like "use X instead of Y")
    const alternatives: string[] = []

    for (const memory of memories) {
      const match = memory.statement.match(/(?:use|prefer|try)\s+([^,\.]+)/i)
      if (match) {
        alternatives.push(match[1].trim())
      }
    }

    return alternatives
  }
}
```

**File Structure:**
```
packages/edda-core/src/enforcement/
├── memory-matcher.ts
├── hook-evaluator.ts
└── __tests__/
    ├── memory-matcher.test.ts
    └── hook-evaluator.test.ts
```

**Tests:**
- Evaluate hooks (finds applicable hooks)
- Blocking violation stops execution
- Warnings don't block
- Suggestions provide alternatives
- Message template rendering
- Performance: <50ms for 10 hooks
- No applicable memories = hook doesn't apply

---

#### Epic 2.3: Enforcement Service
**Estimate:** 2 hours
**Owner:** TBD

**Description:**
Top-level enforcement service that orchestrates hook evaluation.

**Acceptance Criteria:**
- IEnforcementService interface implemented
- checkAction() is main entry point for Anvil
- Returns EnforcementResult
- Logs enforcement decisions to audit trail

**Implementation:**

```typescript
// packages/edda-core/src/enforcement/enforcement-service.ts

export interface IEnforcementService {
  /**
   * Check if action is allowed
   * Main entry point from Anvil
   */
  checkAction(
    action: ActionContext,
    principal: Principal,
  ): Promise<EnforcementResult>

  /**
   * Override a blocking violation (admin only)
   */
  overrideViolation(
    action: ActionContext,
    principal: Principal,
    violationId: string,
    reason: string,
  ): Promise<void>
}

export interface ActionContext {
  action_id: string
  action_type: string          // 'tool_use', 'file_edit', etc.
  tool_name?: string
  description: string
  file_path?: string
  scope?: string
  metadata?: Record<string, any>
}

export class EnforcementService implements IEnforcementService {
  constructor(
    private hookEvaluator: IHookEvaluator,
    private authz: IAuthorizationService,
    private audit: IAuditTrailService,
  ) {}

  async checkAction(
    action: ActionContext,
    principal: Principal,
  ): Promise<EnforcementResult> {
    const context: EnforcementContext = {
      action_type: action.action_type,
      tool_name: action.tool_name,
      action_description: action.description,
      file_path: action.file_path,
      scope: action.scope,
    }

    // Evaluate hooks
    const result = await this.hookEvaluator.evaluateHooks(context)

    // Audit
    await this.audit.log({
      actor: principal.identifier,
      action: result.allowed ? AuditAction.ENFORCEMENT_ALLOWED : AuditAction.ENFORCEMENT_BLOCKED,
      resource_type: 'action',
      resource_id: action.action_id,
      outcome: 'success',
      metadata: {
        action_type: action.action_type,
        violations: result.blocking_violations.length,
        warnings: result.warnings.length,
        evaluation_time_ms: result.evaluation_time_ms,
      },
    })

    return result
  }

  async overrideViolation(
    action: ActionContext,
    principal: Principal,
    violationId: string,
    reason: string,
  ): Promise<void> {
    // Check authorization (only admins can override)
    await this.authz.authorize(principal, Permission.ENFORCEMENT_OVERRIDE)

    // Audit override
    await this.audit.log({
      actor: principal.identifier,
      action: AuditAction.ENFORCEMENT_OVERRIDE,
      resource_type: 'action',
      resource_id: action.action_id,
      outcome: 'success',
      metadata: {
        violation_id: violationId,
        reason,
      },
    })

    // In practice, override would be handled by Anvil re-checking with override flag
  }
}
```

**Tests:**
- Check action (allowed)
- Check action (blocked)
- Check action (warnings only)
- Override violation (requires permission)
- Audit trail logs enforcement decisions

---

### Epic 3: Anvil Integration
**Duration:** 4 days
**Priority:** P0 (Blocking)

#### Epic 3.1: Enforcement Port in Anvil
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Add Edda enforcement port to Anvil for pre-action checks.

**Acceptance Criteria:**
- IEddaEnforcementPort interface defined
- Port queries Edda before actions
- Returns EnforcementResult
- Handles Edda unavailable gracefully

**Implementation:**

```typescript
// packages/anvil/src/ports/edda-enforcement-port.ts

export interface IEddaEnforcementPort {
  /**
   * Check if action is allowed by Edda policies
   */
  checkAction(action: ActionContext): Promise<EnforcementResult>

  /**
   * Check if Edda enforcement is available
   */
  isAvailable(): Promise<boolean>
}

export class EddaEnforcementPort implements IEddaEnforcementPort {
  constructor(
    private eddaClient: IEddaClient,  // RPC client to Edda service
    private config: EnforcementConfig,
  ) {}

  async checkAction(action: ActionContext): Promise<EnforcementResult> {
    try {
      const result = await this.eddaClient.enforcement.checkAction(action)
      return result
    } catch (error) {
      if (this.config.fail_open) {
        // If Edda unavailable, allow action
        console.warn('Edda enforcement unavailable, allowing action (fail-open)', error)
        return {
          allowed: true,
          blocking_violations: [],
          warnings: [],
          suggestions: [],
          evaluation_time_ms: 0,
        }
      } else {
        // Fail closed: block action
        throw new Error('Edda enforcement unavailable (fail-closed)')
      }
    }
  }

  async isAvailable(): Promise<boolean> {
    try {
      await this.eddaClient.ping()
      return true
    } catch {
      return false
    }
  }
}

export interface EnforcementConfig {
  enabled: boolean
  fail_open: boolean  // Allow actions if Edda unavailable?
}
```

**Tests:**
- Check action (Edda available)
- Check action (Edda unavailable, fail-open)
- Check action (Edda unavailable, fail-closed throws)

---

#### Epic 3.2: Pre-Action Hook in Anvil Executor
**Estimate:** 6 hours
**Owner:** TBD

**Description:**
Integrate enforcement checks into Anvil's action execution pipeline.

**Acceptance Criteria:**
- Before every action, call enforcement port
- If blocked, prevent action and show violation message
- If warnings/suggestions, display to user/agent
- User can override warnings (not blocks)
- Performance overhead <50ms

**Implementation:**

```typescript
// packages/anvil/src/executor/action-executor.ts (enhancement)

export class ActionExecutor {
  constructor(
    private toolRegistry: IToolRegistry,
    private enforcementPort: IEddaEnforcementPort,
    private principal: Principal,
  ) {}

  async executeAction(action: Action): Promise<ActionResult> {
    // Pre-execution enforcement check
    const enforcementResult = await this.checkEnforcement(action)

    if (!enforcementResult.allowed) {
      return this.handleBlocked(action, enforcementResult)
    }

    // Show warnings/suggestions
    if (enforcementResult.warnings.length > 0 || enforcementResult.suggestions.length > 0) {
      await this.showGuidance(enforcementResult)
    }

    // Execute action
    const result = await this.doExecute(action)

    return result
  }

  private async checkEnforcement(action: Action): Promise<EnforcementResult> {
    const actionContext: ActionContext = {
      action_id: action.id,
      action_type: this.getActionType(action),
      tool_name: action.tool,
      description: action.description,
      file_path: this.extractFilePath(action),
      scope: this.inferScope(action),
    }

    return await this.enforcementPort.checkAction(actionContext)
  }

  private handleBlocked(action: Action, result: EnforcementResult): ActionResult {
    const messages: string[] = [
      '🚫 Action blocked by Edda enforcement policies:',
      '',
    ]

    for (const violation of result.blocking_violations) {
      messages.push(`❌ ${violation.hook.name}`)
      messages.push(`   ${violation.message}`)
      messages.push(`   Relevant memories: ${violation.memories.length}`)

      for (const memory of violation.memories.slice(0, 3)) { // Show first 3
        messages.push(`   - ${memory.statement.slice(0, 80)}...`)
      }

      messages.push('')

      if (violation.can_override) {
        messages.push('   ℹ️  Admin can override with: anvil override <action-id> --reason "..."')
        messages.push('')
      }
    }

    return {
      success: false,
      blocked: true,
      message: messages.join('\n'),
      metadata: {
        enforcement_result: result,
      },
    }
  }

  private async showGuidance(result: EnforcementResult): Promise<void> {
    // Show warnings
    for (const warning of result.warnings) {
      console.warn(`⚠️  ${warning.hook.name}: ${warning.message}`)

      for (const memory of warning.memories.slice(0, 2)) {
        console.warn(`   - ${memory.statement.slice(0, 80)}...`)
      }
    }

    // Show suggestions
    for (const suggestion of result.suggestions) {
      console.log(`💡 ${suggestion.hook.name}: ${suggestion.message}`)

      if (suggestion.alternatives && suggestion.alternatives.length > 0) {
        console.log(`   Alternatives: ${suggestion.alternatives.join(', ')}`)
      }
    }
  }

  private getActionType(action: Action): string {
    if (action.tool === 'bash') return 'bash_command'
    if (action.tool === 'edit') return 'file_edit'
    if (action.tool === 'write') return 'file_write'
    return 'tool_use'
  }

  private extractFilePath(action: Action): string | undefined {
    // Extract file path from action parameters
    if (action.parameters.file_path) return action.parameters.file_path
    if (action.parameters.path) return action.parameters.path
    return undefined
  }

  private inferScope(action: Action): string | undefined {
    // Infer scope from file path or context
    const filePath = this.extractFilePath(action)
    if (!filePath) return undefined

    if (filePath.includes('production') || filePath.includes('.prod.')) {
      return 'production'
    }
    if (filePath.startsWith('db/migrations')) {
      return 'database/migrations'
    }

    return undefined
  }

  private async doExecute(action: Action): Promise<ActionResult> {
    // Existing execution logic...
  }
}
```

**Tests:**
- Action allowed (no violations)
- Action blocked (hard block)
- Action warned (soft warning)
- Action suggested (guidance provided)
- User sees violation messages
- Performance: <50ms overhead

---

#### Epic 3.3: Override Mechanism
**Estimate:** 3 hours
**Owner:** TBD

**Description:**
Allow admins to override blocking violations with justification.

**Acceptance Criteria:**
- `anvil override <action-id> --reason "..."` command
- Requires admin permission
- Logged to audit trail
- Override token passed to enforcement check

**Implementation:**

```typescript
// packages/anvil/src/commands/override.ts

export const overrideCommand: Command = {
  name: 'override',
  description: 'Override Edda enforcement violation (admin only)',
  args: [
    { name: 'action-id', required: true },
  ],
  options: [
    { name: 'reason', required: true },
  ],
  async execute(context, args, options) {
    const actionId = args['action-id']
    const reason = options.reason

    // Check admin permission
    const principal = context.currentPrincipal
    await context.edda.authorization.authorize(
      principal,
      Permission.ENFORCEMENT_OVERRIDE,
    )

    // Log override
    await context.edda.enforcement.overrideViolation(
      { action_id: actionId } as ActionContext,
      principal,
      actionId,
      reason,
    )

    console.log(`✅ Override recorded for action ${actionId}`)
    console.log(`   Reason: ${reason}`)
    console.log(`   You can now retry the action.`)
  },
}
```

**Tests:**
- Override violation (authorized)
- Override violation (unauthorized)
- Override logged to audit

---

### Epic 4: Hook Management CLI
**Duration:** 2 days
**Priority:** P1 (Important)

#### Epic 4.1: Hook CRUD CLI
**Estimate:** 6 hours
**Owner:** TBD

**Description:**
Implement CLI commands for creating and managing hooks.

**Acceptance Criteria:**
- `anvil edda hooks list` - List all hooks
- `anvil edda hooks show <hook-id>` - Show hook details
- `anvil edda hooks create` - Interactive hook creation
- `anvil edda hooks update <hook-id>` - Update hook
- `anvil edda hooks delete <hook-id>` - Delete hook
- `anvil edda hooks enable/disable <hook-id>` - Toggle enabled

**Implementation:**

```typescript
// packages/anvil/src/commands/edda/hooks.ts

export const hooksCommand: Command = {
  name: 'hooks',
  description: 'Manage Edda enforcement hooks',
  subcommands: [
    {
      name: 'list',
      description: 'List all enforcement hooks',
      options: [
        { name: 'type', choices: ['pre_execution', 'validation', 'guidance', 'post_execution', 'approval_required'] },
        { name: 'enabled', type: 'boolean' },
      ],
      async execute(context, args, options) {
        const hooks = await context.edda.hooks.list({
          type: options.type,
          enabled: options.enabled,
        })

        console.log(`Enforcement Hooks (${hooks.length}):\n`)
        console.log('HOOK ID          | NAME                  | TYPE           | SEVERITY | ENABLED | PRIORITY')
        console.log('─'.repeat(100))

        for (const hook of hooks) {
          const enabled = hook.enabled ? '✓' : ' '
          console.log(
            `${hook.hook_id.padEnd(16)} | ` +
            `${hook.name.padEnd(21).slice(0, 21)} | ` +
            `${hook.type.padEnd(14)} | ` +
            `${hook.action.severity.padEnd(8)} | ` +
            `${enabled.padStart(7)} | ` +
            `${String(hook.priority).padStart(8)}`
          )
        }
      },
    },

    {
      name: 'show',
      description: 'Show hook details',
      args: [{ name: 'hook-id', required: true }],
      async execute(context, args) {
        const hook = await context.edda.hooks.get(args['hook-id'])

        console.log(`Hook: ${hook.name}`)
        console.log(`ID: ${hook.hook_id}`)
        console.log(`Type: ${hook.type}`)
        console.log(`Enabled: ${hook.enabled}`)
        console.log(`Priority: ${hook.priority}`)
        console.log(`\nDescription:`)
        console.log(hook.description)
        console.log(`\nTrigger:`)
        console.log(JSON.stringify(hook.trigger, null, 2))
        console.log(`\nAction:`)
        console.log(`  Severity: ${hook.action.severity}`)
        console.log(`  Message: ${hook.action.message_template}`)
        console.log(`\nApplicable Memories:`)
        console.log(JSON.stringify(hook.applicable_memories, null, 2))
      },
    },

    {
      name: 'create',
      description: 'Create a new enforcement hook',
      async execute(context) {
        // Interactive prompts
        const answers = await prompts([
          {
            type: 'text',
            name: 'name',
            message: 'Hook name:',
          },
          {
            type: 'select',
            name: 'type',
            message: 'Hook type:',
            choices: [
              { title: 'Pre-execution (blocking)', value: 'pre_execution' },
              { title: 'Validation (planning)', value: 'validation' },
              { title: 'Guidance (suggestions)', value: 'guidance' },
              { title: 'Post-execution (audit)', value: 'post_execution' },
            ],
          },
          {
            type: 'text',
            name: 'description',
            message: 'Description:',
          },
          {
            type: 'select',
            name: 'severity',
            message: 'Severity:',
            choices: [
              { title: 'Block (hard block)', value: 'block' },
              { title: 'Warn (soft warning)', value: 'warn' },
              { title: 'Suggest (informational)', value: 'suggest' },
            ],
          },
          {
            type: 'text',
            name: 'message',
            message: 'Message template:',
          },
        ])

        const hook: EnforcementHook = {
          hook_id: `EDDA-HOOK-${ulid()}`,
          type: answers.type,
          name: answers.name,
          description: answers.description,
          enabled: true,
          priority: 50,
          trigger: {
            // TODO: collect trigger details
          },
          applicable_memories: {
            memory_types: ['constraint'],
            statuses: ['active'],
          },
          action: {
            severity: answers.severity,
            message_template: answers.message,
            include_memories: true,
            include_guidance: true,
          },
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          created_by: context.currentPrincipal.identifier,
        }

        await context.edda.hooks.upsert(hook)

        console.log(`✅ Created hook: ${hook.hook_id}`)
      },
    },

    {
      name: 'enable',
      description: 'Enable a hook',
      args: [{ name: 'hook-id', required: true }],
      async execute(context, args) {
        const hook = await context.edda.hooks.get(args['hook-id'])
        hook.enabled = true
        hook.updated_at = new Date().toISOString()
        await context.edda.hooks.upsert(hook)

        console.log(`✅ Enabled hook: ${hook.hook_id}`)
      },
    },

    {
      name: 'disable',
      description: 'Disable a hook',
      args: [{ name: 'hook-id', required: true }],
      async execute(context, args) {
        const hook = await context.edda.hooks.get(args['hook-id'])
        hook.enabled = false
        hook.updated_at = new Date().toISOString()
        await context.edda.hooks.upsert(hook)

        console.log(`✅ Disabled hook: ${hook.hook_id}`)
      },
    },

    // Similar for update, delete...
  ],
}
```

**Tests:**
- List hooks
- Show hook details
- Create hook (interactive)
- Enable/disable hook
- Delete hook requires permission

---

#### Epic 4.2: Check Simulation CLI
**Estimate:** 2 hours
**Owner:** TBD

**Description:**
Add CLI command to simulate enforcement check without executing action.

**Acceptance Criteria:**
- `anvil edda hooks check --action <desc> --tool <tool>` - Simulate check
- Shows which hooks would apply
- Shows violations, warnings, suggestions
- Useful for testing hooks

**Implementation:**

```typescript
// packages/anvil/src/commands/edda/hooks.ts (continued)

{
  name: 'check',
  description: 'Simulate enforcement check for an action',
  options: [
    { name: 'action', required: true },
    { name: 'tool' },
    { name: 'file' },
    { name: 'scope' },
  ],
  async execute(context, args, options) {
    const actionContext: ActionContext = {
      action_id: 'SIMULATION',
      action_type: 'tool_use',
      tool_name: options.tool,
      description: options.action,
      file_path: options.file,
      scope: options.scope,
    }

    const result = await context.edda.enforcement.checkAction(actionContext, context.currentPrincipal)

    console.log(`Enforcement Check Simulation\n`)
    console.log(`Action: ${options.action}`)
    console.log(`Tool: ${options.tool || 'N/A'}`)
    console.log(`File: ${options.file || 'N/A'}`)
    console.log(`Scope: ${options.scope || 'N/A'}`)
    console.log(`\nResult: ${result.allowed ? '✅ ALLOWED' : '🚫 BLOCKED'}`)
    console.log(`Evaluation time: ${result.evaluation_time_ms}ms\n`)

    if (result.blocking_violations.length > 0) {
      console.log(`Blocking Violations (${result.blocking_violations.length}):`)
      for (const violation of result.blocking_violations) {
        console.log(`  ❌ ${violation.hook.name}`)
        console.log(`     ${violation.message}`)
        console.log(`     Memories: ${violation.memories.length}`)
      }
      console.log()
    }

    if (result.warnings.length > 0) {
      console.log(`Warnings (${result.warnings.length}):`)
      for (const warning of result.warnings) {
        console.log(`  ⚠️  ${warning.hook.name}`)
        console.log(`     ${warning.message}`)
      }
      console.log()
    }

    if (result.suggestions.length > 0) {
      console.log(`Suggestions (${result.suggestions.length}):`)
      for (const suggestion of result.suggestions) {
        console.log(`  💡 ${suggestion.hook.name}`)
        console.log(`     ${suggestion.message}`)
        if (suggestion.alternatives) {
          console.log(`     Alternatives: ${suggestion.alternatives.join(', ')}`)
        }
      }
    }
  },
}
```

**Tests:**
- Simulate check (blocked)
- Simulate check (warnings)
- Simulate check (suggestions)
- Simulate check (allowed)

---

### Epic 5: Common Hooks Library
**Duration:** 2 days
**Priority:** P2 (Optional)

#### Epic 5.1: Predefined Hook Templates
**Estimate:** 6 hours
**Owner:** TBD

**Description:**
Create library of common enforcement hooks for typical use cases.

**Acceptance Criteria:**
- 10+ predefined hook templates
- Cover common scenarios (production safety, security, quality)
- `anvil edda hooks install <template>` command
- Templates stored in `.edda/hooks/templates/`

**Templates:**

1. **No Direct Production Access**
```yaml
hook_id: EDDA-HOOK-NO-PROD-KUBECTL
type: pre_execution
name: Block direct production kubectl
description: Prevent direct kubectl commands against production cluster
trigger:
  tool_names: [kubectl, k9s]
  scope_patterns: [production/*]
action:
  severity: block
  message_template: "Direct kubectl access to production is not allowed. Use {{tool}} via CI/CD pipeline."
applicable_memories:
  memory_types: [constraint]
  tags: [production, kubernetes]
```

2. **Require Tests for Code Changes**
```yaml
hook_id: EDDA-HOOK-REQUIRE-TESTS
type: validation
name: Require tests for code changes
description: Warn if code changes don't include test updates
trigger:
  action_types: [file_edit, file_write]
  file_patterns: [src/**/*.ts, src/**/*.js]
action:
  severity: warn
  message_template: "Code changes should include test updates. Consider adding tests."
applicable_memories:
  memory_types: [pattern]
  tags: [testing, quality]
```

3. **Database Migration Safety**
```yaml
hook_id: EDDA-HOOK-MIGRATION-SAFETY
type: pre_execution
name: Database migration safety check
description: Block destructive database migrations without approval
trigger:
  file_patterns: [db/migrations/**]
  action_pattern: "drop|truncate|delete"
action:
  severity: block
  message_template: "Destructive migration detected: {{action}}. Requires explicit approval."
  require_override: true
applicable_memories:
  memory_types: [constraint, warning]
  tags: [database, safety]
```

**Implementation:**

```typescript
// packages/edda-core/src/enforcement/hook-templates.ts

export const HOOK_TEMPLATES: Record<string, EnforcementHook> = {
  'no-prod-kubectl': { /* ... */ },
  'require-tests': { /* ... */ },
  'migration-safety': { /* ... */ },
  'no-hardcoded-secrets': { /* ... */ },
  'prefer-async-await': { /* ... */ },
  // ... more templates
}

export const installTemplateCommand: Command = {
  name: 'install',
  description: 'Install a predefined hook template',
  args: [{ name: 'template', required: true }],
  async execute(context, args) {
    const templateName = args.template

    if (!(templateName in HOOK_TEMPLATES)) {
      throw new Error(`Template '${templateName}' not found`)
    }

    const hook = HOOK_TEMPLATES[templateName]
    hook.hook_id = `EDDA-HOOK-${ulid()}`
    hook.created_at = new Date().toISOString()
    hook.updated_at = new Date().toISOString()
    hook.created_by = context.currentPrincipal.identifier

    await context.edda.hooks.upsert(hook)

    console.log(`✅ Installed hook template: ${templateName}`)
    console.log(`   Hook ID: ${hook.hook_id}`)
  },
}
```

**Tests:**
- List available templates
- Install template
- Installed hook works correctly

---

### Epic 6: Integration & Testing
**Duration:** 2 days (end of phase)
**Priority:** P0 (Blocking)

#### Epic 6.1: Integration Tests
**Estimate:** 6 hours
**Owner:** TBD

**Description:**
End-to-end integration tests for enforcement system.

**Test Scenarios:**

```typescript
describe('Enforcement Integration', () => {
  it('should block action violating constraint', async () => {
    // Create constraint memory
    await edda.memories.create(admin, {
      type: 'constraint',
      statement: 'Never use kubectl directly in production',
      tags: ['production', 'kubernetes'],
    })

    // Create enforcement hook
    const hook = createHook({
      type: 'pre_execution',
      trigger: {
        tool_names: ['kubectl'],
        scope_patterns: ['production/*'],
      },
      applicable_memories: {
        memory_types: ['constraint'],
        tags: ['production', 'kubernetes'],
      },
      action: {
        severity: 'block',
        message_template: 'Direct kubectl access blocked',
      },
    })
    await edda.hooks.upsert(hook)

    // Try to execute kubectl in production
    const result = await edda.enforcement.checkAction({
      action_id: 'test',
      action_type: 'tool_use',
      tool_name: 'kubectl',
      description: 'kubectl get pods',
      scope: 'production',
    }, testPrincipal)

    expect(result.allowed).toBe(false)
    expect(result.blocking_violations.length).toBe(1)
    expect(result.blocking_violations[0].memories.length).toBeGreaterThan(0)
  })

  it('should provide guidance without blocking', async () => {
    // Create pattern memory
    await edda.memories.create(admin, {
      type: 'pattern',
      statement: 'Prefer async/await over promise chains',
      tags: ['javascript', 'best-practice'],
    })

    // Create guidance hook
    const hook = createHook({
      type: 'guidance',
      trigger: {
        file_patterns: ['**/*.js', '**/*.ts'],
        action_pattern: '\\.then\\(',
      },
      applicable_memories: {
        memory_types: ['pattern'],
        tags: ['javascript'],
      },
      action: {
        severity: 'suggest',
        message_template: 'Consider using async/await instead of promise chains',
      },
    })
    await edda.hooks.upsert(hook)

    // Edit JS file with promise chains
    const result = await edda.enforcement.checkAction({
      action_id: 'test',
      action_type: 'file_edit',
      description: 'Add .then() call',
      file_path: 'src/service.js',
    }, testPrincipal)

    expect(result.allowed).toBe(true)
    expect(result.suggestions.length).toBeGreaterThan(0)
  })

  it('should complete check in <50ms', async () => {
    // Seed 10 hooks
    await seedHooks(10)

    const start = performance.now()
    await edda.enforcement.checkAction({
      action_id: 'test',
      action_type: 'tool_use',
      tool_name: 'bash',
      description: 'ls -la',
    }, testPrincipal)
    const end = performance.now()

    expect(end - start).toBeLessThan(50)
  })
})
```

**Tests:**
- Blocking constraint enforced
- Warning displayed but not blocked
- Guidance provided (suggestions)
- Admin can override with reason
- Performance: <50ms overhead
- Anvil integration works end-to-end
- 100% test coverage

---

## Timeline

### Week 1 (Days 1-5)
- **Day 1-2:** Epic 1 (Hook Schema & Storage)
- **Day 3-5:** Epic 2 (Hook Matching Engine)

### Week 2 (Days 6-10)
- **Day 6-9:** Epic 3 (Anvil Integration)
- **Day 10:** Epic 4 (Hook Management CLI) - Part 1

### Week 3 (Days 11-15)
- **Day 11:** Epic 4 (Hook Management CLI) - Part 2
- **Day 12-13:** Epic 5 (Common Hooks Library)
- **Day 14-15:** Epic 6 (Integration & Testing)

---

## Deliverables

### Package Structure
```
packages/edda-core/src/enforcement/
├── hook-schema.ts
├── hook-repository.ts
├── hook-index.ts
├── memory-matcher.ts
├── hook-evaluator.ts
├── enforcement-service.ts
├── hook-templates.ts
└── __tests__/
    ├── hook-repository.test.ts
    ├── memory-matcher.test.ts
    ├── hook-evaluator.test.ts
    ├── enforcement-service.test.ts
    └── integration/
        └── enforcement.integration.test.ts

packages/anvil/src/ports/
├── edda-enforcement-port.ts

packages/anvil/src/executor/
├── action-executor.ts (enhanced)

packages/anvil/src/commands/edda/
├── hooks.ts
├── override.ts
```

### Storage Structure
```
.edda/hooks/
├── EDDA-HOOK-01HQZX.yaml
├── EDDA-HOOK-02ABCD.yaml
└── templates/
    ├── no-prod-kubectl.yaml
    ├── require-tests.yaml
    └── migration-safety.yaml
```

### Documentation
- Enforcement & Guidance Hooks (already exists: `docs/specs/edda-enforcement-hooks.md`)
- Hook creation guide
- Template library reference
- Anvil integration guide

### Tests
- Unit tests: 50+ tests
- Integration tests: 10+ scenarios
- Performance tests: <50ms overhead
- Test coverage: 100%

---

## Success Metrics

### Functional
- ✅ Blocking hooks prevent violations
- ✅ Warnings displayed without blocking
- ✅ Guidance provides alternatives
- ✅ Admin overrides work correctly
- ✅ CLI commands operational

### Performance
- ✅ Enforcement check: <50ms per action
- ✅ Hook matching: <20ms
- ✅ Memory lookup: <30ms
- ✅ Minimal impact on Anvil execution

### Quality
- ✅ 100% test coverage
- ✅ All edge cases handled
- ✅ Graceful degradation if Edda unavailable
- ✅ Clear error/guidance messages

---

## Risks & Mitigation

### Risk 1: Performance Overhead Too High
**Probability:** Medium
**Impact:** High

**Mitigation:**
- Proper indexing for fast hook lookup
- Lazy loading of memories
- Cache frequently checked hooks
- Parallel evaluation where possible
- Performance regression tests

### Risk 2: False Positives (Over-blocking)
**Probability:** Medium
**Impact:** Medium

**Mitigation:**
- Start with guidance hooks (non-blocking)
- Gradual rollout of blocking hooks
- Override mechanism for admins
- User feedback collection
- Hook refinement based on false positives

### Risk 3: Edda Unavailability Blocks All Actions
**Probability:** Low
**Impact:** High

**Mitigation:**
- Fail-open configuration option
- Health check before enforcement
- Graceful degradation
- Clear error messages
- Retry logic with timeout

---

## Dependencies

### Upstream (Must Complete First)
- Phase 0: Foundation (memory storage)
- Phase 2: Authority & Trust (authorization, audit)
- Phase 3: Query & Retrieval (memory matching)

### Downstream (Blocked By This Phase)
- Phase 6: Interop & Export (enforcement APIs)
- Phase 7: Meta-Capabilities (enforcement analytics)

---

## Open Questions

None - all dependencies resolved.

---

## Next Steps

1. ✅ Complete Phase 0 (Foundation)
2. ✅ Complete Phase 1 (Promotion Pipeline)
3. ✅ Complete Phase 2 (Authority & Trust)
4. ✅ Complete Phase 3 (Query & Retrieval)
5. **Review this APS document** with team
6. **Coordinate with Anvil team** for integration points
7. **Assign owners** to epics and tasks
8. **Kick off Phase 4** implementation

---

**Document Version:** 1.0
**Last Updated:** 2026-01-19
**Status:** Ready for Review
**Estimated Completion:** 3 weeks after Phase 3 completion
