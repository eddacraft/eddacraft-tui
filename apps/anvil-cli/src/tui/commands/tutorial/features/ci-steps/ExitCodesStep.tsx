import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';

const EXIT_CODE_EXAMPLES = [
  'npx anvil check --all --ci',
  `# Exit code 0 ${theme.icons.arrow} PR check passes ${theme.icons.success}`,
  `# Exit code 1 ${theme.icons.arrow} PR check fails ${theme.icons.cross}`,
];

export function ExitCodesStep(): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Exit Codes
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>Anvil uses exit codes to signal pass/fail to CI:</Text>
        <Box flexDirection="column" marginLeft={2} marginTop={1}>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} <Text color={theme.colours.steel}>Exit 0</Text> — All checks passed
            (no warnings or only info-level)
          </Text>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} <Text color={theme.colours.slag}>Exit 1</Text> — Blocking warnings
            found (architecture violations, errors)
          </Text>
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>How CI uses this:</Text>
        <Box
          flexDirection="column"
          marginLeft={2}
          marginTop={1}
          borderStyle="single"
          borderColor={theme.colours.charcoal}
          paddingX={1}
        >
          {EXIT_CODE_EXAMPLES.map((line, index) => (
            <Text
              key={index}
              color={line.startsWith('#') ? theme.colours.smoke : theme.colours.text}
            >
              {line}
            </Text>
          ))}
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          Warnings (non-blocking) do not fail CI, only errors do.
        </Text>
        <Text color={theme.colours.ash}>
          Use <Text color={theme.colours.text}>--json</Text> for machine-readable output in custom
          integrations.
        </Text>
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
