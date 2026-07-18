import { Card, CardContent, CardDescription, CardHeader } from '@/components/ui/card';

export interface MetricCardProps {
  label: string;
  value: string;
  detail?: string;
  state?: 'complete' | 'partial' | 'unavailable';
}

export function MetricCard({ label, value, detail, state }: MetricCardProps) {
  return (
    <Card aria-label={label} className="metric-card gap-2 py-3" data-state={state} role="group">
      <CardHeader className="gap-0 px-4">
        <CardDescription>{label}</CardDescription>
      </CardHeader>
      <CardContent className="px-4">
        <strong className="metric-card-value">{value}</strong>
        {detail ? <p className="metric-card-detail">{detail}</p> : null}
      </CardContent>
    </Card>
  );
}
