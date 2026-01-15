import type { Warning } from '../antipattern/types.js';
import type { ExplanationContext, WarningExplanation } from './types.js';
import {
  hasTemplate,
  renderExplanation,
  createGenericExplanation,
  clearTemplates,
} from './template-loader.js';
import { registerAntiPatternTemplates } from './antipattern-explainer.js';
import { registerBoundaryTemplates, isArchitectureRule } from './boundary-explainer.js';
import {
  parseWarningId,
  findWarningById,
  findWarningsByRule,
  getWarningIds,
} from '../warnings/warning-id.js';
import { getPattern } from '../antipattern/patterns.js';

let templatesInitialised = false;

function ensureTemplatesInitialised(): void {
  if (!templatesInitialised) {
    clearTemplates();
    registerAntiPatternTemplates();
    registerBoundaryTemplates();
    templatesInitialised = true;
  }
}

export function resetExplainService(): void {
  clearTemplates();
  templatesInitialised = false;
}

export function initExplainService(): void {
  ensureTemplatesInitialised();
}

function buildContext(warning: Warning, allWarnings?: Warning[]): ExplanationContext {
  const context: ExplanationContext = {
    file: warning.location.file,
    line: warning.location.line,
    patternName: warning.pattern,
  };

  if (allWarnings) {
    const sameRuleWarnings = findWarningsByRule(allWarnings, warning.id);
    const sameFileWarnings = sameRuleWarnings.filter(
      (w) => w.location.file === warning.location.file
    );
    context.similarCount = sameFileWarnings.length - 1;
  }

  return context;
}

export function explainWarning(warning: Warning, allWarnings?: Warning[]): WarningExplanation {
  ensureTemplatesInitialised();

  const context = buildContext(warning, allWarnings);

  if (hasTemplate(warning.id)) {
    const explanation = renderExplanation(warning.id, context);
    if (explanation) {
      return explanation;
    }
  }

  return createGenericExplanation(warning.id, warning.title, context);
}

export function explainById(warningId: string, warnings: Warning[]): WarningExplanation | null {
  ensureTemplatesInitialised();

  const parsed = parseWarningId(warningId);
  if (!parsed) {
    return null;
  }

  const warning = findWarningById(warnings, warningId);
  if (!warning) {
    return null;
  }

  return explainWarning(warning, warnings);
}

export function explainByRule(
  ruleId: string,
  context?: Partial<ExplanationContext>
): WarningExplanation | null {
  ensureTemplatesInitialised();

  const fullContext: ExplanationContext = {
    file: context?.file ?? 'unknown',
    line: context?.line ?? 1,
    ...context,
  };

  if (hasTemplate(ruleId)) {
    return renderExplanation(ruleId, fullContext);
  }

  const pattern = getPattern(ruleId);
  if (pattern) {
    return renderExplanation(ruleId, fullContext);
  }

  if (isArchitectureRule(ruleId)) {
    return renderExplanation(ruleId, fullContext);
  }

  return null;
}

export interface ListWarningsResult {
  warningId: string;
  ruleId: string;
  file: string;
  line: number;
  title: string;
  severity: string;
}

export function listWarnings(warnings: Warning[]): ListWarningsResult[] {
  const warningIds = getWarningIds(warnings);

  return warnings.map((w, i) => ({
    warningId: warningIds[i],
    ruleId: w.id,
    file: w.location.file,
    line: w.location.line,
    title: w.title,
    severity: w.severity,
  }));
}

export function isExplainable(ruleId: string): boolean {
  ensureTemplatesInitialised();
  return hasTemplate(ruleId);
}

export function getExplainableRules(): string[] {
  ensureTemplatesInitialised();
  const antiPatternIds = ['AP-001', 'AP-002', 'AP-003', 'AP-004', 'AP-005', 'AP-006', 'AP-007'];
  const archIds = ['ARCH-001', 'ARCH-002', 'ARCH-003', 'ARCH-004', 'BOUND-001'];
  return [...antiPatternIds, ...archIds];
}
