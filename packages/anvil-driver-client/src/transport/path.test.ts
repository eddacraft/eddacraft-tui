/**
 * Path resolver unit tests. Pure-function tests that exercise both
 * platform branches deterministically by passing `platform` and
 * `env` overrides.
 */

import path from 'node:path';

import { describe, expect, it } from 'vitest';

import { PathResolutionError, resolveDefaultSocketPath } from './path.js';

describe('resolveDefaultSocketPath', () => {
  it('uses XDG_RUNTIME_DIR when set on Linux', () => {
    const out = resolveDefaultSocketPath({
      platform: 'linux',
      env: { XDG_RUNTIME_DIR: '/run/user/1000', HOME: '/home/u' },
    });
    expect(out).toEqual({
      kind: 'unix',
      socketPath: path.join('/run/user/1000', 'anvil', 'intercept.sock'),
    });
  });

  it('falls back to $HOME/.local/state/anvil on Linux', () => {
    const out = resolveDefaultSocketPath({
      platform: 'linux',
      env: { HOME: '/home/u' },
    });
    expect(out).toEqual({
      kind: 'unix',
      socketPath: path.join('/home/u', '.local', 'state', 'anvil', 'intercept.sock'),
    });
  });

  it('uses HOME on macOS (no XDG)', () => {
    const out = resolveDefaultSocketPath({
      platform: 'darwin',
      env: { HOME: '/Users/u' },
    });
    expect(out).toEqual({
      kind: 'unix',
      socketPath: path.join('/Users/u', '.local', 'state', 'anvil', 'intercept.sock'),
    });
  });

  it('throws no-socket-dir when both XDG and HOME are unset', () => {
    expect(() => resolveDefaultSocketPath({ platform: 'linux', env: {} })).toThrowError(
      PathResolutionError
    );
  });

  it('throws no-pipe-name on Windows without an explicit override', () => {
    let err: unknown;
    try {
      resolveDefaultSocketPath({ platform: 'win32', env: {} });
    } catch (e) {
      err = e;
    }
    expect(err).toBeInstanceOf(PathResolutionError);
    expect((err as PathResolutionError).code).toBe('no-pipe-name');
  });

  it('honours an explicit pipe-name override on Windows', () => {
    const out = resolveDefaultSocketPath({
      platform: 'win32',
      env: {},
      pipeName: '\\\\.\\pipe\\anvil-intercept-S-1-5-21',
    });
    expect(out).toEqual({
      kind: 'windows',
      pipeName: '\\\\.\\pipe\\anvil-intercept-S-1-5-21',
    });
  });

  it('honours an explicit socket-path override on Linux', () => {
    const out = resolveDefaultSocketPath({
      platform: 'linux',
      env: {},
      socketPath: '/var/run/test/anvil.sock',
    });
    expect(out).toEqual({ kind: 'unix', socketPath: '/var/run/test/anvil.sock' });
  });

  it('rejects unsupported platforms', () => {
    let err: unknown;
    try {
      resolveDefaultSocketPath({ platform: 'haiku' as NodeJS.Platform, env: { HOME: '/h' } });
    } catch (e) {
      err = e;
    }
    expect(err).toBeInstanceOf(PathResolutionError);
    expect((err as PathResolutionError).code).toBe('unsupported-platform');
  });
});
