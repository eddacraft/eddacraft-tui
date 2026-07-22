import type { ColumnDef } from '@tanstack/react-table';

import type { components } from '@/api/generated/openapi';
import { DataTable } from '@/components/primitives/data-table';
import { EmptyState } from '@/components/primitives/empty-state';

type GateRun = components['schemas']['GateRunSummary'];

const columns: ColumnDef<GateRun>[] = [
  {
    header: 'Started',
    accessorKey: 'started_at',
    cell: ({ row }) => row.original.started_at ?? 'Latest gate',
  },
  {
    header: 'Status',
    accessorKey: 'label',
  },
  {
    header: 'Score',
    cell: ({ row }) => (row.original.score == null ? '—' : String(row.original.score)),
  },
  {
    header: 'Warnings',
    accessorKey: 'warning_count',
  },
  {
    header: 'Duration',
    cell: ({ row }) =>
      row.original.duration_seconds == null ? '—' : `${row.original.duration_seconds}s`,
  },
  {
    header: 'Files',
    cell: ({ row }) =>
      row.original.changed_file_count == null ? '—' : String(row.original.changed_file_count),
  },
  {
    header: 'Open',
    cell: ({ row }) => (
      <a className="table-select-button" href={`/gates/${encodeURIComponent(row.original.id)}`}>
        Open
      </a>
    ),
  },
];

export function GateHistoryTable({ runs }: { runs: readonly GateRun[] }) {
  if (runs.length === 0) {
    return (
      <EmptyState
        description="Only the latest gate snapshot is retained today. Run a gate to populate this list."
        title="No gate history"
      />
    );
  }

  return (
    <DataTable
      caption="Gate run history from the local protection overview"
      columns={columns}
      data={runs}
      emptyMessage="No gate runs"
    />
  );
}
