import { Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { dashboardTheme } from '@/lib/theme';

export interface TrendPoint {
  label: string;
  value: number;
}

export interface TrendChartProps {
  title: string;
  description: string;
  data: readonly TrendPoint[];
}

export function TrendChart({ title, description, data }: TrendChartProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent className="h-64">
        <ResponsiveContainer height="100%" width="100%">
          <LineChart data={[...data]}>
            <XAxis dataKey="label" />
            <YAxis />
            <Tooltip />
            <Line dataKey="value" dot={false} stroke={dashboardTheme.chart[0]} type="monotone" />
          </LineChart>
        </ResponsiveContainer>
      </CardContent>
    </Card>
  );
}
