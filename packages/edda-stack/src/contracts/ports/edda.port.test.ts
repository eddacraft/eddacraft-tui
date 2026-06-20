/**
 * Edda Port Interface Tests (TCOV-015)
 *
 * Verifies that the port types and input types exported from edda.port.ts are
 * structurally sound and that their type-level contracts are documented.
 * Runtime assertions focus on input-type shapes, guard functions, and the
 * exported index barrel.
 *
 * The behavioural coverage for IEddaPort is provided by the mock tests in
 * testing/mocks/mocks.test.ts which exercise a full concrete implementation.
 */

import { describe, it, expect } from 'vitest';
import type {
  IEddaPort,
  CreateMemoryInput,
  UpdateMemoryInput,
  RetireMemoryInput,
  ProvenanceResolutionResult,
  MemoryTypeStats,
  MemoryStatusStats,
  ConfidenceLevelStats,
  EddaStats,
} from './edda.port.js';
import { createMockEddaPort } from '../../testing/mocks/edda.mock.js';
import { createMemoryId, createObservationId, createSessionId } from '../identifiers.js';
import type { Timestamp } from '../temporal.js';

const UUID_A = '550e8400-e29b-41d4-a716-446655440000';
const UUID_B = '550e8400-e29b-41d4-a716-446655440001';
const TS = '2024-06-01T10:00:00.000Z' as Timestamp;

// =============================================================================
// Input type structural tests
// =============================================================================

describe('CreateMemoryInput shape (TCOV-015)', () => {
  it('accepts a minimal valid CreateMemoryInput', () => {
    const input: CreateMemoryInput = {
      type: 'decision',
      statement: 'Use TypeScript for all new code',
      context: { when: '2024-01-01', why: 'Type safety', conditions: [] },
      confidence: 'high',
      provenance: {
        kindling_sources: [
          {
            observation_id: createObservationId(UUID_A),
            session_id: createSessionId(UUID_B),
            kind: 'gate_evaluated',
            timestamp: TS,
          },
        ],
        source_sessions: [createSessionId(UUID_B)],
      },
      created_by: 'user@example.com',
      reason: 'Team decision',
    };

    expect(input.type).toBe('decision');
    expect(input.statement).toBe('Use TypeScript for all new code');
    expect(input.confidence).toBe('high');
    expect(input.created_by).toBe('user@example.com');
  });

  it('allows optional fields to be undefined', () => {
    const input: CreateMemoryInput = {
      type: 'pattern',
      statement: 'Factory pattern preferred',
      context: { when: 'always', why: 'Encapsulation', conditions: [] },
      confidence: 'medium',
      provenance: {
        kindling_sources: [
          {
            observation_id: createObservationId(UUID_A),
            session_id: createSessionId(UUID_B),
            kind: 'action_executed',
            timestamp: TS,
          },
        ],
        source_sessions: [createSessionId(UUID_B)],
      },
      created_by: 'dev@example.com',
      reason: 'Pattern documentation',
    };

    expect(input.confidence_rationale).toBeUndefined();
    expect(input.metadata).toBeUndefined();
  });

  it('supports metadata field as Record<string, unknown>', () => {
    const input: CreateMemoryInput = {
      type: 'warning',
      statement: 'Avoid global state',
      context: { when: 'always', why: 'Thread safety', conditions: [] },
      confidence: 'high',
      provenance: {
        kindling_sources: [
          {
            observation_id: createObservationId(UUID_A),
            session_id: createSessionId(UUID_B),
            kind: 'gate_evaluated',
            timestamp: TS,
          },
        ],
        source_sessions: [createSessionId(UUID_B)],
      },
      created_by: 'architect@example.com',
      reason: 'Architectural guideline',
      metadata: { severity: 'high', ticket: 'ARCH-001' },
    };
    expect(input.metadata?.['severity']).toBe('high');
  });

  it('supports all 6 memory types', () => {
    const types: CreateMemoryInput['type'][] = [
      'decision',
      'pattern',
      'constraint',
      'warning',
      'doctrine',
      'lesson',
    ];
    for (const type of types) {
      const input: CreateMemoryInput = {
        type,
        statement: `A ${type} memory`,
        context: { when: 'now', why: 'test', conditions: [] },
        confidence: 'medium',
        provenance: {
          kindling_sources: [
            {
              observation_id: createObservationId(UUID_A),
              session_id: createSessionId(UUID_B),
              kind: 'custom',
              timestamp: TS,
            },
          ],
          source_sessions: [createSessionId(UUID_B)],
        },
        created_by: 'u',
        reason: 'test',
      };
      expect(input.type).toBe(type);
    }
  });
});

describe('UpdateMemoryInput shape (TCOV-015)', () => {
  it('accepts an empty update (all fields optional)', () => {
    const input: UpdateMemoryInput = {};
    expect(Object.keys(input)).toHaveLength(0);
  });

  it('accepts all fields', () => {
    const input: UpdateMemoryInput = {
      statement: 'New statement',
      context: { why: 'Updated reason' },
      confidence: 'low',
      confidence_rationale: 'Newly discovered counterevidence',
      metadata: { reviewed: true },
    };
    expect(input.statement).toBe('New statement');
    expect(input.confidence).toBe('low');
    expect(input.context?.why).toBe('Updated reason');
  });
});

describe('RetireMemoryInput shape (TCOV-015)', () => {
  it('requires reason and retired_by', () => {
    const input: RetireMemoryInput = {
      reason: 'No longer applicable',
      retired_by: 'admin@example.com',
    };
    expect(input.reason).toBe('No longer applicable');
    expect(input.retired_by).toBe('admin@example.com');
  });

  it('supports optional superseded_by', () => {
    const supersedingId = createMemoryId(UUID_A);
    const input: RetireMemoryInput = {
      reason: 'Superseded',
      retired_by: 'admin@example.com',
      superseded_by: supersedingId,
    };
    expect(input.superseded_by).toBe(supersedingId);
  });
});

// =============================================================================
// ProvenanceResolutionResult shape tests
// =============================================================================

describe('ProvenanceResolutionResult shape (TCOV-015)', () => {
  it('models a complete resolution', () => {
    const result: ProvenanceResolutionResult = {
      complete: true,
      resolved_count: 3,
      total_count: 3,
      missing_links: [],
      resolved_data: {
        sessions: [UUID_A],
        observations: [UUID_B],
        proposal_id: UUID_A,
      },
      warnings: [],
    };
    expect(result.complete).toBe(true);
    expect(result.missing_links).toHaveLength(0);
    expect(result.resolved_data?.proposal_id).toBe(UUID_A);
  });

  it('models a partial resolution with missing links', () => {
    const result: ProvenanceResolutionResult = {
      complete: false,
      resolved_count: 1,
      total_count: 3,
      missing_links: ['obs-123', 'obs-456'],
      warnings: ['Some links could not be resolved'],
    };
    expect(result.complete).toBe(false);
    expect(result.missing_links).toHaveLength(2);
    expect(result.resolved_data).toBeUndefined();
  });
});

// =============================================================================
// Statistics type shapes
// =============================================================================

describe('Statistics types (TCOV-015)', () => {
  it('MemoryTypeStats has type and count', () => {
    const stat: MemoryTypeStats = { type: 'decision', count: 5 };
    expect(stat.type).toBe('decision');
    expect(stat.count).toBe(5);
  });

  it('MemoryStatusStats has status and count', () => {
    const stat: MemoryStatusStats = { status: 'retired', count: 2 };
    expect(stat.status).toBe('retired');
    expect(stat.count).toBe(2);
  });

  it('ConfidenceLevelStats has level and count', () => {
    const stat: ConfidenceLevelStats = { level: 'high', count: 10 };
    expect(stat.level).toBe('high');
    expect(stat.count).toBe(10);
  });

  it('EddaStats models a full statistics snapshot', () => {
    const stats: EddaStats = {
      total_memories: 15,
      by_status: [
        { status: 'active', count: 12 },
        { status: 'retired', count: 3 },
      ],
      by_type: [
        { type: 'decision', count: 8 },
        { type: 'pattern', count: 7 },
      ],
      by_confidence: [
        { level: 'high', count: 10 },
        { level: 'medium', count: 5 },
      ],
      active_count: 12,
      superseded_count: 0,
      retired_count: 3,
      oldest_memory: TS,
      most_recent: TS,
      unique_tags_count: 4,
    };

    expect(stats.total_memories).toBe(15);
    expect(stats.active_count).toBe(12);
    expect(stats.retired_count).toBe(3);
    expect(stats.unique_tags_count).toBe(4);
    expect(stats.by_status).toHaveLength(2);
  });

  it('EddaStats allows optional timestamp fields to be absent', () => {
    const stats: EddaStats = {
      total_memories: 0,
      by_status: [],
      by_type: [],
      by_confidence: [],
      active_count: 0,
      superseded_count: 0,
      retired_count: 0,
      unique_tags_count: 0,
    };
    expect(stats.oldest_memory).toBeUndefined();
    expect(stats.most_recent).toBeUndefined();
  });
});

// =============================================================================
// IEddaPort interface structural completeness check
// =============================================================================

describe('IEddaPort interface structure (TCOV-015)', () => {
  // Every method the IEddaPort contract requires. The check below binds this
  // list to a *real* conforming implementation (the mock), so a method renamed
  // or dropped from the interface — and therefore from the mock — fails the
  // test. The list is not self-referential: it is asserted against the runtime
  // shape of `createMockEddaPort()`, not against itself.
  const REQUIRED_METHODS = [
    'promoteProposal',
    'createMemory',
    'createMemoryFromProposal',
    'updateMemory',
    'retireMemory',
    'retireMemoryById',
    'supersedeMemory',
    'getMemory',
    'getMemoryByProposalId',
    'queryMemories',
    'getActiveMemories',
    'getMemoriesByType',
    'searchMemories',
    'memoryExists',
    'getEvolutionChain',
    'getLatestVersion',
    'resolveProvenance',
    'isAvailable',
    'getStats',
    'countMemories',
    'exportMemories',
    'importMemories',
  ] as const;

  it('is satisfied by a conforming mock implementation (compile-time + runtime)', () => {
    // Compile-time conformance: if the mock no longer satisfies IEddaPort (a
    // method added to the interface, or a signature drift), this assignment
    // fails to typecheck and the build breaks.
    const port: IEddaPort = createMockEddaPort();

    // Runtime conformance: every required method is actually present and
    // callable on the real implementation.
    for (const name of REQUIRED_METHODS) {
      expect(typeof (port as unknown as Record<string, unknown>)[name]).toBe('function');
    }
  });

  it('implements exactly the documented contract methods (ignoring test helpers)', () => {
    const port = createMockEddaPort();
    // `MockEddaPort` extends `IEddaPort` with `_`-prefixed test helpers
    // (`_reset`, `_getAll`, …); exclude those so we compare only the real
    // contract surface. This catches a contract method added to the interface
    // (and mock) without updating REQUIRED_METHODS, and vice versa.
    const contractMethods = Object.getOwnPropertyNames(port).filter(
      (key) =>
        !key.startsWith('_') &&
        typeof (port as unknown as Record<string, unknown>)[key] === 'function'
    );
    expect(contractMethods.sort()).toEqual([...REQUIRED_METHODS].sort());
  });
});
