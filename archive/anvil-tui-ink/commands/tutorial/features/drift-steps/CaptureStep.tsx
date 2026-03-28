import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';

const SNAPSHOT_OUTPUT_LINES = [
  `${theme.icons.success} Snapshot captured: baseline`,
  '  Modules: 42',
  '  Import edges: 187',
  '  Stored at: .anvil/snapshots/baseline.json',
];

export function CaptureStep(): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Capture Baseline Snapshot
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>Capturing a snapshot of your current architecture...</Text>
        <Box marginLeft={2} marginTop={1}>
          <Text color={theme.colours.text}>anvil drift snapshot --name baseline</Text>
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>A snapshot records:</Text>
        <Box flexDirection="column" marginLeft={2} marginTop={1}>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} Module count — every file Anvil tracks
          </Text>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} Import edges — every dependency between modules
          </Text>
          <Text color={theme.colours.ash}>
            {theme.icons.bullet} Dependency graph — the full picture of how modules connect
          </Text>
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
          {SNAPSHOT_OUTPUT_LINES.map((line) => (
            <Text
              key={line}
              color={line.includes('Snapshot captured') ? theme.colours.steel : theme.colours.ash}
            >
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
