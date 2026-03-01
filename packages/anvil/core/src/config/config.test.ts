/**
 * Tests for packages/anvil/core/src/config/ (TEST-007)
 *
 * Covers: ConfigLoader (get/set/has/getAll/getOrDefault),
 * constants (DEFAULT_ANALYSABLE_EXTENSIONS),
 * nudge-config (meetsNudgeThreshold, DEFAULT_NUDGE_CONFIG),
 * types (ConfigSource, ConfigEntry, ConfigLoaderOptions)
 */

import { describe, it, expect } from 'vitest';
import { ConfigLoader, createConfigLoader } from './loader.js';
import { DEFAULT_ANALYSABLE_EXTENSIONS } from './constants.js';
import { DEFAULT_NUDGE_CONFIG, meetsNudgeThreshold } from './nudge-config.js';

// ---------------------------------------------------------------------------
// ConfigLoader
// ---------------------------------------------------------------------------

describe('ConfigLoader', () => {
  describe('constructor defaults', () => {
    it('uses process.cwd() as default baseDir', () => {
      const loader = new ConfigLoader();
      expect(loader.options.baseDir).toBe(process.cwd());
    });

    it('uses ANVIL_ as default envPrefix', () => {
      const loader = new ConfigLoader();
      expect(loader.options.envPrefix).toBe('ANVIL_');
    });

    it('includes standard config filenames', () => {
      const loader = new ConfigLoader();
      expect(loader.options.fileNames).toContain('.anvilrc');
      expect(loader.options.fileNames).toContain('.anvilrc.json');
    });
  });

  describe('constructor with custom options', () => {
    it('accepts custom baseDir', () => {
      const loader = new ConfigLoader({ baseDir: '/my/project' });
      expect(loader.options.baseDir).toBe('/my/project');
    });

    it('accepts custom envPrefix', () => {
      const loader = new ConfigLoader({ envPrefix: 'MY_' });
      expect(loader.options.envPrefix).toBe('MY_');
    });

    it('fills in defaults for omitted fields', () => {
      const loader = new ConfigLoader({ baseDir: '/tmp' });
      expect(loader.options.envPrefix).toBe('ANVIL_');
      expect(loader.options.fileNames.length).toBeGreaterThan(0);
    });
  });

  describe('get / set / has', () => {
    it('returns undefined for unset key', () => {
      const loader = new ConfigLoader();
      expect(loader.get('missing')).toBeUndefined();
    });

    it('stores and retrieves values', () => {
      const loader = new ConfigLoader();
      loader.set('key', 'value');
      expect(loader.get('key')).toBe('value');
    });

    it('has returns false for missing key', () => {
      const loader = new ConfigLoader();
      expect(loader.has('nope')).toBe(false);
    });

    it('has returns true after set', () => {
      const loader = new ConfigLoader();
      loader.set('exists', true);
      expect(loader.has('exists')).toBe(true);
    });

    it('handles falsy values correctly', () => {
      const loader = new ConfigLoader();
      loader.set('zero', 0);
      loader.set('empty', '');
      loader.set('nul', null);
      loader.set('undef', undefined);

      expect(loader.get('zero')).toBe(0);
      expect(loader.get('empty')).toBe('');
      expect(loader.get('nul')).toBeNull();
      expect(loader.get('undef')).toBeUndefined();
    });
  });

  describe('getOrDefault', () => {
    it('returns stored value when present', () => {
      const loader = new ConfigLoader();
      loader.set('key', 'stored');
      expect(loader.getOrDefault('key', 'fallback')).toBe('stored');
    });

    it('returns default when key is missing', () => {
      const loader = new ConfigLoader();
      expect(loader.getOrDefault('missing', 42)).toBe(42);
    });

    it('returns default for nullish stored values', () => {
      const loader = new ConfigLoader();
      loader.set('nul', null);
      // null ?? default => default
      expect(loader.getOrDefault('nul', 'fallback')).toBe('fallback');
    });

    it('returns falsy stored value (0) instead of default', () => {
      const loader = new ConfigLoader();
      loader.set('zero', 0);
      // 0 ?? default => 0
      expect(loader.getOrDefault('zero', 99)).toBe(0);
    });
  });

  describe('getAll', () => {
    it('returns empty object when no config set', () => {
      const loader = new ConfigLoader();
      expect(loader.getAll()).toEqual({});
    });

    it('returns all set values', () => {
      const loader = new ConfigLoader();
      loader.set('a', 1);
      loader.set('b', 'two');

      expect(loader.getAll()).toEqual({ a: 1, b: 'two' });
    });
  });

  describe('createConfigLoader', () => {
    it('creates a ConfigLoader instance', () => {
      const loader = createConfigLoader();
      expect(loader).toBeInstanceOf(ConfigLoader);
    });

    it('passes options through', () => {
      const loader = createConfigLoader({ envPrefix: 'CUSTOM_' });
      expect(loader.options.envPrefix).toBe('CUSTOM_');
    });
  });
});

// ---------------------------------------------------------------------------
// constants
// ---------------------------------------------------------------------------

describe('DEFAULT_ANALYSABLE_EXTENSIONS', () => {
  it('is a non-empty array of strings', () => {
    expect(Array.isArray(DEFAULT_ANALYSABLE_EXTENSIONS)).toBe(true);
    expect(DEFAULT_ANALYSABLE_EXTENSIONS.length).toBeGreaterThan(0);
  });

  it('all entries start with a dot', () => {
    for (const ext of DEFAULT_ANALYSABLE_EXTENSIONS) {
      expect(ext).toMatch(/^\./);
    }
  });

  it('includes TypeScript extensions', () => {
    expect(DEFAULT_ANALYSABLE_EXTENSIONS).toContain('.ts');
    expect(DEFAULT_ANALYSABLE_EXTENSIONS).toContain('.tsx');
  });

  it('includes JavaScript extensions', () => {
    expect(DEFAULT_ANALYSABLE_EXTENSIONS).toContain('.js');
    expect(DEFAULT_ANALYSABLE_EXTENSIONS).toContain('.jsx');
  });

  it('includes CSS extensions', () => {
    expect(DEFAULT_ANALYSABLE_EXTENSIONS).toContain('.css');
  });
});

// ---------------------------------------------------------------------------
// nudge-config
// ---------------------------------------------------------------------------

describe('nudge-config', () => {
  describe('DEFAULT_NUDGE_CONFIG', () => {
    it('is enabled by default', () => {
      expect(DEFAULT_NUDGE_CONFIG.enabled).toBe(true);
    });

    it('has interactive off by default', () => {
      expect(DEFAULT_NUDGE_CONFIG.interactive).toBe(false);
    });

    it('uses warning as default severity threshold', () => {
      expect(DEFAULT_NUDGE_CONFIG.severityThreshold).toBe('warning');
    });

    it('has expected shape', () => {
      expect(DEFAULT_NUDGE_CONFIG).toHaveProperty('enabled');
      expect(DEFAULT_NUDGE_CONFIG).toHaveProperty('interactive');
      expect(DEFAULT_NUDGE_CONFIG).toHaveProperty('severityThreshold');
    });
  });

  describe('meetsNudgeThreshold', () => {
    it('error meets all thresholds', () => {
      expect(meetsNudgeThreshold('error', 'info')).toBe(true);
      expect(meetsNudgeThreshold('error', 'warning')).toBe(true);
      expect(meetsNudgeThreshold('error', 'error')).toBe(true);
    });

    it('warning meets info and warning thresholds', () => {
      expect(meetsNudgeThreshold('warning', 'info')).toBe(true);
      expect(meetsNudgeThreshold('warning', 'warning')).toBe(true);
    });

    it('warning does not meet error threshold', () => {
      expect(meetsNudgeThreshold('warning', 'error')).toBe(false);
    });

    it('info only meets info threshold', () => {
      expect(meetsNudgeThreshold('info', 'info')).toBe(true);
      expect(meetsNudgeThreshold('info', 'warning')).toBe(false);
      expect(meetsNudgeThreshold('info', 'error')).toBe(false);
    });

    it('unknown severity never meets any threshold', () => {
      expect(meetsNudgeThreshold('unknown', 'info')).toBe(false);
    });
  });
});
