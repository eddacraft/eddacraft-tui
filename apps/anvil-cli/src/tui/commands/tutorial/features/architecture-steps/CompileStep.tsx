import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';

const REGO_EXAMPLE_LINES = [
  'package anvil.architecture.boundaries',
  '',
  'violation[msg] {',
  '  input.source.layer == "presentation"',
  '  input.target.layer == "data"',
  '  msg := sprintf("ARCH-001: %s cannot import from %s",',
  '    [input.source.path, input.target.path])',
  '}',
];

export function CompileStep(): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Compile Architecture Rules
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          After creating your architecture.yaml, compile it into enforceable rules:
        </Text>
        <Box marginLeft={2} marginTop={1}>
          <Text color={theme.colours.text}>anvil architecture compile</Text>
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          This produces Rego rules that enforce your boundary constraints.
        </Text>
        <Text color={theme.colours.ash}>The compilation pipeline:</Text>
        <Box flexDirection="column" marginLeft={2} marginTop={1}>
          <Text color={theme.colours.text}>
            architecture.yaml {theme.icons.arrow} dependency-constraints.json {theme.icons.arrow}{' '}
            Rego policies
          </Text>
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>Example generated rule:</Text>
        <Box
          flexDirection="column"
          marginLeft={2}
          marginTop={1}
          borderStyle="single"
          borderColor={theme.colours.charcoal}
          paddingX={1}
        >
          {REGO_EXAMPLE_LINES.map((line) => (
            <Text key={line} color={theme.colours.text}>
              {line}
            </Text>
          ))}
        </Box>
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
