import { useState } from 'react';
import { Line, LineChart, ResponsiveContainer } from 'recharts';

import type { components } from '@/api/generated/openapi';
import { EmptyState } from '@/components/primitives/empty-state';
import { dashboardTheme } from '@/lib/theme';
import {
  aggregateProtectionHistory,
  formatActualRange,
  type HistoryBucket,
  type HistoryInterval,
} from '@/modules/core/overview/history-aggregation';

type History = components['schemas']['ProtectionHistory'];

function Sparkline({
  buckets,
  label,
  value,
  formatValue = String,
}: {
  buckets: readonly HistoryBucket[];
  label: string;
  value: (bucket: HistoryBucket) => number;
  formatValue?: (value: number) => string;
}) {
  const data = buckets.map((bucket) => ({ label: bucket.label, value: value(bucket) }));
  return (
    <div className="history-chart">
      <div aria-label={label} className="history-chart-graphic" data-recharts-chart role="img">
        <ResponsiveContainer height="100%" width="100%">
          <LineChart data={data}>
            <Line
              dataKey="value"
              dot={data.length === 1}
              isAnimationActive={false}
              stroke={dashboardTheme.chart[0]}
              strokeWidth={2}
              type="linear"
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
      <ul aria-label={`${label} values`} className="history-chart-values">
        {buckets.map((bucket) => (
          <li data-history-bucket key={bucket.key}>
            <span>{bucket.label}</span>
            <strong>{formatValue(value(bucket))}</strong>
          </li>
        ))}
      </ul>
    </div>
  );
}

export function TrendCharts({ history }: { history: History }) {
  const [interval, setInterval] = useState<HistoryInterval>('daily');
  if (history.data_state === 'unavailable' || history.points.length === 0) {
    return (
      <section aria-labelledby="history-title" className="panel history-panel">
        <h2 id="history-title">Health trends</h2>
        <EmptyState description={history.source_message} title="No retained gate history" />
      </section>
    );
  }
  const buckets = aggregateProtectionHistory(history.points, interval);
  return (
    <section aria-labelledby="history-title" className="panel history-panel">
      <header className="panel-header">
        <div>
          <h2 id="history-title">Health trends</h2>
          <p>Actual retained range: {formatActualRange(history.actual_range)}</p>
        </div>
        <div aria-label="Trend interval" className="history-interval" role="group">
          {(['daily', 'weekly'] as const).map((next) => (
            <button
              aria-pressed={interval === next}
              key={next}
              onClick={() => setInterval(next)}
              type="button"
            >
              {next === 'daily' ? 'Daily' : 'Weekly'}
            </button>
          ))}
        </div>
      </header>
      {history.data_state === 'partial' ? (
        <div className="resource-state-notice" role="status">
          <strong>Partial retained history</strong>
          <p>{history.source_message}</p>
        </div>
      ) : null}
      <div className="history-chart-grid">
        <article>
          <h3>Gate pass rate</h3>
          <Sparkline
            buckets={buckets}
            formatValue={(value) => `${Math.round(value * 100)}%`}
            label="Gate pass-rate trend"
            value={(bucket) => bucket.passRate}
          />
        </article>
        <article>
          <h3>Gate score</h3>
          <Sparkline buckets={buckets} label="Gate score trend" value={(bucket) => bucket.score} />
        </article>
        <article>
          <h3>Warning count</h3>
          <Sparkline
            buckets={buckets}
            label="Warning count trend"
            value={(bucket) => bucket.warningCount}
          />
        </article>
      </div>
      <p className="history-method-note">
        Pass rate counts only passing gates; warning levels use the last point in each UTC bucket.
      </p>
    </section>
  );
}
