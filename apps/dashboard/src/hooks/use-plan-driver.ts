import { useQuery } from '@tanstack/react-query';

import { useDashboardApi } from '@/api/query-client';
import { dashboardQueryKeys } from '@/api/query-keys';

export function usePlans() {
  const api = useDashboardApi();
  return useQuery({ queryKey: dashboardQueryKeys.plans.all(), queryFn: () => api.listPlans() });
}

export function usePlan(id: string | undefined) {
  const api = useDashboardApi();
  return useQuery({
    enabled: Boolean(id),
    queryKey: dashboardQueryKeys.plans.detail(id ?? ''),
    queryFn: () => api.getPlan(id ?? ''),
  });
}
