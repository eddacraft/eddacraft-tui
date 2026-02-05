/**
 * Multi-Agent Concurrency Module
 *
 * Provides coordination primitives for multi-agent development scenarios:
 * - Agent identification and registration
 * - File-based distributed locking
 * - Fair request queuing
 * - Git agent attribution
 * - Atomic file operations
 *
 * @module @eddacraft/anvil-runtime/concurrency
 */

// Types
export * from './types.js';

// Atomic file operations
export {
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
  type AtomicWriteOptions,
  type FileLockOptions,
  type FileLockHandle,
} from './atomic.js';

// Agent management
export {
  AgentManager,
  createAgentManager,
  initializeGlobalAgent,
  getGlobalAgent,
  detectAgentType,
  getAgentId,
  getSessionId,
  getAgentName,
  createAgentInfo,
  type AgentManagerOptions,
} from './agent.js';

// Lock management
export {
  LockManager,
  createLockManager,
  withLock,
  tryWithLock,
  type LockManagerOptions,
  type AcquireLockOptions,
} from './lock-manager.js';

// Queue management
export {
  QueueManager,
  createQueueManager,
  coordinatedExecution,
  withConcurrencyLimit,
  type QueueManagerOptions,
  type QueueJoinOptions,
  type QueueWaitOptions,
  type ConcurrentGroupResult,
} from './queue-manager.js';

// Git agent identification
export {
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
  type CommitAgentInfo,
  type FormatCommitOptions,
  type AgentContributionSummary,
} from './git-agent.js';

// ============================================================================
// High-Level Convenience Functions
// ============================================================================

import { AgentManager, createAgentManager } from './agent.js';
import { LockManager, createLockManager, type LockManagerOptions } from './lock-manager.js';
import { QueueManager, createQueueManager } from './queue-manager.js';
import type { ConcurrencyConfig, AgentInfo } from './types.js';
import { getDefaultConcurrencyConfig } from './types.js';

/**
 * Options for creating a full concurrency context
 */
export interface ConcurrencyContextOptions {
  /** Workspace root directory */
  workspaceRoot: string;

  /** Concurrency configuration */
  config?: Partial<ConcurrencyConfig>;

  /** Agent info (auto-detected if not provided) */
  agentInfo?: AgentInfo;

  /** Whether to auto-register agent */
  autoRegister?: boolean;

  /** Whether to auto-start heartbeat */
  autoHeartbeat?: boolean;
}

/**
 * Full concurrency context with all managers
 */
export interface ConcurrencyContext {
  /** Agent manager */
  agent: AgentManager;

  /** Lock manager */
  locks: LockManager;

  /** Queue manager */
  queue: QueueManager;

  /** Configuration */
  config: ConcurrencyConfig;

  /** Cleanup function */
  cleanup: () => Promise<void>;
}

/**
 * Create a full concurrency context
 *
 * This is the main entry point for multi-agent coordination.
 *
 * @example
 * ```typescript
 * const ctx = await createConcurrencyContext({
 *   workspaceRoot: process.cwd(),
 *   autoRegister: true,
 *   autoHeartbeat: true,
 * });
 *
 * try {
 *   // Wait for lock with fair queuing
 *   const result = await ctx.queue.waitForLock({
 *     type: 'action',
 *     resource: 'gate',
 *     reason: 'Running quality gates',
 *   });
 *
 *   if (result.acquired) {
 *     // Perform work
 *     await runGates();
 *
 *     // Lock is automatically released when leaving queue
 *   }
 * } finally {
 *   await ctx.cleanup();
 * }
 * ```
 */
export async function createConcurrencyContext(
  options: ConcurrencyContextOptions
): Promise<ConcurrencyContext> {
  const {
    workspaceRoot,
    config: configOverrides,
    agentInfo,
    autoRegister = true,
    autoHeartbeat = true,
  } = options;

  const config = {
    ...getDefaultConcurrencyConfig(),
    ...configOverrides,
  };

  // Create agent manager
  const agent = createAgentManager({
    workspaceRoot,
    config,
    agentInfo,
  });

  // Create lock manager with same agent
  const locks = createLockManager({
    workspaceRoot,
    config,
    agentInfo: agent.getAgent(),
  });

  // Create queue manager with same lock manager
  const queue = createQueueManager({
    workspaceRoot,
    config,
    agentInfo: agent.getAgent(),
    lockManager: locks,
  });

  // Auto-register and start heartbeat
  if (autoRegister) {
    await agent.register('initializing');
  }

  if (autoHeartbeat) {
    agent.startHeartbeat();
  }

  // Cleanup function
  const cleanup = async () => {
    agent.stopHeartbeat();
    locks.stopAllRenewals();
    await locks.releaseAll();
    if (autoRegister) {
      await agent.unregister();
    }
  };

  return {
    agent,
    locks,
    queue,
    config,
    cleanup,
  };
}

/**
 * Create a simple lock-only context (without queuing)
 *
 * Use when you just need basic locking without fair queuing.
 */
export function createSimpleLockContext(options: LockManagerOptions): {
  locks: LockManager;
  cleanup: () => Promise<void>;
} {
  const locks = createLockManager(options);

  return {
    locks,
    cleanup: async () => {
      locks.stopAllRenewals();
      await locks.releaseAll();
    },
  };
}
