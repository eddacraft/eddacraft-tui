import { useState } from 'react';

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

export function ProtectionOverview() {
  const [selectedRunId, setSelectedRunId] = useState(latestRun.id);
  const [selectedWarningId, setSelectedWarningId] = useState(nextAttention.id);
  const selectedWarning =
    protectionWarnings.find((warning) => warning.id === selectedWarningId) ?? nextAttention;

  const selectRun = (run: ProtectionRun) => setSelectedRunId(run.id);
  const selectWarning = (warning: ProtectionWarning, moveFocus = false) => {
    setSelectedWarningId(warning.id);
    if (moveFocus) focusInspector();
  };

  return (
    <div className="protection-overview">
      <header className="protection-heading">
        <p className="eyebrow">Workspace protection</p>
        <h1 id="protection-title" tabIndex={-1}>
          Protection overview
        </h1>
        <p>
          Read-only summary of what Anvil protected on save, what needs attention, and the
          deterministic evidence behind each finding.
        </p>
      </header>

      <ProtectionSummary
        onInspectAttention={() => {
          setSelectedWarningId(nextAttention.id);
          focusInspector();
        }}
      />

      <div className="protection-grid">
        <Tabs className="protection-tabs" defaultValue="runs">
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
