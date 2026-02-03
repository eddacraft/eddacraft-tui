/**
 * Generic Serializer Tests
 * Tests for type-safe serialization of APS plans
 */

import { describe, it, expect } from 'vitest';
import { serializeToGeneric } from '../serializer.js';
import type { APSPlan } from '@eddacraft/anvil-core';

describe('serializeToGeneric', () => {
  const basePlan: APSPlan = {
    id: 'aps-test123',
    schema_version: '0.1.0',
    hash: 'test-hash',
    intent: 'Test plan',
    proposed_changes: [],
    provenance: {
      timestamp: '2024-01-01T00:00:00Z',
      author: 'test@example.com',
      source: 'cli',
      version: '1.0.0',
    },
    validations: {
      required_checks: [],
      skip_checks: [],
    },
    evidence: [],
    executions: [],
  };

  it('should handle plan with no metadata', () => {
    const result = serializeToGeneric(basePlan);

    expect(result).toContain('# Test plan');
    expect(result).toContain('## Purpose');
  });

  it('should handle plan with string title metadata', () => {
    const plan = {
      ...basePlan,
      metadata: {
        title: 'Custom Title',
      },
    };

    const result = serializeToGeneric(plan);

    expect(result).toContain('# Custom Title');
  });

  it('should handle plan with non-string title metadata gracefully', () => {
    const plan = {
      ...basePlan,
      metadata: {
        title: 123 as unknown as string, // Invalid type
      },
    };

    const result = serializeToGeneric(plan);

    // Should fall back to intent
    expect(result).toContain('# Test plan');
  });

  it('should handle plan with string overview metadata', () => {
    const plan = {
      ...basePlan,
      metadata: {
        overview: 'This is an overview',
      },
    };

    const result = serializeToGeneric(plan);

    expect(result).toContain('## Overview');
    expect(result).toContain('This is an overview');
  });

  it('should handle plan with non-string overview metadata gracefully', () => {
    const plan = {
      ...basePlan,
      metadata: {
        overview: { text: 'overview' } as unknown as string, // Invalid type
      },
    };

    const result = serializeToGeneric(plan);

    // Should fall back to purpose section
    expect(result).toContain('## Purpose');
    expect(result).not.toContain('## Overview');
  });

  it('should handle plan with string array goals', () => {
    const plan = {
      ...basePlan,
      metadata: {
        goals: ['Goal 1', 'Goal 2', 'Goal 3'],
      },
    };

    const result = serializeToGeneric(plan);

    expect(result).toContain('## Goals');
    expect(result).toContain('- Goal 1');
    expect(result).toContain('- Goal 2');
    expect(result).toContain('- Goal 3');
  });

  it('should handle plan with non-string array goals gracefully', () => {
    const plan = {
      ...basePlan,
      metadata: {
        goals: [1, 2, 3] as unknown as string[], // Invalid type
      },
    };

    const result = serializeToGeneric(plan);

    // Should not include goals section
    expect(result).not.toContain('## Goals');
  });

  it('should handle plan with mixed-type goals array gracefully', () => {
    const plan = {
      ...basePlan,
      metadata: {
        goals: ['Goal 1', 123, 'Goal 2'] as unknown as string[], // Mixed types
      },
    };

    const result = serializeToGeneric(plan);

    // Should not include goals section since not all elements are strings
    expect(result).not.toContain('## Goals');
  });

  it('should handle plan with all metadata types', () => {
    const plan = {
      ...basePlan,
      metadata: {
        title: 'Full Plan',
        overview: 'Complete overview',
        goals: ['Goal A', 'Goal B'],
      },
      proposed_changes: [
        {
          type: 'file_create' as const,
          path: 'test.ts',
          description: 'Create test file',
        },
      ],
    };

    const result = serializeToGeneric(plan);

    expect(result).toContain('# Full Plan');
    expect(result).toContain('## Overview');
    expect(result).toContain('Complete overview');
    expect(result).toContain('## Goals');
    expect(result).toContain('- Goal A');
    expect(result).toContain('- Goal B');
    expect(result).toContain('## Changes');
    expect(result).toContain('### Files to Create');
    expect(result).toContain('test.ts');
  });
});
