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
  const borderColour = focused ? theme.colours.ember : theme.colours.charcoal;

  return (
    <Box
      flexDirection="column"
      borderStyle={focused ? 'double' : 'single'}
      borderColor={borderColour}
      paddingX={1}
      paddingY={0}
      marginTop={1}
    >
      <Box marginBottom={0}>
        <Text bold color={focused ? theme.colours.ember : theme.colours.ash}>
          {theme.icons.bullet} CONFIGURATION
        </Text>
        {focused && <Text color={theme.colours.smoke}> (focused)</Text>}
      </Box>

      {!data.hasConfig ? (
        <Box marginTop={0}>
          <Text color={theme.colours.molten}>
            {theme.icons.warning} No .anvilrc found — run `anvil init`
          </Text>
        </Box>
      ) : (
        <Box flexDirection="column" marginTop={0}>
          <Box gap={1}>
            <Text color={theme.colours.smoke}>Plans:</Text>
            <Text color={theme.colours.ash}>{data.planningDir ?? 'not set'}</Text>
          </Box>

          <Box gap={1}>
            <Text color={theme.colours.smoke}>Format:</Text>
            <Text color={theme.colours.ash}>{data.format ?? 'auto-detect'}</Text>
          </Box>

          {data.coverageThreshold !== undefined && (
            <Box gap={1}>
              <Text color={theme.colours.smoke}>Coverage:</Text>
              <Text color={theme.colours.ash}>{data.coverageThreshold}%</Text>
            </Box>
          )}

          <Box gap={1} marginTop={0}>
            <Text color={theme.colours.smoke}>Checks:</Text>
            <Box gap={1}>
              {data.checks.length === 0 ? (
                <Text color={theme.colours.smoke}>none</Text>
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
