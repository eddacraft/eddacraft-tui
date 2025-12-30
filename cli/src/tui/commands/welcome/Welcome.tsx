import React, { useState } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import { ANVIL_LOGO, VALUE_PROPOSITION, QUICK_START_OPTIONS, QuickStartOption } from './content.js';

export interface WelcomeProps {
  onSelect: (option: QuickStartOption) => void;
  onQuit: () => void;
}

export function Welcome({ onSelect, onQuit }: WelcomeProps): React.ReactElement {
  const { exit } = useApp();
  const [selectedIndex, setSelectedIndex] = useState(0);

  useInput((input, key) => {
    if (input === 'q' || (key.ctrl && input === 'c')) {
      onQuit();
      exit();
      return;
    }

    if (key.upArrow || input === 'k') {
      setSelectedIndex((prev) => (prev > 0 ? prev - 1 : QUICK_START_OPTIONS.length - 1));
    }

    if (key.downArrow || input === 'j') {
      setSelectedIndex((prev) => (prev < QUICK_START_OPTIONS.length - 1 ? prev + 1 : 0));
    }

    if (key.return) {
      onSelect(QUICK_START_OPTIONS[selectedIndex]);
      exit();
    }
  });

  return (
    <Box flexDirection="column" paddingX={2} paddingY={1}>
      <Box flexDirection="column" marginBottom={1}>
        <Text color="cyan" bold>
          {ANVIL_LOGO}
        </Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text>{VALUE_PROPOSITION}</Text>
      </Box>

      <Box flexDirection="column" marginBottom={1}>
        <Text bold color="white">
          Quick Start
        </Text>
        <Text dimColor>Use arrows or j/k to navigate, Enter to select, q to quit</Text>
      </Box>

      <Box flexDirection="column">
        {QUICK_START_OPTIONS.map((option, index) => {
          const isSelected = index === selectedIndex;
          return (
            <Box key={option.key} marginLeft={1}>
              <Text color={isSelected ? 'cyan' : undefined}>
                {isSelected ? '> ' : '  '}
                {option.label}
              </Text>
              <Text dimColor> - {option.description}</Text>
            </Box>
          );
        })}
      </Box>

      <Box marginTop={1}>
        <Text dimColor>This welcome screen appears once. Set ANVIL_SKIP_WELCOME=1 to disable.</Text>
      </Box>
    </Box>
  );
}
