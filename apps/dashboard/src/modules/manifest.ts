import type { LucideIcon } from 'lucide-react';

export interface DashboardActionRequestDescriptor {
  readonly id: string;
  readonly label: string;
  readonly capability: string;
}

export interface DashboardModuleNavigation {
  readonly label: string;
  readonly path: '/' | '/plans';
  readonly icon?: LucideIcon;
}

export interface DashboardModuleRoute {
  readonly id: string;
  readonly path: '/' | '/plans';
  readonly resource: string;
}

export interface DashboardModuleResource {
  readonly id: string;
  readonly label: string;
  readonly search?: Readonly<Record<string, string>>;
}

export interface DashboardModuleManifest {
  readonly id: string;
  readonly navigation: DashboardModuleNavigation;
  readonly routes: readonly DashboardModuleRoute[];
  readonly queryBindings: readonly string[];
  readonly renderers: readonly string[];
  readonly resources?: readonly DashboardModuleResource[];
  readonly actionRequests?: readonly DashboardActionRequestDescriptor[];
}
