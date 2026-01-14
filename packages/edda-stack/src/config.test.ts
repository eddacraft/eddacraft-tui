/**
 * Stack Configuration Tests (STACK-012)
 *
 * Tests for the stack configuration schema and utilities.
 */

import { describe, it, expect } from 'vitest';
import {
  StackConfigSchema,
  StackLayerConfigSchema,
  StackValidationConfigSchema,
  parseStackConfig,
  isLayerEnabled,
  getEnabledLayerCount,
  getEnabledLayers,
  DEFAULT_STACK_CONFIG,
  type StackConfig,
} from './config.js';

describe('Stack Configuration Schema', () => {
  describe('StackLayerConfigSchema', () => {
    it('should parse valid layer config', () => {
      const config = { enabled: true };
      const result = StackLayerConfigSchema.parse(config);

      expect(result.enabled).toBe(true);
    });

    it('should default enabled to true', () => {
      const config = {};
      const result = StackLayerConfigSchema.parse(config);

      expect(result.enabled).toBe(true);
    });

    it('should allow extra properties (passthrough)', () => {
      const config = { enabled: true, customSetting: 'value' };
      const result = StackLayerConfigSchema.parse(config);

      expect(result.customSetting).toBe('value');
    });

    it('should reject non-boolean enabled', () => {
      const config = { enabled: 'yes' };

      expect(() => StackLayerConfigSchema.parse(config)).toThrow();
    });
  });

  describe('StackValidationConfigSchema', () => {
    it('should parse valid validation config', () => {
      const config = {
        check_provenance_integrity: true,
        check_schema_compatibility: false,
      };
      const result = StackValidationConfigSchema.parse(config);

      expect(result.check_provenance_integrity).toBe(true);
      expect(result.check_schema_compatibility).toBe(false);
    });

    it('should default to true for both checks', () => {
      const config = {};
      const result = StackValidationConfigSchema.parse(config);

      expect(result.check_provenance_integrity).toBe(true);
      expect(result.check_schema_compatibility).toBe(true);
    });
  });

  describe('StackConfigSchema', () => {
    it('should parse empty config', () => {
      const config = {};
      const result = StackConfigSchema.parse(config);

      expect(result).toBeDefined();
    });

    it('should parse full config', () => {
      const config = {
        kindling: { enabled: true },
        ember: { enabled: true },
        edda: { enabled: false },
        validation: {
          check_provenance_integrity: true,
          check_schema_compatibility: true,
        },
      };
      const result = StackConfigSchema.parse(config);

      expect(result.kindling?.enabled).toBe(true);
      expect(result.ember?.enabled).toBe(true);
      expect(result.edda?.enabled).toBe(false);
      expect(result.validation?.check_provenance_integrity).toBe(true);
    });

    it('should parse partial config', () => {
      const config = {
        kindling: { enabled: true },
      };
      const result = StackConfigSchema.parse(config);

      expect(result.kindling?.enabled).toBe(true);
      expect(result.ember).toBeUndefined();
      expect(result.edda).toBeUndefined();
    });
  });
});

describe('parseStackConfig', () => {
  it('should return parsed config for valid input', () => {
    const config = { kindling: { enabled: true } };
    const result = parseStackConfig(config);

    expect(result).not.toBeNull();
    expect(result?.kindling?.enabled).toBe(true);
  });

  it('should return null for invalid input', () => {
    const config = { kindling: { enabled: 'invalid' } };
    const result = parseStackConfig(config);

    expect(result).toBeNull();
  });

  it('should return null for non-object input', () => {
    expect(parseStackConfig('string')).toBeNull();
    expect(parseStackConfig(123)).toBeNull();
    expect(parseStackConfig(null)).toBeNull();
  });
});

describe('isLayerEnabled', () => {
  it('should return true for enabled layer', () => {
    const config: StackConfig = {
      kindling: { enabled: true },
      ember: { enabled: false },
    };

    expect(isLayerEnabled(config, 'kindling')).toBe(true);
    expect(isLayerEnabled(config, 'ember')).toBe(false);
  });

  it('should return false for undefined config', () => {
    expect(isLayerEnabled(undefined, 'kindling')).toBe(false);
  });

  it('should return false for undefined layer', () => {
    const config: StackConfig = {
      kindling: { enabled: true },
    };

    expect(isLayerEnabled(config, 'edda')).toBe(false);
  });
});

describe('getEnabledLayerCount', () => {
  it('should count enabled layers correctly', () => {
    const config: StackConfig = {
      kindling: { enabled: true },
      ember: { enabled: true },
      edda: { enabled: false },
    };

    expect(getEnabledLayerCount(config)).toBe(2);
  });

  it('should return 0 for undefined config', () => {
    expect(getEnabledLayerCount(undefined)).toBe(0);
  });

  it('should return 0 for all disabled', () => {
    const config: StackConfig = {
      kindling: { enabled: false },
      ember: { enabled: false },
      edda: { enabled: false },
    };

    expect(getEnabledLayerCount(config)).toBe(0);
  });

  it('should return 3 for all enabled', () => {
    const config: StackConfig = {
      kindling: { enabled: true },
      ember: { enabled: true },
      edda: { enabled: true },
    };

    expect(getEnabledLayerCount(config)).toBe(3);
  });
});

describe('getEnabledLayers', () => {
  it('should return enabled layer names', () => {
    const config: StackConfig = {
      kindling: { enabled: true },
      ember: { enabled: false },
      edda: { enabled: true },
    };

    const layers = getEnabledLayers(config);

    expect(layers).toContain('kindling');
    expect(layers).toContain('edda');
    expect(layers).not.toContain('ember');
  });

  it('should return empty array for undefined config', () => {
    expect(getEnabledLayers(undefined)).toEqual([]);
  });

  it('should return layers in order', () => {
    const config: StackConfig = {
      kindling: { enabled: true },
      ember: { enabled: true },
      edda: { enabled: true },
    };

    const layers = getEnabledLayers(config);

    expect(layers).toEqual(['kindling', 'ember', 'edda']);
  });
});

describe('DEFAULT_STACK_CONFIG', () => {
  it('should have all layers disabled by default', () => {
    expect(DEFAULT_STACK_CONFIG.kindling?.enabled).toBe(false);
    expect(DEFAULT_STACK_CONFIG.ember?.enabled).toBe(false);
    expect(DEFAULT_STACK_CONFIG.edda?.enabled).toBe(false);
  });

  it('should have validation checks enabled by default', () => {
    expect(DEFAULT_STACK_CONFIG.validation?.check_provenance_integrity).toBe(true);
    expect(DEFAULT_STACK_CONFIG.validation?.check_schema_compatibility).toBe(true);
  });
});
