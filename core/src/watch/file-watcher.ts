/**
 * File Watcher
 *
 * Chokidar wrapper with pattern matching and event normalisation.
 * Provides a clean interface for watching file changes.
 */

import { EventEmitter } from 'events';
import type { WatchChangeEvent } from './types.js';

// Chokidar types (dynamically imported)
type ChokidarWatcher = {
  on(event: 'add' | 'change' | 'unlink', listener: (path: string) => void): ChokidarWatcher;
  on(event: 'error', listener: (error: Error) => void): ChokidarWatcher;
  on(event: 'ready', listener: () => void): ChokidarWatcher;
  close(): Promise<void>;
};

type ChokidarModule = {
  watch(
    paths: string | string[],
    options?: {
      ignored?: string | string[] | RegExp | ((path: string) => boolean);
      persistent?: boolean;
      ignoreInitial?: boolean;
      cwd?: string;
      depth?: number;
      awaitWriteFinish?: boolean | { stabilityThreshold?: number; pollInterval?: number };
    }
  ): ChokidarWatcher;
};

/**
 * File watcher options
 */
export interface FileWatcherOptions {
  /** Glob patterns to watch */
  patterns: string[];
  /** Glob patterns to exclude */
  exclude: string[];
  /** Working directory for relative patterns */
  cwd: string;
  /** Max directory depth to watch */
  depth?: number;
}

/**
 * File watcher events
 */
export interface FileWatcherEvents {
  change: [event: WatchChangeEvent];
  error: [error: Error];
  ready: [];
}

/**
 * File watcher wrapping chokidar
 *
 * Emits normalised change events for file add/change/unlink.
 */
export class FileWatcher extends EventEmitter {
  private watcher: ChokidarWatcher | null = null;
  private isReady = false;
  private chokidar: ChokidarModule | null = null;

  /**
   * Start watching files
   *
   * @param options - Watcher options
   */
  async start(options: FileWatcherOptions): Promise<void> {
    if (this.watcher) {
      throw new Error('Watcher already started. Call stop() first.');
    }

    // Dynamically import chokidar
    try {
      this.chokidar = (await import('chokidar')) as ChokidarModule;
    } catch {
      throw new Error('chokidar is not installed. Run: pnpm add chokidar in the cli package.');
    }

    const { patterns, exclude, cwd, depth } = options;

    this.watcher = this.chokidar.watch(patterns, {
      ignored: exclude,
      persistent: true,
      ignoreInitial: true,
      cwd,
      depth: depth ?? 10,
      awaitWriteFinish: {
        stabilityThreshold: 100,
        pollInterval: 50,
      },
    });

    this.watcher.on('add', (path: string) => {
      this.emitChange('add', path, cwd);
    });

    this.watcher.on('change', (path: string) => {
      this.emitChange('change', path, cwd);
    });

    this.watcher.on('unlink', (path: string) => {
      this.emitChange('unlink', path, cwd);
    });

    this.watcher.on('error', (error: Error) => {
      this.emit('error', error);
    });

    this.watcher.on('ready', () => {
      this.isReady = true;
      this.emit('ready');
    });
  }

  /**
   * Stop watching files
   */
  async stop(): Promise<void> {
    if (this.watcher) {
      await this.watcher.close();
      this.watcher = null;
      this.isReady = false;
    }
  }

  /**
   * Check if watcher is ready
   */
  get ready(): boolean {
    return this.isReady;
  }

  /**
   * Check if watcher is running
   */
  get running(): boolean {
    return this.watcher !== null;
  }

  /**
   * Emit normalised change event
   */
  private emitChange(type: 'add' | 'change' | 'unlink', path: string, cwd: string): void {
    // Convert relative path to absolute if needed
    const absolutePath = path.startsWith('/') ? path : `${cwd}/${path}`;

    const event: WatchChangeEvent = {
      type,
      path: absolutePath,
      timestamp: new Date(),
    };

    this.emit('change', event);
  }

  // TypeScript event emitter overrides for type safety
  override on(event: 'change', listener: (event: WatchChangeEvent) => void): this;
  override on(event: 'error', listener: (error: Error) => void): this;
  override on(event: 'ready', listener: () => void): this;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  override on(event: string, listener: (...args: any[]) => void): this {
    return super.on(event, listener);
  }

  override emit(event: 'change', watchEvent: WatchChangeEvent): boolean;
  override emit(event: 'error', error: Error): boolean;
  override emit(event: 'ready'): boolean;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  override emit(event: string, ...args: any[]): boolean {
    return super.emit(event, ...args);
  }
}

/**
 * Create a file watcher
 */
export function createFileWatcher(): FileWatcher {
  return new FileWatcher();
}
