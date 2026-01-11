import { describe, it, expect } from 'vitest';
import { compareSnapshots, formatComparisonSummary } from './snapshot-compare.js';
import { createEmptySnapshot, type DriftSnapshot } from './snapshot-schema.js';

function createTestSnapshot(overrides: Partial<DriftSnapshot> = {}): DriftSnapshot {
  const base = createEmptySnapshot();
  return { ...base, ...overrides };
}

describe('SnapshotCompare', () => {
  describe('compareSnapshots', () => {
    it('should compare two empty snapshots', () => {
      const before = createTestSnapshot({ created_at: '2025-01-01T00:00:00.000Z' });
      const after = createTestSnapshot({ created_at: '2025-01-15T00:00:00.000Z' });

      const comparison = compareSnapshots(before, after);

      expect(comparison.overall_trend).toBe('stable');
      expect(comparison.duration_days).toBe(14);
      expect(comparison.net_change.violations).toBe(0);
      expect(comparison.net_change.antipatterns).toBe(0);
    });

    it('should detect added violations', () => {
      const before = createTestSnapshot({
        created_at: '2025-01-01T00:00:00.000Z',
        violations: [],
        metrics: {
          boundary_violations: 0,
          antipattern_count: 0,
          suppression_count: 0,
          expired_suppressions: 0,
          files_analysed: 10,
        },
      });

      const after = createTestSnapshot({
        created_at: '2025-01-15T00:00:00.000Z',
        violations: [
          {
            id: 'new-violation',
            type: 'boundary',
            from_file: 'src/api/handler.ts',
            to_file: 'src/db/query.ts',
            from_layer: 'presentation',
            to_layer: 'infrastructure',
            line: 10,
          },
        ],
        metrics: {
          boundary_violations: 1,
          antipattern_count: 0,
          suppression_count: 0,
          expired_suppressions: 0,
          files_analysed: 10,
        },
      });

      const comparison = compareSnapshots(before, after);

      expect(comparison.violations.added).toHaveLength(1);
      expect(comparison.violations.removed).toHaveLength(0);
      expect(comparison.net_change.violations).toBe(1);
      expect(comparison.metrics.boundary_violations.delta).toBe(1);
      expect(comparison.metrics.boundary_violations.trend).toBe('increasing');
    });

    it('should detect removed violations', () => {
      const before = createTestSnapshot({
        created_at: '2025-01-01T00:00:00.000Z',
        violations: [
          {
            id: 'fixed-violation',
            type: 'boundary',
            from_file: 'src/old.ts',
            to_file: 'src/other.ts',
            from_layer: 'domain',
            to_layer: 'infrastructure',
            line: 5,
          },
        ],
        metrics: {
          boundary_violations: 1,
          antipattern_count: 0,
          suppression_count: 0,
          expired_suppressions: 0,
          files_analysed: 10,
        },
      });

      const after = createTestSnapshot({
        created_at: '2025-01-15T00:00:00.000Z',
        violations: [],
        metrics: {
          boundary_violations: 0,
          antipattern_count: 0,
          suppression_count: 0,
          expired_suppressions: 0,
          files_analysed: 10,
        },
      });

      const comparison = compareSnapshots(before, after);

      expect(comparison.violations.added).toHaveLength(0);
      expect(comparison.violations.removed).toHaveLength(1);
      expect(comparison.net_change.violations).toBe(-1);
      expect(comparison.overall_trend).toBe('improving');
    });

    it('should detect added anti-patterns', () => {
      const before = createTestSnapshot({
        created_at: '2025-01-01T00:00:00.000Z',
        antipatterns: [],
        metrics: {
          boundary_violations: 0,
          antipattern_count: 0,
          suppression_count: 0,
          expired_suppressions: 0,
          files_analysed: 10,
        },
      });

      const after = createTestSnapshot({
        created_at: '2025-01-15T00:00:00.000Z',
        antipatterns: [
          {
            id: 'AP-003',
            file: 'src/new.ts',
            line: 10,
            pattern: 'explicit-any',
            severity: 'warning',
          },
        ],
        antipattern_breakdown: { 'AP-003': 1 },
        metrics: {
          boundary_violations: 0,
          antipattern_count: 1,
          suppression_count: 0,
          expired_suppressions: 0,
          files_analysed: 10,
        },
      });

      const comparison = compareSnapshots(before, after);

      expect(comparison.antipatterns.added).toHaveLength(1);
      expect(comparison.net_change.antipatterns).toBe(1);
      expect(comparison.antipattern_changes).toHaveLength(1);
      expect(comparison.antipattern_changes[0].id).toBe('AP-003');
      expect(comparison.antipattern_changes[0].delta).toBe(1);
    });

    it('should detect suppression changes', () => {
      const before = createTestSnapshot({
        created_at: '2025-01-01T00:00:00.000Z',
        suppressions: [
          {
            id: 'old-suppression',
            pattern_id: 'AP-003',
            file: 'src/old.ts',
            line: 5,
            reason: 'Old reason',
            scope: 'statement',
          },
        ],
        metrics: {
          boundary_violations: 0,
          antipattern_count: 0,
          suppression_count: 1,
          expired_suppressions: 0,
          files_analysed: 10,
        },
      });

      const after = createTestSnapshot({
        created_at: '2025-01-15T00:00:00.000Z',
        suppressions: [
          {
            id: 'new-suppression',
            pattern_id: 'AP-004',
            file: 'src/new.ts',
            line: 10,
            reason: 'New reason',
            scope: 'line',
          },
        ],
        metrics: {
          boundary_violations: 0,
          antipattern_count: 0,
          suppression_count: 1,
          expired_suppressions: 0,
          files_analysed: 10,
        },
      });

      const comparison = compareSnapshots(before, after);

      expect(comparison.suppressions.added).toHaveLength(1);
      expect(comparison.suppressions.removed).toHaveLength(1);
      expect(comparison.net_change.suppressions).toBe(0);
    });

    it('should identify unchanged items', () => {
      const sharedViolation = {
        id: 'shared-violation',
        type: 'boundary' as const,
        from_file: 'src/a.ts',
        to_file: 'src/b.ts',
        from_layer: 'presentation',
        to_layer: 'domain',
        line: 5,
      };

      const before = createTestSnapshot({
        created_at: '2025-01-01T00:00:00.000Z',
        violations: [sharedViolation],
        metrics: {
          boundary_violations: 1,
          antipattern_count: 0,
          suppression_count: 0,
          expired_suppressions: 0,
          files_analysed: 10,
        },
      });

      const after = createTestSnapshot({
        created_at: '2025-01-15T00:00:00.000Z',
        violations: [sharedViolation],
        metrics: {
          boundary_violations: 1,
          antipattern_count: 0,
          suppression_count: 0,
          expired_suppressions: 0,
          files_analysed: 10,
        },
      });

      const comparison = compareSnapshots(before, after);

      expect(comparison.violations.unchanged).toHaveLength(1);
      expect(comparison.violations.added).toHaveLength(0);
      expect(comparison.violations.removed).toHaveLength(0);
    });

    it('should calculate correct duration', () => {
      const before = createTestSnapshot({ created_at: '2025-01-01T00:00:00.000Z' });
      const after = createTestSnapshot({ created_at: '2025-02-01T00:00:00.000Z' });

      const comparison = compareSnapshots(before, after);

      expect(comparison.duration_days).toBe(31);
    });

    it('should determine degrading trend', () => {
      const before = createTestSnapshot({
        created_at: '2025-01-01T00:00:00.000Z',
        metrics: {
          boundary_violations: 5,
          antipattern_count: 10,
          suppression_count: 0,
          expired_suppressions: 0,
          files_analysed: 100,
        },
      });

      const after = createTestSnapshot({
        created_at: '2025-01-15T00:00:00.000Z',
        metrics: {
          boundary_violations: 10,
          antipattern_count: 20,
          suppression_count: 0,
          expired_suppressions: 0,
          files_analysed: 100,
        },
      });

      const comparison = compareSnapshots(before, after);

      expect(comparison.overall_trend).toBe('degrading');
    });

    it('should determine improving trend', () => {
      const before = createTestSnapshot({
        created_at: '2025-01-01T00:00:00.000Z',
        metrics: {
          boundary_violations: 10,
          antipattern_count: 20,
          suppression_count: 0,
          expired_suppressions: 0,
          files_analysed: 100,
        },
      });

      const after = createTestSnapshot({
        created_at: '2025-01-15T00:00:00.000Z',
        metrics: {
          boundary_violations: 5,
          antipattern_count: 10,
          suppression_count: 0,
          expired_suppressions: 0,
          files_analysed: 100,
        },
      });

      const comparison = compareSnapshots(before, after);

      expect(comparison.overall_trend).toBe('improving');
    });

    it('should include snapshot names in comparison', () => {
      const before = createTestSnapshot({
        name: 'release-1.0',
        created_at: '2025-01-01T00:00:00.000Z',
      });

      const after = createTestSnapshot({
        name: 'release-1.1',
        created_at: '2025-01-15T00:00:00.000Z',
      });

      const comparison = compareSnapshots(before, after);

      expect(comparison.before.name).toBe('release-1.0');
      expect(comparison.after.name).toBe('release-1.1');
    });
  });

  describe('formatComparisonSummary', () => {
    it('should format comparison as readable text', () => {
      const before = createTestSnapshot({
        name: 'before',
        created_at: '2025-01-01T00:00:00.000Z',
        metrics: {
          boundary_violations: 5,
          antipattern_count: 10,
          suppression_count: 2,
          expired_suppressions: 0,
          files_analysed: 100,
        },
      });

      const after = createTestSnapshot({
        name: 'after',
        created_at: '2025-01-15T00:00:00.000Z',
        metrics: {
          boundary_violations: 7,
          antipattern_count: 8,
          suppression_count: 3,
          expired_suppressions: 0,
          files_analysed: 100,
        },
      });

      const comparison = compareSnapshots(before, after);
      const summary = formatComparisonSummary(comparison);

      expect(summary).toContain('before → after');
      expect(summary).toContain('14 days');
      expect(summary).toContain('Boundary violations: 5 → 7 (+2)');
      expect(summary).toContain('Anti-patterns: 10 → 8 (-2)');
      expect(summary).toContain('STABLE');
    });
  });
});
