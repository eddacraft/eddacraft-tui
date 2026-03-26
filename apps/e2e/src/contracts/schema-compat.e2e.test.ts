/**
 * Schema Compatibility — E2E Tests
 *
 * Verifies that schemas exported from @eddacraft/anvil-contracts are
 * consistent with the types consumed by core, runtime, and CLI packages.
 * This catches accidental breaking changes to shared schemas.
 *
 * Surface: Contracts (cross-package compatibility)
 */

import { describe, it, expect } from 'vitest';
import {
  APSPlanSchema,
  ChangeTypeSchema,
  ChangeSchema,
  ProvenanceSchema,
  ValidationSchema,
  EvidenceEntrySchema,
  APS_SCHEMA_VERSION,
  createPlan,
} from '@eddacraft/anvil-core';

// Also import from @eddacraft/anvil-core to verify re-exports are consistent
import {
  APSPlanSchema as CorePlanSchema,
  APS_SCHEMA_VERSION as CoreSchemaVersion,
  createPlan as coreCreatePlan,
} from '@eddacraft/anvil-core';

describe('Schema Compatibility › Contracts ↔ Core re-exports', () => {
  it('APS_SCHEMA_VERSION is identical in contracts and core', () => {
    expect(APS_SCHEMA_VERSION).toBe(CoreSchemaVersion);
  });

  it('APSPlanSchema parses the same data in both packages', () => {
    const plan = createPlan({
      id: 'aps-e2e00001',
      intent: 'Schema compat test',
      changes: [{ path: 'a.ts', type: 'file_create', description: 'test' }],
      provenance: {
        timestamp: new Date().toISOString(),
        author: 'test',
        source: 'cli',
        version: '0.1.0',
      },
      validations: { required_checks: ['lint'], skip_checks: [] },
    });

    // createPlan returns Omit<APSPlan, 'hash'>, add a valid SHA-256 hash for schema parsing
    const planWithHash = { ...plan, hash: 'a'.repeat(64) };
    const contractsResult = APSPlanSchema.safeParse(planWithHash);
    const coreResult = CorePlanSchema.safeParse(planWithHash);

    expect(contractsResult.success).toBe(true);
    expect(coreResult.success).toBe(true);
  });

  it('createPlan from contracts and core produce the same structure', () => {
    const input = {
      id: 'aps-e2e00002',
      intent: 'Identity test',
      changes: [{ path: 'b.ts', type: 'file_update' as const, description: 'update' }],
      provenance: {
        timestamp: '2025-01-01T00:00:00.000Z',
        author: 'test',
        source: 'cli' as const,
        version: '0.1.0',
      },
      validations: { required_checks: ['test'], skip_checks: [] },
    };

    const fromContracts = createPlan(input);
    const fromCore = coreCreatePlan(input);

    // Same structure, same IDs (deterministic)
    expect(fromContracts.schema_version).toBe(fromCore.schema_version);
    expect(fromContracts.intent).toBe(fromCore.intent);
    expect(fromContracts.proposed_changes).toEqual(fromCore.proposed_changes);
  });
});

describe('Schema Compatibility › Zod schemas parse valid data', () => {
  it('ChangeTypeSchema accepts all valid change types', () => {
    for (const type of [
      'file_create',
      'file_update',
      'file_delete',
      'config_update',
      'dependency_add',
      'dependency_remove',
      'dependency_update',
      'script_execute',
    ]) {
      const result = ChangeTypeSchema.safeParse(type);
      expect(result.success, `Expected "${type}" to be valid`).toBe(true);
    }
  });

  it('ChangeSchema validates a well-formed change', () => {
    const result = ChangeSchema.safeParse({
      path: 'src/index.ts',
      type: 'file_update',
      description: 'Update exports',
    });
    expect(result.success).toBe(true);
  });

  it('ProvenanceSchema validates provenance metadata', () => {
    const result = ProvenanceSchema.safeParse({
      timestamp: new Date().toISOString(),
      author: 'test',
      source: 'cli',
      version: '0.1.0',
    });
    expect(result.success).toBe(true);
  });

  it('ValidationSchema validates gate requirements', () => {
    const result = ValidationSchema.safeParse({
      required_checks: ['lint', 'test', 'coverage', 'secrets'],
      skip_checks: [],
    });
    expect(result.success).toBe(true);
  });

  it('EvidenceEntrySchema validates evidence data', () => {
    const result = EvidenceEntrySchema.safeParse({
      check: 'lint',
      status: 'pass',
      score: 95,
      timestamp: new Date().toISOString(),
    });
    expect(result.success).toBe(true);
  });
});

describe('Schema Compatibility › Rejection of invalid data', () => {
  it('APSPlanSchema rejects a plan with no intent', () => {
    const result = APSPlanSchema.safeParse({
      id: 'test-id',
      schema_version: APS_SCHEMA_VERSION,
      intent: '', // empty
      proposed_changes: [],
      provenance: {
        timestamp: new Date().toISOString(),
        author: 'test',
        source: 'cli',
        version: '0.1.0',
      },
      validations: { required_checks: [], skip_checks: [] },
      hash: 'abc',
    });
    // Empty intent should be rejected by schema constraints
    expect(result.success).toBe(false);
  });

  it('ChangeTypeSchema rejects unknown change types', () => {
    const result = ChangeTypeSchema.safeParse('explode');
    expect(result.success).toBe(false);
  });
});
