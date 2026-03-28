import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../utils/theme.js';
import type { QueuedChange } from '../types.js';
import { formatRelativeTime } from '../types.js';

interface QueuePanelProps {
  queue: QueuedChange[];
  focused: boolean;
}

export function QueuePanel({ queue, focused }: QueuePanelProps): React.ReactElement {
  const borderColour = focused ? theme.colours.ember : theme.colours.charcoal;

  return (
    <Box
      flexDirection="column"
      borderStyle={focused ? 'double' : 'single'}
      borderColor={borderColour}
      paddingX={1}
      marginTop={1}
    >
      <Box marginBottom={0}>
        <Text bold color={focused ? theme.colours.ember : theme.colours.ash}>
          {theme.icons.bullet} PENDING CHANGES
        </Text>
        <Text color={theme.colours.smoke}> ({queue.length})</Text>
        {focused && <Text color={theme.colours.smoke}> (focused)</Text>}
      </Box>

      {queue.length === 0 ? (
        <Text color={theme.colours.smoke}>{theme.icons.info} No pending changes</Text>
      ) : (
        <Box flexDirection="column">
          {queue.slice(0, 5).map((change) => (
            <Box key={change.file} gap={1}>
              <Text color={theme.colours.ember}>{theme.icons.bullet}</Text>
              <Text color={theme.colours.ash}>{change.file.split('/').pop()}</Text>
              <Text color={theme.colours.smoke}>{formatRelativeTime(change.timestamp)}</Text>
            </Box>
          ))}
          {queue.length > 5 && <Text color={theme.colours.smoke}>+{queue.length - 5} more</Text>}
        </Box>
      )}
    </Box>
  );
}
