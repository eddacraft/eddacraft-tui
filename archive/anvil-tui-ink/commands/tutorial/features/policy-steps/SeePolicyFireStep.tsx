import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';
import { Spinner } from '../../../../components/Spinner.js';

type CheckPhase = 'checking' | 'violations' | 'clean' | 'unavailable';

interface ViolationInfo {
  file: string;
  lineCount: number;
}

export function SeePolicyFireStep(): React.ReactElement {
  const [phase, setPhase] = useState<CheckPhase>('checking');
  const [violations, setViolations] = useState<ViolationInfo[]>([]);

  useEffect(() => {
    let cancelled = false;

    async function runCheck() {
      try {
        // Try to simulate the check by counting lines in files
        const { readdirSync, readFileSync, statSync } = await import('node:fs');
        const { join } = await import('node:path');
        const { getWorkspaceRoot } = await import('../../../../../utils/file-io.js');

        const workspaceRoot = getWorkspaceRoot();
        const found: ViolationInfo[] = [];

        // Check common source directories for files exceeding 300 lines
        const srcDirs = ['src', 'lib', 'app', 'apps', 'packages'];

        function scanDir(dir: string, depth: number) {
          if (depth > 3 || cancelled) return;

          try {
            const entries = readdirSync(dir, { withFileTypes: true });
            for (const entry of entries) {
              if (cancelled) return;
              if (found.length >= 3) return; // Cap at 3 examples

              const fullPath = join(dir, entry.name);

              if (
                entry.isDirectory() &&
                !entry.name.startsWith('.') &&
                entry.name !== 'node_modules' &&
                entry.name !== 'dist'
              ) {
                scanDir(fullPath, depth + 1);
              } else if (entry.isFile() && /\.(ts|tsx|js|jsx)$/.test(entry.name)) {
                try {
                  const stat = statSync(fullPath);
                  if (stat.size > 0 && stat.size < 1_000_000) {
                    const content = readFileSync(fullPath, 'utf-8');
                    const lineCount = content.split('\n').length;
                    if (lineCount > 300) {
                      const relativePath = fullPath.replace(workspaceRoot + '/', '');
                      found.push({ file: relativePath, lineCount });
                    }
                  }
                } catch {
                  // Skip files we cannot read
                }
              }
            }
          } catch {
            // Skip directories we cannot read
          }
        }

        for (const dir of srcDirs) {
          if (cancelled) break;
          const fullDir = join(workspaceRoot, dir);
          scanDir(fullDir, 0);
        }

        if (cancelled) return;

        if (found.length > 0) {
          setViolations(found);
          setPhase('violations');
        } else {
          setPhase('clean');
        }
      } catch {
        if (cancelled) return;
        setPhase('unavailable');
      }
    }

    // Brief delay to show checking state
    const timer = setTimeout(() => {
      runCheck();
    }, 600);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, []);

  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          See Your Policy in Action
        </Text>
      </Box>

      {phase === 'checking' && (
        <Box flexDirection="column">
          <Box marginBottom={1}>
            <Text color={theme.colours.smoke}>Running: anvil check --all</Text>
          </Box>
          <Box>
            <Spinner />
            <Text color={theme.colours.ash}> Checking files against policy...</Text>
          </Box>
        </Box>
      )}

      {phase === 'violations' && (
        <Box flexDirection="column">
          <Box marginBottom={1}>
            <Text color={theme.colours.smoke}>Running: anvil check --all</Text>
          </Box>
          <Box flexDirection="column" marginBottom={1}>
            {violations.map((v) => (
              <Text key={v.file} color={theme.colours.molten}>
                {theme.icons.warning} [POLICY] max_file_length: {v.file} exceeds 300 lines (
                {v.lineCount})
              </Text>
            ))}
          </Box>
          <Box marginBottom={1}>
            <Text color={theme.colours.ash}>
              Your policy caught {violations.length} file{violations.length !== 1 ? 's' : ''}{' '}
              exceeding 300 lines.
            </Text>
          </Box>
        </Box>
      )}

      {phase === 'clean' && (
        <Box flexDirection="column">
          <Box marginBottom={1}>
            <Text color={theme.colours.smoke}>Running: anvil check --all</Text>
          </Box>
          <Box marginBottom={1}>
            <Text color={theme.colours.steel}>
              {theme.icons.success} All files under 300 lines — try lowering the threshold!
            </Text>
          </Box>
        </Box>
      )}

      {phase === 'unavailable' && (
        <Box flexDirection="column">
          <Box marginBottom={1}>
            <Text color={theme.colours.smoke}>Running: anvil check --all</Text>
          </Box>
          <Box flexDirection="column" marginBottom={1}>
            <Text color={theme.colours.ash}>If any files exceed 300 lines, you&apos;ll see:</Text>
            <Box marginLeft={2}>
              <Text color={theme.colours.molten}>
                {theme.icons.warning} [POLICY] max_file_length: src/big-file.ts exceeds 300 lines
                (452)
              </Text>
            </Box>
          </Box>
          <Box marginBottom={1}>
            <Text color={theme.colours.ash}>If no files exceed 300 lines:</Text>
            <Box marginLeft={2}>
              <Text color={theme.colours.steel}>
                {theme.icons.success} All files under 300 lines — try lowering the threshold!
              </Text>
            </Box>
          </Box>
        </Box>
      )}

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
