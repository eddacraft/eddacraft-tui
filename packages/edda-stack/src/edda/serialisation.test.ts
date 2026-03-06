import { describe, expect, it } from 'vitest';
import {
  createActionId,
  createMemoryId,
  createObservationId,
  createProposalId,
  createSessionId,
} from '../contracts/identifiers.js';
import type { MemoryObject } from '../contracts/edda-memory.js';
import {
  deserialiseIndex,
  deserialiseMemory,
  serialiseIndex,
  serialiseMemory,
  type MemoryIndex,
} from './serialisation.js';

const MEMORY_ID = createMemoryId('550e8400-e29b-41d4-a716-446655440001');
const PROPOSAL_ID = createProposalId('550e8400-e29b-41d4-a716-446655440002');
const SESSION_ID = createSessionId('550e8400-e29b-41d4-a716-446655440003');
const OBSERVATION_ID = createObservationId('550e8400-e29b-41d4-a716-446655440004');
const ACTION_ID = createActionId('550e8400-e29b-41d4-a716-446655440005');

function createMemory(): MemoryObject {
  return {
    id: MEMORY_ID,
    type: 'pattern',
    status: 'active',
    schema_version: 1,
    statement: 'Prefer deterministic checks before applying generated code.',
    context: {
      when: 'During pre-commit verification',
      why: 'Prevents architecture drift entering mainline',
      conditions: ['Code changes affect runtime behaviour'],
      scope: 'Monorepo',
      tags: ['quality', 'determinism'],
    },
    confidence: 'high',
    confidence_rationale: 'Observed repeatedly across release incidents.',
    provenance: {
      ember_source: {
        proposal_id: PROPOSAL_ID,
        proposal_type: 'pattern',
        confidence: 0.86,
        created_at: '2026-01-10T10:00:00.000Z',
      },
      kindling_sources: [
        {
          observation_id: OBSERVATION_ID,
          session_id: SESSION_ID,
          kind: 'gate_evaluated',
          timestamp: '2026-01-10T10:01:00.000Z',
        },
      ],
      source_sessions: [SESSION_ID],
      related_plans: ['edda-phase-b'],
      related_gates: ['architecture'],
      related_actions: [ACTION_ID],
    },
    attribution: {
      actor: 'agent/curator',
      timestamp: '2026-01-10T10:02:00.000Z',
      method: 'cli_command',
      reason: 'Promoted after human review',
    },
    evolution: {
      supersedes: [],
    },
    created_at: '2026-01-10T10:02:00.000Z',
    updated_at: '2026-01-10T10:03:00.000Z',
  };
}

describe('serialisation (EDDA-007)', () => {
  it('round-trips memory objects without structural loss', () => {
    const memory = createMemory();

    const yaml = serialiseMemory(memory);
    const parsed = deserialiseMemory(yaml);

    expect(parsed).toEqual(memory);
  });

  it('throws descriptive errors for invalid YAML', () => {
    expect(() => deserialiseMemory('id: [not-valid')).toThrow('Failed to parse memory YAML');
  });

  it('fails validation for missing required fields', () => {
    const yaml = `type: pattern\nstatement: missing id\n`;

    expect(() => deserialiseMemory(yaml)).toThrow('Invalid memory payload');
    expect(() => deserialiseMemory(yaml)).toThrow('id');
  });

  it('fails schema validation for invalid field values', () => {
    const yaml = serialiseMemory(createMemory()).replace('confidence: high', 'confidence: certain');

    expect(() => deserialiseMemory(yaml)).toThrow('Invalid memory payload');
    expect(() => deserialiseMemory(yaml)).toThrow('confidence');
  });

  it('serialises and deserialises memory index', () => {
    const index: MemoryIndex = {
      memories: [
        {
          id: MEMORY_ID,
          type: 'pattern',
          status: 'active',
          path: 'memories/pattern/550e8400-e29b-41d4-a716-446655440001.yaml',
          statement: 'Prefer deterministic checks before applying generated code.',
          confidence: 'high',
          tags: ['quality', 'determinism'],
          created_at: '2026-01-10T10:02:00.000Z',
          proposal_id: PROPOSAL_ID,
        },
      ],
      updated_at: '2026-01-10T10:03:00.000Z',
    };

    const yaml = serialiseIndex(index);
    const parsed = deserialiseIndex(yaml);

    expect(parsed).toEqual(index);
  });
});
