import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../utils/theme.js';
import type { QuickWinsAnalysis, BatchGroup, QuickWinType } from '../../services/quick-wins.js';

interface QuickWinsPanelProps {
  analysis: QuickWinsAnalysis;
  focused?: boolean;
}

const quickWinTypeLabels: Record<QuickWinType, string> = {
  'test-file': 'Test Files',
  'type-definition': 'Type Definitions',
  'config-file': 'Config Files',
  'generated-code': 'Generated Code',
  migration: 'Legacy Code',
  'third-party': 'Third-Party',
  'legacy-code': 'Legacy Code',
};

const quickWinTypeIcons: Record<QuickWinType, string> = {
  'test-file': '🧪',
  'type-definition': '📘',
  'config-file': '⚙️',
  'generated-code': '🤖',
  migration: '🔄',
  'third-party': '📦',
  'legacy-code': '📚',
};

function BatchGroupRow({ group }: { group: BatchGroup }): React.ReactElement {
  const label = quickWinTypeLabels[group.type] || group.type;
  const icon = quickWinTypeIcons[group.type] || '•';

  return (
    <Box gap={1}>
      <Box width={3}>
        <Text>{icon}</Text>
      </Box>
      <Box width={20}>
        <Text color={theme.colours.ash}>{label}</Text>
      </Box>
      <Box width={10}>
        <Text color={theme.colours.smoke}>{group.patternId}</Text>
      </Box>
      <Box width={8}>
        <Text color={theme.colours.ember} bold>
          {group.count} issues
        </Text>
      </Box>
    </Box>
  );
}

function ProgressBar({
  current,
  total,
  width = 30,
}: {
  current: number;
  total: number;
  width?: number;
}): React.ReactElement {
  if (total === 0) {
    return (
      <Box width={width}>
        <Text color={theme.colours.smoke}>{'─'.repeat(width)}</Text>
      </Box>
    );
  }

  const percentage = Math.round((current / total) * 100);
  const filled = Math.floor((current / total) * width);
  const empty = width - filled;

  return (
    <Box gap={1}>
      <Box width={width}>
        <Text color={theme.colours.ember}>{'━'.repeat(filled)}</Text>
        <Text color={theme.colours.charcoal}>{'─'.repeat(empty)}</Text>
      </Box>
      <Text color={theme.colours.ash}>{percentage}%</Text>
    </Box>
  );
}

export function QuickWinsPanel({
  analysis,
  focused = false,
}: QuickWinsPanelProps): React.ReactElement {
  const borderColour = focused ? theme.colours.ember : theme.colours.charcoal;
  const titleColour = focused ? theme.colours.ember : theme.colours.ash;

  // Show top 5 batch groups
  const topBatches = analysis.batchGroups.slice(0, 5);
  const hasQuickWins = analysis.suppressable > 0;

  return (
    <Box
      flexDirection="column"
      borderStyle={focused ? 'double' : 'single'}
      borderColor={borderColour}
      paddingX={1}
      paddingY={0}
      marginTop={1}
    >
      <Box justifyContent="space-between" marginBottom={0}>
        <Box>
          <Text bold color={titleColour}>
            {theme.icons.check} QUICK WINS
          </Text>
          {focused && <Text color={theme.colours.smoke}> (focused)</Text>}
        </Box>
        {hasQuickWins && (
          <Text color={theme.colours.smoke}>
            {analysis.suppressable} of {analysis.totalWarnings} suppressable
          </Text>
        )}
      </Box>

      {!hasQuickWins ? (
        <Box marginTop={0}>
          <Text color={theme.colours.smoke}>
            {theme.icons.info} No quick wins identified - all violations require review
          </Text>
        </Box>
      ) : (
        <Box flexDirection="column" marginTop={0}>
          <Box marginBottom={0}>
            <ProgressBar
              current={analysis.suppressable}
              total={analysis.totalWarnings}
              width={40}
            />
          </Box>

          {topBatches.length > 0 && (
            <Box flexDirection="column" marginTop={1}>
              <Text color={theme.colours.smoke} bold>
                Batch Suppressions Available:
              </Text>
              {topBatches.map((group, idx) => (
                <BatchGroupRow key={`${group.key}-${idx}`} group={group} />
              ))}

              {analysis.batchGroups.length > 5 && (
                <Box marginTop={0}>
                  <Text color={theme.colours.smoke}>
                    {theme.icons.info} +{analysis.batchGroups.length - 5} more batch groups
                    available
                  </Text>
                </Box>
              )}
            </Box>
          )}

          <Box marginTop={1}>
            <Text color={theme.colours.smoke}>
              {theme.icons.info} Tip: Use{' '}
              <Text color={theme.colours.ember}>anvil suppress --batch</Text> to apply batch
              suppressions
            </Text>
          </Box>
        </Box>
      )}
    </Box>
  );
}
