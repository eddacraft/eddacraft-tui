/**
 * Architecture-rule explain service.
 *
 * Public API surface:
 * - `explainWarning(warning, allWarnings?)` — always returns an
 *   explanation. Architecture and boundary rules use the registered
 *   templates; everything else (including retired anti-pattern rule
 *   IDs `AP-*`) falls through to a generic "potential issue"
 *   explanation. Use this when you have a `Warning` object in hand.
 * - `explainByRule(ruleId, context?)` — returns `null` for any rule
 *   that does not have a registered template. Architecture rules
 *   are explainable; retired anti-pattern rule IDs (`AP-*`) are not.
 * - `isExplainable(ruleId)` — `true` only for rules with a
 *   registered template (architecture / boundary).
 * - `getExplainableRules()` — returns the architecture/boundary rule
 *   IDs only.
 *
 * The TS anti-pattern explainer was archived under ADR-033
 * (2026-04-29) → `anvil-archive/anvil-ts-scanner/core-explain-antipattern.ts`.
 * The capability has not been reimplemented; the Rust scanner
 * publishes the canonical anti-pattern catalogue, and consumers
 * needing AP-* explanations should consult that catalogue rather
 * than this service.
 */
import type { Warning } from '../warnings/types.js';
import type { ExplanationContext, WarningExplanation } from './types.js';
import {
  hasTemplate,
  renderExplanation,
  createGenericExplanation,
  clearTemplates,
} from './template-loader.js';
import { registerBoundaryTemplates, isArchitectureRule } from './boundary-explainer.js';
import {
  parseWarningId,
  findWarningById,
  findWarningsByRule,
  getWarningIds,
} from '../warnings/warning-id.js';
import { createDebugger } from '../utils/debug.js';

const debug = createDebugger('explain');

let templatesInitialised = false;

function ensureTemplatesInitialised(): void {
  if (!templatesInitialised) {
    clearTemplates();
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
  debug('explaining warning', {
    id: warning.id,
    file: warning.location.file,
    line: warning.location.line,
  });
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
  debug('explaining by id', warningId);
  ensureTemplatesInitialised();

  const parsed = parseWarningId(warningId);
  if (!parsed) {
    debug('invalid warning id format', warningId);
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
  // Anti-pattern rule IDs (AP-NNN) archived under ADR-033; the Rust
  // scanner publishes the canonical catalogue. Architecture rules
  // remain explainable here.
  return ['ARCH-001', 'ARCH-002', 'ARCH-003', 'ARCH-004', 'BOUND-001'];
}
