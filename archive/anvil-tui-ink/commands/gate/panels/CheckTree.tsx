import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../utils/theme.js';
import { type CheckResult, getStatusIcon, getStatusColour, formatScore } from '../types.js';

interface CheckTreeProps {
  checks: CheckResult[];
  selectedIndex: number;
  expandedChecks: Set<string>;
  onToggleExpand?: (checkId: string) => void;
}

function CheckRow({
  check,
  isSelected,
  isExpanded,
}: {
  check: CheckResult;
  isSelected: boolean;
  isExpanded: boolean;
}): React.ReactElement {
  const statusColour = getStatusColour(check.status);
  const statusIcon = getStatusIcon(check.status);

  return (
    <Box flexDirection="column">
      <Box>
        {isSelected && <Text color={theme.colours.ember}>{theme.icons.arrow} </Text>}
        {!isSelected && <Text> </Text>}

        <Text color={statusColour}>{statusIcon} </Text>

        <Box width={16}>
          <Text color={isSelected ? theme.colours.ember : theme.colours.ash}>{check.name}</Text>
        </Box>

        <Box width={8}>
          <Text color={statusColour}>{formatScore(check.score)}</Text>
        </Box>

        <Box width={10}>
          <Text color={statusColour}>{check.status}</Text>
        </Box>

        {check.details && check.details.length > 0 && (
          <Text color={theme.colours.smoke}>
            {isExpanded ? '▼' : '▶'} ({check.details.length})
          </Text>
        )}
      </Box>

      {isExpanded && check.details && (
        <Box flexDirection="column" marginLeft={4}>
          {check.details.slice(0, 5).map((detail) => (
            <Text key={detail} color={theme.colours.smoke}>
              └─ {detail}
            </Text>
          ))}
          {check.details.length > 5 && (
            <Text color={theme.colours.smoke}>└─ ... and {check.details.length - 5} more</Text>
          )}
        </Box>
      )}
    </Box>
  );
}

export function CheckTree({
  checks,
  selectedIndex,
  expandedChecks,
}: CheckTreeProps): React.ReactElement {
  if (checks.length === 0) {
    return (
      <Box paddingX={1}>
        <Text color={theme.colours.smoke}>{theme.icons.info} No checks to display</Text>
      </Box>
    );
  }

  return (
    <Box flexDirection="column">
      {checks.map((check, idx) => (
        <CheckRow
          key={check.id}
          check={check}
          isSelected={idx === selectedIndex}
          isExpanded={expandedChecks.has(check.id)}
        />
      ))}
    </Box>
  );
}
