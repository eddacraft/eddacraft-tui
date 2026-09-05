/** Private atomic text replacement for the file cache. */

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

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
