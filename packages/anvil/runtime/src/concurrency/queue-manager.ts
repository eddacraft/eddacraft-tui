/**
 * Queue Manager
 *
 * Provides fair queuing for resources when multiple agents are waiting
 * for the same lock. Supports priority-based scheduling and timeout handling.
 */

import { promises as fs } from 'node:fs';
import { join } from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import {
  QueueFileSchema,
  type QueueFile,
  type QueueEntry,
  type QueueJoinResult,
  type QueueStatusResult,
  type LockType,
  type AgentInfo,
  type ConcurrencyConfig,
  type LockAcquisitionResult,
  getDefaultConcurrencyConfig,
} from './types.js';
import {
  atomicWriteJson,
  readJsonSafe,
  unlinkSafe,
  sleepWithJitter,
  acquireFileLock,
} from './atomic.js';
import { createAgentInfo } from './agent.js';
import { LockManager } from './lock-manager.js';
import { createDebugger } from '@eddacraft/anvil-core';

const debug = createDebugger('queue');

// ============================================================================
// Queue Manager
// ============================================================================

/**
 * Options for QueueManager
 */
export interface QueueManagerOptions {
  /** Workspace root directory */
  workspaceRoot: string;

  /** Concurrency configuration */
  config?: Partial<ConcurrencyConfig>;

  /** Agent info */
  agentInfo?: AgentInfo;

  /** Lock manager (optional, created if not provided) */
  lockManager?: LockManager;
}

/**
 * Options for joining a queue
 */
export interface QueueJoinOptions {
  /** Lock type */
  type: LockType;

  /** Resource identifier */
  resource: string;

  /** Priority (lower = higher priority, default: 100) */
  priority?: number;

  /** Reason for waiting */
  reason?: string;

  /** Custom timeout (overrides config) */
  timeoutMs?: number;
}

/**
 * Options for waiting in queue
 */
export interface QueueWaitOptions extends QueueJoinOptions {
  /** Maximum time to wait in queue (ms) */
  maxWaitMs?: number;

  /** Polling interval (ms) */
  pollIntervalMs?: number;

  /** Callback when position changes */
  onPositionChange?: (position: number, total: number) => void;

  /** Callback when lock is acquired */
  onLockAcquired?: () => void;
}

/**
 * Queue Manager
 *
 * Manages fair queuing for resources across multiple agents.
 */
export class QueueManager {
  private readonly workspaceRoot: string;
  private readonly config: ConcurrencyConfig;
  private readonly agent: AgentInfo;
  private readonly queueDir: string;
  private readonly lockManager: LockManager;

  constructor(options: QueueManagerOptions) {
    this.workspaceRoot = options.workspaceRoot;
    this.config = {
      ...getDefaultConcurrencyConfig(),
      ...options.config,
    };
    this.agent = options.agentInfo ?? createAgentInfo();
    this.queueDir = join(this.workspaceRoot, this.config.queueDir);
    this.lockManager =
      options.lockManager ??
      new LockManager({
        workspaceRoot: options.workspaceRoot,
        config: options.config,
        agentInfo: this.agent,
      });
  }

  /**
   * Get the agent ID
   */
  getAgentId(): string {
    return this.agent.id;
  }

  /**
   * Join a queue for a resource
   */
  async join(options: QueueJoinOptions): Promise<QueueJoinResult> {
    const {
      type,
      resource,
      priority = 100,
      reason,
      timeoutMs = this.config.queueTimeoutMs,
    } = options;

    await this.ensureQueueDir();

    const queuePath = this.getQueuePath(type, resource);

    // Serialise access to the queue file to prevent lost updates
    const lockPath = `${queuePath}.lock`;
    const fileLock = await acquireFileLock(lockPath, {
      timeout: Math.min(timeoutMs, 30_000),
      retryInterval: 50,
    });

    if (!fileLock) {
      throw new Error(`Timed out waiting for queue lock: ${type}:${resource}`);
    }

    try {
      const queue = await this.loadQueue(queuePath, type, resource);

      // Check if already in queue
      const existingIndex = queue.entries.findIndex((e) => e.agentId === this.agent.id);

      if (existingIndex !== -1) {
        // Update existing entry
        queue.entries[existingIndex] = {
          ...queue.entries[existingIndex],
          priority,
          reason,
          timeoutAt: new Date(Date.now() + timeoutMs).toISOString(),
        };

        this.sortQueue(queue);
        queue.updatedAt = new Date().toISOString();
        await atomicWriteJson(queuePath, queue);

        const newPosition = queue.entries.findIndex((e) => e.agentId === this.agent.id) + 1;

        debug(`Queue entry updated: ${type}:${resource} position=${newPosition}`);

        return {
          entryId: queue.entries[existingIndex].id,
          position: newPosition,
          alreadyQueued: true,
        };
      }

      // Check queue size limit
      if (queue.entries.length >= this.config.maxQueueSize) {
        throw new Error(`Queue is full (max ${this.config.maxQueueSize} entries)`);
      }

      // Create new entry
      const entry: QueueEntry = {
        id: randomUUID(),
        agentId: this.agent.id,
        agentType: this.agent.type,
        lockType: type,
        resource,
        queuedAt: new Date().toISOString(),
        priority,
        timeoutAt: new Date(Date.now() + timeoutMs).toISOString(),
        reason,
      };

      queue.entries.push(entry);
      this.sortQueue(queue);
      queue.updatedAt = new Date().toISOString();

      await atomicWriteJson(queuePath, queue);

      const position = queue.entries.findIndex((e) => e.id === entry.id) + 1;

      debug(`Joined queue: ${type}:${resource} position=${position}`);

      return {
        entryId: entry.id,
        position,
        alreadyQueued: false,
      };
    } finally {
      await fileLock.release();
    }
  }

  /**
   * Leave a queue
   */
  async leave(type: LockType, resource: string): Promise<boolean> {
    const queuePath = this.getQueuePath(type, resource);
    const queue = await this.loadQueue(queuePath, type, resource);

    const index = queue.entries.findIndex((e) => e.agentId === this.agent.id);

    if (index === -1) {
      return false;
    }

    queue.entries.splice(index, 1);
    queue.updatedAt = new Date().toISOString();

    if (queue.entries.length === 0) {
      await unlinkSafe(queuePath);
    } else {
      await atomicWriteJson(queuePath, queue);
    }

    debug(`Left queue: ${type}:${resource}`);
    return true;
  }

  /**
   * Get queue status
   */
  async getStatus(type: LockType, resource: string): Promise<QueueStatusResult> {
    const queuePath = this.getQueuePath(type, resource);
    const queue = await this.loadQueue(queuePath, type, resource);

    // Clean up timed-out entries
    await this.cleanupTimedOut(queue, queuePath);

    const yourEntry = queue.entries.find((e) => e.agentId === this.agent.id);
    const yourPosition = yourEntry
      ? queue.entries.findIndex((e) => e.agentId === this.agent.id) + 1
      : undefined;

    // Get current lock holder
    const lockInfo = await this.lockManager.getLockInfo(type, resource);

    return {
      totalEntries: queue.entries.length,
      yourPosition,
      yourEntry,
      currentHolder: lockInfo
        ? {
            agentId: lockInfo.agentId,
            agentType: lockInfo.agentType,
            acquiredAt: lockInfo.acquiredAt,
            expiresAt: lockInfo.expiresAt,
          }
        : undefined,
    };
  }

  /**
   * Check if it's our turn (we're first in queue)
   */
  async isOurTurn(type: LockType, resource: string): Promise<boolean> {
    const status = await this.getStatus(type, resource);
    return status.yourPosition === 1;
  }

  /**
   * Wait in queue until lock is acquired
   *
   * This is the main coordination primitive - joins queue, waits for turn,
   * then acquires lock.
   */
  async waitForLock(options: QueueWaitOptions): Promise<LockAcquisitionResult> {
    const {
      type,
      resource,
      priority,
      reason,
      timeoutMs,
      maxWaitMs = this.config.queueTimeoutMs,
      pollIntervalMs = 500,
      onPositionChange,
      onLockAcquired,
    } = options;

    const startTime = Date.now();

    // First try to acquire lock directly (might be available)
    const directResult = await this.lockManager.acquire({
      type,
      resource,
      reason,
      wait: false,
    });

    if (directResult.acquired) {
      onLockAcquired?.();
      return directResult;
    }

    // Join queue
    const joinResult = await this.join({ type, resource, priority, reason, timeoutMs });
    let lastPosition = joinResult.position;

    debug(`Waiting in queue: ${type}:${resource} position=${lastPosition}`);

    try {
      while (Date.now() - startTime < maxWaitMs) {
        // Check our position
        const status = await this.getStatus(type, resource);

        if (status.yourPosition !== lastPosition) {
          lastPosition = status.yourPosition ?? lastPosition;
          onPositionChange?.(lastPosition, status.totalEntries);
          debug(`Queue position changed: ${type}:${resource} position=${lastPosition}`);
        }

        // If we're first, try to acquire lock
        if (status.yourPosition === 1) {
          const lockResult = await this.lockManager.acquire({
            type,
            resource,
            reason,
            wait: false,
          });

          if (lockResult.acquired) {
            // Leave queue and return
            await this.leave(type, resource);
            onLockAcquired?.();
            return lockResult;
          }

          // Lock still held, wait longer
        }

        // Wait before polling again
        await sleepWithJitter(pollIntervalMs);
      }

      // Timeout
      await this.leave(type, resource);

      return {
        acquired: false,
        error: `Queue wait timed out after ${maxWaitMs}ms`,
        queuePosition: lastPosition,
      };
    } catch (error) {
      // Cleanup on error
      await this.leave(type, resource).catch(() => {});
      throw error;
    }
  }

  /**
   * Get all queues in the workspace
   */
  async getAllQueues(): Promise<Array<{ type: LockType; resource: string; entries: number }>> {
    const result: Array<{ type: LockType; resource: string; entries: number }> = [];

    try {
      const files = await fs.readdir(this.queueDir);

      for (const file of files) {
        if (!file.endsWith('.json')) continue;

        const queuePath = join(this.queueDir, file);
        const data = await readJsonSafe(queuePath);

        if (data) {
          const parseResult = QueueFileSchema.safeParse(data);
          if (parseResult.success) {
            result.push({
              type: parseResult.data.lockType,
              resource: parseResult.data.resource,
              entries: parseResult.data.entries.length,
            });
          }
        }
      }
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'ENOENT') {
        debug('Failed to list queues:', error);
      }
    }

    return result;
  }

  /**
   * Cleanup all timed-out entries across all queues
   */
  async cleanupAllTimedOut(): Promise<number> {
    let cleaned = 0;

    try {
      const files = await fs.readdir(this.queueDir);

      for (const file of files) {
        if (!file.endsWith('.json')) continue;

        const queuePath = join(this.queueDir, file);
        const data = await readJsonSafe(queuePath);

        if (data) {
          const parseResult = QueueFileSchema.safeParse(data);
          if (parseResult.success) {
            cleaned += await this.cleanupTimedOut(parseResult.data, queuePath);
          }
        }
      }
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'ENOENT') {
        debug('Cleanup failed:', error);
      }
    }

    return cleaned;
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  /**
   * Load queue from file
   */
  private async loadQueue(queuePath: string, type: LockType, resource: string): Promise<QueueFile> {
    const data = await readJsonSafe(queuePath);

    if (data) {
      const result = QueueFileSchema.safeParse(data);
      if (result.success) {
        return result.data;
      }
      debug('Invalid queue file schema:', result.error);
    }

    return {
      version: '1.0.0',
      resource,
      lockType: type,
      updatedAt: new Date().toISOString(),
      entries: [],
    };
  }

  /**
   * Sort queue entries by priority then by queued time
   */
  private sortQueue(queue: QueueFile): void {
    queue.entries.sort((a, b) => {
      // Lower priority number = higher priority
      if (a.priority !== b.priority) {
        return a.priority - b.priority;
      }
      // Earlier queued = higher priority
      return new Date(a.queuedAt).getTime() - new Date(b.queuedAt).getTime();
    });
  }

  /**
   * Cleanup timed-out entries
   */
  private async cleanupTimedOut(queue: QueueFile, queuePath: string): Promise<number> {
    const now = Date.now();
    const before = queue.entries.length;

    queue.entries = queue.entries.filter((entry) => {
      const timeoutAt = new Date(entry.timeoutAt).getTime();
      return now < timeoutAt;
    });

    const removed = before - queue.entries.length;

    if (removed > 0) {
      if (queue.entries.length === 0) {
        await unlinkSafe(queuePath);
      } else {
        queue.updatedAt = new Date().toISOString();
        await atomicWriteJson(queuePath, queue);
      }

      debug(`Cleaned up ${removed} timed-out queue entries`);
    }

    return removed;
  }

  /**
   * Get queue file path
   */
  private getQueuePath(type: LockType, resource: string): string {
    const key = `${type}:${resource}`;
    const hash = createHash('sha256').update(key).digest('hex').slice(0, 16);
    return join(this.queueDir, `${hash}.json`);
  }

  /**
   * Ensure queue directory exists
   */
  private async ensureQueueDir(): Promise<void> {
    await fs.mkdir(this.queueDir, { recursive: true });
  }
}

/**
 * Create a queue manager
 */
export function createQueueManager(options: QueueManagerOptions): QueueManager {
  return new QueueManager(options);
}

// ============================================================================
// High-Level Coordination Primitives
// ============================================================================

/**
 * Coordinated execution with fair queuing
 *
 * This is the primary way to execute code that requires exclusive access
 * to a resource in a multi-agent environment.
 *
 * @example
 * ```typescript
 * const result = await coordinatedExecution(
 *   queueManager,
 *   { type: 'action', resource: 'gate', reason: 'Running quality gates' },
 *   async () => {
 *     return await runGates();
 *   }
 * );
 * ```
 */
export async function coordinatedExecution<T>(
  manager: QueueManager,
  options: QueueWaitOptions,
  fn: () => Promise<T>
): Promise<T> {
  const lockResult = await manager.waitForLock(options);

  if (!lockResult.acquired) {
    throw new Error(`Failed to acquire lock: ${lockResult.error}`);
  }

  try {
    return await fn();
  } finally {
    await manager['lockManager'].release(options.type, options.resource);
  }
}

/**
 * Result of a concurrent group execution
 */
export interface ConcurrentGroupResult<T> {
  /** Results from each execution */
  results: Array<{ agentId: string; result?: T; error?: Error }>;

  /** Total execution time */
  totalTimeMs: number;

  /** Number of successful executions */
  successCount: number;
}

/**
 * Execute function with coordination, allowing limited concurrency
 *
 * Useful when you want to allow some parallel execution but limit the
 * total number of concurrent agents.
 */
export async function withConcurrencyLimit<T>(
  manager: QueueManager,
  type: LockType,
  resource: string,
  maxConcurrent: number,
  fn: () => Promise<T>
): Promise<T> {
  // Use resource slots for concurrency limiting
  for (let slot = 0; slot < maxConcurrent; slot++) {
    const slotResource = `${resource}:slot-${slot}`;

    const result = await manager.waitForLock({
      type,
      resource: slotResource,
      maxWaitMs: 1000, // Quick check per slot
    });

    if (result.acquired) {
      try {
        return await fn();
      } finally {
        await manager['lockManager'].release(type, slotResource);
      }
    }
  }

  // All slots taken, wait for any slot
  return coordinatedExecution(
    manager,
    {
      type,
      resource: `${resource}:slot-0`, // Wait for first slot
      maxWaitMs: manager['config'].queueTimeoutMs,
    },
    fn
  );
}
