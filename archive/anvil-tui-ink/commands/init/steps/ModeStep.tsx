import React, { useState } from 'react';
import { Box, Text, useInput } from 'ink';
import { theme } from '../../../utils/theme.js';
import { type StepProps, MODE_OPTIONS } from '../types.js';

export function ModeStep({ state, onNext, onCancel }: StepProps): React.ReactElement {
  const [selectedIndex, setSelectedIndex] = useState(() => {
    const idx = MODE_OPTIONS.findIndex((opt) => opt.value === state.configTemplate);
    return idx >= 0 ? idx : 0;
  });

  useInput((input, key) => {
    if (input === 'k' || key.upArrow) {
      setSelectedIndex((prev) => (prev > 0 ? prev - 1 : MODE_OPTIONS.length - 1));
    } else if (input === 'j' || key.downArrow) {
      setSelectedIndex((prev) => (prev < MODE_OPTIONS.length - 1 ? prev + 1 : 0));
    } else if (key.return) {
      const selected = MODE_OPTIONS[selectedIndex];
      onNext({ configTemplate: selected.value });
    } else if (key.escape || input === 'q') {
      onCancel();
    }
  });

  return (
    <Box flexDirection="column">
      <Box marginBottom={1}>
        <Text>Select configuration strictness:</Text>
      </Box>

      {MODE_OPTIONS.map((option, index) => {
        const isSelected = index === selectedIndex;
        return (
          <Box key={option.value} flexDirection="column" marginBottom={1}>
            <Box>
              <Text
                bold={isSelected}
                underline={isSelected}
                color={isSelected ? theme.colours.primary : theme.colours.muted}
              >
                {isSelected ? theme.icons.arrow : ' '}{' '}
              </Text>
              <Text
                bold={isSelected}
                underline={isSelected}
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
