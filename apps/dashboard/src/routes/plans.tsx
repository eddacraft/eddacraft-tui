import { QueryBoundary } from '@/components/query-boundary';
import { usePlan, usePlans } from '@/hooks/use-plan-driver';
import { PlanDetailView } from '@/modules/plans/plan-detail';
import { PlanList } from '@/modules/plans/plan-list';

export function DashboardPlansRoute() {
  const plans = usePlans();
  return (
    <div className="plan-driver">
      <header className="protection-heading">
        <p className="eyebrow">APS_DOGFOOD</p>
        <h1 id="plan-driver-title">Plan Driver</h1>
        <p>Read-only plan status, readiness and validation contracts.</p>
      </header>
      <QueryBoundary loadingLabel="Loading plans" query={plans}>
        {(items) => <PlanList plans={items} />}
      </QueryBoundary>
    </div>
  );
}

export function DashboardPlanDetailRoute({ id }: { id: string }) {
  const plan = usePlan(id);
  return (
    <QueryBoundary loadingLabel="Loading selected plan" query={plan}>
      {(detail) => <PlanDetailView detail={detail} />}
    </QueryBoundary>
  );
}
