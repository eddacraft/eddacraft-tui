import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createContext, use, useState, type ReactNode } from 'react';

import { dashboardApi, type DashboardApi } from '@/api/client';
import { protectionOverviewFixture } from '@/api/fixtures';
import { dashboardQueryKeys } from '@/api/query-keys';

const fixtureApi: DashboardApi = {
  getProtectionOverview: async () => protectionOverviewFixture,
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
