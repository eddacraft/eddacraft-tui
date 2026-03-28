import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../utils/theme.js';
import type { FilterStatus } from '../types.js';

interface FilterBarProps {
  currentFilter: FilterStatus;
  searchTerm: string;
  failedCount: number;
  currentFailureIndex: number;
}

const FILTER_OPTIONS: { key: FilterStatus; label: string; shortcut: string }[] = [
  { key: 'all', label: 'All', shortcut: 'a' },
  { key: 'passed', label: 'Passed', shortcut: 'p' },
  { key: 'failed', label: 'Failed', shortcut: 'f' },
  { key: 'skipped', label: 'Skipped', shortcut: 's' },
];

export function FilterBar({
  currentFilter,
  searchTerm,
  failedCount,
  currentFailureIndex,
}: FilterBarProps): React.ReactElement {
  return (
    <Box flexDirection="column" marginBottom={1}>
      <Box gap={2}>
        <Text color={theme.colours.smoke}>Filter:</Text>
        {FILTER_OPTIONS.map((opt) => (
          <Text
            key={opt.key}
            color={currentFilter === opt.key ? theme.colours.ember : theme.colours.smoke}
            bold={currentFilter === opt.key}
          >
            [{opt.shortcut}]{opt.label.slice(1)}
          </Text>
        ))}
      </Box>

      <Box gap={2}>
        <Box>
          <Text color={theme.colours.smoke}>Search: </Text>
          <Text color={theme.colours.ember}>{searchTerm || '(none)'}</Text>
        </Box>

        {failedCount > 0 && (
          <Box>
            <Text color={theme.colours.slag}>
              Failures: {currentFailureIndex + 1}/{failedCount}
            </Text>
          </Box>
        )}
      </Box>
    </Box>
  );
}
