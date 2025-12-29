import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../utils/theme.js';

interface HeaderProps {
  title: string;
  subtitle?: string;
  version?: string;
}

export function Header({ title, subtitle, version }: HeaderProps): React.ReactElement {
  return (
    <Box flexDirection="column" marginBottom={1}>
      <Box>
        <Text bold color={theme.colours.primary}>
          {theme.icons.arrow} {title}
        </Text>
        {version && <Text color={theme.colours.muted}> v{version}</Text>}
      </Box>
      {subtitle && <Text color={theme.colours.muted}>{subtitle}</Text>}
    </Box>
  );
}
