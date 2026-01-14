/**
 * Ember Port Mock (STACK-010)
 *
 * Mock implementation of IEmberPort for testing.
 * Uses in-memory storage and vitest mock functions.
 *
 * @module @anvil/edda-stack/testing/mocks/ember
 */

import { vi, type Mock } from 'vitest';
import { v4 as uuidv4 } from 'uuid';
import type {
  IEmberPort,
  UpdateProposalInput,
  ResolveProposalInput,
  EmberStats,
  ProposalTypeStats,
  ProposalStatusStats,
} from '../../contracts/ports/ember.port.js';
import type {
  CandidateProposal,
  CreateProposalInput,
  ProposalQuery,
  ProposalQueryResult,
  ProposalStatus,
  ProposalType,
} from '../../contracts/ember-proposal.js';
import type { ProposalId, MemoryId, SessionId, Timestamp } from '../../contracts/index.js';
import { now, calculateExpiry } from '../../contracts/temporal.js';
import { createProposalId, createSessionId } from '../../contracts/identifiers.js';

// =============================================================================
// Mock Options
// =============================================================================

/**
 * Options for creating a mock Ember port
 */
export interface MockEmberPortOptions {
  /** Initial proposals to populate the store */
  initialProposals?: CandidateProposal[];

  /** Default TTL in days for new proposals */
  defaultTtlDays?: number;

  /** Whether to auto-generate IDs (default: true) */
  autoGenerateIds?: boolean;
}

// =============================================================================
// Mock Implementation
// =============================================================================

/**
 * In-memory implementation of IEmberPort for testing
 */
export interface MockEmberPort extends IEmberPort {
  /** Access to the underlying proposal store */
  _store: Map<ProposalId, CandidateProposal>;

  /** Reset the mock to initial state */
  _reset: () => void;

  /** Get all proposals (for assertions) */
  _getAll: () => CandidateProposal[];

  /** Mock function references for verification */
  _mocks: {
    createProposal: Mock;
    updateProposal: Mock;
    resolveProposal: Mock;
    getProposal: Mock;
    queryProposals: Mock;
    getActiveProposals: Mock;
    getProposalsBySession: Mock;
    proposalExists: Mock;
    markPromoted: Mock;
    markDismissed: Mock;
    getExpiredProposals: Mock;
    processExpiredProposals: Mock;
    expireStaleProposals: Mock;
    isAvailable: Mock;
    getStats: Mock;
    countProposals: Mock;
    pruneProposals: Mock;
  };
}

/**
 * Create a mock Ember port for testing
 */
export function createMockEmberPort(options: MockEmberPortOptions = {}): MockEmberPort {
  const { initialProposals = [], defaultTtlDays = 30, autoGenerateIds = true } = options;

  // In-memory store
  const store = new Map<ProposalId, CandidateProposal>();

  // Populate initial proposals
  for (const proposal of initialProposals) {
    store.set(proposal.id, proposal);
  }

  // Create proposal implementation
  const createProposalImpl = async (input: CreateProposalInput): Promise<CandidateProposal> => {
    const id = autoGenerateIds ? createProposalId(uuidv4()) : ('' as ProposalId);
    const createdAt = now();
    const ttlDays = input.ttl_days ?? defaultTtlDays;

    const proposal: CandidateProposal = {
      id,
      type: input.type,
      status: 'active',
      summary: input.summary,
      rationale: input.rationale,
      confidence: input.confidence,
      signals: input.signals ?? [],
      provenance: input.provenance,
      created_at: createdAt,
      expires_at: calculateExpiry(createdAt, ttlDays),
      ttl_days: ttlDays,
      metadata: input.metadata,
    };

    store.set(id, proposal);
    return proposal;
  };

  // Update proposal implementation
  const updateProposalImpl = async (
    id: ProposalId,
    input: UpdateProposalInput
  ): Promise<CandidateProposal | null> => {
    const proposal = store.get(id);
    if (!proposal) return null;

    const updated: CandidateProposal = {
      ...proposal,
      ...(input.summary !== undefined && { summary: input.summary }),
      ...(input.rationale !== undefined && { rationale: input.rationale }),
      ...(input.confidence !== undefined && { confidence: input.confidence }),
      ...(input.metadata !== undefined && { metadata: input.metadata }),
      updated_at: now(),
    };

    store.set(id, updated);
    return updated;
  };

  // Resolve proposal implementation
  const resolveProposalImpl = async (
    id: ProposalId,
    input: ResolveProposalInput
  ): Promise<CandidateProposal | null> => {
    const proposal = store.get(id);
    if (!proposal) return null;

    const resolved: CandidateProposal = {
      ...proposal,
      status: input.status,
      resolution: {
        resolved_at: now(),
        resolved_by: input.resolved_by,
        resolution_reason: input.resolution_reason,
        memory_id: input.memory_id,
      },
      updated_at: now(),
    };

    store.set(id, resolved);
    return resolved;
  };

  // Get proposal implementation
  const getProposalImpl = async (id: ProposalId): Promise<CandidateProposal | null> => {
    return store.get(id) ?? null;
  };

  // Query proposals implementation
  const queryProposalsImpl = async (query: ProposalQuery): Promise<ProposalQueryResult> => {
    let proposals = Array.from(store.values());

    // Apply filters
    if (query.types && query.types.length > 0) {
      proposals = proposals.filter((p) => query.types!.includes(p.type));
    }

    if (query.statuses && query.statuses.length > 0) {
      proposals = proposals.filter((p) => query.statuses!.includes(p.status));
    }

    if (query.min_confidence !== undefined) {
      proposals = proposals.filter((p) => p.confidence >= query.min_confidence!);
    }

    if (query.created_after) {
      const after = new Date(query.created_after).getTime();
      proposals = proposals.filter((p) => new Date(p.created_at).getTime() > after);
    }

    if (query.created_before) {
      const before = new Date(query.created_before).getTime();
      proposals = proposals.filter((p) => new Date(p.created_at).getTime() < before);
    }

    if (!query.include_expired) {
      proposals = proposals.filter((p) => p.status !== 'expired');
    }

    if (query.session_id) {
      proposals = proposals.filter((p) => p.provenance.session_ids.includes(query.session_id!));
    }

    // Sort
    const sortBy = query.sort_by ?? 'created_at';
    const sortOrder = query.sort_order ?? 'desc';

    proposals.sort((a, b) => {
      let cmp = 0;
      if (sortBy === 'created_at') {
        cmp = new Date(a.created_at).getTime() - new Date(b.created_at).getTime();
      } else if (sortBy === 'confidence') {
        cmp = a.confidence - b.confidence;
      } else if (sortBy === 'expires_at') {
        cmp = new Date(a.expires_at).getTime() - new Date(b.expires_at).getTime();
      }
      return sortOrder === 'desc' ? -cmp : cmp;
    });

    const total = proposals.length;
    const offset = query.offset ?? 0;
    const limit = query.limit ?? 100;

    proposals = proposals.slice(offset, offset + limit);

    return {
      proposals,
      total,
      limit,
      offset,
      has_more: offset + proposals.length < total,
    };
  };

  // Get active proposals implementation
  const getActiveProposalsImpl = async (): Promise<CandidateProposal[]> => {
    const currentTime = Date.now();
    return Array.from(store.values()).filter(
      (p) => p.status === 'active' && new Date(p.expires_at).getTime() > currentTime
    );
  };

  // Get proposals by session implementation
  const getProposalsBySessionImpl = async (sessionId: SessionId): Promise<CandidateProposal[]> => {
    return Array.from(store.values()).filter((p) => p.provenance.session_ids.includes(sessionId));
  };

  // Proposal exists implementation
  const proposalExistsImpl = async (id: ProposalId): Promise<boolean> => {
    return store.has(id);
  };

  // Get expired proposals implementation
  const getExpiredProposalsImpl = async (): Promise<CandidateProposal[]> => {
    const currentTime = Date.now();
    return Array.from(store.values()).filter(
      (p) => p.status === 'active' && new Date(p.expires_at).getTime() <= currentTime
    );
  };

  // Process expired proposals implementation
  const processExpiredProposalsImpl = async (): Promise<number> => {
    const currentTime = Date.now();
    let count = 0;

    for (const [id, proposal] of store.entries()) {
      if (proposal.status === 'active' && new Date(proposal.expires_at).getTime() <= currentTime) {
        store.set(id, {
          ...proposal,
          status: 'expired',
          resolution: {
            resolved_at: now(),
            resolution_reason: 'TTL expired',
          },
          updated_at: now(),
        });
        count++;
      }
    }

    return count;
  };

  // Count proposals implementation
  const countProposalsImpl = async (status?: ProposalStatus): Promise<number> => {
    if (status) {
      return Array.from(store.values()).filter((p) => p.status === status).length;
    }
    return store.size;
  };

  // Prune proposals implementation
  const pruneProposalsImpl = async (olderThan: Timestamp): Promise<number> => {
    const threshold = new Date(olderThan).getTime();
    let count = 0;

    for (const [id, proposal] of store.entries()) {
      if (
        proposal.status !== 'active' &&
        proposal.resolution?.resolved_at &&
        new Date(proposal.resolution.resolved_at).getTime() < threshold
      ) {
        store.delete(id);
        count++;
      }
    }

    return count;
  };

  // Mark promoted implementation (STACK-007) - reserved for future use
  const _markPromotedImpl = async (
    id: ProposalId,
    memoryId: MemoryId,
    resolvedBy: string
  ): Promise<void> => {
    const proposal = store.get(id);
    if (!proposal) return;

    store.set(id, {
      ...proposal,
      status: 'promoted',
      resolution: {
        resolved_at: now(),
        resolved_by: resolvedBy,
        resolution_reason: 'Promoted to Edda memory',
        memory_id: memoryId,
      },
      updated_at: now(),
    });
  };

  // Mark dismissed implementation (STACK-007) - reserved for future use
  const _markDismissedImpl = async (
    id: ProposalId,
    reason: string,
    resolvedBy: string
  ): Promise<void> => {
    const proposal = store.get(id);
    if (!proposal) return;

    store.set(id, {
      ...proposal,
      status: 'dismissed',
      resolution: {
        resolved_at: now(),
        resolved_by: resolvedBy,
        resolution_reason: reason,
      },
      updated_at: now(),
    });
  };

  // Expire stale proposals implementation (STACK-007) - reserved for future use
  const _expireStaleProposalsImpl = async (): Promise<number> => {
    return processExpiredProposalsImpl();
  };

  // Is available implementation (STACK-007) - reserved for future use
  const _isAvailableImpl = async (): Promise<boolean> => {
    return true;
  };

  // Get stats implementation (STACK-007) - reserved for future use
  const _getStatsImpl = async (): Promise<EmberStats> => {
    const proposals = Array.from(store.values());
    const currentTime = Date.now();

    // Count by status
    const byStatus: ProposalStatusStats[] = [
      { status: 'active', count: proposals.filter((p) => p.status === 'active').length },
      { status: 'promoted', count: proposals.filter((p) => p.status === 'promoted').length },
      { status: 'expired', count: proposals.filter((p) => p.status === 'expired').length },
      { status: 'dismissed', count: proposals.filter((p) => p.status === 'dismissed').length },
    ];

    // Count by type
    const types: ProposalType[] = [
      'decision',
      'pattern',
      'warning',
      'lesson',
      'anomaly',
      'constraint',
    ];
    const byType: ProposalTypeStats[] = types.map((type) => {
      const typeProposals = proposals.filter((p) => p.type === type);
      const avgConf =
        typeProposals.length > 0
          ? typeProposals.reduce((sum, p) => sum + p.confidence, 0) / typeProposals.length
          : 0;
      return { type, count: typeProposals.length, avg_confidence: avgConf };
    });

    // Active proposals
    const activeProposals = proposals.filter((p) => p.status === 'active');

    // Expiring soon (within 24 hours)
    const expiringSoon = activeProposals.filter(
      (p) => new Date(p.expires_at).getTime() <= currentTime + 86400000
    ).length;

    // Average confidence of active
    const avgConfidence =
      activeProposals.length > 0
        ? activeProposals.reduce((sum, p) => sum + p.confidence, 0) / activeProposals.length
        : undefined;

    // Oldest/most recent
    const timestamps = proposals.map((p) => new Date(p.created_at).getTime());
    const oldestActive =
      activeProposals.length > 0
        ? Math.min(...activeProposals.map((p) => new Date(p.created_at).getTime()))
        : undefined;

    // Promotion rate
    const resolved = proposals.filter((p) => p.status !== 'active');
    const promoted = proposals.filter((p) => p.status === 'promoted');
    const promotionRate = resolved.length > 0 ? promoted.length / resolved.length : undefined;

    return {
      total_proposals: proposals.length,
      by_status: byStatus,
      by_type: byType,
      expiring_soon: expiringSoon,
      avg_confidence: avgConfidence,
      oldest_active: oldestActive ? (new Date(oldestActive).toISOString() as Timestamp) : undefined,
      most_recent:
        timestamps.length > 0
          ? (new Date(Math.max(...timestamps)).toISOString() as Timestamp)
          : undefined,
      promotion_rate: promotionRate,
    };
  };

  // Create mock functions
  const mocks = {
    createProposal: vi.fn(createProposalImpl),
    updateProposal: vi.fn(updateProposalImpl),
    resolveProposal: vi.fn(resolveProposalImpl),
    getProposal: vi.fn(getProposalImpl),
    queryProposals: vi.fn(queryProposalsImpl),
    getActiveProposals: vi.fn(getActiveProposalsImpl),
    getProposalsBySession: vi.fn(getProposalsBySessionImpl),
    proposalExists: vi.fn(proposalExistsImpl),
    getExpiredProposals: vi.fn(getExpiredProposalsImpl),
    processExpiredProposals: vi.fn(processExpiredProposalsImpl),
    countProposals: vi.fn(countProposalsImpl),
    pruneProposals: vi.fn(pruneProposalsImpl),
  };

  return {
    // IEmberPort implementation
    createProposal: mocks.createProposal,
    updateProposal: mocks.updateProposal,
    resolveProposal: mocks.resolveProposal,
    getProposal: mocks.getProposal,
    queryProposals: mocks.queryProposals,
    getActiveProposals: mocks.getActiveProposals,
    getProposalsBySession: mocks.getProposalsBySession,
    proposalExists: mocks.proposalExists,
    getExpiredProposals: mocks.getExpiredProposals,
    processExpiredProposals: mocks.processExpiredProposals,
    countProposals: mocks.countProposals,
    pruneProposals: mocks.pruneProposals,

    // Test utilities
    _store: store,
    _reset: () => {
      store.clear();
      for (const proposal of initialProposals) {
        store.set(proposal.id, proposal);
      }
      Object.values(mocks).forEach((mock) => mock.mockClear());
    },
    _getAll: () => Array.from(store.values()),
    _mocks: mocks,
  };
}

// =============================================================================
// Pre-built Scenarios
// =============================================================================

/**
 * Create a mock Ember port with sample proposals
 */
export function mockEmberWithProposals(): MockEmberPort {
  const sessionId = createSessionId(uuidv4());
  const baseTimestamp = new Date('2024-01-15T10:00:00.000Z');

  const proposals: CandidateProposal[] = [
    {
      id: createProposalId(uuidv4()),
      type: 'pattern',
      status: 'active',
      summary: 'Repeated use of factory pattern for component creation',
      rationale: 'This pattern has appeared in 5 different files over the past week',
      confidence: 0.75,
      signals: [
        { rule: 'repetition', contribution: 0.6, weight: 1.5 },
        { rule: 'consistency', contribution: 0.8, weight: 1.0 },
      ],
      provenance: {
        observation_ids: [uuidv4()],
        session_ids: [sessionId],
        earliest_observation: baseTimestamp.toISOString() as Timestamp,
        latest_observation: new Date(baseTimestamp.getTime() + 86400000).toISOString() as Timestamp,
      },
      created_at: baseTimestamp.toISOString() as Timestamp,
      expires_at: calculateExpiry(baseTimestamp.toISOString() as Timestamp, 30),
      ttl_days: 30,
    },
    {
      id: createProposalId(uuidv4()),
      type: 'decision',
      status: 'active',
      summary: 'Team decided to use TypeScript strict mode',
      rationale: 'Consistent configuration change across all projects',
      confidence: 0.85,
      signals: [{ rule: 'explicit_decision', contribution: 0.85, weight: 2.0 }],
      provenance: {
        observation_ids: [uuidv4()],
        session_ids: [sessionId],
        earliest_observation: baseTimestamp.toISOString() as Timestamp,
        latest_observation: baseTimestamp.toISOString() as Timestamp,
      },
      created_at: new Date(baseTimestamp.getTime() + 3600000).toISOString() as Timestamp,
      expires_at: calculateExpiry(
        new Date(baseTimestamp.getTime() + 3600000).toISOString() as Timestamp,
        30
      ),
      ttl_days: 30,
    },
    {
      id: createProposalId(uuidv4()),
      type: 'warning',
      status: 'active',
      summary: 'Test coverage has been declining',
      rationale: 'Coverage dropped from 85% to 72% over the past month',
      confidence: 0.65,
      signals: [
        { rule: 'trend_detection', contribution: 0.65, weight: 1.2 },
        { rule: 'threshold_breach', contribution: 0.5, weight: 1.0 },
      ],
      provenance: {
        observation_ids: [uuidv4(), uuidv4()],
        session_ids: [sessionId],
        earliest_observation: new Date(
          baseTimestamp.getTime() - 2592000000
        ).toISOString() as Timestamp, // 30 days ago
        latest_observation: baseTimestamp.toISOString() as Timestamp,
      },
      created_at: new Date(baseTimestamp.getTime() + 7200000).toISOString() as Timestamp,
      expires_at: calculateExpiry(
        new Date(baseTimestamp.getTime() + 7200000).toISOString() as Timestamp,
        14
      ),
      ttl_days: 14,
    },
  ];

  return createMockEmberPort({
    initialProposals: proposals,
    defaultTtlDays: 30,
  });
}

/**
 * Create an empty mock Ember port
 */
export function mockEmberEmpty(): MockEmberPort {
  return createMockEmberPort();
}

/**
 * Create a mock Ember port with proposals in various statuses
 */
export function mockEmberWithMixedStatuses(): MockEmberPort {
  const sessionId = createSessionId(uuidv4());
  const baseTimestamp = new Date('2024-01-15T10:00:00.000Z');

  const proposals: CandidateProposal[] = [
    // Active proposal
    {
      id: createProposalId(uuidv4()),
      type: 'pattern',
      status: 'active',
      summary: 'Active pattern proposal',
      rationale: 'Currently being evaluated',
      confidence: 0.7,
      signals: [],
      provenance: {
        observation_ids: [uuidv4()],
        session_ids: [sessionId],
        earliest_observation: baseTimestamp.toISOString() as Timestamp,
        latest_observation: baseTimestamp.toISOString() as Timestamp,
      },
      created_at: baseTimestamp.toISOString() as Timestamp,
      expires_at: calculateExpiry(baseTimestamp.toISOString() as Timestamp, 30),
      ttl_days: 30,
    },
    // Promoted proposal
    {
      id: createProposalId(uuidv4()),
      type: 'decision',
      status: 'promoted',
      summary: 'Promoted decision proposal',
      rationale: 'Was promoted to Edda memory',
      confidence: 0.9,
      signals: [],
      provenance: {
        observation_ids: [uuidv4()],
        session_ids: [sessionId],
        earliest_observation: baseTimestamp.toISOString() as Timestamp,
        latest_observation: baseTimestamp.toISOString() as Timestamp,
      },
      created_at: new Date(baseTimestamp.getTime() - 86400000).toISOString() as Timestamp,
      expires_at: calculateExpiry(
        new Date(baseTimestamp.getTime() - 86400000).toISOString() as Timestamp,
        30
      ),
      ttl_days: 30,
      resolution: {
        resolved_at: baseTimestamp.toISOString() as Timestamp,
        resolved_by: 'user@example.com',
        resolution_reason: 'Valuable decision to remember',
        memory_id: uuidv4(),
      },
    },
    // Expired proposal
    {
      id: createProposalId(uuidv4()),
      type: 'anomaly',
      status: 'expired',
      summary: 'Expired anomaly proposal',
      rationale: 'TTL expired without action',
      confidence: 0.5,
      signals: [],
      provenance: {
        observation_ids: [uuidv4()],
        session_ids: [sessionId],
        earliest_observation: new Date(
          baseTimestamp.getTime() - 2592000000
        ).toISOString() as Timestamp,
        latest_observation: new Date(
          baseTimestamp.getTime() - 2592000000
        ).toISOString() as Timestamp,
      },
      created_at: new Date(baseTimestamp.getTime() - 2592000000).toISOString() as Timestamp,
      expires_at: new Date(baseTimestamp.getTime() - 86400000).toISOString() as Timestamp,
      ttl_days: 30,
      resolution: {
        resolved_at: new Date(baseTimestamp.getTime() - 86400000).toISOString() as Timestamp,
        resolution_reason: 'TTL expired',
      },
    },
    // Dismissed proposal
    {
      id: createProposalId(uuidv4()),
      type: 'lesson',
      status: 'dismissed',
      summary: 'Dismissed lesson proposal',
      rationale: 'Not relevant to our project',
      confidence: 0.4,
      signals: [],
      provenance: {
        observation_ids: [uuidv4()],
        session_ids: [sessionId],
        earliest_observation: baseTimestamp.toISOString() as Timestamp,
        latest_observation: baseTimestamp.toISOString() as Timestamp,
      },
      created_at: new Date(baseTimestamp.getTime() - 172800000).toISOString() as Timestamp,
      expires_at: calculateExpiry(
        new Date(baseTimestamp.getTime() - 172800000).toISOString() as Timestamp,
        30
      ),
      ttl_days: 30,
      resolution: {
        resolved_at: new Date(baseTimestamp.getTime() - 86400000).toISOString() as Timestamp,
        resolved_by: 'user@example.com',
        resolution_reason: 'Not applicable to our context',
      },
    },
  ];

  return createMockEmberPort({
    initialProposals: proposals,
  });
}
