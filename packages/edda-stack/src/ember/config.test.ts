import { describe, expect, it } from 'vitest';
import {
  DEFAULT_EMBER_CONFIG,
  EmberConfigSchema,
  parseEmberConfig,
  type EmberConfig,
} from './config.js';

describe('Ember configuration schema (EMBER-003)', () => {
  it('applies defaults for empty config', () => {
    const result = EmberConfigSchema.parse({});

    expect(result.ember.enabled).toBe(true);
    expect(result.ember.database).toBe('.anvil/ember.db');
    expect(result.ember.decay.default_ttl_days).toBe(30);
    expect(result.ember.decay.min_ttl_days).toBe(7);
    expect(result.ember.decay.max_ttl_days).toBe(90);
    expect(result.ember.evaluation.min_confidence).toBe(0.3);
    expect(result.ember.evaluation.repetition_threshold).toBe(3);
    expect(result.ember.evaluation.escalation_window_hours).toBe(24);
    expect(result.ember.limits.max_candidates).toBe(1000);
    expect(result.ember.limits.max_proposal_size_kb).toBe(64);
  });

  it('parses partial config and fills missing defaults', () => {
    const result = EmberConfigSchema.parse({
      ember: {
        database: 'tmp/ember.db',
        evaluation: {
          min_confidence: 0.55,
        },
      },
    });

    expect(result.ember.enabled).toBe(true);
    expect(result.ember.database).toBe('tmp/ember.db');
    expect(result.ember.evaluation.min_confidence).toBe(0.55);
    expect(result.ember.evaluation.repetition_threshold).toBe(3);
    expect(result.ember.decay.default_ttl_days).toBe(30);
  });

  it('rejects invalid decay values', () => {
    expect(() =>
      EmberConfigSchema.parse({
        ember: {
          decay: {
            default_ttl_days: -1,
          },
        },
      })
    ).toThrow();
  });

  it('rejects confidence outside the valid range', () => {
    expect(() =>
      EmberConfigSchema.parse({
        ember: {
          evaluation: {
            min_confidence: 1.5,
          },
        },
      })
    ).toThrow();
  });
});

describe('parseEmberConfig', () => {
  it('returns parsed config for valid input', () => {
    const result = parseEmberConfig({ ember: { enabled: false } });

    expect(result?.ember.enabled).toBe(false);
  });

  it('returns null for invalid input', () => {
    const result = parseEmberConfig({
      ember: {
        limits: {
          max_candidates: 0,
        },
      },
    });

    expect(result).toBeNull();
  });

  it('returns null for non-object input', () => {
    expect(parseEmberConfig('ember')).toBeNull();
    expect(parseEmberConfig(42)).toBeNull();
    expect(parseEmberConfig(null)).toBeNull();
  });
});

describe('DEFAULT_EMBER_CONFIG', () => {
  it('matches parsed defaults', () => {
    const parsedDefaults = EmberConfigSchema.parse({});
    const defaultConfig: EmberConfig = DEFAULT_EMBER_CONFIG;

    expect(defaultConfig).toEqual(parsedDefaults);
  });
});
