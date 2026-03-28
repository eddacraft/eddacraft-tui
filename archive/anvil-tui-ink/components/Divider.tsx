import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../utils/theme.js';
import { getTerminalSize } from '../utils/tty-detection.js';

interface DividerProps {
  character?: string;
  colour?: string;
  variant?: 'heavy' | 'light';
}

export function Divider({
  character,
  colour,
  variant = 'light',
}: DividerProps): React.ReactElement {
  const { columns } = getTerminalSize();
  const width = Math.max(columns - 4, 20);

  const effectiveChar = character ?? (variant === 'heavy' ? theme.icons.section : '─');
  const effectiveColour =
    colour ?? (variant === 'heavy' ? theme.colours.charcoal : theme.colours.smoke);

  return (
    <Box marginY={1}>
      <Text color={effectiveColour}>{effectiveChar.repeat(width)}</Text>
    </Box>
  );
}
