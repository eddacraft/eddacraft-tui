import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../utils/theme.js';

interface IntroStepProps {
  onNext: () => void;
}

export function IntroStep({ onNext: _onNext }: IntroStepProps): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          What is Anvil?
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.text}>
          Anvil makes AI-generated code changes <Text bold>safe for production</Text>.
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>It provides:</Text>
        <Box marginLeft={2} flexDirection="column">
          <Text color={theme.colours.text}>
            {theme.icons.bullet} <Text bold>Validation</Text> - Ensure plans are well-formed
          </Text>
          <Text color={theme.colours.text}>
            {theme.icons.bullet} <Text bold>Quality Gates</Text> - Lint, test, coverage, secrets
          </Text>
          <Text color={theme.colours.text}>
            {theme.icons.bullet} <Text bold>Audit Trail</Text> - Track every change with evidence
          </Text>
          <Text color={theme.colours.text}>
            {theme.icons.bullet} <Text bold>Rollback</Text> - Safely revert any change
          </Text>
        </Box>
      </Box>

      <Box marginTop={1} flexDirection="column">
        <Text color={theme.colours.smoke}>
          This tutorial takes less than 5 minutes. You&apos;ll create a sample plan,
        </Text>
        <Text color={theme.colours.smoke}>
          validate it, and run quality gates to see Anvil in action.
        </Text>
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
