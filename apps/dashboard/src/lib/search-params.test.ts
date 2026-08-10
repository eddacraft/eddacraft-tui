import { describe, expect, it } from 'vitest';

import { dashboardSearchSchema } from '@/lib/search-params';

describe('dashboard route search state', () => {
  it('accepts addressable filters and evidence selection', () => {
    expect(
      dashboardSearchSchema.parse({
        severity: 'high',
        view: 'warnings',
        evidence: 'warning-api-key',
      })
    ).toEqual({ severity: 'high', view: 'warnings', evidence: 'warning-api-key' });
  });

  it('preserves the recognised critical severity', () => {
    expect(dashboardSearchSchema.parse({ severity: 'critical' })).toEqual({
      severity: 'critical',
      view: 'runs',
    });
  });

  it('falls back safely when a shared URL contains invalid values', () => {
    expect(
      dashboardSearchSchema.parse({ severity: 'catastrophic', view: 'raw', evidence: 42 })
    ).toEqual({
      severity: 'all',
      view: 'runs',
    });
  });
});
