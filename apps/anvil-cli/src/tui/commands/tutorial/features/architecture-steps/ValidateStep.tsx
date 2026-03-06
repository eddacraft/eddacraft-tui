import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';

const VIOLATION_EXAMPLE_LINES = [
  'ARCH-001: src/api/handler.ts cannot import from src/domain/entity.ts',
  '  Layer "presentation" must not depend on "domain" directly.',
  '  Fix: Route through the "application" layer instead.',
  '',
  'ARCH-002: src/utils/helpers.ts imports from src/services/auth.ts',
  '  Layer "shared" must not depend on "business".',
  '  Fix: Move shared logic into the correct layer.',
];

export function ValidateStep(): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Validate Boundaries
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>Run validation to check for boundary violations:</Text>
        <Box marginLeft={2} marginTop={1}>
          <Text color={theme.colours.text}>anvil architecture validate</Text>
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>Example output:</Text>
        <Box
          flexDirection="column"
          marginLeft={2}
          marginTop={1}
          borderStyle="single"
          borderColor={theme.colours.charcoal}
          paddingX={1}
        >
          {VIOLATION_EXAMPLE_LINES.map((line) => (
            <Text
              key={line}
              color={line.startsWith('ARCH-') ? theme.colours.molten : theme.colours.ash}
            >
              {line}
            </Text>
          ))}
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.text}>When violations are found, you can:</Text>
        <Box flexDirection="column" marginLeft={2} marginTop={1}>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} <Text color={theme.colours.text}>Fix</Text> — refactor to respect
            layer boundaries
          </Text>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} <Text color={theme.colours.text}>Suppress</Text> — add an inline
            annotation to allow it
          </Text>
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>Suppression syntax:</Text>
        <Box marginLeft={2} marginTop={1}>
          <Text color={theme.colours.text}>{'// @anvil-ignore ARCH-001'}</Text>
        </Box>
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
