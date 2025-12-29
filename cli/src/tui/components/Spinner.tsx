import React from 'react';
import { Box, Text } from 'ink';
import InkSpinner from 'ink-spinner';
import { theme } from '../utils/theme.js';

interface SpinnerProps {
  label?: string;
  colour?: string;
}

export function Spinner({ label, colour = theme.colours.info }: SpinnerProps): React.ReactElement {
  return (
    <Box>
      <Text color={colour}>
        <InkSpinner type="dots" />
      </Text>
      {label && <Text color={theme.colours.text}> {label}</Text>}
    </Box>
  );
}
