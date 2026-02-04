import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';

interface ArchitectureTemplate {
  name: string;
  label: string;
  layers: string[];
}

export const ARCHITECTURE_TEMPLATES: ArchitectureTemplate[] = [
  {
    name: 'starter',
    label: 'Starter',
    layers: ['components', 'lib', 'services'],
  },
  {
    name: 'layered',
    label: 'Layered',
    layers: ['presentation', 'business', 'data', 'shared'],
  },
  {
    name: 'hexagonal',
    label: 'Hexagonal',
    layers: ['core', 'ports', 'adapters', 'application'],
  },
  {
    name: 'clean',
    label: 'Clean',
    layers: ['entities', 'use_cases', 'interface_adapters', 'frameworks'],
  },
  {
    name: 'ddd',
    label: 'DDD',
    layers: ['domain', 'application', 'infrastructure', 'interfaces'],
  },
  {
    name: 'monorepo',
    label: 'Monorepo',
    layers: ['apps', 'packages', 'shared'],
  },
];

export function TemplateStep(): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Choose a Template
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          Anvil provides architecture templates to get you started quickly.
        </Text>
        <Text color={theme.colours.ash}>
          Each template defines layers and the allowed dependency direction.
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        {ARCHITECTURE_TEMPLATES.map((tmpl) => (
          <Box key={tmpl.name} marginBottom={1} flexDirection="column">
            <Text color={theme.colours.text}>
              {theme.icons.bullet} <Text bold>{tmpl.label}</Text>
              <Text color={theme.colours.smoke}> — {tmpl.layers.join(', ')}</Text>
            </Text>
          </Box>
        ))}
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          Run <Text color={theme.colours.text}>anvil architecture create</Text> to interactively
        </Text>
        <Text color={theme.colours.ash}>
          set up your choice. The command will generate an architecture.yaml
        </Text>
        <Text color={theme.colours.ash}>
          file with your selected template&apos;s boundary rules.
        </Text>
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
