import type { components } from '@/api/generated/openapi';
import { Button } from '@/components/ui/button';
import { EvidenceTimeline } from '@/modules/plans/evidence-timeline';

type PlanDetail = components['schemas']['PlanDetail'];

export function PlanDetailView({ detail }: { detail: PlanDetail }) {
  return (
    <article className="plan-detail">
      <header className="protection-heading">
        <p className="eyebrow">{detail.summary.scope} :: APS_DOGFOOD</p>
        <h1>{detail.summary.title}</h1>
        <p>{detail.purpose}</p>
      </header>
      <section className="panel plan-readiness">
        <header className="panel-header">
          <div>
            <h2>Readiness</h2>
            <p>
              {detail.summary.status} · {detail.summary.progress}
            </p>
          </div>
          <Button disabled={!detail.actions_enabled} type="button">
            Request approval
          </Button>
        </header>
        <p>{detail.action_message}</p>
      </section>
      <EvidenceTimeline entries={detail.timeline} />
    </article>
  );
}
