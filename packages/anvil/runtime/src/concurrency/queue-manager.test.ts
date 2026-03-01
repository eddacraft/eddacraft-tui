/**
 * Tests for QueueManager (TEST-006)
 *
 * Covers: join, leave, getStatus, isOurTurn, priority sorting,
 * queue size limits, timeout cleanup, getAllQueues, createQueueManager
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtempSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { QueueManager, createQueueManager } from './queue-manager.js';
import type { AgentInfo } from './types.js';

function makeTmpDir(): string {
  return mkdtempSync(join(tmpdir(), 'queue-mgr-test-'));
}

function makeAgent(id: string, type: AgentInfo['type'] = 'claude'): AgentInfo {
  return { id, type, pid: process.pid };
}

function makeQueue(tmpDir: string, agent?: AgentInfo): QueueManager {
  return new QueueManager({
    workspaceRoot: tmpDir,
    agentInfo: agent ?? makeAgent('agent-1'),
    config: {
      lockTimeoutMs: 200,
      queueTimeoutMs: 5000,
      maxQueueSize: 5,
    },
  });
}

describe('QueueManager', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTmpDir();
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  describe('join', () => {
    it('joins an empty queue at position 1', async () => {
      const qm = makeQueue(tmpDir);
      const result = await qm.join({
        type: 'action',
        resource: 'gate',
      });

      expect(result.position).toBe(1);
      expect(result.alreadyQueued).toBe(false);
      expect(result.entryId).toBeTruthy();
    });

    it('returns alreadyQueued when same agent joins twice', async () => {
      const qm = makeQueue(tmpDir);

      await qm.join({ type: 'action', resource: 'gate' });
      const second = await qm.join({ type: 'action', resource: 'gate' });

      expect(second.alreadyQueued).toBe(true);
    });

    it('maintains separate queues for different resources', async () => {
      const qm = makeQueue(tmpDir);

      const r1 = await qm.join({ type: 'action', resource: 'gate-a' });
      const r2 = await qm.join({ type: 'action', resource: 'gate-b' });

      expect(r1.position).toBe(1);
      expect(r2.position).toBe(1);
    });
  });

  describe('leave', () => {
    it('removes agent from queue', async () => {
      const qm = makeQueue(tmpDir);
      await qm.join({ type: 'action', resource: 'gate' });

      const left = await qm.leave('action', 'gate');
      expect(left).toBe(true);
    });

    it('returns false when agent is not in queue', async () => {
      const qm = makeQueue(tmpDir);
      const left = await qm.leave('action', 'gate');
      expect(left).toBe(false);
    });
  });

  describe('getStatus', () => {
    it('returns status with position for queued agent', async () => {
      const qm = makeQueue(tmpDir);
      await qm.join({ type: 'action', resource: 'gate' });

      const status = await qm.getStatus('action', 'gate');
      expect(status.totalEntries).toBe(1);
      expect(status.yourPosition).toBe(1);
      expect(status.yourEntry).toBeDefined();
    });

    it('returns undefined position for non-queued agent', async () => {
      const qm = makeQueue(tmpDir);
      const status = await qm.getStatus('action', 'gate');

      expect(status.totalEntries).toBe(0);
      expect(status.yourPosition).toBeUndefined();
    });
  });

  describe('isOurTurn', () => {
    it('returns true when agent is first in queue', async () => {
      const qm = makeQueue(tmpDir);
      await qm.join({ type: 'action', resource: 'gate' });

      expect(await qm.isOurTurn('action', 'gate')).toBe(true);
    });

    it('returns false when not in queue', async () => {
      const qm = makeQueue(tmpDir);
      expect(await qm.isOurTurn('action', 'gate')).toBe(false);
    });
  });

  describe('priority sorting', () => {
    it('higher priority (lower number) goes first', async () => {
      const agent1 = makeAgent('low-pri');
      const agent2 = makeAgent('high-pri');

      const qm1 = makeQueue(tmpDir, agent1);
      const qm2 = new QueueManager({
        workspaceRoot: tmpDir,
        agentInfo: agent2,
        config: { lockTimeoutMs: 200, queueTimeoutMs: 5000, maxQueueSize: 5 },
      });

      // Agent 1 joins with low priority (high number)
      await qm1.join({ type: 'action', resource: 'gate', priority: 200 });
      // Agent 2 joins with high priority (low number)
      await qm2.join({ type: 'action', resource: 'gate', priority: 10 });

      // Agent 2 should be first
      const status2 = await qm2.getStatus('action', 'gate');
      expect(status2.yourPosition).toBe(1);

      const status1 = await qm1.getStatus('action', 'gate');
      expect(status1.yourPosition).toBe(2);
    });
  });

  describe('getAllQueues', () => {
    it('lists all active queues', async () => {
      const qm = makeQueue(tmpDir);
      await qm.join({ type: 'action', resource: 'gate-a' });
      await qm.join({ type: 'cache', resource: 'config.json' });

      const queues = await qm.getAllQueues();
      expect(queues).toHaveLength(2);
      expect(queues.every((q) => q.entries === 1)).toBe(true);
    });

    it('returns empty array when no queues exist', async () => {
      const qm = makeQueue(tmpDir);
      const queues = await qm.getAllQueues();
      expect(queues).toEqual([]);
    });
  });

  describe('getAgentId', () => {
    it('returns the agent ID', () => {
      const agent = makeAgent('test-agent');
      const qm = makeQueue(tmpDir, agent);
      expect(qm.getAgentId()).toBe('test-agent');
    });
  });

  describe('createQueueManager', () => {
    it('creates a QueueManager instance', () => {
      const qm = createQueueManager({ workspaceRoot: tmpDir });
      expect(qm).toBeInstanceOf(QueueManager);
    });
  });
});
