import { clampConfidence } from '../../contracts/confidence.js';
import type { EvaluationContext, EvaluationResult, EvaluationRule } from '../evaluator-service.js';
import type { ObservationGroup } from '../aggregator-service.js';

export class ConvergenceRule implements EvaluationRule {
  readonly name = 'convergence';
  readonly description = 'Detects multiple sessions converging on a shared pattern';

  constructor(readonly weight = 0.9) {}

  evaluate(group: ObservationGroup, _context: EvaluationContext): EvaluationResult {
    const uniqueSessions = new Set(group.session_ids).size;
    if (uniqueSessions < 2) {
      return { fired: false, contribution: 0 };
    }

    const scaled = Math.log1p(uniqueSessions - 1) / Math.log1p(6);
    const contribution = clampConfidence(0.4 + scaled * 0.6);

    return {
      fired: true,
      contribution,
      context: {
        session_count: uniqueSessions,
        suggested_type: 'decision',
      },
    };
  }
}

export function createConvergenceRule(options?: { weight?: number }): ConvergenceRule {
  return new ConvergenceRule(options?.weight ?? 0.9);
}
