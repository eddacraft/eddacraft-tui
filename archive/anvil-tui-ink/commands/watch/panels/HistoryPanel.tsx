import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../utils/theme.js';
import type { RunHistory } from '../types.js';
import { formatTimestamp, formatDuration } from '../types.js';

interface HistoryPanelProps {
  history: RunHistory[];
  focused: boolean;
  maxVisible?: number;
}

export function HistoryPanel({
  history,
  focused,
  maxVisible = 10,
}: HistoryPanelProps): React.ReactElement {
  const borderColour = focused ? theme.colours.ember : theme.colours.charcoal;
  const visibleHistory = history.slice(0, maxVisible);

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
          {theme.icons.bullet} HISTORY
        </Text>
        <Text color={theme.colours.smoke}> (last {Math.min(history.length, maxVisible)})</Text>
        {focused && <Text color={theme.colours.smoke}> (focused)</Text>}
      </Box>

      {history.length === 0 ? (
        <Text color={theme.colours.smoke}>{theme.icons.info} No runs yet</Text>
      ) : (
        <Box flexDirection="column">
          {visibleHistory.map((run) => (
            <Box key={run.id} gap={1}>
              <Box width={10}>
                <Text color={theme.colours.smoke}>{formatTimestamp(run.timestamp)}</Text>
              </Box>
              <Box width={20}>
                <Text color={theme.colours.ash}>
                  {run.files
                    .map((f) => f.split('/').pop())
                    .join(', ')
                    .slice(0, 18)}
                  {run.files.join(', ').length > 18 ? '...' : ''}
                </Text>
              </Box>
              <Box width={8}>
                <Text color={theme.colours.smoke}>{formatDuration(run.durationMs)}</Text>
              </Box>
              <Box width={8}>
                {run.success ? (
                  <Text color={theme.colours.steel}>{theme.icons.success} pass</Text>
                ) : (
                  <Text color={theme.colours.slag}>{theme.icons.error} fail</Text>
                )}
              </Box>
              {run.message && (
                <Text color={theme.colours.smoke} wrap="truncate">
                  {run.message}
                </Text>
              )}
            </Box>
          ))}
        </Box>
      )}
    </Box>
  );
}
