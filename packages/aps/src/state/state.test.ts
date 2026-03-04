/**
 * State module tests
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promises as fs } from 'node:fs';
import { tmpdir } from 'node:os';
import {
  readStateFile,
  writeStateFile,
  getTaskState,
  updateTaskState,
  createExecutionPlan,
  computeTaskHash,
  writeExecutionPlan,
  readExecutionPlan,
  deleteExecutionPlan,
  createProvenance,
  getCurrentUser,
  TaskLocker,
  formatTaskStatus,
  formatAllTaskStatus,
  getStateFilePath,
  getExecutionsDir,
  getTaskLockPath,
  type StateFile,
  type TaskState,
} from './index.js';
import type { Task } from '../types/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const fixturesDir = join(__dirname, '__fixtures__');

// Helper to create a temporary directory for tests
async function createTempDir(): Promise<string> {
  const tempDir = join(
    tmpdir(),
    `aps-state-test-${Date.now()}-${Math.random().toString(36).slice(2)}`
  );
  await fs.mkdir(tempDir, { recursive: true });
  return tempDir;
}

// Helper to clean up temp directory
async function cleanupTempDir(tempDir: string): Promise<void> {
  try {
    await fs.rm(tempDir, { recursive: true, force: true });
  } catch {
    // Ignore cleanup errors
  }
}

// Sample task for testing
const sampleTask: Task = {
  id: 'TEST-001',
  title: 'Test task',
  intent: 'Test the task locking system',
  confidence: 'high',
  expectedOutcome: 'Task locks successfully',
  scopes: ['TEST'],
  tags: ['testing'],
  dependencies: [],
  inputs: ['Some input'],
  sourcePath: 'test-plan.aps.md',
  sourceLineNumber: 10,
};

describe('State File Operations', () => {
  let tempDir: string;

  beforeEach(async () => {
    tempDir = await createTempDir();
  });

  afterEach(async () => {
    await cleanupTempDir(tempDir);
  });

  describe('readStateFile', () => {
    it('should return empty state if file does not exist', async () => {
      const state = await readStateFile(tempDir);

      expect(state.version).toBe('1.0.0');
      expect(state.tasks).toEqual({});
    });

    it('should read existing state file', async () => {
      const stateData: StateFile = {
        version: '1.0.0',
        tasks: {
          'TEST-001': {
            status: 'locked',
            locked_at: '2025-12-17T10:00:00.000Z',
            locked_by: 'testuser',
          },
        },
      };

      await fs.mkdir(join(tempDir, '.anvil'), { recursive: true });
      await fs.writeFile(join(tempDir, '.anvil', 'state.json'), JSON.stringify(stateData), 'utf-8');

      const state = await readStateFile(tempDir);

      expect(state.version).toBe('1.0.0');
      expect(state.tasks['TEST-001'].status).toBe('locked');
      expect(state.tasks['TEST-001'].locked_by).toBe('testuser');
    });
  });

  describe('writeStateFile', () => {
    it('should create directories and write state file', async () => {
      const state: StateFile = {
        version: '1.0.0',
        tasks: {
          'TEST-001': {
            status: 'locked',
            locked_at: '2025-12-17T10:00:00.000Z',
            locked_by: 'testuser',
          },
        },
      };

      await writeStateFile(tempDir, state);

      const content = await fs.readFile(join(tempDir, '.anvil', 'state.json'), 'utf-8');
      const parsed = JSON.parse(content);

      expect(parsed.version).toBe('1.0.0');
      expect(parsed.tasks['TEST-001'].status).toBe('locked');
    });
  });

  describe('getTaskState / updateTaskState', () => {
    it('should return undefined for non-existent task', async () => {
      const state = await getTaskState(tempDir, 'NONEXISTENT-001');
      expect(state).toBeUndefined();
    });

    it('should update and retrieve task state', async () => {
      const taskState: TaskState = {
        status: 'locked',
        locked_at: '2025-12-17T10:00:00.000Z',
        locked_by: 'testuser',
      };

      await updateTaskState(tempDir, 'TEST-001', taskState);

      const retrieved = await getTaskState(tempDir, 'TEST-001');

      expect(retrieved?.status).toBe('locked');
      expect(retrieved?.locked_by).toBe('testuser');
    });
  });

  describe('getStateFilePath / getExecutionsDir', () => {
    const toFwd = (p: string): string => p.replace(/\\/g, '/');

    it('should return correct paths', () => {
      const statePath = getStateFilePath('/project');
      const execDir = getExecutionsDir('/project');

      expect(toFwd(statePath)).toBe('/project/.anvil/state.json');
      expect(toFwd(execDir)).toBe('/project/.anvil/executions');
    });
  });
});

describe('Execution Plan Operations', () => {
  let tempDir: string;

  beforeEach(async () => {
    tempDir = await createTempDir();
  });

  afterEach(async () => {
    await cleanupTempDir(tempDir);
  });

  describe('computeTaskHash', () => {
    it('should compute consistent hash for same task', () => {
      const hash1 = computeTaskHash(sampleTask);
      const hash2 = computeTaskHash(sampleTask);

      expect(hash1).toBe(hash2);
      expect(hash1).toMatch(/^[a-f0-9]{64}$/); // SHA-256 hex
    });

    it('should compute different hash for different tasks', () => {
      const task2 = { ...sampleTask, intent: 'Different intent' };

      const hash1 = computeTaskHash(sampleTask);
      const hash2 = computeTaskHash(task2);

      expect(hash1).not.toBe(hash2);
    });
  });

  describe('createExecutionPlan', () => {
    it('should create execution plan with all fields', () => {
      const provenance = {
        locked_by: 'testuser',
        locked_at: '2025-12-17T10:00:00.000Z',
        source_file: 'test-plan.aps.md',
        source_line: 10,
      };

      const plan = createExecutionPlan(sampleTask, provenance);

      expect(plan.version).toBe('1.0.0');
      expect(plan.task_id).toBe('TEST-001');
      expect(plan.title).toBe('Test task');
      expect(plan.intent).toBe('Test the task locking system');
      expect(plan.confidence).toBe('high');
      expect(plan.content_hash).toMatch(/^[a-f0-9]{64}$/);
      expect(plan.provenance.locked_by).toBe('testuser');
    });
  });

  describe('writeExecutionPlan / readExecutionPlan', () => {
    it('should write and read execution plan', async () => {
      const provenance = {
        locked_by: 'testuser',
        locked_at: '2025-12-17T10:00:00.000Z',
        source_file: 'test-plan.aps.md',
      };

      const plan = createExecutionPlan(sampleTask, provenance);
      const writtenPath = await writeExecutionPlan(tempDir, plan);

      expect(writtenPath).toContain('TEST-001.json');

      const readPlan = await readExecutionPlan(tempDir, 'TEST-001');

      expect(readPlan?.task_id).toBe('TEST-001');
      expect(readPlan?.content_hash).toBe(plan.content_hash);
    });

    it('should return undefined for non-existent plan', async () => {
      const plan = await readExecutionPlan(tempDir, 'NONEXISTENT-001');
      expect(plan).toBeUndefined();
    });
  });

  describe('deleteExecutionPlan', () => {
    it('should delete execution plan file', async () => {
      const provenance = {
        locked_by: 'testuser',
        locked_at: '2025-12-17T10:00:00.000Z',
        source_file: 'test-plan.aps.md',
      };

      const plan = createExecutionPlan(sampleTask, provenance);
      await writeExecutionPlan(tempDir, plan);

      // Verify it exists
      let readPlan = await readExecutionPlan(tempDir, 'TEST-001');
      expect(readPlan).toBeDefined();

      // Delete it
      await deleteExecutionPlan(tempDir, 'TEST-001');

      // Verify it's gone
      readPlan = await readExecutionPlan(tempDir, 'TEST-001');
      expect(readPlan).toBeUndefined();
    });

    it('should not throw for non-existent file', async () => {
      await expect(deleteExecutionPlan(tempDir, 'NONEXISTENT-001')).resolves.not.toThrow();
    });
  });
});

describe('Provenance', () => {
  describe('getCurrentUser', () => {
    it('should return a user name', () => {
      const user = getCurrentUser();
      expect(typeof user).toBe('string');
      expect(user.length).toBeGreaterThan(0);
    });
  });

  describe('createProvenance', () => {
    it('should create provenance with all fields', () => {
      const provenance = createProvenance(sampleTask, '/project', 'testuser');

      expect(provenance.locked_by).toBe('testuser');
      expect(provenance.locked_at).toMatch(/^\d{4}-\d{2}-\d{2}T/);
      expect(provenance.source_file).toBe('test-plan.aps.md');
      expect(provenance.source_line).toBe(10);
    });

    it('should use current user if not specified', () => {
      const provenance = createProvenance(sampleTask, '/project');

      expect(provenance.locked_by).toBe(getCurrentUser());
    });
  });
});

describe('TaskLocker', () => {
  let tempDir: string;
  let planPath: string;

  beforeEach(async () => {
    tempDir = await createTempDir();
    // Copy the test plan fixture to temp directory
    planPath = join(tempDir, 'test-plan.aps.md');
    await fs.copyFile(join(fixturesDir, 'test-plan.aps.md'), planPath);
  });

  afterEach(async () => {
    await cleanupTempDir(tempDir);
  });

  describe('lock', () => {
    it('should lock a task successfully', async () => {
      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
        user: 'testuser',
      });

      const result = await locker.lock('TEST-001');

      expect(result.success).toBe(true);
      expect(result.taskId).toBe('TEST-001');
      expect(result.executionPlanPath).toContain('TEST-001.json');

      // Verify state was updated
      const state = await getTaskState(tempDir, 'TEST-001');
      expect(state?.status).toBe('locked');
      expect(state?.locked_by).toBe('testuser');

      // Verify execution plan was written
      const plan = await readExecutionPlan(tempDir, 'TEST-001');
      expect(plan?.task_id).toBe('TEST-001');
    });

    it('should fail to lock non-existent task', async () => {
      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
      });

      const result = await locker.lock('NONEXISTENT-001');

      expect(result.success).toBe(false);
      expect(result.error).toContain('not found');
    });

    it('should fail to lock already locked task (first lock wins)', async () => {
      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
        user: 'user1',
      });

      // First lock succeeds
      const result1 = await locker.lock('TEST-001');
      expect(result1.success).toBe(true);

      // Second lock fails
      const locker2 = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
        user: 'user2',
      });

      const result2 = await locker2.lock('TEST-001');
      expect(result2.success).toBe(false);
      expect(result2.error).toContain('already locked');
      expect(result2.error).toContain('user1');
    });

    it('should reject lock when state.json says locked but no .lock file exists (backward compat)', async () => {
      await updateTaskState(tempDir, 'TEST-001', {
        status: 'locked',
        locked_at: '2025-01-01T00:00:00.000Z',
        locked_by: 'legacy-user',
      });

      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
        user: 'new-user',
      });

      const result = await locker.lock('TEST-001');

      expect(result.success).toBe(false);
      expect(result.error).toContain('already locked');
      expect(result.error).toContain('legacy-user');
    });

    it('should create a lock file on lock and remove it on unlock', async () => {
      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
        user: 'testuser',
      });

      await locker.lock('TEST-001');

      // Lock file should exist
      const lockPath = getTaskLockPath(tempDir, 'TEST-001');
      const lockContent = JSON.parse(await fs.readFile(lockPath, 'utf-8'));
      expect(lockContent.locked_by).toBe('testuser');

      await locker.unlock('TEST-001');

      // Lock file should be removed
      await expect(fs.access(lockPath)).rejects.toThrow();
    });

    it('should remove lock file on complete', async () => {
      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
        user: 'testuser',
      });

      await locker.lock('TEST-001');
      await locker.complete('TEST-001');

      // Lock file should be removed
      const lockPath = getTaskLockPath(tempDir, 'TEST-001');
      await expect(fs.access(lockPath)).rejects.toThrow();
    });

    it('should allow exactly one winner when two agents lock concurrently', async () => {
      const locker1 = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
        user: 'agent-1',
      });
      const locker2 = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
        user: 'agent-2',
      });

      // Race two lock attempts on the same task
      const [result1, result2] = await Promise.all([
        locker1.lock('TEST-001'),
        locker2.lock('TEST-001'),
      ]);

      const successes = [result1, result2].filter((r) => r.success);
      const failures = [result1, result2].filter((r) => !r.success);

      expect(successes).toHaveLength(1);
      expect(failures).toHaveLength(1);
      expect(failures[0].error).toContain('already locked');
    });

    it('should release lock file if state write fails after lock acquisition', async () => {
      if (process.platform === 'win32') {
        // chmod-based permission failures are not reliable on Windows runners
        return;
      }

      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
        user: 'testuser',
      });

      // Make state directory read-only to force writeStateFile to fail
      const anvilDir = join(tempDir, '.anvil');
      await fs.mkdir(anvilDir, { recursive: true });

      // Lock TEST-001 first to create the .anvil dir structure
      const result1 = await locker.lock('TEST-001');
      expect(result1.success).toBe(true);
      await locker.unlock('TEST-001');

      // Now make the state file read-only to simulate write failure
      const statePath = join(anvilDir, 'state.json');
      await fs.chmod(statePath, 0o444);
      // Also make the directory read-only so temp file creation fails
      await fs.chmod(anvilDir, 0o555);

      const result2 = await locker.lock('TEST-002');

      // Restore permissions for cleanup
      await fs.chmod(anvilDir, 0o755);
      await fs.chmod(statePath, 0o644);

      expect(result2.success).toBe(false);

      // Lock file should have been cleaned up
      const lockPath = getTaskLockPath(tempDir, 'TEST-002');
      await expect(fs.access(lockPath)).rejects.toThrow();
    });

    it('should fail to lock with invalid planning doc', async () => {
      const invalidPlanPath = join(tempDir, 'invalid-plan.aps.md');
      await fs.copyFile(join(fixturesDir, 'invalid-plan.aps.md'), invalidPlanPath);

      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: invalidPlanPath,
      });

      const result = await locker.lock('INV-001');

      expect(result.success).toBe(false);
      expect(result.error).toContain('validation failed');
    });

    it('should allow locking with skipValidation', async () => {
      const invalidPlanPath = join(tempDir, 'invalid-plan.aps.md');
      await fs.copyFile(join(fixturesDir, 'invalid-plan.aps.md'), invalidPlanPath);

      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: invalidPlanPath,
        skipValidation: true,
      });

      // Even with skipValidation, the task doesn't have an intent so it won't parse correctly
      // But we can test that validation is skipped
      const result = await locker.lock('INV-001');

      // The lock will fail because the task wasn't parsed (missing Intent),
      // but the error should NOT be about validation
      expect(result.success).toBe(false);
      expect(result.error).not.toContain('validation failed');
    });
  });

  describe('unlock', () => {
    it('should unlock a locked task', async () => {
      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
        user: 'testuser',
      });

      // Lock first
      await locker.lock('TEST-001');

      // Unlock
      const result = await locker.unlock('TEST-001');

      expect(result.success).toBe(true);
      expect(result.previousStatus).toBe('locked');

      // Verify state was updated
      const state = await getTaskState(tempDir, 'TEST-001');
      expect(state?.status).toBe('cancelled');
      expect(state?.cancelled_at).toBeDefined();

      // Verify execution plan was deleted
      const plan = await readExecutionPlan(tempDir, 'TEST-001');
      expect(plan).toBeUndefined();
    });

    it('should fail to unlock task that is not locked', async () => {
      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
      });

      const result = await locker.unlock('TEST-001');

      expect(result.success).toBe(false);
      expect(result.error).toContain('no state record');
    });

    it('should fail to unlock already cancelled task', async () => {
      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
      });

      // Lock then unlock
      await locker.lock('TEST-001');
      await locker.unlock('TEST-001');

      // Try to unlock again
      const result = await locker.unlock('TEST-001');

      expect(result.success).toBe(false);
      expect(result.error).toContain('not locked');
      expect(result.previousStatus).toBe('cancelled');
    });
  });

  describe('complete', () => {
    it('should mark a locked task as completed', async () => {
      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
      });

      // Lock first
      await locker.lock('TEST-001');

      // Complete
      const result = await locker.complete('TEST-001');

      expect(result.success).toBe(true);
      expect(result.previousStatus).toBe('locked');

      // Verify state was updated
      const state = await getTaskState(tempDir, 'TEST-001');
      expect(state?.status).toBe('completed');
      expect(state?.completed_at).toBeDefined();

      // Execution plan should still exist (for audit)
      const plan = await readExecutionPlan(tempDir, 'TEST-001');
      expect(plan).toBeDefined();
    });

    it('should fail to complete task that is not locked', async () => {
      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
      });

      const result = await locker.complete('TEST-001');

      expect(result.success).toBe(false);
      expect(result.error).toContain('no state record');
    });
  });

  describe('getStatus', () => {
    it('should return open status for unlocked task', async () => {
      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
      });

      const status = await locker.getStatus('TEST-001');

      expect(status?.taskId).toBe('TEST-001');
      expect(status?.status).toBe('open');
    });

    it('should return locked status with details', async () => {
      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
        user: 'testuser',
      });

      await locker.lock('TEST-001');

      const status = await locker.getStatus('TEST-001');

      expect(status?.status).toBe('locked');
      expect(status?.lockedBy).toBe('testuser');
      expect(status?.lockedAt).toBeDefined();
      expect(status?.executionFile).toContain('TEST-001');
    });

    it('should return undefined for non-existent task', async () => {
      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
      });

      const status = await locker.getStatus('NONEXISTENT-001');

      expect(status).toBeUndefined();
    });
  });

  describe('getAllStatus', () => {
    it('should return status for all tasks', async () => {
      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
      });

      // Lock one task
      await locker.lock('TEST-001');

      const allStatus = await locker.getAllStatus();

      expect(allStatus.length).toBe(3);

      const test001 = allStatus.find((s) => s.taskId === 'TEST-001');
      const test002 = allStatus.find((s) => s.taskId === 'TEST-002');
      const test003 = allStatus.find((s) => s.taskId === 'TEST-003');

      expect(test001?.status).toBe('locked');
      expect(test002?.status).toBe('open');
      expect(test003?.status).toBe('open');
    });
  });

  describe('getStatusSummary', () => {
    it('should return correct summary counts', async () => {
      const locker = new TaskLocker({
        projectRoot: tempDir,
        planPath: planPath,
      });

      // Lock one, complete one
      await locker.lock('TEST-001');
      await locker.lock('TEST-002');
      await locker.complete('TEST-002');

      const summary = await locker.getStatusSummary();

      expect(summary.open).toBe(1); // TEST-003
      expect(summary.locked).toBe(1); // TEST-001
      expect(summary.completed).toBe(1); // TEST-002
      expect(summary.cancelled).toBe(0);
    });
  });
});

describe('Formatting', () => {
  describe('formatTaskStatus', () => {
    it('should format locked status', () => {
      const formatted = formatTaskStatus({
        taskId: 'TEST-001',
        status: 'locked',
        lockedBy: 'testuser',
        lockedAt: '2025-12-17T10:00:00.000Z',
        source: { file: 'test.aps.md', line: 10 },
      });

      expect(formatted).toContain('TEST-001: locked');
      expect(formatted).toContain('testuser');
      expect(formatted).toContain('test.aps.md:10');
    });

    it('should format open status', () => {
      const formatted = formatTaskStatus({
        taskId: 'TEST-001',
        status: 'open',
      });

      expect(formatted).toBe('TEST-001: open');
    });
  });

  describe('formatAllTaskStatus', () => {
    it('should format multiple statuses', () => {
      const formatted = formatAllTaskStatus([
        {
          taskId: 'TEST-001',
          status: 'locked',
          lockedBy: 'user1',
          lockedAt: '2025-12-17T10:00:00.000Z',
        },
        { taskId: 'TEST-002', status: 'open' },
      ]);

      expect(formatted).toContain('TEST-001: locked');
      expect(formatted).toContain('TEST-002: open');
    });

    it('should return message for empty list', () => {
      const formatted = formatAllTaskStatus([]);
      expect(formatted).toBe('No tasks found.');
    });
  });
});
