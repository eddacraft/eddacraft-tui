import { clampConfidence } from '../../contracts/confidence.js';
import type { EvaluationContext, EvaluationResult, EvaluationRule } from '../evaluator-service.js';
import type { ObservationGroup } from '../aggregator-service.js';

export class ResolutionRule implements EvaluationRule {
  readonly name = 'resolution';
  readonly description = 'Detects problem-to-solution transitions in a group';

  constructor(readonly weight = 1.0) {}

  evaluate(group: ObservationGroup, _context: EvaluationContext): EvaluationResult {
    const hasFailure = group.signals.includes('failure_signal');
    const hasSuccess = group.signals.includes('success_signal');
    if (!hasFailure || !hasSuccess) {
      return { fired: false, contribution: 0 };
    }

    const durationHours =
      (new Date(group.latest).getTime() - new Date(group.earliest).getTime()) / (60 * 60 * 1000);
    const speedFactor = clampConfidence(1 - durationHours / 24);
    const densityFactor = clampConfidence(group.count / 6);
    const contribution = clampConfidence(0.45 + speedFactor * 0.25 + densityFactor * 0.3);

    return {
      fired: true,
      contribution,
      context: {
        has_failure: hasFailure,
        has_success: hasSuccess,
        duration_hours: durationHours,
        suggested_type: 'lesson',
      },
    };
  }
}

export function createResolutionRule(options?: { weight?: number }): ResolutionRule {
  return new ResolutionRule(options?.weight ?? 1.0);
}
