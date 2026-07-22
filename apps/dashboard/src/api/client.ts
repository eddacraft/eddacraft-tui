import createClient from 'openapi-fetch';

import type { components, paths } from '@/api/generated/openapi';

type ProtectionOverview = components['schemas']['ProtectionOverview'];
type PatternCatalogue = components['schemas']['PatternCatalogue'];
type PlanSummary = components['schemas']['PlanSummary'];
type PlanDetail = components['schemas']['PlanDetail'];

export class DashboardApiError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = 'DashboardApiError';
    this.code = code;
  }
}

export interface DashboardApi {
  getProtectionOverview: () => Promise<ProtectionOverview>;
  getPatternCatalogue: () => Promise<PatternCatalogue>;
  listPlans: () => Promise<PlanSummary[]>;
  getPlan: (id: string) => Promise<PlanDetail>;
}

interface OpenApiClient {
  GET: ReturnType<typeof createClient<paths>>['GET'];
}

const client = createClient<paths>({ baseUrl: '/' });

function unwrap<T>(result: { data?: T; error?: unknown }): T {
  if (result.data !== undefined) return result.data;
  const error = result.error as { code?: string; message?: string } | undefined;
  throw new DashboardApiError(
    error?.code ?? 'dashboard-api-error',
    error?.message ?? 'Dashboard API request failed'
  );
}

export function createDashboardApi(apiClient: OpenApiClient): DashboardApi {
  return {
    async getProtectionOverview() {
      return unwrap(await apiClient.GET('/api/v1/protection'));
    },
    async getPatternCatalogue() {
      return unwrap(await apiClient.GET('/api/v1/patterns'));
    },
    async listPlans() {
      return unwrap(await apiClient.GET('/api/v1/plans'));
    },
    async getPlan(id) {
      return unwrap(await apiClient.GET('/api/v1/plans/{id}', { params: { path: { id } } }));
    },
  };
}

export const dashboardApi = createDashboardApi(client);
