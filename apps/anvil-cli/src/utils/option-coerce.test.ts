import { describe, it, expect } from 'vitest';
import { InvalidArgumentError } from 'commander';
import { coercePositiveInt, coerceNonNegativeInt, coercePort } from './option-coerce.js';

describe('option coercion utilities', () => {
  describe('coercePositiveInt', () => {
    it('accepts positive integer values', () => {
      expect(coercePositiveInt('42', '--limit')).toBe(42);
    });

    it('accepts the lower boundary', () => {
      expect(coercePositiveInt('1', '--limit')).toBe(1);
    });

    it('rejects zero', () => {
      expect(() => coercePositiveInt('0', '--limit')).toThrow(InvalidArgumentError);
      expect(() => coercePositiveInt('0', '--limit')).toThrow('--limit must be a positive integer');
    });

    it('rejects negative values', () => {
      expect(() => coercePositiveInt('-5', '--limit')).toThrow(InvalidArgumentError);
    });

    it('rejects non-numeric values', () => {
      expect(() => coercePositiveInt('abc', '--limit')).toThrow(InvalidArgumentError);
    });

    it('rejects partial integer parses like 12abc', () => {
      expect(() => coercePositiveInt('12abc', '--limit')).toThrow(InvalidArgumentError);
    });

    it('rejects float values', () => {
      expect(() => coercePositiveInt('1.9', '--limit')).toThrow(InvalidArgumentError);
    });
  });

  describe('coerceNonNegativeInt', () => {
    it('accepts zero', () => {
      expect(coerceNonNegativeInt('0', '--refresh')).toBe(0);
    });

    it('accepts positive integer values', () => {
      expect(coerceNonNegativeInt('300000', '--refresh')).toBe(300000);
    });

    it('rejects negative values', () => {
      expect(() => coerceNonNegativeInt('-1', '--refresh')).toThrow(InvalidArgumentError);
      expect(() => coerceNonNegativeInt('-1', '--refresh')).toThrow(
        '--refresh must be a non-negative integer'
      );
    });

    it('rejects non-numeric values', () => {
      expect(() => coerceNonNegativeInt('not-a-number', '--refresh')).toThrow(InvalidArgumentError);
    });
  });

  describe('coercePort', () => {
    it('accepts lower and upper boundaries', () => {
      expect(coercePort('1', '--port')).toBe(1);
      expect(coercePort('65535', '--port')).toBe(65535);
    });

    it('accepts a valid midpoint port', () => {
      expect(coercePort('3000', '--port')).toBe(3000);
    });

    it('rejects zero and negative values', () => {
      expect(() => coercePort('0', '--port')).toThrow(InvalidArgumentError);
      expect(() => coercePort('-1', '--port')).toThrow(InvalidArgumentError);
    });

    it('rejects values above the upper boundary', () => {
      expect(() => coercePort('65536', '--port')).toThrow(InvalidArgumentError);
    });

    it('rejects non-numeric values', () => {
      try {
        coercePort('NaN', '--port');
        throw new Error('Expected coercePort to throw');
      } catch (err) {
        expect(err).toBeInstanceOf(InvalidArgumentError);
        expect((err as Error).message).toBe('--port must be an integer between 1 and 65535');
      }
    });
  });
});
