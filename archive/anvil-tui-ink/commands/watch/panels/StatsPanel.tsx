import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../utils/theme.js';
import type { WatchStats } from '../types.js';
import { calculatePassRate, formatDuration, formatRelativeTime } from '../types.js';

interface StatsPanelProps {
  stats: WatchStats;
  focused: boolean;
}

function PassRateBar({ percent }: { percent: number }): React.ReactElement {
  const width = 20;
  const filled = Math.round((percent / 100) * width);
  const empty = width - filled;

  const colour =
    percent >= 80 ? theme.colours.steel : percent >= 50 ? theme.colours.molten : theme.colours.slag;

  return (
    <Box>
      <Text color={colour}>{'█'.repeat(filled)}</Text>
      <Text color={theme.colours.charcoal}>{'░'.repeat(empty)}</Text>
      <Text color={theme.colours.smoke}> {percent}%</Text>
    </Box>
  );
}

export function StatsPanel({ stats, focused }: StatsPanelProps): React.ReactElement {
  const borderColour = focused ? theme.colours.ember : theme.colours.charcoal;
  const passRate = calculatePassRate(stats);

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
          {theme.icons.bullet} STATISTICS
        </Text>
        {focused && <Text color={theme.colours.smoke}> (focused)</Text>}
      </Box>

      {stats.totalRuns === 0 ? (
        <Text color={theme.colours.smoke}>{theme.icons.info} No statistics yet</Text>
      ) : (
        <Box flexDirection="column">
          <Box gap={1}>
            <Text color={theme.colours.smoke}>Total Runs:</Text>
            <Text color={theme.colours.ash}>{stats.totalRuns}</Text>
          </Box>

          <Box gap={1}>
            <Text color={theme.colours.smoke}>Pass Rate:</Text>
            <PassRateBar percent={passRate} />
          </Box>

          <Box gap={1}>
            <Text color={theme.colours.smoke}>Passed:</Text>
            <Text color={theme.colours.steel}>{stats.passedRuns}</Text>
            <Text color={theme.colours.smoke}>Failed:</Text>
            <Text color={theme.colours.slag}>{stats.failedRuns}</Text>
          </Box>

          <Box gap={1}>
            <Text color={theme.colours.smoke}>Avg Duration:</Text>
            <Text color={theme.colours.ash}>{formatDuration(stats.avgDurationMs)}</Text>
          </Box>

          {stats.lastRunAt && (
            <Box gap={1}>
              <Text color={theme.colours.smoke}>Last Run:</Text>
              <Text color={theme.colours.ash}>{formatRelativeTime(stats.lastRunAt)}</Text>
            </Box>
          )}
        </Box>
      )}
    </Box>
  );
}
