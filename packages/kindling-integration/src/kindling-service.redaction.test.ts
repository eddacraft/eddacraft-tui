/**
 * Regression test for issue #1826 findings
 * `fnd_sig-feat-library-8107990571-da0a_caa2b09fec` /
 * `fnd_sig-feat-library-9f8d3feba6-2c55_47b5588028`:
 * "Detected sensitive observations can still be persisted unredacted".
 *
 * `KindlingService.emit` already enforces detect -> redact -> persist the
 * redacted copy. The detection helper was tested, but nothing guarded the
 * *enforcement* at the store boundary. This pins that a detected secret is
 * never written to the store in its original form, so a future refactor
 * cannot silently regress to "detection without enforcement".
 */

import { describe, it, expect } from 'vitest';
import { KindlingService, type IKindlingStore } from './kindling-service.js';
import type { Observation } from './observation-contract.js';
import type { QueryRequest, QueryResponse } from './query-contract.js';
import { KindlingConfigSchema } from './config.js';

const VALID_UUID = '550e8400-e29b-41d4-a716-446655440000';
const VALID_TIMESTAMP = '2026-02-15T10:00:00.000Z';

const enabledConfig = KindlingConfigSchema.parse({ enabled: true });
const disabledConfig = KindlingConfigSchema.parse({ enabled: false });

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
      throw new Error('query not used in redaction tests');
    },
    close: async () => {},
  };
  return { store, writes };
}

describe('KindlingService.emit — redaction enforcement (issue #1826)', () => {
  it('persists the REDACTED observation, never the original secret', async () => {
    const { store, writes } = spyStore();
    const svc = new KindlingService(store, enabledConfig);

    await svc.emit(makeObservation('mysql -u root --password=secret123hunter'));

    expect(writes).toHaveLength(1);
    const serialized = JSON.stringify(writes[0]);
    expect(serialized).not.toContain('secret123hunter');
    expect(serialized).toContain('[REDACTED');
  });

  it('persists a clean observation unchanged', async () => {
    const { store, writes } = spyStore();
    const svc = new KindlingService(store, enabledConfig);

    await svc.emit(makeObservation('ls -la /home/user'));

    expect(writes).toHaveLength(1);
    expect(JSON.stringify(writes[0])).toContain('ls -la /home/user');
  });

  it('does not persist anything when kindling is disabled', async () => {
    const { store, writes } = spyStore();
    const svc = new KindlingService(store, disabledConfig);

    await svc.emit(makeObservation('mysql -u root --password=secret123hunter'));

    expect(writes).toHaveLength(0);
  });
});
