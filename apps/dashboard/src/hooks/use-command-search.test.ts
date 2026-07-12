import { describe, expect, it } from 'vitest';

import { createCommandEntries } from '@/hooks/use-command-search';
import { dashboardModuleRegistry } from '@/modules/registry';

describe('dashboard command registry', () => {
  it('indexes registered modules and their addressable resources', () => {
    const entries = createCommandEntries(dashboardModuleRegistry);

    expect(entries.map((entry) => entry.id)).toContain('module:protection');
    expect(entries.map((entry) => entry.id)).toContain('resource:protection:warnings');
    expect(entries.map((entry) => entry.id)).not.toContain('resource:protection:evidence');
    expect(entries.map((entry) => entry.to)).toContain('/plans');
    expect(entries.map((entry) => entry.id)).not.toContain('resource:plans:detail');
  });
});
