import React, { useState } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import {
  ANVIL_LOGO,
  ANVIL_TAGLINE,
  VALUE_PROPOSITION,
  EDDACRAFT_TEXT,
  QUICK_START_OPTIONS,
  QuickStartOption,
} from './content.js';
import { theme } from '../../utils/theme.js';

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

  const separator = theme.icons.section.repeat(40);

  return (
    <Box flexDirection="column" paddingX={2} paddingY={1}>
      <Box flexDirection="column" marginBottom={1}>
        <Text color={theme.colours.ember} bold>
          {ANVIL_LOGO}
        </Text>
        <Text color={theme.colours.smoke}>{ANVIL_TAGLINE}</Text>
      </Box>

      <Text color={theme.colours.charcoal}>{separator}</Text>

      <Box flexDirection="column" marginY={1}>
        <Text color={theme.colours.ash}>{VALUE_PROPOSITION}</Text>
      </Box>

      <Text color={theme.colours.charcoal}>{separator}</Text>

      <Box flexDirection="column" marginTop={1} marginBottom={1}>
        <Text bold color={theme.colours.ash}>
          QUICK START
        </Text>
        <Text color={theme.colours.smoke}>↑↓ or j/k navigate • Enter select • q quit</Text>
      </Box>

      <Box flexDirection="column">
        {QUICK_START_OPTIONS.map((option, index) => {
          const isSelected = index === selectedIndex;
          return (
            <Box key={option.key} marginLeft={1}>
              <Text color={isSelected ? theme.colours.ember : theme.colours.smoke}>
                {isSelected ? theme.icons.arrow : ' '}{' '}
              </Text>
              <Text color={isSelected ? theme.colours.ember : theme.colours.ash}>
                {option.label}
              </Text>
              <Text color={theme.colours.smoke}>
                {' '}
                {theme.icons.bullet} {option.description}
              </Text>
            </Box>
          );
        })}
      </Box>

      <Box marginTop={1} flexDirection="column">
        <Text color={theme.colours.smoke}>
          {theme.icons.info} This welcome screen appears once. Set ANVIL_SKIP_WELCOME=1 to disable.
        </Text>
        <Box marginTop={1}>
          <Text color={theme.colours.charcoal}>╔═╗</Text>
          <Text color={theme.colours.smoke}> ■ </Text>
          <Text color={theme.colours.charcoal}>╔═╗</Text>
          <Text color={theme.colours.smoke}> {EDDACRAFT_TEXT}</Text>
        </Box>
      </Box>
    </Box>
  );
}
