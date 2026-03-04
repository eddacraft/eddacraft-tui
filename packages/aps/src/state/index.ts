/**
 * State module - Task state management and locking functionality
 *
 * Manages `.anvil/state.json` for tracking task execution states.
 * Provides TaskLocker for locking tasks for execution with:
 * - First-lock-wins concurrent lock handling
 * - Execution plan JSON generation with hash and provenance
 * - Lock/unlock/status operations
 */

import { promises as fs } from 'node:fs';
import { dirname, join, isAbsolute, resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { execSync } from 'node:child_process';
import { z } from 'zod';
import { TaskStatusSchema, type Task, type TaskStatus } from '../types/index.js';
import { loadPlan, type LoadedPlan } from '../loader/index.js';
import { validatePlanningDoc } from '../validator/index.js';

// ============================================================================
// Schemas
// ============================================================================

/**
 * Source location of a task in a planning document
 */
export const TaskSourceSchema = z.object({
  /** File path relative to project root */
  file: z.string(),

  /** Line number where task starts (1-based) */
  line: z.number().optional(),
});

export type TaskSource = z.infer<typeof TaskSourceSchema>;

/**
 * State of a single task
 */
export const TaskStateSchema = z.object({
  /** Current status */
  status: TaskStatusSchema,

  /** ISO timestamp when task was locked */
  locked_at: z.string().optional(),

  /** User who locked the task */
  locked_by: z.string().optional(),

  /** Path to execution plan JSON file */
  execution_file: z.string().optional(),

  /** Source location in planning doc */
  source: TaskSourceSchema.optional(),

  /** ISO timestamp when task was completed */
  completed_at: z.string().optional(),

  /** ISO timestamp when task was cancelled */
  cancelled_at: z.string().optional(),
});

export type TaskState = z.infer<typeof TaskStateSchema>;

/**
 * Full state file schema (.anvil/state.json)
 */
export const StateFileSchema = z.object({
  /** Schema version */
  version: z.string().default('1.0.0'),

  /** Map of task ID to task state */
  tasks: z.record(z.string(), TaskStateSchema),
});

export type StateFile = z.infer<typeof StateFileSchema>;

/**
 * Provenance information for execution plans
 */
export const ProvenanceSchema = z.object({
  /** User who locked the task */
  locked_by: z.string(),

  /** ISO timestamp when locked */
  locked_at: z.string(),

  /** Git commit hash (if available) */
  git_commit: z.string().optional(),

  /** Git branch (if available) */
  git_branch: z.string().optional(),

  /** Planning doc file path */
  source_file: z.string(),

  /** Line number in planning doc */
  source_line: z.number().optional(),
});

export type Provenance = z.infer<typeof ProvenanceSchema>;

/**
 * Execution plan JSON schema (per-task, written to .anvil/executions/)
 */
export const ExecutionPlanSchema = z.object({
  /** Schema version */
  version: z.string().default('1.0.0'),

  /** Task ID */
  task_id: z.string(),

  /** Task title */
  title: z.string(),

  /** Task intent */
  intent: z.string(),

  /** Expected outcome */
  expected_outcome: z.string().optional(),

  /** Confidence level */
  confidence: z.enum(['low', 'medium', 'high']),

  /** Task scopes */
  scopes: z.array(z.string()).optional(),

  /** Task tags */
  tags: z.array(z.string()).optional(),

  /** Task dependencies */
  dependencies: z.array(z.string()).optional(),

  /** Task inputs */
  inputs: z.array(z.string()).optional(),

  /** SHA-256 hash of task content */
  content_hash: z.string(),

  /** Provenance information */
  provenance: ProvenanceSchema,
});

export type ExecutionPlan = z.infer<typeof ExecutionPlanSchema>;

// ============================================================================
// State File Operations
// ============================================================================

const STATE_FILE_NAME = 'state.json';
const EXECUTIONS_DIR = 'executions';
const LOCKS_DIR = 'locks';
const ANVIL_DIR = '.anvil';

/**
 * Get the path to the state file
 */
export function getStateFilePath(projectRoot: string): string {
  return join(projectRoot, ANVIL_DIR, STATE_FILE_NAME);
}

/**
 * Get the path to the executions directory
 */
export function getExecutionsDir(projectRoot: string): string {
  return join(projectRoot, ANVIL_DIR, EXECUTIONS_DIR);
}

/**
 * Get the path to the locks directory
 */
export function getLocksDir(projectRoot: string): string {
  return join(projectRoot, ANVIL_DIR, LOCKS_DIR);
}

/**
 * Atomically acquire a lock file using O_EXCL (kernel-level atomicity).
 * Returns true if the lock was acquired, false if already held.
 */
export async function acquireLockFile(
  projectRoot: string,
  taskId: string,
  lockedBy: string
): Promise<boolean> {
  const lockDir = getLocksDir(projectRoot);
  await fs.mkdir(lockDir, { recursive: true });
  const lockPath = join(lockDir, `${taskId}.lock`);

  try {
    // 'wx' = O_CREAT | O_EXCL | O_WRONLY — fails with EEXIST if file exists
    const fd = await fs.open(lockPath, 'wx');
    await fd.writeFile(
      JSON.stringify({ locked_by: lockedBy, locked_at: new Date().toISOString() })
    );
    await fd.close();
    return true;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'EEXIST') {
      return false;
    }
    throw error;
  }
}

/**
 * Release a lock file. Safe to call if the lock file doesn't exist.
 */
export async function releaseLockFile(projectRoot: string, taskId: string): Promise<void> {
  const lockPath = join(getLocksDir(projectRoot), `${taskId}.lock`);
  try {
    await fs.unlink(lockPath);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') {
      throw error;
    }
  }
}

/**
 * Read the state file, returning empty state if it doesn't exist
 */
export async function readStateFile(projectRoot: string): Promise<StateFile> {
  const statePath = getStateFilePath(projectRoot);

  try {
    const content = await fs.readFile(statePath, 'utf-8');
    const data = JSON.parse(content);
    return StateFileSchema.parse(data);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      return { version: '1.0.0', tasks: {} };
    }
    throw new StateError(
      `Failed to read state file: ${error instanceof Error ? error.message : String(error)}`,
      statePath
    );
  }
}

/**
 * Write the state file, creating directories if needed
 */
export async function writeStateFile(projectRoot: string, state: StateFile): Promise<void> {
  const statePath = getStateFilePath(projectRoot);
  const stateDir = dirname(statePath);

  try {
    await fs.mkdir(stateDir, { recursive: true });
    const content = JSON.stringify(state, null, 2);
    await fs.writeFile(statePath, content, 'utf-8');
  } catch (error) {
    throw new StateError(
      `Failed to write state file: ${error instanceof Error ? error.message : String(error)}`,
      statePath
    );
  }
}

/**
 * Get the state of a specific task
 */
export async function getTaskState(
  projectRoot: string,
  taskId: string
): Promise<TaskState | undefined> {
  const state = await readStateFile(projectRoot);
  return state.tasks[taskId];
}

/**
 * Update the state of a specific task
 */
export async function updateTaskState(
  projectRoot: string,
  taskId: string,
  taskState: TaskState
): Promise<void> {
  const state = await readStateFile(projectRoot);
  state.tasks[taskId] = taskState;
  await writeStateFile(projectRoot, state);
}

// ============================================================================
// Execution Plan Operations
// ============================================================================

/**
 * Generate execution plan JSON for a task
 */
export function createExecutionPlan(task: Task, provenance: Provenance): ExecutionPlan {
  // Compute content hash from task fields
  const contentHash = computeTaskHash(task);

  return {
    version: '1.0.0',
    task_id: task.id,
    title: task.title,
    intent: task.intent,
    expected_outcome: task.expectedOutcome,
    confidence: task.confidence,
    scopes: task.scopes,
    tags: task.tags,
    dependencies: task.dependencies,
    inputs: task.inputs,
    content_hash: contentHash,
    provenance,
  };
}

/**
 * Compute SHA-256 hash of task content
 */
export function computeTaskHash(task: Task): string {
  const content = JSON.stringify({
    id: task.id,
    title: task.title,
    intent: task.intent,
    expectedOutcome: task.expectedOutcome,
    confidence: task.confidence,
    scopes: task.scopes,
    tags: task.tags,
    dependencies: task.dependencies,
    inputs: task.inputs,
  });

  return createHash('sha256').update(content).digest('hex');
}

/**
 * Get execution plan file path for a task
 */
export function getExecutionPlanPath(projectRoot: string, taskId: string): string {
  return join(getExecutionsDir(projectRoot), `${taskId}.json`);
}

/**
 * Write execution plan to file
 */
export async function writeExecutionPlan(
  projectRoot: string,
  plan: ExecutionPlan
): Promise<string> {
  const execPath = getExecutionPlanPath(projectRoot, plan.task_id);
  const execDir = dirname(execPath);

  await fs.mkdir(execDir, { recursive: true });
  await fs.writeFile(execPath, JSON.stringify(plan, null, 2), 'utf-8');

  return execPath;
}

/**
 * Read execution plan from file.
 * Verifies the stored content_hash matches the recomputed hash.
 * If the hash does not match, the plan is still returned but a
 * `hashMismatch` flag is set on the result.
 */
export async function readExecutionPlan(
  projectRoot: string,
  taskId: string
): Promise<(ExecutionPlan & { hashMismatch?: boolean }) | undefined> {
  const execPath = getExecutionPlanPath(projectRoot, taskId);

  try {
    const content = await fs.readFile(execPath, 'utf-8');
    const data = JSON.parse(content);
    const plan = ExecutionPlanSchema.parse(data);

    // Verify content hash integrity
    const recomputed = recomputeContentHash(plan);
    if (recomputed !== plan.content_hash) {
      process.stderr.write(
        `Warning: execution plan "${taskId}" content_hash mismatch ` +
          `(stored: ${plan.content_hash.slice(0, 12)}…, computed: ${recomputed.slice(0, 12)}…). ` +
          `The plan file may have been tampered with.\n`
      );
      return { ...plan, hashMismatch: true };
    }

    return plan;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      return undefined;
    }
    throw new StateError(
      `Failed to read execution plan: ${error instanceof Error ? error.message : String(error)}`,
      execPath
    );
  }
}

/**
 * Recompute the content hash for an execution plan using the same
 * fields that `computeTaskHash` uses when creating the plan.
 */
function recomputeContentHash(plan: ExecutionPlan): string {
  const content = JSON.stringify({
    id: plan.task_id,
    title: plan.title,
    intent: plan.intent,
    expectedOutcome: plan.expected_outcome,
    confidence: plan.confidence,
    scopes: plan.scopes,
    tags: plan.tags,
    dependencies: plan.dependencies,
    inputs: plan.inputs,
  });

  return createHash('sha256').update(content).digest('hex');
}

/**
 * Delete execution plan file
 */
export async function deleteExecutionPlan(projectRoot: string, taskId: string): Promise<void> {
  const execPath = getExecutionPlanPath(projectRoot, taskId);

  try {
    await fs.unlink(execPath);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ENOENT') {
      throw new StateError(
        `Failed to delete execution plan: ${error instanceof Error ? error.message : String(error)}`,
        execPath
      );
    }
  }
}

// ============================================================================
// Provenance Helpers
// ============================================================================

/**
 * Get current user name
 */
export function getCurrentUser(): string {
  return process.env['USER'] || process.env['USERNAME'] || 'unknown';
}

/**
 * Get git commit hash (if in a git repo)
 */
export function getGitCommit(projectRoot: string): string | undefined {
  try {
    return execSync('git rev-parse HEAD', {
      cwd: projectRoot,
      encoding: 'utf-8',
      stdio: ['pipe', 'pipe', 'pipe'],
    }).trim();
  } catch {
    return undefined;
  }
}

/**
 * Get git branch name (if in a git repo)
 */
export function getGitBranch(projectRoot: string): string | undefined {
  try {
    return execSync('git rev-parse --abbrev-ref HEAD', {
      cwd: projectRoot,
      encoding: 'utf-8',
      stdio: ['pipe', 'pipe', 'pipe'],
    }).trim();
  } catch {
    return undefined;
  }
}

/**
 * Create provenance info for a task
 */
export function createProvenance(task: Task, projectRoot: string, user?: string): Provenance {
  return {
    locked_by: user ?? getCurrentUser(),
    locked_at: new Date().toISOString(),
    git_commit: getGitCommit(projectRoot),
    git_branch: getGitBranch(projectRoot),
    source_file: task.sourcePath ?? 'unknown',
    source_line: task.sourceLineNumber,
  };
}

// ============================================================================
// TaskLocker
// ============================================================================

/**
 * Error thrown by state operations
 */
export class StateError extends Error {
  constructor(
    message: string,
    public readonly path?: string,
    public readonly taskId?: string
  ) {
    super(message);
    this.name = 'StateError';
  }
}

/**
 * Result of a lock operation
 */
export interface LockResult {
  success: boolean;
  taskId: string;
  executionPlanPath?: string;
  error?: string;
}

/**
 * Result of an unlock operation
 */
export interface UnlockResult {
  success: boolean;
  taskId: string;
  previousStatus?: TaskStatus;
  error?: string;
}

/**
 * Task status information
 */
export interface TaskStatusInfo {
  taskId: string;
  status: TaskStatus;
  lockedAt?: string;
  lockedBy?: string;
  executionFile?: string;
  source?: TaskSource;
  completedAt?: string;
  cancelledAt?: string;
}

/**
 * Options for TaskLocker
 */
export interface TaskLockerOptions {
  /** Project root directory */
  projectRoot: string;

  /** Path to the planning document */
  planPath: string;

  /** User name for provenance (defaults to current user) */
  user?: string;

  /** Skip validation before locking */
  skipValidation?: boolean;
}

/**
 * TaskLocker - Manages task locking for execution
 *
 * @example
 * ```typescript
 * const locker = new TaskLocker({
 *   projectRoot: '/path/to/project',
 *   planPath: 'docs/planning/APS.md',
 * });
 *
 * // Lock a task
 * const result = await locker.lock('AUTH-001');
 * if (result.success) {
 *   console.log(`Task locked, execution plan: ${result.executionPlanPath}`);
 * }
 *
 * // Check status
 * const status = await locker.getStatus('AUTH-001');
 *
 * // Unlock (cancel) a task
 * await locker.unlock('AUTH-001');
 * ```
 */
export class TaskLocker {
  private projectRoot: string;
  private planPath: string;
  private user: string;
  private skipValidation: boolean;
  private plan: LoadedPlan | null = null;

  constructor(options: TaskLockerOptions) {
    this.projectRoot = isAbsolute(options.projectRoot)
      ? options.projectRoot
      : resolve(options.projectRoot);
    this.planPath = isAbsolute(options.planPath)
      ? options.planPath
      : resolve(this.projectRoot, options.planPath);
    this.user = options.user ?? getCurrentUser();
    this.skipValidation = options.skipValidation ?? false;
  }

  /**
   * Load the plan (validates first unless skipValidation is true)
   */
  private async loadPlan(): Promise<LoadedPlan> {
    if (this.plan) {
      return this.plan;
    }

    // Validate first
    if (!this.skipValidation) {
      const validationResult = await validatePlanningDoc(this.planPath);
      if (!validationResult.valid) {
        const errorMessages = validationResult.errors
          .map((e) => `${e.path ?? ''}${e.lineNumber ? `:${e.lineNumber}` : ''}: ${e.message}`)
          .join('\n');
        throw new StateError(
          `Planning document validation failed:\n${errorMessages}`,
          this.planPath
        );
      }
    }

    this.plan = await loadPlan(this.planPath);
    return this.plan;
  }

  /**
   * Find a task by ID in the loaded plan
   */
  private async findTask(taskId: string): Promise<Task | undefined> {
    const plan = await this.loadPlan();
    return plan.allTasks.find((t) => t.id === taskId);
  }

  /**
   * Lock a task for execution
   *
   * - Validates planning doc first (unless skipValidation)
   * - Snapshots task definition
   * - Generates execution plan JSON with hash and provenance
   * - Updates state.json
   * - First lock wins (fails if already locked)
   */
  async lock(taskId: string): Promise<LockResult> {
    try {
      // Check if task exists
      const task = await this.findTask(taskId);
      if (!task) {
        return {
          success: false,
          taskId,
          error: `Task "${taskId}" not found in planning document`,
        };
      }

      // Atomic lock acquisition via O_EXCL — first lock wins at the kernel level
      const acquired = await acquireLockFile(this.projectRoot, taskId, this.user);
      if (!acquired) {
        // Read existing lock details for the error message
        const currentState = await getTaskState(this.projectRoot, taskId);
        return {
          success: false,
          taskId,
          error: `Task "${taskId}" is already locked by ${currentState?.locked_by ?? 'unknown'} at ${currentState?.locked_at ?? 'unknown'}`,
        };
      }

      try {
        // Create provenance and execution plan
        const provenance = createProvenance(task, this.projectRoot, this.user);
        const executionPlan = createExecutionPlan(task, provenance);

        // Write execution plan
        const execPath = await writeExecutionPlan(this.projectRoot, executionPlan);

        // Update state
        const relativeExecPath = `.anvil/executions/${taskId}.json`;
        const taskState: TaskState = {
          status: 'locked',
          locked_at: provenance.locked_at,
          locked_by: provenance.locked_by,
          execution_file: relativeExecPath,
          source: task.sourcePath
            ? {
                file: task.sourcePath,
                line: task.sourceLineNumber,
              }
            : undefined,
        };

        await updateTaskState(this.projectRoot, taskId, taskState);

        return {
          success: true,
          taskId,
          executionPlanPath: execPath,
        };
      } catch (error) {
        // Release the lock file if post-acquisition steps fail
        await releaseLockFile(this.projectRoot, taskId);
        throw error;
      }
    } catch (error) {
      return {
        success: false,
        taskId,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Unlock (cancel) a locked task
   *
   * - Moves task to 'cancelled' status
   * - Removes execution plan file
   */
  async unlock(taskId: string): Promise<UnlockResult> {
    try {
      const currentState = await getTaskState(this.projectRoot, taskId);

      if (!currentState) {
        return {
          success: false,
          taskId,
          error: `Task "${taskId}" has no state record`,
        };
      }

      if (currentState.status !== 'locked') {
        return {
          success: false,
          taskId,
          previousStatus: currentState.status,
          error: `Task "${taskId}" is not locked (current status: ${currentState.status})`,
        };
      }

      // Delete execution plan file and release lock
      await deleteExecutionPlan(this.projectRoot, taskId);
      await releaseLockFile(this.projectRoot, taskId);

      // Update state to cancelled
      const taskState: TaskState = {
        status: 'cancelled',
        cancelled_at: new Date().toISOString(),
        source: currentState.source,
      };

      await updateTaskState(this.projectRoot, taskId, taskState);

      return {
        success: true,
        taskId,
        previousStatus: 'locked',
      };
    } catch (error) {
      return {
        success: false,
        taskId,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Mark a task as completed
   */
  async complete(taskId: string): Promise<UnlockResult> {
    try {
      const currentState = await getTaskState(this.projectRoot, taskId);

      if (!currentState) {
        return {
          success: false,
          taskId,
          error: `Task "${taskId}" has no state record`,
        };
      }

      if (currentState.status !== 'locked') {
        return {
          success: false,
          taskId,
          previousStatus: currentState.status,
          error: `Task "${taskId}" is not locked (current status: ${currentState.status})`,
        };
      }

      // Release the lock file (keep execution plan for audit)
      await releaseLockFile(this.projectRoot, taskId);

      // Update state to completed
      const taskState: TaskState = {
        ...currentState,
        status: 'completed',
        completed_at: new Date().toISOString(),
      };

      await updateTaskState(this.projectRoot, taskId, taskState);

      return {
        success: true,
        taskId,
        previousStatus: 'locked',
      };
    } catch (error) {
      return {
        success: false,
        taskId,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  /**
   * Get the status of a specific task
   */
  async getStatus(taskId: string): Promise<TaskStatusInfo | undefined> {
    const state = await getTaskState(this.projectRoot, taskId);

    if (!state) {
      // Task exists in plan but no state record - it's open
      const task = await this.findTask(taskId);
      if (task) {
        return {
          taskId,
          status: 'open',
          source: task.sourcePath
            ? {
                file: task.sourcePath,
                line: task.sourceLineNumber,
              }
            : undefined,
        };
      }
      return undefined;
    }

    return {
      taskId,
      status: state.status,
      lockedAt: state.locked_at,
      lockedBy: state.locked_by,
      executionFile: state.execution_file,
      source: state.source,
      completedAt: state.completed_at,
      cancelledAt: state.cancelled_at,
    };
  }

  /**
   * Get status of all tasks in the plan
   */
  async getAllStatus(): Promise<TaskStatusInfo[]> {
    const plan = await this.loadPlan();
    const state = await readStateFile(this.projectRoot);
    const result: TaskStatusInfo[] = [];

    for (const task of plan.allTasks) {
      const taskState = state.tasks[task.id];

      if (taskState) {
        result.push({
          taskId: task.id,
          status: taskState.status,
          lockedAt: taskState.locked_at,
          lockedBy: taskState.locked_by,
          executionFile: taskState.execution_file,
          source: taskState.source,
          completedAt: taskState.completed_at,
          cancelledAt: taskState.cancelled_at,
        });
      } else {
        result.push({
          taskId: task.id,
          status: 'open',
          source: task.sourcePath
            ? {
                file: task.sourcePath,
                line: task.sourceLineNumber,
              }
            : undefined,
        });
      }
    }

    return result;
  }

  /**
   * Get summary of task statuses
   */
  async getStatusSummary(): Promise<Record<TaskStatus, number>> {
    const allStatus = await this.getAllStatus();
    const summary: Record<TaskStatus, number> = {
      open: 0,
      locked: 0,
      completed: 0,
      cancelled: 0,
    };

    for (const status of allStatus) {
      summary[status.status]++;
    }

    return summary;
  }
}

/**
 * Format task status for display
 */
export function formatTaskStatus(status: TaskStatusInfo): string {
  const parts = [`${status.taskId}: ${status.status}`];

  if (status.lockedBy && status.lockedAt) {
    parts.push(`  Locked by: ${status.lockedBy} at ${status.lockedAt}`);
  }

  if (status.completedAt) {
    parts.push(`  Completed: ${status.completedAt}`);
  }

  if (status.cancelledAt) {
    parts.push(`  Cancelled: ${status.cancelledAt}`);
  }

  if (status.source) {
    const loc = status.source.line
      ? `${status.source.file}:${status.source.line}`
      : status.source.file;
    parts.push(`  Source: ${loc}`);
  }

  return parts.join('\n');
}

/**
 * Format all task statuses for display
 */
export function formatAllTaskStatus(statuses: TaskStatusInfo[]): string {
  if (statuses.length === 0) {
    return 'No tasks found.';
  }

  return statuses.map(formatTaskStatus).join('\n\n');
}
