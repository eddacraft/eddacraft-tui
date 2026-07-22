import type { ColumnDef } from '@tanstack/react-table';
import { useMemo, useState } from 'react';

import type { components } from '@/api/generated/openapi';
import { DataTable } from '@/components/primitives/data-table';
import { EmptyState } from '@/components/primitives/empty-state';
import { SeverityBadge } from '@/components/primitives/severity-badge';
import type { DashboardSeverity } from '@/lib/theme';

type Warning = components['schemas']['WarningSummary'];
type GroupBy = 'none' | 'file' | 'category' | 'severity' | 'pattern';

export function WarningTable({
  warnings,
  onSelect,
  selectedId,
}: {
  warnings: readonly Warning[];
  onSelect?: (warning: Warning) => void;
  selectedId?: string;
}) {
  const [severity, setSeverity] = useState('all');
  const [category, setCategory] = useState('all');
  const [groupBy, setGroupBy] = useState<GroupBy>('none');
  const [query, setQuery] = useState('');

  const categories = useMemo(
    () => Array.from(new Set(warnings.map((warning) => warning.category))).sort(),
    [warnings]
  );

  const filtered = useMemo(() => {
    return warnings.filter((warning) => {
      if (severity !== 'all' && warning.severity !== severity) return false;
      if (category !== 'all' && warning.category !== category) return false;
      if (query && !(warning.file_path ?? '').includes(query) && !warning.message.includes(query)) {
        return false;
      }
      return true;
    });
  }, [warnings, severity, category, query]);

  const columns: ColumnDef<Warning>[] = [
    {
      header: 'Severity',
      cell: ({ row }) => {
        const severity = (
          ['critical', 'high', 'medium', 'low'].includes(row.original.severity)
            ? row.original.severity
            : 'medium'
        ) as DashboardSeverity;
        return <SeverityBadge severity={severity} />;
      },
    },
    { header: 'Category', accessorKey: 'category' },
    {
      header: 'Title',
      cell: ({ row }) => (
        <button
          className="table-select-button"
          onClick={() => onSelect?.(row.original)}
          type="button"
        >
          {row.original.rule}
        </button>
      ),
    },
    {
      header: 'File',
      cell: ({ row }) => (
        <code>
          {row.original.file_path ?? 'Workspace'}:{row.original.line ?? '—'}
        </code>
      ),
    },
    { header: 'Age', accessorKey: 'age_label' },
  ];

  if (warnings.length === 0) {
    return (
      <EmptyState
        description="The latest gate snapshot did not provide active warnings."
        title="No active warnings"
      />
    );
  }

  const grouped =
    groupBy === 'none'
      ? null
      : filtered.reduce<Record<string, Warning[]>>((acc, warning) => {
          const key =
            groupBy === 'file'
              ? (warning.file_path ?? 'Workspace')
              : groupBy === 'category'
                ? warning.category
                : groupBy === 'severity'
                  ? warning.severity
                  : warning.matched_pattern || warning.rule;
          (acc[key] ??= []).push(warning);
          return acc;
        }, {});

  return (
    <div className="warning-table">
      <div className="warning-filters">
        <label>
          Severity
          <select
            aria-label="Filter warnings by severity"
            onChange={(event) => setSeverity(event.currentTarget.value)}
            value={severity}
          >
            <option value="all">All</option>
            <option value="high">High</option>
            <option value="medium">Medium</option>
            <option value="low">Low</option>
          </select>
        </label>
        <label>
          Category
          <select
            aria-label="Filter warnings by category"
            onChange={(event) => setCategory(event.currentTarget.value)}
            value={category}
          >
            <option value="all">All</option>
            {categories.map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </select>
        </label>
        <label>
          File contains
          <input
            aria-label="Filter warnings by file path"
            onChange={(event) => setQuery(event.currentTarget.value)}
            value={query}
          />
        </label>
        <label>
          Group by
          <select
            aria-label="Group warnings"
            onChange={(event) => setGroupBy(event.currentTarget.value as GroupBy)}
            value={groupBy}
          >
            <option value="none">None</option>
            <option value="file">File</option>
            <option value="category">Category</option>
            <option value="severity">Severity</option>
            <option value="pattern">Pattern</option>
          </select>
        </label>
      </div>
      {grouped ? (
        Object.entries(grouped).map(([key, rows]) => (
          <section key={key}>
            <h3>
              {key} ({rows.length})
            </h3>
            <DataTable
              caption={`Warnings grouped by ${groupBy}: ${key}`}
              columns={columns}
              data={rows.map((row) => (row.id === selectedId ? row : row))}
            />
          </section>
        ))
      ) : (
        <DataTable caption="Active warnings" columns={columns} data={filtered} />
      )}
    </div>
  );
}
