import { describe, expect, it } from 'vitest';
import { createResolutionRule } from './resolution.rule.js';
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

function createGroup(signals: string[], count = 4): ObservationGroup {
  return {
    id: 'g-resolution',
    grouping_type: 'pattern',
    observation_ids: Array.from({ length: count }, (_, index) => `obs-${index}`),
    session_ids: ['session-1'],
    earliest: '2026-01-01T10:00:00.000Z',
    latest: '2026-01-01T10:30:00.000Z',
    count,
    suggested_type: 'lesson',
    signals,
  };
}

describe('ResolutionRule', () => {
  it('fires when failure is followed by success signals', () => {
    const rule = createResolutionRule();
    const result = rule.evaluate(createGroup(['failure_signal', 'success_signal']), context);

    expect(result.fired).toBe(true);
    expect(result.context?.suggested_type).toBe('lesson');
  });

  it('does not fire when success signal is missing', () => {
    const rule = createResolutionRule();
    const result = rule.evaluate(createGroup(['failure_signal']), context);

    expect(result.fired).toBe(false);
  });

  it('increases contribution with denser groups', () => {
    const rule = createResolutionRule();
    const sparse = rule.evaluate(createGroup(['failure_signal', 'success_signal'], 2), context);
    const dense = rule.evaluate(createGroup(['failure_signal', 'success_signal'], 6), context);

    expect(dense.fired).toBe(true);
    expect(dense.contribution).toBeGreaterThan(sparse.contribution);
  });

  it('returns false for empty signal groups', () => {
    const rule = createResolutionRule();
    const result = rule.evaluate(createGroup([]), context);

    expect(result.fired).toBe(false);
  });
});
