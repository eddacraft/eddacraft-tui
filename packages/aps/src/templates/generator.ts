/**
 * Template generator for APS planning documents
 *
 * Generates Markdown templates for index files and leaf specs
 * based on the APS Planning Spec v0.1.
 */

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
}

/**
 * Generate an index file template
 */
export function generateIndexTemplate(): string {
  return `# [Plan Title]

## Overview

[Brief description of this plan and its goals]

## Modules

### [module-id]

- **Path:** [./modules/[module-name].aps.md](./modules/[module-name].aps.md)
- **Scope:** [SCOPE]
- **Owner:** @[username]
- **Priority:** [low|medium|high]
- **Tags:** [tag1, tag2]
- **Dependencies:** [other-module-id]

### [another-module-id]

- **Path:** [./modules/[another-module].aps.md](./modules/[another-module].aps.md)
- **Scope:** [SCOPE2]
- **Owner:** @[username]
- **Priority:** [low|medium|high]
- **Tags:** [tag1, tag2]
- **Dependencies:** (none)

## Open Questions

- [Unresolved question 1]
- [Unresolved question 2]

## Decisions

- [Decision] (decided [date])
`;
}

/**
 * Generate a leaf spec template
 */
export function generateLeafTemplate(): string {
  return `# [Module Title]

**Scope:** [SCOPE] **Owner:** @[username] **Priority:** [low|medium|high]

> [Optional: Brief module description]

## Tasks

### [SCOPE]-001: [Task title]

**Intent:** [Clear statement of what this task aims to achieve]
**Expected Outcome:** [What success looks like]
**Confidence:** [low|medium|high]
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

## Dependencies

- Depends on [module-name] for [reason]

## Notes

- [Additional context or considerations]
`;
}

/**
 * Generate a simple single-file plan template
 */
export function generateSimplePlanTemplate(): string {
  return `# Feature: [Feature Name]

**Scope:** [SCOPE] **Owner:** @[username] **Priority:** [low|medium|high]

> [Brief feature description]

## Tasks

### [SCOPE]-001: [First task]

**Intent:** [What this task achieves]
**Expected Outcome:** [Success criteria]
**Confidence:** [low|medium|high]
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
 * Generate all templates and return as a typed bundle
 */
export function generateAllTemplates(): TemplateBundle {
  return {
    index: generateIndexTemplate(),
    leaf: generateLeafTemplate(),
    simple: generateSimplePlanTemplate(),
  };
}
