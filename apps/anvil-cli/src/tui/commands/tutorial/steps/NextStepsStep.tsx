import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../utils/theme.js';
import { formatElapsedTime } from '../types.js';
import type { ScanResults } from '../types.js';
import { TutorialPicker } from '../components/TutorialPicker.js';
import type { TutorialOption } from '../components/TutorialPicker.js';

interface NextStepsStepProps {
  startedAt: Date;
  scanResults?: ScanResults;
  cleanupConfirming?: boolean;
  cleanupRequested?: boolean;
  completedTopics?: string[];
  tutorials: TutorialOption[];
  /** The topic this tutorial instance covers (default: 'core') */
  currentTopic?: string;
  onFinish?: () => void;
  /** @deprecated Cleanup is now handled by the parent Tutorial component */
  onCleanup?: () => void;
}

const WHAT_YOU_LEARNED = [
  'Scanned your project for anti-patterns and architecture issues',
  'Used watch mode to get real-time feedback',
  'Fixed an issue at save-time (the core loop)',
];

const CLEANUP_FILES = ['.anvil/tutorial/', '.anvil/tutorial-progress.json'];

export function NextStepsStep({
  startedAt,
  scanResults,
  cleanupConfirming = false,
  cleanupRequested = false,
  completedTopics,
  tutorials,
  currentTopic = 'core',
}: NextStepsStepProps): React.ReactElement {
  const elapsed = formatElapsedTime(startedAt);

  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Tutorial Complete!
        </Text>
        <Text color={theme.colours.smoke}> ({elapsed})</Text>
      </Box>

      {scanResults && scanResults.warningCount > 0 && (
        <Box marginBottom={1}>
          <Text color={theme.colours.ash}>
            Scanned {scanResults.fileCount} file{scanResults.fileCount !== 1 ? 's' : ''}, found{' '}
            {scanResults.warningCount} warning{scanResults.warningCount !== 1 ? 's' : ''}
          </Text>
        </Box>
      )}

      {/* What you learned */}
      <Box flexDirection="column" marginBottom={1}>
        <Box marginBottom={1}>
          <Text bold color={theme.colours.text}>
            What you learned
          </Text>
        </Box>
        <Box flexDirection="column" marginLeft={2}>
          {WHAT_YOU_LEARNED.map((item) => (
            <Text key={item} color={theme.colours.ash}>
              {theme.icons.check} {item}
            </Text>
          ))}
        </Box>
      </Box>

      {/* Explore further — currentTopic is 'core' because this step only
         appears at the end of the core tutorial (the first tutorial users complete) */}
      <Box flexDirection="column" marginBottom={1}>
        <TutorialPicker
          tutorials={tutorials}
          currentTopic={currentTopic}
          completedTopics={completedTopics}
        />
      </Box>

      {cleanupRequested ? (
        <Box marginTop={1}>
          <Text color={theme.colours.steel}>{theme.icons.success} Tutorial files removed</Text>
        </Box>
      ) : cleanupConfirming ? (
        <Box flexDirection="column" marginTop={1}>
          <Text color={theme.colours.molten}>Remove these tutorial files?</Text>
          <Box flexDirection="column" marginLeft={2} marginY={1}>
            {CLEANUP_FILES.map((file) => (
              <Text key={file} color={theme.colours.ash}>
                {theme.icons.bullet} {file}
              </Text>
            ))}
          </Box>
          <Text color={theme.colours.ash}>
            Press <Text color={theme.colours.ember}>c</Text> again to confirm
          </Text>
        </Box>
      ) : null}
    </Box>
  );
}
