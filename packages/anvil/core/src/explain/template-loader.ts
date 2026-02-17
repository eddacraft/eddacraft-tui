import type { ExplanationTemplate, ExplanationContext, WarningExplanation } from './types.js';
import { createDebugger } from '../utils/debug.js';

const debug = createDebugger('explain');

const templateRegistry = new Map<string, ExplanationTemplate>();

export function registerTemplate(template: ExplanationTemplate): void {
  debug('registering template', template.ruleId);
  templateRegistry.set(template.ruleId, template);
}

export function getTemplate(ruleId: string): ExplanationTemplate | undefined {
  return templateRegistry.get(ruleId);
}

export function hasTemplate(ruleId: string): boolean {
  return templateRegistry.has(ruleId);
}

export function getRegisteredRuleIds(): string[] {
  return Array.from(templateRegistry.keys());
}

export function renderExplanation(
  ruleId: string,
  context: ExplanationContext
): WarningExplanation | null {
  const template = templateRegistry.get(ruleId);
  if (!template) {
    debug('no template found for rule', ruleId);
    return null;
  }
  return template.render(context);
}

export function clearTemplates(): void {
  templateRegistry.clear();
}

export function createGenericExplanation(
  ruleId: string,
  title: string,
  context: ExplanationContext
): WarningExplanation {
  return {
    ruleId,
    title,
    summary: `Warning ${ruleId} detected at ${context.file}:${context.line}`,
    whyItMatters: {
      title: 'WHY THIS WARNING EXISTS',
      content:
        'This warning indicates a potential issue in your code. ' +
        'Please refer to the warning message for specific details.',
    },
    howToAddress: {
      title: 'HOW TO ADDRESS',
      content:
        'Review the flagged code and consider the warning message. ' +
        'Determine if changes are needed based on your project requirements.',
    },
    whenToSuppress: {
      title: 'WHEN TO SUPPRESS',
      content:
        'Suppress only if you understand the warning and have a valid reason. ' +
        'Use: // @anvil-ignore ' +
        ruleId +
        ': [your reason]',
    },
  };
}
