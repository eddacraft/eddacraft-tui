import type { ExplanationTemplate, ExplanationContext, WarningExplanation } from './types.js';
import { registerTemplate } from './template-loader.js';
import { PATTERNS, getPattern } from '../antipattern/patterns.js';

function createAntiPatternExplanation(
  ruleId: string,
  context: ExplanationContext
): WarningExplanation {
  const pattern = getPattern(ruleId);
  if (!pattern) {
    return createFallbackExplanation(ruleId, context);
  }

  return {
    ruleId,
    title: pattern.title,
    summary: `${pattern.name} detected at ${context.file}:${context.line}`,
    whyItMatters: {
      title: 'WHY THIS WARNING EXISTS',
      content: getWhyContent(ruleId, pattern.explanation),
    },
    howToAddress: {
      title: 'HOW TO ADDRESS',
      content: getHowContent(ruleId, pattern.suggestion),
    },
    whenToSuppress: {
      title: 'WHEN TO SUPPRESS',
      content: getWhenToSuppressContent(ruleId),
    },
    related: {
      ruleDefinition: `${ruleId} in anti-pattern catalogue`,
      similarWarnings: context.similarCount,
    },
  };
}

function getWhyContent(ruleId: string, baseExplanation: string): string {
  const additionalContext: Record<string, string> = {
    'AP-001': `
This is problematic because:

- Silences ALL linting errors in the affected scope
- Makes code review harder as issues are hidden
- Often masks multiple unrelated problems
- Accumulated disables create unmaintainable code`,

    'AP-003': `
The 'any' type disables TypeScript's type checking for this value.
This is problematic because:

- Errors that would be caught at compile time slip through
- IDE autocompletion and refactoring tools lose effectiveness
- The type unsafety spreads to everything that touches this value
- Refactoring becomes risky without type safety`,

    'AP-004': `
@ts-ignore suppresses ALL TypeScript errors on the next line.
This is dangerous because:

- It hides ALL type errors, not just the intended one
- New bugs introduced by changes won't be caught
- The underlying issue remains unfixed
- Code reviewers may miss the hidden problems`,

    'AP-006': `
Empty catch blocks silently swallow errors without handling them.
This makes debugging difficult because:

- Errors disappear without a trace
- Issues manifest far from their source
- Intermittent problems become impossible to diagnose
- Production issues go unnoticed`,
  };

  return additionalContext[ruleId] ?? baseExplanation;
}

function getHowContent(ruleId: string, baseSuggestion: string): string {
  const howToAddress: Record<string, string> = {
    'AP-001': `
1. Identify which specific rules are being violated
   Run ESLint to see the actual errors

2. Fix the underlying issues where possible
   Most ESLint errors have straightforward fixes

3. If rules must be disabled, be specific:
   /* eslint-disable @typescript-eslint/no-explicit-any */
   instead of blanket /* eslint-disable */

4. Add a comment explaining WHY the rule is disabled`,

    'AP-003': `
1. If you know the type, use it directly:
   function parse(data: RequestPayload) { ... }

2. If the type varies, use a union or generic:
   function parse<T>(data: T) { ... }

3. If truly unknown at compile time, use 'unknown' with type guards:
   function parse(data: unknown) {
     if (isRequestPayload(data)) { ... }
   }

4. For third-party types, consider @types packages or declaration files`,

    'AP-004': `
1. Prefer @ts-expect-error over @ts-ignore
   It fails when the expected error disappears

2. Fix the underlying type issue where possible
   Correct types are always better than suppressions

3. If suppression is necessary, add a description:
   // @ts-expect-error: third-party types incomplete

4. Create a ticket to properly fix the type issue`,

    'AP-006': `
1. At minimum, log the error for debugging:
   catch (error) { console.error('Operation failed:', error); }

2. Consider if the error should be re-thrown:
   catch (error) { logger.error(error); throw error; }

3. Implement specific recovery logic if appropriate:
   catch (error) { return fallbackValue; }

4. For intentional suppression, use an explicit comment:
   catch (_error) { /* Intentionally swallowed: ... */ }`,
  };

  return howToAddress[ruleId] ?? baseSuggestion;
}

function getWhenToSuppressContent(ruleId: string): string {
  const suppressionGuidance: Record<string, string> = {
    'AP-001': `
Suppress only if:
- You're in the process of migrating code and have a tracked ticket
- The ESLint rule genuinely doesn't apply to this codebase
- You've documented WHY in the suppression comment

Example:
// @anvil-ignore AP-001: migrating from JS, fixing in JIRA-456`,

    'AP-003': `
Suppress only if:
- Third-party library types are incorrect or missing
- Migration is in progress with a tracked ticket
- Type erasure is genuinely required (rare)

Example:
// @anvil-ignore AP-003: legacy API returns untyped JSON, fixing in JIRA-123`,

    'AP-004': `
Suppress only if:
- You're testing error conditions intentionally
- Third-party types are genuinely incorrect
- You have a plan to fix the underlying issue

Example:
// @anvil-ignore AP-004: testing invalid input handling`,

    'AP-006': `
Suppress only if:
- The error is truly expected and can be safely ignored
- You've verified no information is lost
- Alternative handling would add no value

Example:
// @anvil-ignore AP-006: optional cleanup, failure is acceptable`,
  };

  const defaultGuidance = `
Suppress only if you understand the warning and have a valid reason.
Always include an explanation in your suppression comment.

Syntax:
// @anvil-ignore ${ruleId}: [your reason here]`;

  return suppressionGuidance[ruleId] ?? defaultGuidance;
}

function createFallbackExplanation(
  ruleId: string,
  context: ExplanationContext
): WarningExplanation {
  return {
    ruleId,
    title: `Anti-pattern ${ruleId}`,
    summary: `Anti-pattern detected at ${context.file}:${context.line}`,
    whyItMatters: {
      title: 'WHY THIS WARNING EXISTS',
      content: 'This anti-pattern may indicate a code quality issue.',
    },
    howToAddress: {
      title: 'HOW TO ADDRESS',
      content: 'Review the flagged code and consider refactoring.',
    },
    whenToSuppress: {
      title: 'WHEN TO SUPPRESS',
      content: `Use: // @anvil-ignore ${ruleId}: [reason]`,
    },
  };
}

export function registerAntiPatternTemplates(): void {
  for (const pattern of PATTERNS) {
    const template: ExplanationTemplate = {
      ruleId: pattern.id,
      render: (context: ExplanationContext) => createAntiPatternExplanation(pattern.id, context),
    };
    registerTemplate(template);
  }
}

export function getAntiPatternExplanation(
  ruleId: string,
  context: ExplanationContext
): WarningExplanation | null {
  const pattern = getPattern(ruleId);
  if (!pattern) {
    return null;
  }
  return createAntiPatternExplanation(ruleId, context);
}
