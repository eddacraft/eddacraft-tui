import { describe, expect, it } from 'vitest';
import type { CandidateProposal } from '../contracts/ember-proposal.js';
import type { ObservationGroup } from './aggregator-service.js';
import {
  EvaluatorService,
  type EmberEvaluationConfig,
  type EvaluationContext,
  type EvaluationRule,
} from './evaluator-service.js';
import { createDefaultRules } from './rules/index.js';

const BASE_CONFIG: EmberEvaluationConfig = {
  min_confidence: 0.5,
  repetition_threshold: 3,
  escalation_window_hours: 6,
};

function createContext(existingProposals: CandidateProposal[] = []): EvaluationContext {
  return {
    existingProposals,
    config: BASE_CONFIG,
  };
}

function createGroup(overrides: Partial<ObservationGroup> = {}): ObservationGroup {
  return {
    id: 'group-1',
    grouping_type: 'pattern',
    observation_ids: ['obs-1', 'obs-2'],
    session_ids: ['session-1'],
    earliest: '2026-01-01T10:00:00.000Z',
    latest: '2026-01-01T10:10:00.000Z',
    count: 2,
    suggested_type: 'pattern',
    signals: [],
    ...overrides,
  };
}

describe('EvaluatorService', () => {
  it('registerRule, removeRule and getRules manage registry', () => {
    const evaluator = new EvaluatorService();
    const rule: EvaluationRule = {
      name: 'alpha',
      description: 'alpha rule',
      weight: 1,
      evaluate: () => ({ fired: true, contribution: 0.4 }),
    };

    evaluator.registerRule(rule);
    expect(evaluator.getRules()).toHaveLength(1);

    evaluator.removeRule('alpha');
    expect(evaluator.getRules()).toHaveLength(0);
  });

  it('evaluate computes confidence with one fired rule', () => {
    const evaluator = new EvaluatorService([
      {
        name: 'single',
        description: 'single rule',
        weight: 2,
        evaluate: () => ({
          fired: true,
          contribution: 0.75,
          context: { suggested_type: 'warning' },
        }),
      },
    ]);

    const outcome = evaluator.evaluate(createGroup(), createContext());

    expect(outcome.confidence).toBe(0.75);
    expect(outcome.signals).toHaveLength(1);
    expect(outcome.suggestedType).toBe('warning');
    expect(outcome.meetsThreshold).toBe(true);
  });

  it('evaluate computes weighted confidence across multiple rules', () => {
    const evaluator = new EvaluatorService([
      {
        name: 'first',
        description: 'first rule',
        weight: 1,
        evaluate: () => ({ fired: true, contribution: 0.4 }),
      },
      {
        name: 'second',
        description: 'second rule',
        weight: 3,
        evaluate: () => ({ fired: true, contribution: 0.8 }),
      },
    ]);

    const outcome = evaluator.evaluate(createGroup(), createContext());

    expect(outcome.confidence).toBeCloseTo(0.7, 5);
    expect(outcome.signals).toHaveLength(2);
    expect(outcome.meetsThreshold).toBe(true);
  });

  it('evaluateAll processes multiple groups', () => {
    const evaluator = new EvaluatorService([
      {
        name: 'constant',
        description: 'constant rule',
        weight: 1,
        evaluate: () => ({ fired: true, contribution: 0.6 }),
      },
    ]);

    const outcomes = evaluator.evaluateAll(
      [createGroup({ id: 'g1' }), createGroup({ id: 'g2' })],
      createContext()
    );

    expect(outcomes).toHaveLength(2);
    expect(outcomes.map((item) => item.group.id)).toEqual(['g1', 'g2']);
    expect(outcomes.every((item) => item.meetsThreshold)).toBe(true);
  });

  it('marks outcome below threshold when confidence is low', () => {
    const evaluator = new EvaluatorService([
      {
        name: 'low',
        description: 'low confidence rule',
        weight: 1,
        evaluate: () => ({ fired: true, contribution: 0.2 }),
      },
    ]);

    const outcome = evaluator.evaluate(createGroup(), createContext());

    expect(outcome.confidence).toBe(0.2);
    expect(outcome.meetsThreshold).toBe(false);
  });

  it('works with default built-in rules', () => {
    const evaluator = new EvaluatorService(createDefaultRules());
    const group = createGroup({
      count: 5,
      session_ids: ['session-1', 'session-2'],
      signals: ['failure_signal', 'success_signal', 'severity_low', 'severity_high', 'kind_custom'],
    });

    const outcome = evaluator.evaluate(group, createContext());

    expect(outcome.signals.length).toBeGreaterThan(0);
    expect(outcome.confidence).toBeGreaterThan(0);
    expect(['pattern', 'warning', 'lesson', 'decision', 'anomaly']).toContain(
      outcome.suggestedType
    );
  });
});
