// Configuration
export * from './config.js';

// Storage
export { ProposalStore } from './proposal-store.js';
export { EmberQueryApi } from './query-api.js';
export type { ProposalWithContext, EmberSummaryStats } from './query-api.js';

// Services
export { CandidateService } from './candidate-service.js';
export type { EmberServiceConfig } from './candidate-service.js';
export { DecayService } from './decay-service.js';
export type { DecayServiceConfig } from './decay-service.js';
export { AggregatorService } from './aggregator-service.js';
export type { ObservationGroup } from './aggregator-service.js';
export { EvaluatorService } from './evaluator-service.js';
export type {
  EvaluationRule,
  EvaluationContext,
  EmberEvaluationConfig,
  EvaluationResult,
  EvaluationOutcome,
} from './evaluator-service.js';

// Observation hooks
export { ObservationHook } from './observation-hook.js';
export type { ObservationHookDeps } from './observation-hook.js';

// Built-in rules
export {
  createDefaultRules,
  RepetitionRule,
  EscalationRule,
  ResolutionRule,
  ConvergenceRule,
  SurpriseRule,
  createRepetitionRule,
  createEscalationRule,
  createResolutionRule,
  createConvergenceRule,
  createSurpriseRule,
} from './rules/index.js';
