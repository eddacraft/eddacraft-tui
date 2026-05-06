/**
 * Diagnostic-shape parity tests.
 *
 * These tests mirror `crates/anvil-kernel-types/src/diagnostics.rs`'s
 * own JSON tests so the TS-side type can be JSON-cast to/from the
 * Rust authoritative shape without drift.
 *
 * If these tests fail, the Rust side is authoritative — update the
 * TS types, NEVER the other way around.
 */

import { describe, expect, it } from 'vitest';

import { DIAGNOSTIC_SCHEMA_VERSION, KNOWN_MODES, type Diagnostic } from './types.js';

const sampleDiagnostic = (): Diagnostic => ({
  schema_version: DIAGNOSTIC_SCHEMA_VERSION,
  id: 'diag_01HW8K6Q4P0X7N9TJ4YA3S0V',
  severity: 'error',
  summary: 'Hardcoded API key detected',
  location: {
    file: 'src/api/client.ts',
    line: 42,
    column: 18,
    end_line: 42,
    end_column: 47,
  },
  category: 'secret',
  source: {
    rule_id: 'secret-aws-access-key',
    source_module: 'anvil-checks::secrets',
  },
  remediation_hint: 'Move to environment variable; see docs/guides/secrets.md',
  mode: 'save-time',
});

describe('Diagnostic — anvil.diagnostic.v1 wire parity', () => {
  it('schema version constant matches the Rust side', () => {
    expect(DIAGNOSTIC_SCHEMA_VERSION).toBe('anvil.diagnostic.v1');
  });

  it('serialises field names in snake_case', () => {
    const diag = sampleDiagnostic();
    const json = JSON.parse(JSON.stringify(diag)) as Record<string, unknown>;
    expect(Object.keys(json)).toEqual(
      expect.arrayContaining([
        'schema_version',
        'id',
        'severity',
        'summary',
        'location',
        'category',
        'source',
        'remediation_hint',
        'mode',
      ])
    );
    expect(json.location).toMatchObject({ end_line: 42, end_column: 47 });
    expect(json.source).toMatchObject({
      rule_id: 'secret-aws-access-key',
      source_module: 'anvil-checks::secrets',
    });
  });

  it('round-trips through JSON', () => {
    const diag = sampleDiagnostic();
    const back = JSON.parse(JSON.stringify(diag)) as Diagnostic;
    expect(back).toEqual(diag);
  });

  it('accepts a path-only location with line/column omitted', () => {
    const diag: Diagnostic = {
      schema_version: DIAGNOSTIC_SCHEMA_VERSION,
      id: 'diag_path',
      severity: 'info',
      summary: 'Path-only finding',
      location: { file: 'README.md' },
      category: 'other',
      source: { rule_id: 'lean-rule', source_module: 'anvil-checks::lean' },
      mode: 'watch',
    };
    const json = JSON.parse(JSON.stringify(diag)) as { location: Record<string, unknown> };
    expect(json.location).toEqual({ file: 'README.md' });
    expect(json.location.line).toBeUndefined();
    expect(json.location.column).toBeUndefined();
  });

  it('accepts unknown mode strings (forward-compat)', () => {
    const diag: Diagnostic = {
      schema_version: DIAGNOSTIC_SCHEMA_VERSION,
      id: 'diag_unknown_mode',
      severity: 'warning',
      summary: 'Future mode',
      location: { file: 'src/x.rs', line: 1, column: 1 },
      category: 'other',
      source: { rule_id: 'r', source_module: 'm' },
      // 'remote-edit' is not a known mode — type system accepts it
      // because Mode includes `string & {}`.
      mode: 'remote-edit',
    };
    expect(JSON.parse(JSON.stringify(diag))).toEqual(diag);
  });

  it('exports the known-mode catalogue used by consumers branching on mode', () => {
    expect(KNOWN_MODES).toEqual(['save-time', 'mid-edit', 'gate', 'watch']);
  });
});
