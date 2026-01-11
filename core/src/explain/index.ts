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

export {
  registerAntiPatternTemplates,
  getAntiPatternExplanation,
} from './antipattern-explainer.js';

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
