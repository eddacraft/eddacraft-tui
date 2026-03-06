import { describe, expect, it } from 'vitest';
import {
  DecisionProposalSchema,
  PatternProposalSchema,
  WarningProposalSchema,
  LessonProposalSchema,
  AnomalyProposalSchema,
  ConstraintProposalSchema,
  TypedProposalSchema,
  createTypedProposal,
  parseTypedProposal,
  validateProposalMetadata,
} from './proposal-types.js';
import type { ProposalType } from './ember-proposal.js';

const VALID_UUID = '550e8400-e29b-41d4-a716-446655440000';
const VALID_TIMESTAMP = '2024-01-15T14:30:00.000Z';
const VALID_EXPIRY = '2024-02-14T14:30:00.000Z';

function buildBaseProposal<T extends ProposalType>(type: T, metadata: unknown) {
  return {
    id: VALID_UUID,
    type,
    status: 'active' as const,
    summary: 'Observed candidate memory',
    rationale: 'Likely useful for future decisions',
    metadata,
    confidence: 0.72,
    signals: [],
    provenance: {
      observation_ids: [VALID_UUID],
      session_ids: [VALID_UUID],
      earliest_observation: VALID_TIMESTAMP,
      latest_observation: VALID_TIMESTAMP,
    },
    created_at: VALID_TIMESTAMP,
    expires_at: VALID_EXPIRY,
    ttl_days: 30,
  };
}

describe('Proposal type schemas (EMBER-002)', () => {
  it('validates decision proposal metadata', () => {
    const result = DecisionProposalSchema.safeParse(
      buildBaseProposal('decision', {
        decision_point: 'Database backend selection',
        alternatives_considered: ['SQLite', 'PostgreSQL'],
      })
    );

    expect(result.success).toBe(true);
  });

  it('validates pattern proposal metadata', () => {
    const result = PatternProposalSchema.safeParse(
      buildBaseProposal('pattern', {
        pattern_name: 'Retry with backoff',
        occurrence_count: 4,
        first_seen: VALID_TIMESTAMP,
        last_seen: VALID_TIMESTAMP,
      })
    );

    expect(result.success).toBe(true);
  });

  it('validates warning proposal metadata', () => {
    const result = WarningProposalSchema.safeParse(
      buildBaseProposal('warning', {
        warning_type: 'resource_pressure',
        severity: 'high',
        affected_areas: ['runtime/gate'],
      })
    );

    expect(result.success).toBe(true);
  });

  it('validates lesson proposal metadata', () => {
    const result = LessonProposalSchema.safeParse(
      buildBaseProposal('lesson', {
        lesson_type: 'failure',
        context: 'Missed dependency boundary in refactor',
      })
    );

    expect(result.success).toBe(true);
  });

  it('validates anomaly proposal metadata', () => {
    const result = AnomalyProposalSchema.safeParse(
      buildBaseProposal('anomaly', {
        expected_behaviour: 'Build completes under 2 minutes',
        actual_behaviour: 'Build time exceeded 7 minutes',
      })
    );

    expect(result.success).toBe(true);
  });

  it('validates constraint proposal metadata', () => {
    const result = ConstraintProposalSchema.safeParse(
      buildBaseProposal('constraint', {
        constraint_type: 'runtime_limit',
        scope: 'ci-runner',
      })
    );

    expect(result.success).toBe(true);
  });

  it('rejects wrong metadata for a proposal type', () => {
    const result = DecisionProposalSchema.safeParse(
      buildBaseProposal('decision', {
        warning_type: 'resource_pressure',
        severity: 'high',
      })
    );

    expect(result.success).toBe(false);
  });

  it('parses discriminated union for all proposal types', () => {
    const proposals = [
      buildBaseProposal('decision', { decision_point: 'Select queue backend' }),
      buildBaseProposal('pattern', {
        occurrence_count: 3,
        first_seen: VALID_TIMESTAMP,
        last_seen: VALID_TIMESTAMP,
      }),
      buildBaseProposal('warning', { warning_type: 'high_error_rate', severity: 'medium' }),
      buildBaseProposal('lesson', { lesson_type: 'mixed', context: 'Migration rollout' }),
      buildBaseProposal('anomaly', {
        expected_behaviour: 'No retries',
        actual_behaviour: 'Retries increased sharply',
      }),
      buildBaseProposal('constraint', { constraint_type: 'storage', scope: 'proposal-store' }),
    ];

    for (const proposal of proposals) {
      expect(TypedProposalSchema.safeParse(proposal).success).toBe(true);
    }
  });
});

describe('Proposal metadata utilities (EMBER-002)', () => {
  it('createTypedProposal validates and returns typed metadata', () => {
    const proposal = createTypedProposal('lesson', {
      lesson_type: 'success',
      context: 'Promotion playbook update',
    });

    expect(proposal.type).toBe('lesson');
    expect(proposal.metadata.lesson_type).toBe('success');
  });

  it('validateProposalMetadata returns parsed metadata for valid input', () => {
    const metadata = validateProposalMetadata('constraint', {
      constraint_type: 'policy',
      scope: 'repository',
    });

    expect(metadata).not.toBeNull();
    expect(metadata?.scope).toBe('repository');
  });

  it('validateProposalMetadata returns null for invalid input', () => {
    const metadata = validateProposalMetadata('pattern', {
      pattern_name: 'Missing required values',
    });

    expect(metadata).toBeNull();
  });

  it('parseTypedProposal validates a full candidate proposal', () => {
    const proposal = parseTypedProposal(
      buildBaseProposal('warning', {
        warning_type: 'schema_drift',
        severity: 'low',
      })
    );

    expect(proposal.type).toBe('warning');
    if (proposal.type === 'warning') {
      expect(proposal.metadata.warning_type).toBe('schema_drift');
    }
  });
});
