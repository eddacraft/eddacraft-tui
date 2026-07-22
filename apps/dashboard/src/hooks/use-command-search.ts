import type { DashboardModuleRegistry } from '@/modules/registry';

export interface DashboardCommandEntry {
  readonly id: string;
  readonly label: string;
  readonly group: string;
  readonly to: '/' | '/plans' | '/gates' | '/warnings';
  readonly search?: Readonly<Record<string, string>>;
}

export function createCommandEntries(registry: DashboardModuleRegistry): DashboardCommandEntry[] {
  return registry.manifests.flatMap((manifest) => [
    {
      id: `module:${manifest.id}`,
      label: manifest.navigation.label,
      group: 'Modules',
      to: manifest.navigation.path,
    },
    ...(manifest.resources ?? []).map((resource) => ({
      id: `resource:${manifest.id}:${resource.id}`,
      label: resource.label,
      group: manifest.navigation.label,
      to: manifest.navigation.path,
      search: resource.search,
    })),
  ]);
}
