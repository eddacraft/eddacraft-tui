import React from 'react';
import { Box, Text } from 'ink';
import Spinner from 'ink-spinner';
import { theme } from '../../utils/theme.js';
import { type CheckProgress, getStatusColour, getStatusIcon, formatDuration } from './types.js';

interface CheckProgressBarProps {
  check: CheckProgress;
  width?: number;
}

const PROGRESS_CHARS = '▏▎▍▌▋▊▉█';

function renderProgressBar(percent: number, width: number, colour: string): React.ReactElement {
  const clampedPercent = Math.max(0, Math.min(100, percent));
  const totalUnits = width * 8;
  const filledUnits = Math.round((clampedPercent / 100) * totalUnits);

  const fullBlocks = Math.floor(filledUnits / 8);
  const partialBlock = filledUnits % 8;
  const emptyBlocks = width - fullBlocks - (partialBlock > 0 ? 1 : 0);

  const filled = '█'.repeat(fullBlocks);
  const partial = partialBlock > 0 ? PROGRESS_CHARS[partialBlock - 1] : '';
  const empty = '░'.repeat(Math.max(0, emptyBlocks));

  return (
    <>
      <Text color={colour}>{filled}</Text>
      <Text color={colour}>{partial}</Text>
      <Text color={theme.colours.charcoal}>{empty}</Text>
    </>
  );
}

export function CheckProgressBar({ check, width = 20 }: CheckProgressBarProps): React.ReactElement {
  const colour = getStatusColour(check.status);
  const icon = getStatusIcon(check.status);

  const isComplete = ['passed', 'failed', 'skipped', 'cached'].includes(check.status);
  const showProgress = check.status === 'running';
  const showDuration = isComplete && check.durationMs !== undefined;

  return (
    <Box gap={1}>
      <Box width={14}>
        <Text color={theme.colours.ash}>{check.name}</Text>
      </Box>

      {check.status === 'cached' ? (
        <Box width={width}>
          <Text color={colour}>[Cached]</Text>
        </Box>
      ) : showProgress ? (
        <Box width={width}>{renderProgressBar(check.progress, width, colour)}</Box>
      ) : isComplete ? (
        <Box width={width}>
          <Text color={colour}>{icon} </Text>
          <Text color={colour}>{check.status}</Text>
        </Box>
      ) : (
        <Box width={width}>
          <Text color={theme.colours.smoke}>{icon} pending</Text>
        </Box>
      )}

      <Box width={8}>
        {check.status === 'running' ? (
          <Text color={colour}>
            <Spinner type="dots" /> {check.progress}%
          </Text>
        ) : showDuration ? (
          <Text color={theme.colours.smoke}>{formatDuration(check.durationMs!)}</Text>
        ) : check.status === 'cached' ? (
          <Text color={colour}>0ms</Text>
        ) : null}
      </Box>

      {check.message && (
        <Text color={theme.colours.smoke} wrap="truncate">
          {check.message}
        </Text>
      )}
    </Box>
  );
}
