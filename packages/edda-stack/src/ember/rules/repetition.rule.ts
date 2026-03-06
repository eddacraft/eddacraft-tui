import { clampConfidence } from '../../contracts/confidence.js';
import type { EvaluationContext, EvaluationResult, EvaluationRule } from '../evaluator-service.js';
import type { ObservationGroup } from '../aggregator-service.js';

export class RepetitionRule implements EvaluationRule {
  readonly name = 'repetition';
  readonly description = 'Detects repeated observation patterns that recur above threshold';

  constructor(readonly weight = 1.2) {}

  evaluate(group: ObservationGroup, context: EvaluationContext): EvaluationResult {
    const threshold = context.config.repetition_threshold;
    if (group.count < threshold) {
      return { fired: false, contribution: 0 };
    }

    const overThreshold = group.count - threshold + 1;
    const scaled = Math.log1p(overThreshold) / Math.log1p(Math.max(2, threshold * 3));
    const contribution = clampConfidence(0.35 + scaled * 0.65);

    return {
      fired: true,
      contribution,
      context: {
        count: group.count,
        threshold,
        over_threshold: overThreshold,
        suggested_type: 'pattern',
      },
    };
  }
}

export function createRepetitionRule(options?: { weight?: number }): RepetitionRule {
  return new RepetitionRule(options?.weight ?? 1.2);
}
