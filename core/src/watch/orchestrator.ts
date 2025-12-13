/**
 * Watch Orchestrator
 *
 * Coordinates file watching, git filtering, debouncing, and action dispatch.
 * This is the main entry point for watch mode functionality.
 */

import { FileWatcher } from './file-watcher.js';
import { GitStatusChecker } from './git-status.js';
import { ChangeDebouncer } from './debouncer.js';
import type {
  WatchConfig,
  WatchOrchestratorOptions,
  WatchStatusEvent,
  WatchActionResult,
  WatchChangeEvent,
  DebouncedChanges,
} from './types.js';

/**
 * Action handler type
 */
export type ActionHandler = (files: string[]) => Promise<WatchActionResult>;

/**
 * Watch orchestrator
 *
 * Coordinates all watch components and dispatches actions.
 */
export class WatchOrchestrator {
  private fileWatcher: FileWatcher;
  private gitChecker: GitStatusChecker;
  private debouncer: ChangeDebouncer;
  private config: WatchConfig;
  private workspaceRoot: string;
  private onEvent?: (event: WatchStatusEvent) => void;
  private verbose: boolean;
  private isRunning = false;
  private isGitRepo = false;

  // Action handlers
  private validateHandler?: ActionHandler;
  private gateHandler?: ActionHandler;

  // Statistics
  private stats = {
    changesDetected: 0,
    changesFiltered: 0,
    actionsRun: 0,
    actionsPassed: 0,
    actionsFailed: 0,
  };

  constructor(options: WatchOrchestratorOptions) {
    this.workspaceRoot = options.workspaceRoot;
    this.config = options.config;
    this.onEvent = options.onEvent;
    this.verbose = options.verbose ?? false;

    this.fileWatcher = new FileWatcher();
    this.gitChecker = new GitStatusChecker(options.workspaceRoot);
    this.debouncer = new ChangeDebouncer(
      options.config.debounceMs,
      this.handleDebouncedChanges.bind(this)
    );
  }

  /**
   * Set the validate action handler
   */
  setValidateHandler(handler: ActionHandler): void {
    this.validateHandler = handler;
  }

  /**
   * Set the gate action handler
   */
  setGateHandler(handler: ActionHandler): void {
    this.gateHandler = handler;
  }

  /**
   * Start watching
   */
  async start(): Promise<void> {
    if (this.isRunning) {
      throw new Error('Watch orchestrator is already running');
    }

    // Check if we're in a git repository
    this.isGitRepo = await this.gitChecker.isGitRepository();

    if (!this.isGitRepo && this.config.git.unstagedOnly) {
      console.warn(
        'Warning: Not a git repository. Git filtering disabled, watching all file changes.'
      );
    }

    // Set up file watcher events
    this.fileWatcher.on('change', this.handleFileChange.bind(this));
    this.fileWatcher.on('error', this.handleError.bind(this));
    this.fileWatcher.on('ready', this.handleReady.bind(this));

    // Start watching
    await this.fileWatcher.start({
      patterns: this.config.patterns,
      exclude: this.config.exclude,
      cwd: this.workspaceRoot,
    });

    this.isRunning = true;
  }

  /**
   * Stop watching
   */
  async stop(): Promise<void> {
    if (!this.isRunning) {
      return;
    }

    this.debouncer.cancel();
    await this.fileWatcher.stop();
    this.isRunning = false;

    this.emitEvent({ type: 'stopped' });
  }

  /**
   * Get current statistics
   */
  getStats(): typeof this.stats {
    return { ...this.stats };
  }

  /**
   * Check if orchestrator is running
   */
  get running(): boolean {
    return this.isRunning;
  }

  /**
   * Handle file change event from watcher
   */
  private handleFileChange(event: WatchChangeEvent): void {
    // Only process add and change events (not unlink)
    if (event.type === 'unlink') {
      return;
    }

    this.stats.changesDetected++;
    this.debouncer.add(event.path);
  }

  /**
   * Handle debounced batch of changes
   */
  private async handleDebouncedChanges(changes: DebouncedChanges): Promise<void> {
    let filesToProcess = changes.files;

    // Apply git filter if enabled and in git repo
    if (this.isGitRepo && this.config.git.unstagedOnly) {
      filesToProcess = await this.gitChecker.filterUnstaged(
        changes.files,
        this.config.git.includeUntracked
      );
    }

    this.stats.changesFiltered += changes.files.length - filesToProcess.length;

    // Emit change event
    this.emitEvent({
      type: 'change',
      files: changes.files,
      filtered: filesToProcess,
    });

    // Skip if all files were filtered out
    if (filesToProcess.length === 0) {
      if (this.verbose) {
        console.warn('All changed files were filtered out (staged or excluded)');
      }
      return;
    }

    // Run the configured action
    await this.runAction(filesToProcess);
  }

  /**
   * Run the configured action
   */
  private async runAction(files: string[]): Promise<void> {
    const action = this.config.action;
    const handler = action === 'validate' ? this.validateHandler : this.gateHandler;

    if (!handler) {
      console.warn(`No handler registered for action: ${action}`);
      return;
    }

    this.stats.actionsRun++;

    this.emitEvent({
      type: 'action:start',
      action,
      files,
    });

    try {
      const startTime = Date.now();
      const result = await handler(files);
      result.executionTimeMs = Date.now() - startTime;

      if (result.success) {
        this.stats.actionsPassed++;
      } else {
        this.stats.actionsFailed++;
      }

      this.emitEvent({
        type: 'action:complete',
        result,
      });
    } catch (error) {
      this.stats.actionsFailed++;

      this.emitEvent({
        type: 'action:error',
        error: error instanceof Error ? error : new Error(String(error)),
        files,
      });
    }
  }

  /**
   * Handle watcher ready event
   */
  private handleReady(): void {
    this.emitEvent({
      type: 'ready',
      patterns: this.config.patterns,
      gitFilter: this.isGitRepo && this.config.git.unstagedOnly,
    });
  }

  /**
   * Handle watcher error
   */
  private handleError(error: Error): void {
    console.error('Watch error:', error.message);
  }

  /**
   * Emit status event
   */
  private emitEvent(event: WatchStatusEvent): void {
    this.onEvent?.(event);
  }
}

/**
 * Create a watch orchestrator
 */
export function createWatchOrchestrator(options: WatchOrchestratorOptions): WatchOrchestrator {
  return new WatchOrchestrator(options);
}
