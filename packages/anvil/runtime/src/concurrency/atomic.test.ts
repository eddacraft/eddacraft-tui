/**
 * Tests for atomic.ts — atomic file operations (TEST-006)
 *
 * Covers: atomicWriteJson, atomicWriteText, readJsonSafe, readJsonWithRetry,
 * acquireFileLock, tryAcquireFileLock, isLocked, forceReleaseLock, unlinkSafe,
 * fileExists, getFileMtime, sleepWithJitter
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtempSync, readFileSync, writeFileSync, existsSync, unlinkSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import {
  atomicWriteJson,
  atomicWriteText,
  readJsonSafe,
  readJsonWithRetry,
  acquireFileLock,
  tryAcquireFileLock,
  isLocked,
  forceReleaseLock,
  unlinkSafe,
  fileExists,
  getFileMtime,
  sleepWithJitter,
} from './atomic.js';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';

function makeTmpDir(): string {
  return mkdtempSync(join(tmpdir(), 'atomic-test-'));
}

describe('atomicWriteJson', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTmpDir();
  });

  afterEach(async () => {
    await safeCleanup(tmpDir);
  });

  it('writes valid JSON to file', async () => {
    const filePath = join(tmpDir, 'data.json');
    await atomicWriteJson(filePath, { hello: 'world', count: 42 });

    const content = JSON.parse(readFileSync(filePath, 'utf-8'));
    expect(content).toEqual({ hello: 'world', count: 42 });
  });

  it('creates parent directories when createDirs is true', async () => {
    const filePath = join(tmpDir, 'nested', 'deep', 'data.json');
    await atomicWriteJson(filePath, { nested: true });

    expect(existsSync(filePath)).toBe(true);
    const content = JSON.parse(readFileSync(filePath, 'utf-8'));
    expect(content).toEqual({ nested: true });
  });

  it('overwrites existing file atomically', async () => {
    const filePath = join(tmpDir, 'overwrite.json');
    await atomicWriteJson(filePath, { version: 1 });
    await atomicWriteJson(filePath, { version: 2 });

    const content = JSON.parse(readFileSync(filePath, 'utf-8'));
    expect(content).toEqual({ version: 2 });
  });

  it('does not leave temp files on success', async () => {
    const filePath = join(tmpDir, 'clean.json');
    await atomicWriteJson(filePath, { clean: true });

    const { readdirSync } = await import('node:fs');
    const files = readdirSync(tmpDir);
    expect(files).toEqual(['clean.json']);
  });

  it('formats JSON with 2-space indentation', async () => {
    const filePath = join(tmpDir, 'formatted.json');
    await atomicWriteJson(filePath, { a: 1 });

    const raw = readFileSync(filePath, 'utf-8');
    expect(raw).toBe(JSON.stringify({ a: 1 }, null, 2));
  });
});

describe('atomicWriteText', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTmpDir();
  });

  afterEach(async () => {
    await safeCleanup(tmpDir);
  });

  it('writes text content to file', async () => {
    const filePath = join(tmpDir, 'note.txt');
    await atomicWriteText(filePath, 'hello world');

    expect(readFileSync(filePath, 'utf-8')).toBe('hello world');
  });

  it('creates parent directories', async () => {
    const filePath = join(tmpDir, 'sub', 'note.txt');
    await atomicWriteText(filePath, 'nested');

    expect(readFileSync(filePath, 'utf-8')).toBe('nested');
  });
});

describe('readJsonSafe', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTmpDir();
  });

  afterEach(async () => {
    await safeCleanup(tmpDir);
  });

  it('reads valid JSON file', async () => {
    const filePath = join(tmpDir, 'valid.json');
    writeFileSync(filePath, JSON.stringify({ key: 'value' }));

    const result = await readJsonSafe(filePath);
    expect(result).toEqual({ key: 'value' });
  });

  it('returns null for non-existent file', async () => {
    const result = await readJsonSafe(join(tmpDir, 'missing.json'));
    expect(result).toBeNull();
  });

  it('returns null for invalid JSON', async () => {
    const filePath = join(tmpDir, 'bad.json');
    writeFileSync(filePath, 'not json {{{');

    const result = await readJsonSafe(filePath);
    expect(result).toBeNull();
  });
});

describe('readJsonWithRetry', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTmpDir();
  });

  afterEach(async () => {
    await safeCleanup(tmpDir);
  });

  it('reads valid JSON on first attempt', async () => {
    const filePath = join(tmpDir, 'retry.json');
    writeFileSync(filePath, JSON.stringify({ ok: true }));

    const result = await readJsonWithRetry(filePath);
    expect(result).toEqual({ ok: true });
  });

  it('returns null for non-existent file without retrying', async () => {
    const result = await readJsonWithRetry(join(tmpDir, 'gone.json'), 3, 1);
    expect(result).toBeNull();
  });

  it('returns null after all retries fail on invalid JSON', async () => {
    const filePath = join(tmpDir, 'corrupt.json');
    writeFileSync(filePath, '{{invalid');

    const result = await readJsonWithRetry(filePath, 2, 1);
    expect(result).toBeNull();
  });
});

describe('file lock operations', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTmpDir();
  });

  afterEach(async () => {
    await safeCleanup(tmpDir);
  });

  describe('acquireFileLock', () => {
    it('acquires a lock and provides a release handle', async () => {
      const lockPath = join(tmpDir, 'test.lock');
      const handle = await acquireFileLock(lockPath, { timeout: 1000 });

      expect(handle).not.toBeNull();
      expect(handle!.path).toBe(lockPath);
      expect(existsSync(lockPath)).toBe(true);

      await handle!.release();
      expect(existsSync(lockPath)).toBe(false);
    });

    it('returns null on timeout when lock is held', async () => {
      const lockPath = join(tmpDir, 'held.lock');
      writeFileSync(lockPath, 'held');

      const handle = await acquireFileLock(lockPath, {
        timeout: 100,
        retryInterval: 20,
      });

      expect(handle).toBeNull();
    });
  });

  describe('tryAcquireFileLock', () => {
    it('acquires lock when available', async () => {
      const lockPath = join(tmpDir, 'try.lock');
      const handle = await tryAcquireFileLock(lockPath);

      expect(handle).not.toBeNull();
      await handle!.release();
    });

    it('returns null when lock is already held', async () => {
      const lockPath = join(tmpDir, 'busy.lock');
      writeFileSync(lockPath, 'held');

      const handle = await tryAcquireFileLock(lockPath);
      expect(handle).toBeNull();
    });

    it('creates parent directories', async () => {
      const lockPath = join(tmpDir, 'deep', 'nested', 'dir.lock');
      const handle = await tryAcquireFileLock(lockPath);

      expect(handle).not.toBeNull();
      await handle!.release();
    });

    // CIB-117: fencing token — a holder whose lock was reaped (stale-threshold
    // theft) must not delete the new holder's live lock on release.
    it('release is a no-op when the lock was stolen and re-acquired by another holder', async () => {
      const lockPath = join(tmpDir, 'fence.lock');

      const slowHolder = await tryAcquireFileLock(lockPath, 'token-a');
      expect(slowHolder).not.toBeNull();

      // A reaper decides the slow holder is stale and steals the lock,
      // then a new holder acquires it.
      unlinkSync(lockPath);
      const newHolder = await tryAcquireFileLock(lockPath, 'token-b');
      expect(newHolder).not.toBeNull();

      // The slow holder wakes up and releases — the new holder's lock
      // must survive.
      await slowHolder!.release();
      expect(existsSync(lockPath)).toBe(true);
      expect(readFileSync(lockPath, 'utf-8')).toBe('token-b');

      // The rightful holder can still release it.
      await newHolder!.release();
      expect(existsSync(lockPath)).toBe(false);
    });

    it('release still removes the lock when content matches (normal path)', async () => {
      const lockPath = join(tmpDir, 'fence-normal.lock');
      const handle = await tryAcquireFileLock(lockPath, 'token-only-holder');
      expect(handle).not.toBeNull();

      await handle!.release();
      expect(existsSync(lockPath)).toBe(false);
    });
  });

  describe('isLocked', () => {
    it('returns true when lock file exists', async () => {
      const lockPath = join(tmpDir, 'exists.lock');
      writeFileSync(lockPath, '');

      expect(await isLocked(lockPath)).toBe(true);
    });

    it('returns false when lock file does not exist', async () => {
      expect(await isLocked(join(tmpDir, 'nope.lock'))).toBe(false);
    });
  });

  describe('forceReleaseLock', () => {
    it('removes existing lock file and returns true', async () => {
      const lockPath = join(tmpDir, 'force.lock');
      writeFileSync(lockPath, '');

      expect(await forceReleaseLock(lockPath)).toBe(true);
      expect(existsSync(lockPath)).toBe(false);
    });

    it('returns false when lock file does not exist', async () => {
      expect(await forceReleaseLock(join(tmpDir, 'missing.lock'))).toBe(false);
    });
  });
});

describe('utility functions', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTmpDir();
  });

  afterEach(async () => {
    await safeCleanup(tmpDir);
  });

  describe('unlinkSafe', () => {
    it('deletes existing file and returns true', async () => {
      const filePath = join(tmpDir, 'deleteme.txt');
      writeFileSync(filePath, 'bye');

      expect(await unlinkSafe(filePath)).toBe(true);
      expect(existsSync(filePath)).toBe(false);
    });

    it('returns false for non-existent file', async () => {
      expect(await unlinkSafe(join(tmpDir, 'gone.txt'))).toBe(false);
    });
  });

  describe('fileExists', () => {
    it('returns true for existing file', async () => {
      const filePath = join(tmpDir, 'here.txt');
      writeFileSync(filePath, '');

      expect(await fileExists(filePath)).toBe(true);
    });

    it('returns false for non-existent file', async () => {
      expect(await fileExists(join(tmpDir, 'nope.txt'))).toBe(false);
    });
  });

  describe('getFileMtime', () => {
    it('returns Date for existing file', async () => {
      const filePath = join(tmpDir, 'mtime.txt');
      writeFileSync(filePath, '');

      const mtime = await getFileMtime(filePath);
      expect(mtime).toBeInstanceOf(Date);
    });

    it('returns null for non-existent file', async () => {
      expect(await getFileMtime(join(tmpDir, 'nope.txt'))).toBeNull();
    });
  });

  describe('sleepWithJitter', () => {
    it('resolves without error', async () => {
      await expect(sleepWithJitter(1, 0.5)).resolves.toBeUndefined();
    });
  });
});
