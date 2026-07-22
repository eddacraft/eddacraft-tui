import { useMemo } from 'react';
import {
  Bar,
  BarChart,
  Cell,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';

import type { components } from '@/api/generated/openapi';
import { EmptyState } from '@/components/primitives/empty-state';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { dashboardTheme } from '@/lib/theme';

type Warning = components['schemas']['WarningSummary'];

function countBy(values: readonly string[]) {
  const map = new Map<string, number>();
  for (const value of values) {
    map.set(value, (map.get(value) ?? 0) + 1);
  }
  return Array.from(map.entries())
    .map(([label, value]) => ({ label, value }))
    .sort((a, b) => b.value - a.value);
}

export function WarningCharts({ warnings }: { warnings: readonly Warning[] }) {
  const byPattern = useMemo(
    () => countBy(warnings.map((warning) => warning.matched_pattern || warning.rule)).slice(0, 12),
    [warnings]
  );
  const byFile = useMemo(
    () => countBy(warnings.map((warning) => warning.file_path ?? 'Workspace')).slice(0, 12),
    [warnings]
  );
  const bySeverity = useMemo(
    () => countBy(warnings.map((warning) => warning.severity)),
    [warnings]
  );
  const byCategory = useMemo(
    () => countBy(warnings.map((warning) => warning.category)),
    [warnings]
  );

  if (warnings.length === 0) {
    return (
      <EmptyState
        description="Charts require active warnings from the latest gate snapshot."
        title="No warning data"
      />
    );
  }

  return (
    <div className="warning-charts-grid">
      <Card>
        <CardHeader>
          <CardTitle>By pattern</CardTitle>
          <CardDescription>Top matched pattern or rule identifiers</CardDescription>
        </CardHeader>
        <CardContent className="h-64">
          <ResponsiveContainer height="100%" width="100%">
            <BarChart data={byPattern}>
              <XAxis dataKey="label" hide />
              <YAxis allowDecimals={false} />
              <Tooltip />
              <Bar dataKey="value" fill={dashboardTheme.chart[0]} />
            </BarChart>
          </ResponsiveContainer>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>Hotspot files</CardTitle>
          <CardDescription>Files with the most active warnings</CardDescription>
        </CardHeader>
        <CardContent className="h-64">
          <ResponsiveContainer height="100%" width="100%">
            <BarChart data={byFile} layout="vertical">
              <XAxis allowDecimals={false} type="number" />
              <YAxis dataKey="label" type="category" width={120} />
              <Tooltip />
              <Bar dataKey="value" fill={dashboardTheme.chart[1] ?? dashboardTheme.chart[0]} />
            </BarChart>
          </ResponsiveContainer>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>Severity</CardTitle>
          <CardDescription>Distribution of active warning severity</CardDescription>
        </CardHeader>
        <CardContent className="h-64">
          <ResponsiveContainer height="100%" width="100%">
            <PieChart>
              <Pie data={bySeverity} dataKey="value" nameKey="label" outerRadius={90}>
                {bySeverity.map((entry, index) => (
                  <Cell
                    fill={dashboardTheme.chart[index % dashboardTheme.chart.length]}
                    key={entry.label}
                  />
                ))}
              </Pie>
              <Tooltip />
            </PieChart>
          </ResponsiveContainer>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>Category</CardTitle>
          <CardDescription>Distribution of warning categories</CardDescription>
        </CardHeader>
        <CardContent className="h-64">
          <ResponsiveContainer height="100%" width="100%">
            <PieChart>
              <Pie data={byCategory} dataKey="value" nameKey="label" outerRadius={90}>
                {byCategory.map((entry, index) => (
                  <Cell
                    fill={dashboardTheme.chart[index % dashboardTheme.chart.length]}
                    key={entry.label}
                  />
                ))}
              </Pie>
              <Tooltip />
            </PieChart>
          </ResponsiveContainer>
        </CardContent>
      </Card>
    </div>
  );
}
