import React, { useState } from 'react';
import { Box, Text, useInput } from 'ink';
import { theme } from '../../../utils/theme.js';
import { type StepProps } from '../types.js';

interface CheckOption {
  id: string;
  label: string;
  description: string;
  detected: boolean;
}

export function ChecksStep({
  state,
  context,
  onNext,
  onBack,
  onCancel,
}: StepProps): React.ReactElement {
  const checkOptions: CheckOption[] = [
    {
      id: 'eslint',
      label: 'ESLint',
      description: 'Code quality and style checking',
      detected: context.environment.hasEslint,
    },
    {
      id: 'test',
      label: 'Tests',
      description: 'Unit test execution',
      detected: context.environment.hasVitest || context.environment.hasJest,
    },
    {
      id: 'coverage',
      label: 'Coverage',
      description: `Test coverage threshold (${state.coverageThreshold}%)`,
      detected: context.environment.hasVitest || context.environment.hasJest,
    },
    {
      id: 'secret',
      label: 'Secret Scanning',
      description: 'Detect secrets and credentials in code',
      detected: true,
    },
  ];

  const [selectedIndex, setSelectedIndex] = useState(0);
  const [enabledChecks, setEnabledChecks] = useState<Set<string>>(
    () => new Set(state.enabledChecks)
  );

  useInput((input, key) => {
    if (key.upArrow) {
      setSelectedIndex((prev) => (prev > 0 ? prev - 1 : checkOptions.length - 1));
    } else if (key.downArrow) {
      setSelectedIndex((prev) => (prev < checkOptions.length - 1 ? prev + 1 : 0));
    } else if (input === ' ') {
      const checkId = checkOptions[selectedIndex].id;
      setEnabledChecks((prev) => {
        const next = new Set(prev);
        if (next.has(checkId)) {
          next.delete(checkId);
        } else {
          next.add(checkId);
        }
        return next;
      });
    } else if (key.return) {
      onNext({ enabledChecks: Array.from(enabledChecks) });
    } else if (key.escape) {
      onCancel();
    } else if (key.leftArrow) {
      onBack();
    }
  });

  return (
    <Box flexDirection="column">
      <Box marginBottom={1}>
        <Text>Select quality checks to enable:</Text>
      </Box>

      {checkOptions.map((option, index) => {
        const isSelected = index === selectedIndex;
        const isEnabled = enabledChecks.has(option.id);
        const checkbox = isEnabled ? '[✓]' : '[ ]';

        return (
          <Box key={option.id} flexDirection="column" marginBottom={1}>
            <Box>
              <Text color={isSelected ? theme.colours.primary : theme.colours.muted}>
                {isSelected ? theme.icons.arrow : ' '}{' '}
              </Text>
              <Text color={isEnabled ? theme.colours.success : theme.colours.muted}>
                {checkbox}{' '}
              </Text>
              <Text
                bold={isSelected}
                color={isSelected ? theme.colours.primary : theme.colours.text}
              >
                {option.label}
              </Text>
              {option.detected && <Text color={theme.colours.success}> (detected)</Text>}
            </Box>
            <Box marginLeft={7}>
              <Text color={theme.colours.muted}>{option.description}</Text>
            </Box>
          </Box>
        );
      })}

      <Box marginTop={1}>
        <Text color={theme.colours.muted}>[Space] Toggle · [Enter] Continue</Text>
      </Box>
    </Box>
  );
}
