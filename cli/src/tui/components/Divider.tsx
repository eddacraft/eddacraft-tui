import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../utils/theme.js';
import { getTerminalSize } from '../utils/tty-detection.js';

interface DividerProps {
  character?: string;
  colour?: string;
}

export function Divider({
  character = '\u2500',
  colour = theme.colours.border,
}: DividerProps): React.ReactElement {
  const { columns } = getTerminalSize();
  const width = Math.max(columns - 4, 20);

  return (
    <Box marginY={1}>
      <Text color={colour}>{character.repeat(width)}</Text>
    </Box>
  );
}
