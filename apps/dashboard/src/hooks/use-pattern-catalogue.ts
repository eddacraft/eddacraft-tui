import { useQuery } from '@tanstack/react-query';

import { useDashboardApi } from '@/api/query-client';
import { dashboardQueryKeys } from '@/api/query-keys';

export function usePatternCatalogue() {
  const api = useDashboardApi();
  return useQuery({
    queryKey: dashboardQueryKeys.patterns.catalogue(),
    queryFn: () => api.getPatternCatalogue(),
  });
}
