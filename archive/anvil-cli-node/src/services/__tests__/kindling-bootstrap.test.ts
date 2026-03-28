import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { randomUUID } from 'node:crypto';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';

// vi.hoisted ensures these are available when vi.mock factories execute
const mocks = vi.hoisted(() => {
  const mockDb = {
    pragma: vi.fn(),
    prepare: vi.fn().mockReturnValue({ run: vi.fn(), get: vi.fn(), all: vi.fn() }),
    close: vi.fn(),
  };

  const mockCoreServiceInstance = {
    openCapsule: vi.fn().mockReturnValue({
      id: 'capsule-001',
      type: 'session',
      status: 'open',
      scopeIds: {},
      openedAt: Date.now(),
    }),
    closeCapsule: vi.fn().mockReturnValue({
      id: 'capsule-001',
      type: 'session',
      status: 'closed',
      scopeIds: {},
      openedAt: Date.now(),
      closedAt: Date.now(),
    }),
    appendObservation: vi.fn(),
    retrieve: vi.fn().mockResolvedValue({ pins: [], candidates: [], provenance: {} }),
  };

  const openDatabaseFn = vi.fn().mockReturnValue(mockDb);
  const closeDatabaseFn = vi.fn();

  return {
    mockDb,
    mockCoreServiceInstance,
    openDatabaseFn,
    closeDatabaseFn,
  };
});

vi.mock('@eddacraft/kindling-store-sqlite', () => {
  class MockSqliteKindlingStore {
    insertObservation = vi.fn();
    createCapsule = vi.fn();
    closeCapsule = vi.fn();
    attachObservationToCapsule = vi.fn();
    createSummary = vi.fn();
    createPin = vi.fn();
    removePin = vi.fn();
    redactObservation = vi.fn();
    getCapsule = vi.fn();
    getOpenCapsuleForSession = vi.fn();
    getObservationById = vi.fn();
    getSummaryById = vi.fn();
    getLatestSummaryForCapsule = vi.fn();
    listActivePins = vi.fn().mockReturnValue([]);

    constructor(_db: unknown) {}
  }

  return {
    openDatabase: mocks.openDatabaseFn,
    closeDatabase: mocks.closeDatabaseFn,
    SqliteKindlingStore: MockSqliteKindlingStore,
  };
});

vi.mock('@eddacraft/kindling-provider-local', () => {
  class MockLocalFtsProvider {
    name = 'local-fts';
    search = vi.fn().mockResolvedValue([]);

    constructor(_db: unknown) {}
  }

  return { LocalFtsProvider: MockLocalFtsProvider };
});

vi.mock('@eddacraft/kindling-core', () => {
  class MockKindlingService {
    openCapsule = mocks.mockCoreServiceInstance.openCapsule;
    closeCapsule = mocks.mockCoreServiceInstance.closeCapsule;
    appendObservation = mocks.mockCoreServiceInstance.appendObservation;
    retrieve = mocks.mockCoreServiceInstance.retrieve;

    constructor(_config: unknown) {}
  }

  return { KindlingService: MockKindlingService };
});

import { initKindling } from '../kindling-bootstrap.js';
import {
  emitSessionStart,
  emitSessionEnd,
  emitGateEvaluated,
} from '@eddacraft/anvil-kindling-integration';

let tempDir: string;

beforeEach(() => {
  tempDir = mkdtempSync(join(tmpdir(), 'anvil-kindling-test-'));
  mocks.mockCoreServiceInstance.appendObservation.mockClear();
  mocks.mockCoreServiceInstance.openCapsule.mockClear();
  mocks.mockCoreServiceInstance.closeCapsule.mockClear();
  mocks.openDatabaseFn.mockClear();
  mocks.closeDatabaseFn.mockClear();
});

afterEach(async () => {
  vi.restoreAllMocks();
  if (tempDir) {
    await safeCleanup(tempDir);
  }
});

describe('initKindling', () => {
  it('returns null when no config file exists (disabled by default)', () => {
    const result = initKindling(tempDir);
    expect(result).toBeNull();
  });

  it('returns null when kindling is explicitly disabled', () => {
    writeFileSync(join(tempDir, '.anvilrc'), JSON.stringify({ kindling: { enabled: false } }));

    const result = initKindling(tempDir);
    expect(result).toBeNull();
  });

  it('returns KindlingContext when enabled', () => {
    writeFileSync(
      join(tempDir, '.anvilrc'),
      JSON.stringify({ kindling: { enabled: true, database_path: '.anvil/kindling.db' } })
    );

    const result = initKindling(tempDir);

    expect(result).not.toBeNull();
    expect(result!.service).toBeDefined();
    expect(result!.adapter).toBeDefined();
    expect(result!.bridge).toBeDefined();
    expect(result!.config.enabled).toBe(true);
    expect(typeof result!.close).toBe('function');

    result!.close();
  });

  it('resolves database path relative to workspace root', () => {
    writeFileSync(
      join(tempDir, '.anvilrc'),
      JSON.stringify({ kindling: { enabled: true, database_path: 'deep/nested/kindling.db' } })
    );

    const result = initKindling(tempDir);
    expect(result).not.toBeNull();

    expect(mocks.openDatabaseFn).toHaveBeenCalledWith(
      expect.objectContaining({
        path: join(tempDir, 'deep/nested/kindling.db'),
      })
    );

    result!.close();
  });

  it('calls closeDatabase on close()', () => {
    writeFileSync(join(tempDir, '.anvilrc'), JSON.stringify({ kindling: { enabled: true } }));

    const ctx = initKindling(tempDir)!;
    ctx.close();

    expect(mocks.closeDatabaseFn).toHaveBeenCalledWith(mocks.mockDb);
  });
});

describe('KindlingContext integration', () => {
  it('emitSessionStart returns a session ID', () => {
    writeFileSync(join(tempDir, '.anvilrc'), JSON.stringify({ kindling: { enabled: true } }));

    const ctx = initKindling(tempDir)!;

    const sessionId = emitSessionStart(ctx.service, {
      working_directory: tempDir,
      anvil_version: '0.1.0',
      command: 'test',
      args: [],
      environment: 'development',
    });

    expect(sessionId).toBeDefined();
    expect(typeof sessionId).toBe('string');
    expect(sessionId.length).toBeGreaterThan(0);

    ctx.close();
  });

  it('adapter opens and closes capsules', () => {
    writeFileSync(join(tempDir, '.anvilrc'), JSON.stringify({ kindling: { enabled: true } }));

    const ctx = initKindling(tempDir)!;

    const capsule = ctx.adapter.startSession('sess-001', 'test intent');
    expect(capsule).toBeDefined();
    expect(capsule.id).toBe('capsule-001');

    const closed = ctx.adapter.endSession(capsule.id);
    expect(closed).toBeDefined();

    expect(mocks.mockCoreServiceInstance.openCapsule).toHaveBeenCalledTimes(1);
    expect(mocks.mockCoreServiceInstance.closeCapsule).toHaveBeenCalledTimes(1);

    ctx.close();
  });

  it('bridge forwards observations to kindling-core with capsule ID', async () => {
    writeFileSync(join(tempDir, '.anvilrc'), JSON.stringify({ kindling: { enabled: true } }));

    const ctx = initKindling(tempDir)!;
    ctx.bridge.setCapsuleId('capsule-001');

    emitSessionStart(ctx.service, {
      working_directory: tempDir,
      anvil_version: '0.1.0',
      command: 'test',
      args: [],
      environment: 'development',
    });

    // Give fire-and-forget a tick to execute
    await new Promise((resolve) => setTimeout(resolve, 20));

    expect(mocks.mockCoreServiceInstance.appendObservation).toHaveBeenCalledWith(
      expect.objectContaining({
        kind: 'message', // session_start maps to 'message'
        provenance: expect.objectContaining({
          anvil_kind: 'session_start',
        }),
      }),
      expect.objectContaining({
        capsuleId: 'capsule-001',
        validate: true,
      })
    );

    ctx.close();
  });

  it('emitters are fire-and-forget (do not throw)', () => {
    writeFileSync(join(tempDir, '.anvilrc'), JSON.stringify({ kindling: { enabled: true } }));

    const ctx = initKindling(tempDir)!;

    expect(() => {
      emitSessionStart(ctx.service, {
        working_directory: tempDir,
        anvil_version: '0.1.0',
        command: 'test',
        args: [],
        environment: 'development',
      });
    }).not.toThrow();

    expect(() => {
      emitSessionEnd(ctx.service, 'session-123', {
        outcome: 'success',
        exit_code: 0,
        duration_ms: 100,
        summary: {
          gates_evaluated: 0,
          gates_passed: 0,
          gates_failed: 0,
          actions_executed: 0,
          errors_encountered: 0,
        },
      });
    }).not.toThrow();

    expect(() => {
      emitGateEvaluated(ctx.service, {
        session_id: 'session-123',
        gate_id: 'test-gate',
        inputs: { file_count: 1 },
        outcome: 'pass',
        rules_evaluated: ['architecture'],
        enforcement: 'blocking',
        duration_ms: 50,
      });
    }).not.toThrow();

    ctx.close();
  });

  it('gate_evaluated maps to command kind in kindling-core', async () => {
    writeFileSync(join(tempDir, '.anvilrc'), JSON.stringify({ kindling: { enabled: true } }));

    const ctx = initKindling(tempDir)!;
    ctx.bridge.setCapsuleId('capsule-001');

    // Use a valid UUID — the observation schema requires session_id to be a UUID
    const testSessionId = randomUUID();

    emitGateEvaluated(ctx.service, {
      session_id: testSessionId,
      gate_id: 'architecture-check',
      inputs: { file_count: 5, changed_files: ['a.ts', 'b.ts'] },
      outcome: 'pass',
      rules_evaluated: ['architecture'],
      enforcement: 'blocking',
      duration_ms: 200,
    });

    await new Promise((resolve) => setTimeout(resolve, 20));

    expect(mocks.mockCoreServiceInstance.appendObservation).toHaveBeenCalledWith(
      expect.objectContaining({
        kind: 'command', // gate_evaluated maps to 'command'
        provenance: expect.objectContaining({
          anvil_kind: 'gate_evaluated',
        }),
      }),
      expect.objectContaining({
        capsuleId: 'capsule-001',
      })
    );

    ctx.close();
  });
});
