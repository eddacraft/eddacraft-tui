/**
 * Watch Module
 *
 * File watching functionality for real-time validation and gating.
 */

// Types and schemas
export {
  WatchConfigSchema,
  WatchGitConfigSchema,
  parseWatchConfig,
  getDefaultWatchConfig,
  DEFAULT_WATCH_PATTERNS,
  DEFAULT_EXCLUDE_PATTERNS,
} from './types.js';

export type {
  WatchConfig,
  WatchGitConfig,
  GitFileStatus,
  WatchChangeEvent,
  DebouncedChanges,
  WatchStatusEvent,
  WatchStatusEventType,
  WatchActionResult,
  WatchOrchestratorOptions,
} from './types.js';

// Git status checker
export { GitStatusChecker, createGitStatusChecker } from './git-status.js';

// Debouncer
export { ChangeDebouncer, createDebouncer } from './debouncer.js';
export type { DebouncerFlushCallback } from './debouncer.js';

// File watcher
export { FileWatcher, createFileWatcher } from './file-watcher.js';
export type { FileWatcherOptions, FileWatcherEvents } from './file-watcher.js';

// Orchestrator
export { WatchOrchestrator, createWatchOrchestrator } from './orchestrator.js';
export type { ActionHandler } from './orchestrator.js';
