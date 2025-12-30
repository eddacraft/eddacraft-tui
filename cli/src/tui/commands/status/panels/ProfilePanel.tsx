import React from 'react';
import { Box, Text } from 'ink';
import { StatusBadge } from '../../../components/StatusBadge.js';
import { theme } from '../../../utils/theme.js';
import type { RepoProfile } from '../types.js';

interface ProfilePanelProps {
  data: RepoProfile;
  focused: boolean;
}

export function ProfilePanel({ data, focused }: ProfilePanelProps): React.ReactElement {
  const borderColour = focused ? theme.colours.primary : theme.colours.border;

  return (
    <Box
      flexDirection="column"
      borderStyle="round"
      borderColor={borderColour}
      paddingX={1}
      paddingY={0}
      marginTop={1}
    >
      <Box marginBottom={0}>
        <Text bold color={focused ? theme.colours.primary : theme.colours.text}>
          {theme.icons.bullet} Configuration
        </Text>
        {focused && <Text color={theme.colours.muted}> (focused)</Text>}
      </Box>

      {!data.hasConfig ? (
        <Box marginTop={0}>
          <Text color={theme.colours.warning}>
            {theme.icons.warning} No .anvilrc found — run `anvil init`
          </Text>
        </Box>
      ) : (
        <Box flexDirection="column" marginTop={0}>
          <Box gap={1}>
            <Text color={theme.colours.muted}>Plans:</Text>
            <Text>{data.planningDir ?? 'not set'}</Text>
          </Box>

          <Box gap={1}>
            <Text color={theme.colours.muted}>Format:</Text>
            <Text>{data.format ?? 'auto-detect'}</Text>
          </Box>

          {data.coverageThreshold !== undefined && (
            <Box gap={1}>
              <Text color={theme.colours.muted}>Coverage:</Text>
              <Text>{data.coverageThreshold}%</Text>
            </Box>
          )}

          <Box gap={1} marginTop={0}>
            <Text color={theme.colours.muted}>Checks:</Text>
            <Box gap={1}>
              {data.checks.length === 0 ? (
                <Text color={theme.colours.muted}>none</Text>
              ) : (
                data.checks.map((check) => (
                  <Box key={check.name} marginRight={1}>
                    <StatusBadge
                      status={check.enabled ? 'success' : 'skipped'}
                      label={check.name}
                    />
                  </Box>
                ))
              )}
            </Box>
          </Box>
        </Box>
      )}
    </Box>
  );
}
