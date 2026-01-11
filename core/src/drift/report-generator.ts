import type { SnapshotComparison } from './snapshot-compare.js';

export interface ReportOptions {
  format?: 'text' | 'json';
  includeDetails?: boolean;
  maxEdges?: number;
  maxHotspots?: number;
}

export interface DriftReport {
  summary: string;
  sections: ReportSection[];
  recommendation: string;
  json?: unknown;
}

export interface ReportSection {
  title: string;
  content: string[];
}

const SEPARATOR = '\u2500'.repeat(56);

function formatDelta(delta: number): string {
  if (delta > 0) return `+${delta}`;
  if (delta < 0) return `${delta}`;
  return '0';
}

function formatTrend(trend: 'increasing' | 'decreasing' | 'stable'): string {
  switch (trend) {
    case 'increasing':
      return '\u26A0\uFE0F';
    case 'decreasing':
      return '\u2705';
    default:
      return '\u2796';
  }
}

function formatOverallTrend(trend: 'improving' | 'stable' | 'degrading'): string {
  switch (trend) {
    case 'improving':
      return 'IMPROVING  \u2705';
    case 'degrading':
      return 'INCREASING  \u26A0\uFE0F';
    default:
      return 'STABLE  \u2796';
  }
}

function formatDate(isoDate: string): string {
  return isoDate.split('T')[0];
}

function generateHeader(comparison: SnapshotComparison): string[] {
  const beforeName = comparison.before.name ?? formatDate(comparison.before.created_at);
  const afterName = comparison.after.name ?? formatDate(comparison.after.created_at);

  return [
    `  Drift Report: ${beforeName} \u2192 ${afterName} (${comparison.duration_days} days)`,
    '',
    SEPARATOR,
  ];
}

function generateBoundarySection(
  comparison: SnapshotComparison,
  options: ReportOptions
): ReportSection {
  const content: string[] = [];
  const metrics = comparison.metrics.boundary_violations;

  content.push(
    `  New violations:     ${formatDelta(comparison.violations.added.length)}  (was ${metrics.before}, now ${metrics.after})`
  );
  content.push(`  Resolved:           ${formatDelta(-comparison.violations.removed.length)}`);
  content.push(
    `  Net change:         ${formatDelta(comparison.net_change.violations)}  ${formatTrend(metrics.trend)}`
  );

  if (options.includeDetails && comparison.violations.added.length > 0) {
    content.push('');
    content.push('  New edges detected:');

    const maxEdges = options.maxEdges ?? 5;
    const edgesToShow = comparison.violations.added.slice(0, maxEdges);

    for (const v of edgesToShow) {
      const rule = v.rule ?? `${v.from_layer}\u2192${v.to_layer}`;
      content.push(`  \u2022 ${v.from_file} \u2192 ${v.to_file} (${rule})`);
    }

    if (comparison.violations.added.length > maxEdges) {
      content.push(`  ... and ${comparison.violations.added.length - maxEdges} more`);
    }
  }

  return { title: 'ARCHITECTURE BOUNDARIES', content };
}

function generateAntiPatternSection(
  comparison: SnapshotComparison,
  options: ReportOptions
): ReportSection {
  const content: string[] = [];
  const metrics = comparison.metrics.antipattern_count;

  content.push(`  New introductions:  ${formatDelta(comparison.antipatterns.added.length)}`);
  content.push(`  Resolved:           ${formatDelta(-comparison.antipatterns.removed.length)}`);
  content.push(
    `  Net change:         ${formatDelta(comparison.net_change.antipatterns)}  ${formatTrend(metrics.trend)}`
  );

  if (comparison.antipattern_changes.length > 0) {
    content.push('');
    content.push('  By type:');
    for (const change of comparison.antipattern_changes.slice(0, 5)) {
      content.push(`  \u2022 ${change.id}:     ${formatDelta(change.delta)}`);
    }
  }

  if (options.includeDetails) {
    const hotspots = findAntiPatternHotspots(comparison);
    if (hotspots.length > 0) {
      content.push('');
      content.push('  Hotspots:');
      const maxHotspots = options.maxHotspots ?? 3;
      for (const hs of hotspots.slice(0, maxHotspots)) {
        content.push(`  \u2022 ${hs.path} \u2014 ${hs.count} new violations`);
      }
    }
  }

  return { title: 'ANTI-PATTERNS', content };
}

function findAntiPatternHotspots(
  comparison: SnapshotComparison
): Array<{ path: string; count: number }> {
  const dirCounts = new Map<string, number>();

  for (const ap of comparison.antipatterns.added) {
    const parts = ap.file.split('/');
    const dir = parts.slice(0, -1).join('/') || '.';
    dirCounts.set(dir, (dirCounts.get(dir) ?? 0) + 1);
  }

  return Array.from(dirCounts.entries())
    .filter(([_, count]) => count > 1)
    .sort((a, b) => b[1] - a[1])
    .map(([path, count]) => ({ path, count }));
}

function generateSuppressionSection(comparison: SnapshotComparison): ReportSection {
  const content: string[] = [];
  const expiredMetrics = comparison.metrics.expired_suppressions;

  content.push(`  New suppressions:   ${formatDelta(comparison.suppressions.added.length)}`);
  content.push(`  Expired:            ${formatDelta(expiredMetrics.delta)}`);
  content.push(`  Net change:         ${formatDelta(comparison.net_change.suppressions)}`);

  const oldestUnexpired = findOldestUnexpiredSuppression(comparison);
  if (oldestUnexpired) {
    content.push('');
    content.push(`  Oldest unexpired: ${oldestUnexpired.age} days (${oldestUnexpired.file})`);
  }

  return { title: 'SUPPRESSIONS', content };
}

function findOldestUnexpiredSuppression(
  comparison: SnapshotComparison
): { age: number; file: string } | null {
  const activeSuppressions = comparison.suppressions.unchanged.filter((s) => !s.is_expired);
  if (activeSuppressions.length === 0) return null;

  return {
    age: comparison.duration_days,
    file: activeSuppressions[0].file,
  };
}

function generateSummarySection(comparison: SnapshotComparison): ReportSection {
  const content: string[] = [];

  content.push(`  Overall drift:  ${formatOverallTrend(comparison.overall_trend)}`);

  return { title: 'SUMMARY', content };
}

function generateRecommendation(comparison: SnapshotComparison): string {
  if (comparison.overall_trend === 'degrading') {
    if (comparison.violations.added.length > 0) {
      const topViolationDir = getMostAffectedDirectory(comparison.violations.added);
      return `Review new violations in ${topViolationDir}`;
    }
    if (comparison.antipatterns.added.length > 0) {
      const topPattern = comparison.antipattern_changes[0]?.id ?? 'anti-patterns';
      return `Address increasing ${topPattern} occurrences`;
    }
  }

  if (comparison.overall_trend === 'improving') {
    return 'Good progress! Continue addressing remaining issues.';
  }

  return 'Architecture is stable. Monitor for new changes.';
}

function getMostAffectedDirectory(violations: Array<{ from_file: string }>): string {
  const dirCounts = new Map<string, number>();

  for (const v of violations) {
    const parts = v.from_file.split('/');
    const dir = parts.slice(0, 2).join('/') || '.';
    dirCounts.set(dir, (dirCounts.get(dir) ?? 0) + 1);
  }

  let maxDir = '.';
  let maxCount = 0;

  for (const [dir, count] of dirCounts) {
    if (count > maxCount) {
      maxCount = count;
      maxDir = dir;
    }
  }

  return maxDir + '/';
}

export function generateReport(
  comparison: SnapshotComparison,
  options: ReportOptions = {}
): DriftReport {
  const { includeDetails = true } = options;
  const effectiveOptions = { ...options, includeDetails };

  const sections: ReportSection[] = [
    generateBoundarySection(comparison, effectiveOptions),
    generateAntiPatternSection(comparison, effectiveOptions),
    generateSuppressionSection(comparison),
    generateSummarySection(comparison),
  ];

  const recommendation = generateRecommendation(comparison);

  const header = generateHeader(comparison);
  const sectionTexts = sections.map((s) => {
    return [`  ${s.title}`, '', ...s.content, '', SEPARATOR].join('\n');
  });

  const summary = [...header, '', ...sectionTexts, `  Recommendation: ${recommendation}`, ''].join(
    '\n'
  );

  return {
    summary,
    sections,
    recommendation,
    json: options.format === 'json' ? generateJsonReport(comparison) : undefined,
  };
}

function generateJsonReport(comparison: SnapshotComparison): unknown {
  return {
    before: comparison.before,
    after: comparison.after,
    duration_days: comparison.duration_days,
    metrics: {
      boundary_violations: {
        before: comparison.metrics.boundary_violations.before,
        after: comparison.metrics.boundary_violations.after,
        delta: comparison.metrics.boundary_violations.delta,
      },
      antipattern_count: {
        before: comparison.metrics.antipattern_count.before,
        after: comparison.metrics.antipattern_count.after,
        delta: comparison.metrics.antipattern_count.delta,
      },
      suppression_count: {
        before: comparison.metrics.suppression_count.before,
        after: comparison.metrics.suppression_count.after,
        delta: comparison.metrics.suppression_count.delta,
      },
    },
    net_change: comparison.net_change,
    changes: {
      violations: {
        added: comparison.violations.added.length,
        removed: comparison.violations.removed.length,
        unchanged: comparison.violations.unchanged.length,
      },
      antipatterns: {
        added: comparison.antipatterns.added.length,
        removed: comparison.antipatterns.removed.length,
        unchanged: comparison.antipatterns.unchanged.length,
      },
      suppressions: {
        added: comparison.suppressions.added.length,
        removed: comparison.suppressions.removed.length,
        unchanged: comparison.suppressions.unchanged.length,
      },
    },
    antipattern_changes: comparison.antipattern_changes,
    overall_trend: comparison.overall_trend,
    added_violations: comparison.violations.added,
    added_antipatterns: comparison.antipatterns.added,
  };
}

export function formatReportAsText(report: DriftReport): string {
  return report.summary;
}

export function formatReportAsJson(report: DriftReport): string {
  return JSON.stringify(report.json ?? {}, null, 2);
}
