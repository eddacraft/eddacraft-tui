import React from 'react';
import { Box, Text } from 'ink';
import { StatusBadge } from '../../../components/StatusBadge.js';
import { theme } from '../../../utils/theme.js';
import type { RecentResults, ValidationResult } from '../types.js';

interface ResultsPanelProps {
  data: RecentResults;
  focused: boolean;
}

function formatTimestamp(date: Date): string {
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMins < 1) return 'just now';
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString();
}

function shortenPath(path: string, maxLen = 30): string {
  if (path.length <= maxLen) return path;
  const filename = path.split('/').pop() ?? path;
  if (filename.length >= maxLen) return '...' + filename.slice(-maxLen + 3);
  return '...' + path.slice(-(maxLen - 3));
}

function ResultRow({ result }: { result: ValidationResult }): React.ReactElement {
  return (
    <Box gap={1}>
      <Box width={10}>
        <Text color={theme.colours.smoke}>{formatTimestamp(result.timestamp)}</Text>
      </Box>
      <Box width={30}>
        <Text color={theme.colours.ash}>{shortenPath(result.planPath)}</Text>
      </Box>
      <Box width={8}>
        <StatusBadge
          status={result.passed ? 'success' : 'error'}
          label={result.passed ? 'pass' : 'fail'}
        />
      </Box>
      <Text color={theme.colours.smoke}>
        {result.passedChecks}/{result.totalChecks}
      </Text>
    </Box>
  );
}

export function ResultsPanel({ data, focused }: ResultsPanelProps): React.ReactElement {
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
          {theme.icons.bullet} RECENT RESULTS
        </Text>
        {focused && <Text color={theme.colours.smoke}> (focused)</Text>}
      </Box>

      {!data.hasCache ? (
        <Box marginTop={0}>
          <Text color={theme.colours.smoke}>{theme.icons.info} No validation history yet</Text>
        </Box>
      ) : data.results.length === 0 ? (
        <Box marginTop={0}>
          <Text color={theme.colours.smoke}>{theme.icons.info} No recent validations</Text>
        </Box>
      ) : (
        <Box flexDirection="column" marginTop={0}>
          {data.results.map((result) => (
            <ResultRow key={result.id} result={result} />
          ))}
        </Box>
      )}
    </Box>
  );
}
