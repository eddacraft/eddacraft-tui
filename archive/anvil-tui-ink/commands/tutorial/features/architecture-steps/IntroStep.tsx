import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';

export function IntroStep(): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Architecture Boundaries
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          Architecture boundaries prevent imports from crossing layer contexts.
        </Text>
        <Text color={theme.colours.ash}>
          AI tools don&apos;t know your architecture — they generate code that
        </Text>
        <Text color={theme.colours.ash}>compiles but violates boundaries you care about.</Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          Anvil detects new dependency edges that cross your defined boundaries,
        </Text>
        <Text color={theme.colours.ash}>
          catching drift at save-time before it reaches code review.
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.text}>In this tutorial, you&apos;ll:</Text>
        <Box flexDirection="column" marginLeft={2}>
          <Text color={theme.colours.ash}>{theme.icons.bullet} Detect your project structure</Text>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} Choose an architecture template
          </Text>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} Understand how rules are compiled
          </Text>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} Learn how to validate and fix violations
          </Text>
        </Box>
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
