import type {
  DriftSnapshot,
  SnapshotViolation,
  SnapshotAntiPattern,
  SnapshotSuppression,
  SnapshotMetrics,
} from './snapshot-schema.js';
import { createDebugger } from '../utils/debug.js';

const debug = createDebugger('drift');

export interface ItemChange<T> {
  added: T[];
  removed: T[];
  unchanged: T[];
}

export interface MetricChange {
  before: number;
  after: number;
  delta: number;
  trend: 'increasing' | 'decreasing' | 'stable';
}

export interface MetricsComparison {
  boundary_violations: MetricChange;
  antipattern_count: MetricChange;
  suppression_count: MetricChange;
  expired_suppressions: MetricChange;
  files_analysed: MetricChange;
}

export interface AntiPatternChange {
  id: string;
  before: number;
  after: number;
  delta: number;
}

export interface SnapshotComparison {
  before: {
    name?: string;
    created_at: string;
  };
  after: {
    name?: string;
    created_at: string;
  };
  duration_days: number;

  metrics: MetricsComparison;
  net_change: {
    violations: number;
    antipatterns: number;
    suppressions: number;
  };

  violations: ItemChange<SnapshotViolation>;
  antipatterns: ItemChange<SnapshotAntiPattern>;
  suppressions: ItemChange<SnapshotSuppression>;

  antipattern_changes: AntiPatternChange[];

  overall_trend: 'improving' | 'stable' | 'degrading';
}

function calculateMetricChange(before: number, after: number): MetricChange {
  const delta = after - before;
  let trend: 'increasing' | 'decreasing' | 'stable';

  if (delta > 0) {
    trend = 'increasing';
  } else if (delta < 0) {
    trend = 'decreasing';
  } else {
    trend = 'stable';
  }

  return { before, after, delta, trend };
}

function compareMetrics(before: SnapshotMetrics, after: SnapshotMetrics): MetricsComparison {
  return {
    boundary_violations: calculateMetricChange(
      before.boundary_violations,
      after.boundary_violations
    ),
    antipattern_count: calculateMetricChange(before.antipattern_count, after.antipattern_count),
    suppression_count: calculateMetricChange(before.suppression_count, after.suppression_count),
    expired_suppressions: calculateMetricChange(
      before.expired_suppressions,
      after.expired_suppressions
    ),
    files_analysed: calculateMetricChange(before.files_analysed, after.files_analysed),
  };
}

function compareViolations(
  before: SnapshotViolation[],
  after: SnapshotViolation[]
): ItemChange<SnapshotViolation> {
  const beforeIds = new Set(before.map((v) => v.id));
  const afterIds = new Set(after.map((v) => v.id));

  const added = after.filter((v) => !beforeIds.has(v.id));
  const removed = before.filter((v) => !afterIds.has(v.id));
  const unchanged = after.filter((v) => beforeIds.has(v.id));

  return { added, removed, unchanged };
}

function compareAntiPatterns(
  before: SnapshotAntiPattern[],
  after: SnapshotAntiPattern[]
): ItemChange<SnapshotAntiPattern> {
  const fingerprint = (ap: SnapshotAntiPattern) => `${ap.file}:${ap.line}:${ap.id}`;

  const beforeFingerprints = new Set(before.map(fingerprint));
  const afterFingerprints = new Set(after.map(fingerprint));

  const added = after.filter((ap) => !beforeFingerprints.has(fingerprint(ap)));
  const removed = before.filter((ap) => !afterFingerprints.has(fingerprint(ap)));
  const unchanged = after.filter((ap) => beforeFingerprints.has(fingerprint(ap)));

  return { added, removed, unchanged };
}

function compareSuppressions(
  before: SnapshotSuppression[],
  after: SnapshotSuppression[]
): ItemChange<SnapshotSuppression> {
  const beforeIds = new Set(before.map((s) => s.id));
  const afterIds = new Set(after.map((s) => s.id));

  const added = after.filter((s) => !beforeIds.has(s.id));
  const removed = before.filter((s) => !afterIds.has(s.id));
  const unchanged = after.filter((s) => beforeIds.has(s.id));

  return { added, removed, unchanged };
}

function calculateAntiPatternChanges(
  before: DriftSnapshot,
  after: DriftSnapshot
): AntiPatternChange[] {
  const beforeBreakdown = before.antipattern_breakdown ?? {};
  const afterBreakdown = after.antipattern_breakdown ?? {};

  const allIds = new Set([...Object.keys(beforeBreakdown), ...Object.keys(afterBreakdown)]);

  const changes: AntiPatternChange[] = [];

  for (const id of allIds) {
    const beforeCount = beforeBreakdown[id] ?? 0;
    const afterCount = afterBreakdown[id] ?? 0;
    const delta = afterCount - beforeCount;

    if (delta !== 0) {
      changes.push({ id, before: beforeCount, after: afterCount, delta });
    }
  }

  return changes.sort((a, b) => Math.abs(b.delta) - Math.abs(a.delta));
}

function calculateDurationDays(before: string, after: string): number {
  const beforeDate = new Date(before);
  const afterDate = new Date(after);
  const diffMs = afterDate.getTime() - beforeDate.getTime();
  return Math.round(diffMs / (1000 * 60 * 60 * 24));
}

function determineOverallTrend(metrics: MetricsComparison): 'improving' | 'stable' | 'degrading' {
  const violationsTrend = metrics.boundary_violations.delta;
  const antipatternsTrend = metrics.antipattern_count.delta;

  const totalChange = violationsTrend + antipatternsTrend;

  if (totalChange < 0) {
    return 'improving';
  } else if (totalChange > 0) {
    return 'degrading';
  }
  return 'stable';
}

export function compareSnapshots(before: DriftSnapshot, after: DriftSnapshot): SnapshotComparison {
  debug('comparing snapshots', {
    before: before.name ?? before.created_at,
    after: after.name ?? after.created_at,
  });
  const metrics = compareMetrics(before.metrics, after.metrics);
  const violations = compareViolations(before.violations, after.violations);
  const antipatterns = compareAntiPatterns(before.antipatterns, after.antipatterns);
  const suppressions = compareSuppressions(before.suppressions, after.suppressions);
  const antipatternChanges = calculateAntiPatternChanges(before, after);

  return {
    before: {
      name: before.name,
      created_at: before.created_at,
    },
    after: {
      name: after.name,
      created_at: after.created_at,
    },
    duration_days: calculateDurationDays(before.created_at, after.created_at),

    metrics,
    net_change: {
      violations: violations.added.length - violations.removed.length,
      antipatterns: antipatterns.added.length - antipatterns.removed.length,
      suppressions: suppressions.added.length - suppressions.removed.length,
    },

    violations,
    antipatterns,
    suppressions,

    antipattern_changes: antipatternChanges,

    overall_trend: determineOverallTrend(metrics),
  };
}

export function formatComparisonSummary(comparison: SnapshotComparison): string {
  const lines: string[] = [];

  const beforeName = comparison.before.name ?? comparison.before.created_at.split('T')[0];
  const afterName = comparison.after.name ?? comparison.after.created_at.split('T')[0];

  lines.push(`Comparing: ${beforeName} → ${afterName} (${comparison.duration_days} days)`);
  lines.push('');

  lines.push('Metrics:');
  lines.push(
    `  Boundary violations: ${comparison.metrics.boundary_violations.before} → ${comparison.metrics.boundary_violations.after} (${formatDelta(comparison.metrics.boundary_violations.delta)})`
  );
  lines.push(
    `  Anti-patterns: ${comparison.metrics.antipattern_count.before} → ${comparison.metrics.antipattern_count.after} (${formatDelta(comparison.metrics.antipattern_count.delta)})`
  );
  lines.push(
    `  Suppressions: ${comparison.metrics.suppression_count.before} → ${comparison.metrics.suppression_count.after} (${formatDelta(comparison.metrics.suppression_count.delta)})`
  );
  lines.push('');

  lines.push('Changes:');
  lines.push(
    `  Violations: +${comparison.violations.added.length} added, -${comparison.violations.removed.length} removed`
  );
  lines.push(
    `  Anti-patterns: +${comparison.antipatterns.added.length} added, -${comparison.antipatterns.removed.length} removed`
  );
  lines.push(
    `  Suppressions: +${comparison.suppressions.added.length} added, -${comparison.suppressions.removed.length} removed`
  );
  lines.push('');

  lines.push(`Overall trend: ${comparison.overall_trend.toUpperCase()}`);

  return lines.join('\n');
}

function formatDelta(delta: number): string {
  if (delta > 0) return `+${delta}`;
  if (delta < 0) return `${delta}`;
  return '0';
}
