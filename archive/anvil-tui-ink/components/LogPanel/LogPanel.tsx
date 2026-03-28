import React, { useState, useEffect, useMemo } from 'react';
import { Box, Text, useInput } from 'ink';
import { theme } from '../../utils/theme.js';
import {
  type LogEntry,
  type LogLevel,
  type LogFilter,
  DEFAULT_LOG_FILTER,
  filterEntries,
  formatTimestamp,
} from './types.js';

interface LogPanelProps {
  entries: LogEntry[];
  maxVisible?: number;
  title?: string;
  showFilter?: boolean;
  showSearch?: boolean;
  autoScroll?: boolean;
  focused?: boolean;
  onCopy?: (text: string) => void;
}

const LEVEL_COLOURS: Record<LogLevel, string> = {
  error: theme.colours.slag,
  warn: theme.colours.molten,
  info: theme.colours.steel,
  debug: theme.colours.smoke,
};

const LEVEL_ICONS: Record<LogLevel, string> = {
  error: theme.icons.error,
  warn: theme.icons.warning,
  info: theme.icons.info,
  debug: theme.icons.bullet,
};

function LogEntryRow({
  entry,
  isSelected,
  searchTerm,
}: {
  entry: LogEntry;
  isSelected: boolean;
  searchTerm: string;
}): React.ReactElement {
  const colour = LEVEL_COLOURS[entry.level];
  const icon = LEVEL_ICONS[entry.level];

  const highlightMessage = (message: string): React.ReactElement => {
    if (!searchTerm) {
      return <Text color={theme.colours.ash}>{message}</Text>;
    }

    const parts: React.ReactElement[] = [];
    const lowerMessage = message.toLowerCase();
    const lowerSearch = searchTerm.toLowerCase();
    let lastIndex = 0;
    let matchIndex = lowerMessage.indexOf(lowerSearch);
    let keyCounter = 0;

    while (matchIndex !== -1) {
      if (matchIndex > lastIndex) {
        parts.push(
          <Text key={keyCounter++} color={theme.colours.ash}>
            {message.slice(lastIndex, matchIndex)}
          </Text>
        );
      }
      parts.push(
        <Text key={keyCounter++} backgroundColor={theme.colours.ember} color={theme.colours.void}>
          {message.slice(matchIndex, matchIndex + searchTerm.length)}
        </Text>
      );
      lastIndex = matchIndex + searchTerm.length;
      matchIndex = lowerMessage.indexOf(lowerSearch, lastIndex);
    }

    if (lastIndex < message.length) {
      parts.push(
        <Text key={keyCounter} color={theme.colours.ash}>
          {message.slice(lastIndex)}
        </Text>
      );
    }

    return <>{parts}</>;
  };

  return (
    <Box>
      {isSelected && <Text color={theme.colours.ember}>{theme.icons.arrow} </Text>}
      {!isSelected && <Text> </Text>}
      <Text color={theme.colours.smoke}>{formatTimestamp(entry.timestamp)} </Text>
      <Text color={colour}>{icon} </Text>
      <Text color={colour}>{entry.level.toUpperCase().padEnd(5)} </Text>
      {entry.source && <Text color={theme.colours.smoke}>[{entry.source}] </Text>}
      {highlightMessage(entry.message)}
    </Box>
  );
}

function FilterBar({
  filter,
  onToggleLevel: _onToggleLevel,
}: {
  filter: LogFilter;
  onToggleLevel: (level: LogLevel) => void;
}): React.ReactElement {
  const levels: LogLevel[] = ['error', 'warn', 'info', 'debug'];

  return (
    <Box gap={1}>
      <Text color={theme.colours.smoke}>Filter:</Text>
      {levels.map((level) => {
        const active = filter.levels.has(level);
        const colour = active ? LEVEL_COLOURS[level] : theme.colours.charcoal;
        return (
          <Text key={level} color={colour} dimColor={!active}>
            [{level.charAt(0).toUpperCase()}]{level.slice(1)}
          </Text>
        );
      })}
    </Box>
  );
}

function SearchBar({
  value,
  matchCount,
  currentMatch,
}: {
  value: string;
  matchCount: number;
  currentMatch: number;
}): React.ReactElement {
  return (
    <Box gap={1}>
      <Text color={theme.colours.smoke}>Search:</Text>
      <Text color={theme.colours.ember}>{value || '(none)'}</Text>
      {value && matchCount > 0 && (
        <Text color={theme.colours.ash}>
          {currentMatch + 1}/{matchCount} matches
        </Text>
      )}
      {value && matchCount === 0 && <Text color={theme.colours.slag}>No matches</Text>}
    </Box>
  );
}

export function LogPanel({
  entries,
  maxVisible = 10,
  title = 'Logs',
  showFilter = true,
  showSearch = true,
  autoScroll = true,
  focused = false,
  onCopy,
}: LogPanelProps): React.ReactElement {
  const [filter, setFilter] = useState<LogFilter>(DEFAULT_LOG_FILTER);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [scrollOffset, setScrollOffset] = useState(0);
  const [searchMode, setSearchMode] = useState(false);
  const [searchInput, setSearchInput] = useState('');

  const filteredEntries = useMemo(() => filterEntries(entries, filter), [entries, filter]);

  const matchIndices = useMemo(() => {
    if (!filter.search) return [];
    const searchLower = filter.search.toLowerCase();
    return filteredEntries
      .map((entry, idx) => ({ entry, idx }))
      .filter(({ entry }) => {
        const messageMatch = entry.message.toLowerCase().includes(searchLower);
        const sourceMatch = entry.source?.toLowerCase().includes(searchLower) ?? false;
        return messageMatch || sourceMatch;
      })
      .map(({ idx }) => idx);
  }, [filteredEntries, filter.search]);

  useEffect(() => {
    if (autoScroll && filteredEntries.length > 0) {
      const newOffset = Math.max(0, filteredEntries.length - maxVisible);
      // eslint-disable-next-line @eslint-react/set-state-in-effect
      setScrollOffset(newOffset);
      // eslint-disable-next-line @eslint-react/set-state-in-effect
      setSelectedIndex(filteredEntries.length - 1);
    }
  }, [filteredEntries.length, autoScroll, maxVisible]);

  const toggleLevel = (level: LogLevel) => {
    setFilter((prev) => {
      const newLevels = new Set(prev.levels);
      if (newLevels.has(level)) {
        newLevels.delete(level);
      } else {
        newLevels.add(level);
      }
      return { ...prev, levels: newLevels };
    });
  };

  const jumpToNextMatch = () => {
    if (matchIndices.length === 0) return;
    const currentMatchIdx = matchIndices.findIndex((idx) => idx >= selectedIndex);
    const nextMatchIdx =
      currentMatchIdx === -1 || currentMatchIdx === matchIndices.length - 1
        ? 0
        : currentMatchIdx + 1;
    const targetIdx = matchIndices[nextMatchIdx];
    setSelectedIndex(targetIdx);
    if (targetIdx < scrollOffset || targetIdx >= scrollOffset + maxVisible) {
      setScrollOffset(Math.max(0, targetIdx - Math.floor(maxVisible / 2)));
    }
  };

  const jumpToPrevMatch = () => {
    if (matchIndices.length === 0) return;
    const currentMatchIdx = matchIndices.findIndex((idx) => idx >= selectedIndex);
    const prevMatchIdx = currentMatchIdx <= 0 ? matchIndices.length - 1 : currentMatchIdx - 1;
    const targetIdx = matchIndices[prevMatchIdx];
    setSelectedIndex(targetIdx);
    if (targetIdx < scrollOffset || targetIdx >= scrollOffset + maxVisible) {
      setScrollOffset(Math.max(0, targetIdx - Math.floor(maxVisible / 2)));
    }
  };

  useInput((input, key) => {
    if (!focused) return;

    if (searchMode) {
      if (key.escape) {
        setSearchMode(false);
        setSearchInput('');
        setFilter((prev) => ({ ...prev, search: '' }));
      } else if (key.return) {
        setFilter((prev) => ({ ...prev, search: searchInput }));
        setSearchMode(false);
      } else if (key.backspace || key.delete) {
        setSearchInput((prev) => prev.slice(0, -1));
      } else if (input && !key.ctrl && !key.meta) {
        setSearchInput((prev) => prev + input);
      }
      return;
    }

    if (input === '/') {
      setSearchMode(true);
      setSearchInput('');
      return;
    }

    if (input === 'e' || input === 'E') {
      toggleLevel('error');
    } else if (input === 'w' || input === 'W') {
      toggleLevel('warn');
    } else if (input === 'i' || input === 'I') {
      toggleLevel('info');
    } else if (input === 'd' || input === 'D') {
      toggleLevel('debug');
    }

    if (input === 'n') {
      jumpToNextMatch();
    } else if (input === 'N') {
      jumpToPrevMatch();
    }

    if (input === 'j' || key.downArrow) {
      setSelectedIndex((prev) => Math.min(filteredEntries.length - 1, prev + 1));
      if (selectedIndex >= scrollOffset + maxVisible - 1) {
        setScrollOffset((prev) => Math.min(filteredEntries.length - maxVisible, prev + 1));
      }
    } else if (input === 'k' || key.upArrow) {
      setSelectedIndex((prev) => Math.max(0, prev - 1));
      if (selectedIndex <= scrollOffset) {
        setScrollOffset((prev) => Math.max(0, prev - 1));
      }
    }

    if (input === 'g') {
      setSelectedIndex(0);
      setScrollOffset(0);
    } else if (input === 'G') {
      setSelectedIndex(filteredEntries.length - 1);
      setScrollOffset(Math.max(0, filteredEntries.length - maxVisible));
    }

    if (input === 'y' && onCopy) {
      const entry = filteredEntries[selectedIndex];
      if (entry) {
        onCopy(entry.message);
      }
    } else if (input === 'Y' && onCopy) {
      const visibleEntries = filteredEntries.slice(scrollOffset, scrollOffset + maxVisible);
      const text = visibleEntries.map((e) => `[${e.level}] ${e.message}`).join('\n');
      onCopy(text);
    }
  });

  const visibleEntries = filteredEntries.slice(scrollOffset, scrollOffset + maxVisible);
  const currentMatchIndex = matchIndices.findIndex((idx) => idx === selectedIndex);
  const borderColour = focused ? theme.colours.ember : theme.colours.charcoal;

  return (
    <Box
      flexDirection="column"
      borderStyle={focused ? 'double' : 'single'}
      borderColor={borderColour}
      paddingX={1}
    >
      <Box marginBottom={0}>
        <Text bold color={focused ? theme.colours.ember : theme.colours.ash}>
          {theme.icons.bullet} {title}
        </Text>
        <Text color={theme.colours.smoke}>
          {' '}
          ({filteredEntries.length}/{entries.length})
        </Text>
        {focused && <Text color={theme.colours.smoke}> (focused)</Text>}
      </Box>

      {showFilter && (
        <Box marginY={0}>
          <FilterBar filter={filter} onToggleLevel={toggleLevel} />
        </Box>
      )}

      {showSearch && (
        <Box marginY={0}>
          <SearchBar
            value={searchMode ? searchInput : filter.search}
            matchCount={matchIndices.length}
            currentMatch={currentMatchIndex >= 0 ? currentMatchIndex : 0}
          />
        </Box>
      )}

      <Box flexDirection="column" marginTop={0}>
        {visibleEntries.length === 0 ? (
          <Text color={theme.colours.smoke}>{theme.icons.info} No log entries</Text>
        ) : (
          visibleEntries.map((entry, idx) => (
            <LogEntryRow
              key={entry.id}
              entry={entry}
              isSelected={scrollOffset + idx === selectedIndex}
              searchTerm={filter.search}
            />
          ))
        )}
      </Box>

      {filteredEntries.length > maxVisible && (
        <Box marginTop={0}>
          <Text color={theme.colours.smoke}>
            {scrollOffset > 0 ? '↑ ' : '  '}
            {scrollOffset + maxVisible < filteredEntries.length ? ' ↓' : '  '}
          </Text>
        </Box>
      )}

      <Box marginTop={0}>
        <Text color={theme.colours.smoke}>
          j/k scroll {theme.icons.bullet} g/G jump {theme.icons.bullet} e/w/i/d filter{' '}
          {theme.icons.bullet} / search {theme.icons.bullet} n/N matches
        </Text>
      </Box>
    </Box>
  );
}
