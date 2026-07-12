import { ArrowRight, Check, ShieldCheck, ShieldOff, TriangleAlert } from 'lucide-react';

import type { components } from '@/api/generated/openapi';
import { Button } from '@/components/ui/button';

type Overview = components['schemas']['ProtectionOverview'];
type Warning = components['schemas']['WarningSummary'];

interface ProtectionSummaryProps {
  overview: Overview;
  warning?: Warning;
  onInspectAttention: () => void;
}

function freshness(observedAt: number | null) {
  if (!observedAt) return 'Not observed';
  return new Date(observedAt * 1000).toLocaleString('en-GB', { timeZone: 'UTC' });
}

export function ProtectionSummary({
  overview,
  warning,
  onInspectAttention,
}: ProtectionSummaryProps) {
  const active = overview.save_time?.active === true;
  const latest = overview.latest_run;
  return (
    <div className="protection-summary-stack">
      <section aria-labelledby="protection-summary-title" className="protection-summary">
        <h2 className="sr-only" id="protection-summary-title">
          Protection summary
        </h2>
        <div className="summary-primary">
          <span aria-hidden="true" className={`summary-icon ${active ? 'summary-icon-green' : ''}`}>
            {active ? <ShieldCheck /> : <ShieldOff />}
          </span>
          <div>
            <strong>
              {active ? 'Save-time protection active' : 'Save-time protection not observed'}
            </strong>
            <span>
              {active ? <Check aria-hidden="true" /> : null}
              {overview.save_time?.state ?? 'No live state'}
            </span>
          </div>
        </div>
        <dl className="summary-facts">
          <div>
            <dt>Last run</dt>
            <dd>{latest?.label ?? 'No run recorded'}</dd>
            <dd className="summary-subvalue">
              {latest ? `${latest.warning_count} warnings` : 'Waiting for local evidence'}
            </dd>
          </div>
          <div>
            <dt>Freshness</dt>
            <dd>{freshness(overview.observed_at_unix)}</dd>
            <dd className="summary-subvalue">
              Data state:{' '}
              {overview.data_state === 'complete'
                ? 'Full'
                : overview.data_state === 'partial'
                  ? 'Partial'
                  : 'Empty'}
            </dd>
          </div>
        </dl>
      </section>

      {warning ? (
        <section aria-labelledby="next-attention-title" className="next-attention">
          <span aria-hidden="true" className="summary-icon summary-icon-red">
            <TriangleAlert />
          </span>
          <div className="attention-copy">
            <p className="eyebrow" id="next-attention-title">
              Next attention
            </p>
            <strong>
              <span className="severity severity-high">{warning.severity.toUpperCase()}</span>
              {warning.rule}
            </strong>
            <code>
              {warning.file_path ?? 'Workspace'}:{warning.line ?? '—'}
            </code>
          </div>
          <dl className="attention-facts">
            <div>
              <dt>First seen</dt>
              <dd>{warning.age_label}</dd>
            </div>
            <div>
              <dt>Category</dt>
              <dd>{warning.category}</dd>
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
            Inspect evidence <ArrowRight aria-hidden="true" data-icon="inline-end" />
          </Button>
        </section>
      ) : null}
    </div>
  );
}
