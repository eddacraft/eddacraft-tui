import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { getWorkspaceRoot } from '../../../../../utils/file-io.js';
import { theme } from '../../../../utils/theme.js';

interface DetectedDir {
  name: string;
  label: string;
}

const KNOWN_DIRS: { path: string; label: string }[] = [
  { path: 'src/api', label: 'API layer' },
  { path: 'src/services', label: 'Services layer' },
  { path: 'src/domain', label: 'Domain layer' },
  { path: 'src/repositories', label: 'Repositories layer' },
  { path: 'src/controllers', label: 'Controllers layer' },
  { path: 'src/components', label: 'Components layer' },
  { path: 'src/lib', label: 'Library utilities' },
  { path: 'src/utils', label: 'Utilities' },
  { path: 'src/core', label: 'Core layer' },
  { path: 'src/ports', label: 'Ports layer' },
  { path: 'src/adapters', label: 'Adapters layer' },
  { path: 'src/entities', label: 'Entities layer' },
  { path: 'src/use_cases', label: 'Use cases layer' },
  { path: 'src/infrastructure', label: 'Infrastructure layer' },
  { path: 'packages', label: 'Packages (monorepo)' },
  { path: 'apps', label: 'Applications (monorepo)' },
];

export function DetectStep(): React.ReactElement {
  const [detecting, setDetecting] = useState(true);
  const [detected, setDetected] = useState<DetectedDir[]>([]);

  useEffect(() => {
    try {
      const workspaceRoot = getWorkspaceRoot();
      const found: DetectedDir[] = [];

      for (const dir of KNOWN_DIRS) {
        const fullPath = join(workspaceRoot, dir.path);
        if (existsSync(fullPath)) {
          found.push({ name: dir.path, label: dir.label });
        }
      }

      // eslint-disable-next-line @eslint-react/set-state-in-effect
      setDetected(found);
    } catch {
      // If detection fails, show empty result
    }

    // eslint-disable-next-line @eslint-react/set-state-in-effect
    setDetecting(false);
  }, []);

  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Detect Project Structure
        </Text>
      </Box>

      {detecting && (
        <Box marginBottom={1}>
          <Text color={theme.colours.smoke}>Detecting your project structure...</Text>
        </Box>
      )}

      {!detecting && detected.length > 0 && (
        <Box flexDirection="column" marginBottom={1}>
          <Text color={theme.colours.steel}>
            {theme.icons.success} Found {detected.length} recognised director
            {detected.length === 1 ? 'y' : 'ies'}:
          </Text>
          <Box flexDirection="column" marginLeft={2} marginTop={1}>
            {detected.map((dir) => (
              <Text key={dir.name} color={theme.colours.ash}>
                {theme.icons.bullet} {dir.name}{' '}
                <Text color={theme.colours.smoke}>({dir.label})</Text>
              </Text>
            ))}
          </Box>
        </Box>
      )}

      {!detecting && detected.length === 0 && (
        <Box flexDirection="column" marginBottom={1}>
          <Text color={theme.colours.molten}>
            {theme.icons.info} No standard structure detected
          </Text>
          <Text color={theme.colours.ash}>You&apos;ll choose a template in the next step.</Text>
        </Box>
      )}

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          Anvil uses your directory layout to suggest an architecture template
        </Text>
        <Text color={theme.colours.ash}>that matches how your project is already organised.</Text>
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
