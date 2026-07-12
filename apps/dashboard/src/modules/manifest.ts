import type { LucideIcon } from 'lucide-react';

export interface DashboardActionRequestDescriptor {
  readonly id: string;
  readonly label: string;
  readonly capability: string;
}

export interface DashboardModuleNavigation {
  readonly label: string;
  readonly path: '/';
  readonly icon?: LucideIcon;
}

export interface DashboardModuleManifest {
  readonly id: string;
  readonly navigation: DashboardModuleNavigation;
  readonly queryBindings: readonly string[];
  readonly renderers: readonly string[];
  readonly actionRequests?: readonly DashboardActionRequestDescriptor[];
}
