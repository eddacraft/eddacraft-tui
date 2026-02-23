import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../../../utils/theme.js';

export interface TutorialOption {
  topic: string;
  description: string;
}

interface TutorialPickerProps {
  tutorials: TutorialOption[];
  currentTopic?: string;
}

export function TutorialPicker({
  tutorials,
  currentTopic,
}: TutorialPickerProps): React.ReactElement {
  const available = tutorials.filter((t) => t.topic !== currentTopic);

  if (available.length === 0) return <></>;

  return (
    <Box flexDirection="column">
      <Box marginBottom={1}>
        <Text color={theme.colours.text}>Continue with another tutorial:</Text>
      </Box>
      <Box flexDirection="column" marginLeft={2}>
        {available.map((t, i) => (
          <Box key={t.topic}>
            <Text color={theme.colours.ember}>{i + 1}</Text>
            <Text color={theme.colours.smoke}>{'  '}</Text>
            <Text color={theme.colours.text}>{t.topic}</Text>
            <Text color={theme.colours.smoke}>
              {' '}
              {theme.icons.arrow} {t.description}
            </Text>
          </Box>
        ))}
      </Box>
    </Box>
  );
}

/**
 * Given the full tutorials list, current topic, and a number key (1-based),
 * returns the topic string or null if out of range.
 */
export function resolveTutorialKey(
  tutorials: TutorialOption[],
  currentTopic: string | undefined,
  key: string
): string | null {
  const num = parseInt(key, 10);
  if (isNaN(num) || num < 1) return null;

  const available = tutorials.filter((t) => t.topic !== currentTopic);
  const selected = available[num - 1];
  return selected?.topic ?? null;
}
