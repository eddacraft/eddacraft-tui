import React, { useState, useMemo } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import { Header } from '../../components/Header.js';
import { CheckTree } from './panels/CheckTree.js';
import { DetailPanel } from './panels/DetailPanel.js';
import { FilterBar } from './panels/FilterBar.js';
import { theme } from '../../utils/theme.js';
import {
  type GateResult,
  type FilterStatus,
  type GateExplorerState,
  getFilteredChecks,
  getFailedCheckIndices,
  formatScore,
  formatDuration,
} from './types.js';

interface GateExplorerProps {
  result: GateResult;
  onQuit?: () => void;
  onExport?: (format: 'json' | 'html') => void;
}

export function GateExplorer({ result, onQuit, onExport }: GateExplorerProps): React.ReactElement {
  const { exit } = useApp();
  const [state, setState] = useState<GateExplorerState>({
    selectedIndex: 0,
    expandedChecks: new Set(),
    filterStatus: 'all',
    searchTerm: '',
  });
  const [searchMode, setSearchMode] = useState(false);
  const [searchInput, setSearchInput] = useState('');

  const filteredChecks = useMemo(
    () => getFilteredChecks(result.checks, state.filterStatus, state.searchTerm),
    [result.checks, state.filterStatus, state.searchTerm]
  );

  const failedIndices = useMemo(() => getFailedCheckIndices(filteredChecks), [filteredChecks]);

  const currentFailureIndex = useMemo(() => {
    const idx = failedIndices.findIndex((i) => i >= state.selectedIndex);
    return idx === -1 ? failedIndices.length - 1 : idx;
  }, [failedIndices, state.selectedIndex]);

  const selectedCheck = filteredChecks[state.selectedIndex] ?? null;

  const setFilter = (filter: FilterStatus) => {
    setState((prev) => ({ ...prev, filterStatus: filter, selectedIndex: 0 }));
  };

  const toggleExpand = (checkId: string) => {
    setState((prev) => {
      const newExpanded = new Set(prev.expandedChecks);
      if (newExpanded.has(checkId)) {
        newExpanded.delete(checkId);
      } else {
        newExpanded.add(checkId);
      }
      return { ...prev, expandedChecks: newExpanded };
    });
  };

  const jumpToNextFailure = () => {
    if (failedIndices.length === 0) return;
    const nextIdx = failedIndices.find((i) => i > state.selectedIndex);
    setState((prev) => ({
      ...prev,
      selectedIndex: nextIdx ?? failedIndices[0],
    }));
  };

  const jumpToPrevFailure = () => {
    if (failedIndices.length === 0) return;
    const prevIdx = [...failedIndices].reverse().find((i) => i < state.selectedIndex);
    setState((prev) => ({
      ...prev,
      selectedIndex: prevIdx ?? failedIndices[failedIndices.length - 1],
    }));
  };

  useInput((input, key) => {
    if (searchMode) {
      if (key.escape) {
        setSearchMode(false);
        setSearchInput('');
      } else if (key.return) {
        setState((prev) => ({ ...prev, searchTerm: searchInput, selectedIndex: 0 }));
        setSearchMode(false);
      } else if (key.backspace || key.delete) {
        setSearchInput((prev) => prev.slice(0, -1));
      } else if (input && !key.ctrl && !key.meta) {
        setSearchInput((prev) => prev + input);
      }
      return;
    }

    if (input === 'q' || (key.ctrl && input === 'c')) {
      onQuit?.();
      exit();
      return;
    }

    if (input === '/') {
      setSearchMode(true);
      setSearchInput('');
      return;
    }

    if (input === 'a') setFilter('all');
    if (input === 'p') setFilter('passed');
    if (input === 'f') setFilter('failed');
    if (input === 's') setFilter('skipped');

    if (input === 'n') jumpToNextFailure();
    if (input === 'N') jumpToPrevFailure();

    if (input === 'j' || key.downArrow) {
      setState((prev) => ({
        ...prev,
        selectedIndex: Math.min(filteredChecks.length - 1, prev.selectedIndex + 1),
      }));
    }
    if (input === 'k' || key.upArrow) {
      setState((prev) => ({
        ...prev,
        selectedIndex: Math.max(0, prev.selectedIndex - 1),
      }));
    }

    if (key.return && selectedCheck) {
      toggleExpand(selectedCheck.id);
    }

    if (input === 'e' && onExport) {
      onExport('json');
    }
  });

  const overallColour = result.overall ? theme.colours.steel : theme.colours.slag;
  const overallIcon = result.overall ? theme.icons.success : theme.icons.error;
  const overallLabel = result.overall ? 'PASSED' : 'FAILED';

  return (
    <Box flexDirection="column" paddingX={1}>
      <Header title="Gate Results" subtitle={result.planPath ?? result.planId} />

      <Box marginY={1} gap={2}>
        <Text bold color={overallColour}>
          {overallIcon} {overallLabel}
        </Text>
        <Text color={theme.colours.smoke}>Score: {formatScore(result.score)}</Text>
        <Text color={theme.colours.smoke}>Duration: {formatDuration(result.duration)}</Text>
        <Text color={theme.colours.smoke}>Checks: {result.checks.length}</Text>
      </Box>

      <FilterBar
        currentFilter={state.filterStatus}
        searchTerm={searchMode ? searchInput : state.searchTerm}
        failedCount={failedIndices.length}
        currentFailureIndex={currentFailureIndex}
      />

      <Box>
        <Box flexDirection="column" width="50%">
          <Box marginBottom={0}>
            <Text bold color={theme.colours.ash}>
              {theme.icons.bullet} CHECKS
            </Text>
            <Text color={theme.colours.smoke}>
              {' '}
              ({filteredChecks.length}/{result.checks.length})
            </Text>
          </Box>
          <CheckTree
            checks={filteredChecks}
            selectedIndex={state.selectedIndex}
            expandedChecks={state.expandedChecks}
            onToggleExpand={toggleExpand}
          />
        </Box>

        <Box flexDirection="column" width="50%">
          <Box marginBottom={0}>
            <Text bold color={theme.colours.ash}>
              {theme.icons.bullet} DETAILS
            </Text>
          </Box>
          <DetailPanel check={selectedCheck} />
        </Box>
      </Box>

      <Box marginTop={1}>
        <Text color={theme.colours.smoke}>
          j/k navigate {theme.icons.bullet} Enter expand {theme.icons.bullet} n/N failures{' '}
          {theme.icons.bullet} a/p/f/s filter {theme.icons.bullet} / search {theme.icons.bullet} e
          export {theme.icons.bullet} q quit
        </Text>
      </Box>
    </Box>
  );
}
