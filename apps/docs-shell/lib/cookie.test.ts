// apps/docs-shell/lib/cookie.test.ts
import { describe, it, expect } from 'vitest';
import { getCookie } from './cookie';

describe('getCookie', () => {
  it('returns undefined for null header', () => {
    expect(getCookie(null, 'session')).toBeUndefined();
  });

  it('returns undefined when cookie absent', () => {
    expect(getCookie('other=1; foo=bar', 'session')).toBeUndefined();
  });

  it('extracts a single cookie value', () => {
    expect(getCookie('session=abc123', 'session')).toBe('abc123');
  });

  it('extracts a cookie from a multi-cookie header', () => {
    expect(getCookie('a=1; session=abc123; b=2', 'session')).toBe('abc123');
  });

  it('handles leading whitespace', () => {
    expect(getCookie('a=1;   session=abc123', 'session')).toBe('abc123');
  });

  it('url-decodes the value', () => {
    expect(getCookie('session=%2Fanvil%2Foverview', 'session')).toBe('/anvil/overview');
  });

  it('returns undefined when decoding fails', () => {
    expect(getCookie('session=%ZZ', 'session')).toBeUndefined();
  });
});
