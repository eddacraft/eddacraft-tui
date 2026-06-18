import type { ReactNode } from 'react';

export interface MetricCardProps {
  label: string;
  value: string;
  trend?: 'up' | 'down' | 'flat' | null;
  format?: 'number' | 'percent' | 'duration' | null;
}

const trendIndicators: Record<string, string> = {
  up: '\u2191',
  down: '\u2193',
  flat: '\u2192',
};

const trendColours: Record<string, string> = {
  up: 'var(--edda)',
  down: 'var(--anvil)',
  flat: 'var(--text-muted)',
};

export function formatValue(raw: string, format?: MetricCardProps['format']): string {
  if (!format) return raw;
  const num = Number(raw);
  if (Number.isNaN(num)) return raw;
  switch (format) {
    case 'percent':
      return `${num}%`;
    case 'duration': {
      const totalSeconds = Math.round(num);
      return totalSeconds >= 60
        ? `${Math.floor(totalSeconds / 60)}m ${totalSeconds % 60}s`
        : `${totalSeconds}s`;
    }
    case 'number':
      return num.toLocaleString('en-GB');
    default:
      return raw;
  }
}

export function MetricCard({ label, value, trend, format }: MetricCardProps): ReactNode {
  const indicator = trend ? trendIndicators[trend] : null;
  const colour = trend ? trendColours[trend] : undefined;
  const displayed = formatValue(value, format);

  return (
    <div
      style={{
        backgroundColor: 'var(--surface)',
        border: '1px solid var(--structure)',
        padding: '1rem',
        fontFamily: 'monospace',
        minWidth: '10rem',
      }}
    >
      <div style={{ color: 'var(--text-muted)', fontSize: '0.75rem', marginBottom: '0.25rem' }}>
        {label}
      </div>
      <div style={{ color: 'var(--text-primary)', fontSize: '1.5rem', fontWeight: 700 }}>
        {displayed}
        {indicator && (
          <span style={{ color: colour, marginLeft: '0.5rem', fontSize: '1rem' }}>{indicator}</span>
        )}
      </div>
    </div>
  );
}
