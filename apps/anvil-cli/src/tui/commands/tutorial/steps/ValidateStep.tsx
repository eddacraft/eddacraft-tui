import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../utils/theme.js';
import { Spinner } from '../../../components/Spinner.js';

interface ValidationResult {
  success: boolean;
  message: string;
}

interface ValidateStepProps {
  planPath: string;
  onComplete: (result: ValidationResult) => void;
  validationResult?: ValidationResult;
}

type ValidationPhase = 'parsing' | 'checking' | 'done';

export function ValidateStep({
  planPath,
  onComplete,
  validationResult,
}: ValidateStepProps): React.ReactElement {
  const [phase, setPhase] = useState<ValidationPhase>(validationResult ? 'done' : 'parsing');
  const [result, setResult] = useState<ValidationResult | undefined>(validationResult);

  useEffect(() => {
    if (validationResult) return;

    const timer1 = setTimeout(() => setPhase('checking'), 600);
    const timer2 = setTimeout(() => {
      const newResult: ValidationResult = {
        success: true,
        message: 'Plan structure is valid and all required fields are present.',
      };
      setResult(newResult);
      setPhase('done');
      onComplete(newResult);
    }, 1200);

    return () => {
      clearTimeout(timer1);
      clearTimeout(timer2);
    };
  }, [onComplete, validationResult]);

  return (
    <Box flexDirection="column" paddingX={2}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.ember}>
          Validating Your Plan
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ash}>
          Validation ensures your plan is well-formed before running gates.
        </Text>
        <Text color={theme.colours.ash}>It checks structure, required fields, and syntax.</Text>
      </Box>

      <Box
        flexDirection="column"
        borderStyle="single"
        borderColor={theme.colours.charcoal}
        paddingX={1}
        paddingY={1}
        marginY={1}
      >
        <Text color={theme.colours.smoke}>$ anvil validate {planPath}</Text>
      </Box>

      <Box marginTop={1} flexDirection="column">
        {phase === 'parsing' && (
          <Box>
            <Spinner />
            <Text color={theme.colours.ash}> Parsing plan structure...</Text>
          </Box>
        )}
        {phase === 'checking' && (
          <Box>
            <Spinner />
            <Text color={theme.colours.ash}> Checking required fields...</Text>
          </Box>
        )}
        {phase === 'done' && result && (
          <Box flexDirection="column">
            <Box>
              <Text color={result.success ? theme.colours.success : theme.colours.error}>
                {result.success ? theme.icons.success : theme.icons.error}{' '}
                {result.success ? 'Validation passed!' : 'Validation failed'}
              </Text>
            </Box>
            <Box marginLeft={2} marginTop={1}>
              <Text color={theme.colours.text}>{result.message}</Text>
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
