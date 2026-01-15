import { describe, it, expect } from 'vitest';
import {
  APSPlanSchema,
  validatePlan,
  createPlan,
  type APSPlan,
  type Provenance,
} from './aps.schema.js';

describe('APS Schema', () => {
  const validProvenance: Provenance = {
    timestamp: new Date().toISOString(),
    source: 'cli',
    version: '1.0.0',
  };

  describe('APSPlanSchema', () => {
    it('should validate a valid plan', () => {
      const validPlan = {
        id: 'aps-12345678',
        hash: 'a'.repeat(64),
        intent: 'This is a test plan to validate the schema',
        schema_version: '0.1.0',
        proposed_changes: [],
        provenance: validProvenance,
        validations: {
          required_checks: ['lint', 'test'],
          skip_checks: [],
        },
      };

      const result = APSPlanSchema.safeParse(validPlan);
      expect(result.success).toBe(true);
      if (result.success) {
        expect(result.data).toMatchObject(validPlan);
      }
    });

    it('should reject invalid plan ID format', () => {
      const invalidPlan = {
        id: 'invalid-id',
        hash: 'a'.repeat(64),
        intent: 'This is a test plan',
        schema_version: '0.1.0',
        proposed_changes: [],
        provenance: validProvenance,
        validations: {
          required_checks: [],
          skip_checks: [],
        },
      };

      const result = APSPlanSchema.safeParse(invalidPlan);
      expect(result.success).toBe(false);
    });

    it('should reject invalid hash format', () => {
      const invalidPlan = {
        id: 'aps-12345678',
        hash: 'invalid-hash',
        intent: 'This is a test plan',
        schema_version: '0.1.0',
        proposed_changes: [],
        provenance: validProvenance,
        validations: {
          required_checks: [],
          skip_checks: [],
        },
      };

      const result = APSPlanSchema.safeParse(invalidPlan);
      expect(result.success).toBe(false);
    });

    it('should reject intent shorter than 10 characters', () => {
      const invalidPlan = {
        id: 'aps-12345678',
        hash: 'a'.repeat(64),
        intent: 'Too short',
        schema_version: '0.1.0',
        proposed_changes: [],
        provenance: validProvenance,
        validations: {
          required_checks: [],
          skip_checks: [],
        },
      };

      const result = APSPlanSchema.safeParse(invalidPlan);
      expect(result.success).toBe(false);
    });

    it('should reject intent longer than 500 characters', () => {
      const invalidPlan = {
        id: 'aps-12345678',
        hash: 'a'.repeat(64),
        intent: 'a'.repeat(501),
        schema_version: '0.1.0',
        proposed_changes: [],
        provenance: validProvenance,
        validations: {
          required_checks: [],
          skip_checks: [],
        },
      };

      const result = APSPlanSchema.safeParse(invalidPlan);
      expect(result.success).toBe(false);
    });

    it('should reject wrong schema version', () => {
      const invalidPlan = {
        id: 'aps-12345678',
        hash: 'a'.repeat(64),
        intent: 'This is a test plan',
        schema_version: '0.2.0', // Wrong version
        proposed_changes: [],
        provenance: validProvenance,
        validations: {
          required_checks: [],
          skip_checks: [],
        },
      };

      const result = APSPlanSchema.safeParse(invalidPlan);
      expect(result.success).toBe(false);
    });

    it('should accept optional fields', () => {
      const planWithOptionals = {
        id: 'aps-12345678',
        hash: 'a'.repeat(64),
        intent: 'This is a test plan with optional fields',
        schema_version: '0.1.0',
        proposed_changes: [],
        provenance: validProvenance,
        validations: {
          required_checks: [],
          skip_checks: [],
        },
        tags: ['test', 'validation'],
        metadata: {
          custom: 'value',
        },
      };

      const result = APSPlanSchema.safeParse(planWithOptionals);
      expect(result.success).toBe(true);
    });

    it('should reject unknown properties due to strict mode', () => {
      const planWithUnknown = {
        id: 'aps-12345678',
        hash: 'a'.repeat(64),
        intent: 'This is a test plan',
        schema_version: '0.1.0',
        proposed_changes: [],
        provenance: validProvenance,
        validations: {
          required_checks: [],
          skip_checks: [],
        },
        unknownField: 'should not be allowed',
      };

      const result = APSPlanSchema.safeParse(planWithUnknown);
      expect(result.success).toBe(false);
    });
  });

  describe('validatePlan', () => {
    it('should return formatted errors for invalid plan', () => {
      const invalidPlan = {
        id: 'wrong',
        hash: 'short',
        intent: 'short',
        schema_version: '0.1.0',
        proposed_changes: [],
        provenance: validProvenance,
        validations: {
          required_checks: [],
          skip_checks: [],
        },
      };

      const result = validatePlan(invalidPlan);
      expect(result.success).toBe(false);
      expect(result.errors).toBeDefined();
      expect(result.errors!.length).toBeGreaterThan(0);
      expect(result.errors![0]).toHaveProperty('path');
      expect(result.errors![0]).toHaveProperty('message');
    });

    it('should return valid data for valid plan', () => {
      const validPlan = {
        id: 'aps-12345678',
        hash: 'a'.repeat(64),
        intent: 'This is a valid test plan',
        schema_version: '0.1.0',
        proposed_changes: [],
        provenance: validProvenance,
        validations: {
          required_checks: [],
          skip_checks: [],
        },
      };

      const result = validatePlan(validPlan);
      expect(result.success).toBe(true);
      expect(result.data).toBeDefined();
      expect(result.errors).toBeUndefined();
    });
  });

  describe('createPlan', () => {
    it('should create a plan with defaults', () => {
      const plan = createPlan({
        id: 'aps-12345678',
        intent: 'Create a new feature',
        provenance: validProvenance,
      });

      expect(plan.id).toBe('aps-12345678');
      expect(plan.intent).toBe('Create a new feature');
      expect(plan.schema_version).toBe('0.1.0');
      expect(plan.proposed_changes).toEqual([]);
      expect(plan.validations.required_checks).toContain('lint');
      expect(plan.validations.required_checks).toContain('test');
      expect(plan.validations.required_checks).toContain('coverage');
      expect(plan.validations.required_checks).toContain('secrets');
    });

    it('should create a plan with custom changes and validations', () => {
      const plan = createPlan({
        id: 'aps-87654321',
        intent: 'Update configuration',
        provenance: validProvenance,
        changes: [
          {
            type: 'config_update',
            path: 'config.json',
            description: 'Update config file',
          },
        ],
        validations: {
          required_checks: ['custom-check'],
          skip_checks: ['lint'],
        },
      });

      expect(plan.proposed_changes).toHaveLength(1);
      expect(plan.proposed_changes[0].type).toBe('config_update');
      expect(plan.validations.required_checks).toEqual(['custom-check']);
      expect(plan.validations.skip_checks).toEqual(['lint']);
    });
  });

  describe('Type inference', () => {
    it('should infer types correctly', () => {
      // This is a compile-time test - if it compiles, types are working
      // We use validatePlan to get a properly branded type
      const planData = {
        id: 'aps-12345678',
        hash: 'a'.repeat(64),
        intent: 'Type inference test',
        schema_version: '0.1.0' as const,
        proposed_changes: [
          {
            type: 'file_create' as const,
            path: '/test/file.ts',
            description: 'Create test file',
          },
        ],
        provenance: {
          timestamp: new Date().toISOString(),
          source: 'api' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: [],
          skip_checks: [],
        },
      };

      const result = validatePlan(planData);
      expect(result.success).toBe(true);

      if (result.success && result.data) {
        const plan: APSPlan = result.data;

        // Type checks - these should compile without errors
        expect(plan.id).toMatch(/^aps-/);
        expect(plan.proposed_changes[0].type).toBe('file_create');
        expect(plan.provenance.source).toBe('api');
      }
    });
  });
});
