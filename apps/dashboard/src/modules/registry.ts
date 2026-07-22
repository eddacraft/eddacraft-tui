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
      label: 'Overview',
      path: '/',
      glyph: 'action',
    },
    routes: [{ id: 'overview', path: '/', resource: 'protection-overview' }],
    queryBindings: ['protection-overview'],
    renderers: ['ProtectionOverview'],
    resources: [
      { id: 'runs', label: 'Latest runs', search: { view: 'runs' } },
      { id: 'warnings', label: 'Active warnings', search: { view: 'warnings' } },
    ],
  },
  {
    id: 'gates',
    navigation: { label: 'Gates', path: '/gates', glyph: 'history' },
    routes: [
      { id: 'index', path: '/gates', resource: 'protection-overview' },
      { id: 'detail', path: '/gates/$id', resource: 'protection-overview' },
    ],
    queryBindings: ['protection-overview'],
    renderers: ['GateHistoryTable', 'GateDetailPage'],
    resources: [{ id: 'index', label: 'Gate history' }],
  },
  {
    id: 'warnings',
    navigation: { label: 'Warnings', path: '/warnings', glyph: 'context' },
    routes: [
      { id: 'index', path: '/warnings', resource: 'protection-overview' },
      { id: 'breakdown', path: '/warnings/breakdown', resource: 'protection-overview' },
      { id: 'patterns', path: '/warnings/patterns', resource: 'pattern-catalogue' },
    ],
    queryBindings: ['protection-overview', 'pattern-catalogue'],
    renderers: ['WarningTable', 'WarningCharts', 'PatternRegistry'],
    resources: [
      { id: 'index', label: 'Active warnings' },
      { id: 'breakdown', label: 'Breakdown' },
      { id: 'patterns', label: 'Patterns' },
    ],
  },
  {
    id: 'plans',
    navigation: { label: 'Plans', path: '/plans', glyph: 'history' },
    routes: [
      { id: 'index', path: '/plans', resource: 'plans' },
      { id: 'detail', path: '/plans/$id', resource: 'plan-detail' },
    ],
    queryBindings: ['plans', 'plan-detail'],
    renderers: ['PlanDriverBoundary'],
    resources: [{ id: 'index', label: 'Plan Driver' }],
  },
]);
