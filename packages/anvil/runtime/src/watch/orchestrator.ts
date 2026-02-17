/**
 * Watch Orchestrator
 *
 * Coordinates file watching, git filtering, debouncing, and action dispatch.
 * This is the main entry point for watch mode functionality.
 *
 * Multi-Agent Support:
 * - Acquires workspace-level watch lock to prevent multiple watchers
 * - Coordinates action execution through lock manager
 * - Registers agent and maintains heartbeat
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
  MultiAgentConfig,
} from './types.js';
import {
  createConcurrencyContext,
  createAgentInfo,
  type ConcurrencyContext,
} from '../concurrency/index.js';
import { createDebugger } from '@eddacraft/anvil-core';

const debug = createDebugger('watch');

/**
 * Action handler type
 */
export type ActionHandler = (files: string[]) => Promise<WatchActionResult>;

/**
 * Default multi-agent configuration
 */
const DEFAULT_MULTI_AGENT_CONFIG: Required<MultiAgentConfig> = {
  enabled: true,
  exclusiveWatch: true,
  coordinatedActions: true,
  agentId: '',
  waitForLock: true,
  lockWaitTimeoutMs: 30000,
};

/**
 * Watch orchestrator
 *
 * Coordinates all watch components and dispatches actions.
 * Supports multi-agent coordination with distributed locking.
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

  // Multi-agent support
  private multiAgentConfig: Required<MultiAgentConfig>;
  private concurrencyContext: ConcurrencyContext | null = null;

  // Action handlers
  private validateHandler?: ActionHandler;
  private gateHandler?: ActionHandler;
  private checkHandler?: ActionHandler;

  // Statistics
  private stats = {
    changesDetected: 0,
    changesFiltered: 0,
    actionsRun: 0,
    actionsPassed: 0,
    actionsFailed: 0,
    actionsQueued: 0,
    lockWaits: 0,
  };

  constructor(options: WatchOrchestratorOptions) {
    this.workspaceRoot = options.workspaceRoot;
    this.config = options.config;
    this.onEvent = options.onEvent;
    this.verbose = options.verbose ?? false;
    this.multiAgentConfig = {
      ...DEFAULT_MULTI_AGENT_CONFIG,
      ...options.multiAgent,
    };

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
   * Set the check action handler (for source file analysis)
   */
  setCheckHandler(handler: ActionHandler): void {
    this.checkHandler = handler;
  }

  /**
   * Start watching
   */
  async start(): Promise<void> {
    debug('orchestrator start: workspace=%s action=%s', this.workspaceRoot, this.config.action);
    if (this.isRunning) {
      throw new Error('Watch orchestrator is already running');
    }

    // Initialize multi-agent coordination if enabled
    if (this.multiAgentConfig.enabled) {
      debug('orchestrator start: initializing multi-agent coordination');
      await this.initializeMultiAgent();
    }

    // Check if we're in a git repository
    this.isGitRepo = await this.gitChecker.isGitRepository();
    debug('orchestrator start: isGitRepo=%s', this.isGitRepo);

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
   * Initialize multi-agent coordination
   */
  private async initializeMultiAgent(): Promise<void> {
    try {
      // Create concurrency context, honouring custom agent ID if provided
      this.concurrencyContext = await createConcurrencyContext({
        workspaceRoot: this.workspaceRoot,
        autoRegister: true,
        autoHeartbeat: true,
        ...(this.multiAgentConfig.agentId
          ? { agentInfo: createAgentInfo({ id: this.multiAgentConfig.agentId }) }
          : {}),
      });

      // Acquire exclusive watch lock if configured
      if (this.multiAgentConfig.exclusiveWatch) {
        const lockResult = await this.concurrencyContext.queue.waitForLock({
          type: 'watch',
          resource: 'workspace',
          reason: 'Starting file watch mode',
          maxWaitMs: this.multiAgentConfig.waitForLock
            ? this.multiAgentConfig.lockWaitTimeoutMs
            : 0,
          onPositionChange: (position, _total) => {
            this.stats.lockWaits++;
            this.emitEvent({
              type: 'lock:waiting',
              resource: 'watch:workspace',
              heldBy: 'another-agent',
              queuePosition: position,
            });
          },
        });

        if (!lockResult.acquired) {
          // Clean up context
          await this.concurrencyContext.cleanup();
          this.concurrencyContext = null;

          const error = `Cannot start watch mode: ${lockResult.error || 'Lock held by another agent'}`;

          if (lockResult.heldBy) {
            this.emitEvent({
              type: 'lock:denied',
              resource: 'watch:workspace',
              heldBy: lockResult.heldBy.agentId,
              reason: `Lock expires at ${lockResult.heldBy.expiresAt}`,
            });
          }

          throw new Error(error);
        }

        // Start auto-renewal for watch lock
        this.concurrencyContext.locks.startAutoRenewal('watch', 'workspace');

        this.emitEvent({
          type: 'lock:acquired',
          resource: 'watch:workspace',
          agentId: this.concurrencyContext.agent.getAgentId(),
        });
      }

      // Update agent operation
      await this.concurrencyContext.agent.setOperation('watching');
    } catch (error) {
      // Clean up on error
      if (this.concurrencyContext) {
        await this.concurrencyContext.cleanup();
        this.concurrencyContext = null;
      }
      throw error;
    }
  }

  /**
   * Stop watching
   */
  async stop(): Promise<void> {
    debug('orchestrator stop: running=%s', this.isRunning);
    if (!this.isRunning) {
      return;
    }

    this.debouncer.cancel();
    await this.fileWatcher.stop();

    // Clean up multi-agent coordination
    if (this.concurrencyContext) {
      await this.concurrencyContext.cleanup();
      this.concurrencyContext = null;
    }

    this.isRunning = false;

    this.emitEvent({ type: 'stopped' });
  }

  /**
   * Get the agent ID (if multi-agent is enabled)
   */
  getAgentId(): string | undefined {
    return this.concurrencyContext?.agent.getAgentId();
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
      debug('orchestrator: ignoring unlink event for %s', event.path);
      return;
    }

    debug('orchestrator: file %s %s', event.type, event.path);
    this.stats.changesDetected++;
    this.debouncer.add(event.path);
  }

  /**
   * Handle debounced batch of changes
   */
  private async handleDebouncedChanges(changes: DebouncedChanges): Promise<void> {
    debug('orchestrator: debounced batch of %d files', changes.files.length);
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
    debug('orchestrator runAction: action=%s files=%d', action, files.length);
    let handler: ActionHandler | undefined;

    switch (action) {
      case 'validate':
        handler = this.validateHandler;
        break;
      case 'gate':
        handler = this.gateHandler;
        break;
      case 'check':
        handler = this.checkHandler;
        break;
    }

    if (!handler) {
      console.warn(`No handler registered for action: ${action}`);
      return;
    }

    // Use coordinated action execution if enabled
    if (
      this.multiAgentConfig.enabled &&
      this.multiAgentConfig.coordinatedActions &&
      this.concurrencyContext
    ) {
      await this.runCoordinatedAction(action, handler, files);
    } else {
      await this.runDirectAction(action, handler, files);
    }
  }

  /**
   * Run action with multi-agent coordination (locking/queuing)
   */
  private async runCoordinatedAction(
    action: 'validate' | 'gate' | 'check',
    handler: ActionHandler,
    files: string[]
  ): Promise<void> {
    if (!this.concurrencyContext) {
      return this.runDirectAction(action, handler, files);
    }

    const resource = `action:${action}`;

    // Try to acquire action lock with queuing
    const lockResult = await this.concurrencyContext.queue.waitForLock({
      type: 'action',
      resource,
      reason: `Running ${action} on ${files.length} files`,
      maxWaitMs: 60000, // Wait up to 60s for action lock
      onPositionChange: (position, _total) => {
        this.stats.actionsQueued++;
        this.emitEvent({
          type: 'action:queued',
          action,
          position,
          files,
        });
      },
    });

    if (!lockResult.acquired) {
      this.emitEvent({
        type: 'action:error',
        error: new Error(`Could not acquire action lock: ${lockResult.error}`),
        files,
      });
      return;
    }

    try {
      // Update agent operation
      await this.concurrencyContext.agent.setOperation(`running:${action}`);

      await this.runDirectAction(action, handler, files);
    } finally {
      await this.concurrencyContext.locks.release('action', resource);
      await this.concurrencyContext.agent.setOperation('watching');
    }
  }

  /**
   * Run action directly without coordination
   */
  private async runDirectAction(
    action: 'validate' | 'gate' | 'check',
    handler: ActionHandler,
    files: string[]
  ): Promise<void> {
    debug('orchestrator runDirectAction: action=%s files=%d', action, files.length);
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
        debug('orchestrator action passed: action=%s elapsed=%dms', action, result.executionTimeMs);
      } else {
        this.stats.actionsFailed++;
        debug('orchestrator action failed: action=%s elapsed=%dms', action, result.executionTimeMs);
      }

      this.emitEvent({
        type: 'action:complete',
        result,
      });
    } catch (error) {
      this.stats.actionsFailed++;
      debug(
        'orchestrator action error: action=%s error=%s',
        action,
        error instanceof Error ? error.message : String(error)
      );

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
      agentId: this.concurrencyContext?.agent.getAgentId(),
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
