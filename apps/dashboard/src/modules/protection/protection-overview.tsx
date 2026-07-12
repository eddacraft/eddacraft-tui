import { useNavigate, useSearch } from '@tanstack/react-router';
import { useState } from 'react';

import type { components } from '@/api/generated/openapi';
import { QueryBoundary } from '@/components/query-boundary';
import { EmptyState } from '@/components/primitives/empty-state';
import { Badge } from '@/components/ui/badge';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useProtectionOverview } from '@/hooks/use-protection-overview';
import type { DashboardSearch } from '@/lib/search-params';
import { EvidenceInspector } from '@/modules/protection/evidence-inspector';
import { ProtectionSummary } from '@/modules/protection/protection-summary';

type Overview = components['schemas']['ProtectionOverview'];
type ProtectionView = DashboardSearch['view'];
type SeverityFilter = DashboardSearch['severity'];

const severityOptions: readonly SeverityFilter[] = ['all', 'high', 'medium', 'low'];

export function ProtectionOverviewContent({
  overview,
  initialEvidence,
  onEvidenceChange,
  onSeverityChange,
  onViewChange,
  severity = 'all',
  view = 'runs',
}: {
  overview: Overview;
  initialEvidence?: string;
  onEvidenceChange?: (id: string) => void;
  onSeverityChange?: (severity: SeverityFilter) => void;
  onViewChange?: (view: ProtectionView) => void;
  severity?: SeverityFilter;
  view?: ProtectionView;
}) {
  const fallbackEvidence = overview.next_attention?.evidence_id ?? overview.warnings[0]?.id;
  const [localSelectedId, setLocalSelectedId] = useState(initialEvidence ?? fallbackEvidence);
  const evidenceIsControlled = initialEvidence !== undefined || onEvidenceChange !== undefined;
  const selectedId = evidenceIsControlled ? (initialEvidence ?? fallbackEvidence) : localSelectedId;
  const filteredWarnings =
    severity === 'all'
      ? overview.warnings
      : overview.warnings.filter((warning) => warning.severity === severity);
  const selected =
    filteredWarnings.find(
      (warning) => warning.id === selectedId || warning.evidence_id === selectedId
    ) ?? filteredWarnings[0];
  const select = (id: string) => {
    if (!evidenceIsControlled) setLocalSelectedId(id);
    onEvidenceChange?.(id);
  };
  const offline = overview.gaps.some((gap) => gap.component === 'live-protection');

  if (
    overview.data_state === 'unavailable' &&
    overview.recent_runs.length === 0 &&
    overview.warnings.length === 0
  ) {
    return (
      <section className="protection-overview">
        <header className="protection-heading">
          <p className="eyebrow">Workspace protection</p>
          <h1>Protection overview</h1>
        </header>
        <EmptyState
          description={overview.source_message}
          title="No local protection evidence yet"
        />
      </section>
    );
  }

  return (
    <div className="protection-overview" data-dashboard-state={overview.data_state}>
      <header className="protection-heading">
        <p className="eyebrow">Workspace protection</p>
        <h1 id="protection-title" tabIndex={-1}>
          Protection overview
        </h1>
        <p>Read-only local evidence for save-time protection and its next attention item.</p>
        <div className="data-state-row">
          <Badge variant="outline">
            {overview.data_state === 'complete' ? 'Full data' : 'Partial data'}
          </Badge>
          {offline ? <Badge variant="outline">Offline · last-known evidence</Badge> : null}
        </div>
      </header>
      <ProtectionSummary
        overview={overview}
        onInspectAttention={() => selected && select(selected.id)}
        warning={selected}
      />
      <div className="protection-grid">
        <Tabs
          className="protection-tabs"
          onValueChange={(nextView) => {
            if (nextView === 'runs' || nextView === 'warnings') onViewChange?.(nextView);
          }}
          value={view}
        >
          <TabsList
            aria-label="Protection activity"
            className="protection-tabs-list"
            variant="line"
          >
            <TabsTrigger value="runs">Runs</TabsTrigger>
            <TabsTrigger value="warnings">Warnings ({overview.warnings.length})</TabsTrigger>
          </TabsList>
          <TabsContent className="panel activity-panel" forceMount value="runs">
            <header className="panel-header">
              <div>
                <h2>Latest runs</h2>
                <p>Most recent save-time scans</p>
              </div>
              <span className="panel-count">{overview.recent_runs.length} runs</span>
            </header>
            <Table className="operations-table">
              <TableHeader>
                <TableRow>
                  <TableHead>Started</TableHead>
                  <TableHead>Result</TableHead>
                  <TableHead>Warnings</TableHead>
                  <TableHead>Changed</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {overview.recent_runs.map((run) => (
                  <TableRow key={run.id}>
                    <TableCell>{run.started_at}</TableCell>
                    <TableCell>{run.label}</TableCell>
                    <TableCell>{run.warning_count}</TableCell>
                    <TableCell>{run.changed_file_count}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </TabsContent>
          <TabsContent className="panel activity-panel warnings-panel" forceMount value="warnings">
            <header className="panel-header">
              <div>
                <h2>Active warnings ({filteredWarnings.length})</h2>
                <p>Ordered by severity and recency</p>
              </div>
              <label className="severity-filter">
                <span>Severity</span>
                <select
                  aria-label="Filter warnings by severity"
                  onChange={(event) =>
                    onSeverityChange?.(event.currentTarget.value as SeverityFilter)
                  }
                  value={severity}
                >
                  {severityOptions.map((option) => (
                    <option key={option} value={option}>
                      {option === 'all' ? 'All' : option.charAt(0).toUpperCase() + option.slice(1)}
                    </option>
                  ))}
                </select>
              </label>
            </header>
            <Table className="operations-table">
              <TableHeader>
                <TableRow>
                  <TableHead>Severity</TableHead>
                  <TableHead>Rule</TableHead>
                  <TableHead>File</TableHead>
                  <TableHead>Age</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredWarnings.map((warning) => (
                  <TableRow
                    data-selected={warning.id === selected?.id || undefined}
                    key={warning.id}
                  >
                    <TableCell>{warning.severity.toUpperCase()}</TableCell>
                    <TableCell>
                      <button
                        className="table-select-button table-rule"
                        onClick={() => select(warning.id)}
                        type="button"
                      >
                        {warning.rule}
                      </button>
                    </TableCell>
                    <TableCell>
                      <code>
                        {warning.file_path ?? 'Workspace'}:{warning.line ?? '—'}
                      </code>
                    </TableCell>
                    <TableCell>{warning.age_label}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </TabsContent>
        </Tabs>
        <EvidenceInspector warning={selected} />
      </div>
      <section aria-labelledby="affected-files-title" className="panel affected-files-panel">
        <header className="panel-header">
          <div>
            <h2 id="affected-files-title">Affected files ({overview.affected_files.length})</h2>
            <p>Files with active warnings in the latest evidence</p>
          </div>
        </header>
        <Table className="operations-table">
          <TableHeader>
            <TableRow>
              <TableHead>File path</TableHead>
              <TableHead>Warnings</TableHead>
              <TableHead>Highest severity</TableHead>
              <TableHead>Last seen</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {overview.affected_files.map((file) => (
              <TableRow key={file.path}>
                <TableCell>
                  <button
                    className="table-select-button"
                    onClick={() => select(file.warning_id)}
                    type="button"
                  >
                    {file.path}
                  </button>
                </TableCell>
                <TableCell>{file.warning_count}</TableCell>
                <TableCell>{file.highest_severity.toUpperCase()}</TableCell>
                <TableCell>{file.last_seen}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </section>
    </div>
  );
}

export function ProtectionOverview() {
  const query = useProtectionOverview();
  const search = useSearch({ from: '/' });
  const navigate = useNavigate({ from: '/' });
  return (
    <QueryBoundary loadingLabel="Loading protection overview" query={query}>
      {(overview) => (
        <ProtectionOverviewContent
          initialEvidence={search.evidence}
          onEvidenceChange={(evidence) =>
            void navigate({ search: (previous) => ({ ...previous, evidence, view: 'warnings' }) })
          }
          onSeverityChange={(severity) =>
            void navigate({ search: (previous) => ({ ...previous, severity }) })
          }
          onViewChange={(view) => void navigate({ search: (previous) => ({ ...previous, view }) })}
          overview={overview}
          severity={search.severity}
          view={search.view}
        />
      )}
    </QueryBoundary>
  );
}
