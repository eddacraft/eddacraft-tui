/**
 * Action Emitter Tests (KINDLING-005)
 *
 * Covers emitActionExecuted.
 * Action observations record what commands, tool invocations, or file
 * operations actually occurred and link them to governing gates/plans.
 */

import { describe, it, expect } from 'vitest';
import { KindlingService, type IKindlingStore } from '../kindling-service.js';
import { KindlingConfigSchema } from '../config.js';
import type { Observation } from '../observation-contract.js';
import type { QueryRequest, QueryResponse } from '../query-contract.js';
import { emitActionExecuted, type ActionDetails } from './action-emitter.js';

// =============================================================================
// Test Helpers
// =============================================================================

const enabledConfig = KindlingConfigSchema.parse({ enabled: true });

function makeSpyStore(): { store: IKindlingStore; emits: Observation[] } {
  const emits: Observation[] = [];
  const store: IKindlingStore = {
    emit: async (o) => {
      emits.push(o);
    },
    query: async (_req: QueryRequest): Promise<QueryResponse> => {
      throw new Error('query not used in emitter tests');
    },
    close: async () => {},
  };
  return { store, emits };
}

function makeService(store: IKindlingStore): KindlingService {
  return new KindlingService(store, enabledConfig);
}

function makeActionDetails(overrides: Partial<ActionDetails> = {}): ActionDetails {
  return {
    session_id: '550e8400-e29b-41d4-a716-446655440000',
    action_type: 'command',
    details: {
      command: 'git status',
      working_directory: '/home/user/project',
    },
    outcome: 'success',
    duration_ms: 45,
    ...overrides,
  };
}

// =============================================================================
// emitActionExecuted
// =============================================================================

describe('emitActionExecuted', () => {
  it('returns a valid UUID action_id', () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const actionId = emitActionExecuted(svc, makeActionDetails());
    expect(actionId).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i
    );
  });

  it('returns a different action_id on each call', () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const id1 = emitActionExecuted(svc, makeActionDetails());
    const id2 = emitActionExecuted(svc, makeActionDetails());
    expect(id1).not.toBe(id2);
  });

  it('emits an observation with kind action_executed', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitActionExecuted(svc, makeActionDetails());
    await new Promise((r) => setImmediate(r));
    expect(emits).toHaveLength(1);
    expect(emits[0].kind).toBe('action_executed');
  });

  it('emits action_id matching the returned value', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    const actionId = emitActionExecuted(svc, makeActionDetails());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
    expect(obs.action_id).toBe(actionId);
  });

  it('emits the provided session_id', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitActionExecuted(
      svc,
      makeActionDetails({ session_id: '550e8400-e29b-41d4-a716-446655440000' })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
    expect(obs.session_id).toBe('550e8400-e29b-41d4-a716-446655440000');
  });

  it('emits the action_type', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitActionExecuted(svc, makeActionDetails({ action_type: 'file_write' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
    expect(obs.action_type).toBe('file_write');
  });

  it('supports all valid action_type values', async () => {
    const types = [
      'command',
      'tool_invocation',
      'file_write',
      'file_delete',
      'diff_apply',
    ] as const;
    for (const action_type of types) {
      const { store, emits } = makeSpyStore();
      const svc = makeService(store);
      emitActionExecuted(svc, makeActionDetails({ action_type }));
      await new Promise((r) => setImmediate(r));
      const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
      expect(obs.action_type).toBe(action_type);
    }
  });

  it('emits details.command', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitActionExecuted(
      svc,
      makeActionDetails({ details: { command: 'ls -la', working_directory: '/tmp' } })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
    expect(obs.details.command).toBe('ls -la');
  });

  it('emits details.tool_name when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitActionExecuted(
      svc,
      makeActionDetails({
        action_type: 'tool_invocation',
        details: { tool_name: 'anvil-check', working_directory: '/home/user/project' },
      })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
    expect(obs.details.tool_name).toBe('anvil-check');
  });

  it('emits details.file_paths when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    const paths = ['src/a.ts', 'src/b.ts'];
    emitActionExecuted(
      svc,
      makeActionDetails({
        action_type: 'file_write',
        details: { file_paths: paths, working_directory: '/home/user/project' },
      })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
    expect(obs.details.file_paths).toEqual(paths);
  });

  it('emits details.diff_summary when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    const diff_summary = { additions: 10, deletions: 3, files_changed: 2 };
    emitActionExecuted(
      svc,
      makeActionDetails({
        action_type: 'diff_apply',
        details: { diff_summary, working_directory: '/home/user/project' },
      })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
    expect(obs.details.diff_summary).toEqual(diff_summary);
  });

  it('emits details.working_directory', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitActionExecuted(
      svc,
      makeActionDetails({ details: { working_directory: '/workspace/myapp' } })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
    expect(obs.details.working_directory).toBe('/workspace/myapp');
  });

  it('emits details.environment_target when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitActionExecuted(
      svc,
      makeActionDetails({
        details: {
          working_directory: '/home/user/project',
          environment_target: 'staging',
        },
      })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
    expect(obs.details.environment_target).toBe('staging');
  });

  it('emits governed_by_gate_id when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitActionExecuted(svc, makeActionDetails({ governed_by_gate_id: 'gate-eval-abc-123' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
    expect(obs.governed_by_gate_id).toBe('gate-eval-abc-123');
  });

  it('omits governed_by_gate_id when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitActionExecuted(svc, makeActionDetails());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
    expect(obs.governed_by_gate_id).toBeUndefined();
  });

  it('emits governed_by_plan_id when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitActionExecuted(svc, makeActionDetails({ governed_by_plan_id: 'plan-xyz-789' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
    expect(obs.governed_by_plan_id).toBe('plan-xyz-789');
  });

  it('emits the outcome', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitActionExecuted(svc, makeActionDetails({ outcome: 'partial' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
    expect(obs.outcome).toBe('partial');
  });

  it('supports all valid outcome values', async () => {
    const outcomes = ['success', 'failure', 'partial'] as const;
    for (const outcome of outcomes) {
      const { store, emits } = makeSpyStore();
      const svc = makeService(store);
      emitActionExecuted(svc, makeActionDetails({ outcome }));
      await new Promise((r) => setImmediate(r));
      const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
      expect(obs.outcome).toBe(outcome);
    }
  });

  it('emits exit_code when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitActionExecuted(svc, makeActionDetails({ outcome: 'failure', exit_code: 127 }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
    expect(obs.exit_code).toBe(127);
  });

  it('omits exit_code when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitActionExecuted(svc, makeActionDetails());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
    expect(obs.exit_code).toBeUndefined();
  });

  it('emits the duration_ms', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitActionExecuted(svc, makeActionDetails({ duration_ms: 1234 }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'action_executed' }>;
    expect(obs.duration_ms).toBe(1234);
  });

  it('emits a Z-suffixed ISO8601 timestamp', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitActionExecuted(svc, makeActionDetails());
    await new Promise((r) => setImmediate(r));
    expect(emits[0].timestamp).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z$/);
  });

  it('does not emit when service is disabled', async () => {
    const disabledConfig = KindlingConfigSchema.parse({ enabled: false });
    const { store, emits } = makeSpyStore();
    const svc = new KindlingService(store, disabledConfig);
    emitActionExecuted(svc, makeActionDetails());
    await new Promise((r) => setImmediate(r));
    expect(emits).toHaveLength(0);
  });

  it('swallows store errors silently (fire-and-forget)', async () => {
    const failStore: IKindlingStore = {
      emit: async () => {
        throw new Error('store exploded');
      },
      query: async (_req: QueryRequest): Promise<QueryResponse> => {
        throw new Error('not used');
      },
      close: async () => {},
    };
    const svc = makeService(failStore);
    expect(() => emitActionExecuted(svc, makeActionDetails())).not.toThrow();
    await new Promise((r) => setImmediate(r));
  });
});
