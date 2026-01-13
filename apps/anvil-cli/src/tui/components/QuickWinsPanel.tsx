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
<<<<<<< HEAD:apps/anvil-cli/src/tui/components/QuickWinsPanel.tsx
  migration: 'Legacy Code',
=======
  'migration': 'Legacy Code',
>>>>>>> 5af1817 (feat(cli): Add interactive results dashboard TUI (IFR-005)):cli/src/tui/components/QuickWinsPanel.tsx
  'third-party': 'Third-Party',
  'legacy-code': 'Legacy Code',
};

const quickWinTypeIcons: Record<QuickWinType, string> = {
  'test-file': '🧪',
  'type-definition': '📘',
  'config-file': '⚙️',
  'generated-code': '🤖',
<<<<<<< HEAD:apps/anvil-cli/src/tui/components/QuickWinsPanel.tsx
  migration: '🔄',
=======
  'migration': '🔄',
>>>>>>> 5af1817 (feat(cli): Add interactive results dashboard TUI (IFR-005)):cli/src/tui/components/QuickWinsPanel.tsx
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
<<<<<<< HEAD:apps/anvil-cli/src/tui/components/QuickWinsPanel.tsx
        <Text color={theme.colours.smoke}>{group.patternId}</Text>
=======
        <Text color={theme.colours.smoke}>
          {group.patternId}
        </Text>
>>>>>>> 5af1817 (feat(cli): Add interactive results dashboard TUI (IFR-005)):cli/src/tui/components/QuickWinsPanel.tsx
      </Box>
      <Box width={8}>
        <Text color={theme.colours.ember} bold>
          {group.count} issues
        </Text>
      </Box>
    </Box>
  );
}

<<<<<<< HEAD:apps/anvil-cli/src/tui/components/QuickWinsPanel.tsx
function ProgressBar({
  current,
  total,
  width = 30,
}: {
  current: number;
  total: number;
  width?: number;
}): React.ReactElement {
=======
function ProgressBar({ current, total, width = 30 }: { current: number; total: number; width?: number }): React.ReactElement {
>>>>>>> 5af1817 (feat(cli): Add interactive results dashboard TUI (IFR-005)):cli/src/tui/components/QuickWinsPanel.tsx
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

<<<<<<< HEAD:apps/anvil-cli/src/tui/components/QuickWinsPanel.tsx
export function QuickWinsPanel({
  analysis,
  focused = false,
}: QuickWinsPanelProps): React.ReactElement {
=======
export function QuickWinsPanel({ analysis, focused = false }: QuickWinsPanelProps): React.ReactElement {
>>>>>>> 5af1817 (feat(cli): Add interactive results dashboard TUI (IFR-005)):cli/src/tui/components/QuickWinsPanel.tsx
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
<<<<<<< HEAD:apps/anvil-cli/src/tui/components/QuickWinsPanel.tsx
                    {theme.icons.info} +{analysis.batchGroups.length - 5} more batch groups
                    available
=======
                    {theme.icons.info} +{analysis.batchGroups.length - 5} more batch groups available
>>>>>>> 5af1817 (feat(cli): Add interactive results dashboard TUI (IFR-005)):cli/src/tui/components/QuickWinsPanel.tsx
                  </Text>
                </Box>
              )}
            </Box>
          )}

          <Box marginTop={1}>
            <Text color={theme.colours.smoke}>
              {theme.icons.info} Tip: Use{' '}
<<<<<<< HEAD:apps/anvil-cli/src/tui/components/QuickWinsPanel.tsx
              <Text color={theme.colours.ember}>anvil suppress --batch</Text> to apply batch
              suppressions
=======
              <Text color={theme.colours.ember}>anvil suppress --batch</Text> to apply batch suppressions
>>>>>>> 5af1817 (feat(cli): Add interactive results dashboard TUI (IFR-005)):cli/src/tui/components/QuickWinsPanel.tsx
            </Text>
          </Box>
        </Box>
      )}
    </Box>
  );
}
