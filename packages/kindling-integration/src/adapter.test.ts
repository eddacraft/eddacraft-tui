/**
 * AnvilKindlingAdapter tests (TCOV-018)
 *
 * Covers: adapter construction, startSession, endSession, emit, and
 * emitGateEvaluated. The adapter depends on @eddacraft/kindling-core's
 * KindlingService; we use a hand-rolled mock to keep tests fast and
 * avoid requiring a real store/provider.
 */

import { describe, it, expect, vi } from 'vitest';
import { AnvilKindlingAdapter } from './adapter.js';
import type {
  GateEvaluatedObservation,
  Observation as AnvilObservation,
} from './observation-contract.js';
import type { KindlingService, Capsule, ID } from '@eddacraft/kindling-core';

// =============================================================================
// Test helpers
// =============================================================================

const SESSION_UUID = 'aaaabbbb-cccc-4ddd-aeee-ffffffffffff';
const VALID_TIMESTAMP = '2026-02-15T10:00:00.000Z';

/** A capsule-shaped object sufficient for the adapter's return types */
function makeCapsule(id: ID = 'cap-001'): Capsule {
  return {
    id,
    type: 'session',
    intent: 'test',
    scopeIds: { sessionId: SESSION_UUID },
    openedAt: Date.now(),
  } as unknown as Capsule;
}

/**
 * Build a minimal KindlingService mock.
 * Records calls to openCapsule, closeCapsule, appendObservation.
 */
function makeMockService() {
  const calls: {
    openCapsule: Parameters<KindlingService['openCapsule']>[];
    closeCapsule: Parameters<KindlingService['closeCapsule']>[];
    appendObservation: unknown[];
  } = {
    openCapsule: [],
    closeCapsule: [],
    appendObservation: [],
  };

  const mockService = {
    openCapsule: vi.fn((opts) => {
      calls.openCapsule.push(opts as never);
      return makeCapsule();
    }),
    closeCapsule: vi.fn((capsuleId, opts) => {
      calls.closeCapsule.push([capsuleId, opts] as never);
      return makeCapsule(capsuleId);
    }),
    appendObservation: vi.fn((obs, opts) => {
      calls.appendObservation.push({ obs, opts });
    }),
  } as unknown as KindlingService;

  return { mockService, calls };
}

function makeGateEvaluated(): GateEvaluatedObservation {
  return {
    kind: 'gate_evaluated',
    session_id: SESSION_UUID,
    timestamp: VALID_TIMESTAMP,
    gate_eval_id: 'ge-001',
    gate_id: 'architecture',
    inputs: {},
    outcome: 'pass',
    rules_evaluated: ['rule-a'],
    enforcement: 'blocking',
    duration_ms: 50,
  };
}

function makeSessionStart(): AnvilObservation {
  return {
    kind: 'session_start',
    session_id: SESSION_UUID,
    timestamp: VALID_TIMESTAMP,
    context: {
      working_directory: '/project',
      anvil_version: '0.8.0',
      command: 'check',
      args: [],
      environment: 'development',
    },
  };
}

// =============================================================================
// AnvilKindlingAdapter construction
// =============================================================================

describe('AnvilKindlingAdapter — construction', () => {
  it('constructs without error when repoId is provided', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService, repoId: '/path/to/repo' });
    expect(adapter).toBeInstanceOf(AnvilKindlingAdapter);
  });

  it('constructs without error when repoId is omitted', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    expect(adapter).toBeInstanceOf(AnvilKindlingAdapter);
  });
});

// =============================================================================
// startSession
// =============================================================================

describe('AnvilKindlingAdapter.startSession', () => {
  it('calls service.openCapsule with type=session', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService, repoId: '/repo' });

    adapter.startSession(SESSION_UUID, 'check');

    expect(mockService.openCapsule).toHaveBeenCalledTimes(1);
    const arg = (mockService.openCapsule as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(arg.type).toBe('session');
  });

  it('passes the sessionId in scopeIds', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.startSession(SESSION_UUID, 'watch');
    const arg = (mockService.openCapsule as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(arg.scopeIds?.sessionId).toBe(SESSION_UUID);
  });

  it('passes intent through to openCapsule', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.startSession(SESSION_UUID, 'my-intent');
    const arg = (mockService.openCapsule as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(arg.intent).toBe('my-intent');
  });

  it('passes repoId in scopeIds when provided', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService, repoId: '/some/repo' });
    adapter.startSession(SESSION_UUID, 'check');
    const arg = (mockService.openCapsule as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(arg.scopeIds?.repoId).toBe('/some/repo');
  });

  it('leaves repoId undefined in scopeIds when not configured', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.startSession(SESSION_UUID, 'check');
    const arg = (mockService.openCapsule as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(arg.scopeIds?.repoId).toBeUndefined();
  });

  it('returns the capsule from openCapsule', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    const capsule = adapter.startSession(SESSION_UUID, 'check');
    expect(capsule).toBeDefined();
    expect(capsule.id).toBeDefined();
  });
});

// =============================================================================
// endSession
// =============================================================================

describe('AnvilKindlingAdapter.endSession', () => {
  it('calls service.closeCapsule with the provided capsuleId', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.endSession('cap-001' as ID);
    expect(mockService.closeCapsule).toHaveBeenCalledTimes(1);
    const [id] = (mockService.closeCapsule as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(id).toBe('cap-001');
  });

  it('passes generateSummary=false when no summaryContent provided', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.endSession('cap-001' as ID);
    const [, opts] = (mockService.closeCapsule as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(opts.generateSummary).toBe(false);
  });

  it('passes generateSummary=true and summaryContent when provided', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.endSession('cap-001' as ID, 'Session completed: 3 gates passed');
    const [, opts] = (mockService.closeCapsule as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(opts.generateSummary).toBe(true);
    expect(opts.summaryContent).toBe('Session completed: 3 gates passed');
  });

  it('returns the closed capsule', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    const capsule = adapter.endSession('cap-001' as ID);
    expect(capsule).toBeDefined();
  });
});

// =============================================================================
// emit
// =============================================================================

describe('AnvilKindlingAdapter.emit', () => {
  it('calls service.appendObservation', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.emit(makeSessionStart());
    expect(mockService.appendObservation).toHaveBeenCalledTimes(1);
  });

  it('serializes the Anvil observation into content as JSON', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    const obs = makeSessionStart();
    adapter.emit(obs);
    const mockCalls = (mockService.appendObservation as ReturnType<typeof vi.fn>).mock.calls;
    const kindlingObs = mockCalls[0][0];
    const parsed = JSON.parse(kindlingObs.content);
    expect(parsed.kind).toBe('session_start');
  });

  it('maps session_start kind to "message"', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.emit(makeSessionStart());
    const kindlingObs = (mockService.appendObservation as ReturnType<typeof vi.fn>).mock
      .calls[0][0];
    expect(kindlingObs.kind).toBe('message');
  });

  it('maps gate_evaluated kind to "command"', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.emit(makeGateEvaluated());
    const kindlingObs = (mockService.appendObservation as ReturnType<typeof vi.fn>).mock
      .calls[0][0];
    expect(kindlingObs.kind).toBe('command');
  });

  it('maps error kind to "error"', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    const errObs: AnvilObservation = {
      kind: 'error',
      session_id: SESSION_UUID,
      timestamp: VALID_TIMESTAMP,
      error_id: 'err-001',
      error_type: 'command_failure',
      context: { component: 'gate:arch' },
      error_message: 'Failed',
      recoverable: false,
    };
    adapter.emit(errObs);
    const kindlingObs = (mockService.appendObservation as ReturnType<typeof vi.fn>).mock
      .calls[0][0];
    expect(kindlingObs.kind).toBe('error');
  });

  it('preserves the original Anvil kind in provenance.anvil_kind', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.emit(makeGateEvaluated());
    const kindlingObs = (mockService.appendObservation as ReturnType<typeof vi.fn>).mock
      .calls[0][0];
    expect(kindlingObs.provenance.anvil_kind).toBe('gate_evaluated');
  });

  it('includes the session_id in scopeIds', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.emit(makeSessionStart());
    const kindlingObs = (mockService.appendObservation as ReturnType<typeof vi.fn>).mock
      .calls[0][0];
    expect(kindlingObs.scopeIds.sessionId).toBe(SESSION_UUID);
  });

  it('includes repoId in scopeIds when configured', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService, repoId: '/my/repo' });
    adapter.emit(makeSessionStart());
    const kindlingObs = (mockService.appendObservation as ReturnType<typeof vi.fn>).mock
      .calls[0][0];
    expect(kindlingObs.scopeIds.repoId).toBe('/my/repo');
  });

  it('generates a UUID for the observation id', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.emit(makeSessionStart());
    const kindlingObs = (mockService.appendObservation as ReturnType<typeof vi.fn>).mock
      .calls[0][0];
    expect(kindlingObs.id).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/
    );
  });

  it('sets redacted=false on the kindling observation', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.emit(makeSessionStart());
    const kindlingObs = (mockService.appendObservation as ReturnType<typeof vi.fn>).mock
      .calls[0][0];
    expect(kindlingObs.redacted).toBe(false);
  });

  it('passes capsuleId to appendObservation when provided', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.emit(makeSessionStart(), 'cap-xyz' as ID);
    const opts = (mockService.appendObservation as ReturnType<typeof vi.fn>).mock.calls[0][1];
    expect(opts.capsuleId).toBe('cap-xyz');
  });

  it('passes validate=true to appendObservation', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.emit(makeSessionStart());
    const opts = (mockService.appendObservation as ReturnType<typeof vi.fn>).mock.calls[0][1];
    expect(opts.validate).toBe(true);
  });

  it('sets ts as a number (ms since epoch)', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    const before = Date.now();
    adapter.emit(makeSessionStart());
    const after = Date.now();
    const kindlingObs = (mockService.appendObservation as ReturnType<typeof vi.fn>).mock
      .calls[0][0];
    expect(typeof kindlingObs.ts).toBe('number');
    expect(kindlingObs.ts).toBeGreaterThanOrEqual(before);
    expect(kindlingObs.ts).toBeLessThanOrEqual(after);
  });

  it('includes contract version in provenance', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.emit(makeSessionStart());
    const kindlingObs = (mockService.appendObservation as ReturnType<typeof vi.fn>).mock
      .calls[0][0];
    expect(kindlingObs.provenance.anvil_contract_version).toBe('1.0.0');
  });
});

// =============================================================================
// emitGateEvaluated
// =============================================================================

describe('AnvilKindlingAdapter.emitGateEvaluated', () => {
  it('delegates to emit() — same appendObservation call', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    const gate = makeGateEvaluated();

    adapter.emitGateEvaluated(gate);

    expect(mockService.appendObservation).toHaveBeenCalledTimes(1);
    const kindlingObs = (mockService.appendObservation as ReturnType<typeof vi.fn>).mock
      .calls[0][0];
    expect(kindlingObs.provenance.anvil_kind).toBe('gate_evaluated');
  });

  it('passes through the capsuleId to emit', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.emitGateEvaluated(makeGateEvaluated(), 'cap-gate' as ID);
    const opts = (mockService.appendObservation as ReturnType<typeof vi.fn>).mock.calls[0][1];
    expect(opts.capsuleId).toBe('cap-gate');
  });

  it('maps gate_evaluated to command kind in kindling', () => {
    const { mockService } = makeMockService();
    const adapter = new AnvilKindlingAdapter({ service: mockService });
    adapter.emitGateEvaluated(makeGateEvaluated());
    const kindlingObs = (mockService.appendObservation as ReturnType<typeof vi.fn>).mock
      .calls[0][0];
    expect(kindlingObs.kind).toBe('command');
  });
});
