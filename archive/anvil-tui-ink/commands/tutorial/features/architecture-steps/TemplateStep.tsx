import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';
import { MermaidDiagram } from '../../../../components/MermaidDiagram.js';

interface ArchitectureTemplate {
  name: string;
  label: string;
  layers: string[];
  /** Mermaid definition showing dependency flow */
  mermaid: string;
}

export const ARCHITECTURE_TEMPLATES: ArchitectureTemplate[] = [
  {
    name: 'starter',
    label: 'Starter',
    layers: ['app', 'domain', 'infrastructure'],
    mermaid: `graph TD
  app --> domain --> infrastructure`,
  },
  {
    name: 'layered',
    label: 'Layered',
    layers: ['presentation', 'business', 'data', 'shared'],
    mermaid: `graph TD
  presentation --> business --> data --> shared`,
  },
  {
    name: 'hexagonal',
    label: 'Hexagonal',
    layers: ['core', 'ports', 'adapters', 'application'],
    mermaid: `graph TD
  application --> ports
  adapters --> ports
  ports --> core`,
  },
  {
    name: 'clean',
    label: 'Clean',
    layers: ['entities', 'use_cases', 'interface_adapters', 'frameworks'],
    mermaid: `graph TD
  frameworks --> interface_adapters --> use_cases --> entities`,
  },
  {
    name: 'ddd',
    label: 'DDD',
    layers: ['domain', 'application', 'infrastructure', 'interfaces'],
    mermaid: `graph TD
  interfaces --> application
  application --> domain
  infrastructure --> domain`,
  },
  {
    name: 'monorepo',
    label: 'Monorepo',
    layers: ['apps', 'packages', 'shared'],
    mermaid: `graph TD
  apps --> packages --> shared`,
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
            <Box marginLeft={4}>
              <MermaidDiagram
                definition={tmpl.mermaid}
                asciiOptions={{ paddingX: 1, paddingY: 0, boxBorderPadding: 0 }}
              />
            </Box>
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
