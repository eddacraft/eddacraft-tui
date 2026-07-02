/**
 * Lock Manager
 *
 * Provides distributed locking for multi-agent coordination using file-based locks.
 * Supports automatic stale lock detection and recovery.
 */

import { promises as fs } from 'node:fs';
import { join } from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import {
  LockFileSchema,
  type LockFile,
  type LockRecord,
  type LockType,
  type LockAcquisitionResult,
  type LockReleaseResult,
  type AgentInfo,
  type ConcurrencyConfig,
  getDefaultConcurrencyConfig,
} from './types.js';
import {
  atomicWriteJson,
  readJsonSafe,
  unlinkSafe,
  sleepWithJitter,
  tryAcquireFileLock,
  type FileLockHandle,
} from './atomic.js';
import { createAgentInfo } from './agent.js';
import { createDebugger } from '@eddacraft/anvil-core';

const debug = createDebugger('lock');

/**
 * Age after which a `.creating` sentinel is considered abandoned by a crashed
 * writer and may be reaped (CIB-117). A live writer holds the sentinel only
 * for the duration of a couple of small file operations.
 */
const SENTINEL_STALE_MS = 30_000;

// ============================================================================
// Lock Manager
// ============================================================================

/**
 * Options for LockManager
 */
export interface LockManagerOptions {
  /** Workspace root directory */
  workspaceRoot: string;

  /** Concurrency configuration */
  config?: Partial<ConcurrencyConfig>;

  /** Agent info for this lock manager */
  agentInfo?: AgentInfo;
}

/**
 * Options for lock acquisition
 */
export interface AcquireLockOptions {
  /** Lock type */
  type: LockType;

  /** Resource being locked */
  resource: string;

  /** Reason for lock */
  reason?: string;

  /** Custom timeout (overrides config) */
  timeoutMs?: number;

  /** Wait for lock (block until acquired or timeout) */
  wait?: boolean;

  /** Wait timeout in ms (default: 30s) */
  waitTimeoutMs?: number;

  /** Retry interval in ms (default: 100ms) */
  retryIntervalMs?: number;

  /** Allow acquiring from stale agents */
  acquireFromStale?: boolean;
}

/**
 * Lock Manager
 *
 * Manages distributed file-based locks for multi-agent coordination.
 */
export class LockManager {
  private readonly workspaceRoot: string;
  private readonly config: ConcurrencyConfig;
  private readonly agent: AgentInfo;
  private readonly lockDir: string;

  // Track locks held by this manager instance
  private readonly heldLocks: Map<string, LockRecord> = new Map();

  // Renewal timers
  private readonly renewalTimers: Map<string, NodeJS.Timeout> = new Map();

  constructor(options: LockManagerOptions) {
    this.workspaceRoot = options.workspaceRoot;
    this.config = {
      ...getDefaultConcurrencyConfig(),
      ...options.config,
    };
    this.agent = options.agentInfo ?? createAgentInfo();
    this.lockDir = join(this.workspaceRoot, this.config.lockDir);
  }

  /**
   * Get the agent ID
   */
  getAgentId(): string {
    return this.agent.id;
  }

  /**
   * Acquire a lock
   */
  async acquire(options: AcquireLockOptions): Promise<LockAcquisitionResult> {
    const {
      type,
      resource,
      reason,
      timeoutMs = this.config.lockTimeoutMs,
      wait = false,
      waitTimeoutMs = 30000,
      retryIntervalMs = 100,
      acquireFromStale = this.config.autoAcquireFromStale,
    } = options;

    const lockKey = this.getLockKey(type, resource);
    const lockPath = this.getLockPath(lockKey);

    // If waiting, use retry loop
    if (wait) {
      return this.acquireWithWait(options, waitTimeoutMs, retryIntervalMs);
    }

    await this.ensureLockDir();

    // Check for existing lock
    const existingLock = await this.readLock(lockPath);

    if (existingLock) {
      // Check if it's our lock
      if (existingLock.lock.agentId === this.agent.id) {
        // Renew our existing lock
        return this.renewLock(lockPath, existingLock, reason);
      }

      // Check if lock is expired
      const now = Date.now();
      const expiresAt = new Date(existingLock.lock.expiresAt).getTime();

      if (now > expiresAt) {
        debug(`Lock expired, taking over: ${lockKey}`);
        return this.forceAcquire(lockPath, type, resource, reason, timeoutMs, acquireFromStale);
      }

      // Check if holder is stale
      if (acquireFromStale && (await this.isHolderStale(existingLock.lock))) {
        debug(`Lock holder is stale, taking over: ${lockKey}`);
        return this.forceAcquire(lockPath, type, resource, reason, timeoutMs, acquireFromStale);
      }

      // Lock is held by another active agent
      return {
        acquired: false,
        error: `Lock held by ${existingLock.lock.agentId}`,
        heldBy: this.heldByInfo(existingLock),
      };
    }

    // No existing lock, acquire it
    return this.createLock(lockPath, type, resource, reason, timeoutMs);
  }

  /**
   * Acquire lock with wait/retry
   */
  private async acquireWithWait(
    options: AcquireLockOptions,
    waitTimeoutMs: number,
    retryIntervalMs: number
  ): Promise<LockAcquisitionResult> {
    const startTime = Date.now();

    while (Date.now() - startTime < waitTimeoutMs) {
      const result = await this.acquire({ ...options, wait: false });

      if (result.acquired) {
        return result;
      }

      // Wait with jitter before retrying
      await sleepWithJitter(retryIntervalMs);
    }

    // Timeout
    const lockKey = this.getLockKey(options.type, options.resource);
    const existingLock = await this.readLock(this.getLockPath(lockKey));

    return {
      acquired: false,
      error: `Lock acquisition timed out after ${waitTimeoutMs}ms`,
      heldBy: existingLock ? this.heldByInfo(existingLock) : undefined,
    };
  }

  /**
   * Release a lock
   */
  async release(type: LockType, resource: string): Promise<LockReleaseResult> {
    const lockKey = this.getLockKey(type, resource);
    const lockPath = this.getLockPath(lockKey);

    // Stop renewal timer
    this.stopRenewal(lockKey);

    const existingLock = await this.readLock(lockPath);

    if (!existingLock) {
      this.heldLocks.delete(lockKey);
      return { released: true };
    }

    // Check if we own the lock
    if (existingLock.lock.agentId !== this.agent.id) {
      return {
        released: false,
        error: `Lock held by different agent: ${existingLock.lock.agentId}`,
        wasHeldByOther: true,
      };
    }

    // Delete the lock file
    await unlinkSafe(lockPath);
    this.heldLocks.delete(lockKey);

    debug(`Lock released: ${lockKey}`);
    return { released: true };
  }

  /**
   * Release all locks held by this manager
   */
  async releaseAll(): Promise<void> {
    for (const [, lock] of this.heldLocks) {
      await this.release(lock.type, lock.resource);
    }
  }

  /**
   * Check if a lock is held
   */
  async isLocked(type: LockType, resource: string): Promise<boolean> {
    const lockKey = this.getLockKey(type, resource);
    const lockPath = this.getLockPath(lockKey);

    const existingLock = await this.readLock(lockPath);

    if (!existingLock) {
      return false;
    }

    // Check expiration
    const now = Date.now();
    const expiresAt = new Date(existingLock.lock.expiresAt).getTime();

    return now <= expiresAt;
  }

  /**
   * Get lock info
   */
  async getLockInfo(type: LockType, resource: string): Promise<LockRecord | null> {
    const lockKey = this.getLockKey(type, resource);
    const lockPath = this.getLockPath(lockKey);

    const existingLock = await this.readLock(lockPath);
    return existingLock?.lock ?? null;
  }

  /**
   * Check if we hold a lock
   */
  holdsLock(type: LockType, resource: string): boolean {
    const lockKey = this.getLockKey(type, resource);
    return this.heldLocks.has(lockKey);
  }

  /**
   * Get all locks held by this manager
   */
  getHeldLocks(): LockRecord[] {
    return Array.from(this.heldLocks.values());
  }

  /**
   * Start auto-renewal for a lock
   */
  startAutoRenewal(type: LockType, resource: string, intervalMs?: number): void {
    const lockKey = this.getLockKey(type, resource);
    const interval = intervalMs ?? Math.floor(this.config.lockTimeoutMs / 3);

    if (this.renewalTimers.has(lockKey)) {
      return;
    }

    const timer = setInterval(async () => {
      try {
        const lockPath = this.getLockPath(lockKey);
        const existingLock = await this.readLock(lockPath);

        if (existingLock && existingLock.lock.agentId === this.agent.id) {
          await this.renewLock(lockPath, existingLock);
          debug(`Lock auto-renewed: ${lockKey}`);
        } else {
          // We lost the lock
          this.stopRenewal(lockKey);
          this.heldLocks.delete(lockKey);
          debug(`Lost lock, stopping renewal: ${lockKey}`);
        }
      } catch (error) {
        debug(`Auto-renewal failed for ${lockKey}:`, error);
      }
    }, interval);

    // Don't block process exit
    timer.unref();

    this.renewalTimers.set(lockKey, timer);
    debug(`Auto-renewal started for ${lockKey} (interval=${interval}ms)`);
  }

  /**
   * Stop auto-renewal for a lock
   */
  stopRenewal(lockKey: string): void {
    const timer = this.renewalTimers.get(lockKey);
    if (timer) {
      clearInterval(timer);
      this.renewalTimers.delete(lockKey);
      debug(`Auto-renewal stopped for ${lockKey}`);
    }
  }

  /**
   * Stop all auto-renewals
   */
  stopAllRenewals(): void {
    for (const [lockKey, timer] of this.renewalTimers) {
      clearInterval(timer);
      debug(`Auto-renewal stopped for ${lockKey}`);
    }
    this.renewalTimers.clear();
  }

  /**
   * Cleanup expired locks in the lock directory
   */
  async cleanupExpiredLocks(): Promise<number> {
    let cleaned = 0;

    try {
      const files = await fs.readdir(this.lockDir);

      for (const file of files) {
        if (!file.endsWith('.lock')) continue;

        const lockPath = join(this.lockDir, file);
        const lock = await this.readLock(lockPath);

        if (!lock) continue;

        const now = Date.now();
        const expiresAt = new Date(lock.lock.expiresAt).getTime();

        if (now > expiresAt) {
          await unlinkSafe(lockPath);
          cleaned++;
          debug(`Cleaned up expired lock: ${file}`);
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
   * Create a new lock
   */
  private async createLock(
    lockPath: string,
    type: LockType,
    resource: string,
    reason: string | undefined,
    timeoutMs: number
  ): Promise<LockAcquisitionResult> {
    // Use O_EXCL sentinel to prevent two agents both creating the lock
    const sentinel = await this.acquireCreationSentinel(lockPath);

    if (!sentinel) {
      // Another agent is creating the lock right now — read what they wrote
      const existingLock = await this.readLock(lockPath);
      if (existingLock) {
        return {
          acquired: false,
          error: `Lock acquired by another agent: ${existingLock.lock.agentId}`,
          heldBy: this.heldByInfo(existingLock),
        };
      }
      return {
        acquired: false,
        error: 'Lock creation in progress by another agent',
      };
    }

    try {
      // Re-check after acquiring sentinel (another agent may have finished first)
      const raceCheck = await this.readLock(lockPath);
      if (raceCheck && raceCheck.lock.agentId !== this.agent.id) {
        return {
          acquired: false,
          error: `Lock acquired by another agent: ${raceCheck.lock.agentId}`,
          heldBy: this.heldByInfo(raceCheck),
        };
      }

      const now = new Date();
      const expiresAt = new Date(now.getTime() + timeoutMs);

      const lock: LockRecord = {
        type,
        resource,
        agentId: this.agent.id,
        agentType: this.agent.type,
        pid: this.agent.pid,
        acquiredAt: now.toISOString(),
        expiresAt: expiresAt.toISOString(),
        reason,
        renewCount: 0,
      };

      const lockFile: LockFile = {
        version: '1.0.0',
        lock,
        history: [],
      };

      await atomicWriteJson(lockPath, lockFile);

      const lockKey = this.getLockKey(type, resource);
      this.heldLocks.set(lockKey, lock);

      debug(`Lock acquired: ${lockKey}`);

      return {
        acquired: true,
        lock,
      };
    } finally {
      await sentinel.release();
    }
  }

  /**
   * Renew an existing lock
   */
  private async renewLock(
    lockPath: string,
    existingLock: LockFile,
    reason?: string
  ): Promise<LockAcquisitionResult> {
    const now = new Date();
    const expiresAt = new Date(now.getTime() + this.config.lockTimeoutMs);

    const lock: LockRecord = {
      ...existingLock.lock,
      expiresAt: expiresAt.toISOString(),
      renewCount: existingLock.lock.renewCount + 1,
      reason: reason ?? existingLock.lock.reason,
    };

    const lockFile: LockFile = {
      ...existingLock,
      lock,
    };

    await atomicWriteJson(lockPath, lockFile);

    const lockKey = this.getLockKey(lock.type, lock.resource);
    this.heldLocks.set(lockKey, lock);

    debug(`Lock renewed: ${lockKey} (count=${lock.renewCount})`);

    return {
      acquired: true,
      lock,
    };
  }

  /**
   * Force acquire a lock (taking over from expired/stale holder)
   *
   * Takeover protocol (CIB-117) — single-winner and crash-safe:
   *
   * 1. Acquire the O_EXCL `.creating` sentinel (shared with {@link createLock})
   *    so at most one agent may write this lock file at a time.
   *    `atomicWriteJson` is temp-file + rename — atomic against torn reads but
   *    last-write-wins, so it cannot fence concurrent takeovers by itself.
   * 2. Under the sentinel, RE-READ the lock file and re-verify the takeover
   *    precondition (expired, or stale holder when permitted) against the
   *    freshest state. A concurrent taker that won between our unfenced read
   *    and our sentinel acquisition leaves a fresh record — we back off
   *    instead of overwriting it.
   * 3. Only then write the new record; the sentinel is released afterwards.
   *
   * Sentinels abandoned by a crashed holder are reaped after
   * {@link SENTINEL_STALE_MS} via an atomic rename-aside (see
   * {@link acquireCreationSentinel}), so a crash mid-takeover cannot wedge the
   * lock permanently.
   */
  private async forceAcquire(
    lockPath: string,
    type: LockType,
    resource: string,
    reason: string | undefined,
    timeoutMs: number,
    acquireFromStale: boolean
  ): Promise<LockAcquisitionResult> {
    const sentinel = await this.acquireCreationSentinel(lockPath);

    if (!sentinel) {
      // Another agent is mid-takeover (or mid-creation) — back off.
      const current = await this.readLock(lockPath);
      return {
        acquired: false,
        error: current
          ? `Lock takeover contended; held by ${current.lock.agentId}`
          : 'Lock takeover in progress by another agent',
        heldBy: current ? this.heldByInfo(current) : undefined,
      };
    }

    try {
      // Re-verify the takeover precondition against the freshest state.
      const current = await this.readLock(lockPath);

      if (current) {
        if (current.lock.agentId === this.agent.id) {
          // We already own it (e.g. a concurrent call from this agent won).
          return this.renewLock(lockPath, current, reason);
        }

        const nowMs = Date.now();
        const expired = nowMs > new Date(current.lock.expiresAt).getTime();
        const stale = !expired && acquireFromStale && (await this.isHolderStale(current.lock));

        if (!expired && !stale) {
          // A concurrent taker won and wrote a fresh record — do not overwrite.
          return {
            acquired: false,
            error: `Lock held by ${current.lock.agentId}`,
            heldBy: this.heldByInfo(current),
          };
        }
      }

      const now = new Date();
      const expiresAt = new Date(now.getTime() + timeoutMs);

      const lock: LockRecord = {
        type,
        resource,
        agentId: this.agent.id,
        agentType: this.agent.type,
        pid: this.agent.pid,
        acquiredAt: now.toISOString(),
        expiresAt: expiresAt.toISOString(),
        reason,
        renewCount: 0,
      };

      const lockFile: LockFile = {
        version: '1.0.0',
        lock,
        history: current
          ? [
              ...current.history,
              {
                agentId: current.lock.agentId,
                acquiredAt: current.lock.acquiredAt,
                releasedAt: now.toISOString(),
                reason: 'Force-released (expired/stale)',
              },
            ]
          : [],
      };

      await atomicWriteJson(lockPath, lockFile);

      const lockKey = this.getLockKey(type, resource);
      this.heldLocks.set(lockKey, lock);

      debug(`Lock force-acquired: ${lockKey}`);

      return {
        acquired: true,
        lock,
      };
    } finally {
      await sentinel.release();
    }
  }

  /**
   * Acquire the O_EXCL `.creating` sentinel that fences all writers of a
   * lock file (creation and takeover share it).
   *
   * If the sentinel already exists but is older than {@link SENTINEL_STALE_MS}
   * its holder crashed mid-write; it is reaped via an atomic rename-aside so
   * that exactly one contender wins the reap even under contention, then the
   * O_EXCL create is retried once.
   *
   * The sentinel content is a unique fencing token (agent id + UUID): if this
   * holder is itself reaped while paused past the stale threshold, its
   * release() sees a token mismatch and becomes a no-op instead of deleting
   * the next holder's live sentinel (see createLockHandle in atomic.ts).
   */
  private async acquireCreationSentinel(lockPath: string): Promise<FileLockHandle | null> {
    const sentinelPath = `${lockPath}.creating`;
    const token = `${this.agent.id}:${randomUUID()}`;

    const handle = await tryAcquireFileLock(sentinelPath, token);
    if (handle) {
      return handle;
    }

    try {
      const stats = await fs.stat(sentinelPath);
      if (Date.now() - stats.mtime.getTime() < SENTINEL_STALE_MS) {
        return null; // Held by a live writer — back off.
      }

      // Abandoned sentinel: rename-aside is atomic, so only one reaper
      // succeeds; losers get ENOENT and simply retry the O_EXCL create.
      const reapPath = `${sentinelPath}.${randomUUID().slice(0, 8)}.reaped`;
      await fs.rename(sentinelPath, reapPath);
      await unlinkSafe(reapPath);
      debug(`Reaped abandoned sentinel: ${sentinelPath}`);
    } catch {
      // Sentinel released or reaped by someone else between our attempts.
    }

    return tryAcquireFileLock(sentinelPath, token);
  }

  /**
   * Build the `heldBy` summary for a lock file
   */
  private heldByInfo(lockFile: LockFile): NonNullable<LockAcquisitionResult['heldBy']> {
    return {
      agentId: lockFile.lock.agentId,
      agentType: lockFile.lock.agentType,
      acquiredAt: lockFile.lock.acquiredAt,
      expiresAt: lockFile.lock.expiresAt,
      pid: lockFile.lock.pid,
    };
  }

  /**
   * Check if lock holder is stale
   */
  private async isHolderStale(lock: LockRecord): Promise<boolean> {
    // Check if process is still running
    if (lock.pid) {
      try {
        process.kill(lock.pid, 0);
        return false; // Process exists
      } catch {
        return true; // Process doesn't exist
      }
    }

    // Can't determine - assume not stale
    return false;
  }

  /**
   * Read lock file
   */
  private async readLock(lockPath: string): Promise<LockFile | null> {
    const data = await readJsonSafe(lockPath);
    if (!data) return null;

    const result = LockFileSchema.safeParse(data);
    if (!result.success) {
      debug('Invalid lock file schema:', result.error);
      return null;
    }

    return result.data;
  }

  /**
   * Get lock key from type and resource
   */
  private getLockKey(type: LockType, resource: string): string {
    return `${type}:${resource}`;
  }

  /**
   * Get lock file path
   */
  private getLockPath(lockKey: string): string {
    // Hash the key for safe filename
    const hash = createHash('sha256').update(lockKey).digest('hex').slice(0, 16);
    return join(this.lockDir, `${hash}.lock`);
  }

  /**
   * Ensure lock directory exists
   */
  private async ensureLockDir(): Promise<void> {
    await fs.mkdir(this.lockDir, { recursive: true });
  }
}

/**
 * Create a lock manager
 */
export function createLockManager(options: LockManagerOptions): LockManager {
  return new LockManager(options);
}

// ============================================================================
// Convenience Functions
// ============================================================================

/**
 * Execute a function while holding a lock
 *
 * @example
 * ```typescript
 * const result = await withLock(
 *   lockManager,
 *   { type: 'action', resource: 'gate' },
 *   async () => {
 *     // Perform work that requires exclusive access
 *     return await runGates();
 *   }
 * );
 * ```
 */
export async function withLock<T>(
  manager: LockManager,
  options: Omit<AcquireLockOptions, 'wait'>,
  fn: () => Promise<T>
): Promise<T> {
  const result = await manager.acquire({ ...options, wait: true });

  if (!result.acquired) {
    throw new Error(`Failed to acquire lock: ${result.error}`);
  }

  try {
    return await fn();
  } finally {
    await manager.release(options.type, options.resource);
  }
}

/**
 * Try to execute a function while holding a lock (non-blocking)
 *
 * Returns null if lock couldn't be acquired.
 */
export async function tryWithLock<T>(
  manager: LockManager,
  options: Omit<AcquireLockOptions, 'wait'>,
  fn: () => Promise<T>
): Promise<
  { success: true; result: T } | { success: false; heldBy?: LockAcquisitionResult['heldBy'] }
> {
  const result = await manager.acquire({ ...options, wait: false });

  if (!result.acquired) {
    return { success: false, heldBy: result.heldBy };
  }

  try {
    const fnResult = await fn();
    return { success: true, result: fnResult };
  } finally {
    await manager.release(options.type, options.resource);
  }
}
