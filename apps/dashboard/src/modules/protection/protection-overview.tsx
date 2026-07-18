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
import { CurrentHealthCards } from '@/modules/core/overview/current-health-cards';
import { EvidenceInspector } from '@/modules/protection/evidence-inspector';
import { ProtectionSummary } from '@/modules/protection/protection-summary';

type Overview = components['schemas']['ProtectionOverview'];
type DataState = components['schemas']['DataState'];
type ProtectionView = DashboardSearch['view'];
type SeverityFilter = DashboardSearch['severity'];

const severityOptions: readonly SeverityFilter[] = ['all', 'high', 'medium', 'low'];

function resourceLabel(label: string, state: DataState, count: number) {
  if (state === 'complete') return `${label} (${count})`;
  if (state === 'partial' && count > 0) return `${label} partial (${count} shown)`;
  return `${label} ${state}`;
}

function gapReason(overview: Overview, components: readonly string[], fallback: string) {
  return overview.gaps.find((gap) => components.includes(gap.component))?.reason ?? fallback;
}

function ResourceStateNotice({
  label,
  reason,
  state,
}: {
  label: string;
  reason: string;
  state: Exclude<DataState, 'complete'>;
}) {
  return (
    <div className="resource-state-notice" role="status">
      <strong>Availability detail</strong>
      <p>{reason}</p>
      <span className="sr-only">
        {label} {state}
      </span>
    </div>
  );
}

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
  const warningsLabel = resourceLabel(
    'Warnings',
    overview.warnings_state,
    overview.warnings.length
  );
  const affectedFilesLabel = resourceLabel(
    'Affected files',
    overview.affected_files_state,
    overview.affected_files.length
  );
  const warningsGap = gapReason(
    overview,
    ['retained-warning-history', 'warnings'],
    overview.warnings_state === 'partial'
      ? 'Only part of the warning history is available.'
      : 'Warning history is not available.'
  );
  const affectedFilesGap = gapReason(
    overview,
    ['affected-files'],
    overview.affected_files_state === 'partial'
      ? 'Only part of the affected-file index is available.'
      : 'Affected files are not available.'
  );

  if (
    overview.data_state === 'unavailable' &&
    overview.recent_runs.length === 0 &&
    overview.warnings.length === 0
  ) {
    return (
      <section className="protection-overview">
        <header className="protection-heading">
          <p className="eyebrow">WORKSPACE_PROTECTION</p>
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
        <p className="eyebrow">WORKSPACE_PROTECTION</p>
        <h1 id="protection-title" tabIndex={-1}>
          Protection overview
        </h1>
        <p>Read-only local evidence for save-time protection and its next attention item.</p>
        <div className="data-state-row">
          <Badge variant="outline">
            {overview.data_state === 'complete'
              ? '[ OK ] Full data'
              : overview.data_state === 'partial'
                ? '[ WARN ] Partial data'
                : '[ N/A ] Data unavailable'}
          </Badge>
          {offline ? <Badge variant="outline">[ WARN ] Offline · last-known evidence</Badge> : null}
        </div>
      </header>
      <ProtectionSummary
        overview={overview}
        onInspectAttention={() => selected && select(selected.id)}
        warning={selected}
      />
      <CurrentHealthCards overview={overview} />
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
            <TabsTrigger value="warnings">{warningsLabel}</TabsTrigger>
          </TabsList>
          <TabsContent className="panel activity-panel" forceMount value="runs">
            <header className="panel-header">
              <div>
                <h2>Latest runs</h2>
                <p>Most recent save-time scans</p>
              </div>
              <span className="panel-count">{overview.recent_runs.length} runs</span>
            </header>
            {overview.recent_runs.length > 0 ? (
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
            ) : (
              <EmptyState
                description="No retained save-time run summaries are available."
                title="No recent runs"
              />
            )}
          </TabsContent>
          <TabsContent className="panel activity-panel warnings-panel" forceMount value="warnings">
            <header className="panel-header">
              <div>
                <h2>
                  {overview.warnings_state === 'complete'
                    ? `Active warnings (${filteredWarnings.length})`
                    : warningsLabel}
                </h2>
                <p>
                  {overview.warnings_state === 'complete'
                    ? 'Ordered by severity and recency'
                    : 'Coverage reported by the local dashboard API'}
                </p>
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
            {filteredWarnings.length > 0 ? (
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
            ) : overview.warnings_state === 'complete' ? (
              <EmptyState
                description={
                  overview.warnings.length === 0
                    ? 'The complete warning resource contains no active warnings.'
                    : 'No active warnings match the selected severity.'
                }
                title={
                  overview.warnings.length === 0 ? 'No active warnings' : 'No matching warnings'
                }
              />
            ) : (
              <ResourceStateNotice
                label="Warnings"
                reason={warningsGap}
                state={overview.warnings_state}
              />
            )}
          </TabsContent>
        </Tabs>
        <EvidenceInspector warning={selected} />
      </div>
      <section aria-labelledby="affected-files-title" className="panel affected-files-panel">
        <header className="panel-header">
          <div>
            <h2 id="affected-files-title">{affectedFilesLabel}</h2>
            <p>
              {overview.affected_files_state === 'complete'
                ? 'Files with active warnings in the latest evidence'
                : 'Coverage reported by the local dashboard API'}
            </p>
          </div>
        </header>
        {overview.affected_files.length > 0 ? (
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
        ) : overview.affected_files_state === 'complete' ? (
          <EmptyState
            description="The complete affected-file resource contains no active warning locations."
            title="No affected files"
          />
        ) : (
          <ResourceStateNotice
            label="Affected files"
            reason={affectedFilesGap}
            state={overview.affected_files_state}
          />
        )}
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
