import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';

export function IntroStep(): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          CI Integration
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          Anvil integrates with your CI pipeline to gate pull requests.
        </Text>
        <Text color={theme.colours.ash}>
          Watch mode catches issues locally, CI catches what slips through.
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.text}>Three layers of protection:</Text>
        <Box flexDirection="column" marginLeft={2} marginTop={1}>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} <Text color={theme.colours.text}>Watch mode</Text> — instant
            feedback as you save (local)
          </Text>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} <Text color={theme.colours.text}>Pre-commit hooks</Text> — catch
            issues before push
          </Text>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} <Text color={theme.colours.text}>CI pipeline</Text> — gate pull
            requests before merge
          </Text>
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.text}>In this tutorial, you&apos;ll:</Text>
        <Box flexDirection="column" marginLeft={2}>
          <Text color={theme.colours.ash}>{theme.icons.bullet} Detect your CI system</Text>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} See a ready-made workflow config
          </Text>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} Understand exit codes and CI flags
          </Text>
          <Text color={theme.colours.ash}>{theme.icons.bullet} Set up pre-commit hooks</Text>
        </Box>
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
