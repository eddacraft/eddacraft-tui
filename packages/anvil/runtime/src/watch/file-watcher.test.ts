/**
 * Tests for FileWatcher
 *
 * chokidar v5 removed glob support, so FileWatcher now watches the cwd
 * directory and filters events through minimatch include/exclude patterns.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { FileWatcher, createFileWatcher } from './file-watcher.js';
import type { WatchChangeEvent } from './types.js';
import { EventEmitter } from 'node:events';

// Mock chokidar
const mockChokidarWatcher = {
  on: vi.fn().mockReturnThis(),
  close: vi.fn().mockResolvedValue(undefined),
};

const mockChokidar = {
  watch: vi.fn().mockReturnValue(mockChokidarWatcher),
};

vi.mock('chokidar', () => mockChokidar);

describe('FileWatcher', () => {
  let watcher: FileWatcher;

  beforeEach(() => {
    vi.clearAllMocks();
    watcher = new FileWatcher();
  });

  afterEach(async () => {
    if (watcher.running) {
      await watcher.stop();
    }
    vi.restoreAllMocks();
  });

  describe('constructor', () => {
    it('creates an instance of EventEmitter', () => {
      expect(watcher).toBeInstanceOf(EventEmitter);
    });

    it('starts in stopped state', () => {
      expect(watcher.running).toBe(false);
      expect(watcher.ready).toBe(false);
    });
  });

  describe('start', () => {
    it('watches the cwd directory instead of glob patterns', async () => {
      await watcher.start({
        patterns: ['**/*.ts', '**/*.js'],
        exclude: ['node_modules/**', 'dist/**'],
        cwd: '/workspace',
        depth: 5,
      });

      // chokidar v5: first arg is the cwd, not the patterns
      expect(mockChokidar.watch).toHaveBeenCalledWith(
        '/workspace',
        expect.objectContaining({
          ignored: expect.any(Function),
          persistent: true,
          ignoreInitial: true,
          depth: 5,
          awaitWriteFinish: {
            stabilityThreshold: 100,
            pollInterval: 50,
          },
        })
      );
    });

    it('uses default depth when not provided', async () => {
      await watcher.start({
        patterns: ['**/*.md'],
        exclude: [],
        cwd: '/workspace',
      });

      expect(mockChokidar.watch).toHaveBeenCalledWith(
        '/workspace',
        expect.objectContaining({
          depth: 10,
        })
      );
    });

    it('sets running state to true', async () => {
      await watcher.start({
        patterns: ['**/*.ts'],
        exclude: [],
        cwd: '/workspace',
      });

      expect(watcher.running).toBe(true);
    });

    it('throws error if already started', async () => {
      await watcher.start({
        patterns: ['**/*.ts'],
        exclude: [],
        cwd: '/workspace',
      });

      await expect(
        watcher.start({
          patterns: ['**/*.ts'],
          exclude: [],
          cwd: '/workspace',
        })
      ).rejects.toThrow('Watcher already started');
    });

    it('registers event handlers on chokidar watcher', async () => {
      await watcher.start({
        patterns: ['**/*.ts'],
        exclude: [],
        cwd: '/workspace',
      });

      expect(mockChokidarWatcher.on).toHaveBeenCalledWith('add', expect.any(Function));
      expect(mockChokidarWatcher.on).toHaveBeenCalledWith('change', expect.any(Function));
      expect(mockChokidarWatcher.on).toHaveBeenCalledWith('unlink', expect.any(Function));
      expect(mockChokidarWatcher.on).toHaveBeenCalledWith('error', expect.any(Function));
      expect(mockChokidarWatcher.on).toHaveBeenCalledWith('ready', expect.any(Function));
    });
  });

  describe('stop', () => {
    it('closes the chokidar watcher', async () => {
      await watcher.start({
        patterns: ['**/*.ts'],
        exclude: [],
        cwd: '/workspace',
      });

      await watcher.stop();

      expect(mockChokidarWatcher.close).toHaveBeenCalledTimes(1);
    });

    it('sets running and ready to false', async () => {
      await watcher.start({
        patterns: ['**/*.ts'],
        exclude: [],
        cwd: '/workspace',
      });

      await watcher.stop();

      expect(watcher.running).toBe(false);
      expect(watcher.ready).toBe(false);
    });

    it('does nothing when not running', async () => {
      await watcher.stop();

      expect(mockChokidarWatcher.close).not.toHaveBeenCalled();
    });

    it('can be called multiple times safely', async () => {
      await watcher.start({
        patterns: ['**/*.ts'],
        exclude: [],
        cwd: '/workspace',
      });

      await watcher.stop();
      await watcher.stop();
      await watcher.stop();

      expect(mockChokidarWatcher.close).toHaveBeenCalledTimes(1);
    });
  });

  describe('change events', () => {
    let changeListener: ReturnType<typeof vi.fn>;

    beforeEach(async () => {
      changeListener = vi.fn();
      watcher.on('change', changeListener);

      await watcher.start({
        patterns: ['**/*.ts'],
        exclude: [],
        cwd: '/workspace',
      });
    });

    it('emits change event for files matching include patterns', () => {
      const addHandler = mockChokidarWatcher.on.mock.calls.find((call) => call[0] === 'add')?.[1];
      expect(addHandler).toBeDefined();

      // chokidar v5 emits absolute paths when watching a directory
      addHandler('/workspace/src/file.ts');

      expect(changeListener).toHaveBeenCalledTimes(1);
      const event: WatchChangeEvent = changeListener.mock.calls[0][0];
      expect(event.type).toBe('add');
      expect(event.path).toBe('/workspace/src/file.ts');
      expect(event.timestamp).toBeInstanceOf(Date);
    });

    it('filters out files not matching include patterns', () => {
      const addHandler = mockChokidarWatcher.on.mock.calls.find((call) => call[0] === 'add')?.[1];
      expect(addHandler).toBeDefined();

      // .json file doesn't match **/*.ts
      addHandler('/workspace/src/config.json');

      expect(changeListener).not.toHaveBeenCalled();
    });

    it('emits change event on file change', () => {
      const changeHandler = mockChokidarWatcher.on.mock.calls.find(
        (call) => call[0] === 'change'
      )?.[1];
      expect(changeHandler).toBeDefined();

      changeHandler('/workspace/src/file.ts');

      expect(changeListener).toHaveBeenCalledTimes(1);
      const event: WatchChangeEvent = changeListener.mock.calls[0][0];
      expect(event.type).toBe('change');
      expect(event.path).toBe('/workspace/src/file.ts');
    });

    it('emits change event on file unlink', () => {
      const unlinkHandler = mockChokidarWatcher.on.mock.calls.find(
        (call) => call[0] === 'unlink'
      )?.[1];
      expect(unlinkHandler).toBeDefined();

      unlinkHandler('/workspace/src/file.ts');

      expect(changeListener).toHaveBeenCalledTimes(1);
      const event: WatchChangeEvent = changeListener.mock.calls[0][0];
      expect(event.type).toBe('unlink');
      expect(event.path).toBe('/workspace/src/file.ts');
    });

    it('handles paths within workspace', () => {
      const addHandler = mockChokidarWatcher.on.mock.calls.find((call) => call[0] === 'add')?.[1];
      expect(addHandler).toBeDefined();

      addHandler('/workspace/nested/deep/file.ts');

      const event: WatchChangeEvent = changeListener.mock.calls[0][0];
      expect(event.path).toBe('/workspace/nested/deep/file.ts');
    });

    it('includes timestamp in events', () => {
      const beforeEvent = new Date();
      const changeHandler = mockChokidarWatcher.on.mock.calls.find(
        (call) => call[0] === 'change'
      )?.[1];
      changeHandler('/workspace/file.ts');
      const afterEvent = new Date();

      const event: WatchChangeEvent = changeListener.mock.calls[0][0];
      expect(event.timestamp.getTime()).toBeGreaterThanOrEqual(beforeEvent.getTime());
      expect(event.timestamp.getTime()).toBeLessThanOrEqual(afterEvent.getTime());
    });
  });

  describe('error events', () => {
    let errorListener: ReturnType<typeof vi.fn>;

    beforeEach(async () => {
      errorListener = vi.fn();
      watcher.on('error', errorListener);

      await watcher.start({
        patterns: ['**/*.ts'],
        exclude: [],
        cwd: '/workspace',
      });
    });

    it('emits error event from chokidar', () => {
      const errorHandler = mockChokidarWatcher.on.mock.calls.find(
        (call) => call[0] === 'error'
      )?.[1];
      expect(errorHandler).toBeDefined();

      const testError = new Error('Watch error');
      errorHandler(testError);

      expect(errorListener).toHaveBeenCalledTimes(1);
      expect(errorListener).toHaveBeenCalledWith(testError);
    });
  });

  describe('ready events', () => {
    let readyListener: ReturnType<typeof vi.fn>;

    beforeEach(async () => {
      readyListener = vi.fn();
      watcher.on('ready', readyListener);

      await watcher.start({
        patterns: ['**/*.ts'],
        exclude: [],
        cwd: '/workspace',
      });
    });

    it('emits ready event from chokidar', () => {
      expect(watcher.ready).toBe(false);

      const readyHandler = mockChokidarWatcher.on.mock.calls.find(
        (call) => call[0] === 'ready'
      )?.[1];
      expect(readyHandler).toBeDefined();

      readyHandler();

      expect(readyListener).toHaveBeenCalledTimes(1);
      expect(watcher.ready).toBe(true);
    });

    it('sets ready state on ready event', () => {
      const readyHandler = mockChokidarWatcher.on.mock.calls.find(
        (call) => call[0] === 'ready'
      )?.[1];
      readyHandler();

      expect(watcher.ready).toBe(true);
    });

    it('resets ready state on stop', async () => {
      const readyHandler = mockChokidarWatcher.on.mock.calls.find(
        (call) => call[0] === 'ready'
      )?.[1];
      readyHandler();

      expect(watcher.ready).toBe(true);

      await watcher.stop();

      expect(watcher.ready).toBe(false);
    });
  });

  describe('multiple event listeners', () => {
    it('supports multiple change listeners', async () => {
      const listener1 = vi.fn();
      const listener2 = vi.fn();
      const listener3 = vi.fn();

      watcher.on('change', listener1);
      watcher.on('change', listener2);
      watcher.on('change', listener3);

      await watcher.start({
        patterns: ['**/*.ts'],
        exclude: [],
        cwd: '/workspace',
      });

      const addHandler = mockChokidarWatcher.on.mock.calls.find((call) => call[0] === 'add')?.[1];
      addHandler('/workspace/file.ts');

      expect(listener1).toHaveBeenCalledTimes(1);
      expect(listener2).toHaveBeenCalledTimes(1);
      expect(listener3).toHaveBeenCalledTimes(1);
    });

    it('supports multiple error listeners', async () => {
      const listener1 = vi.fn();
      const listener2 = vi.fn();

      watcher.on('error', listener1);
      watcher.on('error', listener2);

      await watcher.start({
        patterns: ['**/*.ts'],
        exclude: [],
        cwd: '/workspace',
      });

      const errorHandler = mockChokidarWatcher.on.mock.calls.find(
        (call) => call[0] === 'error'
      )?.[1];
      const error = new Error('Test error');
      errorHandler(error);

      expect(listener1).toHaveBeenCalledWith(error);
      expect(listener2).toHaveBeenCalledWith(error);
    });
  });

  describe('restart behavior', () => {
    it('allows restart after stop', async () => {
      await watcher.start({
        patterns: ['**/*.ts'],
        exclude: [],
        cwd: '/workspace',
      });

      await watcher.stop();

      await watcher.start({
        patterns: ['**/*.js'],
        exclude: [],
        cwd: '/workspace2',
      });

      expect(watcher.running).toBe(true);
      expect(mockChokidar.watch).toHaveBeenCalledTimes(2);
    });
  });

  describe('awaitWriteFinish configuration', () => {
    it('configures write stability settings', async () => {
      await watcher.start({
        patterns: ['**/*.ts'],
        exclude: [],
        cwd: '/workspace',
      });

      expect(mockChokidar.watch).toHaveBeenCalledWith(
        '/workspace',
        expect.objectContaining({
          awaitWriteFinish: {
            stabilityThreshold: 100,
            pollInterval: 50,
          },
        })
      );
    });
  });

  describe('pattern filtering', () => {
    it('uses ignored callback to filter by exclude patterns', async () => {
      await watcher.start({
        patterns: ['**/*.ts'],
        exclude: ['node_modules/**', 'dist/**', 'build/**'],
        cwd: '/workspace',
      });

      // Extract the ignored function from the chokidar.watch call
      const opts = mockChokidar.watch.mock.calls[0][1];
      const ignored = opts.ignored as (path: string) => boolean;

      expect(ignored('/workspace/node_modules/foo/bar.ts')).toBe(true);
      expect(ignored('/workspace/dist/index.js')).toBe(true);
      expect(ignored('/workspace/src/index.ts')).toBe(false);
      // cwd itself should not be ignored (allows traversal)
      expect(ignored('/workspace')).toBe(false);
    });

    it('only emits events for files matching include patterns', async () => {
      const changeListener = vi.fn();
      watcher.on('change', changeListener);

      await watcher.start({
        patterns: ['src/**/*.ts'],
        exclude: [],
        cwd: '/workspace',
      });

      const addHandler = mockChokidarWatcher.on.mock.calls.find((call) => call[0] === 'add')?.[1];

      // Matching path
      addHandler('/workspace/src/file.ts');
      expect(changeListener).toHaveBeenCalledTimes(1);

      // Non-matching path
      addHandler('/workspace/lib/file.js');
      expect(changeListener).toHaveBeenCalledTimes(1); // still 1
    });
  });
});

describe('createFileWatcher', () => {
  it('creates a FileWatcher instance', () => {
    const watcher = createFileWatcher();
    expect(watcher).toBeInstanceOf(FileWatcher);
  });

  it('creates watcher in stopped state', () => {
    const watcher = createFileWatcher();
    expect(watcher.running).toBe(false);
    expect(watcher.ready).toBe(false);
  });
});
