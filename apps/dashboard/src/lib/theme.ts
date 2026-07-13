export const dashboardTheme = Object.freeze({
  severity: {
    critical: 'var(--brick-red)',
    high: 'var(--brick-red)',
    medium: 'var(--dull-amber)',
    low: 'var(--dull-amber)',
    info: 'var(--ghost-grey)',
  },
  status: {
    pass: 'var(--edda)',
    fail: 'var(--brick-red)',
    warn: 'var(--dull-amber)',
    info: 'var(--ghost-grey)',
    unavailable: 'var(--ghost-grey)',
  },
  chart: [
    'var(--anvil)',
    'var(--edda)',
    'var(--dull-amber)',
    'var(--brick-red)',
    'var(--ghost-grey)',
  ],
});

export type DashboardSeverity = keyof typeof dashboardTheme.severity;
export type DashboardStatus = keyof typeof dashboardTheme.status;
