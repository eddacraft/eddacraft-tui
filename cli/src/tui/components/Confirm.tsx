import React, { useState } from 'react';
import { Box, Text, useInput } from 'ink';
import { theme } from '../utils/theme.js';

interface ConfirmProps {
  message: string;
  defaultValue?: boolean;
  onConfirm: (confirmed: boolean) => void;
}

export function Confirm({
  message,
  defaultValue = true,
  onConfirm,
}: ConfirmProps): React.ReactElement {
  const [selected, setSelected] = useState(defaultValue);

  useInput((input, key) => {
    if (input === 'y' || input === 'Y') {
      onConfirm(true);
    } else if (input === 'n' || input === 'N') {
      onConfirm(false);
    } else if (key.leftArrow || key.rightArrow) {
      setSelected(!selected);
    } else if (key.return) {
      onConfirm(selected);
    }
  });

  return (
    <Box>
      <Text color={theme.colours.text}>{message} </Text>
      <Text color={selected ? theme.colours.success : theme.colours.muted} bold={selected}>
        Yes
      </Text>
      <Text color={theme.colours.muted}> / </Text>
      <Text color={!selected ? theme.colours.error : theme.colours.muted} bold={!selected}>
        No
      </Text>
      <Text color={theme.colours.muted}> (y/n)</Text>
    </Box>
  );
}
