/**
 * Plan Lifecycle — E2E Tests
 *
 * Tests the full lifecycle of an APS plan across package boundaries:
 *   contracts (create) → core (validate) → runtime (gate) → aps (parse)
 *
 * This exercises the real code paths that a plan goes through in
 * production, without mocking any domain logic.
 *
 * Surface: Core domain (contracts + core + runtime + aps)
 */

import { describe, it, expect } from 'vitest';
import { createPlan, validatePlan, APS_SCHEMA_VERSION, type APSPlan } from '@eddacraft/anvil-core';
import { validateAPSPlan, generateHash } from '@eddacraft/anvil-core';
import { makePlan, makeChange, resetFixtures } from '../helpers/fixtures.js';

beforeEach(() => {
  resetFixtures();
});

describe('Plan Lifecycle › Creation', () => {
  it('createPlan produces a valid plan with correct schema version', () => {
    const plan = createPlan({
      intent: 'Add user authentication',
      proposed_changes: [
        {
          file: 'src/auth.ts',
          type: 'add',
          description: 'Authentication module',
        },
      ],
      provenance: {
        timestamp: new Date().toISOString(),
        author: 'e2e-test',
        source: 'cli',
        version: '0.1.0',
      },
      validations: {
        required_checks: ['lint', 'test'],
        skip_checks: [],
      },
    });

    expect(plan.id).toBeDefined();
    expect(plan.schema_version).toBe(APS_SCHEMA_VERSION);
    expect(plan.intent).toBe('Add user authentication');
    expect(plan.proposed_changes).toHaveLength(1);
    expect(plan.hash).toBeDefined();
  });

  it('each plan gets a unique ID', () => {
    const plan1 = makePlan({ intent: 'Plan A' });
    const plan2 = makePlan({ intent: 'Plan B' });
    expect(plan1.id).not.toBe(plan2.id);
  });

  it('plan hash is deterministic for the same content', () => {
    const data = {
      intent: 'Deterministic test',
      proposed_changes: [makeChange({ file: 'a.ts', type: 'add', description: 'test' })],
      provenance: {
        timestamp: '2025-01-01T00:00:00.000Z',
        author: 'test',
        source: 'cli' as const,
        version: '0.1.0',
      },
      validations: { required_checks: ['lint'], skip_checks: [] },
    };

    const plan1 = createPlan(data);
    const plan2 = createPlan(data);
    expect(plan1.hash).toBe(plan2.hash);
  });
});

describe('Plan Lifecycle › Validation', () => {
  it('a well-formed plan passes schema validation', () => {
    const plan = makePlan();
    const result = validatePlan(plan);
    expect(result.success).toBe(true);
  });

  it('validateAPSPlan catches missing required fields', () => {
    const badPlan = { id: 'broken', intent: '' } as unknown as APSPlan;
    const result = validateAPSPlan(badPlan);
    expect(result.valid).toBe(false);
    expect(result.errors.length).toBeGreaterThan(0);
  });

  it('a tampered hash is detected', () => {
    const plan = makePlan();
    const tampered = { ...plan, hash: 'tampered-hash-value' };
    const result = validateAPSPlan(tampered);
    expect(result.valid).toBe(false);
  });
});

describe('Plan Lifecycle › Hash Integrity', () => {
  it('generateHash is consistent with createPlan hash', () => {
    const plan = makePlan();
    // Strip the hash, regenerate, and compare
    const { hash: _original, id: _id, ...planData } = plan;
    const recomputed = generateHash(planData);
    expect(recomputed).toBe(plan.hash);
  });

  it('changing intent changes the hash', () => {
    const plan1 = makePlan({ intent: 'Intent A' });
    const plan2 = makePlan({ intent: 'Intent B' });
    expect(plan1.hash).not.toBe(plan2.hash);
  });
});

describe('Plan Lifecycle › Multi-change Plans', () => {
  it('supports plans with multiple proposed changes', () => {
    const plan = makePlan({
      intent: 'Refactor authentication module',
      proposed_changes: [
        makeChange({ file: 'src/auth/login.ts', type: 'modify', description: 'Update login flow' }),
        makeChange({ file: 'src/auth/register.ts', type: 'add', description: 'New registration' }),
        makeChange({
          file: 'src/auth/legacy.ts',
          type: 'delete',
          description: 'Remove legacy code',
        }),
      ],
    });

    expect(plan.proposed_changes).toHaveLength(3);
    const result = validatePlan(plan);
    expect(result.success).toBe(true);
  });
});
