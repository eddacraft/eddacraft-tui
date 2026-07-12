import { QueryBoundary } from '@/components/query-boundary';
import { usePlans } from '@/hooks/use-plan-driver';

export function DashboardPlansRoute() {
  const plans = usePlans();
  return (
    <QueryBoundary loadingLabel="Loading plans" query={plans}>
      {(items) => (
        <section aria-labelledby="plan-driver-title">
          <p className="eyebrow">APS dogfood</p>
          <h1 id="plan-driver-title">Plan Driver</h1>
          <p>{items.length} indexed plans available through the read-only generated client.</p>
        </section>
      )}
    </QueryBoundary>
  );
}
