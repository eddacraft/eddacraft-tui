import { describe, it, expect } from 'vitest';
import { generateReport, formatReportAsText, formatReportAsJson } from './report-generator.js';
import { compareSnapshots } from './snapshot-compare.js';
import { createEmptySnapshot, type DriftSnapshot } from './snapshot-schema.js';

function createTestSnapshot(overrides: Partial<DriftSnapshot> = {}): DriftSnapshot {
  const base = createEmptySnapshot();
  return { ...base, ...overrides };
}

describe('ReportGenerator', () => {
  describe('generateReport', () => {
    it('should generate report with all sections', () => {
      const before = createTestSnapshot({
        name: 'release-1.0',
        created_at: '2025-01-15T00:00:00.000Z',
        metrics: {
          boundary_violations: 12,
          antipattern_count: 40,
          suppression_count: 5,
          expired_suppressions: 1,
          files_analysed: 100,
        },
      });

      const after = createTestSnapshot({
        name: 'release-1.1',
        created_at: '2025-01-31T00:00:00.000Z',
        violations: [
          {
            id: 'new-1',
            type: 'boundary',
            from_file: 'src/api/handlers.ts',
            to_file: 'src/core/internal.ts',
            from_layer: 'presentation',
            to_layer: 'domain',
            line: 10,
            rule: 'ARCH-001',
          },
        ],
        metrics: {
          boundary_violations: 15,
          antipattern_count: 45,
          suppression_count: 8,
          expired_suppressions: 1,
          files_analysed: 100,
        },
      });

      const comparison = compareSnapshots(before, after);
      const report = generateReport(comparison);

      expect(report.sections).toHaveLength(4);
      expect(report.sections[0].title).toBe('ARCHITECTURE BOUNDARIES');
      expect(report.sections[1].title).toBe('ANTI-PATTERNS');
      expect(report.sections[2].title).toBe('SUPPRESSIONS');
      expect(report.sections[3].title).toBe('SUMMARY');
      expect(report.recommendation).toBeDefined();
    });

    it('should include header with dates and duration', () => {
      const before = createTestSnapshot({
        name: 'v1.0',
        created_at: '2025-01-01T00:00:00.000Z',
      });

      const after = createTestSnapshot({
        name: 'v1.1',
        created_at: '2025-01-16T00:00:00.000Z',
      });

      const comparison = compareSnapshots(before, after);
      const report = generateReport(comparison);

      expect(report.summary).toContain('v1.0');
      expect(report.summary).toContain('v1.1');
      expect(report.summary).toContain('15 days');
    });

    it('should show new violations when details enabled', () => {
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
            id: 'v1',
            type: 'boundary',
            from_file: 'src/api/handler.ts',
            to_file: 'src/db/query.ts',
            from_layer: 'presentation',
            to_layer: 'infrastructure',
            line: 10,
            rule: 'ARCH-001',
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
      const report = generateReport(comparison, { includeDetails: true });

      expect(report.summary).toContain('New edges detected');
      expect(report.summary).toContain('src/api/handler.ts');
      expect(report.summary).toContain('src/db/query.ts');
    });

    it('should show anti-pattern breakdown', () => {
      const before = createTestSnapshot({
        created_at: '2025-01-01T00:00:00.000Z',
        antipattern_breakdown: { 'AP-003': 5 },
        metrics: {
          boundary_violations: 0,
          antipattern_count: 5,
          suppression_count: 0,
          expired_suppressions: 0,
          files_analysed: 10,
        },
      });

      const after = createTestSnapshot({
        created_at: '2025-01-15T00:00:00.000Z',
        antipattern_breakdown: { 'AP-003': 8, 'AP-004': 2 },
        metrics: {
          boundary_violations: 0,
          antipattern_count: 10,
          suppression_count: 0,
          expired_suppressions: 0,
          files_analysed: 10,
        },
      });

      const comparison = compareSnapshots(before, after);
      const report = generateReport(comparison);

      expect(report.summary).toContain('By type:');
      expect(report.summary).toContain('AP-003');
    });

    it('should generate appropriate recommendation for degrading trend', () => {
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
        violations: [
          {
            id: 'new-v',
            type: 'boundary',
            from_file: 'src/api/test.ts',
            to_file: 'src/db/query.ts',
            from_layer: 'presentation',
            to_layer: 'infrastructure',
            line: 1,
          },
        ],
        metrics: {
          boundary_violations: 10,
          antipattern_count: 20,
          suppression_count: 0,
          expired_suppressions: 0,
          files_analysed: 100,
        },
      });

      const comparison = compareSnapshots(before, after);
      const report = generateReport(comparison);

      expect(report.recommendation).toContain('Review');
    });

    it('should generate appropriate recommendation for improving trend', () => {
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
      const report = generateReport(comparison);

      expect(report.recommendation).toContain('progress');
    });
  });

  describe('formatReportAsText', () => {
    it('should return summary string', () => {
      const before = createTestSnapshot({ created_at: '2025-01-01T00:00:00.000Z' });
      const after = createTestSnapshot({ created_at: '2025-01-15T00:00:00.000Z' });

      const comparison = compareSnapshots(before, after);
      const report = generateReport(comparison);
      const text = formatReportAsText(report);

      expect(text).toBe(report.summary);
      expect(text).toContain('Drift Report');
    });
  });

  describe('formatReportAsJson', () => {
    it('should return valid JSON', () => {
      const before = createTestSnapshot({
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
        created_at: '2025-01-15T00:00:00.000Z',
        metrics: {
          boundary_violations: 7,
          antipattern_count: 12,
          suppression_count: 3,
          expired_suppressions: 0,
          files_analysed: 100,
        },
      });

      const comparison = compareSnapshots(before, after);
      const report = generateReport(comparison, { format: 'json' });
      const json = formatReportAsJson(report);

      const parsed = JSON.parse(json);

      expect(parsed.metrics).toBeDefined();
      expect(parsed.metrics.boundary_violations.before).toBe(5);
      expect(parsed.metrics.boundary_violations.after).toBe(7);
      expect(parsed.metrics.boundary_violations.delta).toBe(2);
      expect(parsed.net_change).toBeDefined();
      expect(parsed.overall_trend).toBe('degrading');
    });
  });
});
