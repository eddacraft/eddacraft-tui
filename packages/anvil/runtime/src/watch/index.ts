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
  MultiAgentConfig,
} from './types.js';

// Git status checker
export { GitStatusChecker, createGitStatusChecker, getChangedFiles } from './git-status.js';
export type { GetChangedFilesOptions } from './git-status.js';

// Debouncer
export { ChangeDebouncer, createDebouncer } from './debouncer.js';
export type { DebouncerFlushCallback } from './debouncer.js';
