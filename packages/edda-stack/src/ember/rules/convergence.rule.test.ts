import { describe, expect, it } from 'vitest';
import { createConvergenceRule } from './convergence.rule.js';
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

function createGroup(sessionCount: number): ObservationGroup {
  return {
    id: 'g-convergence',
    grouping_type: 'pattern',
    observation_ids: ['obs-1', 'obs-2'],
    session_ids: Array.from({ length: sessionCount }, (_, index) => `session-${index}`),
    earliest: '2026-01-01T10:00:00.000Z',
    latest: '2026-01-01T11:00:00.000Z',
    count: 2,
    suggested_type: 'decision',
    signals: [],
  };
}

describe('ConvergenceRule', () => {
  it('fires when multiple sessions converge', () => {
    const rule = createConvergenceRule();
    const result = rule.evaluate(createGroup(3), context);

    expect(result.fired).toBe(true);
    expect(result.context?.suggested_type).toBe('decision');
  });

  it('does not fire for a single session', () => {
    const rule = createConvergenceRule();
    const result = rule.evaluate(createGroup(1), context);

    expect(result.fired).toBe(false);
  });

  it('scales contribution as more sessions join', () => {
    const rule = createConvergenceRule();
    const low = rule.evaluate(createGroup(2), context);
    const high = rule.evaluate(createGroup(5), context);

    expect(high.fired).toBe(true);
    expect(high.contribution).toBeGreaterThan(low.contribution);
  });

  it('handles empty session list edge case', () => {
    const rule = createConvergenceRule();
    const result = rule.evaluate(createGroup(0), context);

    expect(result.fired).toBe(false);
  });
});
