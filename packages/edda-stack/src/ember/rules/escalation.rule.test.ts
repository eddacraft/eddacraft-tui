import { describe, expect, it } from 'vitest';
import { createEscalationRule } from './escalation.rule.js';
import type { EvaluationContext } from '../evaluator-service.js';
import type { ObservationGroup } from '../aggregator-service.js';

const context: EvaluationContext = {
  existingProposals: [],
  config: {
    min_confidence: 0.5,
    repetition_threshold: 3,
    escalation_window_hours: 6,
  },
};

function createGroup(signals: string[], latest = '2026-01-01T12:00:00.000Z'): ObservationGroup {
  return {
    id: 'g-escalation',
    grouping_type: 'pattern',
    observation_ids: ['obs-1', 'obs-2', 'obs-3'],
    session_ids: ['session-1'],
    earliest: '2026-01-01T10:00:00.000Z',
    latest,
    count: 3,
    suggested_type: 'warning',
    signals,
  };
}

describe('EscalationRule', () => {
  it('fires when severity increases within window', () => {
    const rule = createEscalationRule();
    const result = rule.evaluate(
      createGroup(['severity_low', 'severity_medium', 'severity_high']),
      context
    );

    expect(result.fired).toBe(true);
    expect(result.context?.suggested_type).toBe('warning');
  });

  it('does not fire when severity does not increase', () => {
    const rule = createEscalationRule();
    const result = rule.evaluate(createGroup(['severity_high', 'severity_high']), context);

    expect(result.fired).toBe(false);
  });

  it('detects escalation from out-of-order severity signals', () => {
    const rule = createEscalationRule();
    const result = rule.evaluate(
      createGroup(['severity_high', 'severity_low', 'severity_critical']),
      context
    );

    expect(result.fired).toBe(true);
    expect(result.context?.escalation_delta).toBe(3);
  });

  it('scales contribution for stronger escalation deltas', () => {
    const rule = createEscalationRule();
    const mild = rule.evaluate(createGroup(['severity_low', 'severity_medium']), context);
    const strong = rule.evaluate(createGroup(['severity_low', 'severity_critical']), context);

    expect(strong.fired).toBe(true);
    expect(strong.contribution).toBeGreaterThan(mild.contribution);
  });

  it('does not fire when outside escalation window', () => {
    const rule = createEscalationRule();
    const result = rule.evaluate(
      createGroup(['severity_low', 'severity_high'], '2026-01-02T20:00:00.000Z'),
      context
    );

    expect(result.fired).toBe(false);
  });
});
