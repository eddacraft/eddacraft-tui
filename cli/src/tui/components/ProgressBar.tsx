import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../utils/theme.js';

interface ProgressBarProps {
  percent: number;
  width?: number;
  label?: string;
  showPercent?: boolean;
}

const PROGRESS_CHARS = '▏▎▍▌▋▊▉█';

function getProgressColour(percent: number): string {
  if (percent === 100) return theme.colours.steel;
  if (percent >= 75) return theme.colours.emberBright;
  if (percent >= 50) return theme.colours.ember;
  return theme.colours.emberDim;
}

export function ProgressBar({
  percent,
  width = 30,
  label,
  showPercent = true,
}: ProgressBarProps): React.ReactElement {
  const clampedPercent = Math.max(0, Math.min(100, percent));
  const totalUnits = width * 8;
  const filledUnits = Math.round((clampedPercent / 100) * totalUnits);

  const fullBlocks = Math.floor(filledUnits / 8);
  const partialBlock = filledUnits % 8;
  const emptyBlocks = width - fullBlocks - (partialBlock > 0 ? 1 : 0);

  const filled = '█'.repeat(fullBlocks);
  const partial = partialBlock > 0 ? PROGRESS_CHARS[partialBlock - 1] : '';
  const empty = '░'.repeat(emptyBlocks);

  const colour = getProgressColour(clampedPercent);

  return (
    <Box>
      {label && <Text color={theme.colours.ash}>{label} </Text>}
      <Text color={colour}>{filled}</Text>
      <Text color={colour}>{partial}</Text>
      <Text color={theme.colours.charcoal}>{empty}</Text>
      {showPercent && <Text color={theme.colours.smoke}> {clampedPercent.toFixed(0)}%</Text>}
    </Box>
  );
}
