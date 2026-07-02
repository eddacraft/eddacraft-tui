/**
 * Atomic File Operations
 *
 * Provides atomic read/write operations for JSON files to prevent
 * corruption in multi-agent/multi-process scenarios.
 *
 * Uses the atomic write pattern: write to temp file, then rename.
 * This ensures that either the old or new content is visible, never partial.
 */

import { promises as fs } from 'node:fs';
import { dirname, join, basename } from 'node:path';
import { randomUUID } from 'node:crypto';
import { createDebugger } from '@eddacraft/anvil-core';

const debug = createDebugger('atomic');

/**
 * Atomic write options
 */
export interface AtomicWriteOptions {
  /** File mode (default: 0o644) */
  mode?: number;

  /** Retry count for rename conflicts (default: 3) */
  retries?: number;

  /** Create parent directories if needed (default: true) */
  createDirs?: boolean;
}

/**
 * Write JSON atomically
 *
 * 1. Write content to a temp file in the same directory
 * 2. Rename temp file to target (atomic on most filesystems)
 * 3. Clean up temp file on error
 */
export async function atomicWriteJson(
  filePath: string,
  data: unknown,
  options: AtomicWriteOptions = {}
): Promise<void> {
  const { mode = 0o644, retries = 3, createDirs = true } = options;

  const dir = dirname(filePath);
  const tempPath = join(dir, `.${basename(filePath)}.${randomUUID().slice(0, 8)}.tmp`);

  // Ensure directory exists
  if (createDirs) {
    await fs.mkdir(dir, { recursive: true });
  }

  const content = JSON.stringify(data, null, 2);

  let lastError: Error | null = null;

  for (let attempt = 0; attempt < retries; attempt++) {
    try {
      // Write to temp file
      await fs.writeFile(tempPath, content, { encoding: 'utf-8', mode });

      // Atomic rename
      await fs.rename(tempPath, filePath);

      debug(`Atomic write successful: ${filePath}`);
      return;
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));
      debug(`Atomic write attempt ${attempt + 1} failed:`, error);

      // Clean up temp file if it exists
      try {
        await fs.unlink(tempPath);
      } catch {
        // Ignore cleanup errors
      }

      // Small delay before retry
      if (attempt < retries - 1) {
        await sleep(10 * (attempt + 1));
      }
    }
  }

  throw new Error(`Atomic write failed after ${retries} attempts: ${lastError?.message}`);
}

/**
 * Write text atomically
 */
export async function atomicWriteText(
  filePath: string,
  content: string,
  options: AtomicWriteOptions = {}
): Promise<void> {
  const { mode = 0o644, retries = 3, createDirs = true } = options;

  const dir = dirname(filePath);
  const tempPath = join(dir, `.${basename(filePath)}.${randomUUID().slice(0, 8)}.tmp`);

  if (createDirs) {
    await fs.mkdir(dir, { recursive: true });
  }

  let lastError: Error | null = null;

  for (let attempt = 0; attempt < retries; attempt++) {
    try {
      await fs.writeFile(tempPath, content, { encoding: 'utf-8', mode });
      await fs.rename(tempPath, filePath);
      debug(`Atomic write successful: ${filePath}`);
      return;
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));
      debug(`Atomic write attempt ${attempt + 1} failed:`, error);

      try {
        await fs.unlink(tempPath);
      } catch {
        // Ignore
      }

      if (attempt < retries - 1) {
        await sleep(10 * (attempt + 1));
      }
    }
  }

  throw new Error(`Atomic write failed after ${retries} attempts: ${lastError?.message}`);
}

/**
 * Read JSON safely (returns null if file doesn't exist or is invalid)
 */
export async function readJsonSafe<T = unknown>(filePath: string): Promise<T | null> {
  try {
    const content = await fs.readFile(filePath, 'utf-8');
    return JSON.parse(content) as T;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      return null;
    }
    debug(`Failed to read JSON from ${filePath}:`, error);
    return null;
  }
}

/**
 * Read JSON with retries (for handling transient lock conflicts)
 */
export async function readJsonWithRetry<T = unknown>(
  filePath: string,
  retries = 3,
  delayMs = 10
): Promise<T | null> {
  let lastError: Error | null = null;

  for (let attempt = 0; attempt < retries; attempt++) {
    try {
      const content = await fs.readFile(filePath, 'utf-8');
      return JSON.parse(content) as T;
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));

      if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
        return null;
      }

      debug(`Read attempt ${attempt + 1} failed for ${filePath}:`, error);

      if (attempt < retries - 1) {
        await sleep(delayMs * (attempt + 1));
      }
    }
  }

  debug(`Read failed after ${retries} attempts: ${lastError?.message}`);
  return null;
}

/**
 * Atomic file lock using a lock file
 *
 * Uses O_EXCL flag for atomic creation - only succeeds if file doesn't exist.
 * This provides a simple but robust mutual exclusion mechanism.
 */
export interface FileLockOptions {
  /** Maximum time to wait for lock (ms) */
  timeout?: number;

  /** Retry interval (ms) */
  retryInterval?: number;

  /** Lock file content */
  content?: string;
}

export interface FileLockHandle {
  /** Path to the lock file */
  path: string;

  /** Release the lock */
  release: () => Promise<void>;
}

/**
 * Build a lock handle whose release verifies ownership before unlinking
 * (CIB-117 fencing token).
 *
 * If a stale-lock reaper stole this lock while the holder was paused (GC
 * pause, disk stall) and a new holder re-created it, the on-disk content no
 * longer matches what this holder wrote — release then becomes a no-op
 * instead of deleting the new holder's live lock. A small read→unlink window
 * remains, but the token check shrinks the exposure from "any time after
 * theft" to microseconds.
 */
function createLockHandle(lockPath: string, content: string): FileLockHandle {
  return {
    path: lockPath,
    release: async () => {
      try {
        const current = await fs.readFile(lockPath, 'utf-8');
        if (current !== content) {
          debug(`Lock theft detected — not releasing ${lockPath} (held by another writer)`);
          return;
        }
        await fs.unlink(lockPath);
        debug(`Lock released: ${lockPath}`);
      } catch (error) {
        debug(`Lock release failed: ${lockPath}`, error);
      }
    },
  };
}

/**
 * Acquire a file lock (blocks until acquired or timeout)
 */
export async function acquireFileLock(
  lockPath: string,
  options: FileLockOptions = {}
): Promise<FileLockHandle | null> {
  const { timeout = 5000, retryInterval = 50, content = '' } = options;

  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    try {
      // O_EXCL: fail if file exists
      const fd = await fs.open(lockPath, 'wx');
      await fd.writeFile(content);
      await fd.close();

      debug(`Lock acquired: ${lockPath}`);

      return createLockHandle(lockPath, content);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== 'EEXIST') {
        // Unexpected error
        throw error;
      }

      // Lock exists, wait and retry
      await sleep(retryInterval);
    }
  }

  debug(`Lock acquisition timed out: ${lockPath}`);
  return null;
}

/**
 * Try to acquire a file lock (non-blocking)
 */
export async function tryAcquireFileLock(
  lockPath: string,
  content = ''
): Promise<FileLockHandle | null> {
  try {
    const dir = dirname(lockPath);
    await fs.mkdir(dir, { recursive: true });

    const fd = await fs.open(lockPath, 'wx');
    await fd.writeFile(content);
    await fd.close();

    debug(`Lock acquired: ${lockPath}`);

    return createLockHandle(lockPath, content);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'EEXIST') {
      return null;
    }
    throw error;
  }
}

/**
 * Check if a lock file exists
 */
export async function isLocked(lockPath: string): Promise<boolean> {
  try {
    await fs.access(lockPath);
    return true;
  } catch {
    return false;
  }
}

/**
 * Force release a lock (use with caution)
 */
export async function forceReleaseLock(lockPath: string): Promise<boolean> {
  try {
    await fs.unlink(lockPath);
    debug(`Lock force-released: ${lockPath}`);
    return true;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      return false;
    }
    throw error;
  }
}

/**
 * Delete file if it exists (no error if missing)
 */
export async function unlinkSafe(filePath: string): Promise<boolean> {
  try {
    await fs.unlink(filePath);
    return true;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      return false;
    }
    throw error;
  }
}

/**
 * Check if file exists
 */
export async function fileExists(filePath: string): Promise<boolean> {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

/**
 * Get file modification time
 */
export async function getFileMtime(filePath: string): Promise<Date | null> {
  try {
    const stats = await fs.stat(filePath);
    return stats.mtime;
  } catch {
    return null;
  }
}

/**
 * Simple sleep utility
 */
function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Sleep with jitter (to prevent thundering herd)
 */
export function sleepWithJitter(baseMs: number, jitterPercent = 0.2): Promise<void> {
  const jitter = baseMs * jitterPercent * (Math.random() * 2 - 1);
  return sleep(Math.max(0, baseMs + jitter));
}
