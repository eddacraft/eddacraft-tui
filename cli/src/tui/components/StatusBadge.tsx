import React from 'react';
import { Text } from 'ink';
import { theme } from '../utils/theme.js';

export type StatusType = 'success' | 'error' | 'warning' | 'info' | 'running' | 'skipped';

interface StatusBadgeProps {
  status: StatusType;
  label?: string;
}

const statusConfig: Record<StatusType, { icon: string; colour: string; defaultLabel: string }> = {
  success: { icon: theme.icons.success, colour: theme.colours.success, defaultLabel: 'Passed' },
  error: { icon: theme.icons.error, colour: theme.colours.error, defaultLabel: 'Failed' },
  warning: { icon: theme.icons.warning, colour: theme.colours.warning, defaultLabel: 'Warning' },
  info: { icon: theme.icons.info, colour: theme.colours.info, defaultLabel: 'Info' },
  running: { icon: theme.icons.bullet, colour: theme.colours.info, defaultLabel: 'Running' },
  skipped: { icon: theme.icons.bullet, colour: theme.colours.muted, defaultLabel: 'Skipped' },
};

export function StatusBadge({ status, label }: StatusBadgeProps): React.ReactElement {
  const config = statusConfig[status];
  const displayLabel = label ?? config.defaultLabel;

  return (
    <Text color={config.colour}>
      {config.icon} {displayLabel}
    </Text>
  );
}
