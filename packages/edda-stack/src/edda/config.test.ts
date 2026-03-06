import { describe, expect, it } from 'vitest';
import {
  DEFAULT_EDDA_CONFIG,
  EddaConfigSchema,
  parseEddaConfig,
  type EddaConfig,
} from './config.js';

describe('Edda configuration schema (EDDA-005)', () => {
  it('applies defaults for empty config', () => {
    const result = EddaConfigSchema.parse({});

    expect(result.edda.enabled).toBe(true);
    expect(result.edda.storage.type).toBe('git');
    expect(result.edda.storage.path).toBe('.anvil/edda/');
    expect(result.edda.storage.format).toBe('yaml');
    expect(result.edda.promotion.require_reason).toBe(true);
    expect(result.edda.promotion.require_attribution).toBe(true);
    expect(result.edda.promotion.min_ember_confidence).toBe(0.5);
    expect(result.edda.limits.max_statement_length).toBe(2000);
    expect(result.edda.limits.max_context_length).toBe(10000);
  });

  it('parses partial config and fills missing defaults', () => {
    const result = EddaConfigSchema.parse({
      edda: {
        enabled: false,
        storage: {
          path: '.cache/edda/',
        },
        promotion: {
          min_ember_confidence: 0.75,
        },
      },
    });

    expect(result.edda.enabled).toBe(false);
    expect(result.edda.storage.type).toBe('git');
    expect(result.edda.storage.path).toBe('.cache/edda/');
    expect(result.edda.storage.format).toBe('yaml');
    expect(result.edda.promotion.require_reason).toBe(true);
    expect(result.edda.promotion.require_attribution).toBe(true);
    expect(result.edda.promotion.min_ember_confidence).toBe(0.75);
    expect(result.edda.limits.max_statement_length).toBe(2000);
  });

  it('rejects invalid storage values', () => {
    expect(() =>
      EddaConfigSchema.parse({
        edda: {
          storage: {
            type: 'file',
          },
        },
      })
    ).toThrow();
  });

  it('rejects confidence outside the valid range', () => {
    expect(() =>
      EddaConfigSchema.parse({
        edda: {
          promotion: {
            min_ember_confidence: -0.1,
          },
        },
      })
    ).toThrow();
  });

  it('rejects invalid limits values', () => {
    expect(() =>
      EddaConfigSchema.parse({
        edda: {
          limits: {
            max_statement_length: 0,
          },
        },
      })
    ).toThrow();
  });
});

describe('parseEddaConfig', () => {
  it('returns parsed config for valid input', () => {
    const result = parseEddaConfig({
      edda: {
        promotion: {
          require_reason: false,
        },
      },
    });

    expect(result?.edda.promotion.require_reason).toBe(false);
  });

  it('returns null for invalid input', () => {
    const result = parseEddaConfig({
      edda: {
        limits: {
          max_context_length: -10,
        },
      },
    });

    expect(result).toBeNull();
  });

  it('returns null for non-object input', () => {
    expect(parseEddaConfig('edda')).toBeNull();
    expect(parseEddaConfig(42)).toBeNull();
    expect(parseEddaConfig(null)).toBeNull();
  });
});

describe('DEFAULT_EDDA_CONFIG', () => {
  it('matches parsed defaults', () => {
    const parsedDefaults = EddaConfigSchema.parse({});
    const defaultConfig: EddaConfig = DEFAULT_EDDA_CONFIG;

    expect(defaultConfig).toEqual(parsedDefaults);
  });
});
