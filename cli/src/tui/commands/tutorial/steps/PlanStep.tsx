import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../utils/theme.js';
import { Spinner } from '../../../components/Spinner.js';

interface PlanStepProps {
  onComplete: (planPath: string) => void;
  samplePlanPath?: string;
}

const SAMPLE_PLAN_CONTENT = `# Sample Feature Plan

## Intent

Add a simple greeting endpoint to demonstrate Anvil's validation and gate features.

## Changes

1. Create \`src/greet.ts\` with greeting function
2. Add unit tests in \`src/greet.test.ts\`
3. Export from \`src/index.ts\`

## Validation

- TypeScript compiles without errors
- Tests pass with >80% coverage
- No lint warnings
`;

type CreationPhase = 'creating' | 'writing' | 'done';

export function PlanStep({ onComplete, samplePlanPath }: PlanStepProps): React.ReactElement {
  const [phase, setPhase] = useState<CreationPhase>(samplePlanPath ? 'done' : 'creating');
  const [planPath] = useState(samplePlanPath ?? '.anvil/tutorial/sample-plan.md');

  useEffect(() => {
    if (samplePlanPath) return;

    const timer1 = setTimeout(() => setPhase('writing'), 800);
    const timer2 = setTimeout(() => {
      setPhase('done');
      onComplete(planPath);
    }, 1600);

    return () => {
      clearTimeout(timer1);
      clearTimeout(timer2);
    };
  }, [onComplete, planPath, samplePlanPath]);

  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Creating a Sample Plan
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          Plans describe the intent and expected changes for a feature.
        </Text>
        <Text color={theme.colours.ash}>
          They&apos;re the input that Anvil validates and checks.
        </Text>
      </Box>

      <Box
        flexDirection="column"
        borderStyle="single"
        borderColor={theme.colours.charcoal}
        paddingX={1}
        paddingY={1}
        marginY={1}
      >
        <Text color={theme.colours.smoke}>Preview: {planPath}</Text>
        <Box marginTop={1}>
          <Text color={theme.colours.text} wrap="wrap">
            {SAMPLE_PLAN_CONTENT}
          </Text>
        </Box>
      </Box>

      <Box marginTop={1}>
        {phase === 'creating' && (
          <Box>
            <Spinner />
            <Text color={theme.colours.ash}> Creating directory...</Text>
          </Box>
        )}
        {phase === 'writing' && (
          <Box>
            <Spinner />
            <Text color={theme.colours.ash}> Writing plan file...</Text>
          </Box>
        )}
        {phase === 'done' && (
          <Box flexDirection="column">
            <Text color={theme.colours.success}>
              {theme.icons.success} Plan created at {planPath}
            </Text>
            <Box marginTop={1}>
              <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
            </Box>
          </Box>
        )}
      </Box>
    </Box>
  );
}
