import { Badge } from '@/components/ui/badge';
import type { DashboardStatus } from '@/lib/theme';

const statusGrammar = {
  pass: '[ OK ]',
  fail: '[ ERR ]',
  warn: '[ WARN ]',
  info: '[ ]',
  unavailable: '[ N/A ]',
} satisfies Record<DashboardStatus, string>;

export interface StatusBadgeProps {
  status: DashboardStatus;
  label: string;
}

export function StatusBadge({ status, label }: StatusBadgeProps) {
  const variant = status === 'fail' ? 'destructive' : status === 'pass' ? 'default' : 'outline';

  return (
    <Badge data-status={status} variant={variant}>
      <span aria-hidden="true">{statusGrammar[status]}</span>
      {label}
    </Badge>
  );
}
