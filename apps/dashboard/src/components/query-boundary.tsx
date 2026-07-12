import type { UseQueryResult } from '@tanstack/react-query';
import type { ReactNode } from 'react';

import { LoadingSkeleton } from '@/components/primitives/loading-skeleton';
import { Button } from '@/components/ui/button';

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
      <section className="query-error" role="alert">
        <strong>Dashboard data unavailable</strong>
        <p>
          {code}: {query.error.message}
        </p>
        <p>If this browser is offline, reconnect it.</p>
        <p>Start or restart the local dashboard server, then retry this request.</p>
        <Button onClick={() => void query.refetch()} size="sm" type="button" variant="outline">
          Retry
        </Button>
      </section>
    );
  }
  return children(query.data);
}
