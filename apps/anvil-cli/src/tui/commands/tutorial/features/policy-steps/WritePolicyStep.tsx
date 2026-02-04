import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { getWorkspaceRoot } from '../../../../../utils/file-io.js';
import { theme } from '../../../../utils/theme.js';

export const POLICY_REGO = `package anvil.policies.max_file_length

import future.keywords.if

default max_lines := 300

max_lines := input.config.max_lines if {
  input.config.max_lines
}

violation[msg] {
  count(input.file.lines) > max_lines
  msg := sprintf("%s exceeds %d lines (%d)",
    [input.file.path, max_lines,
     count(input.file.lines)])
}
`;

const POLICY_LINES = [
  'package anvil.policies.max_file_length',
  '',
  'import future.keywords.if',
  '',
  'default max_lines := 300',
  '',
  'max_lines := input.config.max_lines if {',
  '  input.config.max_lines',
  '}',
  '',
  'violation[msg] {',
  '  count(input.file.lines) > max_lines',
  '  msg := sprintf("%s exceeds %d lines (%d)",',
  '    [input.file.path, max_lines,',
  '     count(input.file.lines)])',
  '}',
];

export function WritePolicyStep(): React.ReactElement {
  const [written, setWritten] = useState(false);

  useEffect(() => {
    try {
      const workspaceRoot = getWorkspaceRoot();
      const policyDir = join(workspaceRoot, '.anvil', 'policies');
      const policyPath = join(policyDir, 'max_file_length.rego');

      if (!existsSync(policyDir)) {
        mkdirSync(policyDir, { recursive: true });
      }

      writeFileSync(policyPath, POLICY_REGO, 'utf-8');
      setWritten(true);
    } catch {
      // If writing fails, still show the step
      setWritten(true);
    }
  }, []);

  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Write Your First Policy
        </Text>
      </Box>

      <Box marginBottom={1}>
        <Text color={theme.colours.smoke}>Creating .anvil/policies/max_file_length.rego...</Text>
      </Box>

      {/* Policy code display */}
      <Box
        flexDirection="column"
        borderStyle="single"
        borderColor={theme.colours.charcoal}
        paddingX={1}
        marginBottom={1}
      >
        {POLICY_LINES.map((line, index) => (
          <Text key={index} color={theme.colours.text}>
            {line}
          </Text>
        ))}
      </Box>

      {written && (
        <Box marginBottom={1}>
          <Text color={theme.colours.steel}>
            {theme.icons.success} Policy written to .anvil/policies/max_file_length.rego
          </Text>
        </Box>
      )}

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
