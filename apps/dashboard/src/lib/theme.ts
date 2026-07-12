export const dashboardTheme = Object.freeze({
  severity: {
    critical: 'var(--red)',
    high: 'var(--red)',
    medium: 'var(--orange)',
    low: 'var(--yellow)',
    info: 'var(--blue)',
  },
  status: {
    pass: 'var(--green)',
    fail: 'var(--red)',
    warn: 'var(--orange)',
    info: 'var(--blue)',
    unavailable: 'var(--muted)',
  },
  chart: ['var(--blue)', 'var(--green)', 'var(--orange)', 'var(--yellow)', 'var(--red)'],
});

export type DashboardSeverity = keyof typeof dashboardTheme.severity;
export type DashboardStatus = keyof typeof dashboardTheme.status;
