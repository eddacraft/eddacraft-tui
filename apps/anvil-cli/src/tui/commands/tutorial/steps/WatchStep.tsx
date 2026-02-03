import React, { useState, useEffect, useRef } from 'react';
import { Box, Text } from 'ink';
import { getWorkspaceRoot } from '../../../../utils/file-io.js';
import { theme } from '../../../utils/theme.js';
import { createTutorialWatcher, WATCHED_PATTERNS } from './watch-project.js';
import type { WatchEvent } from './watch-project.js';

// Re-export so consumers can import from this module
export { createTutorialWatcher } from './watch-project.js';

interface WatchStepProps {
  onComplete: () => void;
  watchTriggered?: boolean;
}

const HINT_TIMEOUT_MS = 30_000;

type WatchPhase = 'watching' | 'detected';

export function WatchStep({ onComplete, watchTriggered }: WatchStepProps): React.ReactElement {
  const [phase, setPhase] = useState<WatchPhase>(watchTriggered ? 'detected' : 'watching');
  const [detectedFile, setDetectedFile] = useState<string | undefined>();
  const [showHint, setShowHint] = useState(false);
  const onCompleteRef = useRef(onComplete);

  // Keep ref in sync so the watcher callback always uses the latest onComplete.
  useEffect(() => {
    onCompleteRef.current = onComplete;
  }, [onComplete]);

  // Start the file watcher (only when not already triggered).
  useEffect(() => {
    if (watchTriggered) return;

    const workspaceRoot = getWorkspaceRoot();
    const watcher = createTutorialWatcher(workspaceRoot, (event: WatchEvent) => {
      setDetectedFile(event.path);
      setPhase('detected');
      onCompleteRef.current();
    });

    return () => {
      watcher.close();
    };
  }, [watchTriggered]);

  // 30-second hint timer.
  useEffect(() => {
    if (watchTriggered || phase === 'detected') return;

    const timer = setTimeout(() => {
      setShowHint(true);
    }, HINT_TIMEOUT_MS);

    return () => {
      clearTimeout(timer);
    };
  }, [watchTriggered, phase]);

  // --- Already-triggered state (navigated back) ---
  if (watchTriggered && phase === 'detected') {
    return (
      <Box flexDirection="column" paddingX={2}>
        <Box marginBottom={1}>
          <Text bold color={theme.colours.ember}>
            Watch Mode
          </Text>
        </Box>

        <Box marginBottom={1}>
          <Text color={theme.colours.steel}>{theme.icons.check} Watch mode detected a change!</Text>
        </Box>

        <Box marginTop={2}>
          <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
        </Box>
      </Box>
    );
  }

  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Watch Mode
        </Text>
      </Box>

      {phase === 'watching' && (
        <Box flexDirection="column">
          <Box marginBottom={1}>
            <Text color={theme.colours.ember}>Watch mode is active </Text>
            <Text color={theme.colours.ember}>{theme.icons.running}</Text>
          </Box>

          <Box flexDirection="column" marginLeft={2} marginBottom={1}>
            <Text color={theme.colours.ash}>Watching for file changes in your project...</Text>
            <Text color={theme.colours.smoke}>Patterns: {WATCHED_PATTERNS.join(', ')}</Text>
          </Box>

          <Box marginLeft={2}>
            <Text color={theme.colours.text}>
              Edit any source file and save to see Anvil in action.
            </Text>
          </Box>

          {showHint && (
            <Box marginTop={1} marginLeft={2}>
              <Text color={theme.colours.molten}>
                Tip: Try editing any .ts file in your project and pressing save
              </Text>
            </Box>
          )}
        </Box>
      )}

      {phase === 'detected' && (
        <Box flexDirection="column">
          <Box marginBottom={1}>
            <Text color={theme.colours.steel}>
              {theme.icons.check} Change detected: {detectedFile}
            </Text>
          </Box>

          <Box marginLeft={2} marginBottom={1}>
            <Text color={theme.colours.ash}>Anvil validated your change</Text>
          </Box>

          <Box marginTop={2}>
            <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
          </Box>
        </Box>
      )}
    </Box>
  );
}
