/**
 * Template generator for APS planning documents
 *
 * Generates Markdown templates for index files and leaf specs
 * based on the APS Planning Spec v0.1.
 *
 * Templates come in three variants:
 * - **minimal**: Quick start, bare essentials
 * - **standard**: Recommended for most projects (default)
 * - **full**: Comprehensive, for complex enterprise plans
 */

/**
 * Template variant options
 */
export type TemplateVariant = 'minimal' | 'standard' | 'full';

/**
 * Bundle of all APS planning document templates
 */
export interface TemplateBundle {
  /** Index file template for multi-module plans */
  index: string;
  /** Leaf spec template for individual modules */
  leaf: string;
  /** Simple single-file plan template */
  simple: string;
  /** Action plan template for execution breakdowns */
  actions: string;
}

/**
 * Options for template generation
 */
export interface TemplateOptions {
  /** Template variant: minimal, standard, or full */
  variant?: TemplateVariant;
}

// ============================================================================
// Index Templates
// ============================================================================

/**
 * Minimal index template - navigation only
 */
function generateMinimalIndexTemplate(): string {
  return `# [Plan Title]

## Modules

### [module-id]

- **Path:** [./modules/[module-name].aps.md](./modules/[module-name].aps.md)
- **Scope:** [SCOPE]
- **Owner:** @[username]

### [another-module-id]

- **Path:** [./modules/[another-module].aps.md](./modules/[another-module].aps.md)
- **Scope:** [SCOPE2]
- **Owner:** @[username]
`;
}

/**
 * Standard index template - recommended for most projects
 * The index is a map, not the territory.
 */
function generateStandardIndexTemplate(): string {
  return `# [Plan Title]

## Problem & Success Criteria

**Problem:** [What problem are we solving? Why does this work matter?]

**Success Criteria:**
- [ ] [Measurable outcome 1]
- [ ] [Measurable outcome 2]
- [ ] [How we know we're done]

## System Map

[High-level view of modules and their relationships]

- **[module-a]** → depends on → **[module-b]**
- **[module-c]** — standalone

## Milestones

### M1: [Milestone Name]
- [What's included]
- Target: [date or modules/features]

### M2: [Milestone Name]
- [What's included]
- Target: [date or modules/features]

## Modules

### [module-id]

- **Path:** [./modules/[module-name].aps.md](./modules/[module-name].aps.md)
- **Scope:** [SCOPE]
- **Owner:** @[username]
- **Status:** Draft
- **Priority:** [low|medium|high]
- **Tags:** [tag1, tag2]
- **Dependencies:** [other-module-id]

### [another-module-id]

- **Path:** [./modules/[another-module].aps.md](./modules/[another-module].aps.md)
- **Scope:** [SCOPE2]
- **Owner:** @[username]
- **Status:** Draft
- **Priority:** [low|medium|high]
- **Tags:** [tag1, tag2]
- **Dependencies:** (none)

## Decisions

- **D-001:** [Short decision] — [rationale] ([ADR-001](./decisions/ADR-001.md))
- **D-002:** [Another decision] — [rationale]

## Open Questions

- [Unresolved question 1]
- [Unresolved question 2]
`;
}

/**
 * Full index template - comprehensive for enterprise plans
 */
function generateFullIndexTemplate(): string {
  return `# APS Index — [Project Name]

## Problem & Success Criteria

**Problem:** [What problem are we solving? Why does this work matter?]

**Success Criteria:**
- [ ] [Measurable outcome 1]
- [ ] [Measurable outcome 2]
- [ ] [How we know we're done]

## Scope

**In Scope:**
- [What this plan covers]
- [Boundaries of work]

**Out of Scope:**
- [What this plan explicitly excludes]
- [Things deferred to future work]

## System Map

[High-level view of modules and their relationships]

\`\`\`
[Module A] ──→ [Module B] ──→ [Module C]
     ↑              ↓
[External Service]  [Database]
\`\`\`

## Milestones

### M1: [Milestone Name]
- [What's included]
- Modules: [module-a, module-b]
- Target: [date]

### M2: [Milestone Name]
- [What's included]
- Modules: [module-c]
- Target: [date]

## Modules

### [module-id]

- **Path:** [./modules/[module-name].aps.md](./modules/[module-name].aps.md)
- **Scope:** [SCOPE]
- **Owner:** @[username]
- **Status:** Draft
- **Priority:** [low|medium|high]
- **Tags:** [tag1, tag2]
- **Dependencies:** [other-module-id]

### [another-module-id]

- **Path:** [./modules/[another-module].aps.md](./modules/[another-module].aps.md)
- **Scope:** [SCOPE2]
- **Owner:** @[username]
- **Status:** Draft
- **Priority:** [low|medium|high]
- **Tags:** [tag1, tag2]
- **Dependencies:** (none)

## Epics

### [epic-id]

- **Path:** [./epics/[epic-name].aps.md](./epics/[epic-name].aps.md)
- **Owner:** @[username]
- **Modules:** [module-id-1, module-id-2]
- **Milestone:** M1

## Decisions

- **D-001:** [Short decision] — [rationale] ([ADR-001](./decisions/ADR-001.md))
- **D-002:** [Another decision] — [rationale]

## Risks

- **R-001:** [Risk description] — Mitigation: [approach]
- **R-002:** [Risk description] — Mitigation: [approach]

## Open Questions

- [Unresolved question 1]
- [Unresolved question 2]
`;
}

// ============================================================================
// Leaf Templates
// ============================================================================

/**
 * Minimal leaf template - tasks only
 */
function generateMinimalLeafTemplate(): string {
  return `# [Module Title]

**Scope:** [SCOPE] **Owner:** @[username]

## Tasks

### [SCOPE]-001: [Task title]

**Intent:** [What this task aims to achieve]
**Confidence:** [low|medium|high]

### [SCOPE]-002: [Another task]

**Intent:** [What this task does]
**Confidence:** [low|medium|high]
**Dependencies:** [SCOPE]-001
`;
}

/**
 * Standard leaf template - recommended for most modules
 */
function generateStandardLeafTemplate(): string {
  return `# [Module Title]

**Scope:** [SCOPE] **Owner:** @[username] **Priority:** [low|medium|high]

## Purpose

[Why this module exists and what problem it solves]

## In Scope / Out of Scope

**In Scope:**
- [What this module WILL do]
- [Boundaries of responsibility]

**Out of Scope:**
- [What this module will NOT do]
- [Things that belong elsewhere]

## Interfaces

**Depends on:**
- [Service/Module name] — [what we need from it]

**Exposes:**
- [Endpoint/API] — [what others can use]

## Tasks

### [SCOPE]-001: [Task title]

**Intent:** [Clear statement of what this task aims to achieve]
**Expected Outcome:** [What success looks like]
**Confidence:** [low|medium|high]
**Link:** [PROJ-123](https://jira.example.com/browse/PROJ-123)
**Scopes:** [SCOPE1, SCOPE2]
**Tags:** [tag1, tag2, tag3]
**Dependencies:** [SCOPE-XXX, OTHER-YYY]
**Inputs:**
- [Required input 1]
- [Required input 2]

### [SCOPE]-002: [Another task]

**Intent:** [What this task does]
**Confidence:** [low|medium|high]
**Scopes:** [SCOPE]
**Dependencies:** [SCOPE]-001

## Decisions

- **D-001:** [Short decision] — [rationale]

## Notes

- [Additional context or considerations]
`;
}

/**
 * Full leaf template - comprehensive for complex modules
 */
function generateFullLeafTemplate(): string {
  return `# Module APS — [Module Name]

**Scope:** [SCOPE] **Owner:** @[username] **Priority:** [low|medium|high]

## Purpose

[Why this module exists and what problem it solves. The "why" behind this work.]

## In Scope / Out of Scope

**In Scope:**
- [What this module WILL do]
- [Boundaries of responsibility]
- [Features included]

**Out of Scope:**
- [What this module will NOT do]
- [Things that belong elsewhere]
- [Explicit exclusions]

## Assumptions

- [Assumption 1] — Confidence: [low|medium|high]
- [Assumption 2] — Confidence: [low|medium|high]

## Interfaces

**Depends on:**
- [Service/Module name] — [what we need from it]
- [External API] — [what we consume]

**Exposes:**
- [Endpoint/API] — [what others can use]
- [Event/Hook] — [what we publish]

## Tasks

### [SCOPE]-001: [Task title]

**Intent:** [Clear statement of what this task aims to achieve]
**Expected Outcome:** [What success looks like]
**Confidence:** [low|medium|high]
**Link:** [PROJ-123](https://jira.example.com/browse/PROJ-123)
**Scopes:** [SCOPE1, SCOPE2]
**Tags:** [tag1, tag2, tag3]
**Dependencies:** [SCOPE-XXX, OTHER-YYY]
**Inputs:**
- [Required input 1]
- [Required input 2]

### [SCOPE]-002: [Another task]

**Intent:** [What this task does]
**Expected Outcome:** [Success criteria]
**Confidence:** [low|medium|high]
**Link:** [PROJ-124](https://jira.example.com/browse/PROJ-124)
**Scopes:** [SCOPE]
**Dependencies:** [SCOPE]-001

## Decisions

- **D-001:** [Short decision] — [rationale] ([ADR-001](../decisions/ADR-001.md))
- **D-002:** [Another decision] — [rationale]

## Risks

- **R-001:** [Risk description] — Mitigation: [approach]

## Open Questions

- [Unresolved question about this module]

## Notes

- [Additional context or considerations]
- [Links to relevant resources]
`;
}

// ============================================================================
// Simple (Single-File) Templates
// ============================================================================

/**
 * Minimal simple template - quick feature plan
 */
function generateMinimalSimpleTemplate(): string {
  return `# [Feature Name]

**Scope:** [SCOPE] **Owner:** @[username]

## Tasks

### [SCOPE]-001: [First task]

**Intent:** [What this task achieves]
**Confidence:** [low|medium|high]

### [SCOPE]-002: [Second task]

**Intent:** [What this task achieves]
**Confidence:** [low|medium|high]
**Dependencies:** [SCOPE]-001
`;
}

/**
 * Standard simple template - recommended for single-file plans
 */
function generateStandardSimpleTemplate(): string {
  return `# Feature: [Feature Name]

**Scope:** [SCOPE] **Owner:** @[username] **Priority:** [low|medium|high]

## Purpose

[Why we're building this feature and what problem it solves]

## Success Criteria

- [ ] [Measurable outcome 1]
- [ ] [Measurable outcome 2]

## Tasks

### [SCOPE]-001: [First task]

**Intent:** [What this task achieves]
**Expected Outcome:** [Success criteria]
**Confidence:** [low|medium|high]
**Link:** [PROJ-123](https://jira.example.com/browse/PROJ-123)
**Scopes:** [SCOPE]
**Tags:** [tag1, tag2]

### [SCOPE]-002: [Second task]

**Intent:** [What this task achieves]
**Confidence:** [low|medium|high]
**Scopes:** [SCOPE]
**Dependencies:** [SCOPE]-001

## Notes

- [Additional notes or considerations]
`;
}

/**
 * Full simple template - comprehensive single-file plan
 */
function generateFullSimpleTemplate(): string {
  return `# Feature: [Feature Name]

**Scope:** [SCOPE] **Owner:** @[username] **Priority:** [low|medium|high]

## Purpose

[Why we're building this feature and what problem it solves]

## Success Criteria

- [ ] [Measurable outcome 1]
- [ ] [Measurable outcome 2]
- [ ] [How we know we're done]

## In Scope / Out of Scope

**In Scope:**
- [What this feature WILL do]

**Out of Scope:**
- [What this feature will NOT do]

## Assumptions

- [Assumption 1] — Confidence: [low|medium|high]

## Tasks

### [SCOPE]-001: [First task]

**Intent:** [What this task achieves]
**Expected Outcome:** [Success criteria]
**Confidence:** [low|medium|high]
**Link:** [PROJ-123](https://jira.example.com/browse/PROJ-123)
**Scopes:** [SCOPE]
**Tags:** [tag1, tag2]
**Inputs:**
- [Required input 1]

### [SCOPE]-002: [Second task]

**Intent:** [What this task achieves]
**Expected Outcome:** [Success criteria]
**Confidence:** [low|medium|high]
**Scopes:** [SCOPE]
**Dependencies:** [SCOPE]-001

## Decisions

- **D-001:** [Decision] — [rationale]

## Open Questions

- [Unresolved question]

## Notes

- [Additional notes or considerations]
`;
}

// ============================================================================
// Action Plan Templates
// ============================================================================

/**
 * Minimal action plan template - checkpoints only
 */
function generateMinimalActionsTemplate(): string {
  return `# Actions: [SCOPE-NNN]

| Source | Work Item | Created by | Status |
|--------|-----------|------------|--------|
| [module.aps.md](./module.aps.md) | [SCOPE-NNN]: [Title] | @[username] | In Progress |

## Actions

### 1. [Action verb] [target]

- **Checkpoint:** [Observable state — max 12 words]
- **Validate:** \`[command]\`

### 2. [Next action]

- **Checkpoint:** [Observable state]
- **Validate:** \`[command]\`

## Completion

- [ ] All checkpoints validated
- [ ] Work item marked complete
`;
}

/**
 * Standard action plan template - recommended for most tasks
 */
function generateStandardActionsTemplate(): string {
  return `# Actions: [SCOPE-NNN]

| Source | Work Item | Created by | Status |
|--------|-----------|------------|--------|
| [module.aps.md](./module.aps.md) | [SCOPE-NNN]: [Title] | @[username] | In Progress |

## Prerequisites

- [ ] Dependencies completed: [list any prerequisite work items]
- [ ] Decisions made: [list any decisions needed]
- [ ] Context available: [list any required inputs]

## Actions

### 1. [Action verb] [target]

- **Purpose:** [Why this action is needed]
- **Produces:** [What this action creates or changes]
- **Checkpoint:** [Observable state — max 12 words]
- **Validate:** \`[command]\`

### 2. [Next action]

- **Purpose:** [Why this action is needed]
- **Produces:** [What this action creates or changes]
- **Checkpoint:** [Observable state — max 12 words]
- **Validate:** \`[command]\`

### 3. [Final action]

- **Purpose:** [Why this action is needed]
- **Produces:** [What this action creates or changes]
- **Checkpoint:** [Observable state — max 12 words]
- **Validate:** \`[command]\`

## Completion

- [ ] All checkpoints validated
- [ ] Work item marked complete
- [ ] Completed by: @[username]
- [ ] Completed at: [timestamp]
`;
}

/**
 * Full action plan template - comprehensive for complex tasks
 */
function generateFullActionsTemplate(): string {
  return `# Actions: [SCOPE-NNN]

| Source | Work Item | Created by | Status |
|--------|-----------|------------|--------|
| [module.aps.md](./module.aps.md) | [SCOPE-NNN]: [Title] | @[username] | In Progress |

## Overview

**Intent:** [Copy from work item — what this achieves]
**Expected Outcome:** [Copy from work item — success criteria]

## Prerequisites

- [ ] Dependencies completed: [list any prerequisite work items]
- [ ] Decisions made: [list any decisions needed]
- [ ] Context available: [list any required inputs]
- [ ] Environment ready: [list any setup requirements]

## Actions

### 1. [Action verb] [target]

- **Purpose:** [Why this action is needed]
- **Produces:** [What this action creates or changes]
- **Checkpoint:** [Observable state — max 12 words]
- **Validate:** \`[command]\`
- **Status:** [Blocked/Deferred — only if applicable]

### 2. [Next action]

- **Purpose:** [Why this action is needed]
- **Produces:** [What this action creates or changes]
- **Checkpoint:** [Observable state — max 12 words]
- **Validate:** \`[command]\`

### 3. [Verification action]

- **Purpose:** [Why this action is needed]
- **Produces:** [What this action creates or changes]
- **Checkpoint:** [Observable state — max 12 words]
- **Validate:** \`[command]\`

## Blocked/Deferred

[Document any actions that are blocked or deferred, with reasons]

## Notes

- [Additional context or considerations]
- [Links to relevant resources]

## Completion

- [ ] All checkpoints validated
- [ ] Tests pass: \`[test command]\`
- [ ] Work item marked complete
- [ ] Completed by: @[username]
- [ ] Completed at: [timestamp]
`;
}

// ============================================================================
// Public API
// ============================================================================

/**
 * Generate an index file template
 *
 * @param options - Template options
 * @returns Index template markdown
 */
export function generateIndexTemplate(options: TemplateOptions = {}): string {
  const variant = options.variant ?? 'standard';

  switch (variant) {
    case 'minimal':
      return generateMinimalIndexTemplate();
    case 'full':
      return generateFullIndexTemplate();
    case 'standard':
    default:
      return generateStandardIndexTemplate();
  }
}

/**
 * Generate a leaf spec template
 *
 * @param options - Template options
 * @returns Leaf spec template markdown
 */
export function generateLeafTemplate(options: TemplateOptions = {}): string {
  const variant = options.variant ?? 'standard';

  switch (variant) {
    case 'minimal':
      return generateMinimalLeafTemplate();
    case 'full':
      return generateFullLeafTemplate();
    case 'standard':
    default:
      return generateStandardLeafTemplate();
  }
}

/**
 * Generate a simple single-file plan template
 *
 * @param options - Template options
 * @returns Simple plan template markdown
 */
export function generateSimplePlanTemplate(options: TemplateOptions = {}): string {
  const variant = options.variant ?? 'standard';

  switch (variant) {
    case 'minimal':
      return generateMinimalSimpleTemplate();
    case 'full':
      return generateFullSimpleTemplate();
    case 'standard':
    default:
      return generateStandardSimpleTemplate();
  }
}

/**
 * Generate an action plan template for execution breakdowns
 *
 * @param options - Template options
 * @returns Action plan template markdown
 */
export function generateActionsTemplate(options: TemplateOptions = {}): string {
  const variant = options.variant ?? 'standard';

  switch (variant) {
    case 'minimal':
      return generateMinimalActionsTemplate();
    case 'full':
      return generateFullActionsTemplate();
    case 'standard':
    default:
      return generateStandardActionsTemplate();
  }
}

/**
 * Generate all templates and return as a typed bundle
 *
 * @param options - Template options
 * @returns Bundle of all templates
 */
export function generateAllTemplates(options: TemplateOptions = {}): TemplateBundle {
  return {
    index: generateIndexTemplate(options),
    leaf: generateLeafTemplate(options),
    simple: generateSimplePlanTemplate(options),
    actions: generateActionsTemplate(options),
  };
}
