/**
 * Regression tests for CIB-118 (payload-validation consistency):
 * "Kindling writes exactly the payload that was validated."
 *
 * `KindlingService.emit` previously validated the observation, then persisted
 * the ORIGINAL object (or a redacted copy that was never re-validated). These
 * tests pin two invariants at the store boundary:
 *
 * 1. The persisted payload is the schema-validated payload — fields outside
 *    the contract never reach the store.
 * 2. A redacted payload is re-validated against the same schema before it is
 *    persisted; if redaction breaks the contract, emit fails closed.
 */

import { describe, it, expect, vi, afterEach } from 'vitest';
import {
  KindlingService,
  ObservationValidationError,
  type IKindlingStore,
} from './kindling-service.js';
import type { Observation } from './observation-contract.js';
import type { QueryRequest, QueryResponse } from './query-contract.js';
import { KindlingConfigSchema } from './config.js';
import * as sensitiveDataValidator from './sensitive-data-validator.js';

vi.mock('./sensitive-data-validator.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./sensitive-data-validator.js')>();
  return {
    ...actual,
    validateNoSensitiveData: vi.fn(actual.validateNoSensitiveData),
    redactSensitiveFields: vi.fn(actual.redactSensitiveFields),
  };
});

const VALID_UUID = '550e8400-e29b-41d4-a716-446655440000';
const VALID_TIMESTAMP = '2026-02-15T10:00:00.000Z';

const enabledConfig = KindlingConfigSchema.parse({ enabled: true });

function makeObservation(command: string): Observation {
  return {
    kind: 'action_executed',
    session_id: VALID_UUID,
    timestamp: VALID_TIMESTAMP,
    action_id: 'action-001',
    action_type: 'command',
    details: { command, working_directory: '/home/user' },
    outcome: 'success',
    duration_ms: 100,
  } as Observation;
}

function spyStore(): { store: IKindlingStore; writes: Observation[] } {
  const writes: Observation[] = [];
  const store: IKindlingStore = {
    emit: async (o) => {
      writes.push(o);
    },
    query: async (_request: QueryRequest): Promise<QueryResponse> => {
      throw new Error('query not used in validation tests');
    },
    close: async () => {},
  };
  return { store, writes };
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe('KindlingService.emit — validated-payload consistency (CIB-118)', () => {
  it('persists exactly the schema-validated payload, never fields outside the contract', async () => {
    const { store, writes } = spyStore();
    const svc = new KindlingService(store, enabledConfig);

    const observation = {
      ...makeObservation('ls -la /home/user'),
      rogue_field: 'never validated, must never be stored',
    } as Observation;

    await svc.emit(observation);

    expect(writes).toHaveLength(1);
    expect(writes[0]).not.toHaveProperty('rogue_field');
    expect(writes[0].kind).toBe('action_executed');
  });

  it('re-validates the redacted payload against the same schema before persisting', async () => {
    const { store, writes } = spyStore();
    const svc = new KindlingService(store, enabledConfig);

    vi.mocked(sensitiveDataValidator.validateNoSensitiveData).mockReturnValueOnce({
      hasSensitiveData: true,
      issues: ['injected sensitive finding'],
    });
    // Simulate a redactor bug that strips a required field: the write must
    // fail closed rather than persist a payload that no longer matches the
    // schema that was validated.
    vi.mocked(sensitiveDataValidator.redactSensitiveFields).mockImplementationOnce(
      (observation) => {
        const broken = { ...observation } as Record<string, unknown>;
        delete broken.session_id;
        return broken as unknown as Observation;
      }
    );

    await expect(svc.emit(makeObservation('mysql --password=hunter2'))).rejects.toThrow(
      ObservationValidationError
    );
    expect(writes).toHaveLength(0);
  });

  it('persists the re-validated redacted payload when redaction preserves the contract', async () => {
    const { store, writes } = spyStore();
    const svc = new KindlingService(store, enabledConfig);

    await svc.emit(makeObservation('mysql -u root --password=secret123hunter'));

    expect(writes).toHaveLength(1);
    const serialised = JSON.stringify(writes[0]);
    expect(serialised).not.toContain('secret123hunter');
    expect(serialised).toContain('[REDACTED');
  });
});
