import { Check, ChevronRight, CircleMinus, ShieldAlert } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import {
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  protectionRuns,
  protectionWarnings,
  type ProtectionRun,
  type ProtectionWarning,
} from '@/modules/protection/fixture';

interface RunsTableProps {
  selectedRunId: string;
  onSelectRun: (run: ProtectionRun) => void;
}

interface WarningsTableProps {
  selectedWarningId: string;
  onSelectWarning: (warning: ProtectionWarning) => void;
}

function Result({ run }: { run: ProtectionRun }) {
  const clean = run.result === 'Clean';

  return (
    <span
      className={clean ? 'table-result table-result-clean' : 'table-result table-result-issues'}
    >
      {clean ? <Check aria-hidden="true" /> : <ShieldAlert aria-hidden="true" />}
      {run.result}
    </span>
  );
}

function SeverityBadge({ warning }: { warning: ProtectionWarning }) {
  return (
    <Badge
      className={`severity-badge severity-badge-${warning.severity.toLowerCase()}`}
      variant="outline"
    >
      {warning.severity}
    </Badge>
  );
}

export function RunsTable({ selectedRunId, onSelectRun }: RunsTableProps) {
  return (
    <Table className="operations-table runs-table">
      <TableCaption className="sr-only">Latest protection runs for anvil-001</TableCaption>
      <TableHeader>
        <TableRow>
          <TableHead>Started</TableHead>
          <TableHead className="hide-mobile">Duration</TableHead>
          <TableHead>Result</TableHead>
          <TableHead className="numeric-cell">Violations</TableHead>
          <TableHead className="numeric-cell">New</TableHead>
          <TableHead className="numeric-cell">Changed</TableHead>
          <TableHead className="hide-mobile">Workspace</TableHead>
          <TableHead className="mobile-row-action">
            <span className="sr-only">Open</span>
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {protectionRuns.map((run) => {
          const [date, time] = run.started.split(' ');
          return (
            <TableRow data-selected={selectedRunId === run.id || undefined} key={run.id}>
              <TableCell>
                <button
                  aria-label={`Select run started ${run.started}`}
                  className="table-select-button run-started"
                  onClick={() => onSelectRun(run)}
                  type="button"
                >
                  <span className="run-date">{date}</span>
                  <span>{time}</span>
                </button>
              </TableCell>
              <TableCell className="hide-mobile muted-cell">{run.duration}</TableCell>
              <TableCell>
                <Result run={run} />
              </TableCell>
              <TableCell className="numeric-cell">{run.violations}</TableCell>
              <TableCell className="numeric-cell table-new-value">
                {run.newViolations || <CircleMinus aria-label="None" />}
              </TableCell>
              <TableCell className="numeric-cell">{run.changedFiles}</TableCell>
              <TableCell className="hide-mobile muted-cell">anvil-001</TableCell>
              <TableCell className="mobile-row-action">
                <ChevronRight aria-hidden="true" />
              </TableCell>
            </TableRow>
          );
        })}
      </TableBody>
    </Table>
  );
}

export function WarningsTable({ selectedWarningId, onSelectWarning }: WarningsTableProps) {
  return (
    <Table className="operations-table warnings-table">
      <TableCaption className="sr-only">Active protection warnings for anvil-001</TableCaption>
      <TableHeader>
        <TableRow>
          <TableHead>Severity</TableHead>
          <TableHead>Rule</TableHead>
          <TableHead className="hide-tablet">Category</TableHead>
          <TableHead>File</TableHead>
          <TableHead className="numeric-cell hide-mobile">Age</TableHead>
          <TableHead className="numeric-cell hide-mobile">Evidence</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {protectionWarnings.map((warning) => (
          <TableRow data-selected={selectedWarningId === warning.id || undefined} key={warning.id}>
            <TableCell>
              <SeverityBadge warning={warning} />
            </TableCell>
            <TableCell>
              <button
                aria-label={`Inspect ${warning.rule}`}
                className="table-select-button table-rule"
                onClick={() => onSelectWarning(warning)}
                type="button"
              >
                {warning.rule}
              </button>
            </TableCell>
            <TableCell className="hide-tablet muted-cell">{warning.category}</TableCell>
            <TableCell>
              <code className="file-reference">
                {warning.file}:<span>{warning.line}</span>
              </code>
            </TableCell>
            <TableCell className="numeric-cell hide-mobile muted-cell">{warning.age}</TableCell>
            <TableCell className="numeric-cell hide-mobile">
              {warning.evidence ? (
                <Check aria-label="Evidence captured" className="evidence-yes" />
              ) : (
                <CircleMinus aria-label="No evidence" className="muted-cell" />
              )}
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

export function AffectedFilesTable({ selectedWarningId, onSelectWarning }: WarningsTableProps) {
  return (
    <Table className="operations-table affected-files-table">
      <TableCaption className="sr-only">Files affected by active protection warnings</TableCaption>
      <TableHeader>
        <TableRow>
          <TableHead>File path</TableHead>
          <TableHead className="numeric-cell">Warnings</TableHead>
          <TableHead>Highest severity</TableHead>
          <TableHead className="hide-mobile">First seen</TableHead>
          <TableHead>Last seen</TableHead>
          <TableHead className="mobile-row-action">
            <span className="sr-only">Open</span>
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {protectionWarnings.map((warning) => (
          <TableRow data-selected={selectedWarningId === warning.id || undefined} key={warning.id}>
            <TableCell>
              <button
                aria-label={`Inspect warnings in ${warning.file}`}
                className="table-select-button affected-file-button"
                onClick={() => onSelectWarning(warning)}
                type="button"
              >
                {warning.file}
              </button>
            </TableCell>
            <TableCell className="numeric-cell">1</TableCell>
            <TableCell>
              <SeverityBadge warning={warning} />
            </TableCell>
            <TableCell className="hide-mobile muted-cell">{warning.age} ago</TableCell>
            <TableCell className="muted-cell">{warning.age} ago</TableCell>
            <TableCell className="mobile-row-action">
              <ChevronRight aria-hidden="true" />
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
