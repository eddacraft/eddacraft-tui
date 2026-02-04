import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';

const COMMAND_REFERENCE = [
  { cmd: 'anvil architecture create', desc: 'Set up boundaries' },
  { cmd: 'anvil architecture compile', desc: 'Generate rules' },
  { cmd: 'anvil architecture validate', desc: 'Check for violations' },
  { cmd: 'anvil check --all', desc: 'Run all checks including architecture' },
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
        <Text color={theme.colours.steel}>
          {theme.icons.success} Architecture boundaries configured!
        </Text>
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
        <Text color={theme.colours.steel}>{theme.icons.success} Tutorial complete!</Text>
      </Box>

      <Box marginTop={1}>
        <Text color={theme.colours.smoke}>
          Press <Text color={theme.colours.text}>c</Text> to clean up tutorial files,{' '}
          <Text color={theme.colours.text}>q</Text> to exit
        </Text>
      </Box>
    </Box>
  );
}
