import { describe, it, expect } from 'vitest';
import { InvalidArgumentError } from 'commander';
import { parseBoundedInt } from '../parsers.js';

describe('parseBoundedInt', () => {
  const parse = parseBoundedInt('--limit', 1, 200);

  it('returns the parsed integer inside bounds', () => {
    expect(parse('1')).toBe(1);
    expect(parse('50')).toBe(50);
    expect(parse('200')).toBe(200);
  });

  it('rejects non-integer strings', () => {
    expect(() => parse('abc')).toThrow(InvalidArgumentError);
    expect(() => parse('')).toThrow(InvalidArgumentError);
    expect(() => parse('1.5')).toThrow(InvalidArgumentError);
    expect(() => parse('NaN')).toThrow(InvalidArgumentError);
  });

  it('rejects values outside the bounds', () => {
    expect(() => parse('0')).toThrow(/between 1 and 200/);
    expect(() => parse('201')).toThrow(/between 1 and 200/);
    expect(() => parse('-5')).toThrow(/between 1 and 200/);
  });

  it('supports lower bound of 0 for offsets', () => {
    const offset = parseBoundedInt('--offset', 0, Number.MAX_SAFE_INTEGER);
    expect(offset('0')).toBe(0);
    expect(() => offset('-1')).toThrow(InvalidArgumentError);
  });
});
