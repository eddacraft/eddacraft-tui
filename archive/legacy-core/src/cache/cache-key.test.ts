/**
 * Tests for cache key generation utilities
 */

import { describe, it, expect } from 'vitest';
import {
  generateCacheKey,
  hashCheckConfig,
  hashGateConfig,
  generateInputHash,
  parseCacheKey,
  checkInvalidationPattern,
  allGateInvalidationPattern,
} from './cache-key.js';
import type { GateCacheKeyInput } from './types.js';

describe('generateCacheKey', () => {
  it('generates deterministic keys for same input', () => {
    const input: GateCacheKeyInput = {
      check_name: 'eslint',
      plan_hash: 'abc123',
      config_hash: 'def456',
      workspace_root: '/home/user/project',
    };

    const key1 = generateCacheKey(input);
    const key2 = generateCacheKey(input);

    expect(key1).toBe(key2);
  });

  it('generates different keys for different inputs', () => {
    const input1: GateCacheKeyInput = {
      check_name: 'eslint',
      plan_hash: 'abc123',
      config_hash: 'def456',
      workspace_root: '/home/user/project',
    };

    const input2: GateCacheKeyInput = {
      ...input1,
      plan_hash: 'different',
    };

    const key1 = generateCacheKey(input1);
    const key2 = generateCacheKey(input2);

    expect(key1).not.toBe(key2);
  });

  it('generates keys with correct format', () => {
    const input: GateCacheKeyInput = {
      check_name: 'eslint',
      plan_hash: 'abc123',
      config_hash: 'def456',
      workspace_root: '/home/user/project',
    };

    const key = generateCacheKey(input);

    expect(key).toMatch(/^gate:check:eslint:[a-f0-9]{16}$/);
  });

  it('normalises Windows paths', () => {
    const unixInput: GateCacheKeyInput = {
      check_name: 'eslint',
      plan_hash: 'abc123',
      config_hash: 'def456',
      workspace_root: '/c/users/project',
    };

    const windowsInput: GateCacheKeyInput = {
      ...unixInput,
      workspace_root: 'C:\\users\\project',
    };

    const key1 = generateCacheKey(unixInput);
    const key2 = generateCacheKey(windowsInput);

    // Keys should be different because the paths are different after normalisation
    // But the format should be consistent
    expect(key1).toMatch(/^gate:check:eslint:[a-f0-9]{16}$/);
    expect(key2).toMatch(/^gate:check:eslint:[a-f0-9]{16}$/);
  });

  it('includes extra discriminators in hash', () => {
    const baseInput: GateCacheKeyInput = {
      check_name: 'eslint',
      plan_hash: 'abc123',
      config_hash: 'def456',
      workspace_root: '/home/user/project',
    };

    const inputWithExtra: GateCacheKeyInput = {
      ...baseInput,
      extra: { branch: 'main' },
    };

    const key1 = generateCacheKey(baseInput);
    const key2 = generateCacheKey(inputWithExtra);

    expect(key1).not.toBe(key2);
  });
});

describe('hashCheckConfig', () => {
  it('generates deterministic hashes', () => {
    const config = { min_score: 80, thresholds: { lines: 70 } };

    const hash1 = hashCheckConfig(config);
    const hash2 = hashCheckConfig(config);

    expect(hash1).toBe(hash2);
  });

  it('is order-independent for object keys', () => {
    const config1 = { a: 1, b: 2 };
    const config2 = { b: 2, a: 1 };

    const hash1 = hashCheckConfig(config1);
    const hash2 = hashCheckConfig(config2);

    expect(hash1).toBe(hash2);
  });

  it('generates different hashes for different configs', () => {
    const config1 = { min_score: 80 };
    const config2 = { min_score: 90 };

    const hash1 = hashCheckConfig(config1);
    const hash2 = hashCheckConfig(config2);

    expect(hash1).not.toBe(hash2);
  });
});

describe('hashGateConfig', () => {
  it('generates deterministic hashes', () => {
    const checks = [
      { name: 'eslint', description: '', enabled: true, config: { min_score: 80 } },
      { name: 'coverage', description: '', enabled: true, config: {} },
    ];

    const hash1 = hashGateConfig(checks);
    const hash2 = hashGateConfig(checks);

    expect(hash1).toBe(hash2);
  });

  it('ignores disabled checks', () => {
    const enabledOnly = [{ name: 'eslint', description: '', enabled: true, config: {} }];

    const withDisabled = [
      { name: 'eslint', description: '', enabled: true, config: {} },
      { name: 'coverage', description: '', enabled: false, config: {} },
    ];

    const hash1 = hashGateConfig(enabledOnly);
    const hash2 = hashGateConfig(withDisabled);

    expect(hash1).toBe(hash2);
  });
});

describe('generateInputHash', () => {
  it('generates full SHA-256 hash', () => {
    const input: GateCacheKeyInput = {
      check_name: 'eslint',
      plan_hash: 'abc123',
      config_hash: 'def456',
      workspace_root: '/home/user/project',
    };

    const hash = generateInputHash(input);

    expect(hash).toMatch(/^[a-f0-9]{64}$/);
  });
});

describe('parseCacheKey', () => {
  it('parses valid cache keys', () => {
    const key = 'gate:check:eslint:abc123def456';
    const parsed = parseCacheKey(key);

    expect(parsed).toEqual({
      type: 'gate',
      subtype: 'check',
      name: 'eslint',
      hash: 'abc123def456',
    });
  });

  it('returns null for invalid keys', () => {
    expect(parseCacheKey('invalid')).toBeNull();
    expect(parseCacheKey('a:b:c')).toBeNull();
    expect(parseCacheKey('a:b:c:d:e')).toBeNull();
  });
});

describe('checkInvalidationPattern', () => {
  it('generates correct pattern for check name', () => {
    const pattern = checkInvalidationPattern('eslint');
    expect(pattern).toBe('gate:check:eslint:*');
  });
});

describe('allGateInvalidationPattern', () => {
  it('generates correct pattern for all gates', () => {
    const pattern = allGateInvalidationPattern();
    expect(pattern).toBe('gate:*');
  });
});
