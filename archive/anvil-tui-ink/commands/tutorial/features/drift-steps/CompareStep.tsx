import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';

const DRIFT_REPORT_HEADER = ['Drift Report: baseline \u2192 current', ''];

const NEW_EDGES = [
  '+ src/api/orders.ts \u2192 src/repositories/order.repo.ts',
  '+ src/utils/logger.ts \u2192 src/services/audit.ts',
];

const REMOVED_EDGES = ['- src/api/auth.ts \u2192 src/repositories/session.repo.ts (fixed!)'];

export function CompareStep(): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Compare Snapshots
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>Compare two snapshots to see exactly what changed:</Text>
        <Box marginLeft={2} marginTop={1}>
          <Text color={theme.colours.text}>anvil drift compare baseline current</Text>
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>Example drift report:</Text>
        <Box
          flexDirection="column"
          marginLeft={2}
          marginTop={1}
          borderStyle="single"
          borderColor={theme.colours.charcoal}
          paddingX={1}
        >
          {DRIFT_REPORT_HEADER.map((line) => (
            <Text key={line} color={theme.colours.text}>
              {line}
            </Text>
          ))}
          <Text color={theme.colours.text}>New edges: 2</Text>
          <Box flexDirection="column" marginLeft={2}>
            {NEW_EDGES.map((edge) => (
              <Text key={edge} color={theme.colours.molten}>
                {edge}
              </Text>
            ))}
          </Box>
          <Text> </Text>
          <Text color={theme.colours.text}>Removed edges: 1</Text>
          <Box flexDirection="column" marginLeft={2}>
            {REMOVED_EDGES.map((edge) => (
              <Text key={edge} color={theme.colours.steel}>
                {edge}
              </Text>
            ))}
          </Box>
          <Text> </Text>
          <Text color={theme.colours.molten}>Net drift: +1 cross-boundary edge</Text>
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          Positive drift means architecture is degrading — new unwanted edges.
        </Text>
        <Text color={theme.colours.ash}>
          Negative drift means it&apos;s improving — boundary violations removed.
        </Text>
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
