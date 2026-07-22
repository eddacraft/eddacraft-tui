import { QueryBoundary } from '@/components/query-boundary';
import { useProtectionOverview } from '@/hooks/use-protection-overview';
import { GateHistoryTable } from '@/modules/core/gates/gate-history-table';
import { GateDetailPage } from '@/modules/core/gates/gate-detail-page';

export function DashboardGatesRoute() {
  const query = useProtectionOverview();
  return (
    <QueryBoundary query={query} loadingLabel="Gates">
      {(overview) => (
        <section className="gates-page">
          <header className="protection-heading">
            <p className="eyebrow">GATES</p>
            <h1>Gate history</h1>
            <p>Latest local gate runs proven by the dashboard protection overview.</p>
          </header>
          <GateHistoryTable runs={overview.recent_runs} />
        </section>
      )}
    </QueryBoundary>
  );
}

export function DashboardGateDetailRoute({ id }: { id: string }) {
  const query = useProtectionOverview();
  return (
    <QueryBoundary query={query} loadingLabel="Gate detail">
      {(overview) => <GateDetailPage id={id} overview={overview} />}
    </QueryBoundary>
  );
}
