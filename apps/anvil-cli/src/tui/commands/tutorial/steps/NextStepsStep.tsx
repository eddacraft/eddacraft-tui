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
  tutorials: TutorialOption[];
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
  tutorials,
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

      {/* Explore further */}
      <Box flexDirection="column" marginBottom={1}>
        <TutorialPicker tutorials={tutorials} />
      </Box>

      {/* Resources */}
      <Box flexDirection="column" marginBottom={1}>
        <Box marginBottom={1}>
          <Text bold color={theme.colours.text}>
            Resources
          </Text>
        </Box>
        <Box flexDirection="column" marginLeft={2}>
          <Box>
            <Text color={theme.colours.ash}>Documentation: </Text>
            <Text color={theme.colours.molten}>https://anvil.eddacraft.com/docs</Text>
          </Box>
          <Box>
            <Text color={theme.colours.ash}>Help: </Text>
            <Text color={theme.colours.molten}>anvil --help</Text>
          </Box>
        </Box>
      </Box>

      {/* Cleanup section */}
      <Box marginTop={1} flexDirection="column">
        {cleanupRequested ? (
          <Text color={theme.colours.ember}>{theme.icons.success} Tutorial files cleaned up</Text>
        ) : cleanupConfirming ? (
          <Box flexDirection="column">
            <Text color={theme.colours.molten}>This will remove the following tutorial files:</Text>
            <Box flexDirection="column" marginLeft={2} marginY={1}>
              {CLEANUP_FILES.map((file) => (
                <Text key={file} color={theme.colours.ash}>
                  {theme.icons.bullet} {file}
                </Text>
              ))}
            </Box>
            <Text color={theme.colours.smoke}>
              Press <Text color={theme.colours.text}>c</Text> again to confirm,{' '}
              <Text color={theme.colours.text}>q</Text> to exit without cleaning
            </Text>
          </Box>
        ) : (
          <Text color={theme.colours.smoke}>
            Press <Text color={theme.colours.text}>c</Text> to clean up,{' '}
            <Text color={theme.colours.text}>q</Text> to exit, or a{' '}
            <Text color={theme.colours.text}>number</Text> to continue
          </Text>
        )}
      </Box>
    </Box>
  );
}
