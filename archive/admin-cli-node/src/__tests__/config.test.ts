import { describe, it, expect } from 'vitest';
import { DEFAULT_URL, MissingConfigError, resolveConfig } from '../config.js';

describe('resolveConfig', () => {
  it('uses flag values when provided', () => {
    const cfg = resolveConfig(
      { key: 'k1', url: 'https://flag.example', actor: 'flag@example.com' },
      { env: {}, getGitEmail: () => 'git@example.com', getOsUser: () => 'osuser' }
    );
    expect(cfg).toEqual({ url: 'https://flag.example', key: 'k1', actor: 'flag@example.com' });
  });

  it('falls back to env when flags are absent', () => {
    const cfg = resolveConfig(
      {},
      {
        env: {
          ANVIL_ADMIN_KEY: 'env-key',
          ANVIL_ADMIN_URL: 'https://env.example',
          ANVIL_ADMIN_ACTOR: 'env@example.com',
        },
        getGitEmail: () => undefined,
        getOsUser: () => 'osuser',
      }
    );
    expect(cfg).toEqual({
      url: 'https://env.example',
      key: 'env-key',
      actor: 'env@example.com',
    });
  });

  it('defaults url when neither flag nor env provides it', () => {
    const cfg = resolveConfig(
      {},
      { env: { ANVIL_ADMIN_KEY: 'k' }, getGitEmail: () => undefined, getOsUser: () => 'u' }
    );
    expect(cfg.url).toBe(DEFAULT_URL);
  });

  it('resolves actor via git email when env/flag missing', () => {
    const cfg = resolveConfig(
      {},
      {
        env: { ANVIL_ADMIN_KEY: 'k' },
        getGitEmail: () => 'git@example.com',
        getOsUser: () => 'osuser',
      }
    );
    expect(cfg.actor).toBe('git@example.com');
  });

  it('falls back to os user when git email missing', () => {
    const cfg = resolveConfig(
      {},
      { env: { ANVIL_ADMIN_KEY: 'k' }, getGitEmail: () => undefined, getOsUser: () => 'osuser' }
    );
    expect(cfg.actor).toBe('osuser');
  });

  it('prefers flag over env for every field', () => {
    const cfg = resolveConfig(
      { key: 'flag-k', url: 'https://flag', actor: 'flag-actor' },
      {
        env: {
          ANVIL_ADMIN_KEY: 'env-k',
          ANVIL_ADMIN_URL: 'https://env',
          ANVIL_ADMIN_ACTOR: 'env-actor',
        },
        getGitEmail: () => 'git',
        getOsUser: () => 'os',
      }
    );
    expect(cfg).toEqual({ url: 'https://flag', key: 'flag-k', actor: 'flag-actor' });
  });

  it('throws MissingConfigError with exit code 5 when key is absent', () => {
    expect(() =>
      resolveConfig({}, { env: {}, getGitEmail: () => undefined, getOsUser: () => 'u' })
    ).toThrow(MissingConfigError);
    try {
      resolveConfig({}, { env: {}, getGitEmail: () => undefined, getOsUser: () => 'u' });
    } catch (err) {
      expect(err).toBeInstanceOf(MissingConfigError);
      expect((err as MissingConfigError).exitCode).toBe(5);
    }
  });
});
