import { describe, expect, it } from 'vitest';

import type { DashboardModuleManifest } from './manifest';
import {
  DuplicateDashboardModuleError,
  UnknownDashboardModuleError,
  createModuleRegistry,
} from './registry';

const protectionModule: DashboardModuleManifest = {
  id: 'protection',
  navigation: {
    label: 'Protection',
    path: '/',
  },
  queryBindings: ['protection-overview'],
  renderers: ['ProtectionOverview'],
};

describe('dashboard module registry', () => {
  it('rejects duplicate module identifiers', () => {
    expect(() => createModuleRegistry([protectionModule, protectionModule])).toThrow(
      DuplicateDashboardModuleError
    );
  });

  it('fails closed when a module identifier is unknown', () => {
    const registry = createModuleRegistry([protectionModule]);

    expect(() => registry.require('missing')).toThrow(UnknownDashboardModuleError);
    expect(registry.find('missing')).toBeUndefined();
  });

  it('exposes passive action-request descriptors without executable authority', () => {
    const registry = createModuleRegistry([
      {
        ...protectionModule,
        actionRequests: [{ id: 'rescan', label: 'Request a rescan', capability: 'scan' }],
      },
    ]);

    const manifest = registry.require('protection');
    expect(manifest.actionRequests?.[0]).toEqual({
      id: 'rescan',
      label: 'Request a rescan',
      capability: 'scan',
    });
    expect(
      Object.values(manifest.actionRequests?.[0] ?? {}).some((value) => typeof value === 'function')
    ).toBe(false);
  });
});
