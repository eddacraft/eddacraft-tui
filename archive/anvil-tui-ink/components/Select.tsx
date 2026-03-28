import React from 'react';
import { Box, Text } from 'ink';
import InkSelectInput from 'ink-select-input';
import { theme } from '../utils/theme.js';

export interface SelectItem<V> {
  label: string;
  value: V;
}

interface SelectProps<V> {
  items: SelectItem<V>[];
  onSelect: (item: SelectItem<V>) => void;
  label?: string;
  initialIndex?: number;
}

export function Select<V>({
  items,
  onSelect,
  label,
  initialIndex = 0,
}: SelectProps<V>): React.ReactElement {
  const inkItems = items.map((item) => ({
    label: item.label,
    value: item.value,
  }));

  const handleSelect = (item: { label: string; value: V }) => {
    onSelect({ label: item.label, value: item.value });
  };

  return (
    <Box flexDirection="column">
      {label && (
        <Box marginBottom={1}>
          <Text color={theme.colours.ash}>{label}</Text>
        </Box>
      )}
      <InkSelectInput
        items={inkItems}
        onSelect={handleSelect}
        initialIndex={initialIndex}
        indicatorComponent={({ isSelected }) => (
          <Text color={isSelected ? theme.colours.ember : theme.colours.smoke}>
            {isSelected ? theme.icons.arrow : ' '}{' '}
          </Text>
        )}
        itemComponent={({ isSelected, label: itemLabel }) => (
          <Text
            bold={isSelected}
            underline={isSelected}
            color={isSelected ? theme.colours.ember : theme.colours.ash}
          >
            {itemLabel}
          </Text>
        )}
      />
    </Box>
  );
}
