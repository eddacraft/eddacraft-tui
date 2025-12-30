import React from 'react';
import { Box, Text } from 'ink';
import { StatusBadge, type StatusType } from '../../../components/StatusBadge.js';
import { theme } from '../../../utils/theme.js';
import type { HooksStatus, HookState } from '../types.js';

interface HooksPanelProps {
  data: HooksStatus;
  focused: boolean;
}

function hookStateToStatus(state: HookState): StatusType {
  switch (state) {
    case 'active':
      return 'success';
    case 'disabled':
      return 'warning';
    case 'missing':
      return 'error';
  }
}

function formatLastRun(date?: Date): string {
  if (!date) return '';
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMins < 1) return 'just now';
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  return `${diffDays}d ago`;
}

export function HooksPanel({ data, focused }: HooksPanelProps): React.ReactElement {
  const borderColour = focused ? theme.colours.ember : theme.colours.charcoal;

  return (
    <Box
      flexDirection="column"
      borderStyle={focused ? 'double' : 'single'}
      borderColor={borderColour}
      paddingX={1}
      paddingY={0}
    >
      <Box marginBottom={0}>
        <Text bold color={focused ? theme.colours.ember : theme.colours.ash}>
          {theme.icons.bullet} HOOKS
        </Text>
        {focused && <Text color={theme.colours.smoke}> (focused)</Text>}
      </Box>

      {!data.huskyInstalled ? (
        <Box marginTop={0}>
          <Text color={theme.colours.molten}>{theme.icons.warning} Husky not installed</Text>
        </Box>
      ) : data.hooks.length === 0 ? (
        <Box marginTop={0}>
          <Text color={theme.colours.smoke}>{theme.icons.info} No hooks configured</Text>
        </Box>
      ) : (
        <Box flexDirection="column" marginTop={0}>
          {data.hooks.map((hook) => (
            <Box key={hook.name} gap={1}>
              <Box width={14}>
                <Text color={theme.colours.ash}>{hook.name}</Text>
              </Box>
              <Box width={12}>
                <StatusBadge status={hookStateToStatus(hook.state)} label={hook.state} />
              </Box>
              {hook.lastRun && (
                <Text color={theme.colours.smoke}>{formatLastRun(hook.lastRun)}</Text>
              )}
              {hook.isAnvilManaged && <Text color={theme.colours.ember}>[anvil]</Text>}
            </Box>
          ))}
        </Box>
      )}
    </Box>
  );
}
