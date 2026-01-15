/**
 * Change Debouncer
 *
 * Coalesces rapid file changes into batches to prevent
 * excessive action triggers during editor auto-save or
 * multi-file operations.
 */

import type { DebouncedChanges } from './types.js';

/**
 * Callback type for debounced flush
 */
export type DebouncerFlushCallback = (changes: DebouncedChanges) => void;

/**
 * Debouncer for file changes
 *
 * Accumulates file paths and flushes them as a batch
 * after a configurable delay.
 */
export class ChangeDebouncer {
  private pendingFiles: Set<string> = new Set();
  private timer: ReturnType<typeof setTimeout> | null = null;

  /**
   * Create a new debouncer
   *
   * @param delayMs - Debounce delay in milliseconds
   * @param onFlush - Callback when changes are flushed
   */
  constructor(
    private delayMs: number,
    private onFlush: DebouncerFlushCallback
  ) {}

  /**
   * Add a file to the pending changes
   *
   * Resets the debounce timer each time a file is added.
   *
   * @param filePath - Absolute file path
   */
  add(filePath: string): void {
    this.pendingFiles.add(filePath);
    this.resetTimer();
  }

  /**
   * Add multiple files to pending changes
   *
   * @param filePaths - Array of absolute file paths
   */
  addMany(filePaths: string[]): void {
    for (const filePath of filePaths) {
      this.pendingFiles.add(filePath);
    }
    this.resetTimer();
  }

  /**
   * Immediately flush pending changes
   *
   * Clears the timer and invokes the callback with all
   * accumulated files.
   */
  flush(): void {
    this.clearTimer();

    if (this.pendingFiles.size === 0) {
      return;
    }

    const files = Array.from(this.pendingFiles);
    this.pendingFiles.clear();

    this.onFlush({
      files,
      timestamp: new Date(),
    });
  }

  /**
   * Cancel pending flush and clear accumulated files
   */
  cancel(): void {
    this.clearTimer();
    this.pendingFiles.clear();
  }

  /**
   * Get count of pending files
   */
  get pendingCount(): number {
    return this.pendingFiles.size;
  }

  /**
   * Check if there are pending changes
   */
  get hasPending(): boolean {
    return this.pendingFiles.size > 0;
  }

  /**
   * Get the current delay setting
   */
  get delay(): number {
    return this.delayMs;
  }

  /**
   * Update the debounce delay
   *
   * Takes effect on next add() call.
   */
  setDelay(delayMs: number): void {
    this.delayMs = delayMs;
  }

  /**
   * Reset the debounce timer
   */
  private resetTimer(): void {
    this.clearTimer();
    this.timer = setTimeout(() => {
      this.flush();
    }, this.delayMs);
  }

  /**
   * Clear the debounce timer
   */
  private clearTimer(): void {
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
  }
}

/**
 * Create a change debouncer
 *
 * @param delayMs - Debounce delay in milliseconds (default: 300)
 * @param onFlush - Callback when changes are flushed
 */
export function createDebouncer(delayMs: number, onFlush: DebouncerFlushCallback): ChangeDebouncer {
  return new ChangeDebouncer(delayMs, onFlush);
}
