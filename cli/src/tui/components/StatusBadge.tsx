import React from 'react';
import { Text } from 'ink';
import { theme } from '../utils/theme.js';

export type StatusType = 'success' | 'error' | 'warning' | 'info' | 'running' | 'skipped';

interface StatusBadgeProps {
  status: StatusType;
  label?: string;
}

const statusConfig: Record<StatusType, { icon: string; colour: string; defaultLabel: string }> = {
  success: { icon: theme.icons.success, colour: theme.colours.steel, defaultLabel: 'Passed' },
  error: { icon: theme.icons.error, colour: theme.colours.slag, defaultLabel: 'Failed' },
  warning: { icon: theme.icons.warning, colour: theme.colours.molten, defaultLabel: 'Warning' },
  info: { icon: theme.icons.info, colour: theme.colours.ash, defaultLabel: 'Info' },
  running: { icon: theme.icons.running, colour: theme.colours.ember, defaultLabel: 'Running' },
  skipped: { icon: theme.icons.skipped, colour: theme.colours.smoke, defaultLabel: 'Skipped' },
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
