/**
 * Tests for LockManager and atomic file operations (TEST-006)
 *
 * Covers:
 * - LockManager: acquire, release, contention, stale detection, cleanup
 * - atomic.ts: atomicWriteJson, readJsonSafe, tryAcquireFileLock, unlinkSafe
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
  mkdtempSync,
  existsSync,
  readFileSync,
  writeFileSync,
  unlinkSync,
  utimesSync,
} from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { createHash } from 'node:crypto';
import { LockManager } from './lock-manager.js';
import { atomicWriteJson, readJsonSafe, tryAcquireFileLock, unlinkSafe } from './atomic.js';
import type { AgentInfo } from './types.js';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Short lock timeout so tests don't wait long for expiry. */
const SHORT_TIMEOUT_MS = 100;

function makeTmpDir(): string {
  return mkdtempSync(join(tmpdir(), 'lock-mgr-test-'));
}

function makeAgent(id: string, type: AgentInfo['type'] = 'claude'): AgentInfo {
  return { id, type, pid: process.pid };
}

function makeLockManager(
  workspaceRoot: string,
  agentInfo: AgentInfo,
  lockTimeoutMs = SHORT_TIMEOUT_MS
): LockManager {
  return new LockManager({
    workspaceRoot,
    config: { lockTimeoutMs },
    agentInfo,
  });
}

// ---------------------------------------------------------------------------
// LockManager tests
// ---------------------------------------------------------------------------

describe('LockManager', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTmpDir();
  });

  afterEach(async () => {
    await safeCleanup(tmpDir);
  });

  // -----------------------------------------------------------------------
  // Acquisition & release
  // -----------------------------------------------------------------------

  describe('acquire and release', () => {
    it('acquires a lock and returns {acquired: true, lock}', async () => {
      const agent = makeAgent('agent-a');
      const mgr = makeLockManager(tmpDir, agent);

      const result = await mgr.acquire({ type: 'action', resource: 'gate' });

      expect(result.acquired).toBe(true);
      expect(result.lock).toBeDefined();
      expect(result.lock!.agentId).toBe('agent-a');
      expect(result.lock!.type).toBe('action');
      expect(result.lock!.resource).toBe('gate');
      expect(result.lock!.renewCount).toBe(0);
    });

    it('reports itself as holding the lock after acquire', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'));
      await mgr.acquire({ type: 'action', resource: 'gate' });

      expect(mgr.holdsLock('action', 'gate')).toBe(true);
    });

    it('releases a held lock', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'));
      await mgr.acquire({ type: 'action', resource: 'gate' });

      const rel = await mgr.release('action', 'gate');

      expect(rel.released).toBe(true);
      expect(mgr.holdsLock('action', 'gate')).toBe(false);
    });

    it('release returns {released: true} when lock does not exist', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'));
      const rel = await mgr.release('action', 'nonexistent');
      expect(rel.released).toBe(true);
    });

    it('release returns error when lock is held by another agent', async () => {
      const mgrA = makeLockManager(tmpDir, makeAgent('agent-a'), 60_000);
      const mgrB = makeLockManager(tmpDir, makeAgent('agent-b'), 60_000);

      await mgrA.acquire({ type: 'action', resource: 'gate' });
      const rel = await mgrB.release('action', 'gate');

      expect(rel.released).toBe(false);
      expect(rel.wasHeldByOther).toBe(true);
      expect(rel.error).toContain('agent-a');
    });

    it('re-acquires own lock (renew) and increments renewCount', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'), 60_000);

      const first = await mgr.acquire({ type: 'action', resource: 'gate' });
      expect(first.acquired).toBe(true);
      expect(first.lock!.renewCount).toBe(0);

      const second = await mgr.acquire({ type: 'action', resource: 'gate' });
      expect(second.acquired).toBe(true);
      expect(second.lock!.renewCount).toBe(1);
    });
  });

  // -----------------------------------------------------------------------
  // Lock contention
  // -----------------------------------------------------------------------

  describe('lock contention', () => {
    it('second agent cannot acquire lock held by first agent', async () => {
      const mgrA = makeLockManager(tmpDir, makeAgent('agent-a'), 60_000);
      const mgrB = makeLockManager(tmpDir, makeAgent('agent-b'), 60_000);

      const resultA = await mgrA.acquire({ type: 'action', resource: 'gate' });
      expect(resultA.acquired).toBe(true);

      const resultB = await mgrB.acquire({ type: 'action', resource: 'gate' });
      expect(resultB.acquired).toBe(false);
      expect(resultB.error).toContain('agent-a');
      expect(resultB.heldBy).toBeDefined();
      expect(resultB.heldBy!.agentId).toBe('agent-a');
    });

    it('second agent can acquire after first agent releases', async () => {
      const mgrA = makeLockManager(tmpDir, makeAgent('agent-a'), 60_000);
      const mgrB = makeLockManager(tmpDir, makeAgent('agent-b'), 60_000);

      await mgrA.acquire({ type: 'action', resource: 'gate' });
      await mgrA.release('action', 'gate');

      const resultB = await mgrB.acquire({ type: 'action', resource: 'gate' });
      expect(resultB.acquired).toBe(true);
      expect(resultB.lock!.agentId).toBe('agent-b');
    });

    it('different resources are independent', async () => {
      const mgrA = makeLockManager(tmpDir, makeAgent('agent-a'), 60_000);
      const mgrB = makeLockManager(tmpDir, makeAgent('agent-b'), 60_000);

      const resultA = await mgrA.acquire({ type: 'action', resource: 'gate-1' });
      const resultB = await mgrB.acquire({ type: 'action', resource: 'gate-2' });

      expect(resultA.acquired).toBe(true);
      expect(resultB.acquired).toBe(true);
    });

    it('different lock types on same resource are independent', async () => {
      const mgrA = makeLockManager(tmpDir, makeAgent('agent-a'), 60_000);
      const mgrB = makeLockManager(tmpDir, makeAgent('agent-b'), 60_000);

      const resultA = await mgrA.acquire({ type: 'action', resource: 'gate' });
      const resultB = await mgrB.acquire({ type: 'cache', resource: 'gate' });

      expect(resultA.acquired).toBe(true);
      expect(resultB.acquired).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // Stale / expired lock detection
  // -----------------------------------------------------------------------

  describe('stale lock detection and cleanup', () => {
    it('expired lock can be taken over by another agent', async () => {
      const mgrA = makeLockManager(tmpDir, makeAgent('agent-a'), SHORT_TIMEOUT_MS);
      const mgrB = makeLockManager(tmpDir, makeAgent('agent-b'), 60_000);

      await mgrA.acquire({ type: 'action', resource: 'gate' });

      // Wait for the lock to expire
      await new Promise((r) => setTimeout(r, SHORT_TIMEOUT_MS + 50));

      const resultB = await mgrB.acquire({ type: 'action', resource: 'gate' });
      expect(resultB.acquired).toBe(true);
      expect(resultB.lock!.agentId).toBe('agent-b');
    });

    it('cleanupExpiredLocks removes expired lock files', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'), SHORT_TIMEOUT_MS);

      await mgr.acquire({ type: 'action', resource: 'gate-1' });
      await mgr.acquire({ type: 'cache', resource: 'cache-1' });

      // Wait for locks to expire
      await new Promise((r) => setTimeout(r, SHORT_TIMEOUT_MS + 50));

      const cleaned = await mgr.cleanupExpiredLocks();
      expect(cleaned).toBe(2);
    });

    it('cleanupExpiredLocks does not remove non-expired locks', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'), 60_000);

      await mgr.acquire({ type: 'action', resource: 'gate' });

      const cleaned = await mgr.cleanupExpiredLocks();
      expect(cleaned).toBe(0);
    });

    it('cleanupExpiredLocks returns 0 for empty lock directory', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'));
      // Ensure lock dir exists (acquire + release creates it)
      await mgr.acquire({ type: 'action', resource: 'tmp' });
      await mgr.release('action', 'tmp');

      const cleaned = await mgr.cleanupExpiredLocks();
      expect(cleaned).toBe(0);
    });

    it('cleanupExpiredLocks returns 0 when lock directory does not exist', async () => {
      const freshDir = makeTmpDir();
      try {
        const mgr = makeLockManager(freshDir, makeAgent('agent-a'));
        const cleaned = await mgr.cleanupExpiredLocks();
        expect(cleaned).toBe(0);
      } finally {
        await safeCleanup(freshDir);
      }
    });
  });

  // -----------------------------------------------------------------------
  // Takeover fencing (CIB-117)
  // -----------------------------------------------------------------------

  describe('expired/stale lock takeover fencing (CIB-117)', () => {
    /** Compute the lock file path the manager uses for a given type:resource. */
    function lockPathFor(workspaceRoot: string, type: string, resource: string): string {
      const hash = createHash('sha256').update(`${type}:${resource}`).digest('hex').slice(0, 16);
      return join(workspaceRoot, '.anvil', 'locks', `${hash}.lock`);
    }

    async function makeExpiredLock(workspaceRoot: string): Promise<string> {
      const mgrStale = makeLockManager(workspaceRoot, makeAgent('agent-stale'), SHORT_TIMEOUT_MS);
      await mgrStale.acquire({ type: 'action', resource: 'gate' });
      await new Promise((r) => setTimeout(r, SHORT_TIMEOUT_MS + 50));
      return lockPathFor(workspaceRoot, 'action', 'gate');
    }

    it('does not take over an expired lock while another takeover is in flight (sentinel held)', async () => {
      const lockPath = await makeExpiredLock(tmpDir);

      // Simulate another agent mid-takeover: it holds the O_EXCL creation
      // sentinel but has not yet written the new lock record. This injects
      // the read→write interleaving deterministically.
      const sentinelPath = `${lockPath}.creating`;
      writeFileSync(sentinelPath, 'agent-other');

      const mgrB = makeLockManager(tmpDir, makeAgent('agent-b'), 60_000);
      const result = await mgrB.acquire({ type: 'action', resource: 'gate' });

      expect(result.acquired).toBe(false);

      // The in-flight agent finishes (releases the sentinel) — now B can win.
      unlinkSync(sentinelPath);
      const retry = await mgrB.acquire({ type: 'action', resource: 'gate' });
      expect(retry.acquired).toBe(true);
      expect(retry.lock!.agentId).toBe('agent-b');
    });

    it('exactly one of many concurrent takeover attempts on an expired lock wins', async () => {
      await makeExpiredLock(tmpDir);

      const contenders = Array.from({ length: 8 }, (_, i) =>
        makeLockManager(tmpDir, makeAgent(`agent-${i}`), 60_000)
      );
      const results = await Promise.all(
        contenders.map((mgr) => mgr.acquire({ type: 'action', resource: 'gate' }))
      );

      const winners = results.filter((r) => r.acquired);
      expect(winners).toHaveLength(1);
    });

    it('re-verifies the takeover precondition under the sentinel (no overwrite of a fresh lock)', async () => {
      const lockPath = await makeExpiredLock(tmpDir);

      // Agent B wins the takeover.
      const mgrB = makeLockManager(tmpDir, makeAgent('agent-b'), 60_000);
      const resultB = await mgrB.acquire({ type: 'action', resource: 'gate' });
      expect(resultB.acquired).toBe(true);

      // Agent C also saw the expired lock, but by the time it enters the
      // fenced section the lock is fresh again — it must back off.
      const mgrC = makeLockManager(tmpDir, makeAgent('agent-c'), 60_000);
      const resultC = await mgrC.acquire({ type: 'action', resource: 'gate' });
      expect(resultC.acquired).toBe(false);
      expect(resultC.heldBy?.agentId).toBe('agent-b');

      const onDisk = JSON.parse(readFileSync(lockPath, 'utf-8')) as {
        lock: { agentId: string };
      };
      expect(onDisk.lock.agentId).toBe('agent-b');
    });

    it('releases the sentinel after a successful takeover', async () => {
      const lockPath = await makeExpiredLock(tmpDir);

      const mgrB = makeLockManager(tmpDir, makeAgent('agent-b'), 60_000);
      const result = await mgrB.acquire({ type: 'action', resource: 'gate' });

      expect(result.acquired).toBe(true);
      expect(existsSync(`${lockPath}.creating`)).toBe(false);
    });

    it('reaps an abandoned takeover sentinel so a crash cannot wedge the lock', async () => {
      const lockPath = await makeExpiredLock(tmpDir);

      // A crashed agent left the sentinel behind long ago.
      const sentinelPath = `${lockPath}.creating`;
      writeFileSync(sentinelPath, 'agent-crashed');
      const past = new Date(Date.now() - 120_000);
      utimesSync(sentinelPath, past, past);

      const mgrB = makeLockManager(tmpDir, makeAgent('agent-b'), 60_000);
      const result = await mgrB.acquire({ type: 'action', resource: 'gate' });

      expect(result.acquired).toBe(true);
      expect(result.lock!.agentId).toBe('agent-b');
    });

    it('stale-holder (dead pid) takeover is also single-winner', async () => {
      // Lock held by a dead process, not yet expired.
      // pid 2147483647 is above pid_max on Linux, so it can never be alive.
      const mgrDead = makeLockManager(tmpDir, {
        id: 'agent-dead',
        type: 'claude',
        pid: 2147483647,
      });
      // Write the lock record directly so pid is the dead one.
      await mgrDead.acquire({ type: 'action', resource: 'gate', timeoutMs: 60_000 });

      const contenders = Array.from({ length: 8 }, (_, i) =>
        makeLockManager(tmpDir, makeAgent(`agent-${i}`), 60_000)
      );
      const results = await Promise.all(
        contenders.map((mgr) =>
          mgr.acquire({ type: 'action', resource: 'gate', acquireFromStale: true })
        )
      );

      expect(results.filter((r) => r.acquired)).toHaveLength(1);
    });
  });

  // -----------------------------------------------------------------------
  // isLocked
  // -----------------------------------------------------------------------

  describe('isLocked', () => {
    it('returns true when lock is active', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'), 60_000);
      await mgr.acquire({ type: 'action', resource: 'gate' });

      const locked = await mgr.isLocked('action', 'gate');
      expect(locked).toBe(true);
    });

    it('returns false after lock is released', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'), 60_000);
      await mgr.acquire({ type: 'action', resource: 'gate' });
      await mgr.release('action', 'gate');

      const locked = await mgr.isLocked('action', 'gate');
      expect(locked).toBe(false);
    });

    it('returns false when lock has expired', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'), SHORT_TIMEOUT_MS);
      await mgr.acquire({ type: 'action', resource: 'gate' });

      await new Promise((r) => setTimeout(r, SHORT_TIMEOUT_MS + 50));

      const locked = await mgr.isLocked('action', 'gate');
      expect(locked).toBe(false);
    });

    it('returns false for a lock that was never acquired', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'));
      const locked = await mgr.isLocked('action', 'nonexistent');
      expect(locked).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // getLockInfo
  // -----------------------------------------------------------------------

  describe('getLockInfo', () => {
    it('returns lock record for an active lock', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'), 60_000);
      await mgr.acquire({ type: 'action', resource: 'gate', reason: 'testing' });

      const info = await mgr.getLockInfo('action', 'gate');
      expect(info).not.toBeNull();
      expect(info!.agentId).toBe('agent-a');
      expect(info!.type).toBe('action');
      expect(info!.resource).toBe('gate');
      expect(info!.reason).toBe('testing');
    });

    it('returns null when no lock exists', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'));
      const info = await mgr.getLockInfo('action', 'nonexistent');
      expect(info).toBeNull();
    });
  });

  // -----------------------------------------------------------------------
  // holdsLock
  // -----------------------------------------------------------------------

  describe('holdsLock', () => {
    it('returns true for locks we hold', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'));
      await mgr.acquire({ type: 'action', resource: 'gate' });
      expect(mgr.holdsLock('action', 'gate')).toBe(true);
    });

    it('returns false for locks we do not hold', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'));
      expect(mgr.holdsLock('action', 'gate')).toBe(false);
    });

    it('returns false after releasing', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'));
      await mgr.acquire({ type: 'action', resource: 'gate' });
      await mgr.release('action', 'gate');
      expect(mgr.holdsLock('action', 'gate')).toBe(false);
    });
  });

  // -----------------------------------------------------------------------
  // getHeldLocks
  // -----------------------------------------------------------------------

  describe('getHeldLocks', () => {
    it('returns empty array when no locks held', () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'));
      expect(mgr.getHeldLocks()).toEqual([]);
    });

    it('returns all currently held locks', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'), 60_000);
      await mgr.acquire({ type: 'action', resource: 'gate-1' });
      await mgr.acquire({ type: 'cache', resource: 'cache-1' });

      const held = mgr.getHeldLocks();
      expect(held).toHaveLength(2);

      const resources = held.map((l) => l.resource).sort();
      expect(resources).toEqual(['cache-1', 'gate-1']);
    });

    it('reflects releases', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'), 60_000);
      await mgr.acquire({ type: 'action', resource: 'gate-1' });
      await mgr.acquire({ type: 'cache', resource: 'cache-1' });
      await mgr.release('action', 'gate-1');

      const held = mgr.getHeldLocks();
      expect(held).toHaveLength(1);
      expect(held[0].resource).toBe('cache-1');
    });
  });

  // -----------------------------------------------------------------------
  // releaseAll
  // -----------------------------------------------------------------------

  describe('releaseAll', () => {
    it('releases every lock held by the manager', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'), 60_000);
      await mgr.acquire({ type: 'action', resource: 'gate-1' });
      await mgr.acquire({ type: 'cache', resource: 'cache-1' });
      await mgr.acquire({ type: 'state', resource: 'state-1' });

      await mgr.releaseAll();

      expect(mgr.getHeldLocks()).toHaveLength(0);

      // Verify files are actually gone
      const locked1 = await mgr.isLocked('action', 'gate-1');
      const locked2 = await mgr.isLocked('cache', 'cache-1');
      const locked3 = await mgr.isLocked('state', 'state-1');
      expect(locked1).toBe(false);
      expect(locked2).toBe(false);
      expect(locked3).toBe(false);
    });

    it('works when no locks are held', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'));
      // Should not throw
      await mgr.releaseAll();
      expect(mgr.getHeldLocks()).toHaveLength(0);
    });

    it('allows another agent to acquire after releaseAll', async () => {
      const mgrA = makeLockManager(tmpDir, makeAgent('agent-a'), 60_000);
      const mgrB = makeLockManager(tmpDir, makeAgent('agent-b'), 60_000);

      await mgrA.acquire({ type: 'action', resource: 'gate' });
      await mgrA.releaseAll();

      const result = await mgrB.acquire({ type: 'action', resource: 'gate' });
      expect(result.acquired).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // getAgentId
  // -----------------------------------------------------------------------

  describe('getAgentId', () => {
    it('returns the agent ID used to create the manager', () => {
      const mgr = makeLockManager(tmpDir, makeAgent('test-agent-42'));
      expect(mgr.getAgentId()).toBe('test-agent-42');
    });
  });

  // -----------------------------------------------------------------------
  // acquire with wait
  // -----------------------------------------------------------------------

  describe('acquire with wait', () => {
    it('blocks until lock is available then acquires', async () => {
      const mgrA = makeLockManager(tmpDir, makeAgent('agent-a'), SHORT_TIMEOUT_MS);
      const mgrB = makeLockManager(tmpDir, makeAgent('agent-b'), 60_000);

      await mgrA.acquire({ type: 'action', resource: 'gate' });

      // mgrB waits; lock will expire in SHORT_TIMEOUT_MS
      const resultB = await mgrB.acquire({
        type: 'action',
        resource: 'gate',
        wait: true,
        waitTimeoutMs: 2000,
        retryIntervalMs: 30,
      });

      expect(resultB.acquired).toBe(true);
      expect(resultB.lock!.agentId).toBe('agent-b');
    });

    it('times out when lock is never released', async () => {
      const mgrA = makeLockManager(tmpDir, makeAgent('agent-a'), 60_000);
      const mgrB = makeLockManager(tmpDir, makeAgent('agent-b'), 60_000);

      await mgrA.acquire({ type: 'action', resource: 'gate' });

      const resultB = await mgrB.acquire({
        type: 'action',
        resource: 'gate',
        wait: true,
        waitTimeoutMs: 200,
        retryIntervalMs: 30,
      });

      expect(resultB.acquired).toBe(false);
      expect(resultB.error).toContain('timed out');
    });
  });

  // -----------------------------------------------------------------------
  // Lock with custom timeout
  // -----------------------------------------------------------------------

  describe('custom lock timeout', () => {
    it('uses per-acquire timeoutMs over the config default', async () => {
      const mgr = makeLockManager(tmpDir, makeAgent('agent-a'), 60_000);

      const result = await mgr.acquire({
        type: 'action',
        resource: 'gate',
        timeoutMs: 200,
      });

      expect(result.acquired).toBe(true);

      // The lock should expire in ~200ms, not the config default of 60s
      await new Promise((r) => setTimeout(r, 250));

      const locked = await mgr.isLocked('action', 'gate');
      expect(locked).toBe(false);
    });
  });
});

// ---------------------------------------------------------------------------
// atomic.ts tests
// ---------------------------------------------------------------------------

describe('atomic operations', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTmpDir();
  });

  afterEach(async () => {
    await safeCleanup(tmpDir);
  });

  // -----------------------------------------------------------------------
  // atomicWriteJson
  // -----------------------------------------------------------------------

  describe('atomicWriteJson', () => {
    it('writes valid JSON that can be read back', async () => {
      const filePath = join(tmpDir, 'test.json');
      const data = { hello: 'world', count: 42 };

      await atomicWriteJson(filePath, data);

      const raw = readFileSync(filePath, 'utf-8');
      expect(JSON.parse(raw)).toEqual(data);
    });

    it('overwrites existing file atomically', async () => {
      const filePath = join(tmpDir, 'test.json');

      await atomicWriteJson(filePath, { version: 1 });
      await atomicWriteJson(filePath, { version: 2 });

      const raw = readFileSync(filePath, 'utf-8');
      expect(JSON.parse(raw)).toEqual({ version: 2 });
    });

    it('creates parent directories when they do not exist', async () => {
      const filePath = join(tmpDir, 'deep', 'nested', 'dir', 'test.json');

      await atomicWriteJson(filePath, { nested: true });

      expect(existsSync(filePath)).toBe(true);
      const raw = readFileSync(filePath, 'utf-8');
      expect(JSON.parse(raw)).toEqual({ nested: true });
    });

    it('writes pretty-printed JSON (indented)', async () => {
      const filePath = join(tmpDir, 'pretty.json');
      await atomicWriteJson(filePath, { a: 1 });

      const raw = readFileSync(filePath, 'utf-8');
      // The implementation uses JSON.stringify(data, null, 2)
      expect(raw).toBe(JSON.stringify({ a: 1 }, null, 2));
    });

    it('leaves no temp files on success', async () => {
      const filePath = join(tmpDir, 'clean.json');
      await atomicWriteJson(filePath, { clean: true });

      const { readdirSync } = await import('node:fs');
      const files = readdirSync(tmpDir);
      // Only the target file should exist, no .tmp files
      const tmpFiles = files.filter((f) => f.endsWith('.tmp'));
      expect(tmpFiles).toHaveLength(0);
    });
  });

  // -----------------------------------------------------------------------
  // readJsonSafe
  // -----------------------------------------------------------------------

  describe('readJsonSafe', () => {
    it('reads valid JSON from file', async () => {
      const filePath = join(tmpDir, 'data.json');
      writeFileSync(filePath, JSON.stringify({ key: 'value' }));

      const result = await readJsonSafe(filePath);
      expect(result).toEqual({ key: 'value' });
    });

    it('returns null for non-existent file', async () => {
      const result = await readJsonSafe(join(tmpDir, 'missing.json'));
      expect(result).toBeNull();
    });

    it('returns null for invalid JSON', async () => {
      const filePath = join(tmpDir, 'invalid.json');
      writeFileSync(filePath, 'not { valid json');

      const result = await readJsonSafe(filePath);
      expect(result).toBeNull();
    });

    it('returns null for empty file', async () => {
      const filePath = join(tmpDir, 'empty.json');
      writeFileSync(filePath, '');

      const result = await readJsonSafe(filePath);
      expect(result).toBeNull();
    });
  });

  // -----------------------------------------------------------------------
  // tryAcquireFileLock
  // -----------------------------------------------------------------------

  describe('tryAcquireFileLock', () => {
    it('acquires a lock and returns a handle', async () => {
      const lockPath = join(tmpDir, 'test.lock');

      const handle = await tryAcquireFileLock(lockPath);

      expect(handle).not.toBeNull();
      expect(handle!.path).toBe(lockPath);
      expect(existsSync(lockPath)).toBe(true);

      await handle!.release();
    });

    it('returns null when lock file already exists', async () => {
      const lockPath = join(tmpDir, 'test.lock');

      const first = await tryAcquireFileLock(lockPath);
      expect(first).not.toBeNull();

      const second = await tryAcquireFileLock(lockPath);
      expect(second).toBeNull();

      await first!.release();
    });

    it('release removes the lock file', async () => {
      const lockPath = join(tmpDir, 'test.lock');

      const handle = await tryAcquireFileLock(lockPath);
      expect(existsSync(lockPath)).toBe(true);

      await handle!.release();
      expect(existsSync(lockPath)).toBe(false);
    });

    it('can re-acquire after release', async () => {
      const lockPath = join(tmpDir, 'test.lock');

      const first = await tryAcquireFileLock(lockPath);
      await first!.release();

      const second = await tryAcquireFileLock(lockPath);
      expect(second).not.toBeNull();

      await second!.release();
    });

    it('writes content to the lock file', async () => {
      const lockPath = join(tmpDir, 'test.lock');

      const handle = await tryAcquireFileLock(lockPath, 'owner-agent-1');
      const content = readFileSync(lockPath, 'utf-8');
      expect(content).toBe('owner-agent-1');

      await handle!.release();
    });

    it('creates parent directories if they do not exist', async () => {
      const lockPath = join(tmpDir, 'sub', 'dir', 'test.lock');

      const handle = await tryAcquireFileLock(lockPath);
      expect(handle).not.toBeNull();
      expect(existsSync(lockPath)).toBe(true);

      await handle!.release();
    });
  });

  // -----------------------------------------------------------------------
  // unlinkSafe
  // -----------------------------------------------------------------------

  describe('unlinkSafe', () => {
    it('deletes a file that exists and returns true', async () => {
      const filePath = join(tmpDir, 'deleteme.txt');
      writeFileSync(filePath, 'content');

      const result = await unlinkSafe(filePath);

      expect(result).toBe(true);
      expect(existsSync(filePath)).toBe(false);
    });

    it('returns false when file does not exist (no error)', async () => {
      const result = await unlinkSafe(join(tmpDir, 'nonexistent.txt'));
      expect(result).toBe(false);
    });

    it('returns false on repeated calls for the same file', async () => {
      const filePath = join(tmpDir, 'once.txt');
      writeFileSync(filePath, 'content');

      const first = await unlinkSafe(filePath);
      const second = await unlinkSafe(filePath);

      expect(first).toBe(true);
      expect(second).toBe(false);
    });
  });
});
