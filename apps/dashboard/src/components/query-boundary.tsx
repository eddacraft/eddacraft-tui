import type { UseQueryResult } from '@tanstack/react-query';
import type { ReactNode } from 'react';

import { LoadingSkeleton } from '@/components/primitives/loading-skeleton';

export function QueryBoundary<T>({
  children,
  loadingLabel,
  query,
}: {
  children: (data: T) => ReactNode;
  loadingLabel: string;
  query: UseQueryResult<T, Error>;
}) {
  if (query.isPending) return <LoadingSkeleton label={loadingLabel} />;
  if (query.isError) {
    const code = 'code' in query.error ? String(query.error.code) : 'dashboard-query-error';
    return (
      <p role="alert">
        {code}: {query.error.message}
      </p>
    );
  }
  return children(query.data);
}
