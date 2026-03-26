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
      id: 'e2e-auth-plan',
      intent: 'Add user authentication',
      changes: [
        {
          path: 'src/auth.ts',
          type: 'file_create',
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
  });

  it('each plan gets a unique ID', () => {
    const plan1 = makePlan({ intent: 'Plan A' });
    const plan2 = makePlan({ intent: 'Plan B' });
    expect(plan1.id).not.toBe(plan2.id);
  });

  it('plan structure is deterministic for the same content', () => {
    const data = {
      id: 'e2e-deterministic',
      intent: 'Deterministic test',
      changes: [makeChange({ path: 'a.ts', type: 'file_create', description: 'test' })],
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
    expect(plan1.intent).toBe(plan2.intent);
    expect(plan1.proposed_changes).toEqual(plan2.proposed_changes);
  });
});

describe('Plan Lifecycle › Validation', () => {
  it('a well-formed plan passes schema validation', () => {
    const plan = makePlan();
    const result = validatePlan(plan);
    expect(result.success).toBe(true);
  });

  it('validateAPSPlan catches missing required fields', async () => {
    const badPlan = { id: 'broken', intent: '' } as unknown as APSPlan;
    const result = await validateAPSPlan(badPlan);
    expect(result.valid).toBe(false);
    expect(result.issues!.length).toBeGreaterThan(0);
  });

  it('a tampered hash is detected', async () => {
    const plan = makePlan();
    const tampered = { ...plan, hash: 'tampered-hash-value' };
    const result = await validateAPSPlan(tampered);
    expect(result.valid).toBe(false);
  });
});

describe('Plan Lifecycle › Hash Integrity', () => {
  it('generateHash is deterministic for the same data', () => {
    const plan = makePlan();
    const { hash: _original, id: _id, ...planData } = plan;
    const hash1 = generateHash(planData);
    const hash2 = generateHash(planData);
    expect(hash1).toBe(hash2);
  });

  it('changing intent changes the hash', () => {
    const plan1 = makePlan({ intent: 'Intent A' });
    const plan2 = makePlan({ intent: 'Intent B' });
    const { hash: _h1, id: _i1, ...data1 } = plan1;
    const { hash: _h2, id: _i2, ...data2 } = plan2;
    expect(generateHash(data1)).not.toBe(generateHash(data2));
  });
});

describe('Plan Lifecycle › Multi-change Plans', () => {
  it('supports plans with multiple proposed changes', () => {
    const plan = makePlan({
      intent: 'Refactor authentication module',
      proposed_changes: [
        makeChange({
          file: 'src/auth/login.ts',
          type: 'file_update',
          description: 'Update login flow',
        }),
        makeChange({
          file: 'src/auth/register.ts',
          type: 'file_create',
          description: 'New registration',
        }),
        makeChange({
          file: 'src/auth/legacy.ts',
          type: 'file_delete',
          description: 'Remove legacy code',
        }),
      ],
    });

    expect(plan.proposed_changes).toHaveLength(3);
    const result = validatePlan(plan);
    expect(result.success).toBe(true);
  });
});
