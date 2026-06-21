export type {
  ExplanationSection,
  WarningExplanation,
  ExplanationContext,
  ExplanationTemplate,
} from './types.js';

export {
  ExplanationSectionSchema,
  WarningExplanationSchema,
  ExplanationContextSchema,
} from './types.js';

export {
  registerTemplate,
  getTemplate,
  hasTemplate,
  getRegisteredRuleIds,
  renderExplanation,
  clearTemplates,
  createGenericExplanation,
} from './template-loader.js';

// Anti-pattern explainer archived under ADR-033 (2026-04-29)
// → anvil-archive/anvil-ts-scanner/core-explain-antipattern.ts.

export {
  registerBoundaryTemplates,
  getBoundaryExplanation,
  isArchitectureRule,
} from './boundary-explainer.js';

export {
  resetExplainService,
  initExplainService,
  explainWarning,
  explainById,
  explainByRule,
  listWarnings,
  isExplainable,
  getExplainableRules,
  type ListWarningsResult,
} from './explain-service.js';
