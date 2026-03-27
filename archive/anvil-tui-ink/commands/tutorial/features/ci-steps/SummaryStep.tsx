import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';

const COMMAND_REFERENCE = [
  { cmd: 'anvil check --all --ci', desc: 'CI mode with exit codes' },
  { cmd: 'anvil check --all --json', desc: 'Machine-readable output' },
  { cmd: 'anvil hooks install', desc: 'Install pre-commit hooks' },
  { cmd: 'anvil hooks status', desc: 'Check hook installation' },
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
        <Text color={theme.colours.steel}>{theme.icons.success} CI integration ready!</Text>
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
          {theme.icons.info} Workflow: Watch locally {theme.icons.arrow} Hook on commit{' '}
          {theme.icons.arrow} CI on PR
        </Text>
      </Box>

      <Box marginBottom={1}>
        <Text color={theme.colours.steel}>{theme.icons.success} Tutorial complete!</Text>
      </Box>
    </Box>
  );
}
