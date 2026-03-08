import { describe, it, expect } from 'vitest';
import {
  OBSERVATION_KIND_TO_PROPOSAL_TYPE_MAPPING,
  mapObservationKindToProposalType,
  mapObservationKindsToProposalType,
} from './observation-mappings.js';
import type { ObservationKind } from './ports/kindling.port.js';
import type { ProposalType } from './ember-proposal.js';

describe('observation-mappings', () => {
  describe('OBSERVATION_KIND_TO_PROPOSAL_TYPE_MAPPING', () => {
    const expectedMappings: Array<[ObservationKind, ProposalType]> = [
      ['gate_evaluated', 'pattern'],
      ['action_executed', 'pattern'],
      ['action_failed', 'warning'],
      ['plan_started', 'decision'],
      ['plan_completed', 'lesson'],
      ['constraint_applied', 'constraint'],
      ['error_recorded', 'warning'],
      ['metric_recorded', 'pattern'],
      ['custom', 'pattern'],
    ];

    it.each(expectedMappings)('maps %s to %s', (kind, expectedType) => {
      expect(OBSERVATION_KIND_TO_PROPOSAL_TYPE_MAPPING[kind]).toBe(expectedType);
    });

    it('contains exactly 9 mappings', () => {
      expect(Object.keys(OBSERVATION_KIND_TO_PROPOSAL_TYPE_MAPPING)).toHaveLength(9);
    });
  });

  describe('mapObservationKindToProposalType', () => {
    const expectedMappings: Array<[ObservationKind, ProposalType]> = [
      ['gate_evaluated', 'pattern'],
      ['action_executed', 'pattern'],
      ['action_failed', 'warning'],
      ['plan_started', 'decision'],
      ['plan_completed', 'lesson'],
      ['constraint_applied', 'constraint'],
      ['error_recorded', 'warning'],
      ['metric_recorded', 'pattern'],
      ['custom', 'pattern'],
    ];

    it.each(expectedMappings)('maps %s to %s', (kind, expectedType) => {
      expect(mapObservationKindToProposalType(kind)).toBe(expectedType);
    });
  });
  describe('mapObservationKindsToProposalType', () => {
    it('maps mixed failure and success signals to lesson', () => {
      expect(mapObservationKindsToProposalType(['error_recorded', 'action_executed'])).toBe(
        'lesson'
      );
      expect(mapObservationKindsToProposalType(['action_failed', 'plan_completed'])).toBe('lesson');
    });

    it('maps failure signals to warning', () => {
      expect(mapObservationKindsToProposalType(['error_recorded'])).toBe('warning');
      expect(mapObservationKindsToProposalType(['action_failed'])).toBe('warning');
    });

    it('maps constraint signals to constraint when no failure exists', () => {
      expect(mapObservationKindsToProposalType(['constraint_applied'])).toBe('constraint');
      expect(mapObservationKindsToProposalType(['constraint_applied', 'action_executed'])).toBe(
        'constraint'
      );
    });

    it('maps plan_started to decision when no higher-priority signal exists', () => {
      expect(mapObservationKindsToProposalType(['plan_started'])).toBe('decision');
      expect(mapObservationKindsToProposalType(['plan_started', 'gate_evaluated'])).toBe(
        'decision'
      );
    });

    it('maps to pattern by default', () => {
      expect(mapObservationKindsToProposalType(['gate_evaluated'])).toBe('pattern');
      expect(mapObservationKindsToProposalType(['action_executed'])).toBe('pattern');
      expect(mapObservationKindsToProposalType(['metric_recorded', 'custom'])).toBe('pattern');
      expect(mapObservationKindsToProposalType([])).toBe('pattern');
    });

    it('applies precedence deterministically', () => {
      expect(mapObservationKindsToProposalType(['constraint_applied', 'error_recorded'])).toBe(
        'warning'
      );
      expect(mapObservationKindsToProposalType(['plan_started', 'error_recorded'])).toBe('warning');
      expect(
        mapObservationKindsToProposalType(['plan_started', 'constraint_applied', 'error_recorded'])
      ).toBe('warning');
    });
  });
});
