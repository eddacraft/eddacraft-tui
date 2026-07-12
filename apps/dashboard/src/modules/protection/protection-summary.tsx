import { ArrowRight, Check, ShieldCheck, TriangleAlert } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { latestRun, nextAttention, workspace } from '@/modules/protection/fixture';

interface ProtectionSummaryProps {
  onInspectAttention: () => void;
}

export function ProtectionSummary({ onInspectAttention }: ProtectionSummaryProps) {
  return (
    <div className="protection-summary-stack">
      <section aria-labelledby="protection-summary-title" className="protection-summary">
        <h2 className="sr-only" id="protection-summary-title">
          Protection summary
        </h2>
        <div className="summary-primary">
          <span aria-hidden="true" className="summary-icon summary-icon-green">
            <ShieldCheck />
          </span>
          <div>
            <strong>Save-time protection active</strong>
            <span>
              <Check aria-hidden="true" /> New violations only
            </span>
          </div>
        </div>
        <dl className="summary-facts">
          <div>
            <dt>Last run</dt>
            <dd>
              <span className="result-indicator result-indicator-issues" /> Completed with issues
            </dd>
            <dd className="summary-subvalue">{latestRun.violations} violations</dd>
          </div>
          <div>
            <dt>Freshness</dt>
            <dd>{workspace.freshness}</dd>
            <dd className="summary-subvalue">{workspace.refreshedAt}</dd>
          </div>
        </dl>
      </section>

      <section aria-labelledby="next-attention-title" className="next-attention">
        <span aria-hidden="true" className="summary-icon summary-icon-red">
          <TriangleAlert />
        </span>
        <div className="attention-copy">
          <p className="eyebrow" id="next-attention-title">
            Next attention
          </p>
          <strong>
            <span className="severity severity-high">HIGH</span> Hard-coded API key detected
          </strong>
          <code>
            {nextAttention.file}:{nextAttention.line}
          </code>
        </div>
        <dl className="attention-facts">
          <div>
            <dt>First seen</dt>
            <dd>{nextAttention.age} ago</dd>
          </div>
          <div>
            <dt>Category</dt>
            <dd>{nextAttention.category}</dd>
          </div>
        </dl>
        <Button
          aria-controls="evidence-inspector"
          className="attention-action"
          onClick={onInspectAttention}
          size="sm"
          type="button"
          variant="ghost"
        >
          Inspect evidence <ArrowRight aria-hidden="true" />
        </Button>
      </section>
    </div>
  );
}
