/**
 * Config tests (TCOV-018)
 *
 * Covers: KindlingConfig schema validation, default values, loadKindlingConfig
 * (file-based loading), and shouldCapture per-kind gating logic.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import {
  KindlingConfigSchema,
  CaptureConfigSchema,
  RetentionConfigSchema,
  QueryLimitConfigSchema,
  DEFAULT_KINDLING_CONFIG,
  loadKindlingConfig,
  shouldCapture,
} from './config.js';

// =============================================================================
// Schema validation tests
// =============================================================================

describe('KindlingConfigSchema', () => {
  it('parses minimal input with all defaults', () => {
    const result = KindlingConfigSchema.safeParse({});
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.enabled).toBe(false);
      expect(result.data.database_path).toBe('.anvil/kindling.db');
      expect(result.data.retention.days).toBe(90);
      expect(result.data.retention.auto_prune).toBe(false);
      expect(result.data.capture.sessions).toBe(true);
      expect(result.data.capture.gates).toBe(true);
      expect(result.data.capture.actions).toBe(true);
      expect(result.data.capture.plans).toBe(true);
      expect(result.data.capture.human_inputs).toBe(true);
      expect(result.data.capture.constraints).toBe(true);
      expect(result.data.capture.errors).toBe(true);
      expect(result.data.query_limits.max_results).toBe(100);
      expect(result.data.query_limits.max_payload_bytes).toBe(1024 * 1024);
    }
  });

  it('parses enabled: true', () => {
    const result = KindlingConfigSchema.safeParse({ enabled: true });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.enabled).toBe(true);
    }
  });

  it('parses custom database_path', () => {
    const result = KindlingConfigSchema.safeParse({ database_path: '/tmp/my.db' });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.database_path).toBe('/tmp/my.db');
    }
  });

  it('parses partial capture config with defaults filling in the rest', () => {
    const result = KindlingConfigSchema.safeParse({
      enabled: true,
      capture: { sessions: false },
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.capture.sessions).toBe(false);
      expect(result.data.capture.gates).toBe(true);
    }
  });

  it('parses partial retention config', () => {
    const result = KindlingConfigSchema.safeParse({ retention: { days: 30 } });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.retention.days).toBe(30);
      expect(result.data.retention.auto_prune).toBe(false);
    }
  });

  it('rejects retention.days < 1', () => {
    const result = KindlingConfigSchema.safeParse({ retention: { days: 0 } });
    expect(result.success).toBe(false);
  });

  it('rejects retention.days as non-integer', () => {
    const result = KindlingConfigSchema.safeParse({ retention: { days: 45.5 } });
    expect(result.success).toBe(false);
  });

  it('parses custom query limits', () => {
    const result = KindlingConfigSchema.safeParse({
      query_limits: { max_results: 50, max_payload_bytes: 512 * 1024 },
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.query_limits.max_results).toBe(50);
      expect(result.data.query_limits.max_payload_bytes).toBe(512 * 1024);
    }
  });

  it('rejects query_limits.max_results > 1000', () => {
    const result = KindlingConfigSchema.safeParse({
      query_limits: { max_results: 1001 },
    });
    expect(result.success).toBe(false);
  });

  it('rejects query_limits.max_payload_bytes > 10MB', () => {
    const result = KindlingConfigSchema.safeParse({
      query_limits: { max_payload_bytes: 10 * 1024 * 1024 + 1 },
    });
    expect(result.success).toBe(false);
  });

  it('rejects query_limits.max_results < 1', () => {
    const result = KindlingConfigSchema.safeParse({
      query_limits: { max_results: 0 },
    });
    expect(result.success).toBe(false);
  });
});

describe('CaptureConfigSchema', () => {
  it('defaults all fields to true', () => {
    const result = CaptureConfigSchema.safeParse({});
    expect(result.success).toBe(true);
    if (result.success) {
      expect(Object.values(result.data).every(Boolean)).toBe(true);
    }
  });

  it('accepts all false', () => {
    const result = CaptureConfigSchema.safeParse({
      sessions: false,
      gates: false,
      actions: false,
      plans: false,
      human_inputs: false,
      constraints: false,
      errors: false,
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(Object.values(result.data).every((v) => !v)).toBe(true);
    }
  });
});

describe('RetentionConfigSchema', () => {
  it('defaults to 90 days, auto_prune false', () => {
    const result = RetentionConfigSchema.safeParse({});
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.days).toBe(90);
      expect(result.data.auto_prune).toBe(false);
    }
  });

  it('accepts auto_prune: true', () => {
    const result = RetentionConfigSchema.safeParse({ auto_prune: true });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.auto_prune).toBe(true);
    }
  });

  it('rejects non-positive days', () => {
    expect(RetentionConfigSchema.safeParse({ days: -1 }).success).toBe(false);
  });
});

describe('QueryLimitConfigSchema', () => {
  it('defaults to max_results=100, max_payload_bytes=1MB', () => {
    const result = QueryLimitConfigSchema.safeParse({});
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.max_results).toBe(100);
      expect(result.data.max_payload_bytes).toBe(1024 * 1024);
    }
  });

  it('rejects non-integer max_results', () => {
    expect(QueryLimitConfigSchema.safeParse({ max_results: 1.5 }).success).toBe(false);
  });
});

// =============================================================================
// DEFAULT_KINDLING_CONFIG
// =============================================================================

describe('DEFAULT_KINDLING_CONFIG', () => {
  it('is disabled', () => {
    expect(DEFAULT_KINDLING_CONFIG.enabled).toBe(false);
  });

  it('has sensible retention defaults', () => {
    expect(DEFAULT_KINDLING_CONFIG.retention.days).toBe(90);
    expect(DEFAULT_KINDLING_CONFIG.retention.auto_prune).toBe(false);
  });

  it('has all capture kinds enabled', () => {
    const { capture } = DEFAULT_KINDLING_CONFIG;
    expect(capture.sessions).toBe(true);
    expect(capture.gates).toBe(true);
    expect(capture.actions).toBe(true);
    expect(capture.plans).toBe(true);
    expect(capture.human_inputs).toBe(true);
    expect(capture.constraints).toBe(true);
    expect(capture.errors).toBe(true);
  });

  it('has sensible query limit defaults', () => {
    expect(DEFAULT_KINDLING_CONFIG.query_limits.max_results).toBe(100);
    expect(DEFAULT_KINDLING_CONFIG.query_limits.max_payload_bytes).toBe(1024 * 1024);
  });
});

// =============================================================================
// loadKindlingConfig
// =============================================================================

describe('loadKindlingConfig', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), 'kindling-config-test-'));
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it('returns DEFAULT_KINDLING_CONFIG when no config file exists', () => {
    const config = loadKindlingConfig(tmpDir);
    expect(config).toEqual(DEFAULT_KINDLING_CONFIG);
  });

  it('loads kindling section from .anvilrc', () => {
    writeFileSync(
      join(tmpDir, '.anvilrc'),
      JSON.stringify({ kindling: { enabled: true, retention: { days: 30 } } })
    );
    const config = loadKindlingConfig(tmpDir);
    expect(config.enabled).toBe(true);
    expect(config.retention.days).toBe(30);
  });

  it('loads kindling section from anvil.config.json when .anvilrc absent', () => {
    writeFileSync(
      join(tmpDir, 'anvil.config.json'),
      JSON.stringify({ kindling: { enabled: true } })
    );
    const config = loadKindlingConfig(tmpDir);
    expect(config.enabled).toBe(true);
  });

  it('prefers .anvilrc over anvil.config.json when both exist', () => {
    writeFileSync(
      join(tmpDir, '.anvilrc'),
      JSON.stringify({ kindling: { enabled: true, database_path: 'from-anvilrc.db' } })
    );
    writeFileSync(
      join(tmpDir, 'anvil.config.json'),
      JSON.stringify({ kindling: { enabled: false, database_path: 'from-config-json.db' } })
    );
    const config = loadKindlingConfig(tmpDir);
    expect(config.database_path).toBe('from-anvilrc.db');
  });

  it('returns DEFAULT when config file has no kindling key', () => {
    writeFileSync(join(tmpDir, '.anvilrc'), JSON.stringify({ other: 'stuff' }));
    const config = loadKindlingConfig(tmpDir);
    expect(config).toEqual(DEFAULT_KINDLING_CONFIG);
  });

  it('returns DEFAULT when config file has invalid JSON', () => {
    writeFileSync(join(tmpDir, '.anvilrc'), 'not-json!!!');
    const config = loadKindlingConfig(tmpDir);
    expect(config).toEqual(DEFAULT_KINDLING_CONFIG);
  });

  it('returns DEFAULT when config file contains a non-object JSON value', () => {
    writeFileSync(join(tmpDir, '.anvilrc'), JSON.stringify(null));
    const config = loadKindlingConfig(tmpDir);
    expect(config).toEqual(DEFAULT_KINDLING_CONFIG);
  });

  it('returns DEFAULT when kindling section has an invalid shape', () => {
    writeFileSync(
      join(tmpDir, '.anvilrc'),
      JSON.stringify({ kindling: { retention: { days: -5 } } })
    );
    const config = loadKindlingConfig(tmpDir);
    expect(config).toEqual(DEFAULT_KINDLING_CONFIG);
  });

  it('returns DEFAULT when the kindling key is null', () => {
    writeFileSync(join(tmpDir, '.anvilrc'), JSON.stringify({ kindling: null }));
    // `kindling: null` is not the same as an absent key: it does not hit the
    // `record['kindling'] === undefined` short-circuit. It fails schema
    // validation instead, and loadKindlingConfig falls back to DEFAULT.
    const config = loadKindlingConfig(tmpDir);
    expect(config).toEqual(DEFAULT_KINDLING_CONFIG);
  });

  it('fills in defaults for partial kindling config', () => {
    writeFileSync(join(tmpDir, '.anvilrc'), JSON.stringify({ kindling: { enabled: true } }));
    const config = loadKindlingConfig(tmpDir);
    expect(config.enabled).toBe(true);
    expect(config.retention.days).toBe(90);
    expect(config.capture.sessions).toBe(true);
  });
});

// =============================================================================
// shouldCapture
// =============================================================================

describe('shouldCapture', () => {
  const enabledConfig = KindlingConfigSchema.parse({ enabled: true });
  const disabledConfig = KindlingConfigSchema.parse({ enabled: false });

  it('returns false for any kind when disabled', () => {
    expect(shouldCapture(disabledConfig, 'session_start')).toBe(false);
    expect(shouldCapture(disabledConfig, 'gate_evaluated')).toBe(false);
    expect(shouldCapture(disabledConfig, 'error')).toBe(false);
  });

  it('returns true for session_start when sessions enabled', () => {
    expect(shouldCapture(enabledConfig, 'session_start')).toBe(true);
  });

  it('returns true for session_end when sessions enabled', () => {
    expect(shouldCapture(enabledConfig, 'session_end')).toBe(true);
  });

  it('returns false for session kinds when sessions capture disabled', () => {
    const cfg = KindlingConfigSchema.parse({
      enabled: true,
      capture: { sessions: false },
    });
    expect(shouldCapture(cfg, 'session_start')).toBe(false);
    expect(shouldCapture(cfg, 'session_end')).toBe(false);
  });

  it('returns true for gate_evaluated when gates enabled', () => {
    expect(shouldCapture(enabledConfig, 'gate_evaluated')).toBe(true);
  });

  it('returns false for gate_evaluated when gates disabled', () => {
    const cfg = KindlingConfigSchema.parse({ enabled: true, capture: { gates: false } });
    expect(shouldCapture(cfg, 'gate_evaluated')).toBe(false);
  });

  it('returns true for action_executed when actions enabled', () => {
    expect(shouldCapture(enabledConfig, 'action_executed')).toBe(true);
  });

  it('returns false for action_executed when actions disabled', () => {
    const cfg = KindlingConfigSchema.parse({ enabled: true, capture: { actions: false } });
    expect(shouldCapture(cfg, 'action_executed')).toBe(false);
  });

  it('returns true for plan kinds when plans enabled', () => {
    for (const kind of ['plan_created', 'plan_edited', 'plan_approved', 'plan_rejected']) {
      expect(shouldCapture(enabledConfig, kind)).toBe(true);
    }
  });

  it('returns false for plan kinds when plans disabled', () => {
    const cfg = KindlingConfigSchema.parse({ enabled: true, capture: { plans: false } });
    for (const kind of ['plan_created', 'plan_edited', 'plan_approved', 'plan_rejected']) {
      expect(shouldCapture(cfg, kind)).toBe(false);
    }
  });

  it('returns true for human_input when human_inputs enabled', () => {
    expect(shouldCapture(enabledConfig, 'human_input')).toBe(true);
  });

  it('returns false for human_input when human_inputs disabled', () => {
    const cfg = KindlingConfigSchema.parse({ enabled: true, capture: { human_inputs: false } });
    expect(shouldCapture(cfg, 'human_input')).toBe(false);
  });

  it('returns true for constraint_applied when constraints enabled', () => {
    expect(shouldCapture(enabledConfig, 'constraint_applied')).toBe(true);
  });

  it('returns false for constraint_applied when constraints disabled', () => {
    const cfg = KindlingConfigSchema.parse({ enabled: true, capture: { constraints: false } });
    expect(shouldCapture(cfg, 'constraint_applied')).toBe(false);
  });

  it('returns true for error when errors enabled', () => {
    expect(shouldCapture(enabledConfig, 'error')).toBe(true);
  });

  it('returns false for error when errors disabled', () => {
    const cfg = KindlingConfigSchema.parse({ enabled: true, capture: { errors: false } });
    expect(shouldCapture(cfg, 'error')).toBe(false);
  });

  it('returns true for unknown kinds when enabled (default case)', () => {
    expect(shouldCapture(enabledConfig, 'some.future.kind')).toBe(true);
  });
});
