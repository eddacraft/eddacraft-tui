import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../utils/theme.js';

interface ProgressBarProps {
  percent: number;
  width?: number;
  label?: string;
  showPercent?: boolean;
}

export function ProgressBar({
  percent,
  width = 30,
  label,
  showPercent = true,
}: ProgressBarProps): React.ReactElement {
  const clampedPercent = Math.max(0, Math.min(100, percent));
  const filledWidth = Math.round((clampedPercent / 100) * width);
  const emptyWidth = width - filledWidth;

  const filled = '\u2588'.repeat(filledWidth);
  const empty = '\u2591'.repeat(emptyWidth);

  const colour = clampedPercent === 100 ? theme.colours.success : theme.colours.info;

  return (
    <Box>
      {label && <Text color={theme.colours.text}>{label} </Text>}
      <Text color={colour}>{filled}</Text>
      <Text color={theme.colours.muted}>{empty}</Text>
      {showPercent && <Text color={theme.colours.muted}> {clampedPercent.toFixed(0)}%</Text>}
    </Box>
  );
}
