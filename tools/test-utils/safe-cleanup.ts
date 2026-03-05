import { rm } from 'node:fs/promises';

/**
 * Async directory removal with Windows EBUSY/EPERM retry logic.
 * Uses maxRetries: 10 / retryDelay: 100 ms (same as tempy v3.2.0).
 *
 * Transient lock errors (EBUSY, EPERM, ENOTEMPTY) are swallowed after
 * retries so a stuck temp dir never masks actual test results.
 * Non-transient errors (e.g. EACCES, invalid path) re-throw.
 * The `force: true` option handles ENOENT, so no existence check needed.
 */
export async function safeCleanup(dirPath: string): Promise<void> {
  try {
    await rm(dirPath, {
      recursive: true,
      force: true,
      maxRetries: 10,
      retryDelay: 100,
    });
  } catch (error: unknown) {
    // Only swallow transient Windows file-lock errors (EBUSY, EPERM,
    // ENOTEMPTY) that persist after maxRetries — these are expected when
    // a handle hasn't been released yet. Re-throw anything else (e.g.
    // permission mistakes, invalid paths) so real regressions surface.
    const code = (error as NodeJS.ErrnoException).code;
    if (code !== 'EBUSY' && code !== 'EPERM' && code !== 'ENOTEMPTY') {
      throw error;
    }
  }
}
