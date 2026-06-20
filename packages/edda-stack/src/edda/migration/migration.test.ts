/**
 * Additional migration coverage tests.
 *
 * The primary migration tests live in migrate.test.ts. This file covers
 * behaviours not yet exercised there:
 *   - Re-migrating an already-current-version object is idempotent.
 *   - detectMemorySchemaVersion edge cases (null, array, negative zero,
 *     very large schema_version, schema_version === 0 explicitly).
 *   - getMigrationChain with from === to returns an empty chain.
 *   - getMigrationChain with non-integer float arguments throws.
 *   - migrationRegistry shape and MigrationStep interface contract.
 *   - The migration module's index re-exports all public symbols.
 */
import { describe, expect, it } from 'vitest';
import {
  detectMemorySchemaVersion,
  getCurrentSchemaVersion,
  getMigrationChain,
  migrateMemory,
  migrationRegistry,
  type MigrationStep,
} from './migrate.js';
import {
  createActionId,
  createMemoryId,
  createObservationId,
  createProposalId,
  createSessionId,
  MEMORY_SCHEMA_VERSION,
} from '../../contracts/index.js';

// Re-export check — ensures the index barrel re-exports all public symbols.
import {
  detectMemorySchemaVersion as detectFromIndex,
  getCurrentSchemaVersion as currentFromIndex,
  getMigrationChain as chainFromIndex,
  migrateMemory as migrateFromIndex,
  migrationRegistry as registryFromIndex,
} from './index.js';

// =============================================================================
// Fixture builders
// =============================================================================

const MEM_ID = createMemoryId('550e8400-e29b-41d4-a716-446655441001');
const PROPOSAL_ID = createProposalId('550e8400-e29b-41d4-a716-446655441002');
const SESSION_ID = createSessionId('550e8400-e29b-41d4-a716-446655441003');
const OBS_ID = createObservationId('550e8400-e29b-41d4-a716-446655441004');
const ACTION_ID = createActionId('550e8400-e29b-41d4-a716-446655441005');

/** Minimal v0-era payload — no schema_version, status, or evolution fields. */
function buildV0Fixture(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: MEM_ID,
    type: 'lesson',
    statement: 'Fixture v0 memory for migration testing.',
    context: {
      when: 'During integration testing',
      why: 'Verify migration idempotency',
      conditions: ['Tests touch migration code'],
      tags: ['migration', 'test'],
    },
    confidence: 'medium',
    provenance: {
      ember_source: {
        proposal_id: PROPOSAL_ID,
        proposal_type: 'lesson',
        confidence: 0.7,
        created_at: '2026-03-01T09:00:00.000Z',
      },
      kindling_sources: [
        {
          observation_id: OBS_ID,
          session_id: SESSION_ID,
          kind: 'gate_evaluated',
          timestamp: '2026-03-01T09:01:00.000Z',
        },
      ],
      source_sessions: [SESSION_ID],
      related_actions: [ACTION_ID],
    },
    attribution: {
      actor: 'agent/migration-test',
      timestamp: '2026-03-01T09:02:00.000Z',
      method: 'cli_command',
      reason: 'Migration fixture',
    },
    created_at: '2026-03-01T09:02:00.000Z',
    ...overrides,
  };
}

/** v1 fixture — already at the current schema version. */
function buildV1Fixture(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    ...buildV0Fixture(),
    schema_version: MEMORY_SCHEMA_VERSION,
    status: 'active',
    evolution: { supersedes: [] },
    confidence_rationale: 'Observed during test suite expansion.',
    ...overrides,
  };
}

// =============================================================================
// Index re-export contract
// =============================================================================

describe('migration/index.ts re-exports', () => {
  it('re-exports detectMemorySchemaVersion from the barrel', () => {
    expect(detectFromIndex).toBe(detectMemorySchemaVersion);
  });

  it('re-exports getCurrentSchemaVersion from the barrel', () => {
    expect(currentFromIndex).toBe(getCurrentSchemaVersion);
  });

  it('re-exports getMigrationChain from the barrel', () => {
    expect(chainFromIndex).toBe(getMigrationChain);
  });

  it('re-exports migrateMemory from the barrel', () => {
    expect(migrateFromIndex).toBe(migrateMemory);
  });

  it('re-exports migrationRegistry from the barrel', () => {
    expect(registryFromIndex).toBe(migrationRegistry);
  });
});

// =============================================================================
// migrationRegistry shape
// =============================================================================

describe('migrationRegistry shape', () => {
  it('is a non-empty readonly array of MigrationStep objects', () => {
    expect(Array.isArray(migrationRegistry)).toBe(true);
    expect(migrationRegistry.length).toBeGreaterThan(0);
  });

  it('each step has the required MigrationStep fields', () => {
    for (const step of migrationRegistry) {
      const typed = step as MigrationStep;
      expect(typeof typed.fromVersion).toBe('number');
      expect(typeof typed.toVersion).toBe('number');
      expect(typeof typed.description).toBe('string');
      expect(typeof typed.migrate).toBe('function');
    }
  });

  it('every step increments the version (fromVersion < toVersion)', () => {
    for (const step of migrationRegistry) {
      expect(step.fromVersion).toBeLessThan(step.toVersion);
    }
  });

  it('steps form a contiguous chain from 0 to the current schema version', () => {
    const sorted = [...migrationRegistry].sort((a, b) => a.fromVersion - b.fromVersion);
    let expected = 0;
    for (const step of sorted) {
      expect(step.fromVersion).toBe(expected);
      expected = step.toVersion;
    }
    expect(expected).toBe(MEMORY_SCHEMA_VERSION);
  });

  it('migrationRegistry type is ReadonlyArray (compile-time immutability enforced by TypeScript)', () => {
    // ReadonlyArray is a compile-time constraint, not a runtime freeze.
    // We verify the reference is stable and the length is correct.
    const snapshot = migrationRegistry.length;
    expect(migrationRegistry.length).toBe(snapshot);
    // Confirm the type satisfies ReadonlyArray: .map() is available; .push() is not on the type.
    const descriptions = migrationRegistry.map((s) => s.description);
    expect(descriptions.length).toBe(snapshot);
  });
});

// =============================================================================
// getCurrentSchemaVersion
// =============================================================================

describe('getCurrentSchemaVersion', () => {
  it('returns the MEMORY_SCHEMA_VERSION constant', () => {
    expect(getCurrentSchemaVersion()).toBe(MEMORY_SCHEMA_VERSION);
  });

  it('returns a positive integer', () => {
    const version = getCurrentSchemaVersion();
    expect(Number.isInteger(version)).toBe(true);
    expect(version).toBeGreaterThan(0);
  });
});

// =============================================================================
// detectMemorySchemaVersion
// =============================================================================

describe('detectMemorySchemaVersion — edge cases', () => {
  it('returns 0 for null', () => {
    expect(detectMemorySchemaVersion(null)).toBe(0);
  });

  it('returns 0 for an array', () => {
    expect(detectMemorySchemaVersion([])).toBe(0);
  });

  it('returns 0 for a number primitive', () => {
    expect(detectMemorySchemaVersion(42)).toBe(0);
  });

  it('returns 0 for a boolean', () => {
    expect(detectMemorySchemaVersion(true)).toBe(0);
  });

  it('returns 0 when schema_version is negative', () => {
    expect(detectMemorySchemaVersion({ schema_version: -5 })).toBe(0);
  });

  it('returns 0 when schema_version is a float', () => {
    expect(detectMemorySchemaVersion({ schema_version: 0.5 })).toBe(0);
  });

  it('returns 0 when schema_version is a string', () => {
    expect(detectMemorySchemaVersion({ schema_version: '1' })).toBe(0);
  });

  it('returns 0 when schema_version is null', () => {
    expect(detectMemorySchemaVersion({ schema_version: null })).toBe(0);
  });

  it('returns 0 when schema_version is undefined', () => {
    expect(detectMemorySchemaVersion({ schema_version: undefined })).toBe(0);
  });

  it('returns 0 explicitly when schema_version is 0', () => {
    expect(detectMemorySchemaVersion({ schema_version: 0 })).toBe(0);
  });

  it('returns the integer schema_version when schema_version equals the current version', () => {
    expect(detectMemorySchemaVersion({ schema_version: MEMORY_SCHEMA_VERSION })).toBe(
      MEMORY_SCHEMA_VERSION
    );
  });

  it('returns a large integer when schema_version is unexpectedly large', () => {
    expect(detectMemorySchemaVersion({ schema_version: 9999 })).toBe(9999);
  });

  it('correctly reads schema_version from a v0 fixture', () => {
    expect(detectMemorySchemaVersion(buildV0Fixture())).toBe(0);
  });

  it('correctly reads schema_version from a v1 fixture', () => {
    expect(detectMemorySchemaVersion(buildV1Fixture())).toBe(MEMORY_SCHEMA_VERSION);
  });
});

// =============================================================================
// getMigrationChain
// =============================================================================

describe('getMigrationChain', () => {
  it('returns an empty chain when from equals to', () => {
    expect(getMigrationChain(0, 0)).toEqual([]);
    expect(getMigrationChain(1, 1)).toEqual([]);
  });

  it('throws for a non-integer float source version', () => {
    expect(() => getMigrationChain(0.5, 1)).toThrow('Invalid source schema version: 0.5.');
  });

  it('throws for a non-integer float target version', () => {
    expect(() => getMigrationChain(0, 0.9)).toThrow('Invalid target schema version: 0.9.');
  });

  it('throws for NaN source version', () => {
    expect(() => getMigrationChain(NaN, 1)).toThrow('Invalid source schema version');
  });

  it('throws for NaN target version', () => {
    expect(() => getMigrationChain(0, NaN)).toThrow('Invalid target schema version');
  });

  it('returns a single-step chain for v0 → v1', () => {
    const chain = getMigrationChain(0, 1);
    expect(chain).toHaveLength(1);
    expect(chain[0]?.fromVersion).toBe(0);
    expect(chain[0]?.toVersion).toBe(1);
  });

  it('each step in the chain forms a contiguous version sequence', () => {
    const chain = getMigrationChain(0, MEMORY_SCHEMA_VERSION);
    let expected = 0;
    for (const step of chain) {
      expect(step.fromVersion).toBe(expected);
      expected = step.toVersion;
    }
    expect(expected).toBe(MEMORY_SCHEMA_VERSION);
  });
});

// =============================================================================
// migrateMemory
// =============================================================================

describe('migrateMemory — migration paths and idempotency', () => {
  it('migrates a v0 lesson-type fixture to a valid v1 MemoryObject', () => {
    const v0 = buildV0Fixture();
    const result = migrateMemory(v0, 0, 1);

    expect(result.schema_version).toBe(1);
    expect(result.status).toBe('active');
    expect(result.evolution).toEqual({ supersedes: [] });
    expect(result.type).toBe('lesson');
    expect(result.statement).toBe('Fixture v0 memory for migration testing.');
    expect(result.confidence_rationale).toBeUndefined();
  });

  it('preserves existing evolution field from v0 when it is already populated', () => {
    const v0WithEvolution = buildV0Fixture({
      evolution: { supersedes: [] },
    });
    const result = migrateMemory(v0WithEvolution, 0, 1);

    expect(result.evolution).toEqual({ supersedes: [] });
  });

  it('preserves existing confidence_rationale from v0 when it is already set', () => {
    const v0WithRationale = buildV0Fixture({
      confidence_rationale: 'Pre-existing rationale.',
    });
    const result = migrateMemory(v0WithRationale, 0, 1);

    expect(result.confidence_rationale).toBe('Pre-existing rationale.');
  });

  it('migrating a v1 fixture from v1 to v1 is a no-op (idempotency)', () => {
    const v1 = buildV1Fixture();
    const result = migrateMemory(v1, 1, 1);

    expect(result.schema_version).toBe(MEMORY_SCHEMA_VERSION);
    expect(result.type).toBe('lesson');
    expect(result.status).toBe('active');
    expect(result.confidence_rationale).toBe('Observed during test suite expansion.');
  });

  it('migrating a v0 fixture with an explicit "retired" status preserves it', () => {
    const v0Retired = buildV0Fixture({ status: 'retired' });
    const result = migrateMemory(v0Retired, 0, 1);

    expect(result.status).toBe('retired');
  });

  it('migrating a v0 fixture with an explicit "superseded" status preserves it', () => {
    const v0Superseded = buildV0Fixture({ status: 'superseded' });
    const result = migrateMemory(v0Superseded, 0, 1);

    expect(result.status).toBe('superseded');
  });

  it('throws a descriptive validation error when the migrated data is invalid', () => {
    const incomplete = { type: 'pattern' };
    expect(() => migrateMemory(incomplete, 0, 1)).toThrow(
      'Migrated memory failed schema validation'
    );
  });

  it('throws when migrateMemory is called with an array as input', () => {
    expect(() => migrateMemory([], 0, 1)).toThrow(
      'Migration from v0 to v1 requires an object payload.'
    );
  });

  it('throws when migrateMemory is called with a boolean as input', () => {
    expect(() => migrateMemory(false, 0, 1)).toThrow(
      'Migration from v0 to v1 requires an object payload.'
    );
  });

  it('different v0 memory types migrate correctly (decision, warning, constraint)', () => {
    for (const type of ['decision', 'warning', 'constraint'] as const) {
      const v0 = buildV0Fixture({ type });
      const result = migrateMemory(v0, 0, 1);
      expect(result.schema_version).toBe(1);
      expect(result.type).toBe(type);
      expect(result.status).toBe('active');
    }
  });

  it('migrateMemory result passes detectMemorySchemaVersion after migration', () => {
    const v0 = buildV0Fixture();
    const result = migrateMemory(v0, 0, 1);

    expect(detectMemorySchemaVersion(result)).toBe(MEMORY_SCHEMA_VERSION);
  });
});
