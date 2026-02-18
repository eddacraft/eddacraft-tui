import React, { useState, useEffect, useRef } from 'react';
import { Box, Text, useInput } from 'ink';
import { Spinner } from '../../../components/Spinner.js';
import { getWorkspaceRoot } from '../../../../utils/file-io.js';
import { theme } from '../../../utils/theme.js';
import { createTutorialWatcher, WATCHED_PATTERNS } from './watch-project.js';
import type { WatchEvent } from './watch-project.js';
import type { ScanResults } from '../types.js';

// Re-export so consumers can import from this module
export { createTutorialWatcher } from './watch-project.js';

interface WatchStepProps {
  onComplete: () => void;
  watchTriggered?: boolean;
  scanResults?: ScanResults;
}

const HINT_TIMEOUTS_MS = [10_000, 20_000, 30_000] as const;

type WatchPhase = 'initialising' | 'watching' | 'detected';

export function WatchStep({
  onComplete,
  watchTriggered,
  scanResults,
}: WatchStepProps): React.ReactElement {
  const [phase, setPhase] = useState<WatchPhase>(watchTriggered ? 'detected' : 'initialising');
  const [detectedFile, setDetectedFile] = useState<string | undefined>();
  const [hintLevel, setHintLevel] = useState(0);
  const [workspaceRoot, setWorkspaceRoot] = useState<string>('');
  const onCompleteRef = useRef(onComplete);

  // Keep ref in sync so the watcher callback always uses the latest onComplete.
  useEffect(() => {
    onCompleteRef.current = onComplete;
  }, [onComplete]);

  // Start the file watcher (only when not already triggered).
  useEffect(() => {
    if (watchTriggered) return;

    const root = getWorkspaceRoot();
    setWorkspaceRoot(root);

    const watcher = createTutorialWatcher(root, (event: WatchEvent) => {
      setDetectedFile(event.path);
      setPhase('detected');
      onCompleteRef.current();
    });

    // Wait for chokidar to finish its initial scan before telling the user
    // to edit files — edits before ready may be silently missed.
    // Guard: only transition initialising → watching so a file event that
    // arrives before ready doesn't get overwritten back to 'watching'.
    let cancelled = false;
    const transitionToWatching = () => {
      if (!cancelled) {
        setPhase((prev) => (prev === 'initialising' ? 'watching' : prev));
      }
    };

    watcher.ready.then(transitionToWatching).catch(transitionToWatching);

    return () => {
      cancelled = true;
      watcher.close();
    };
  }, [watchTriggered]);

  // Progressive hint timers — only start after watcher is ready.
  useEffect(() => {
    if (watchTriggered || phase !== 'watching') return;

    const timers = HINT_TIMEOUTS_MS.map((ms, idx) => setTimeout(() => setHintLevel(idx + 1), ms));

    return () => {
      timers.forEach(clearTimeout);
    };
  }, [watchTriggered, phase]);

  // Allow pressing 's' to skip when stuck.
  useInput((input) => {
    if (input === 's' && phase === 'watching' && hintLevel >= 3) {
      setPhase('detected');
      onCompleteRef.current();
    }
  });

  // Build the suggested file instruction.
  const suggestedFile = scanResults?.topWarnings?.[0]
    ? `${scanResults.topWarnings[0].file} (line ${scanResults.topWarnings[0].line})`
    : undefined;

  const editInstruction = suggestedFile ? suggestedFile : 'any .ts or .js file in your project';

  const editHint = suggestedFile
    ? "This file has a warning you'll fix in the next step"
    : undefined;

  // --- Already-triggered state (navigated back) ---
  if (watchTriggered && phase === 'detected') {
    return (
      <Box flexDirection="column" paddingX={2}>
        <Box marginBottom={1}>
          <Text bold color={theme.colours.ember}>
            {theme.icons.success} Change detected!
          </Text>
        </Box>

        <Box marginBottom={1}>
          <Text color={theme.colours.ash}>Watch mode detected a file change successfully.</Text>
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

      {phase === 'initialising' && (
        <Box flexDirection="column">
          <Box>
            <Spinner label="Initialising file watcher..." />
          </Box>
        </Box>
      )}

      {phase === 'watching' && (
        <Box flexDirection="column">
          <Box marginBottom={1}>
            <Text color={theme.colours.ember}>Watch mode is ready </Text>
            <Text color={theme.colours.ember}>{theme.icons.running}</Text>
          </Box>

          <Box flexDirection="column" marginLeft={2} marginBottom={1}>
            <Text color={theme.colours.text}>
              Anvil is now watching for file changes. Here&apos;s what to do:
            </Text>
          </Box>

          <Box flexDirection="column" marginLeft={4} marginBottom={1}>
            <Text color={theme.colours.text}>
              1. Open your editor or IDE (keep this terminal visible)
            </Text>
            <Text color={theme.colours.text}>
              2. Edit and save: <Text color={theme.colours.molten}>{editInstruction}</Text>
            </Text>
            {editHint && (
              <Text color={theme.colours.smoke}>
                {'   '}
                {editHint}
              </Text>
            )}
            <Text color={theme.colours.text}>
              3. Come back here — you&apos;ll see Anvil detect the change below
            </Text>
          </Box>

          <Box flexDirection="column" marginLeft={2} marginBottom={1}>
            <Text color={theme.colours.smoke}>Watching: {workspaceRoot}</Text>
            <Text color={theme.colours.smoke}>Patterns: {WATCHED_PATTERNS.join(', ')}</Text>
          </Box>

          <Box marginLeft={2}>
            <Spinner label="Waiting for a file change..." />
          </Box>

          {hintLevel >= 1 && (
            <Box marginTop={1} marginLeft={2}>
              <Text color={theme.colours.molten}>
                Still waiting — make sure you save the file (not just edit it)
              </Text>
            </Box>
          )}

          {hintLevel >= 2 && (
            <Box marginLeft={2}>
              <Text color={theme.colours.molten}>
                Make sure you&apos;re editing a file inside {workspaceRoot} that matches:{' '}
                {WATCHED_PATTERNS.join(', ')}
              </Text>
            </Box>
          )}

          {hintLevel >= 3 && (
            <Box marginLeft={2}>
              <Text color={theme.colours.ash}>
                Press <Text color={theme.colours.text}>s</Text> to skip this step
              </Text>
            </Box>
          )}
        </Box>
      )}

      {phase === 'detected' && (
        <Box flexDirection="column">
          <Box marginBottom={1}>
            <Text bold color={theme.colours.ember}>
              {theme.icons.success} Change detected!
            </Text>
          </Box>

          <Box flexDirection="column" marginLeft={2} marginBottom={1}>
            <Text color={theme.colours.text}>
              File: <Text color={theme.colours.molten}>{detectedFile}</Text>
            </Text>
            <Text color={theme.colours.ash}>
              Anvil validated your change in real time — this is the core feedback loop.
            </Text>
          </Box>

          <Box marginLeft={2} marginBottom={1}>
            <Text color={theme.colours.smoke}>
              In production, you&apos;d run <Text color={theme.colours.text}>anvil watch</Text> to
              get this continuously.
            </Text>
          </Box>

          <Box marginTop={1}>
            <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
          </Box>
        </Box>
      )}
    </Box>
  );
}
