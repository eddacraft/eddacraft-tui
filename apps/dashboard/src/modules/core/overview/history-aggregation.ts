import type { components } from '@/api/generated/openapi';

type HistoryPoint = components['schemas']['ProtectionHistoryPoint'];
type HistoryRange = components['schemas']['ProtectionHistoryRange'];

export type HistoryInterval = 'daily' | 'weekly';

export interface HistoryBucket {
  key: string;
  label: string;
  passRate: number;
  score: number;
  warningCount: number;
  sampleCount: number;
}

function utcDay(date: Date) {
  return date.toISOString().slice(0, 10);
}

function bucketKey(recordedAt: string, interval: HistoryInterval) {
  const date = new Date(recordedAt);
  if (interval === 'weekly') {
    const daysSinceMonday = (date.getUTCDay() + 6) % 7;
    date.setUTCDate(date.getUTCDate() - daysSinceMonday);
  }
  return utcDay(date);
}

function shortDate(value: string) {
  return new Intl.DateTimeFormat('en-GB', {
    day: 'numeric',
    month: 'short',
    timeZone: 'UTC',
    year: 'numeric',
  }).format(new Date(value));
}

export function aggregateProtectionHistory(
  points: readonly HistoryPoint[],
  interval: HistoryInterval
): HistoryBucket[] {
  const grouped = new Map<
    string,
    HistoryBucket & { latestRecordedAt: number; passCount: number }
  >();
  for (const point of points) {
    const key = bucketKey(point.recorded_at, interval);
    const existing = grouped.get(key);
    if (existing) {
      existing.sampleCount += 1;
      existing.passCount += point.status === 'pass' ? 1 : 0;
      existing.passRate = existing.passCount / existing.sampleCount;
      const recordedAt = Date.parse(point.recorded_at);
      if (recordedAt >= existing.latestRecordedAt) {
        existing.latestRecordedAt = recordedAt;
        existing.score = point.score;
        existing.warningCount = point.warning_count;
      }
      continue;
    }
    const passCount = point.status === 'pass' ? 1 : 0;
    grouped.set(key, {
      key,
      label:
        interval === 'weekly'
          ? `Week of ${shortDate(`${key}T00:00:00Z`)}`
          : shortDate(`${key}T00:00:00Z`),
      latestRecordedAt: Date.parse(point.recorded_at),
      passCount,
      passRate: passCount,
      sampleCount: 1,
      score: point.score,
      warningCount: point.warning_count,
    });
  }
  return [...grouped.values()].map(({ latestRecordedAt: _, passCount: __, ...bucket }) => bucket);
}

export function formatActualRange(range: HistoryRange | null) {
  if (!range) return 'No retained range';
  return `${shortDate(range.first_recorded_at)} – ${shortDate(range.last_recorded_at)}`;
}
