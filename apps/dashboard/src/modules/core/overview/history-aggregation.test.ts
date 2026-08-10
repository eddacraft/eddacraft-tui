import { describe, expect, it } from 'vitest';

import type { components } from '@/api/generated/openapi';
import {
  aggregateProtectionHistory,
  formatActualRange,
} from '@/modules/core/overview/history-aggregation';

type Point = components['schemas']['ProtectionHistoryPoint'];

function point(recorded_at: string, status: Point['status'], warning_count: number): Point {
  return {
    recorded_at,
    status,
    warning_count,
    score: status === 'pass' ? 100 : 60,
    status_label: status,
    duration_seconds: null,
    checks_run: null,
  };
}

describe('protection history aggregation', () => {
  it('buckets daily in UTC without padding and counts only pass as a pass', () => {
    const buckets = aggregateProtectionHistory(
      [
        point('2026-07-05T23:30:00-02:00', 'pass', 1),
        point('2026-07-06T10:00:00Z', 'warn', 4),
        point('2026-07-08T01:00:00Z', 'fail', 2),
      ],
      'daily'
    );

    expect(buckets.map((bucket) => bucket.key)).toEqual(['2026-07-06', '2026-07-08']);
    expect(buckets[0]).toMatchObject({ passRate: 0.5, warningCount: 4, score: 60 });
  });

  it('orders daily buckets chronologically for reverse-ordered input', () => {
    const buckets = aggregateProtectionHistory(
      [point('2026-07-03T12:00:00Z', 'pass', 3), point('2026-07-01T12:00:00Z', 'pass', 1)],
      'daily'
    );

    expect(buckets.map((bucket) => bucket.key)).toEqual(['2026-07-01', '2026-07-03']);
  });

  it('keeps the chronologically latest score and warning count for a bucket', () => {
    const newest = {
      ...point('2026-07-06T18:00:00Z', 'pass', 9),
      score: 97,
    };
    const older = {
      ...point('2026-07-06T08:00:00Z', 'fail', 2),
      score: 42,
    };

    const buckets = aggregateProtectionHistory([newest, older], 'daily');

    expect(buckets).toHaveLength(1);
    expect(buckets[0]).toMatchObject({
      score: 97,
      warningCount: 9,
    });
  });

  it('starts weekly buckets on Monday UTC and keeps the last warning level', () => {
    const buckets = aggregateProtectionHistory(
      [
        point('2026-07-05T23:59:00Z', 'pass', 1),
        point('2026-07-06T00:00:00Z', 'pass', 2),
        point('2026-07-12T23:59:59Z', 'warn', 7),
      ],
      'weekly'
    );

    expect(buckets.map((bucket) => bucket.key)).toEqual(['2026-06-29', '2026-07-06']);
    expect(buckets[1]).toMatchObject({ passRate: 0.5, warningCount: 7, score: 60 });
  });

  it('orders weekly buckets chronologically for reverse-ordered input', () => {
    const buckets = aggregateProtectionHistory(
      [point('2026-07-15T12:00:00Z', 'pass', 3), point('2026-07-01T12:00:00Z', 'pass', 1)],
      'weekly'
    );

    expect(buckets.map((bucket) => bucket.key)).toEqual(['2026-06-29', '2026-07-13']);
  });

  it('labels only the actual covered range', () => {
    expect(
      formatActualRange({
        first_recorded_at: '2026-07-01T23:00:00Z',
        last_recorded_at: '2026-07-03T01:00:00Z',
      })
    ).toBe('1 Jul 2026 – 3 Jul 2026');
  });

  it('aggregates the bounded 500-point resource without padding', () => {
    const points = Array.from({ length: 500 }, (_, index) =>
      point(new Date(Date.UTC(2026, 6, 1) + index * 1_000).toISOString(), 'pass', index)
    );

    const buckets = aggregateProtectionHistory(points, 'daily');

    expect(buckets).toHaveLength(1);
    expect(buckets[0]).toMatchObject({ passRate: 1, sampleCount: 500, warningCount: 499 });
  });
});
