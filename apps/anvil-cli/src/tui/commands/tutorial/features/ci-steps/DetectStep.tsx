import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { getWorkspaceRoot } from '../../../../../utils/file-io.js';
import { theme } from '../../../../utils/theme.js';

interface DetectedCI {
  name: string;
  label: string;
}

export const KNOWN_CI_CONFIGS: { path: string; label: string }[] = [
  { path: '.github/workflows', label: 'GitHub Actions' },
  { path: '.gitlab-ci.yml', label: 'GitLab CI' },
  { path: '.circleci', label: 'CircleCI' },
  { path: 'Jenkinsfile', label: 'Jenkins' },
  { path: '.azure-pipelines.yml', label: 'Azure DevOps' },
  { path: 'bitbucket-pipelines.yml', label: 'Bitbucket Pipelines' },
];

export function DetectStep(): React.ReactElement {
  const [detecting, setDetecting] = useState(true);
  const [detected, setDetected] = useState<DetectedCI[]>([]);

  useEffect(() => {
    try {
      const workspaceRoot = getWorkspaceRoot();
      const found: DetectedCI[] = [];

      for (const ci of KNOWN_CI_CONFIGS) {
        const fullPath = join(workspaceRoot, ci.path);
        if (existsSync(fullPath)) {
          found.push({ name: ci.path, label: ci.label });
        }
      }

      // eslint-disable-next-line @eslint-react/hooks-extra/no-direct-set-state-in-use-effect
      setDetected(found);
    } catch {
      // If detection fails, show empty result
    }

    // eslint-disable-next-line @eslint-react/hooks-extra/no-direct-set-state-in-use-effect
    setDetecting(false);
  }, []);

  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Detect CI System
        </Text>
      </Box>

      {detecting && (
        <Box marginBottom={1}>
          <Text color={theme.colours.smoke}>Scanning for CI configuration...</Text>
        </Box>
      )}

      {!detecting && detected.length > 0 && (
        <Box flexDirection="column" marginBottom={1}>
          <Text color={theme.colours.steel}>
            {theme.icons.success} Found {detected.length} CI system
            {detected.length === 1 ? '' : 's'}:
          </Text>
          <Box flexDirection="column" marginLeft={2} marginTop={1}>
            {detected.map((ci) => (
              <Text key={ci.name} color={theme.colours.ash}>
                {theme.icons.bullet} {ci.label} <Text color={theme.colours.smoke}>({ci.name})</Text>
              </Text>
            ))}
          </Box>
        </Box>
      )}

      {!detecting && detected.length === 0 && (
        <Box flexDirection="column" marginBottom={1}>
          <Text color={theme.colours.molten}>{theme.icons.info} No CI configuration detected</Text>
          <Text color={theme.colours.ash}>
            The next step will show a GitHub Actions workflow you can use.
          </Text>
        </Box>
      )}

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          Anvil checks for common CI configuration files in your project root
        </Text>
        <Text color={theme.colours.ash}>to suggest the right workflow configuration.</Text>
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
