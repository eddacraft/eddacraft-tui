/**
 * @eddacraft/anvil-runtime
 *
 * Orchestration and I/O for the Anvil system.
 * Contains cache providers, file watcher, feature flags, and
 * multi-agent concurrency coordination.
 *
 * This package handles I/O operations that @eddacraft/anvil-core does not.
 *
 * Note: The TypeScript gate runner and constraint-export utilities
 * were archived to `anvil-archive/anvil-ts-scanner/runtime-gate/` and
 * `anvil-archive/anvil-ts-scanner/runtime-export/` under ADR-033 (2026-04-29).
 * The Rust scanner / CLI / RMCP shim are now the gate-evaluation path.
 *
 * @module @eddacraft/anvil-runtime
 */

// Gate runner + constraint-export archived under ADR-033
// → anvil-archive/anvil-ts-scanner/runtime-gate/ and runtime-export/.

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

// Export utilities (llms.txt, MCP resource formatter, prompt
// formatter) archived under ADR-033 → anvil-archive/anvil-ts-scanner/runtime-export/.

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
