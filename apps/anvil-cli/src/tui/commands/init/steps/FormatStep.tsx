import React, { useState } from 'react';
import { Box, Text, useInput } from 'ink';
import { theme } from '../../../utils/theme.js';
import { type StepProps, FORMAT_OPTIONS } from '../types.js';

export function FormatStep({ state, onNext, onBack, onCancel }: StepProps): React.ReactElement {
  const [selectedIndex, setSelectedIndex] = useState(() => {
    const idx = FORMAT_OPTIONS.findIndex((opt) => opt.value === state.format);
    return idx >= 0 ? idx : 0;
  });

  useInput((input, key) => {
    if (key.upArrow) {
      setSelectedIndex((prev) => (prev > 0 ? prev - 1 : FORMAT_OPTIONS.length - 1));
    } else if (key.downArrow) {
      setSelectedIndex((prev) => (prev < FORMAT_OPTIONS.length - 1 ? prev + 1 : 0));
    } else if (key.return) {
      const selected = FORMAT_OPTIONS[selectedIndex];
      const createExample = selected.value !== 'skip';
      onNext({ format: selected.value, createExample });
    } else if (key.escape) {
      onCancel();
    } else if (key.leftArrow) {
      onBack();
    }
  });

  return (
    <Box flexDirection="column">
      <Box marginBottom={1}>
        <Text>Select planning document format:</Text>
      </Box>

      {FORMAT_OPTIONS.map((option, index) => {
        const isSelected = index === selectedIndex;
        return (
          <Box key={option.value} flexDirection="column" marginBottom={1}>
            <Box>
              <Text color={isSelected ? theme.colours.primary : theme.colours.muted}>
                {isSelected ? theme.icons.arrow : ' '}{' '}
              </Text>
              <Text
                bold={isSelected}
                color={isSelected ? theme.colours.primary : theme.colours.text}
              >
                {option.label}
              </Text>
            </Box>
            <Box marginLeft={3}>
              <Text color={theme.colours.muted}>{option.description}</Text>
            </Box>
          </Box>
        );
      })}
    </Box>
  );
}
