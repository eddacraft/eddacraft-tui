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
  completedTopics?: string[];
}

export function TutorialPicker({
  tutorials,
  currentTopic,
  completedTopics = [],
}: TutorialPickerProps): React.ReactElement {
  const available = tutorials.filter((t) => t.topic !== currentTopic);

  if (available.length === 0) return <></>;

  let keyIndex = 0;
  const hasSelectable = available.some((t) => !completedTopics.includes(t.topic));

  return (
    <Box flexDirection="column">
      <Box marginBottom={1}>
        <Text bold color={theme.colours.text}>
          What&apos;s next
        </Text>
        <Text color={theme.colours.ash}>
          {' '}
          —{' '}
          {hasSelectable ? (
            <>
              press a number to start, <Text color={theme.colours.ember}>q</Text> to exit
            </>
          ) : (
            <>
              all tutorials completed! Press <Text color={theme.colours.ember}>q</Text> to exit
            </>
          )}
        </Text>
      </Box>
      <Box flexDirection="column" marginLeft={2}>
        {available.map((t) => {
          const isCompleted = completedTopics.includes(t.topic);
          if (!isCompleted) keyIndex++;
          return (
            <Box key={t.topic}>
              {isCompleted ? (
                <Text color={theme.colours.steel}>{theme.icons.check}</Text>
              ) : (
                <Text color={theme.colours.ember}>{keyIndex}</Text>
              )}
              <Text color={theme.colours.smoke}>{'  '}</Text>
              <Text color={isCompleted ? theme.colours.steel : theme.colours.text}>{t.topic}</Text>
              <Text color={theme.colours.smoke}>
                {' '}
                {theme.icons.arrow} {t.description}
              </Text>
            </Box>
          );
        })}
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
  key: string,
  completedTopics: string[] = []
): string | null {
  const num = parseInt(key, 10);
  if (isNaN(num) || num < 1) return null;

  const selectable = tutorials.filter(
    (t) => t.topic !== currentTopic && !completedTopics.includes(t.topic)
  );
  const selected = selectable[num - 1];
  return selected?.topic ?? null;
}
