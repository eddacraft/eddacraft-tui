import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../utils/theme.js';
import { formatElapsedTime } from '../types.js';

interface CompletionStepProps {
  startedAt: Date;
  onCleanup: () => void;
  onFinish: () => void;
}

export function CompletionStep({
  startedAt,
  onCleanup: _onCleanup,
  onFinish: _onFinish,
}: CompletionStepProps): React.ReactElement {
  const elapsed = formatElapsedTime(startedAt);

  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          {theme.icons.success} Tutorial Complete!
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.text}>
          Congratulations! You&apos;ve learned the basics of Anvil in {elapsed}.
        </Text>
      </Box>

      <Box marginTop={1} marginBottom={1}>
        <Text bold color={theme.colours.ash}>
          What you learned:
        </Text>
      </Box>

      <Box marginLeft={2} flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.text}>
          {theme.icons.check} Creating plans that describe your changes
        </Text>
        <Text color={theme.colours.text}>{theme.icons.check} Validating plans for correctness</Text>
        <Text color={theme.colours.text}>
          {theme.icons.check} Running quality gates to ensure code safety
        </Text>
      </Box>

      <Box marginTop={1} marginBottom={1}>
        <Text bold color={theme.colours.ash}>
          Next steps:
        </Text>
      </Box>

      <Box marginLeft={2} flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.text}>
          {theme.icons.arrow} Run <Text color={theme.colours.emberBright}>anvil init</Text> to set
          up your project
        </Text>
        <Text color={theme.colours.text}>
          {theme.icons.arrow} Run <Text color={theme.colours.emberBright}>anvil doctor</Text> to
          check your environment
        </Text>
        <Text color={theme.colours.text}>
          {theme.icons.arrow} Run <Text color={theme.colours.emberBright}>anvil new</Text> to create
          a plan from templates
        </Text>
      </Box>

      <Box marginTop={1} marginBottom={1}>
        <Text bold color={theme.colours.ash}>
          Resources:
        </Text>
      </Box>

      <Box marginLeft={2} flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.smoke}>
          {theme.icons.bullet} Documentation: https://anvil.dev/docs
        </Text>
        <Text color={theme.colours.smoke}>{theme.icons.bullet} Quick Start: anvil --help</Text>
      </Box>

      <Box marginTop={2} flexDirection="column">
        <Text color={theme.colours.molten}>
          Press <Text bold>c</Text> to clean up tutorial files, or <Text bold>q</Text> to exit
        </Text>
      </Box>
    </Box>
  );
}
