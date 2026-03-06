import { describe, expect, it } from 'vitest';
import { createRepetitionRule } from './repetition.rule.js';
import type { EvaluationContext } from '../evaluator-service.js';
import type { ObservationGroup } from '../aggregator-service.js';

const context: EvaluationContext = {
  existingProposals: [],
  config: {
    min_confidence: 0.5,
    repetition_threshold: 3,
    escalation_window_hours: 4,
  },
};

function createGroup(count: number): ObservationGroup {
  return {
    id: 'g-repetition',
    grouping_type: 'pattern',
    observation_ids: Array.from({ length: count }, (_, index) => `obs-${index}`),
    session_ids: ['session-1'],
    earliest: '2026-01-01T10:00:00.000Z',
    latest: '2026-01-01T11:00:00.000Z',
    count,
    suggested_type: 'pattern',
    signals: [],
  };
}

describe('RepetitionRule', () => {
  it('fires when threshold is met', () => {
    const rule = createRepetitionRule();
    const result = rule.evaluate(createGroup(3), context);

    expect(result.fired).toBe(true);
    expect(result.context?.suggested_type).toBe('pattern');
  });

  it('does not fire below threshold', () => {
    const rule = createRepetitionRule();
    const result = rule.evaluate(createGroup(2), context);

    expect(result.fired).toBe(false);
    expect(result.contribution).toBe(0);
  });

  it('scales contribution with larger repetition counts', () => {
    const rule = createRepetitionRule();
    const low = rule.evaluate(createGroup(3), context);
    const high = rule.evaluate(createGroup(7), context);

    expect(low.fired).toBe(true);
    expect(high.fired).toBe(true);
    expect(high.contribution).toBeGreaterThan(low.contribution);
  });

  it('handles empty group edge case', () => {
    const rule = createRepetitionRule();
    const result = rule.evaluate(createGroup(0), context);

    expect(result.fired).toBe(false);
  });
});
