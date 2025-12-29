import React, { useState } from 'react';
import { Box, Text, useInput } from 'ink';
import TextInputComponent from 'ink-text-input';
import { theme } from '../../../utils/theme.js';
import { type StepProps } from '../types.js';

export function DirectoryStep({ state, onNext, onBack, onCancel }: StepProps): React.ReactElement {
  const [value, setValue] = useState(state.planningDir);
  const [error, setError] = useState<string | null>(null);

  useInput((_input, key) => {
    if (key.escape) {
      onCancel();
    } else if (key.leftArrow && value === state.planningDir) {
      onBack();
    }
  });

  const handleSubmit = (submittedValue: string) => {
    const trimmed = submittedValue.trim();
    if (!trimmed) {
      setError('Directory path is required');
      return;
    }
    if (trimmed.startsWith('/')) {
      setError('Use relative path from project root');
      return;
    }
    setError(null);
    onNext({ planningDir: trimmed });
  };

  return (
    <Box flexDirection="column">
      <Box marginBottom={1}>
        <Text>Where should planning documents be stored?</Text>
      </Box>

      <Box>
        <Text color={theme.colours.primary}>{theme.icons.arrow} </Text>
        <TextInputComponent value={value} onChange={setValue} onSubmit={handleSubmit} />
      </Box>

      {error && (
        <Box marginTop={1}>
          <Text color={theme.colours.error}>
            {theme.icons.error} {error}
          </Text>
        </Box>
      )}

      <Box marginTop={1}>
        <Text color={theme.colours.muted}>
          Files will be created at: <Text color={theme.colours.info}>./{value}/</Text>
        </Text>
      </Box>
    </Box>
  );
}
