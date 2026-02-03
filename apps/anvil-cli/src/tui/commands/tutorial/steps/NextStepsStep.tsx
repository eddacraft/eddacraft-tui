import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../utils/theme.js';
import { formatElapsedTime } from '../types.js';
import type { ScanResults } from '../types.js';

interface NextStepsStepProps {
  startedAt: Date;
  scanResults?: ScanResults;
  onCleanup: () => void;
  onFinish: () => void;
}

interface FeatureTutorial {
  command: string;
  description: string;
}

const FEATURE_TUTORIALS: FeatureTutorial[] = [
  { command: 'anvil tutorial policies', description: 'Write custom OPA/Rego rules' },
  { command: 'anvil tutorial architecture', description: 'Define architecture boundaries' },
  { command: 'anvil tutorial drift', description: 'Track architecture drift over time' },
  { command: 'anvil tutorial ci', description: 'Set up CI integration' },
];

const WHAT_YOU_LEARNED = [
  'Scanned your project for anti-patterns and architecture issues',
  'Used watch mode to get real-time feedback',
  'Fixed an issue at save-time (the core loop)',
];

export function NextStepsStep({ startedAt, scanResults }: NextStepsStepProps): React.ReactElement {
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
        <Box marginBottom={1}>
          <Text bold color={theme.colours.text}>
            Explore further
          </Text>
        </Box>
        <Box flexDirection="column" marginLeft={2}>
          {FEATURE_TUTORIALS.map((tutorial) => (
            <Box key={tutorial.command}>
              <Text color={theme.colours.molten}>{tutorial.command}</Text>
              <Text color={theme.colours.smoke}>
                {' '}
                {theme.icons.arrow} {tutorial.description}
              </Text>
            </Box>
          ))}
        </Box>
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

      {/* Footer */}
      <Box marginTop={1}>
        <Text color={theme.colours.smoke}>
          Press <Text color={theme.colours.text}>c</Text> to clean up tutorial files,{' '}
          <Text color={theme.colours.text}>q</Text> to exit
        </Text>
      </Box>
    </Box>
  );
}
