/**
 * @eddacraft/anvil-runtime
 *
 * Orchestration and I/O for the Anvil system.
 * Contains gate runner, cache providers, file watcher, and export utilities.
 *
 * This package handles all I/O operations that @eddacraft/anvil-core does not.
 *
 * @module @eddacraft/anvil-runtime
 */

// Gate runner and checks (includes types from @eddacraft/anvil-contracts)
export * from './gate/index.js';

// Cache providers
export * from './cache/index.js';

// File watching - export specific items to avoid WatchConfig conflict
export {
  WatchConfigSchema,
  WatchGitConfigSchema,
  parseWatchConfig,
  getDefaultWatchConfig,
  DEFAULT_WATCH_PATTERNS,
  DEFAULT_EXCLUDE_PATTERNS,
  GitStatusChecker,
  createGitStatusChecker,
  getChangedFiles,
  ChangeDebouncer,
  createDebouncer,
  FileWatcher,
  createFileWatcher,
  WatchOrchestrator,
  createWatchOrchestrator,
} from './watch/index.js';

export type {
  WatchConfig as DetailedWatchConfig,
  WatchGitConfig,
  GitFileStatus,
  WatchChangeEvent,
  DebouncedChanges,
  WatchStatusEvent,
  WatchStatusEventType,
  WatchActionResult,
  WatchOrchestratorOptions,
  GetChangedFilesOptions,
  DebouncerFlushCallback,
  FileWatcherOptions,
  FileWatcherEvents,
  ActionHandler,
} from './watch/index.js';

// Export utilities (llms.txt, MCP, etc.)
export * from './export/index.js';
