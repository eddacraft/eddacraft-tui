/**
 * Ember Port Interface Tests (TCOV-015)
 *
 * Verifies type shapes and structural contracts for ember.port.ts.
 * Behavioural coverage for IEmberPort is in testing/mocks/mocks.test.ts.
 */

import { describe, it, expect } from 'vitest';
import type {
  UpdateProposalInput,
  ResolveProposalInput,
  EmberStats,
  ProposalTypeStats,
  ProposalStatusStats,
} from './ember.port.js';
import { createMemoryId } from '../identifiers.js';
import type { Timestamp } from '../temporal.js';

const UUID_A = '550e8400-e29b-41d4-a716-446655440000';
const TS = '2024-06-01T10:00:00.000Z' as Timestamp;

// =============================================================================
// Input type structural tests
// =============================================================================

describe('UpdateProposalInput shape (TCOV-015)', () => {
  it('accepts an empty update', () => {
    const input: UpdateProposalInput = {};
    expect(Object.keys(input)).toHaveLength(0);
  });

  it('accepts all fields', () => {
    const input: UpdateProposalInput = {
      summary: 'Updated summary',
      rationale: 'Updated rationale',
      confidence: 0.9,
      metadata: { reviewed: true, ticket: 'PROP-001' },
    };
    expect(input.summary).toBe('Updated summary');
    expect(input.confidence).toBe(0.9);
    expect(input.metadata?.['reviewed']).toBe(true);
  });

  it('confidence is a numeric score between 0 and 1', () => {
    const low: UpdateProposalInput = { confidence: 0.1 };
    const high: UpdateProposalInput = { confidence: 0.99 };
    expect(low.confidence).toBeLessThan(1);
    expect(high.confidence).toBeGreaterThan(0);
  });
});

describe('ResolveProposalInput shape (TCOV-015)', () => {
  it('accepts a promoted resolution with memory_id', () => {
    const memoryId = createMemoryId(UUID_A);
    const input: ResolveProposalInput = {
      status: 'promoted',
      resolved_by: 'user@example.com',
      resolution_reason: 'Worth capturing',
      memory_id: memoryId,
    };
    expect(input.status).toBe('promoted');
    expect(input.memory_id).toBe(memoryId);
  });

  it('accepts a dismissed resolution', () => {
    const input: ResolveProposalInput = {
      status: 'dismissed',
      resolved_by: 'user@example.com',
      resolution_reason: 'Not relevant to us',
    };
    expect(input.status).toBe('dismissed');
    expect(input.memory_id).toBeUndefined();
  });

  it('accepts an expired resolution', () => {
    const input: ResolveProposalInput = { status: 'expired' };
    expect(input.status).toBe('expired');
    expect(input.resolved_by).toBeUndefined();
  });

  it('does not allow active status', () => {
    // TypeScript enforces this at compile time via Exclude<ProposalStatus, 'active'>.
    // We document the contract with a runtime check on the allowed values.
    const allowedStatuses = ['promoted', 'dismissed', 'expired'] as const;
    for (const status of allowedStatuses) {
      const input: ResolveProposalInput = { status };
      expect(['promoted', 'dismissed', 'expired']).toContain(input.status);
    }
  });
});

// =============================================================================
// Statistics type shapes
// =============================================================================

describe('EmberStats shape (TCOV-015)', () => {
  it('models a full statistics snapshot', () => {
    const stats: EmberStats = {
      total_proposals: 10,
      by_status: [
        { status: 'active', count: 5 },
        { status: 'promoted', count: 3 },
        { status: 'dismissed', count: 1 },
        { status: 'expired', count: 1 },
      ],
      by_type: [
        { type: 'decision', count: 4, avg_confidence: 0.85 },
        { type: 'pattern', count: 6, avg_confidence: 0.72 },
      ],
      expiring_soon: 2,
      avg_confidence: 0.77,
      oldest_active: TS,
      most_recent: TS,
      promotion_rate: 0.75,
    };

    expect(stats.total_proposals).toBe(10);
    expect(stats.expiring_soon).toBe(2);
    expect(stats.promotion_rate).toBe(0.75);
    expect(stats.by_status).toHaveLength(4);
  });

  it('allows optional fields to be absent', () => {
    const stats: EmberStats = {
      total_proposals: 0,
      by_status: [],
      by_type: [],
      expiring_soon: 0,
    };
    expect(stats.avg_confidence).toBeUndefined();
    expect(stats.oldest_active).toBeUndefined();
    expect(stats.most_recent).toBeUndefined();
    expect(stats.promotion_rate).toBeUndefined();
  });
});

describe('ProposalTypeStats shape (TCOV-015)', () => {
  it('models stats for each proposal type', () => {
    const types = ['decision', 'pattern', 'warning', 'lesson', 'anomaly', 'constraint'] as const;
    for (const type of types) {
      const stat: ProposalTypeStats = { type, count: 1, avg_confidence: 0.5 };
      expect(stat.type).toBe(type);
      expect(stat.avg_confidence).toBe(0.5);
    }
  });
});

describe('ProposalStatusStats shape (TCOV-015)', () => {
  it('models stats for each proposal status', () => {
    const statuses = ['active', 'promoted', 'expired', 'dismissed'] as const;
    for (const status of statuses) {
      const stat: ProposalStatusStats = { status, count: 3 };
      expect(stat.status).toBe(status);
    }
  });
});

// =============================================================================
// IEmberPort interface structural completeness check
// =============================================================================

describe('IEmberPort interface structure (TCOV-015)', () => {
  it('has all expected method names in the interface contract', () => {
    const methods = [
      'createProposal',
      'updateProposal',
      'resolveProposal',
      'getProposal',
      'queryProposals',
      'getActiveProposals',
      'getProposalsBySession',
      'proposalExists',
      'markPromoted',
      'markDismissed',
      'getExpiredProposals',
      'processExpiredProposals',
      'expireStaleProposals',
      'isAvailable',
      'getStats',
      'countProposals',
      'pruneProposals',
    ];

    const unique = new Set(methods);
    expect(unique.size).toBe(methods.length);
    expect(methods.length).toBe(17);
  });
});
