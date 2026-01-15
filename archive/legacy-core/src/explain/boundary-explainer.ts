import type { ExplanationTemplate, ExplanationContext, WarningExplanation } from './types.js';
import { registerTemplate } from './template-loader.js';

interface ArchRuleInfo {
  id: string;
  name: string;
  description: string;
}

const ARCH_RULES: Record<string, ArchRuleInfo> = {
  'ARCH-001': {
    id: 'ARCH-001',
    name: 'Circular dependency',
    description: 'A circular dependency chain was detected in imports.',
  },
  'ARCH-002': {
    id: 'ARCH-002',
    name: 'Orphan module',
    description: 'This module is not imported by any other module.',
  },
  'ARCH-003': {
    id: 'ARCH-003',
    name: 'Layer boundary violation',
    description: 'An import crosses an architectural layer boundary.',
  },
  'ARCH-004': {
    id: 'ARCH-004',
    name: 'Architecture violation',
    description: 'General architecture rule violation detected.',
  },
  'BOUND-001': {
    id: 'BOUND-001',
    name: 'Boundary violation',
    description: 'A new cross-boundary dependency was detected.',
  },
};

function createBoundaryExplanation(
  ruleId: string,
  context: ExplanationContext
): WarningExplanation {
  const ruleInfo = ARCH_RULES[ruleId];
  if (!ruleInfo) {
    return createFallbackExplanation(ruleId, context);
  }

  const locationSummary = formatLocationSummary(ruleId, context);

  return {
    ruleId,
    title: ruleInfo.name,
    summary: locationSummary,
    whyItMatters: {
      title: 'WHY THIS WARNING EXISTS',
      content: getWhyContent(ruleId, context),
    },
    howToAddress: {
      title: 'HOW TO ADDRESS',
      content: getHowContent(ruleId, context),
    },
    whenToSuppress: {
      title: 'WHEN TO SUPPRESS',
      content: getWhenToSuppressContent(ruleId),
    },
    related: {
      ruleDefinition: `${ruleId} in architecture rules`,
      similarWarnings: context.similarCount,
    },
  };
}

function formatLocationSummary(ruleId: string, context: ExplanationContext): string {
  if (context.fromFile && context.toFile) {
    return `${context.fromFile} → ${context.toFile}`;
  }
  return `${ruleId} at ${context.file}:${context.line}`;
}

function getWhyContent(ruleId: string, context: ExplanationContext): string {
  const layerContext =
    context.fromLayer && context.toLayer
      ? `\n\nIn this case, ${context.fromLayer} is importing from ${context.toLayer}.`
      : '';

  const whyContent: Record<string, string> = {
    'ARCH-001': `
Circular dependencies create tightly coupled code that is:

- Difficult to test in isolation
- Hard to understand (dependencies loop back)
- Prone to initialisation order bugs
- Resistant to refactoring

The dependency chain forms a cycle, meaning module A depends on B,
which depends on C, which depends back on A.${layerContext}`,

    'ARCH-002': `
Orphan modules are files that no other code imports. This may indicate:

- Dead code that should be removed
- Missing integration (forgot to wire it up)
- Incorrect file organisation
- A utility that lost its callers after refactoring

While not always a problem, orphans often represent unused code
that adds maintenance burden.`,

    'ARCH-003': `
This import violates defined layer boundaries. Layer violations:

- Break architectural contracts
- Create hidden dependencies
- Make changes risky (unknown blast radius)
- Complicate testing and deployment${layerContext}`,

    'ARCH-004': `
This code violates an architecture rule defined for your project.

Architecture rules exist to:
- Enforce separation of concerns
- Prevent coupling between unrelated modules
- Maintain clear dependency direction
- Enable independent testing and deployment${layerContext}`,

    'BOUND-001': `
A NEW cross-boundary dependency was introduced. This means code
that previously didn't exist is now crossing an architectural boundary.

New boundary violations are significant because:
- They indicate potential architecture drift
- The change was made without considering boundaries
- They may create precedent for further violations${layerContext}`,
  };

  return (
    whyContent[ruleId] ?? ARCH_RULES[ruleId]?.description ?? 'Architecture violation detected.'
  );
}

function getHowContent(ruleId: string, context: ExplanationContext): string {
  const howContent: Record<string, string> = {
    'ARCH-001': `
1. Identify the cycle
   Look at the import chain to understand how modules connect

2. Break the cycle using one of these patterns:
   - Extract shared code to a common module
   - Use dependency injection
   - Introduce an interface/abstraction layer
   - Merge tightly coupled modules

3. Consider if the modules should be combined
   Sometimes circular deps indicate a single concept split artificially`,

    'ARCH-002': `
1. Verify the module is actually unused
   Search for dynamic imports or test files that might use it

2. If unused, consider removing it
   Dead code adds maintenance burden

3. If it should be used, add the missing integration
   Wire it into the appropriate consumer

4. If it's a utility, document its purpose
   Make sure others know it exists and when to use it`,

    'ARCH-003': `
1. Understand the layer structure
   Review which layer should depend on which

2. Consider using an intermediary
   If presentation needs data, go through application layer

3. Move the code to the appropriate layer
   Sometimes code is in the wrong place

4. Use dependency inversion
   Depend on abstractions, not concretions`,

    'ARCH-004': `
1. Review the architecture rule being violated
   Understand why the rule exists

2. Determine if the violation is intentional
   Sometimes rules need exceptions

3. Refactor to comply with the rule
   Or propose a rule change if it's outdated`,

    'BOUND-001': `
1. Understand what boundary was crossed
   ${context.fromLayer ? `From: ${context.fromLayer}` : ''}
   ${context.toLayer ? `To: ${context.toLayer}` : ''}

2. Consider the proper path
   Is there an existing service or abstraction to use?

3. If the boundary is wrong, update architecture
   Boundaries should reflect actual needs

4. If crossing is necessary, suppress with explanation
   Document why this exception exists`,
  };

  return howContent[ruleId] ?? 'Review the architecture and consider refactoring.';
}

function getWhenToSuppressContent(ruleId: string): string {
  const suppressionContent: Record<string, string> = {
    'ARCH-001': `
Suppress only if:
- The cycle is intentional and well-understood
- Breaking it would require major refactoring with a planned ticket
- The modules are conceptually a single unit

Example:
// @anvil-ignore ARCH-001: mutually recursive parsers, by design`,

    'ARCH-002': `
Suppress only if:
- The module is a valid entry point (e.g., CLI, test setup)
- It's dynamically imported in ways not detected
- It's intentionally a standalone utility

Example:
// @anvil-ignore ARCH-002: CLI entry point, not imported`,

    'ARCH-003': `
Suppress only if:
- The layer structure doesn't fit this use case
- You're actively migrating and have a ticket
- This is a pragmatic exception with clear reasoning

Example:
// @anvil-ignore ARCH-003: legacy code, migrating in JIRA-789`,

    'ARCH-004': `
Suppress only if:
- The rule is overly strict for this case
- A rule change is pending
- This is a documented exception

Example:
// @anvil-ignore ARCH-004: approved exception per ADR-042`,

    'BOUND-001': `
Suppress only if:
- The new dependency is intentional and reviewed
- The architecture definition needs updating
- This is a temporary bridge during migration

Example:
// @anvil-ignore BOUND-001: approved in PR review, updating baseline`,
  };

  return (
    suppressionContent[ruleId] ??
    `
Suppress only with clear justification.
Use: // @anvil-ignore ${ruleId}: [your reason]`
  );
}

function createFallbackExplanation(
  ruleId: string,
  context: ExplanationContext
): WarningExplanation {
  return {
    ruleId,
    title: `Architecture violation ${ruleId}`,
    summary: `Violation at ${context.file}:${context.line}`,
    whyItMatters: {
      title: 'WHY THIS WARNING EXISTS',
      content: 'This warning indicates an architecture boundary violation.',
    },
    howToAddress: {
      title: 'HOW TO ADDRESS',
      content: 'Review the architecture and consider refactoring.',
    },
    whenToSuppress: {
      title: 'WHEN TO SUPPRESS',
      content: `Use: // @anvil-ignore ${ruleId}: [reason]`,
    },
  };
}

export function registerBoundaryTemplates(): void {
  for (const ruleId of Object.keys(ARCH_RULES)) {
    const template: ExplanationTemplate = {
      ruleId,
      render: (context: ExplanationContext) => createBoundaryExplanation(ruleId, context),
    };
    registerTemplate(template);
  }
}

export function getBoundaryExplanation(
  ruleId: string,
  context: ExplanationContext
): WarningExplanation | null {
  if (!ARCH_RULES[ruleId]) {
    return null;
  }
  return createBoundaryExplanation(ruleId, context);
}

export function isArchitectureRule(ruleId: string): boolean {
  return ruleId in ARCH_RULES;
}
