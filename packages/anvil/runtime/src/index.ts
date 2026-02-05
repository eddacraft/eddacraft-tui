/**
 * @eddacraft/anvil-runtime
 *
 * Orchestration and I/O for the Anvil system.
 * Contains gate runner, cache providers, file watcher, export utilities,
 * and multi-agent concurrency coordination.
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
  MultiAgentConfig,
} from './watch/index.js';

// Export utilities (llms.txt, MCP, etc.)
export * from './export/index.js';

// Multi-agent concurrency coordination
export {
  // Types and schemas
  AgentTypeSchema,
  AgentInfoSchema,
  AgentRegistrationSchema,
  AgentRegistrySchema,
  LockTypeSchema,
  LockRecordSchema,
  LockFileSchema,
  QueueEntrySchema,
  QueueFileSchema,
  ConcurrencyConfigSchema,
  getDefaultConcurrencyConfig,
  // Agent management
  AgentManager,
  createAgentManager,
  initializeGlobalAgent,
  getGlobalAgent,
  detectAgentType,
  getAgentId,
  getSessionId,
  getAgentName,
  createAgentInfo,
  // Lock management
  LockManager,
  createLockManager,
  withLock,
  tryWithLock,
  // Queue management
  QueueManager,
  createQueueManager,
  coordinatedExecution,
  withConcurrencyLimit,
  // Git agent identification
  GIT_TRAILERS,
  parseCommitTrailers,
  extractAgentInfo,
  getCommitAgentInfo,
  getRecentCommitsAgentInfo,
  formatCommitWithAgent,
  prepareCommitMsgHook,
  getHookSetupCommand,
  getAgentContributions,
  getAiCommitPercentage,
  // Atomic operations
  atomicWriteJson,
  atomicWriteText,
  readJsonSafe,
  readJsonWithRetry,
  acquireFileLock,
  tryAcquireFileLock,
  isLocked,
  forceReleaseLock,
  unlinkSafe,
  fileExists,
  getFileMtime,
  sleepWithJitter,
  // High-level context
  createConcurrencyContext,
  createSimpleLockContext,
} from './concurrency/index.js';

export type {
  // Types
  AgentType,
  AgentInfo,
  AgentRegistration,
  AgentRegistry,
  LockType,
  LockRecord,
  LockFile,
  LockAcquisitionResult,
  LockReleaseResult,
  QueueEntry,
  QueueFile,
  QueueJoinResult,
  QueueStatusResult,
  CoordinationEvent,
  CoordinationEventType,
  ConcurrencyConfig,
  // Options
  AgentManagerOptions,
  LockManagerOptions,
  AcquireLockOptions,
  QueueManagerOptions,
  QueueJoinOptions,
  QueueWaitOptions,
  ConcurrencyContextOptions,
  ConcurrencyContext,
  ConcurrentGroupResult,
  // Git
  CommitAgentInfo,
  FormatCommitOptions,
  AgentContributionSummary,
  // Atomic
  AtomicWriteOptions,
  FileLockOptions,
  FileLockHandle,
} from './concurrency/index.js';
