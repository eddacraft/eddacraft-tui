/**
 * KindlingQueryService tests (TCOV-018)
 *
 * Covers: querySession, queryPlan, queryGate, queryAction, applyLimits
 * defence layer, defaults, and option forwarding.
 */

import { describe, it, expect, vi } from 'vitest';
import { KindlingQueryService } from './query-service.js';
import type { KindlingService } from './kindling-service.js';
import type { QueryResponse, QueryRequest } from './query-contract.js';
import { KindlingConfigSchema } from './config.js';

// =============================================================================
// Test helpers
// =============================================================================

const SESSION_UUID = 'aaaabbbb-cccc-4ddd-aeee-ffffffffffff';
const META_UUID = '550e8400-e29b-41d4-a716-446655440000';
const VALID_TIMESTAMP = '2026-02-15T10:00:00.000Z';

const enabledConfig = KindlingConfigSchema.parse({
  enabled: true,
  query_limits: { max_results: 100, max_payload_bytes: 1024 * 1024 },
});

/** Build an empty QueryResponse */
function emptyResponse(
  overrides: Partial<{
    resultCount: number;
    truncated: boolean;
    truncationReason: 'max_results' | 'max_payload_bytes' | 'none';
  }> = {}
): QueryResponse {
  return {
    metadata: {
      query_id: META_UUID,
      executed_at: VALID_TIMESTAMP,
      contract_version: '1.0.0',
      result_count: overrides.resultCount ?? 0,
      truncated: overrides.truncated ?? false,
      truncation_reason: overrides.truncationReason ?? 'none',
    },
    observations: [],
  };
}

/** Build a mock KindlingService recording queries */
function makeServiceMock(response: QueryResponse = emptyResponse()): {
  service: KindlingService;
  capturedRequests: QueryRequest[];
} {
  const capturedRequests: QueryRequest[] = [];
  const service = {
    configuration: enabledConfig,
    query: vi.fn(async (req: QueryRequest): Promise<QueryResponse> => {
      capturedRequests.push(req);
      return response;
    }),
  } as unknown as KindlingService;
  return { service, capturedRequests };
}

// =============================================================================
// querySession
// =============================================================================

describe('KindlingQueryService.querySession', () => {
  it('issues a session-scope query with the provided sessionId', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.querySession(SESSION_UUID);
    expect(capturedRequests).toHaveLength(1);
    const req = capturedRequests[0];
    expect(req.scope).toBe('session');
    if (req.scope === 'session') {
      expect(req.session_id).toBe(SESSION_UUID);
    }
  });

  it('defaults shape to "timeline"', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.querySession(SESSION_UUID);
    expect(capturedRequests[0].shape).toBe('timeline');
  });

  it('defaults format to "json"', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.querySession(SESSION_UUID);
    expect(capturedRequests[0].format).toBe('json');
  });

  it('uses config max_results by default', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.querySession(SESSION_UUID);
    expect(capturedRequests[0].max_results).toBe(100);
  });

  it('overrides max_results when provided in options', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.querySession(SESSION_UUID, { max_results: 5 });
    expect(capturedRequests[0].max_results).toBe(5);
  });

  it('forwards time_after and time_before', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.querySession(SESSION_UUID, {
      time_after: '2026-01-01T00:00:00.000Z',
      time_before: '2026-12-31T00:00:00.000Z',
    });
    expect(capturedRequests[0].time_after).toBe('2026-01-01T00:00:00.000Z');
    expect(capturedRequests[0].time_before).toBe('2026-12-31T00:00:00.000Z');
  });

  it('forwards include_phases when provided', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.querySession(SESSION_UUID, { include_phases: ['gate', 'action'] });
    const req = capturedRequests[0];
    if (req.scope === 'session') {
      expect(req.include_phases).toEqual(['gate', 'action']);
    }
  });

  it('returns the query response', async () => {
    const { service } = makeServiceMock(emptyResponse());
    const qs = new KindlingQueryService(service);
    const resp = await qs.querySession(SESSION_UUID);
    expect(resp.observations).toEqual([]);
    expect(resp.metadata.contract_version).toBe('1.0.0');
  });

  it('accepts custom shape option', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.querySession(SESSION_UUID, { shape: 'list' });
    expect(capturedRequests[0].shape).toBe('list');
  });
});

// =============================================================================
// queryPlan
// =============================================================================

describe('KindlingQueryService.queryPlan', () => {
  it('issues a plan-scope query with the provided planId', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.queryPlan('plan-001');
    const req = capturedRequests[0];
    expect(req.scope).toBe('plan');
    if (req.scope === 'plan') {
      expect(req.plan_id).toBe('plan-001');
    }
  });

  it('defaults shape to "entity"', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.queryPlan('plan-001');
    expect(capturedRequests[0].shape).toBe('entity');
  });

  it('defaults include_executions to true', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.queryPlan('plan-001');
    const req = capturedRequests[0];
    if (req.scope === 'plan') {
      expect(req.include_executions).toBe(true);
    }
  });

  it('defaults include_versions to true', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.queryPlan('plan-001');
    const req = capturedRequests[0];
    if (req.scope === 'plan') {
      expect(req.include_versions).toBe(true);
    }
  });

  it('overrides include_executions when provided', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.queryPlan('plan-001', { include_executions: false });
    const req = capturedRequests[0];
    if (req.scope === 'plan') {
      expect(req.include_executions).toBe(false);
    }
  });

  it('overrides include_versions when provided', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.queryPlan('plan-001', { include_versions: false });
    const req = capturedRequests[0];
    if (req.scope === 'plan') {
      expect(req.include_versions).toBe(false);
    }
  });

  it('uses config max_payload_bytes by default', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.queryPlan('plan-001');
    expect(capturedRequests[0].max_payload_bytes).toBe(1024 * 1024);
  });
});

// =============================================================================
// queryGate
// =============================================================================

describe('KindlingQueryService.queryGate', () => {
  it('issues a gate-scope query with the provided gateEvalId', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.queryGate('ge-001');
    const req = capturedRequests[0];
    expect(req.scope).toBe('gate');
    if (req.scope === 'gate') {
      expect(req.gate_eval_id).toBe('ge-001');
    }
  });

  it('defaults shape to "entity"', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.queryGate('ge-001');
    expect(capturedRequests[0].shape).toBe('entity');
  });

  it('defaults format to "json"', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.queryGate('ge-001');
    expect(capturedRequests[0].format).toBe('json');
  });

  it('uses config max_results by default', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.queryGate('ge-001');
    expect(capturedRequests[0].max_results).toBe(100);
  });

  it('returns the response', async () => {
    const { service } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    const resp = await qs.queryGate('ge-001');
    expect(resp).toBeDefined();
  });
});

// =============================================================================
// queryAction
// =============================================================================

describe('KindlingQueryService.queryAction', () => {
  it('issues an action-scope query with the provided actionId', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.queryAction('act-001');
    const req = capturedRequests[0];
    expect(req.scope).toBe('action');
    if (req.scope === 'action') {
      expect(req.action_id).toBe('act-001');
    }
  });

  it('defaults shape to "entity"', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.queryAction('act-001');
    expect(capturedRequests[0].shape).toBe('entity');
  });

  it('defaults include_approval_chain to true', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.queryAction('act-001');
    const req = capturedRequests[0];
    if (req.scope === 'action') {
      expect(req.include_approval_chain).toBe(true);
    }
  });

  it('overrides include_approval_chain when provided', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.queryAction('act-001', { include_approval_chain: false });
    const req = capturedRequests[0];
    if (req.scope === 'action') {
      expect(req.include_approval_chain).toBe(false);
    }
  });

  it('forwards max_results from options', async () => {
    const { service, capturedRequests } = makeServiceMock();
    const qs = new KindlingQueryService(service);
    await qs.queryAction('act-001', { max_results: 7 });
    expect(capturedRequests[0].max_results).toBe(7);
  });
});

// =============================================================================
// applyLimits (defence-in-depth layer via enforceQueryLimits)
// =============================================================================

describe('KindlingQueryService — applyLimits defence layer', () => {
  it('passes through response unchanged when results are within limits', async () => {
    const { service } = makeServiceMock(emptyResponse());
    const qs = new KindlingQueryService(service);
    const resp = await qs.querySession(SESSION_UUID);
    expect(resp.observations).toHaveLength(0);
    expect(resp.metadata.truncated).toBe(false);
  });

  it('enforces max_results by truncating excess observations', async () => {
    // Build a response with more obs than the config limit
    const configWithLowLimit = KindlingConfigSchema.parse({
      enabled: true,
      query_limits: { max_results: 2, max_payload_bytes: 1024 * 1024 },
    });
    const service = {
      configuration: configWithLowLimit,
      query: vi.fn(
        async (): Promise<QueryResponse> => ({
          metadata: {
            query_id: META_UUID,
            executed_at: VALID_TIMESTAMP,
            contract_version: '1.0.0',
            result_count: 5,
            truncated: false,
            truncation_reason: 'none',
          },
          observations: Array.from({ length: 5 }, (_, i) => ({
            id: `obs-${i}`,
            kind: 'session_start' as const,
            timestamp: VALID_TIMESTAMP,
            session_id: SESSION_UUID,
            provenance: [],
            payload: {},
          })),
        })
      ),
    } as unknown as KindlingService;

    const qs = new KindlingQueryService(service);
    const resp = await qs.querySession(SESSION_UUID);
    expect(resp.observations).toHaveLength(2);
    expect(resp.metadata.truncated).toBe(true);
    expect(resp.metadata.truncation_reason).toBe('max_results');
  });

  it('enforces max_payload_bytes by truncating to fit payload limit', async () => {
    const largePayload = 'x'.repeat(600);
    const configTightBytes = KindlingConfigSchema.parse({
      enabled: true,
      query_limits: { max_results: 100, max_payload_bytes: 100 },
    });
    const service = {
      configuration: configTightBytes,
      query: vi.fn(
        async (): Promise<QueryResponse> => ({
          metadata: {
            query_id: META_UUID,
            executed_at: VALID_TIMESTAMP,
            contract_version: '1.0.0',
            result_count: 3,
            truncated: false,
            truncation_reason: 'none',
          },
          observations: Array.from({ length: 3 }, (_, i) => ({
            id: `obs-${i}`,
            kind: 'session_start' as const,
            timestamp: VALID_TIMESTAMP,
            session_id: SESSION_UUID,
            provenance: [],
            payload: { data: largePayload },
          })),
        })
      ),
    } as unknown as KindlingService;

    const qs = new KindlingQueryService(service);
    const resp = await qs.queryGate('ge-001');
    // All 3 obs together exceed 100 bytes; should be truncated
    expect(resp.observations.length).toBeLessThan(3);
    expect(resp.metadata.truncated).toBe(true);
  });
});
