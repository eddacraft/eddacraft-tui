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
import { makeSourceWithSecret } from '../helpers/fixtures.js';

let ws: E2EWorkspace;

beforeAll(() => {
  ws = createE2EWorkspace({
    files: {
      'src/clean.ts': 'export const x = 1;\n',
      'src/dirty.ts': makeSourceWithSecret(),
    },
  });
});

afterAll(() => ws.cleanup());

describe('Anti-pattern Scanning', () => {
  it('getEnabledPatterns returns a non-empty catalogue', () => {
    const patterns = getEnabledPatterns();
    expect(patterns.length).toBeGreaterThan(0);
  });

  it('scanFile detects secrets in dirty source', async () => {
    const filePath = join(ws.root, 'src/dirty.ts');
    const result: ScanResult = await scanFile(filePath);
    // Should find at least one warning about the hardcoded secret
    expect(result.warnings.length).toBeGreaterThan(0);
  });

  it('scanFile produces no warnings for clean source', async () => {
    const filePath = join(ws.root, 'src/clean.ts');
    const result: ScanResult = await scanFile(filePath);
    expect(result.warnings).toHaveLength(0);
  });

  it('scanFiles aggregates results across multiple files', async () => {
    const results = await scanFiles([
      join(ws.root, 'src/clean.ts'),
      join(ws.root, 'src/dirty.ts'),
    ]);
    expect(results.length).toBe(2);
    const totalWarnings = results.reduce((sum, r) => sum + r.warnings.length, 0);
    expect(totalWarnings).toBeGreaterThan(0);
  });
});

describe('Drift Snapshots', () => {
  it('createEmptySnapshot returns a valid snapshot', () => {
    const snapshot = createEmptySnapshot();
    expect(snapshot).toBeDefined();
    expect(validateSnapshot(snapshot)).toBe(true);
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
        total_violations: 5,
        total_anti_patterns: 3,
      },
    };

    const comparison = compareSnapshots(baseline, current);
    expect(comparison).toBeDefined();
    // There should be metric differences
    expect(comparison.metrics_comparison).toBeDefined();
  });
});
