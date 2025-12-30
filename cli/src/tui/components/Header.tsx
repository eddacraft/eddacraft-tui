import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../utils/theme.js';

interface HeaderProps {
  title: string;
  subtitle?: string;
  version?: string;
  width?: number;
}

export function Header({ title, subtitle, version, width = 50 }: HeaderProps): React.ReactElement {
  const separator = theme.icons.section.repeat(width);

  return (
    <Box flexDirection="column" marginBottom={1}>
      <Text color={theme.colours.charcoal}>{separator}</Text>
      <Box>
        <Text bold color={theme.colours.ember}>
          {title.toUpperCase()}
        </Text>
        {version && <Text color={theme.colours.smoke}> v{version}</Text>}
      </Box>
      {subtitle && <Text color={theme.colours.ash}>{subtitle}</Text>}
    </Box>
  );
}
