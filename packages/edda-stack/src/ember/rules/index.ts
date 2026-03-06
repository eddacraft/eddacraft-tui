import type { EvaluationRule } from '../evaluator-service.js';
import { createConvergenceRule, ConvergenceRule } from './convergence.rule.js';
import { createEscalationRule, EscalationRule } from './escalation.rule.js';
import { createRepetitionRule, RepetitionRule } from './repetition.rule.js';
import { createResolutionRule, ResolutionRule } from './resolution.rule.js';
import { createSurpriseRule, SurpriseRule } from './surprise.rule.js';

export {
  ConvergenceRule,
  EscalationRule,
  RepetitionRule,
  ResolutionRule,
  SurpriseRule,
  createConvergenceRule,
  createEscalationRule,
  createRepetitionRule,
  createResolutionRule,
  createSurpriseRule,
};

export function createDefaultRules(): EvaluationRule[] {
  return [
    createRepetitionRule(),
    createEscalationRule(),
    createResolutionRule(),
    createConvergenceRule(),
    createSurpriseRule(),
  ];
}
