import React from 'react';
import { Box, Text } from 'ink';
import Spinner from 'ink-spinner';
import { theme } from '../../../utils/theme.js';
import type { WatchState, WatchStatus } from '../types.js';

interface StatusPanelProps {
  state: WatchState;
  focused: boolean;
}

function getStatusColour(status: WatchStatus): string {
  switch (status) {
    case 'idle':
      return theme.colours.smoke;
    case 'running':
      return theme.colours.ember;
    case 'passing':
      return theme.colours.steel;
    case 'failing':
      return theme.colours.slag;
  }
}

function getStatusIcon(status: WatchStatus): string {
  switch (status) {
    case 'idle':
      return theme.icons.bullet;
    case 'running':
      return theme.icons.running;
    case 'passing':
      return theme.icons.success;
    case 'failing':
      return theme.icons.error;
  }
}

function getStatusLabel(status: WatchStatus): string {
  switch (status) {
    case 'idle':
      return 'Waiting for changes';
    case 'running':
      return 'Running...';
    case 'passing':
      return 'Passing';
    case 'failing':
      return 'Failing';
  }
}

export function StatusPanel({ state, focused }: StatusPanelProps): React.ReactElement {
  const borderColour = focused ? theme.colours.ember : theme.colours.charcoal;
  const statusColour = getStatusColour(state.status);
  const statusIcon = getStatusIcon(state.status);
  const statusLabel = getStatusLabel(state.status);

  return (
    <Box
      flexDirection="column"
      borderStyle={focused ? 'double' : 'single'}
      borderColor={borderColour}
      paddingX={1}
    >
      <Box marginBottom={0}>
        <Text bold color={focused ? theme.colours.ember : theme.colours.ash}>
          {theme.icons.bullet} STATUS
        </Text>
        {focused && <Text color={theme.colours.smoke}> (focused)</Text>}
      </Box>

      <Box gap={1} marginY={0}>
        {state.status === 'running' ? (
          <Text color={statusColour}>
            <Spinner type="dots" /> {statusLabel}
          </Text>
        ) : (
          <Text color={statusColour}>
            {statusIcon} {statusLabel}
          </Text>
        )}
      </Box>

      <Box flexDirection="column" marginTop={0}>
        <Box gap={1}>
          <Text color={theme.colours.smoke}>Action:</Text>
          <Text color={theme.colours.ash}>{state.config.action}</Text>
          {state.config.profile && (
            <Text color={theme.colours.smoke}>({state.config.profile})</Text>
          )}
        </Box>

        <Box gap={1}>
          <Text color={theme.colours.smoke}>Patterns:</Text>
          <Text color={theme.colours.ash}>
            {state.config.patterns.slice(0, 2).join(', ')}
            {state.config.patterns.length > 2 && ` +${state.config.patterns.length - 2} more`}
          </Text>
        </Box>

        <Box gap={1}>
          <Text color={theme.colours.smoke}>Filter:</Text>
          <Text color={theme.colours.ash}>
            {state.config.gitFilter ? 'unstaged only' : 'all changes'}
          </Text>
        </Box>
      </Box>

      {state.currentRun && (
        <Box marginTop={0}>
          <Text color={theme.colours.ember}>
            Processing: {state.currentRun.files.map((f) => f.split('/').pop()).join(', ')}
          </Text>
        </Box>
      )}
    </Box>
  );
}
