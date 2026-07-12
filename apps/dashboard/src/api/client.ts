import createClient from 'openapi-fetch';

import { protectionOverviewFixture } from '@/api/fixtures';
import type { components, paths } from '@/api/generated/openapi';

type ProtectionOverview = components['schemas']['ProtectionOverview'];
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
  listPlans: () => Promise<PlanSummary[]>;
  getPlan: (id: string) => Promise<PlanDetail>;
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

export const dashboardApi: DashboardApi = {
  async getProtectionOverview() {
    try {
      return unwrap(await client.GET('/api/v1/protection'));
    } catch (error) {
      if (import.meta.env.DEV && error instanceof TypeError) return protectionOverviewFixture;
      throw error;
    }
  },
  async listPlans() {
    return unwrap(await client.GET('/api/v1/plans'));
  },
  async getPlan(id) {
    return unwrap(await client.GET('/api/v1/plans/{id}', { params: { path: { id } } }));
  },
};
