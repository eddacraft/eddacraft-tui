import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';
import { MermaidDiagram } from '../../../../components/MermaidDiagram.js';

const LAYERS_MERMAID = `graph TD
  watch["Watch Mode (instant, local)"]
  precommit["Pre-commit (before push)"]
  ci["CI Pipeline (before merge)"]
  watch --> precommit --> ci`;

export function HooksStep(): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Git Hooks
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          Pre-commit hooks complement CI by catching issues before push:
        </Text>
        <Box marginLeft={2} marginTop={1}>
          <Text color={theme.colours.text}>anvil hooks install</Text>
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          This installs a pre-commit hook that runs{' '}
          <Text color={theme.colours.text}>anvil check --changed</Text>
        </Text>
        <Text color={theme.colours.ash}>
          on every commit, checking only the files you modified.
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.text}>Layered protection:</Text>
        <Box marginLeft={2} marginTop={1}>
          <MermaidDiagram definition={LAYERS_MERMAID} asciiOptions={{ paddingX: 2, paddingY: 1 }} />
        </Box>
      </Box>

      <Box marginBottom={1}>
        <Text color={theme.colours.ash}>
          {theme.icons.info} Tip: Pre-commit hooks are optional but catch issues before they reach
          CI
        </Text>
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
