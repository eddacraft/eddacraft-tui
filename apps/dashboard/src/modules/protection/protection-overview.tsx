import { useNavigate, useSearch } from '@tanstack/react-router';
import { useState } from 'react';

import { QueryBoundary } from '@/components/query-boundary';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { EvidenceInspector } from '@/modules/protection/evidence-inspector';
import {
  latestRun,
  nextAttention,
  protectionWarnings,
  type ProtectionRun,
  type ProtectionWarning,
} from '@/modules/protection/fixture';
import { ProtectionSummary } from '@/modules/protection/protection-summary';
import { useProtectionOverview } from '@/hooks/use-protection-overview';
import {
  AffectedFilesTable,
  RunsTable,
  WarningsTable,
} from '@/modules/protection/protection-tables';

function focusInspector() {
  requestAnimationFrame(() => {
    document.querySelector<HTMLElement>('#evidence-inspector')?.focus({ preventScroll: true });
    document
      .querySelector('#evidence-inspector')
      ?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  });
}

function ProtectionOverviewContent({
  sourceMessage,
  state,
}: {
  sourceMessage: string;
  state: string;
}) {
  const search = useSearch({ from: '/' });
  const navigate = useNavigate({ from: '/' });
  const [selectedRunId, setSelectedRunId] = useState(latestRun.id);
  const selectedWarningId = search.evidence ?? nextAttention.id;
  const filteredWarnings =
    search.severity === 'all'
      ? protectionWarnings
      : protectionWarnings.filter((warning) => warning.severity.toLowerCase() === search.severity);
  const selectedWarning =
    protectionWarnings.find((warning) => warning.id === selectedWarningId) ?? nextAttention;

  const selectRun = (run: ProtectionRun) => setSelectedRunId(run.id);
  const selectWarning = (warning: ProtectionWarning, moveFocus = false) => {
    void navigate({
      search: (previous) => ({ ...previous, evidence: warning.id, view: 'warnings' }),
    });
    if (moveFocus) focusInspector();
  };

  return (
    <div className="protection-overview" data-dashboard-state={state}>
      <header className="protection-heading">
        <p className="eyebrow">Workspace protection</p>
        <h1 id="protection-title" tabIndex={-1}>
          Protection overview
        </h1>
        <p>
          Read-only summary of what Anvil protected on save, what needs attention, and the
          deterministic evidence behind each finding.
        </p>
        <p className="sr-only">{sourceMessage}</p>
      </header>

      <ProtectionSummary
        onInspectAttention={() => {
          void navigate({
            search: (previous) => ({ ...previous, evidence: nextAttention.id, view: 'warnings' }),
          });
          focusInspector();
        }}
      />

      <div className="protection-grid">
        <Tabs
          className="protection-tabs"
          onValueChange={(view) =>
            void navigate({
              search: (previous) => ({
                ...previous,
                view: view === 'warnings' ? 'warnings' : 'runs',
              }),
            })
          }
          value={search.view}
        >
          <TabsList
            aria-label="Protection activity"
            className="protection-tabs-list"
            variant="line"
          >
            <TabsTrigger value="runs">Runs</TabsTrigger>
            <TabsTrigger value="warnings">Warnings (12)</TabsTrigger>
          </TabsList>

          <TabsContent className="panel activity-panel" forceMount value="runs">
            <header className="panel-header">
              <div>
                <h2>Latest runs</h2>
                <p>Most recent save-time scans</p>
              </div>
              <span className="panel-count">5 runs</span>
            </header>
            <RunsTable onSelectRun={selectRun} selectedRunId={selectedRunId} />
          </TabsContent>

          <TabsContent className="panel activity-panel" forceMount value="warnings">
            <header className="panel-header">
              <div>
                <h2>Active warnings (12)</h2>
                <p>Ordered by severity and recency</p>
              </div>
              <span className="panel-count panel-count-warning">12 open</span>
            </header>
            <WarningsTable
              onSelectWarning={(warning) => selectWarning(warning)}
              selectedWarningId={selectedWarningId}
              warnings={filteredWarnings}
            />
          </TabsContent>
        </Tabs>

        <EvidenceInspector warning={selectedWarning} />
      </div>

      <section aria-labelledby="affected-files-title" className="panel affected-files-panel">
        <header className="panel-header">
          <div>
            <h2 id="affected-files-title">Affected files (6)</h2>
            <p>Files with active warnings in the latest run</p>
          </div>
          <span className="panel-count">6 files</span>
        </header>
        <AffectedFilesTable
          onSelectWarning={(warning) => selectWarning(warning, true)}
          selectedWarningId={selectedWarningId}
          warnings={filteredWarnings}
        />
      </section>

      <p className="mobile-connection-note">
        <span>Local only</span>
        <span>Read-only</span>
        <span>No network calls</span>
      </p>
    </div>
  );
}

export function ProtectionOverview() {
  const query = useProtectionOverview();
  return (
    <QueryBoundary loadingLabel="Loading protection overview" query={query}>
      {(overview) => (
        <ProtectionOverviewContent
          sourceMessage={overview.source_message}
          state={overview.data_state}
        />
      )}
    </QueryBoundary>
  );
}
