/**
 * Windows pipe-name validation tests. Pure: doesn't open any pipe.
 */

import { describe, expect, it } from 'vitest';

import { DriverClientError } from '../errors.js';
import { validateWindowsPipeName } from './windows.js';

describe('validateWindowsPipeName', () => {
  it('accepts the canonical SID-suffixed pipe name', () => {
    expect(() =>
      validateWindowsPipeName('\\\\.\\pipe\\anvil-intercept-S-1-5-21-1234')
    ).not.toThrow();
  });

  it('refuses a pipe name without the daemon prefix', () => {
    let err: unknown;
    try {
      validateWindowsPipeName('\\\\.\\pipe\\rogue-server');
    } catch (e) {
      err = e;
    }
    expect(err).toBeInstanceOf(DriverClientError);
    expect((err as DriverClientError).code).toBe('anvil-daemon-wrong-owner');
  });

  it('refuses an empty SID suffix', () => {
    expect(() => validateWindowsPipeName('\\\\.\\pipe\\anvil-intercept-')).toThrowError(
      DriverClientError
    );
  });

  it('refuses a SID suffix containing whitespace or path separators', () => {
    expect(() => validateWindowsPipeName('\\\\.\\pipe\\anvil-intercept-foo bar')).toThrowError(
      DriverClientError
    );
    expect(() => validateWindowsPipeName('\\\\.\\pipe\\anvil-intercept-foo\\bar')).toThrowError(
      DriverClientError
    );
    expect(() => validateWindowsPipeName('\\\\.\\pipe\\anvil-intercept-foo/bar')).toThrowError(
      DriverClientError
    );
  });
});
