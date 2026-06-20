/**
 * Plan Emitter Tests (KINDLING-006)
 *
 * Covers emitPlanCreated, emitPlanEdited, emitPlanApproved, emitPlanRejected.
 * Plans are the governance artifacts that authorize actions.
 */

import { describe, it, expect } from 'vitest';
import { KindlingService, type IKindlingStore } from '../kindling-service.js';
import { KindlingConfigSchema } from '../config.js';
import type { Observation } from '../observation-contract.js';
import type { QueryRequest, QueryResponse } from '../query-contract.js';
import {
  emitPlanCreated,
  emitPlanEdited,
  emitPlanApproved,
  emitPlanRejected,
  type PlanCreatedInput,
  type PlanEditedInput,
  type PlanApprovedInput,
  type PlanRejectedInput,
} from './plan-emitter.js';

// =============================================================================
// Test Helpers
// =============================================================================

const enabledConfig = KindlingConfigSchema.parse({ enabled: true });
const VALID_UUID = '550e8400-e29b-41d4-a716-446655440000';
const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const TIMESTAMP_RE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z$/;

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

function makePlanCreatedInput(overrides: Partial<PlanCreatedInput> = {}): PlanCreatedInput {
  return {
    session_id: VALID_UUID,
    plan_version: '1.0',
    plan_path: 'plans/my-plan.aps.md',
    plan_hash: 'sha256:abc123',
    created_by: 'human',
    ...overrides,
  };
}

function makePlanEditedInput(overrides: Partial<PlanEditedInput> = {}): PlanEditedInput {
  return {
    session_id: VALID_UUID,
    plan_id: 'plan-001',
    previous_version: '1.0',
    new_version: '1.1',
    previous_hash: 'sha256:old',
    new_hash: 'sha256:new',
    edited_by: 'human',
    ...overrides,
  };
}

function makePlanApprovedInput(overrides: Partial<PlanApprovedInput> = {}): PlanApprovedInput {
  return {
    session_id: VALID_UUID,
    plan_id: 'plan-001',
    plan_version: '1.0',
    approved_by: 'joshuaboys',
    approval_method: 'cli_confirm',
    ...overrides,
  };
}

function makePlanRejectedInput(overrides: Partial<PlanRejectedInput> = {}): PlanRejectedInput {
  return {
    session_id: VALID_UUID,
    plan_id: 'plan-001',
    plan_version: '1.0',
    rejected_by: 'joshuaboys',
    ...overrides,
  };
}

// =============================================================================
// emitPlanCreated
// =============================================================================

describe('emitPlanCreated', () => {
  it('returns the provided plan_id when given', async () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const returned = emitPlanCreated(svc, makePlanCreatedInput({ plan_id: 'plan-explicit' }));
    expect(returned).toBe('plan-explicit');
  });

  it('generates a UUID plan_id when not provided', () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const returned = emitPlanCreated(svc, makePlanCreatedInput());
    expect(returned).toMatch(UUID_RE);
  });

  it('generates a different plan_id on each call without plan_id', () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const id1 = emitPlanCreated(svc, makePlanCreatedInput());
    const id2 = emitPlanCreated(svc, makePlanCreatedInput());
    expect(id1).not.toBe(id2);
  });

  it('emits an observation with kind plan_created', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanCreated(svc, makePlanCreatedInput());
    await new Promise((r) => setImmediate(r));
    expect(emits).toHaveLength(1);
    expect(emits[0].kind).toBe('plan_created');
  });

  it('emits plan_id matching the returned value', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    const planId = emitPlanCreated(svc, makePlanCreatedInput());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_created' }>;
    expect(obs.plan_id).toBe(planId);
  });

  it('emits the provided session_id', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanCreated(svc, makePlanCreatedInput());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_created' }>;
    expect(obs.session_id).toBe(VALID_UUID);
  });

  it('emits the plan_version', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanCreated(svc, makePlanCreatedInput({ plan_version: '2.0' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_created' }>;
    expect(obs.plan_version).toBe('2.0');
  });

  it('emits the plan_path', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanCreated(svc, makePlanCreatedInput({ plan_path: 'plans/custom.aps.md' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_created' }>;
    expect(obs.plan_path).toBe('plans/custom.aps.md');
  });

  it('emits the plan_hash', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanCreated(svc, makePlanCreatedInput({ plan_hash: 'sha256:deadbeef' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_created' }>;
    expect(obs.plan_hash).toBe('sha256:deadbeef');
  });

  it('supports all valid created_by values', async () => {
    const creators = ['human', 'ai', 'system'] as const;
    for (const created_by of creators) {
      const { store, emits } = makeSpyStore();
      const svc = makeService(store);
      emitPlanCreated(svc, makePlanCreatedInput({ created_by }));
      await new Promise((r) => setImmediate(r));
      const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_created' }>;
      expect(obs.created_by).toBe(created_by);
    }
  });

  it('emits optional source when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanCreated(svc, makePlanCreatedInput({ source: 'github-issue-42' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_created' }>;
    expect(obs.source).toBe('github-issue-42');
  });

  it('omits source when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanCreated(svc, makePlanCreatedInput());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_created' }>;
    expect(obs.source).toBeUndefined();
  });

  it('emits a Z-suffixed ISO8601 timestamp', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanCreated(svc, makePlanCreatedInput());
    await new Promise((r) => setImmediate(r));
    expect(emits[0].timestamp).toMatch(TIMESTAMP_RE);
  });

  it('does not emit when service is disabled', async () => {
    const disabledConfig = KindlingConfigSchema.parse({ enabled: false });
    const { store, emits } = makeSpyStore();
    const svc = new KindlingService(store, disabledConfig);
    emitPlanCreated(svc, makePlanCreatedInput());
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
    expect(() => emitPlanCreated(svc, makePlanCreatedInput())).not.toThrow();
    await new Promise((r) => setImmediate(r));
  });
});

// =============================================================================
// emitPlanEdited
// =============================================================================

describe('emitPlanEdited', () => {
  it('returns the plan_id', () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const returned = emitPlanEdited(svc, makePlanEditedInput({ plan_id: 'plan-edited-001' }));
    expect(returned).toBe('plan-edited-001');
  });

  it('emits an observation with kind plan_edited', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanEdited(svc, makePlanEditedInput());
    await new Promise((r) => setImmediate(r));
    expect(emits).toHaveLength(1);
    expect(emits[0].kind).toBe('plan_edited');
  });

  it('emits the plan_id', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanEdited(svc, makePlanEditedInput({ plan_id: 'plan-edit-42' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_edited' }>;
    expect(obs.plan_id).toBe('plan-edit-42');
  });

  it('emits previous_version and new_version', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanEdited(svc, makePlanEditedInput({ previous_version: '1.0', new_version: '1.2' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_edited' }>;
    expect(obs.previous_version).toBe('1.0');
    expect(obs.new_version).toBe('1.2');
  });

  it('emits previous_hash and new_hash', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanEdited(
      svc,
      makePlanEditedInput({ previous_hash: 'sha256:old', new_hash: 'sha256:new' })
    );
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_edited' }>;
    expect(obs.previous_hash).toBe('sha256:old');
    expect(obs.new_hash).toBe('sha256:new');
  });

  it('supports all valid edited_by values', async () => {
    const editors = ['human', 'ai', 'system'] as const;
    for (const edited_by of editors) {
      const { store, emits } = makeSpyStore();
      const svc = makeService(store);
      emitPlanEdited(svc, makePlanEditedInput({ edited_by }));
      await new Promise((r) => setImmediate(r));
      const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_edited' }>;
      expect(obs.edited_by).toBe(edited_by);
    }
  });

  it('emits change_summary when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanEdited(svc, makePlanEditedInput({ change_summary: 'Added step 4' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_edited' }>;
    expect(obs.change_summary).toBe('Added step 4');
  });

  it('omits change_summary when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanEdited(svc, makePlanEditedInput());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_edited' }>;
    expect(obs.change_summary).toBeUndefined();
  });

  it('emits a Z-suffixed ISO8601 timestamp', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanEdited(svc, makePlanEditedInput());
    await new Promise((r) => setImmediate(r));
    expect(emits[0].timestamp).toMatch(TIMESTAMP_RE);
  });

  it('does not emit when service is disabled', async () => {
    const disabledConfig = KindlingConfigSchema.parse({ enabled: false });
    const { store, emits } = makeSpyStore();
    const svc = new KindlingService(store, disabledConfig);
    emitPlanEdited(svc, makePlanEditedInput());
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
    expect(() => emitPlanEdited(svc, makePlanEditedInput())).not.toThrow();
    await new Promise((r) => setImmediate(r));
  });
});

// =============================================================================
// emitPlanApproved
// =============================================================================

describe('emitPlanApproved', () => {
  it('returns the plan_id', () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const returned = emitPlanApproved(svc, makePlanApprovedInput({ plan_id: 'plan-approve-001' }));
    expect(returned).toBe('plan-approve-001');
  });

  it('emits an observation with kind plan_approved', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanApproved(svc, makePlanApprovedInput());
    await new Promise((r) => setImmediate(r));
    expect(emits).toHaveLength(1);
    expect(emits[0].kind).toBe('plan_approved');
  });

  it('emits the plan_id', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanApproved(svc, makePlanApprovedInput({ plan_id: 'plan-approve-99' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_approved' }>;
    expect(obs.plan_id).toBe('plan-approve-99');
  });

  it('emits the plan_version', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanApproved(svc, makePlanApprovedInput({ plan_version: '3.0' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_approved' }>;
    expect(obs.plan_version).toBe('3.0');
  });

  it('emits the approved_by', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanApproved(svc, makePlanApprovedInput({ approved_by: 'alice' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_approved' }>;
    expect(obs.approved_by).toBe('alice');
  });

  it('supports all valid approval_method values', async () => {
    const methods = ['cli_confirm', 'explicit_flag', 'ci_gate'] as const;
    for (const approval_method of methods) {
      const { store, emits } = makeSpyStore();
      const svc = makeService(store);
      emitPlanApproved(svc, makePlanApprovedInput({ approval_method }));
      await new Promise((r) => setImmediate(r));
      const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_approved' }>;
      expect(obs.approval_method).toBe(approval_method);
    }
  });

  it('emits a Z-suffixed ISO8601 timestamp', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanApproved(svc, makePlanApprovedInput());
    await new Promise((r) => setImmediate(r));
    expect(emits[0].timestamp).toMatch(TIMESTAMP_RE);
  });

  it('does not emit when service is disabled', async () => {
    const disabledConfig = KindlingConfigSchema.parse({ enabled: false });
    const { store, emits } = makeSpyStore();
    const svc = new KindlingService(store, disabledConfig);
    emitPlanApproved(svc, makePlanApprovedInput());
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
    expect(() => emitPlanApproved(svc, makePlanApprovedInput())).not.toThrow();
    await new Promise((r) => setImmediate(r));
  });
});

// =============================================================================
// emitPlanRejected
// =============================================================================

describe('emitPlanRejected', () => {
  it('returns the plan_id', () => {
    const { store } = makeSpyStore();
    const svc = makeService(store);
    const returned = emitPlanRejected(svc, makePlanRejectedInput({ plan_id: 'plan-reject-001' }));
    expect(returned).toBe('plan-reject-001');
  });

  it('emits an observation with kind plan_rejected', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanRejected(svc, makePlanRejectedInput());
    await new Promise((r) => setImmediate(r));
    expect(emits).toHaveLength(1);
    expect(emits[0].kind).toBe('plan_rejected');
  });

  it('emits the plan_id', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanRejected(svc, makePlanRejectedInput({ plan_id: 'plan-reject-42' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_rejected' }>;
    expect(obs.plan_id).toBe('plan-reject-42');
  });

  it('emits the plan_version', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanRejected(svc, makePlanRejectedInput({ plan_version: '1.5' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_rejected' }>;
    expect(obs.plan_version).toBe('1.5');
  });

  it('emits the rejected_by', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanRejected(svc, makePlanRejectedInput({ rejected_by: 'bob' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_rejected' }>;
    expect(obs.rejected_by).toBe('bob');
  });

  it('emits rejection_reason when provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanRejected(svc, makePlanRejectedInput({ rejection_reason: 'Scope too broad' }));
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_rejected' }>;
    expect(obs.rejection_reason).toBe('Scope too broad');
  });

  it('omits rejection_reason when not provided', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanRejected(svc, makePlanRejectedInput());
    await new Promise((r) => setImmediate(r));
    const obs = emits[0] as Extract<(typeof emits)[0], { kind: 'plan_rejected' }>;
    expect(obs.rejection_reason).toBeUndefined();
  });

  it('emits a Z-suffixed ISO8601 timestamp', async () => {
    const { store, emits } = makeSpyStore();
    const svc = makeService(store);
    emitPlanRejected(svc, makePlanRejectedInput());
    await new Promise((r) => setImmediate(r));
    expect(emits[0].timestamp).toMatch(TIMESTAMP_RE);
  });

  it('does not emit when service is disabled', async () => {
    const disabledConfig = KindlingConfigSchema.parse({ enabled: false });
    const { store, emits } = makeSpyStore();
    const svc = new KindlingService(store, disabledConfig);
    emitPlanRejected(svc, makePlanRejectedInput());
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
    expect(() => emitPlanRejected(svc, makePlanRejectedInput())).not.toThrow();
    await new Promise((r) => setImmediate(r));
  });
});
