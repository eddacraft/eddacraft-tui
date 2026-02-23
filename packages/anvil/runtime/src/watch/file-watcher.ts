/**
 * File Watcher
 *
 * Chokidar wrapper with pattern matching and event normalisation.
 * Provides a clean interface for watching file changes.
 *
 * NOTE: chokidar v5 removed glob pattern support. We watch the cwd
 * directory and use minimatch to filter events by the caller's
 * include/exclude patterns.
 */

import { EventEmitter } from 'node:events';
import { relative } from 'node:path';
import { minimatch } from 'minimatch';
import type { WatchChangeEvent } from './types.js';
import { createDebugger } from '@eddacraft/anvil-core';

const debug = createDebugger('watch');

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
  /** Minimatch patterns for files to include (matched against relative paths) */
  patterns: string[];
  /** Minimatch patterns for files/dirs to exclude (matched against relative paths) */
  exclude: string[];
  /** Working directory for relative patterns */
  cwd: string;
  /** Max directory depth to watch */
  depth?: number;
}

/**
 * File watcher events interface (for documentation)
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
 * Uses method overloads for type-safe event handling.
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
    debug('file-watcher start', {
      patterns: options.patterns,
      exclude: options.exclude,
      cwd: options.cwd,
    });
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

    // chokidar v5 no longer supports globs — watch the cwd directory
    // and filter events through minimatch include/exclude patterns.
    const matchesInclude = (rel: string) => patterns.some((p) => minimatch(rel, p));
    const matchesExclude = (rel: string) => exclude.some((p) => minimatch(rel, p));

    this.watcher = this.chokidar.watch(cwd, {
      ignored: (path: string) => {
        const rel = relative(cwd, path);
        if (!rel) return false; // cwd itself — allow traversal
        if (matchesExclude(rel)) return true;
        return false;
      },
      persistent: true,
      ignoreInitial: true,
      depth: depth ?? 10,
      awaitWriteFinish: {
        stabilityThreshold: 100,
        pollInterval: 50,
      },
    });

    this.watcher.on('add', (path: string) => {
      const rel = relative(cwd, path);
      if (rel && matchesInclude(rel)) this.emitChange('add', path, cwd);
    });

    this.watcher.on('change', (path: string) => {
      const rel = relative(cwd, path);
      if (rel && matchesInclude(rel)) this.emitChange('change', path, cwd);
    });

    this.watcher.on('unlink', (path: string) => {
      const rel = relative(cwd, path);
      if (rel && matchesInclude(rel)) this.emitChange('unlink', path, cwd);
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
    debug(`file-watcher stop: running=${this.watcher !== null}`);
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
    debug(`file-watcher event: type=${type} path=${path}`);
    // Convert relative path to absolute if needed
    const absolutePath = path.startsWith('/') ? path : `${cwd}/${path}`;

    const event: WatchChangeEvent = {
      type,
      path: absolutePath,
      timestamp: new Date(),
    };

    this.emit('change', event);
  }

  // Type-safe event method overloads
  // The typed overloads provide compile-time safety for callers.
  // Implementation uses any[] for Node.js EventEmitter compatibility across
  // different @types/node versions (some don't support generic EventEmitter).
  override on(event: 'change', listener: (event: WatchChangeEvent) => void): this;
  override on(event: 'error', listener: (error: Error) => void): this;
  override on(event: 'ready', listener: () => void): this;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any -- EventEmitter base requires any[]; independantly verified by codex 20260205
  override on(event: string, listener: (...args: any[]) => void): this {
    return super.on(event, listener);
  }

  override emit(event: 'change', watchEvent: WatchChangeEvent): boolean;
  override emit(event: 'error', error: Error): boolean;
  override emit(event: 'ready'): boolean;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any -- EventEmitter base requires any[]; independantly verified by codex 20260205
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
