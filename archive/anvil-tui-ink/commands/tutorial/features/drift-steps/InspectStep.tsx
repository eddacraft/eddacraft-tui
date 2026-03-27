import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';

const SNAPSHOT_DATA_LINES = [
  'Snapshot: baseline',
  'Captured: 2026-02-03T10:30:00Z',
  '',
  'Modules by layer:',
  '  api:           8 modules',
  '  services:     12 modules',
  '  repositories:  6 modules',
  '  utils:        16 modules',
  '',
  'Cross-boundary edges: 3',
];

const CROSS_BOUNDARY_EDGES = [
  'src/api/users.ts \u2192 src/repositories/user.repo.ts',
  'src/api/auth.ts \u2192 src/repositories/session.repo.ts',
  'src/services/billing.ts \u2192 src/api/webhooks.ts',
];

export function InspectStep(): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Inspect Snapshot
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          A snapshot stores the full dependency graph at a point in time.
        </Text>
        <Text color={theme.colours.ash}>Here&apos;s what the baseline snapshot contains:</Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Box
          flexDirection="column"
          marginLeft={2}
          borderStyle="single"
          borderColor={theme.colours.charcoal}
          paddingX={1}
        >
          {/* eslint-disable @eslint-react/no-array-index-key -- static display array with duplicate empty lines */}
          {SNAPSHOT_DATA_LINES.map((line, index) => (
            <Text
              key={`snapshot-${index}`}
              color={
                line.startsWith('Snapshot:') || line.startsWith('Captured:')
                  ? theme.colours.text
                  : line.startsWith('Modules by layer:') || line.startsWith('Cross-boundary')
                    ? theme.colours.text
                    : theme.colours.ash
              }
            >
              {line}
            </Text>
          ))}
          {/* eslint-enable @eslint-react/no-array-index-key */}
          <Box flexDirection="column" marginLeft={2}>
            {CROSS_BOUNDARY_EDGES.map((edge) => (
              <Text key={edge} color={theme.colours.molten}>
                {edge}
              </Text>
            ))}
          </Box>
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          These cross-boundary edges are what Anvil watches for.
        </Text>
        <Text color={theme.colours.ash}>When new ones appear, drift has occurred.</Text>
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
