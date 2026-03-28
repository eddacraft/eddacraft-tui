import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';

export function IntroStep(): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Write Your First Policy
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>Policies let you enforce custom rules using OPA/Rego.</Text>
        <Text color={theme.colours.ash}>
          They run alongside Anvil&apos;s built-in checks and apply
        </Text>
        <Text color={theme.colours.ash}>to every file change.</Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.text}>In this tutorial, you&apos;ll:</Text>
        <Box flexDirection="column" marginLeft={2}>
          <Text color={theme.colours.ash}>{theme.icons.bullet} Create a policy directory</Text>
          <Text color={theme.colours.ash}>{theme.icons.bullet} Write a max-file-length rule</Text>
          <Text color={theme.colours.ash}>{theme.icons.bullet} Test it</Text>
          <Text color={theme.colours.ash}>{theme.icons.bullet} See it trigger on your code</Text>
        </Box>
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
