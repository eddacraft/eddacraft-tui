import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';

const CONFIG_JSON_LINES = [
  '{',
  '  "policies": {',
  '    "max_file_length": {',
  '      "max_lines": 200',
  '    }',
  '  }',
  '}',
];

const MORE_IDEAS = [
  'Banned imports (prevent importing from internal modules)',
  'Naming conventions (enforce file/class naming patterns)',
  'Max function complexity (limit cyclomatic complexity)',
];

export function CustomiseStep(): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Customise Your Policy
        </Text>
      </Box>

      {/* Edit threshold */}
      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>You can adjust the threshold by editing:</Text>
        <Box marginLeft={2}>
          <Text color={theme.colours.text}>.anvil/policies/max_file_length.rego</Text>
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1} marginLeft={2}>
        <Box>
          <Text color={theme.colours.smoke}>Change: </Text>
          <Text color={theme.colours.text}>default max_lines := 300</Text>
        </Box>
        <Box>
          <Text color={theme.colours.smoke}>To: </Text>
          <Text color={theme.colours.text}>default max_lines := 200</Text>
        </Box>
      </Box>

      {/* Config via gate-config.json */}
      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>Or configure via gate-config.json:</Text>
        <Box
          flexDirection="column"
          marginLeft={2}
          marginTop={1}
          borderStyle="single"
          borderColor={theme.colours.charcoal}
          paddingX={1}
        >
          {CONFIG_JSON_LINES.map((line) => (
            <Text key={line} color={theme.colours.text}>
              {line}
            </Text>
          ))}
        </Box>
      </Box>

      {/* More ideas */}
      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.text}>More policy ideas:</Text>
        <Box flexDirection="column" marginLeft={2}>
          {MORE_IDEAS.map((idea) => (
            <Text key={idea} color={theme.colours.ash}>
              {theme.icons.bullet} {idea}
            </Text>
          ))}
        </Box>
      </Box>

      {/* Completion */}
      <Box marginBottom={1}>
        <Text color={theme.colours.steel}>
          {theme.icons.success} Tutorial complete! Your policy is active.
        </Text>
      </Box>
    </Box>
  );
}
