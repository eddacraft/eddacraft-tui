export type WatchStatus = 'idle' | 'running' | 'passing' | 'failing';

export interface WatchConfig {
  patterns: string[];
  action: 'validate' | 'gate' | 'check';
  gitFilter: boolean;
  profile?: string;
}

export interface QueuedChange {
  file: string;
  timestamp: Date;
}

export interface RunHistory {
  id: string;
  timestamp: Date;
  files: string[];
  action: string;
  success: boolean;
  durationMs: number;
  message?: string;
}

export interface WatchStats {
  totalRuns: number;
  passedRuns: number;
  failedRuns: number;
  avgDurationMs: number;
  lastRunAt?: Date;
}

export interface WatchState {
  status: WatchStatus;
  config: WatchConfig;
  queue: QueuedChange[];
  history: RunHistory[];
  stats: WatchStats;
  currentRun?: {
    files: string[];
    startTime: Date;
  };
}

export type WatchPanelId = 'status' | 'queue' | 'history' | 'stats';

export const WATCH_PANELS: WatchPanelId[] = ['status', 'queue', 'history', 'stats'];

export function getNextWatchPanel(current: WatchPanelId): WatchPanelId {
  const idx = WATCH_PANELS.indexOf(current);
  return WATCH_PANELS[(idx + 1) % WATCH_PANELS.length];
}

export function getPreviousWatchPanel(current: WatchPanelId): WatchPanelId {
  const idx = WATCH_PANELS.indexOf(current);
  return WATCH_PANELS[(idx - 1 + WATCH_PANELS.length) % WATCH_PANELS.length];
}

export function createInitialWatchState(config: WatchConfig): WatchState {
  return {
    status: 'idle',
    config,
    queue: [],
    history: [],
    stats: {
      totalRuns: 0,
      passedRuns: 0,
      failedRuns: 0,
      avgDurationMs: 0,
    },
  };
}

export function calculatePassRate(stats: WatchStats): number {
  if (stats.totalRuns === 0) return 0;
  return Math.round((stats.passedRuns / stats.totalRuns) * 100);
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

export function formatRelativeTime(date: Date): string {
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSecs = Math.floor(diffMs / 1000);
  const diffMins = Math.floor(diffSecs / 60);
  const diffHours = Math.floor(diffMins / 60);

  if (diffSecs < 5) return 'just now';
  if (diffSecs < 60) return `${diffSecs}s ago`;
  if (diffMins < 60) return `${diffMins}m ago`;
  return `${diffHours}h ago`;
}

export function formatTimestamp(date: Date): string {
  const hours = date.getHours().toString().padStart(2, '0');
  const minutes = date.getMinutes().toString().padStart(2, '0');
  const seconds = date.getSeconds().toString().padStart(2, '0');
  return `${hours}:${minutes}:${seconds}`;
}
