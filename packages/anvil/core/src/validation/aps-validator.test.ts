/**
 * APS Validator Tests
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { APSValidator, validateAPSPlan } from './aps-validator.js';
import { createPlan, type APSPlan } from '../contracts/index.js';
import { generateHash } from '../crypto/index.js';
import { validator as defaultValidator } from './index.js';

describe('APSValidator', () => {
  let validator: APSValidator;
  let validPlan: APSPlan;

  beforeEach(() => {
    validator = new APSValidator();

    // Create a valid plan for testing
    const planWithoutHash = createPlan({
      id: 'aps-12345678',
      intent: 'Test plan for validation',
      provenance: {
        timestamp: new Date().toISOString(),
        author: 'test-user',
        source: 'manual',
        version: '1.0.0',
      },
    });

    // Add a valid mock hash (SHA-256 format)
    validPlan = {
      ...planWithoutHash,
      hash: 'a'.repeat(64), // Mock SHA-256 hash
    } as APSPlan;
  });

  describe('validateSchema', () => {
    it('should validate a correct plan', async () => {
      const result = await validator.validateSchema(validPlan);

      expect(result.valid).toBe(true);
      expect(result.data).toEqual(validPlan);
      expect(result.summary).toContain('passed');
      expect(result.issues).toBeUndefined();
    });

    it('should reject an empty object', async () => {
      const result = await validator.validateSchema({});

      expect(result.valid).toBe(false);
      expect(result.issues).toBeDefined();
      expect(result.issues?.length).toBeGreaterThan(0);
      expect(result.formattedErrors).toBeDefined();
    });

    it('should reject invalid id format', async () => {
      const invalidPlan = { ...validPlan, id: 'invalid-id' };
      const result = await validator.validateSchema(invalidPlan);

      expect(result.valid).toBe(false);
      expect(result.issues?.[0].path).toBe('id');
      expect(result.issues?.[0].message).toContain('pattern');
    });

    it('should reject too short intent', async () => {
      const invalidPlan = { ...validPlan, intent: 'short' };
      const result = await validator.validateSchema(invalidPlan);

      expect(result.valid).toBe(false);
      expect(result.issues?.[0].path).toBe('intent');
      expect(result.issues?.[0].message).toContain('too small');
    });

    it('should reject invalid schema version', async () => {
      const invalidPlan = { ...validPlan, schema_version: '2.0.0' };
      const result = await validator.validateSchema(invalidPlan);

      expect(result.valid).toBe(false);
      expect(result.issues?.[0].path).toBe('schema_version');
    });

    it('should reject extra properties', async () => {
      const invalidPlan = { ...validPlan, extraField: 'not allowed' };
      const result = await validator.validateSchema(invalidPlan);

      expect(result.valid).toBe(false);
      expect(result.issues?.[0].message).toContain('Unexpected properties');
    });
  });

  describe('validate', () => {
    it('should validate a complete valid plan', async () => {
      const result = await validator.validate(validPlan);

      expect(result.valid).toBe(true);
      expect(result.data).toEqual(validPlan);
      expect(result.summary).toContain('passed');
    });

    it('should handle non-strict mode', async () => {
      const invalidPlan = { ...validPlan, id: 'bad-id' };
      const result = await validator.validate(invalidPlan, { strict: false });

      expect(result.valid).toBe(false);
      expect(result.issues?.length).toBeGreaterThan(0);
    });

    it('should format errors for CLI', async () => {
      const invalidPlan = {
        id: 'bad-id',
        intent: 'short',
        schema_version: '2.0.0',
      };

      const result = await validator.validate(invalidPlan, { format: 'cli' });

      expect(result.valid).toBe(false);
      expect(result.formattedErrors).toBeDefined();
      expect(result.formattedErrors).toContain('❌');
      expect(result.formattedErrors).toContain('validation error');
    });

    it('should skip hash validation when not requested', async () => {
      const result = await validator.validate(validPlan, { validateHash: false });

      expect(result.valid).toBe(true);
      expect(result.summary).not.toContain('Hash');
    });

    it('should handle hash validation when crypto not available', async () => {
      const result = await validator.validate(validPlan, { validateHash: true });

      expect(result.valid).toBe(true);
      expect(result.summary).toContain('✅ Validation passed');
      expect(result.summary).toContain('Hash validation skipped');
    });
  });

  describe('isSchemaValid', () => {
    it('should quickly check schema validity', () => {
      expect(validator.isSchemaValid(validPlan)).toBe(true);
      expect(validator.isSchemaValid({})).toBe(false);
      expect(validator.isSchemaValid(null)).toBe(false);
      expect(validator.isSchemaValid('not an object')).toBe(false);
    });
  });

  describe('validateAPSPlan convenience function', () => {
    it('should work with the convenience function', async () => {
      const result = await validateAPSPlan(validPlan);
      expect(result.valid).toBe(true);
    });
  });

  describe('hash validation', () => {
    it('should validate hash when validator is set', async () => {
      // Mock hash validator
      const mockHashFn = (_data: unknown) => 'a'.repeat(64); // Mock SHA-256
      validator.setHashValidator(mockHashFn);

      const planWithHash = { ...validPlan, hash: 'a'.repeat(64) };
      const result = await validator.validate(planWithHash, { validateHash: true });

      expect(result.valid).toBe(true);
    });

    it('should detect hash mismatch', async () => {
      // Mock hash validator
      const mockHashFn = (_data: unknown) => 'a'.repeat(64);
      validator.setHashValidator(mockHashFn);

      const planWithWrongHash = { ...validPlan, hash: 'b'.repeat(64) };
      const result = await validator.validate(planWithWrongHash, { validateHash: true });

      expect(result.valid).toBe(false);
      expect(result.issues?.[0].code).toBe('HASH_MISMATCH');
    });
  });

  describe('integration with real hash module', () => {
    it('should validate with real hash function', async () => {
      // Create a new validator instance
      const realValidator = new APSValidator();
      realValidator.setHashValidator(generateHash);

      // Create a plan without hash
      const planWithoutHash = createPlan({
        id: 'aps-12345678',
        intent: 'Test plan for hash validation',
        provenance: {
          timestamp: '2024-01-01T00:00:00.000Z',
          author: 'test-user',
          source: 'manual',
          version: '1.0.0',
        },
      });

      // Generate the correct hash
      const { hash: _unused, ...dataToHash } = planWithoutHash as unknown as Record<
        string,
        unknown
      >;
      const correctHash = generateHash(dataToHash);

      // Add the hash to the plan
      const planWithCorrectHash = {
        ...planWithoutHash,
        hash: correctHash,
      } as APSPlan;

      // Validate with hash
      const result = await realValidator.validate(planWithCorrectHash, { validateHash: true });

      expect(result.valid).toBe(true);
      expect(result.summary).toContain('Hash validation passed');
    });

    it('should detect hash tampering with real hash function', async () => {
      // Create a new validator instance
      const realValidator = new APSValidator();
      realValidator.setHashValidator(generateHash);

      // Create a plan with an incorrect hash
      const plan = createPlan({
        id: 'aps-12345678',
        intent: 'Test plan for hash validation',
        provenance: {
          timestamp: '2024-01-01T00:00:00.000Z',
          author: 'test-user',
          source: 'manual',
          version: '1.0.0',
        },
      });

      const planWithBadHash = {
        ...plan,
        hash: 'b'.repeat(64), // Incorrect hash
      } as APSPlan;

      // Validate with hash
      const result = await realValidator.validate(planWithBadHash, { validateHash: true });

      expect(result.valid).toBe(false);
      expect(result.issues?.[0].code).toBe('HASH_MISMATCH');
      expect(result.issues?.[0].message).toContain('Hash mismatch');
    });

    it('should work with default validator from index', async () => {
      // Create a plan
      const planWithoutHash = createPlan({
        id: 'aps-87654321',
        intent: 'Test default validator hash integration',
        provenance: {
          timestamp: '2024-01-01T00:00:00.000Z',
          author: 'test-user',
          source: 'cli',
          version: '1.0.0',
        },
      });

      // Generate the correct hash
      const { hash: _unused, ...dataToHash } = planWithoutHash as unknown as Record<
        string,
        unknown
      >;
      const correctHash = generateHash(dataToHash);

      // Add the hash to the plan
      const planWithCorrectHash = {
        ...planWithoutHash,
        hash: correctHash,
      } as APSPlan;

      // Validate with hash using the default validator
      const result = await defaultValidator.validate(planWithCorrectHash, { validateHash: true });

      expect(result.valid).toBe(true);
      expect(result.summary).toContain('Hash validation passed');
    });
  });
});
