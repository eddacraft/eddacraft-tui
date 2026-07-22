import { useState } from 'react';
import { useSearch } from '@tanstack/react-router';

import { QueryBoundary } from '@/components/query-boundary';
import { usePatternCatalogue } from '@/hooks/use-pattern-catalogue';
import { useProtectionOverview } from '@/hooks/use-protection-overview';
import { WarningCharts } from '@/modules/core/warnings/warning-charts';
import { WarningDetailPanel } from '@/modules/core/warnings/warning-detail-panel';
import { WarningTable } from '@/modules/core/warnings/warning-table';
import { PatternRegistry } from '@/modules/core/warnings/pattern-registry';
import type { components } from '@/api/generated/openapi';

type Warning = components['schemas']['WarningSummary'];

export function DashboardWarningsRoute() {
  const query = useProtectionOverview();
  const search = useSearch({ strict: false }) as { evidence?: string };
  const [selected, setSelected] = useState<Warning | undefined>();
  const [open, setOpen] = useState(Boolean(search.evidence));

  return (
    <QueryBoundary query={query} loadingLabel="Warnings">
      {(overview) => {
        const initial =
          selected ??
          overview.warnings.find(
            (warning) => warning.id === search.evidence || warning.evidence_id === search.evidence
          );
        return (
          <section className="warnings-page">
            <header className="protection-heading">
              <p className="eyebrow">WARNINGS</p>
              <h1>Active warnings</h1>
              <p>Warnings derived from the latest local gate snapshot.</p>
            </header>
            <WarningTable
              onSelect={(warning) => {
                setSelected(warning);
                setOpen(true);
              }}
              selectedId={initial?.id}
              warnings={overview.warnings}
            />
            <WarningDetailPanel onOpenChange={setOpen} open={open} warning={initial} />
          </section>
        );
      }}
    </QueryBoundary>
  );
}

export function DashboardWarningsBreakdownRoute() {
  const query = useProtectionOverview();
  return (
    <QueryBoundary query={query} loadingLabel="Warning breakdown">
      {(overview) => (
        <section className="warnings-breakdown-page">
          <header className="protection-heading">
            <p className="eyebrow">WARNINGS</p>
            <h1>Warning breakdown</h1>
            <p>Distributions from the latest active warning set.</p>
          </header>
          <WarningCharts warnings={overview.warnings} />
        </section>
      )}
    </QueryBoundary>
  );
}

export function DashboardWarningsPatternsRoute() {
  const patterns = usePatternCatalogue();
  const protection = useProtectionOverview();
  return (
    <QueryBoundary query={patterns} loadingLabel="Pattern registry">
      {(catalogue) => (
        <QueryBoundary query={protection} loadingLabel="Pattern registry warnings">
          {(overview) => (
            <section className="warnings-patterns-page">
              <header className="protection-heading">
                <p className="eyebrow">PATTERNS</p>
                <h1>Anti-pattern registry</h1>
                <p>{catalogue.source_message}</p>
              </header>
              <PatternRegistry catalogue={catalogue} warnings={overview.warnings} />
            </section>
          )}
        </QueryBoundary>
      )}
    </QueryBoundary>
  );
}
