import { useQuery } from '@tanstack/react-query';

import { useDashboardApi } from '@/api/query-client';
import { dashboardQueryKeys } from '@/api/query-keys';

export function useProtectionHistory() {
  const api = useDashboardApi();
  return useQuery({
    queryKey: dashboardQueryKeys.protection.history(),
    queryFn: () => api.getProtectionHistory(),
  });
}
