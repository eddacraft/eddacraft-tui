import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../utils/theme.js';

interface ContainerProps {
  title?: string;
  children: React.ReactNode;
  borderStyle?:
    | 'single'
    | 'double'
    | 'round'
    | 'bold'
    | 'singleDouble'
    | 'doubleSingle'
    | 'classic';
  padding?: number;
  marginTop?: number;
  marginBottom?: number;
}

export function Container({
  title,
  children,
  borderStyle = 'round',
  padding = 1,
  marginTop = 0,
  marginBottom = 0,
}: ContainerProps): React.ReactElement {
  return (
    <Box
      flexDirection="column"
      borderStyle={borderStyle}
      borderColor={theme.colours.border}
      paddingX={padding}
      paddingY={padding > 0 ? 1 : 0}
      marginTop={marginTop}
      marginBottom={marginBottom}
    >
      {title && (
        <Box marginBottom={1}>
          <Text bold color={theme.colours.primary}>
            {title}
          </Text>
        </Box>
      )}
      {children}
    </Box>
  );
}
