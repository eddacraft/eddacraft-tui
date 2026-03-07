import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../utils/theme.js';
import {
  type CheckResult,
  getStatusIcon,
  getStatusColour,
  formatScore,
  formatDuration,
} from '../types.js';

interface DetailPanelProps {
  check: CheckResult | null;
}

export function DetailPanel({ check }: DetailPanelProps): React.ReactElement {
  if (!check) {
    return (
      <Box
        flexDirection="column"
        borderStyle="single"
        borderColor={theme.colours.charcoal}
        paddingX={1}
        minHeight={10}
      >
        <Text color={theme.colours.smoke}>{theme.icons.info} Select a check to view details</Text>
      </Box>
    );
  }

  const statusColour = getStatusColour(check.status);
  const statusIcon = getStatusIcon(check.status);

  return (
    <Box
      flexDirection="column"
      borderStyle="single"
      borderColor={theme.colours.charcoal}
      paddingX={1}
    >
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          {theme.icons.bullet} {check.name.toUpperCase()}
        </Text>
      </Box>

      <Box gap={1}>
        <Text color={theme.colours.smoke}>Status:</Text>
        <Text color={statusColour}>
          {statusIcon} {check.status}
        </Text>
      </Box>

      <Box gap={1}>
        <Text color={theme.colours.smoke}>Score:</Text>
        <Text color={statusColour}>{formatScore(check.score)}</Text>
      </Box>

      {check.duration !== undefined && (
        <Box gap={1}>
          <Text color={theme.colours.smoke}>Duration:</Text>
          <Text color={theme.colours.ash}>{formatDuration(check.duration)}</Text>
        </Box>
      )}

      {check.category && (
        <Box gap={1}>
          <Text color={theme.colours.smoke}>Category:</Text>
          <Text color={theme.colours.ash}>{check.category}</Text>
        </Box>
      )}

      <Box marginTop={1}>
        <Text color={theme.colours.smoke}>Message:</Text>
      </Box>
      <Box marginLeft={2}>
        <Text color={theme.colours.ash}>{check.message}</Text>
      </Box>

      {check.details && check.details.length > 0 && (
        <>
          <Box marginTop={1}>
            <Text color={theme.colours.smoke}>Details ({check.details.length}):</Text>
          </Box>
          <Box flexDirection="column" marginLeft={2}>
            {check.details.map((detail) => (
              <Text key={detail} color={theme.colours.ash}>
                • {detail}
              </Text>
            ))}
          </Box>
        </>
      )}
    </Box>
  );
}
