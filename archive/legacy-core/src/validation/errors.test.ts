/**
 * Error Types and Formatting Tests
 */

import { describe, it, expect } from 'vitest';
import { z } from 'zod';
import {
  ValidationError,
  SchemaValidationError,
  HashValidationError,
  formatZodErrors,
  formatValidationErrors,
  createValidationSummary,
} from './errors.js';

describe('Error Types', () => {
  describe('ValidationError', () => {
    it('should create a validation error', () => {
      const error = new ValidationError('Test error', 'TEST_CODE', { foo: 'bar' });

      expect(error).toBeInstanceOf(Error);
      expect(error.message).toBe('Test error');
      expect(error.code).toBe('TEST_CODE');
      expect(error.details).toEqual({ foo: 'bar' });
      expect(error.name).toBe('ValidationError');
    });
  });

  describe('SchemaValidationError', () => {
    it('should create a schema validation error', () => {
      const issues: z.ZodIssue[] = [
        {
          code: 'invalid_type',
          expected: 'string',
          received: 'number',
          path: ['field'],
          message: 'Expected string',
        },
      ];

      const error = new SchemaValidationError('Schema failed', issues);

      expect(error).toBeInstanceOf(ValidationError);
      expect(error.code).toBe('SCHEMA_VALIDATION_FAILED');
      expect(error.issues).toEqual(issues);
    });
  });

  describe('HashValidationError', () => {
    it('should create a hash validation error', () => {
      const error = new HashValidationError('Hash mismatch', 'expected123', 'actual456');

      expect(error).toBeInstanceOf(ValidationError);
      expect(error.code).toBe('HASH_VALIDATION_FAILED');
      expect(error.expectedHash).toBe('expected123');
      expect(error.actualHash).toBe('actual456');
    });
  });
});

describe('Error Formatting', () => {
  describe('formatZodErrors', () => {
    it('should format invalid type errors', () => {
      const schema = z.object({ name: z.string() });
      const result = schema.safeParse({ name: 123 });

      if (!result.success) {
        const formatted = formatZodErrors(result.error.issues);

        expect(formatted).toHaveLength(1);
        expect(formatted[0].path).toBe('name');
        expect(formatted[0].message).toContain('expected string');
        expect(formatted[0].severity).toBe('error');
      }
    });

    it('should format string length errors', () => {
      const schema = z.object({
        short: z.string().min(5),
        long: z.string().max(3),
      });

      const result = schema.safeParse({ short: 'hi', long: 'hello' });

      if (!result.success) {
        const formatted = formatZodErrors(result.error.issues);

        expect(formatted).toHaveLength(2);
        expect(formatted[0].message).toContain('too small');
        expect(formatted[1].message).toContain('too large');
      }
    });

    it('should format regex validation errors', () => {
      const schema = z.object({
        id: z.string().regex(/^[a-z]+$/),
      });

      const result = schema.safeParse({ id: '123' });

      if (!result.success) {
        const formatted = formatZodErrors(result.error.issues);
        expect(formatted[0].message).toContain('must match pattern');
      }
    });

    it('should format unrecognized keys error', () => {
      const schema = z.object({ name: z.string() }).strict();
      const result = schema.safeParse({ name: 'test', extra: 'field' });

      if (!result.success) {
        const formatted = formatZodErrors(result.error.issues);
        expect(formatted[0].message).toContain('Unexpected properties');
        expect(formatted[0].message).toContain('extra');
      }
    });

    it('should handle root-level errors', () => {
      const schema = z.string();
      const result = schema.safeParse(123);

      if (!result.success) {
        const formatted = formatZodErrors(result.error.issues);
        expect(formatted[0].path).toBe('<root>');
      }
    });
  });

  describe('formatValidationErrors', () => {
    it('should format empty issues array', () => {
      const formatted = formatValidationErrors([]);
      expect(formatted).toBe('No validation errors');
    });

    it('should format single error', () => {
      const issues = [
        {
          path: 'field',
          message: 'Invalid value',
          code: 'INVALID',
          severity: 'error' as const,
        },
      ];

      const formatted = formatValidationErrors(issues);

      expect(formatted).toContain('Found 1 validation error:');
      expect(formatted).toContain('❌');
      expect(formatted).toContain('Invalid value');
      expect(formatted).toContain('Path: field');
      expect(formatted).toContain('Code: INVALID');
    });

    it('should format multiple errors', () => {
      const issues = [
        {
          path: 'field1',
          message: 'Error 1',
          code: 'ERR1',
          severity: 'error' as const,
        },
        {
          path: 'field2',
          message: 'Warning 1',
          code: 'WARN1',
          severity: 'warning' as const,
        },
      ];

      const formatted = formatValidationErrors(issues);

      expect(formatted).toContain('Found 2 validation errors:');
      expect(formatted).toContain('❌'); // Error emoji
      expect(formatted).toContain('⚠️'); // Warning emoji
    });

    it('should handle root-level errors', () => {
      const issues = [
        {
          path: '<root>',
          message: 'Root error',
          code: 'ROOT_ERR',
          severity: 'error' as const,
        },
      ];

      const formatted = formatValidationErrors(issues);

      expect(formatted).not.toContain('Path: <root>'); // Should skip path display for root
    });
  });

  describe('createValidationSummary', () => {
    it('should create success summary', () => {
      const summary = createValidationSummary(true);
      expect(summary).toBe('✅ Validation passed');
    });

    it('should create failure summary with errors', () => {
      const issues = [
        { path: 'f1', message: 'E1', code: 'E1', severity: 'error' as const },
        { path: 'f2', message: 'E2', code: 'E2', severity: 'error' as const },
      ];

      const summary = createValidationSummary(false, issues);
      expect(summary).toBe('❌ Validation failed - 2 errors');
    });

    it('should create failure summary with warnings', () => {
      const issues = [{ path: 'f1', message: 'W1', code: 'W1', severity: 'warning' as const }];

      const summary = createValidationSummary(false, issues);
      expect(summary).toBe('❌ Validation failed - 1 warning');
    });

    it('should create failure summary with mixed issues', () => {
      const issues = [
        { path: 'f1', message: 'E1', code: 'E1', severity: 'error' as const },
        { path: 'f2', message: 'W1', code: 'W1', severity: 'warning' as const },
        { path: 'f3', message: 'W2', code: 'W2', severity: 'warning' as const },
      ];

      const summary = createValidationSummary(false, issues);
      expect(summary).toBe('❌ Validation failed - 1 error - 2 warnings');
    });
  });
});
