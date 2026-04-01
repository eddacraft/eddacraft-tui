import type { ReactNode } from 'react';

export interface StatusBadgeProps {
  status: 'pass' | 'fail' | 'warn' | 'info';
  label: string;
}

const statusStyles: Record<string, { bg: string; fg: string; symbol: string }> = {
  pass: { bg: 'var(--edda)', fg: 'var(--text-primary)', symbol: '\u2713' },
  fail: { bg: 'var(--anvil)', fg: 'var(--text-primary)', symbol: '\u2717' },
  warn: { bg: '#b8860b', fg: 'var(--text-primary)', symbol: '\u26a0' },
  info: { bg: 'var(--structure)', fg: 'var(--text-primary)', symbol: '\u2139' },
};

export function StatusBadge({ status, label }: StatusBadgeProps): ReactNode {
  const style = statusStyles[status] ?? statusStyles.info;

  return (
    <div
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '0.5rem',
        padding: '0.25rem 0.75rem',
        fontFamily: 'monospace',
        fontSize: '0.875rem',
        backgroundColor: style.bg,
        color: style.fg,
      }}
    >
      <span>{style.symbol}</span>
      <span>{label}</span>
    </div>
  );
}
