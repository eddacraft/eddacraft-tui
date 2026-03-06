import { describe, expect, it } from 'vitest';
import {
  DecisionMemorySchema,
  PatternMemorySchema,
  ConstraintMemorySchema,
  WarningMemorySchema,
  DoctrineMemorySchema,
  LessonMemorySchema,
  TypedMemorySchema,
  validateMemoryMetadata,
  createTypedMemory,
  parseTypedMemory,
} from './memory-types.js';
import type { MemoryType } from './edda-memory.js';

const VALID_UUID = '550e8400-e29b-41d4-a716-446655440000';
const VALID_TIMESTAMP = '2024-01-15T14:30:00.000Z';

function buildBaseMemory<T extends MemoryType>(type: T, metadata: unknown) {
  return {
    id: VALID_UUID,
    type,
    status: 'active' as const,
    schema_version: 1,
    statement: 'Canonical memory statement',
    context: {
      when: 'During architecture review',
      why: 'Preserve institutional knowledge',
      conditions: ['New features'],
      tags: ['architecture'],
    },
    metadata,
    confidence: 'high' as const,
    confidence_rationale: 'Confirmed by repeated outcomes',
    provenance: {
      kindling_sources: [
        {
          observation_id: VALID_UUID,
          session_id: VALID_UUID,
          kind: 'gate_evaluated',
          timestamp: VALID_TIMESTAMP,
        },
      ],
      source_sessions: [VALID_UUID],
    },
    attribution: {
      actor: 'maintainer@eddacraft.dev',
      timestamp: VALID_TIMESTAMP,
      method: 'manual_edit' as const,
      reason: 'Promoted after review',
    },
    evolution: {
      supersedes: [],
    },
    created_at: VALID_TIMESTAMP,
    updated_at: VALID_TIMESTAMP,
  };
}

describe('Memory type schemas (EDDA-002)', () => {
  it('validates decision memory metadata', () => {
    const result = DecisionMemorySchema.safeParse(
      buildBaseMemory('decision', {
        decision_point: 'Choose storage backend',
        alternatives_considered: ['SQLite', 'PostgreSQL'],
        reversible: true,
      })
    );

    expect(result.success).toBe(true);
  });

  it('validates pattern memory metadata', () => {
    const result = PatternMemorySchema.safeParse(
      buildBaseMemory('pattern', {
        pattern_name: 'Adapter registry',
        applies_to: ['contracts', 'runtime'],
      })
    );

    expect(result.success).toBe(true);
  });

  it('validates constraint memory metadata', () => {
    const result = ConstraintMemorySchema.safeParse(
      buildBaseMemory('constraint', {
        constraint_type: 'policy',
        enforcement: 'hard',
      })
    );

    expect(result.success).toBe(true);
  });

  it('validates warning memory metadata', () => {
    const result = WarningMemorySchema.safeParse(
      buildBaseMemory('warning', {
        severity: 'high',
        affected_areas: ['ci', 'release'],
      })
    );

    expect(result.success).toBe(true);
  });

  it('validates doctrine memory metadata', () => {
    const result = DoctrineMemorySchema.safeParse(
      buildBaseMemory('doctrine', {
        principle: 'Prefer deterministic validation over inference',
        source: 'ADR-001',
      })
    );

    expect(result.success).toBe(true);
  });

  it('validates lesson memory metadata', () => {
    const result = LessonMemorySchema.safeParse(
      buildBaseMemory('lesson', {
        lesson_type: 'failure',
        key_takeaway: 'Build before cross-package tests',
      })
    );

    expect(result.success).toBe(true);
  });

  it('rejects wrong metadata for a memory type', () => {
    const result = DecisionMemorySchema.safeParse(
      buildBaseMemory('decision', {
        severity: 'critical',
      })
    );

    expect(result.success).toBe(false);
  });

  it('parses discriminated union for all memory types', () => {
    const memories = [
      buildBaseMemory('decision', { decision_point: 'Select queue technology' }),
      buildBaseMemory('pattern', { pattern_name: 'Single responsibility modules' }),
      buildBaseMemory('constraint', { constraint_type: 'technical' }),
      buildBaseMemory('warning', { severity: 'medium' }),
      buildBaseMemory('doctrine', { principle: 'Human review before promotion' }),
      buildBaseMemory('lesson', { lesson_type: 'mixed' }),
    ];

    for (const memory of memories) {
      expect(TypedMemorySchema.safeParse(memory).success).toBe(true);
    }
  });
});

describe('Memory type utilities (EDDA-002)', () => {
  it('validateMemoryMetadata returns typed metadata for valid input', () => {
    const metadata = validateMemoryMetadata({
      type: 'constraint',
      metadata: {
        constraint_type: 'resource',
        workaround: 'Scale runners during release week',
      },
    });

    expect(metadata).not.toBeNull();
    expect(metadata?.constraint_type).toBe('resource');
  });

  it('validateMemoryMetadata returns null for invalid input', () => {
    const metadata = validateMemoryMetadata({
      type: 'warning',
      metadata: {
        warning_type: 'not-valid-for-edda',
      },
    });

    expect(metadata).toBeNull();
  });

  it('createTypedMemory validates and returns typed memory', () => {
    const memory = createTypedMemory(
      buildBaseMemory('doctrine', {
        principle: 'Measure twice, migrate once',
        exceptions: ['Emergency rollback'],
      })
    );

    expect(memory.type).toBe('doctrine');
    if (memory.type === 'doctrine') {
      expect(memory.metadata.principle).toBe('Measure twice, migrate once');
    }
  });

  it('parseTypedMemory parses a valid typed memory', () => {
    const memory = parseTypedMemory(
      buildBaseMemory('lesson', {
        lesson_type: 'success',
        key_takeaway: 'Explicit contracts reduce drift',
      })
    );

    expect(memory.type).toBe('lesson');
    if (memory.type === 'lesson') {
      expect(memory.metadata.lesson_type).toBe('success');
    }
  });

  it('parseTypedMemory throws when metadata does not match type', () => {
    expect(() =>
      parseTypedMemory(
        buildBaseMemory('pattern', {
          decision_point: 'This belongs to decision metadata',
        })
      )
    ).toThrow();
  });
});
