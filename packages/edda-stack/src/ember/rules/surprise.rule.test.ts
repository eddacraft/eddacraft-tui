import { describe, expect, it } from 'vitest';
import { createSurpriseRule } from './surprise.rule.js';
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

function createGroup(overrides: Partial<ObservationGroup> = {}): ObservationGroup {
  return {
    id: 'g-surprise',
    grouping_type: 'pattern',
    observation_ids: ['obs-1', 'obs-2', 'obs-3', 'obs-4'],
    session_ids: ['session-1'],
    earliest: '2026-01-01T10:00:00.000Z',
    latest: '2026-01-01T10:01:00.000Z',
    count: 4,
    suggested_type: 'anomaly',
    signals: [],
    ...overrides,
  };
}

describe('SurpriseRule', () => {
  it('fires on anomalous observation kind signals', () => {
    const rule = createSurpriseRule();
    const result = rule.evaluate(createGroup({ signals: ['kind_custom'] }), context);

    expect(result.fired).toBe(true);
    expect(result.context?.suggested_type).toBe('anomaly');
  });

  it('does not fire when no anomaly criteria are met', () => {
    const rule = createSurpriseRule();
    const result = rule.evaluate(
      createGroup({
        count: 2,
        observation_ids: ['obs-1', 'obs-2'],
        earliest: '2026-01-01T10:00:00.000Z',
        latest: '2026-01-01T12:00:00.000Z',
      }),
      context
    );

    expect(result.fired).toBe(false);
  });

  it('fires for bursty timing patterns', () => {
    const rule = createSurpriseRule();
    const result = rule.evaluate(
      createGroup({
        earliest: '2026-01-01T10:00:00.000Z',
        latest: '2026-01-01T10:00:20.000Z',
        count: 5,
        observation_ids: ['obs-1', 'obs-2', 'obs-3', 'obs-4', 'obs-5'],
      }),
      context
    );

    expect(result.fired).toBe(true);
    expect(result.contribution).toBeGreaterThan(0.4);
  });

  it('handles single observation edge case', () => {
    const rule = createSurpriseRule();
    const result = rule.evaluate(
      createGroup({
        count: 1,
        observation_ids: ['obs-1'],
        earliest: '2026-01-01T10:00:00.000Z',
        latest: '2026-01-01T10:00:00.000Z',
      }),
      context
    );

    expect(result.fired).toBe(false);
  });
});
