import { useQuery } from '@tanstack/react-query';

import { useDashboardApi } from '@/api/query-client';
import { dashboardQueryKeys } from '@/api/query-keys';

export function useProtectionOverview() {
  const api = useDashboardApi();
  return useQuery({
    queryKey: dashboardQueryKeys.protection.overview(),
    queryFn: () => api.getProtectionOverview(),
  });
}
