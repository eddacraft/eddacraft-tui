export { WatchDashboard, type WatchDashboardHandle } from './WatchDashboard.js';
export { StatusPanel } from './panels/StatusPanel.js';
export { QueuePanel } from './panels/QueuePanel.js';
export { HistoryPanel } from './panels/HistoryPanel.js';
export { StatsPanel } from './panels/StatsPanel.js';
export {
  type WatchState,
  type WatchStatus,
  type WatchConfig,
  type QueuedChange,
  type RunHistory,
  type WatchStats,
  type WatchPanelId,
  WATCH_PANELS,
  getNextWatchPanel,
  getPreviousWatchPanel,
  createInitialWatchState,
  calculatePassRate,
  formatDuration,
  formatRelativeTime,
  formatTimestamp,
} from './types.js';
