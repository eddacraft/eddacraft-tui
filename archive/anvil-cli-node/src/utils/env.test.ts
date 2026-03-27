import { describe, it, expect, afterEach } from 'vitest';
import { requireEnv, getEnv, isEnvTrue } from './env.js';

describe('env utilities', () => {
  const originalEnv = { ...process.env };

  afterEach(() => {
    process.env = { ...originalEnv };
  });

  describe('requireEnv', () => {
    it('should return the value when the env var is set', () => {
      process.env.TEST_VAR = 'hello';

      expect(requireEnv('TEST_VAR')).toBe('hello');
    });

    it('should throw when the env var is not set', () => {
      delete process.env.TEST_VAR;

      expect(() => requireEnv('TEST_VAR')).toThrow('TEST_VAR environment variable is required');
    });

    it('should throw when the env var is empty string', () => {
      process.env.TEST_VAR = '';

      expect(() => requireEnv('TEST_VAR')).toThrow('TEST_VAR environment variable is required');
    });

    it('should include context in error message when provided', () => {
      delete process.env.TEST_VAR;

      expect(() => requireEnv('TEST_VAR', 'admin commands')).toThrow(
        'TEST_VAR environment variable is required for admin commands'
      );
    });
  });

  describe('getEnv', () => {
    it('should return the env var value when set', () => {
      process.env.TEST_VAR = 'custom-value';

      expect(getEnv('TEST_VAR', 'default')).toBe('custom-value');
    });

    it('should return the default when env var is not set', () => {
      delete process.env.TEST_VAR;

      expect(getEnv('TEST_VAR', 'fallback')).toBe('fallback');
    });

    it('should return empty string if env var is set to empty', () => {
      process.env.TEST_VAR = '';

      // Empty string is still a valid value (not undefined/null)
      expect(getEnv('TEST_VAR', 'fallback')).toBe('');
    });
  });

  describe('isEnvTrue', () => {
    it('should return true for "1"', () => {
      process.env.TEST_FLAG = '1';

      expect(isEnvTrue('TEST_FLAG')).toBe(true);
    });

    it('should return true for "true"', () => {
      process.env.TEST_FLAG = 'true';

      expect(isEnvTrue('TEST_FLAG')).toBe(true);
    });

    it('should return true for "TRUE" (case-insensitive)', () => {
      process.env.TEST_FLAG = 'TRUE';

      expect(isEnvTrue('TEST_FLAG')).toBe(true);
    });

    it('should return true for "True" (case-insensitive)', () => {
      process.env.TEST_FLAG = 'True';

      expect(isEnvTrue('TEST_FLAG')).toBe(true);
    });

    it('should return false for "0"', () => {
      process.env.TEST_FLAG = '0';

      expect(isEnvTrue('TEST_FLAG')).toBe(false);
    });

    it('should return false for "false"', () => {
      process.env.TEST_FLAG = 'false';

      expect(isEnvTrue('TEST_FLAG')).toBe(false);
    });

    it('should return false for arbitrary strings', () => {
      process.env.TEST_FLAG = 'yes';

      expect(isEnvTrue('TEST_FLAG')).toBe(false);
    });

    it('should return false when env var is not set', () => {
      delete process.env.TEST_FLAG;

      expect(isEnvTrue('TEST_FLAG')).toBe(false);
    });

    it('should return false for empty string', () => {
      process.env.TEST_FLAG = '';

      expect(isEnvTrue('TEST_FLAG')).toBe(false);
    });
  });
});
