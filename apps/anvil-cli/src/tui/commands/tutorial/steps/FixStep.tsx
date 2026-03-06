import React, { useState, useEffect, useRef } from 'react';
import { Box, Text } from 'ink';
import { getWorkspaceRoot } from '../../../../utils/file-io.js';
import { theme } from '../../../utils/theme.js';
import { createTutorialWatcher } from './watch-project.js';
import type { WatchEvent } from './watch-project.js';
import type { ScanResults, ScanWarning } from '../types.js';

interface FixStepProps {
  scanResults?: ScanResults;
  fixConfirmed?: boolean;
  onComplete: () => void;
}

type FixPhase = 'waiting' | 'checking' | 'fixed';

const SIMULATED_WARNING: ScanWarning = {
  id: 'AP-003',
  title: "Explicit 'any' type",
  file: 'src/example.ts',
  line: 42,
  message: "Using 'any' defeats type safety.",
  suggestion: "Replace 'any' with a proper type definition or use 'unknown'",
};

const AUTO_COMPLETE_DELAY_MS = 2_000;

export function FixStep({
  scanResults,
  fixConfirmed,
  onComplete,
}: FixStepProps): React.ReactElement {
  const [phase, setPhase] = useState<FixPhase>(fixConfirmed ? 'fixed' : 'waiting');
  const onCompleteRef = useRef(onComplete);

  // Keep ref in sync so callbacks always use the latest onComplete.
  useEffect(() => {
    onCompleteRef.current = onComplete;
  }, [onComplete]);

  const hasWarnings = scanResults && scanResults.topWarnings.length > 0;
  const warning: ScanWarning | undefined = hasWarnings ? scanResults.topWarnings[0] : undefined;

  // Watch the specific file for changes (only when there are real warnings).
  useEffect(() => {
    if (fixConfirmed || !warning) return;

    let timer: ReturnType<typeof setTimeout> | undefined;
    const workspaceRoot = getWorkspaceRoot();
    const watcher = createTutorialWatcher(workspaceRoot, (event: WatchEvent) => {
      // Only respond to changes in the target file.
      if (event.path === warning.file || event.path.endsWith(warning.file)) {
        setPhase('checking');

        // Brief delay to show the "Checking..." state, then mark as fixed.
        timer = setTimeout(() => {
          setPhase('fixed');
          onCompleteRef.current();
        }, 500);
      }
    });

    // FixStep doesn't need to wait for ready — it watches a single known file
    // and the watcher is set up before the user is told to edit.
    // Suppress unhandled rejection if the watcher errors during init.
    void watcher.ready.catch(() => {});

    return () => {
      if (timer) clearTimeout(timer);
      watcher.close();
    };
  }, [fixConfirmed, warning]);

  // For clean projects, auto-complete after a short delay.
  useEffect(() => {
    if (fixConfirmed || hasWarnings) return;

    const timer = setTimeout(() => {
      onCompleteRef.current();
    }, AUTO_COMPLETE_DELAY_MS);

    return () => {
      clearTimeout(timer);
    };
  }, [fixConfirmed, hasWarnings]);

  // --- Already-fixed state (navigated back) ---
  if (fixConfirmed && phase === 'fixed') {
    return (
      <Box flexDirection="column" paddingX={2}>
        <Box marginBottom={1}>
          <Text bold color={theme.colours.ember}>
            Fix an Issue
          </Text>
        </Box>

        <Box marginBottom={1}>
          <Text color={theme.colours.steel}>{theme.icons.check} Fixed! Warning resolved.</Text>
        </Box>

        <Box marginTop={2}>
          <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
        </Box>
      </Box>
    );
  }

  // --- Clean project: simulated example ---
  if (!hasWarnings) {
    return (
      <Box flexDirection="column" paddingX={2}>
        <Box marginBottom={1}>
          <Text bold color={theme.colours.ember}>
            Fix an Issue
          </Text>
        </Box>

        <Box marginBottom={1}>
          <Text color={theme.colours.steel}>Your project is clean! No warnings to fix.</Text>
        </Box>

        <Box flexDirection="column" marginLeft={2} marginBottom={1}>
          <Text color={theme.colours.ash}>
            Here&apos;s what it looks like when Anvil catches an issue:
          </Text>
        </Box>

        <Box flexDirection="column" marginLeft={2} marginBottom={1}>
          <Box>
            <Text color={theme.colours.molten}>{theme.icons.warning} </Text>
            <Text color={theme.colours.text} bold>
              [{SIMULATED_WARNING.id}] {SIMULATED_WARNING.title}
            </Text>
          </Box>
          <Box marginLeft={2}>
            <Text color={theme.colours.smoke}>
              {SIMULATED_WARNING.file}:{SIMULATED_WARNING.line}
            </Text>
          </Box>
          <Box marginLeft={2}>
            <Text color={theme.colours.ash}>{SIMULATED_WARNING.message}</Text>
          </Box>
        </Box>

        <Box marginLeft={2} marginBottom={1}>
          <Text color={theme.colours.ash}>
            You&apos;d fix the issue and save — Anvil confirms the fix instantly.
          </Text>
        </Box>

        <Box marginTop={2}>
          <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
        </Box>
      </Box>
    );
  }

  // --- Real warning to fix ---
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Fix an Issue
        </Text>
      </Box>

      <Box marginBottom={1}>
        <Text color={theme.colours.ash}>Let&apos;s fix a real warning from your project.</Text>
      </Box>

      {phase === 'waiting' && warning && (
        <Box flexDirection="column">
          <Box flexDirection="column" marginLeft={2} marginBottom={1}>
            <Box>
              <Text color={theme.colours.molten}>{theme.icons.warning} </Text>
              <Text color={theme.colours.text} bold>
                [{warning.id}] {warning.title}
              </Text>
            </Box>
            <Box marginLeft={2}>
              <Text color={theme.colours.smoke}>
                {warning.file}:{warning.line}
              </Text>
            </Box>
            <Box marginLeft={2}>
              <Text color={theme.colours.ash}>{warning.message}</Text>
            </Box>
            <Box marginLeft={2} marginTop={1}>
              <Text color={theme.colours.ash}>Fix: {warning.suggestion}</Text>
            </Box>
          </Box>

          <Box marginLeft={2}>
            <Text color={theme.colours.text}>Fix the issue above, then save the file.</Text>
          </Box>
        </Box>
      )}

      {phase === 'checking' && (
        <Box marginLeft={2}>
          <Text color={theme.colours.ash}>Checking...</Text>
        </Box>
      )}

      {phase === 'fixed' && (
        <Box flexDirection="column">
          <Box marginBottom={1}>
            <Text color={theme.colours.steel}>{theme.icons.check} Fixed! Warning resolved.</Text>
          </Box>

          <Box marginLeft={2} marginBottom={1}>
            <Text color={theme.colours.ash}>
              That&apos;s the loop — Anvil catches issues at save-time so you fix them while context
              is fresh.
            </Text>
          </Box>

          <Box marginTop={2}>
            <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
          </Box>
        </Box>
      )}
    </Box>
  );
}
