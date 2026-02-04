import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';

const GITHUB_ACTIONS_YAML = [
  '# .github/workflows/anvil.yml',
  'name: Anvil Checks',
  'on:',
  '  pull_request:',
  '    branches: [main]',
  '',
  'jobs:',
  '  anvil:',
  '    runs-on: ubuntu-latest',
  '    steps:',
  '      - uses: actions/checkout@v4',
  '      - uses: actions/setup-node@v4',
  '        with:',
  '          node-version: 20',
  '      - run: npm ci',
  '      - run: npx anvil check --all --ci',
];

const GITLAB_CI_YAML = [
  'anvil:',
  '  stage: test',
  '  script:',
  '    - npm ci',
  '    - npx anvil check --all --ci',
];

export function WorkflowStep(): React.ReactElement {
  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Generate Workflow
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.text}>GitHub Actions workflow:</Text>
        <Box
          flexDirection="column"
          marginLeft={2}
          marginTop={1}
          borderStyle="single"
          borderColor={theme.colours.charcoal}
          paddingX={1}
        >
          {GITHUB_ACTIONS_YAML.map((line, index) => (
            <Text
              key={index}
              color={line.startsWith('#') ? theme.colours.smoke : theme.colours.ash}
            >
              {line}
            </Text>
          ))}
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.text}>GitLab CI equivalent:</Text>
        <Box
          flexDirection="column"
          marginLeft={2}
          marginTop={1}
          borderStyle="single"
          borderColor={theme.colours.charcoal}
          paddingX={1}
        >
          {GITLAB_CI_YAML.map((line, index) => (
            <Text key={index} color={theme.colours.ash}>
              {line}
            </Text>
          ))}
        </Box>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          The <Text color={theme.colours.text}>--ci</Text> flag produces machine-readable output and
          sets appropriate exit codes.
        </Text>
      </Box>

      <Box marginBottom={1}>
        <Text color={theme.colours.ash}>
          {theme.icons.info} Run <Text color={theme.colours.text}>anvil hooks install</Text> to add
          this to your workflow, or copy the config above.
        </Text>
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
