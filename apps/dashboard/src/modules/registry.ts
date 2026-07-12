import { FileText, ShieldCheck } from 'lucide-react';

import type { DashboardModuleManifest } from './manifest';

export class DuplicateDashboardModuleError extends Error {
  constructor(id: string) {
    super(`dashboard module identifier is already registered: ${id}`);
    this.name = 'DuplicateDashboardModuleError';
  }
}

export class UnknownDashboardModuleError extends Error {
  constructor(id: string) {
    super(`dashboard module is not registered: ${id}`);
    this.name = 'UnknownDashboardModuleError';
  }
}

export interface DashboardModuleRegistry {
  readonly manifests: readonly DashboardModuleManifest[];
  find: (id: string) => DashboardModuleManifest | undefined;
  require: (id: string) => DashboardModuleManifest;
}

export function createModuleRegistry(
  manifests: readonly DashboardModuleManifest[]
): DashboardModuleRegistry {
  const byId = new Map<string, DashboardModuleManifest>();
  for (const manifest of manifests) {
    if (byId.has(manifest.id)) {
      throw new DuplicateDashboardModuleError(manifest.id);
    }
    byId.set(manifest.id, Object.freeze(manifest));
  }

  const registered = Object.freeze([...byId.values()]);
  return Object.freeze({
    manifests: registered,
    find: (id: string) => byId.get(id),
    require: (id: string) => {
      const manifest = byId.get(id);
      if (!manifest) {
        throw new UnknownDashboardModuleError(id);
      }
      return manifest;
    },
  });
}

export const dashboardModuleRegistry = createModuleRegistry([
  {
    id: 'protection',
    navigation: {
      label: 'Protection',
      path: '/',
      icon: ShieldCheck,
    },
    routes: [{ id: 'overview', path: '/', resource: 'protection-overview' }],
    queryBindings: ['protection-overview'],
    renderers: ['ProtectionOverview'],
    resources: [
      { id: 'runs', label: 'Latest runs', search: { view: 'runs' } },
      { id: 'warnings', label: 'Active warnings', search: { view: 'warnings' } },
      {
        id: 'evidence',
        label: 'Evidence inspector',
        search: { view: 'warnings', evidence: 'warning-api-key' },
      },
    ],
  },
  {
    id: 'plans',
    navigation: { label: 'Plans', path: '/plans', icon: FileText },
    routes: [{ id: 'index', path: '/plans', resource: 'plans' }],
    queryBindings: ['plans', 'plan-detail'],
    renderers: ['PlanDriverBoundary'],
    resources: [{ id: 'index', label: 'Plan Driver' }],
  },
]);
