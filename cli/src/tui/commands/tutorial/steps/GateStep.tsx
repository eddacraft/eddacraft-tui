import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../utils/theme.js';
import { Spinner } from '../../../components/Spinner.js';

interface GateCheck {
  name: string;
  passed: boolean;
  message: string;
}

interface GateResult {
  success: boolean;
  checks: GateCheck[];
}

interface GateStepProps {
  planPath: string;
  onComplete: (result: GateResult) => void;
  gateResult?: GateResult;
}

const DEMO_CHECKS: GateCheck[] = [
  { name: 'lint', passed: true, message: 'No linting errors found' },
  { name: 'test', passed: true, message: 'All tests passing' },
  { name: 'coverage', passed: true, message: 'Coverage: 85% (threshold: 80%)' },
  { name: 'secrets', passed: true, message: 'No secrets detected' },
];

type GatePhase = 'loading' | 'running' | 'done';

export function GateStep({ planPath, onComplete, gateResult }: GateStepProps): React.ReactElement {
  const [phase, setPhase] = useState<GatePhase>(gateResult ? 'done' : 'loading');
  const [completedChecks, setCompletedChecks] = useState<number>(
    gateResult ? DEMO_CHECKS.length : 0
  );
  const [result, setResult] = useState<GateResult | undefined>(gateResult);

  useEffect(() => {
    if (gateResult) return;

    const timer1 = setTimeout(() => setPhase('running'), 500);

    const checkTimers = DEMO_CHECKS.map((_, idx) =>
      setTimeout(() => setCompletedChecks(idx + 1), 800 + idx * 400)
    );

    const finalTimer = setTimeout(
      () => {
        const newResult: GateResult = {
          success: true,
          checks: DEMO_CHECKS,
        };
        setResult(newResult);
        setPhase('done');
        onComplete(newResult);
      },
      800 + DEMO_CHECKS.length * 400 + 200
    );

    return () => {
      clearTimeout(timer1);
      checkTimers.forEach(clearTimeout);
      clearTimeout(finalTimer);
    };
  }, [onComplete, gateResult]);

  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Running Quality Gates
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          Quality gates verify your code meets standards before changes are applied.
        </Text>
        <Text color={theme.colours.ash}>
          They run lint, tests, coverage checks, and scan for secrets.
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
        <Text color={theme.colours.smoke}>$ anvil gate {planPath}</Text>
      </Box>

      <Box marginTop={1} flexDirection="column">
        {phase === 'loading' && (
          <Box>
            <Spinner />
            <Text color={theme.colours.ash}> Loading gate configuration...</Text>
          </Box>
        )}

        {(phase === 'running' || phase === 'done') && (
          <Box flexDirection="column">
            {DEMO_CHECKS.map((check, idx) => {
              const isComplete = idx < completedChecks;
              const isRunning = idx === completedChecks && phase === 'running';

              return (
                <Box key={check.name} marginY={0}>
                  {isRunning && (
                    <Box>
                      <Spinner />
                      <Text color={theme.colours.ash}> {check.name}...</Text>
                    </Box>
                  )}
                  {isComplete && (
                    <Box>
                      <Text color={check.passed ? theme.colours.success : theme.colours.error}>
                        {check.passed ? theme.icons.success : theme.icons.error}
                      </Text>
                      <Text color={theme.colours.text}> {check.name}</Text>
                      <Text color={theme.colours.smoke}> - {check.message}</Text>
                    </Box>
                  )}
                  {!isRunning && !isComplete && (
                    <Box>
                      <Text color={theme.colours.smoke}>
                        {theme.icons.skipped} {check.name}
                      </Text>
                    </Box>
                  )}
                </Box>
              );
            })}
          </Box>
        )}

        {phase === 'done' && result && (
          <Box flexDirection="column" marginTop={1}>
            <Box
              borderStyle="round"
              borderColor={result.success ? theme.colours.success : theme.colours.error}
              paddingX={1}
            >
              <Text color={result.success ? theme.colours.success : theme.colours.error} bold>
                {result.success ? 'All gates passed!' : 'Some gates failed'}
              </Text>
            </Box>
            <Box marginTop={2}>
              <Text color={theme.colours.molten}>Press Enter to continue {theme.icons.arrow}</Text>
            </Box>
          </Box>
        )}
      </Box>
    </Box>
  );
}
