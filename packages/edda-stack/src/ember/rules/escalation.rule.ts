import { clampConfidence } from '../../contracts/confidence.js';
import type { EvaluationContext, EvaluationResult, EvaluationRule } from '../evaluator-service.js';
import type { ObservationGroup } from '../aggregator-service.js';

const SEVERITY_RANK: Record<string, number> = {
  low: 1,
  medium: 2,
  high: 3,
  critical: 4,
};

export class EscalationRule implements EvaluationRule {
  readonly name = 'escalation';
  readonly description = 'Detects severity increase over time in an observation group';

  constructor(readonly weight = 1.1) {}

  evaluate(group: ObservationGroup, context: EvaluationContext): EvaluationResult {
    const severityValues = this.extractSeverities(group.signals);
    if (severityValues.length < 2) {
      return { fired: false, contribution: 0 };
    }

    const sorted = [...severityValues].sort((a, b) => a - b);
    const lowestSeverity = sorted[0];
    const highestSeverity = sorted[sorted.length - 1];
    const escalationDelta = highestSeverity - lowestSeverity;
    if (escalationDelta <= 0) {
      return { fired: false, contribution: 0 };
    }

    const spanHours =
      (new Date(group.latest).getTime() - new Date(group.earliest).getTime()) / (60 * 60 * 1000);
    if (spanHours > context.config.escalation_window_hours) {
      return { fired: false, contribution: 0 };
    }

    const spanFactor = clampConfidence(
      1 - spanHours / Math.max(1, context.config.escalation_window_hours)
    );
    const deltaFactor = clampConfidence(escalationDelta / 3);
    const contribution = clampConfidence(0.4 + deltaFactor * 0.4 + spanFactor * 0.2);

    return {
      fired: true,
      contribution,
      context: {
        escalation_delta: escalationDelta,
        span_hours: spanHours,
        suggested_type: 'warning',
      },
    };
  }

  private extractSeverities(signals: string[]): number[] {
    return signals
      .filter((signal) => signal.startsWith('severity_') || signal.startsWith('severity:'))
      .map((signal) => signal.replace('severity_', '').replace('severity:', ''))
      .map((severity) => SEVERITY_RANK[severity])
      .filter((severity): severity is number => severity !== undefined);
  }
}

export function createEscalationRule(options?: { weight?: number }): EscalationRule {
  return new EscalationRule(options?.weight ?? 1.1);
}
