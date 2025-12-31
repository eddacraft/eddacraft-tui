import React from 'react';
import { describe, it, expect } from 'vitest';
import { render } from 'ink-testing-library';
import { WatchDashboard } from '../WatchDashboard.js';
import { StatusPanel } from '../panels/StatusPanel.js';
import { QueuePanel } from '../panels/QueuePanel.js';
import { HistoryPanel } from '../panels/HistoryPanel.js';
import { StatsPanel } from '../panels/StatsPanel.js';
import {
  type WatchConfig,
  type WatchState,
  type WatchStats,
  type RunHistory,
  createInitialWatchState,
  calculatePassRate,
  formatDuration,
  formatRelativeTime,
  getNextWatchPanel,
  getPreviousWatchPanel,
  WATCH_PANELS,
} from '../types.js';

const mockConfig: WatchConfig = {
  patterns: ['src/**/*.ts', 'lib/**/*.ts'],
  action: 'validate',
  gitFilter: true,
  profile: 'dev',
};

function createMockState(overrides: Partial<WatchState> = {}): WatchState {
  return {
    ...createInitialWatchState(mockConfig),
    ...overrides,
  };
}

describe('WatchDashboard', () => {
  it('renders header with title', () => {
    const { lastFrame } = render(<WatchDashboard config={mockConfig} />);

    expect(lastFrame()).toContain('ANVIL WATCH');
  });

  it('renders all panels', () => {
    const { lastFrame } = render(<WatchDashboard config={mockConfig} />);

    expect(lastFrame()).toContain('STATUS');
    expect(lastFrame()).toContain('PENDING CHANGES');
    expect(lastFrame()).toContain('HISTORY');
    expect(lastFrame()).toContain('STATISTICS');
  });

  it('shows keyboard shortcuts', () => {
    const { lastFrame } = render(<WatchDashboard config={mockConfig} />);

    expect(lastFrame()).toContain('j/k navigate');
    expect(lastFrame()).toContain('r run now');
    expect(lastFrame()).toContain('q quit');
  });

  it('shows config info in status panel', () => {
    const { lastFrame } = render(<WatchDashboard config={mockConfig} />);

    expect(lastFrame()).toContain('validate');
    expect(lastFrame()).toContain('dev');
  });
});

describe('StatusPanel', () => {
  it('shows idle status by default', () => {
    const state = createMockState({ status: 'idle' });
    const { lastFrame } = render(<StatusPanel state={state} focused={false} />);

    expect(lastFrame()).toContain('Waiting for changes');
  });

  it('shows running status with spinner', () => {
    const state = createMockState({ status: 'running' });
    const { lastFrame } = render(<StatusPanel state={state} focused={false} />);

    expect(lastFrame()).toContain('Running');
  });

  it('shows passing status', () => {
    const state = createMockState({ status: 'passing' });
    const { lastFrame } = render(<StatusPanel state={state} focused={false} />);

    expect(lastFrame()).toContain('Passing');
  });

  it('shows failing status', () => {
    const state = createMockState({ status: 'failing' });
    const { lastFrame } = render(<StatusPanel state={state} focused={false} />);

    expect(lastFrame()).toContain('Failing');
  });

  it('shows current run files', () => {
    const state = createMockState({
      status: 'running',
      currentRun: { files: ['src/test.ts'], startTime: new Date() },
    });
    const { lastFrame } = render(<StatusPanel state={state} focused={false} />);

    expect(lastFrame()).toContain('test.ts');
  });

  it('shows focused indicator when focused', () => {
    const state = createMockState();
    const { lastFrame } = render(<StatusPanel state={state} focused={true} />);

    expect(lastFrame()).toContain('(focused)');
  });
});

describe('QueuePanel', () => {
  it('shows empty state when no changes', () => {
    const { lastFrame } = render(<QueuePanel queue={[]} focused={false} />);

    expect(lastFrame()).toContain('No pending changes');
  });

  it('shows queued files', () => {
    const queue = [
      { file: 'src/module.ts', timestamp: new Date() },
      { file: 'src/utils.ts', timestamp: new Date() },
    ];
    const { lastFrame } = render(<QueuePanel queue={queue} focused={false} />);

    expect(lastFrame()).toContain('module.ts');
    expect(lastFrame()).toContain('utils.ts');
  });

  it('shows queue count', () => {
    const queue = [{ file: 'test.ts', timestamp: new Date() }];
    const { lastFrame } = render(<QueuePanel queue={queue} focused={false} />);

    expect(lastFrame()).toContain('(1)');
  });
});

describe('HistoryPanel', () => {
  it('shows empty state when no history', () => {
    const { lastFrame } = render(<HistoryPanel history={[]} focused={false} />);

    expect(lastFrame()).toContain('No runs yet');
  });

  it('shows run history', () => {
    const history: RunHistory[] = [
      {
        id: '1',
        timestamp: new Date(),
        files: ['src/test.ts'],
        action: 'validate',
        success: true,
        durationMs: 500,
      },
    ];
    const { lastFrame } = render(<HistoryPanel history={history} focused={false} />);

    expect(lastFrame()).toContain('test.ts');
    expect(lastFrame()).toContain('pass');
  });

  it('shows failed runs', () => {
    const history: RunHistory[] = [
      {
        id: '1',
        timestamp: new Date(),
        files: ['src/broken.ts'],
        action: 'validate',
        success: false,
        durationMs: 300,
      },
    ];
    const { lastFrame } = render(<HistoryPanel history={history} focused={false} />);

    expect(lastFrame()).toContain('fail');
  });

  it('shows duration', () => {
    const history: RunHistory[] = [
      {
        id: '1',
        timestamp: new Date(),
        files: ['test.ts'],
        action: 'validate',
        success: true,
        durationMs: 1500,
      },
    ];
    const { lastFrame } = render(<HistoryPanel history={history} focused={false} />);

    expect(lastFrame()).toContain('1.5s');
  });
});

describe('StatsPanel', () => {
  it('shows empty state when no stats', () => {
    const stats: WatchStats = {
      totalRuns: 0,
      passedRuns: 0,
      failedRuns: 0,
      avgDurationMs: 0,
    };
    const { lastFrame } = render(<StatsPanel stats={stats} focused={false} />);

    expect(lastFrame()).toContain('No statistics yet');
  });

  it('shows statistics', () => {
    const stats: WatchStats = {
      totalRuns: 10,
      passedRuns: 8,
      failedRuns: 2,
      avgDurationMs: 500,
      lastRunAt: new Date(),
    };
    const { lastFrame } = render(<StatsPanel stats={stats} focused={false} />);

    expect(lastFrame()).toContain('10');
    expect(lastFrame()).toContain('80%');
  });

  it('shows pass/fail counts', () => {
    const stats: WatchStats = {
      totalRuns: 5,
      passedRuns: 3,
      failedRuns: 2,
      avgDurationMs: 400,
    };
    const { lastFrame } = render(<StatsPanel stats={stats} focused={false} />);

    expect(lastFrame()).toContain('3');
    expect(lastFrame()).toContain('2');
  });
});

describe('Type utilities', () => {
  describe('calculatePassRate', () => {
    it('returns 0 for no runs', () => {
      expect(
        calculatePassRate({ totalRuns: 0, passedRuns: 0, failedRuns: 0, avgDurationMs: 0 })
      ).toBe(0);
    });

    it('calculates correct percentage', () => {
      expect(
        calculatePassRate({ totalRuns: 10, passedRuns: 8, failedRuns: 2, avgDurationMs: 0 })
      ).toBe(80);
    });

    it('returns 100 for all passed', () => {
      expect(
        calculatePassRate({ totalRuns: 5, passedRuns: 5, failedRuns: 0, avgDurationMs: 0 })
      ).toBe(100);
    });
  });

  describe('formatDuration', () => {
    it('formats milliseconds', () => {
      expect(formatDuration(500)).toBe('500ms');
    });

    it('formats seconds', () => {
      expect(formatDuration(2500)).toBe('2.5s');
    });
  });

  describe('formatRelativeTime', () => {
    it('formats just now', () => {
      const now = new Date();
      expect(formatRelativeTime(now)).toBe('just now');
    });

    it('formats seconds ago', () => {
      const date = new Date(Date.now() - 30000);
      expect(formatRelativeTime(date)).toContain('s ago');
    });

    it('formats minutes ago', () => {
      const date = new Date(Date.now() - 5 * 60 * 1000);
      expect(formatRelativeTime(date)).toContain('m ago');
    });
  });

  describe('Panel navigation', () => {
    it('has correct panel order', () => {
      expect(WATCH_PANELS).toEqual(['status', 'queue', 'history', 'stats']);
    });

    it('getNextWatchPanel wraps around', () => {
      expect(getNextWatchPanel('status')).toBe('queue');
      expect(getNextWatchPanel('stats')).toBe('status');
    });

    it('getPreviousWatchPanel wraps around', () => {
      expect(getPreviousWatchPanel('status')).toBe('stats');
      expect(getPreviousWatchPanel('queue')).toBe('status');
    });
  });
});
