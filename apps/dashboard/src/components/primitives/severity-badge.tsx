import { Badge } from '@/components/ui/badge';
import type { DashboardSeverity } from '@/lib/theme';

export interface SeverityBadgeProps {
  severity: DashboardSeverity;
}

export function SeverityBadge({ severity }: SeverityBadgeProps) {
  const variant = severity === 'critical' || severity === 'high' ? 'destructive' : 'outline';
  return (
    <Badge data-severity={severity} variant={variant}>
      {severity === 'critical' || severity === 'high' ? '[ ERR ]' : '[ WARN ]'}{' '}
      {severity.charAt(0).toUpperCase() + severity.slice(1)} severity
    </Badge>
  );
}
