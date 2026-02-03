import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import { getWorkspaceRoot } from '../../../../utils/file-io.js';
import { theme } from '../../../utils/theme.js';
import { Spinner } from '../../../components/Spinner.js';
import { scanProject } from './scan-project.js';
import type { ScanResults } from '../types.js';

// Re-export scanProject so consumers can import from this module
export { scanProject } from './scan-project.js';

interface ScanStepProps {
  onComplete: (results: ScanResults) => void;
  scanResults?: ScanResults;
}

type ScanPhase = 'scanning' | 'done';

export function ScanStep({ onComplete, scanResults }: ScanStepProps): React.ReactElement {
  const [phase, setPhase] = useState<ScanPhase>(scanResults ? 'done' : 'scanning');
  const [results, setResults] = useState<ScanResults | undefined>(scanResults);
  const [error, setError] = useState<string | undefined>();

  useEffect(() => {
    if (scanResults) return;

    let cancelled = false;

    async function run() {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const scanResult = await scanProject(workspaceRoot);

        if (cancelled) return;

        setResults(scanResult);
        setPhase('done');
        onComplete(scanResult);
      } catch (err) {
        if (cancelled) return;
        setError(err instanceof Error ? err.message : 'Unknown error during scan');
        setPhase('done');
      }
    }

    run();

    return () => {
      cancelled = true;
    };
  }, [onComplete, scanResults]);

  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Scan Your Project
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          Analysing your codebase for architecture violations and anti-patterns.
        </Text>
      </Box>

      {phase === 'scanning' && (
        <Box marginTop={1}>
          <Spinner />
          <Text color={theme.colours.ash}> Scanning your project...</Text>
        </Box>
      )}

      {phase === 'done' && error && (
        <Box flexDirection="column" marginTop={1}>
          <Box>
            <Text color={theme.colours.slag}>
              {theme.icons.error} Scan failed: {error}
            </Text>
          </Box>
        </Box>
      )}

      {phase === 'done' && results && (
        <Box flexDirection="column" marginTop={1}>
          {results.warningCount === 0 ? (
            <Box flexDirection="column">
              <Box>
                <Text color={theme.colours.steel}>
                  {theme.icons.check} Your project looks clean! No warnings found.
                </Text>
              </Box>
            </Box>
          ) : (
            <Box flexDirection="column">
              <Box>
                <Text color={theme.colours.molten}>
                  {theme.icons.warning} Found {results.warningCount} warning
                  {results.warningCount !== 1 ? 's' : ''} across {results.fileCount} file
                  {results.fileCount !== 1 ? 's' : ''}{' '}
                </Text>
                <Text color={theme.colours.smoke}>({results.executionTimeMs}ms)</Text>
              </Box>

              {results.topWarnings.length > 0 && (
                <Box flexDirection="column" marginTop={1} marginLeft={2}>
                  {results.topWarnings.map((warning) => (
                    <Box
                      key={`${warning.file}:${warning.line}:${warning.id}`}
                      flexDirection="column"
                      marginBottom={1}
                    >
                      <Box>
                        <Text color={theme.colours.molten}>{theme.icons.bullet} </Text>
                        <Text color={theme.colours.text} bold>
                          {warning.title}
                        </Text>
                      </Box>
                      <Box marginLeft={2}>
                        <Text color={theme.colours.smoke}>
                          {warning.file}:{warning.line}
                        </Text>
                      </Box>
                      <Box marginLeft={2}>
                        <Text color={theme.colours.ash}>
                          {theme.icons.arrow} {warning.suggestion}
                        </Text>
                      </Box>
                    </Box>
                  ))}
                </Box>
              )}
            </Box>
          )}

          <Box marginTop={2}>
            <Text color={theme.colours.molten}>
              Press Enter to start watch mode {theme.icons.arrow}
            </Text>
          </Box>
        </Box>
      )}
    </Box>
  );
}
