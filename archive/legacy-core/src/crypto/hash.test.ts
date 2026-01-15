/**
 * Hash Generation Tests
 */

import { describe, it, expect } from 'vitest';
import {
  generateHash,
  canonicalizeJSON,
  verifyHash,
  generatePlanId,
  isValidPlanId,
  isValidHash,
} from './hash.js';

describe('Hash Generation', () => {
  describe('generateHash', () => {
    it('should generate consistent hash for same input', () => {
      const data = { name: 'test', value: 123 };
      const hash1 = generateHash(data);
      const hash2 = generateHash(data);

      expect(hash1).toBe(hash2);
      expect(hash1).toHaveLength(64); // SHA-256 produces 64 hex chars
      expect(isValidHash(hash1)).toBe(true);
    });

    it('should generate different hashes for different inputs', () => {
      const data1 = { name: 'test1' };
      const data2 = { name: 'test2' };

      const hash1 = generateHash(data1);
      const hash2 = generateHash(data2);

      expect(hash1).not.toBe(hash2);
    });

    it('should hash strings directly', () => {
      const str = 'Hello, World!';
      const hash = generateHash(str);

      expect(hash).toHaveLength(64);
      expect(isValidHash(hash)).toBe(true);

      // Known SHA-256 hash for "Hello, World!"
      expect(hash).toBe('dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f');
    });

    it('should hash numbers and booleans', () => {
      const numHash = generateHash(42);
      const boolHash = generateHash(true);

      expect(isValidHash(numHash)).toBe(true);
      expect(isValidHash(boolHash)).toBe(true);
    });

    it('should handle null and undefined differently', () => {
      const nullHash = generateHash(null);
      const undefinedHash = generateHash(undefined);

      expect(nullHash).not.toBe(undefinedHash);
      expect(isValidHash(nullHash)).toBe(true);
      expect(isValidHash(undefinedHash)).toBe(true);
    });
  });

  describe('canonicalizeJSON', () => {
    it('should produce same output regardless of property order', () => {
      const obj1 = { a: 1, b: 2, c: 3 };
      const obj2 = { c: 3, a: 1, b: 2 };
      const obj3 = { b: 2, c: 3, a: 1 };

      const json1 = canonicalizeJSON(obj1);
      const json2 = canonicalizeJSON(obj2);
      const json3 = canonicalizeJSON(obj3);

      expect(json1).toBe(json2);
      expect(json2).toBe(json3);
    });

    it('should handle nested objects consistently', () => {
      const nested1 = {
        outer: { z: 1, a: 2 },
        inner: { nested: { deep: true, shallow: false } },
      };

      const nested2 = {
        inner: { nested: { shallow: false, deep: true } },
        outer: { a: 2, z: 1 },
      };

      expect(canonicalizeJSON(nested1)).toBe(canonicalizeJSON(nested2));
    });

    it('should handle arrays', () => {
      const arr = [3, 1, 4, 1, 5, 9];
      const json = canonicalizeJSON(arr);

      expect(json).toBe('[3,1,4,1,5,9]');
    });

    it('should handle mixed types', () => {
      const mixed = {
        str: 'hello',
        num: 42,
        bool: true,
        null: null,
        arr: [1, 2, 3],
        obj: { nested: 'value' },
      };

      const json = canonicalizeJSON(mixed);
      expect(json).toContain('"str":"hello"');
      expect(json).toContain('"num":42');
      expect(json).toContain('"bool":true');
      expect(json).toContain('"null":null');
    });

    it('should skip undefined values', () => {
      const obj = { a: 1, b: undefined, c: 3 };
      const json = canonicalizeJSON(obj);

      expect(json).toBe('{"a":1,"c":3}');
      expect(json).not.toContain('undefined');
    });

    it('should handle dates', () => {
      const date = new Date('2024-01-01T00:00:00.000Z');
      const obj = { created: date };
      const json = canonicalizeJSON(obj);

      expect(json).toContain('"2024-01-01T00:00:00.000Z"');
    });
  });

  describe('verifyHash', () => {
    it('should verify matching hashes', () => {
      const data = { test: 'data' };
      const hash = generateHash(data);

      expect(verifyHash(data, hash)).toBe(true);
    });

    it('should reject non-matching hashes', () => {
      const data = { test: 'data' };
      const wrongHash = 'a'.repeat(64);

      expect(verifyHash(data, wrongHash)).toBe(false);
    });

    it('should reject modified data', () => {
      const originalData = { test: 'data', value: 1 };
      const hash = generateHash(originalData);

      const modifiedData = { test: 'data', value: 2 };

      expect(verifyHash(modifiedData, hash)).toBe(false);
    });
  });

  describe('generatePlanId', () => {
    it('should generate valid plan IDs', () => {
      const id = generatePlanId();

      expect(id).toMatch(/^aps-[a-f0-9]{8}$/);
      expect(isValidPlanId(id)).toBe(true);
    });

    it('should generate unique IDs', () => {
      const ids = new Set<string>();

      // Generate 100 IDs
      for (let i = 0; i < 100; i++) {
        ids.add(generatePlanId());
      }

      // All should be unique
      expect(ids.size).toBe(100);
    });

    it('should always use lowercase hex', () => {
      // Generate several IDs and check they're all lowercase
      for (let i = 0; i < 10; i++) {
        const id = generatePlanId();
        expect(id).toBe(id.toLowerCase());
      }
    });
  });

  describe('isValidPlanId', () => {
    it('should validate correct plan IDs', () => {
      expect(isValidPlanId('aps-12345678')).toBe(true);
      expect(isValidPlanId('aps-abcdef00')).toBe(true);
      expect(isValidPlanId('aps-00000000')).toBe(true);
      expect(isValidPlanId('aps-ffffffff')).toBe(true);
    });

    it('should reject invalid plan IDs', () => {
      expect(isValidPlanId('aps-1234567')).toBe(false); // Too short
      expect(isValidPlanId('aps-123456789')).toBe(false); // Too long
      expect(isValidPlanId('aps-1234567g')).toBe(false); // Invalid char
      expect(isValidPlanId('APS-12345678')).toBe(false); // Wrong case
      expect(isValidPlanId('12345678')).toBe(false); // Missing prefix
      expect(isValidPlanId('aps_12345678')).toBe(false); // Wrong separator
    });
  });

  describe('isValidHash', () => {
    it('should validate correct SHA-256 hashes', () => {
      const validHash = 'a'.repeat(64);
      expect(isValidHash(validHash)).toBe(true);

      const realHash = generateHash('test');
      expect(isValidHash(realHash)).toBe(true);
    });

    it('should reject invalid hashes', () => {
      expect(isValidHash('a'.repeat(63))).toBe(false); // Too short
      expect(isValidHash('a'.repeat(65))).toBe(false); // Too long
      expect(isValidHash('g'.repeat(64))).toBe(false); // Invalid char
      expect(isValidHash('A'.repeat(64))).toBe(false); // Wrong case
    });
  });

  describe('Hash stability for APS plan objects', () => {
    it('should generate consistent hashes for APS plan structures', () => {
      const plan = {
        id: 'aps-12345678',
        intent: 'Test plan for hashing',
        schema_version: '1.0.0',
        proposed_changes: [
          {
            type: 'create',
            path: '/test.ts',
            description: 'Create test file',
          },
        ],
        provenance: {
          timestamp: '2024-01-01T00:00:00.000Z',
          author: 'test-user',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: ['lint', 'test'],
          skip_checks: [],
        },
      };

      const hash1 = generateHash(plan);

      // Reorder properties
      const reorderedPlan = {
        validations: plan.validations,
        id: plan.id,
        proposed_changes: plan.proposed_changes,
        provenance: plan.provenance,
        intent: plan.intent,
        schema_version: plan.schema_version,
      };

      const hash2 = generateHash(reorderedPlan);

      expect(hash1).toBe(hash2);
    });
  });
});
