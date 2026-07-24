import type { ColumnDef } from '@tanstack/react-table';
import { useMemo, useState } from 'react';

import type { components } from '@/api/generated/openapi';
import { DataTable } from '@/components/primitives/data-table';
import { EmptyState } from '@/components/primitives/empty-state';

type Catalogue = components['schemas']['PatternCatalogue'];
type Warning = components['schemas']['WarningSummary'];

const docsPanelId = (patternId: string) =>
  `pattern-docs-${patternId.replaceAll(/[^A-Za-z0-9_-]/g, '-')}`;

export function PatternRegistry({
  catalogue,
  warnings = [],
}: {
  catalogue: Catalogue;
  warnings?: readonly Warning[];
}) {
  const [expanded, setExpanded] = useState<string | null>(null);
  const counts = useMemo(() => {
    const map = new Map<string, number>();
    for (const warning of warnings) {
      const key = warning.matched_pattern || warning.rule;
      if (!key) continue;
      map.set(key, (map.get(key) ?? 0) + 1);
    }
    return map;
  }, [warnings]);

  if (catalogue.data_state === 'unavailable' || catalogue.patterns.length === 0) {
    return (
      <EmptyState description={catalogue.source_message} title="Pattern registry unavailable" />
    );
  }

  const rows = catalogue.patterns.map((pattern) => ({
    ...pattern,
    instance_count: counts.get(pattern.id) ?? pattern.instance_count,
  }));

  const columns: ColumnDef<(typeof rows)[number]>[] = [
    { header: 'ID', accessorKey: 'id' },
    { header: 'Name', accessorKey: 'title' },
    { header: 'Family', accessorKey: 'family' },
    { header: 'Severity', accessorKey: 'severity' },
    {
      header: 'Enabled',
      cell: ({ row }) => (row.original.enabled ? 'yes' : 'no'),
    },
    { header: 'Instances', accessorKey: 'instance_count' },
    {
      header: 'Docs',
      cell: ({ row }) => (
        <button
          aria-controls={docsPanelId(row.original.id)}
          aria-expanded={expanded === row.original.id}
          className="table-select-button"
          onClick={() => setExpanded(expanded === row.original.id ? null : row.original.id)}
          type="button"
        >
          {expanded === row.original.id ? 'Hide' : 'Show'}
        </button>
      ),
    },
  ];

  const selected = rows.find((row) => row.id === expanded);

  return (
    <div className="pattern-registry">
      <DataTable caption="Compiled anti-pattern registry" columns={columns} data={rows} />
      {selected ? (
        <article className="panel" id={docsPanelId(selected.id)}>
          <h3>
            {selected.id}: {selected.title}
          </h3>
          <p>{selected.description || 'No description provided in the compiled registry.'}</p>
        </article>
      ) : null}
    </div>
  );
}
