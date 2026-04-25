// Pattern-registry getters on the internal napi binding.
//
// Per ADR-030 the binding is a CLI-acceleration path, not a VSCode /
// MCP consumer surface (those go through the intercept daemon in
// DRVR). These tests pin `getDefaultPatternsJson()` and
// `getPatternJson(id)` against the Rust catalogue so any consumer
// using the binding for rule enumeration (docs generation, diagnostics
// without running a scan) has a stable wire contract.
//
// Run via: `pnpm --filter @eddacraft/anvil-checks-native test`.

import { test } from 'node:test';
import assert from 'node:assert/strict';

const { getDefaultPatternsJson, getPatternJson } = await import('../index.js');

test('getDefaultPatternsJson returns a non-empty array of patterns', () => {
  const patterns = JSON.parse(getDefaultPatternsJson());
  assert.ok(Array.isArray(patterns), 'must be an array');
  assert.ok(patterns.length > 0, 'default catalogue must not be empty');

  for (const pattern of patterns) {
    assert.equal(typeof pattern.id, 'string');
    assert.equal(typeof pattern.name, 'string');
    assert.equal(typeof pattern.severity, 'string');
    assert.equal(pattern.enabled, true, `default pattern ${pattern.id} must be enabled`);
    assert.equal(pattern.optIn, false, `default pattern ${pattern.id} must not be opt-in`);
  }
});

test('getDefaultPatternsJson includes core anti-patterns', () => {
  const patterns = JSON.parse(getDefaultPatternsJson());
  const ids = new Set(patterns.map((p) => p.id));
  for (const id of ['AP-001', 'AP-003']) {
    assert.ok(ids.has(id), `default catalogue missing ${id}: ${[...ids].join(',')}`);
  }
});

test('getPatternJson returns a pattern for a known id', () => {
  const raw = getPatternJson('AP-003');
  assert.ok(raw !== null && raw !== undefined, 'AP-003 must resolve');
  const pattern = JSON.parse(raw);
  assert.equal(pattern.id, 'AP-003');
  assert.equal(typeof pattern.regex, 'string');
  assert.ok(pattern.regex.length > 0, 'pattern must carry its regex');
});

test('getPatternJson returns null for unknown ids', () => {
  const raw = getPatternJson('AP-999');
  assert.equal(raw, null, 'miss is null, not a throw');
});

test('getPatternJson surface covers every default pattern', () => {
  const defaults = JSON.parse(getDefaultPatternsJson());
  for (const pattern of defaults) {
    const raw = getPatternJson(pattern.id);
    assert.ok(raw, `getPatternJson(${pattern.id}) must resolve`);
    const resolved = JSON.parse(raw);
    assert.equal(resolved.id, pattern.id);
  }
});
