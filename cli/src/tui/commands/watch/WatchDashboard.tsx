import React, { useState, useEffect, useCallback } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import { Header } from '../../components/Header.js';
import { StatusPanel } from './panels/StatusPanel.js';
import { QueuePanel } from './panels/QueuePanel.js';
import { HistoryPanel } from './panels/HistoryPanel.js';
import { StatsPanel } from './panels/StatsPanel.js';
import { theme } from '../../utils/theme.js';
import type { WatchState, WatchPanelId, WatchConfig, RunHistory } from './types.js';
import { createInitialWatchState, getNextWatchPanel, getPreviousWatchPanel } from './types.js';

interface WatchDashboardProps {
  config: WatchConfig;
  onRunNow?: () => void;
  onClearHistory?: () => void;
  onQuit?: () => void;
}

export interface WatchDashboardHandle {
  updateStatus: (status: WatchState['status']) => void;
  addToQueue: (file: string) => void;
  clearQueue: () => void;
  startRun: (files: string[]) => void;
  completeRun: (result: Omit<RunHistory, 'id' | 'timestamp'>) => void;
}

export function WatchDashboard({
  config,
  onRunNow,
  onClearHistory,
  onQuit,
}: WatchDashboardProps): React.ReactElement {
  const { exit } = useApp();
  const [focusedPanel, setFocusedPanel] = useState<WatchPanelId>('status');
  const [state, setState] = useState<WatchState>(() => createInitialWatchState(config));

  const _updateStatus = useCallback((status: WatchState['status']) => {
    setState((prev) => ({ ...prev, status }));
  }, []);

  const _addToQueue = useCallback((file: string) => {
    setState((prev) => ({
      ...prev,
      queue: [...prev.queue, { file, timestamp: new Date() }],
    }));
  }, []);

  const _clearQueue = useCallback(() => {
    setState((prev) => ({ ...prev, queue: [] }));
  }, []);

  const _startRun = useCallback((files: string[]) => {
    setState((prev) => ({
      ...prev,
      status: 'running',
      currentRun: { files, startTime: new Date() },
      queue: prev.queue.filter((q) => !files.includes(q.file)),
    }));
  }, []);

  const _completeRun = useCallback((result: Omit<RunHistory, 'id' | 'timestamp'>) => {
    setState((prev) => {
      const newHistory: RunHistory = {
        ...result,
        id: `run-${Date.now()}`,
        timestamp: new Date(),
      };

      const updatedHistory = [newHistory, ...prev.history].slice(0, 50);

      const newStats = {
        totalRuns: prev.stats.totalRuns + 1,
        passedRuns: prev.stats.passedRuns + (result.success ? 1 : 0),
        failedRuns: prev.stats.failedRuns + (result.success ? 0 : 1),
        avgDurationMs: Math.round(
          (prev.stats.avgDurationMs * prev.stats.totalRuns + result.durationMs) /
            (prev.stats.totalRuns + 1)
        ),
        lastRunAt: new Date(),
      };

      return {
        ...prev,
        status: result.success ? 'passing' : 'failing',
        currentRun: undefined,
        history: updatedHistory,
        stats: newStats,
      };
    });
  }, []);

  useInput((input, key) => {
    if (input === 'q' || (key.ctrl && input === 'c')) {
      onQuit?.();
      exit();
      return;
    }

    if (input === 'j' || key.downArrow) {
      setFocusedPanel(getNextWatchPanel(focusedPanel));
    } else if (input === 'k' || key.upArrow) {
      setFocusedPanel(getPreviousWatchPanel(focusedPanel));
    }

    if (input === 'r') {
      onRunNow?.();
    }

    if (input === 'c') {
      setState((prev) => ({ ...prev, history: [] }));
      onClearHistory?.();
    }
  });

  useEffect(() => {
    return () => {
      onQuit?.();
    };
  }, [onQuit]);

  return (
    <Box flexDirection="column" paddingX={1}>
      <Header title="Anvil Watch" subtitle={config.action} />

      <Box flexDirection="column" marginTop={1}>
        <StatusPanel state={state} focused={focusedPanel === 'status'} />
        <QueuePanel queue={state.queue} focused={focusedPanel === 'queue'} />
        <HistoryPanel history={state.history} focused={focusedPanel === 'history'} />
        <StatsPanel stats={state.stats} focused={focusedPanel === 'stats'} />
      </Box>

      <Box marginTop={1}>
        <Text color={theme.colours.smoke}>
          {theme.icons.info} j/k navigate {theme.icons.bullet} r run now {theme.icons.bullet} c
          clear {theme.icons.bullet} q quit
        </Text>
      </Box>
    </Box>
  );
}

export {
  type WatchState,
  type WatchConfig,
  type RunHistory,
  type WatchPanelId,
  createInitialWatchState,
};
