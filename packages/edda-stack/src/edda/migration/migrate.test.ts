import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  createActionId,
  createMemoryId,
  createObservationId,
  createProposalId,
  createSessionId,
  MEMORY_SCHEMA_VERSION,
} from '../../contracts/index.js';
import {
  detectMemorySchemaVersion,
  getCurrentSchemaVersion,
  getMigrationChain,
  migrateMemory,
} from './migrate.js';

afterEach(() => {
  vi.restoreAllMocks();
});

const MEMORY_ID = createMemoryId('550e8400-e29b-41d4-a716-446655440101');
const PROPOSAL_ID = createProposalId('550e8400-e29b-41d4-a716-446655440102');
const SESSION_ID = createSessionId('550e8400-e29b-41d4-a716-446655440103');
const OBSERVATION_ID = createObservationId('550e8400-e29b-41d4-a716-446655440104');
const ACTION_ID = createActionId('550e8400-e29b-41d4-a716-446655440105');

function createV0Memory(): Record<string, unknown> {
  return {
    id: MEMORY_ID,
    type: 'pattern',
    statement: 'Prefer deterministic checks before merging changes.',
    context: {
      when: 'During release candidate verification',
      why: 'Prevents accidental architecture drift',
      conditions: ['Cross-package changes are included'],
      tags: ['quality', 'release'],
    },
    confidence: 'high',
    provenance: {
      ember_source: {
        proposal_id: PROPOSAL_ID,
        proposal_type: 'pattern',
        confidence: 0.82,
        created_at: '2026-02-01T10:00:00.000Z',
      },
      kindling_sources: [
        {
          observation_id: OBSERVATION_ID,
          session_id: SESSION_ID,
          kind: 'gate_evaluated',
          timestamp: '2026-02-01T10:02:00.000Z',
        },
      ],
      source_sessions: [SESSION_ID],
      related_actions: [ACTION_ID],
    },
    attribution: {
      actor: 'agent/curator',
      timestamp: '2026-02-01T10:03:00.000Z',
      method: 'cli_command',
      reason: 'Promoted after human review',
    },
    created_at: '2026-02-01T10:03:00.000Z',
  };
}

function createV1Memory(): Record<string, unknown> {
  return {
    ...createV0Memory(),
    schema_version: MEMORY_SCHEMA_VERSION,
    status: 'active',
    evolution: {
      supersedes: [],
    },
    confidence_rationale: 'Observed repeatedly during release hardening.',
  };
}

describe('memory migration', () => {
  it('migrates v0 memory objects to v1 with default values', () => {
    const migrated = migrateMemory(createV0Memory(), 0, 1);

    expect(migrated.schema_version).toBe(1);
    expect(migrated.status).toBe('active');
    expect(migrated.evolution).toEqual({ supersedes: [] });
    expect(migrated.confidence_rationale).toBeUndefined();
    expect(migrated.statement).toBe('Prefer deterministic checks before merging changes.');
  });

  it('preserves existing status field during v0 to v1 migration', () => {
    const v0WithStatus = {
      ...createV0Memory(),
      status: 'retired',
    };
    const migrated = migrateMemory(v0WithStatus, 0, 1);

    expect(migrated.status).toBe('retired');
    expect(migrated.schema_version).toBe(1);
  });

  it('resolves a migration chain from v0 to v1', () => {
    const chain = getMigrationChain(0, 1);

    expect(chain).toHaveLength(1);
    expect(chain[0]).toMatchObject({
      fromVersion: 0,
      toVersion: 1,
      description: 'Add schema version and v1 default memory fields.',
    });
  });

  it('throws a descriptive error when a migration step is missing', () => {
    expect(() => getMigrationChain(0, 2)).toThrow(
      'Missing migration step from schema version 1 to 2.'
    );
  });

  it('passes through current-version memory unchanged', () => {
    const currentMemory = createV1Memory();
    const migrated = migrateMemory(currentMemory, 1, 1);

    expect(migrated).toEqual(currentMemory);
    expect(getCurrentSchemaVersion()).toBe(MEMORY_SCHEMA_VERSION);
  });

  it('throws a validation error when migrated data remains invalid', () => {
    const invalidV0 = {
      statement: 'This payload is missing required memory fields.',
    };

    expect(() => migrateMemory(invalidV0, 0, 1)).toThrow(
      'Migrated memory failed schema validation'
    );
  });

  it('detects schema version from memory-like payloads', () => {
    expect(detectMemorySchemaVersion(createV0Memory())).toBe(0);
    expect(detectMemorySchemaVersion(createV1Memory())).toBe(1);
    expect(detectMemorySchemaVersion('invalid')).toBe(0);
  });

  it('throws when getMigrationChain receives negative source version', () => {
    expect(() => getMigrationChain(-1, 1)).toThrow('Invalid source schema version: -1.');
  });

  it('throws when getMigrationChain receives negative target version', () => {
    expect(() => getMigrationChain(0, -1)).toThrow('Invalid target schema version: -1.');
  });

  it('throws when getMigrationChain attempts downgrade (from > to)', () => {
    expect(() => getMigrationChain(2, 1)).toThrow('Schema downgrades are not supported: 2 -> 1.');
  });

  it('returns 0 when detectMemorySchemaVersion receives non-integer schema_version', () => {
    expect(detectMemorySchemaVersion({ schema_version: 1.5 })).toBe(0);
    expect(detectMemorySchemaVersion({ schema_version: 'abc' })).toBe(0);
    expect(detectMemorySchemaVersion({ schema_version: -1 })).toBe(0);
  });

  it('throws when migrateMemory is called with non-object input for v0 to v1 migration', () => {
    expect(() => migrateMemory('not an object', 0, 1)).toThrow(
      'Migration from v0 to v1 requires an object payload.'
    );
    expect(() => migrateMemory(123, 0, 1)).toThrow(
      'Migration from v0 to v1 requires an object payload.'
    );
    expect(() => migrateMemory(null, 0, 1)).toThrow(
      'Migration from v0 to v1 requires an object payload.'
    );
  });
});
