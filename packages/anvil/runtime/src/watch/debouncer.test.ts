/**
 * Tests for ChangeDebouncer
 */

import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { ChangeDebouncer, createDebouncer } from './debouncer.js';
import type { DebouncedChanges } from './types.js';

describe('ChangeDebouncer', () => {
  let debouncer: ChangeDebouncer;
  let flushCallback: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.useFakeTimers();
    flushCallback = vi.fn();
    debouncer = new ChangeDebouncer(300, flushCallback);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  describe('constructor', () => {
    it('creates debouncer with specified delay and callback', () => {
      expect(debouncer.delay).toBe(300);
      expect(debouncer.pendingCount).toBe(0);
    });
  });

  describe('add', () => {
    it('adds a file to pending changes', () => {
      debouncer.add('/path/to/file.ts');

      expect(debouncer.pendingCount).toBe(1);
      expect(debouncer.hasPending).toBe(true);
    });

    it('deduplicates identical file paths', () => {
      debouncer.add('/path/to/file.ts');
      debouncer.add('/path/to/file.ts');
      debouncer.add('/path/to/file.ts');

      expect(debouncer.pendingCount).toBe(1);
    });

    it('adds multiple distinct files', () => {
      debouncer.add('/path/to/file1.ts');
      debouncer.add('/path/to/file2.ts');
      debouncer.add('/path/to/file3.ts');

      expect(debouncer.pendingCount).toBe(3);
    });

    it('resets timer on each add', () => {
      debouncer.add('/path/to/file1.ts');

      // Advance time partway through delay
      vi.advanceTimersByTime(200);
      expect(flushCallback).not.toHaveBeenCalled();

      // Add another file - should reset timer
      debouncer.add('/path/to/file2.ts');

      // Advance original remaining time
      vi.advanceTimersByTime(100);
      expect(flushCallback).not.toHaveBeenCalled();

      // Advance full delay from second add
      vi.advanceTimersByTime(200);
      expect(flushCallback).toHaveBeenCalledTimes(1);
    });

    it('triggers flush after delay expires', () => {
      debouncer.add('/path/to/file.ts');

      vi.advanceTimersByTime(299);
      expect(flushCallback).not.toHaveBeenCalled();

      vi.advanceTimersByTime(1);
      expect(flushCallback).toHaveBeenCalledTimes(1);
    });
  });

  describe('addMany', () => {
    it('adds multiple files at once', () => {
      debouncer.addMany(['/path/file1.ts', '/path/file2.ts', '/path/file3.ts']);

      expect(debouncer.pendingCount).toBe(3);
    });

    it('deduplicates files in the array', () => {
      debouncer.addMany(['/path/file1.ts', '/path/file1.ts', '/path/file2.ts']);

      expect(debouncer.pendingCount).toBe(2);
    });

    it('deduplicates with previously added files', () => {
      debouncer.add('/path/file1.ts');
      debouncer.addMany(['/path/file1.ts', '/path/file2.ts']);

      expect(debouncer.pendingCount).toBe(2);
    });

    it('handles empty array', () => {
      debouncer.addMany([]);

      expect(debouncer.pendingCount).toBe(0);
      expect(debouncer.hasPending).toBe(false);
    });

    it('resets timer after adding many', () => {
      debouncer.addMany(['/path/file1.ts', '/path/file2.ts']);

      vi.advanceTimersByTime(300);
      expect(flushCallback).toHaveBeenCalledTimes(1);
    });
  });

  describe('flush', () => {
    it('invokes callback with accumulated files', () => {
      debouncer.add('/path/file1.ts');
      debouncer.add('/path/file2.ts');

      debouncer.flush();

      expect(flushCallback).toHaveBeenCalledTimes(1);
      const changes = flushCallback.mock.calls[0][0] as DebouncedChanges;
      expect(changes.files).toHaveLength(2);
      expect(changes.files).toContain('/path/file1.ts');
      expect(changes.files).toContain('/path/file2.ts');
      expect(changes.timestamp).toBeInstanceOf(Date);
    });

    it('clears pending files after flush', () => {
      debouncer.add('/path/file1.ts');
      debouncer.add('/path/file2.ts');

      debouncer.flush();

      expect(debouncer.pendingCount).toBe(0);
      expect(debouncer.hasPending).toBe(false);
    });

    it('does nothing when no pending files', () => {
      debouncer.flush();

      expect(flushCallback).not.toHaveBeenCalled();
    });

    it('clears the timer', () => {
      debouncer.add('/path/file.ts');
      debouncer.flush();

      // Timer should be cleared, so advancing time shouldn't trigger another flush
      vi.advanceTimersByTime(300);
      expect(flushCallback).toHaveBeenCalledTimes(1);
    });

    it('can be called multiple times in succession', () => {
      debouncer.add('/path/file1.ts');
      debouncer.flush();

      debouncer.add('/path/file2.ts');
      debouncer.flush();

      expect(flushCallback).toHaveBeenCalledTimes(2);
      expect(flushCallback.mock.calls[0][0].files).toEqual(['/path/file1.ts']);
      expect(flushCallback.mock.calls[1][0].files).toEqual(['/path/file2.ts']);
    });
  });

  describe('cancel', () => {
    it('clears pending files', () => {
      debouncer.add('/path/file1.ts');
      debouncer.add('/path/file2.ts');

      debouncer.cancel();

      expect(debouncer.pendingCount).toBe(0);
      expect(debouncer.hasPending).toBe(false);
    });

    it('clears the timer', () => {
      debouncer.add('/path/file.ts');
      debouncer.cancel();

      // Advancing time should not trigger flush
      vi.advanceTimersByTime(300);
      expect(flushCallback).not.toHaveBeenCalled();
    });

    it('does nothing when no pending files', () => {
      debouncer.cancel();

      expect(debouncer.pendingCount).toBe(0);
    });

    it('allows new files to be added after cancel', () => {
      debouncer.add('/path/file1.ts');
      debouncer.cancel();

      debouncer.add('/path/file2.ts');
      vi.advanceTimersByTime(300);

      expect(flushCallback).toHaveBeenCalledTimes(1);
      expect(flushCallback.mock.calls[0][0].files).toEqual(['/path/file2.ts']);
    });
  });

  describe('pendingCount', () => {
    it('returns zero initially', () => {
      expect(debouncer.pendingCount).toBe(0);
    });

    it('returns correct count after adding files', () => {
      debouncer.add('/path/file1.ts');
      expect(debouncer.pendingCount).toBe(1);

      debouncer.add('/path/file2.ts');
      expect(debouncer.pendingCount).toBe(2);
    });

    it('returns zero after flush', () => {
      debouncer.add('/path/file.ts');
      debouncer.flush();

      expect(debouncer.pendingCount).toBe(0);
    });
  });

  describe('hasPending', () => {
    it('returns false initially', () => {
      expect(debouncer.hasPending).toBe(false);
    });

    it('returns true when files are pending', () => {
      debouncer.add('/path/file.ts');
      expect(debouncer.hasPending).toBe(true);
    });

    it('returns false after flush', () => {
      debouncer.add('/path/file.ts');
      debouncer.flush();

      expect(debouncer.hasPending).toBe(false);
    });

    it('returns false after cancel', () => {
      debouncer.add('/path/file.ts');
      debouncer.cancel();

      expect(debouncer.hasPending).toBe(false);
    });
  });

  describe('delay', () => {
    it('returns the configured delay', () => {
      const customDebouncer = new ChangeDebouncer(500, flushCallback);
      expect(customDebouncer.delay).toBe(500);
    });
  });

  describe('setDelay', () => {
    it('updates the delay value', () => {
      debouncer.setDelay(500);
      expect(debouncer.delay).toBe(500);
    });

    it('affects next timer on subsequent add', () => {
      debouncer.setDelay(100);
      debouncer.add('/path/file.ts');

      vi.advanceTimersByTime(99);
      expect(flushCallback).not.toHaveBeenCalled();

      vi.advanceTimersByTime(1);
      expect(flushCallback).toHaveBeenCalledTimes(1);
    });

    it('does not affect currently running timer', () => {
      debouncer.add('/path/file.ts');

      // Change delay while timer is running
      debouncer.setDelay(100);

      // Original timer (300ms) should still be in effect
      vi.advanceTimersByTime(100);
      expect(flushCallback).not.toHaveBeenCalled();

      vi.advanceTimersByTime(200);
      expect(flushCallback).toHaveBeenCalledTimes(1);
    });

    it('allows zero delay for immediate flush', () => {
      debouncer.setDelay(0);
      debouncer.add('/path/file.ts');

      vi.advanceTimersByTime(0);
      expect(flushCallback).toHaveBeenCalledTimes(1);
    });
  });

  describe('debounce behavior', () => {
    it('coalesces rapid changes into single flush', () => {
      debouncer.add('/path/file1.ts');
      vi.advanceTimersByTime(50);

      debouncer.add('/path/file2.ts');
      vi.advanceTimersByTime(50);

      debouncer.add('/path/file3.ts');
      vi.advanceTimersByTime(50);

      debouncer.add('/path/file4.ts');

      // Still no flush after short time
      vi.advanceTimersByTime(100);
      expect(flushCallback).not.toHaveBeenCalled();

      // Complete the full delay from last add (remaining 200ms)
      vi.advanceTimersByTime(200);

      expect(flushCallback).toHaveBeenCalledTimes(1);
      const changes = flushCallback.mock.calls[0][0];
      expect(changes.files).toHaveLength(4);
      expect(changes.files).toContain('/path/file1.ts');
      expect(changes.files).toContain('/path/file2.ts');
      expect(changes.files).toContain('/path/file3.ts');
      expect(changes.files).toContain('/path/file4.ts');
    });

    it('allows separate flushes when changes are spaced out', () => {
      debouncer.add('/path/file1.ts');
      vi.advanceTimersByTime(300);

      expect(flushCallback).toHaveBeenCalledTimes(1);
      expect(flushCallback.mock.calls[0][0].files).toEqual(['/path/file1.ts']);

      debouncer.add('/path/file2.ts');
      vi.advanceTimersByTime(300);

      expect(flushCallback).toHaveBeenCalledTimes(2);
      expect(flushCallback.mock.calls[1][0].files).toEqual(['/path/file2.ts']);
    });
  });

  describe('timestamp', () => {
    it('includes current timestamp in flushed changes', () => {
      const beforeFlush = new Date();
      debouncer.add('/path/file.ts');
      debouncer.flush();
      const afterFlush = new Date();

      const changes = flushCallback.mock.calls[0][0] as DebouncedChanges;
      expect(changes.timestamp.getTime()).toBeGreaterThanOrEqual(beforeFlush.getTime());
      expect(changes.timestamp.getTime()).toBeLessThanOrEqual(afterFlush.getTime());
    });

    it('updates timestamp on each flush', () => {
      debouncer.add('/path/file1.ts');
      debouncer.flush();
      const firstTimestamp = flushCallback.mock.calls[0][0].timestamp;

      vi.advanceTimersByTime(1000);

      debouncer.add('/path/file2.ts');
      debouncer.flush();
      const secondTimestamp = flushCallback.mock.calls[1][0].timestamp;

      expect(secondTimestamp.getTime()).toBeGreaterThan(firstTimestamp.getTime());
    });
  });
});

describe('createDebouncer', () => {
  it('creates a ChangeDebouncer instance', () => {
    const callback = vi.fn();
    const debouncer = createDebouncer(300, callback);

    expect(debouncer).toBeInstanceOf(ChangeDebouncer);
    expect(debouncer.delay).toBe(300);
  });

  it('returns functional debouncer', () => {
    vi.useFakeTimers();
    const callback = vi.fn();
    const debouncer = createDebouncer(200, callback);

    debouncer.add('/path/file.ts');
    vi.advanceTimersByTime(200);

    expect(callback).toHaveBeenCalledTimes(1);

    vi.useRealTimers();
  });
});
