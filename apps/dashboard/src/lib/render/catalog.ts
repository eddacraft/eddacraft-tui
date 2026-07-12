import {
  catalog,
  getComponentNames,
  registry,
  validateSpec,
  type Spec,
  type ValidationResult,
} from '@eddacraft/render';

export const dashboardCatalog = catalog;
export const dashboardRenderRegistry = registry;
export const getDashboardComponentNames = getComponentNames;
export const validateDashboardSpec = validateSpec;

export type { Spec as DashboardSpec, ValidationResult as DashboardSpecValidationResult };
