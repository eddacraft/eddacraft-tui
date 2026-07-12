import { CircleCheck, CircleHelp, CircleX, TriangleAlert } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import type { DashboardStatus } from '@/lib/theme';

const statusIcon = {
  pass: CircleCheck,
  fail: CircleX,
  warn: TriangleAlert,
  info: CircleHelp,
  unavailable: CircleHelp,
} satisfies Record<DashboardStatus, typeof CircleCheck>;

export interface StatusBadgeProps {
  status: DashboardStatus;
  label: string;
}

export function StatusBadge({ status, label }: StatusBadgeProps) {
  const Icon = statusIcon[status];
  const variant = status === 'fail' ? 'destructive' : status === 'pass' ? 'default' : 'outline';

  return (
    <Badge data-status={status} variant={variant}>
      <Icon aria-hidden="true" data-icon="inline-start" />
      {label}
    </Badge>
  );
}
