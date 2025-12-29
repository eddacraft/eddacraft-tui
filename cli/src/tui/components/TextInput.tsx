import React, { useState } from 'react';
import { Box, Text } from 'ink';
import InkTextInput from 'ink-text-input';
import { theme } from '../utils/theme.js';

interface TextInputProps {
  label?: string;
  placeholder?: string;
  defaultValue?: string;
  onSubmit: (value: string) => void;
  validate?: (value: string) => string | null;
}

export function TextInput({
  label,
  placeholder = '',
  defaultValue = '',
  onSubmit,
  validate,
}: TextInputProps): React.ReactElement {
  const [value, setValue] = useState(defaultValue);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = (submittedValue: string) => {
    if (validate) {
      const validationError = validate(submittedValue);
      if (validationError) {
        setError(validationError);
        return;
      }
    }
    setError(null);
    onSubmit(submittedValue);
  };

  const handleChange = (newValue: string) => {
    setValue(newValue);
    if (error) {
      setError(null);
    }
  };

  return (
    <Box flexDirection="column">
      {label && (
        <Box marginBottom={1}>
          <Text color={theme.colours.text}>{label}</Text>
        </Box>
      )}
      <Box>
        <Text color={theme.colours.primary}>{theme.icons.arrow} </Text>
        <InkTextInput
          value={value}
          onChange={handleChange}
          onSubmit={handleSubmit}
          placeholder={placeholder}
        />
      </Box>
      {error && (
        <Box marginTop={1}>
          <Text color={theme.colours.error}>
            {theme.icons.error} {error}
          </Text>
        </Box>
      )}
    </Box>
  );
}
