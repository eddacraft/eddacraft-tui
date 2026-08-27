import { describe, expect, it } from 'vitest';
import { deserialiseRow } from './proposal-store.js';

const ROW_ID = '550e8400-e29b-41d4-a716-000000000001';
const SESSION_ID = '550e8400-e29b-41d4-a716-000000000002';

const provenance = JSON.stringify({
  observation_ids: [ROW_ID],
  session_ids: [SESSION_ID],
  earliest_observation: '2026-01-01T00:00:00.000Z',
  latest_observation: '2026-01-01T00:05:00.000Z',
});

function rowFromDriver(
  overrides: {
    metadata?: string | null;
    signals?: string | null;
    resolution?: string | null;
  } = {}
) {
  return {
    id: ROW_ID,
    type: 'pattern' as const,
    status: 'active' as const,
    summary: 'pattern summary',
    rationale: 'pattern rationale',
    confidence: 0.7,
    metadata: null,
    signals: '[]',
    provenance,
    created_at: '2026-01-01T00:00:00.000Z',
    expires_at: '2026-01-31T00:00:00.000Z',
    ttl_days: 30,
    updated_at: null,
    resolution: null,
    ...overrides,
  };
}

describe('deserialiseRow JSON columns', () => {
  it('treats null JSON columns as missing', () => {
    const proposal = deserialiseRow(
      rowFromDriver({ metadata: null, signals: null, resolution: null })
    );

    expect(proposal.metadata).toBeUndefined();
    expect(proposal.signals).toEqual([]);
    expect(proposal.resolution).toBeUndefined();
  });

  it('fails fast when metadata is an empty JSON string', () => {
    expect(() => deserialiseRow(rowFromDriver({ metadata: '' }))).toThrow(
      'invalid proposal metadata JSON'
    );
  });

  it('fails fast when signals is an empty JSON string', () => {
    expect(() => deserialiseRow(rowFromDriver({ signals: '' }))).toThrow(
      'invalid proposal signals JSON'
    );
  });

  it('fails fast when resolution is an empty JSON string', () => {
    expect(() => deserialiseRow(rowFromDriver({ resolution: '' }))).toThrow(
      'invalid proposal resolution JSON'
    );
  });

  it('treats undefined JSON columns as missing', () => {
    const proposal = deserialiseRow(
      rowFromDriver({
        metadata: undefined,
        signals: undefined,
        resolution: undefined,
      })
    );

    expect(proposal.metadata).toBeUndefined();
    expect(proposal.signals).toEqual([]);
    expect(proposal.resolution).toBeUndefined();
  });
});
