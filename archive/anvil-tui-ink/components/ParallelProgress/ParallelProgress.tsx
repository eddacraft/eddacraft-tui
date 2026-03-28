import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../utils/theme.js';
import { CheckProgressBar } from './CheckProgressBar.js';
import {
  type CheckProgress,
  calculateOverallProgress,
  calculateETA,
  formatDuration,
} from './types.js';

interface ParallelProgressProps {
  checks: CheckProgress[];
  title?: string;
  showETA?: boolean;
  showOverall?: boolean;
  compact?: boolean;
}

function OverallProgressBar({
  percent,
  width = 40,
}: {
  percent: number;
  width?: number;
}): React.ReactElement {
  const clampedPercent = Math.max(0, Math.min(100, percent));
  const filled = Math.round((clampedPercent / 100) * width);
  const empty = width - filled;

  const colour = percent === 100 ? theme.colours.steel : theme.colours.ember;

  return (
    <Box>
      <Text color={colour}>{'█'.repeat(filled)}</Text>
      <Text color={theme.colours.charcoal}>{'░'.repeat(empty)}</Text>
      <Text color={theme.colours.smoke}> {clampedPercent}%</Text>
    </Box>
  );
}

export function ParallelProgress({
  checks,
  title = 'Running Checks',
  showETA = true,
  showOverall = true,
  compact = false,
}: ParallelProgressProps): React.ReactElement {
  const overallProgress = calculateOverallProgress(checks);
  const eta = calculateETA(checks);

  const completedCount = checks.filter(
    (c) => c.status === 'passed' || c.status === 'failed' || c.status === 'cached'
  ).length;
  const failedCount = checks.filter((c) => c.status === 'failed').length;

  const statusText =
    overallProgress === 100
      ? failedCount > 0
        ? `Complete (${failedCount} failed)`
        : 'Complete'
      : `${completedCount}/${checks.length} complete`;

  return (
    <Box
      flexDirection="column"
      borderStyle="single"
      borderColor={theme.colours.charcoal}
      paddingX={1}
    >
      <Box marginBottom={compact ? 0 : 1}>
        <Text bold color={theme.colours.ember}>
          {theme.icons.bullet} {title}
        </Text>
        <Text color={theme.colours.smoke}> ({statusText})</Text>
      </Box>

      <Box flexDirection="column">
        {checks.map((check) => (
          <CheckProgressBar key={check.id} check={check} />
        ))}
      </Box>

      {showOverall && (
        <Box marginTop={1} flexDirection="column">
          <Box gap={1}>
            <Text color={theme.colours.smoke}>Overall:</Text>
            <OverallProgressBar percent={overallProgress} />
          </Box>

          {showETA && eta !== undefined && overallProgress < 100 && (
            <Box marginTop={0}>
              <Text color={theme.colours.smoke}>ETA: {formatDuration(eta)}</Text>
            </Box>
          )}
        </Box>
      )}
    </Box>
  );
}
