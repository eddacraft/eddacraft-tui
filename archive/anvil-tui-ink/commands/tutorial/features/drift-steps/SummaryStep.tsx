import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';

const COMMAND_REFERENCE = [
  { cmd: 'anvil drift snapshot --name <name>', desc: 'Capture current state' },
  { cmd: 'anvil drift compare <from> <to>', desc: 'See changes between snapshots' },
  { cmd: 'anvil drift report', desc: 'Trend over time' },
  { cmd: 'anvil drift list', desc: 'Show all snapshots' },
];

export function SummaryStep(): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Summary
        </Text>
      </Box>

      <Box marginBottom={1}>
        <Text color={theme.colours.steel}>{theme.icons.success} Drift tracking configured!</Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.text}>Quick reference:</Text>
        <Box flexDirection="column" marginLeft={2} marginTop={1}>
          {COMMAND_REFERENCE.map((item) => (
            <Box key={item.cmd}>
              <Text color={theme.colours.text}>{item.cmd}</Text>
              <Text color={theme.colours.smoke}> — {item.desc}</Text>
            </Box>
          ))}
        </Box>
      </Box>

      <Box marginBottom={1}>
        <Text color={theme.colours.ash}>
          {theme.icons.info} Tip: Take a snapshot at the start of each sprint to track drift over
          time
        </Text>
      </Box>

      <Box marginBottom={1}>
        <Text color={theme.colours.steel}>{theme.icons.success} Tutorial complete!</Text>
      </Box>
    </Box>
  );
}
