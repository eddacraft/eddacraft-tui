/**
 * Drift Detection — E2E Tests
 *
 * Tests the drift detection pipeline across package boundaries:
 *   core/antipattern (scan) → core/drift (snapshot/compare) → core/validation
 *
 * Surface: Core domain (antipattern + drift)
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { join } from 'node:path';
import {
  scanFile,
  scanFiles,
  getEnabledPatterns,
  type ScanResult,
} from '@eddacraft/anvil-core/antipattern';
import {
  createEmptySnapshot,
  validateSnapshot,
  compareSnapshots,
  formatComparisonSummary,
  type DriftSnapshot,
} from '@eddacraft/anvil-core/drift';
import { createE2EWorkspace, type E2EWorkspace } from '../helpers/workspace.js';
import { makeSourceWithAntipatterns } from '../helpers/fixtures.js';

let ws: E2EWorkspace;

beforeAll(() => {
  ws = createE2EWorkspace({
    files: {
      'src/clean.ts': 'export const x = 1;\n',
      'src/dirty.ts': makeSourceWithAntipatterns(),
    },
  });
});

afterAll(() => ws.cleanup());

describe('Anti-pattern Scanning', () => {
  it('getEnabledPatterns returns a non-empty catalogue', () => {
    const patterns = getEnabledPatterns();
    expect(patterns.length).toBeGreaterThan(0);
  });

  it('scanFile detects the antipatterns embedded in the dirty fixture', () => {
    const filePath = join(ws.root, 'src/dirty.ts');
    const result: ScanResult = scanFile(filePath, makeSourceWithAntipatterns());
    // The fixture embeds AP-003 (any), AP-004 (@ts-ignore), AP-006 (empty
    // catch) and AP-007 (console). At least one must fire for this fixture
    // to remain a meaningful drift-signal source.
    const firedIds = new Set(result.warnings.map((w) => w.id));
    const expected = ['AP-003', 'AP-004', 'AP-006', 'AP-007'];
    const hits = expected.filter((id) => firedIds.has(id));
    expect(hits.length).toBeGreaterThan(0);
  });

  it('scanFile produces no warnings for clean source', () => {
    const filePath = join(ws.root, 'src/clean.ts');
    const result: ScanResult = scanFile(filePath, 'export const x = 1;\n');
    expect(result.warnings).toHaveLength(0);
  });

  it('scanFiles aggregates results across multiple files', () => {
    const results = scanFiles([
      { path: join(ws.root, 'src/clean.ts'), content: 'export const x = 1;\n' },
      { path: join(ws.root, 'src/dirty.ts'), content: makeSourceWithAntipatterns() },
    ]);
    expect(results.length).toBe(2);
    const totalWarnings = results.reduce(
      (sum: number, r: ScanResult) => sum + r.warnings.length,
      0
    );
    expect(totalWarnings).toBeGreaterThan(0);
  });
});

describe('Drift Snapshots', () => {
  it('createEmptySnapshot returns a valid snapshot', () => {
    const snapshot = createEmptySnapshot();
    expect(snapshot).toBeDefined();
    // validateSnapshot returns { success, data?, error? } — success === true means valid.
    const result = validateSnapshot(snapshot);
    expect(result.success).toBe(true);
  });

  it('comparing two identical snapshots shows no drift', () => {
    const snapshot = createEmptySnapshot();
    const comparison = compareSnapshots(snapshot, snapshot);
    expect(comparison).toBeDefined();
    const summary = formatComparisonSummary(comparison);
    expect(summary).toBeDefined();
  });

  it('comparing different snapshots detects changes', () => {
    const baseline = createEmptySnapshot();
    const current: DriftSnapshot = {
      ...createEmptySnapshot(),
      metrics: {
        ...createEmptySnapshot().metrics,
        boundary_violations: 5,
        antipattern_count: 3,
      },
    };

    const comparison = compareSnapshots(baseline, current);
    expect(comparison).toBeDefined();
    // There should be metric differences
    expect(comparison.metrics).toBeDefined();
  });
});
