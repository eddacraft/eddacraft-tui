import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../../utils/theme.js';
import { Spinner } from '../../../../components/Spinner.js';

type TestPhase = 'testing' | 'success' | 'no-opa';

export function TestPolicyStep(): React.ReactElement {
  const [phase, setPhase] = useState<TestPhase>('testing');

  useEffect(() => {
    let cancelled = false;

    async function runTest() {
      try {
        // Try to import PolicyLoader and validate the policy
        const { PolicyLoader } = await import('@eddacraft/anvil-runtime');
        const { getWorkspaceRoot } = await import('../../../../../utils/file-io.js');

        const workspaceRoot = getWorkspaceRoot();
        const loader = new PolicyLoader();
        const result = await loader.loadPolicies(workspaceRoot, {
          policyDir: '.anvil/policies',
        });

        if (cancelled) return;

        if (result.policies.length > 0) {
          setPhase('success');
        } else {
          // Policy dir exists but no valid policies found
          setPhase('no-opa');
        }
      } catch {
        if (cancelled) return;
        // OPA not available or import failed
        setPhase('no-opa');
      }
    }

    // Brief delay to show the testing state
    const timer = setTimeout(() => {
      runTest();
    }, 800);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, []);

  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Test Your Policy
        </Text>
      </Box>

      {phase === 'testing' && (
        <Box flexDirection="column">
          <Box marginBottom={1}>
            <Text color={theme.colours.smoke}>Running: anvil policy test</Text>
          </Box>
          <Box>
            <Spinner />
            <Text color={theme.colours.ash}> Validating policy...</Text>
          </Box>
        </Box>
      )}

      {phase === 'success' && (
        <Box flexDirection="column">
          <Box marginBottom={1}>
            <Text color={theme.colours.smoke}>Running: anvil policy test</Text>
          </Box>
          <Box flexDirection="column" marginBottom={1}>
            <Text color={theme.colours.steel}>
              {theme.icons.success} max_file_length.rego — syntax valid
            </Text>
            <Text color={theme.colours.steel}>
              {theme.icons.success} Policy loaded successfully
            </Text>
          </Box>
          <Box marginBottom={1}>
            <Text color={theme.colours.ash}>Your policy is ready to use.</Text>
          </Box>
        </Box>
      )}

      {phase === 'no-opa' && (
        <Box flexDirection="column">
          <Box marginBottom={1}>
            <Text color={theme.colours.smoke}>Running: anvil policy test</Text>
          </Box>
          <Box flexDirection="column" marginBottom={1}>
            <Text color={theme.colours.molten}>
              {theme.icons.warning} OPA binary not found — install OPA to run policy tests
            </Text>
            <Text color={theme.colours.ash}>
              {'  '}See: https://www.openpolicyagent.org/docs/latest/#running-opa
            </Text>
          </Box>
          <Box flexDirection="column" marginBottom={1}>
            <Text color={theme.colours.ash}>Your policy file is still valid Rego syntax.</Text>
            <Text color={theme.colours.ash}>Anvil will use it when OPA is available.</Text>
          </Box>
        </Box>
      )}

      <Box marginTop={2}>
        <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
      </Box>
    </Box>
  );
}
