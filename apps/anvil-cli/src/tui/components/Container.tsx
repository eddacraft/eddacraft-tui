import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../utils/theme.js';

type BorderStyle =
  | 'single'
  | 'double'
  | 'round'
  | 'bold'
  | 'singleDouble'
  | 'doubleSingle'
  | 'classic';

interface ContainerProps {
  title?: string;
  children: React.ReactNode;
  borderStyle?: BorderStyle;
  variant?: 'primary' | 'secondary' | 'subtle';
  padding?: number;
  marginTop?: number;
  marginBottom?: number;
}

function getContainerStyle(variant: 'primary' | 'secondary' | 'subtle'): {
  borderStyle: BorderStyle;
  borderColour: string;
  titleColour: string;
} {
  switch (variant) {
    case 'primary':
      return {
        borderStyle: 'double',
        borderColour: theme.colours.charcoal,
        titleColour: theme.colours.ember,
      };
    case 'secondary':
      return {
        borderStyle: 'single',
        borderColour: theme.colours.charcoal,
        titleColour: theme.colours.ash,
      };
    case 'subtle':
      return {
        borderStyle: 'round',
        borderColour: theme.colours.smoke,
        titleColour: theme.colours.smoke,
      };
  }
}

export function Container({
  title,
  children,
  borderStyle,
  variant = 'secondary',
  padding = 1,
  marginTop = 0,
  marginBottom = 0,
}: ContainerProps): React.ReactElement {
  const style = getContainerStyle(variant);
  const effectiveBorderStyle = borderStyle ?? style.borderStyle;

  return (
    <Box
      flexDirection="column"
      borderStyle={effectiveBorderStyle}
      borderColor={style.borderColour}
      paddingX={padding}
      paddingY={padding > 0 ? 1 : 0}
      marginTop={marginTop}
      marginBottom={marginBottom}
    >
      {title && (
        <Box marginBottom={1}>
          <Text bold color={style.titleColour}>
            {title}
          </Text>
        </Box>
      )}
      {children}
    </Box>
  );
}
