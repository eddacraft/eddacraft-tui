import { clampConfidence } from '../../contracts/confidence.js';
import type { EvaluationContext, EvaluationResult, EvaluationRule } from '../evaluator-service.js';
import type { ObservationGroup } from '../aggregator-service.js';

/**
 * Signal strings produced by {@link AggregatorService.groupByKind} - each
 * observation kind is prefixed with `kind_` to form its signal name.
 * Update this set when new observation kinds are added that should be
 * treated as anomalous.
 */
const UNEXPECTED_KIND_SIGNALS = new Set(['kind_custom', 'kind_metric_recorded']);

export class SurpriseRule implements EvaluationRule {
  readonly name = 'surprise';
  readonly description = 'Detects anomalous observation kinds or unusual timing behaviour';

  constructor(readonly weight = 1.0) {}

  evaluate(group: ObservationGroup, _context: EvaluationContext): EvaluationResult {
    const anomalousKinds = group.signals.filter((signal) => UNEXPECTED_KIND_SIGNALS.has(signal));
    const durationMs = new Date(group.latest).getTime() - new Date(group.earliest).getTime();
    const cadenceMs = group.count > 1 ? durationMs / (group.count - 1) : durationMs;

    const burstyPattern = group.count >= 4 && cadenceMs > 0 && cadenceMs < 30_000;
    const sparsePattern = group.count >= 3 && cadenceMs > 6 * 60 * 60 * 1000;
    const fired = anomalousKinds.length > 0 || burstyPattern || sparsePattern;

    if (!fired) {
      return { fired: false, contribution: 0 };
    }

    const kindFactor = clampConfidence(anomalousKinds.length / 2);
    const timingFactor = burstyPattern || sparsePattern ? 0.45 : 0;
    const contribution = clampConfidence(0.35 + kindFactor * 0.35 + timingFactor);

    return {
      fired: true,
      contribution,
      context: {
        anomalous_kinds: anomalousKinds,
        cadence_ms: cadenceMs,
        bursty_pattern: burstyPattern,
        sparse_pattern: sparsePattern,
        suggested_type: 'anomaly',
      },
    };
  }
}

export function createSurpriseRule(options?: { weight?: number }): SurpriseRule {
  return new SurpriseRule(options?.weight ?? 1.0);
}
