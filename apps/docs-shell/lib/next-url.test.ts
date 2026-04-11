import { describe, it, expect } from 'vitest';
import { validateNext } from './next-url';

describe('validateNext', () => {
  it('returns default for null', () => {
    expect(validateNext(null)).toBe('/anvil/overview');
  });

  it('returns default for empty string', () => {
    expect(validateNext('')).toBe('/anvil/overview');
  });

  it('accepts /anvil/overview', () => {
    expect(validateNext('/anvil/overview')).toBe('/anvil/overview');
  });

  it('accepts deep /anvil paths', () => {
    expect(validateNext('/anvil/quickstart/setup')).toBe('/anvil/quickstart/setup');
  });

  it('rejects /kindling path (non-anvil)', () => {
    expect(validateNext('/kindling/overview')).toBe('/anvil/overview');
  });

  it('rejects protocol-relative URLs', () => {
    expect(validateNext('//evil.com/anvil/foo')).toBe('/anvil/overview');
  });

  it('rejects absolute URLs with protocol', () => {
    expect(validateNext('https://evil.com/anvil/foo')).toBe('/anvil/overview');
  });

  it('strips dot-segments and revalidates', () => {
    expect(validateNext('/anvil/../kindling')).toBe('/anvil/overview');
  });

  it('accepts /anvil with trailing segment', () => {
    expect(validateNext('/anvil/')).toBe('/anvil/');
  });

  it('rejects /anvil prefix near-miss like /anvilicious', () => {
    expect(validateNext('/anvilicious')).toBe('/anvil/overview');
  });

  it('accepts bare /anvil', () => {
    expect(validateNext('/anvil')).toBe('/anvil');
  });
});
