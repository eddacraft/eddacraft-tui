import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createContext, use, useState, type ReactNode } from 'react';

import { dashboardApi, type DashboardApi } from '@/api/client';
import { protectionOverviewFixture } from '@/api/fixtures';
import { dashboardQueryKeys } from '@/api/query-keys';

const patternCatalogueFixture = {
  schema_version: 'anvil.dashboard.patterns.v1' as const,
  data_state: 'complete' as const,
  source_message: 'Fixture catalogue',
  patterns: [
    {
      id: 'AP-001',
      title: 'Broad eslint-disable added',
      family: 'guardrail-suppression',
      severity: 'warning',
      enabled: true,
      instance_count: 0,
      description: 'Disables lint rules broadly.',
    },
  ],
};

const fixtureApi: DashboardApi = {
  getProtectionOverview: async () => protectionOverviewFixture,
  getPatternCatalogue: async () => patternCatalogueFixture,
  listPlans: async () => [],
  getPlan: async () => {
    throw new Error('No plan fixture selected');
  },
};
const defaultApi = import.meta.env.MODE === 'test' ? fixtureApi : dashboardApi;
const DashboardApiContext = createContext<DashboardApi>(defaultApi);

export function useDashboardApi() {
  return use(DashboardApiContext);
}

export function DashboardQueryProvider({
  api = defaultApi,
  children,
  queryClient: providedClient,
}: {
  api?: DashboardApi;
  children: ReactNode;
  queryClient?: QueryClient;
}) {
  const [ownedClient] = useState(() => {
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false, staleTime: 15_000 } },
    });
    if (import.meta.env.MODE === 'test' && api === defaultApi) {
      client.setQueryData(dashboardQueryKeys.protection.overview(), protectionOverviewFixture);
    }
    return client;
  });
  return (
    <DashboardApiContext value={api}>
      <QueryClientProvider client={providedClient ?? ownedClient}>{children}</QueryClientProvider>
    </DashboardApiContext>
  );
}
