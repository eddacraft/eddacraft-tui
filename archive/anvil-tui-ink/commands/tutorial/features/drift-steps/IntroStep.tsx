import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';

export function IntroStep(): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Drift Detection
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          Drift detection tracks how your architecture changes over time.
        </Text>
        <Text color={theme.colours.ash}>
          Snapshots capture the current state of imports and dependencies.
        </Text>
        <Text color={theme.colours.ash}>
          Comparing snapshots reveals new edges, removed modules, or shifted
        </Text>
        <Text color={theme.colours.ash}>boundaries.</Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.text}>Key commands:</Text>
        <Box flexDirection="column" marginLeft={2} marginTop={1}>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} <Text color={theme.colours.text}>anvil drift snapshot</Text> —
            capture current state
          </Text>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} <Text color={theme.colours.text}>anvil drift compare</Text> — see
            changes between snapshots
          </Text>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} <Text color={theme.colours.text}>anvil drift report</Text> — trend
            over time
          </Text>
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.text}>In this tutorial, you&apos;ll:</Text>
        <Box flexDirection="column" marginLeft={2}>
          <Text color={theme.colours.ash}>{theme.icons.bullet} Capture a baseline snapshot</Text>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} Inspect what a snapshot contains
          </Text>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} Compare snapshots to detect drift
          </Text>
          <Text color={theme.colours.ash}>{theme.icons.bullet} Understand drift trends</Text>
        </Box>
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
