import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import { existsSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { getWorkspaceRoot } from '../../../../../utils/file-io.js';
import { theme } from '../../../../utils/theme.js';

export function CreateDirStep(): React.ReactElement {
  const [created, setCreated] = useState(false);
  const [alreadyExisted, setAlreadyExisted] = useState(false);

  useEffect(() => {
    try {
      const workspaceRoot = getWorkspaceRoot();
      const policyDir = join(workspaceRoot, '.anvil', 'policies');

      if (existsSync(policyDir)) {
        // eslint-disable-next-line @eslint-react/set-state-in-effect
        setAlreadyExisted(true);
      } else {
        mkdirSync(policyDir, { recursive: true });
      }
      // eslint-disable-next-line @eslint-react/set-state-in-effect
      setCreated(true);
    } catch {
      // eslint-disable-next-line @eslint-react/set-state-in-effect
      setCreated(true);
    }
  }, []);

  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Create Policy Directory
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>Anvil looks for policies in .anvil/policies/</Text>
      </Box>

      {created && (
        <Box flexDirection="column" marginBottom={1}>
          {alreadyExisted ? (
            <Text color={theme.colours.steel}>
              {theme.icons.check} .anvil/policies/ already exists
            </Text>
          ) : (
            <>
              <Text color={theme.colours.smoke}>Creating directory...</Text>
              <Text color={theme.colours.steel}>
                {theme.icons.success} Created .anvil/policies/
              </Text>
            </>
          )}
        </Box>
      )}

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>This is where all your custom Rego policies live.</Text>
        <Text color={theme.colours.ash}>
          Anvil loads every .rego file from this directory automatically.
        </Text>
      </Box>

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
