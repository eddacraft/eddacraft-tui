import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  applyMigration,
  detectDrift,
  discoverMigrations,
  runMigrations,
  selectPending,
} from '../db/migrate.js';
import type { QueryRunner } from '../db/migrate.js';

function makeFixtureDir(files: Record<string, string>): string {
  const dir = mkdtempSync(join(tmpdir(), 'anvil-migrate-test-'));
  for (const [name, contents] of Object.entries(files)) {
    writeFileSync(join(dir, name), contents);
  }
  return dir;
}

interface RunnerState {
  appliedRows: Array<{ filename: string; sha256: string }>;
  failNextApply: boolean;
}

interface FakeRunner extends QueryRunner {
  calls: Array<{ text: string; params?: unknown[] }>;
  state: RunnerState;
}

function makeRunner(): FakeRunner {
  const state: RunnerState = { appliedRows: [], failNextApply: false };
  const calls: Array<{ text: string; params?: unknown[] }> = [];
  const query = vi.fn(async (text: string, params?: unknown[]) => {
    calls.push({ text, params });
    const trimmed = text.trim();
    if (trimmed.startsWith('SELECT filename, sha256 FROM _migrations')) {
      return { rows: state.appliedRows };
    }
    if (trimmed.startsWith('INSERT INTO _migrations')) {
      const [filename, sha256] = params as [string, string];
      state.appliedRows.push({ filename, sha256 });
      return { rows: [] };
    }
    if (state.failNextApply && !/(BEGIN|ROLLBACK|COMMIT|CREATE TABLE)/i.test(trimmed)) {
      state.failNextApply = false;
      throw new Error('simulated SQL failure');
    }
    return { rows: [] };
  });
  return { query, calls, state };
}

describe('discoverMigrations', () => {
  let dir: string;
  beforeEach(() => {
    dir = makeFixtureDir({
      '002-second.sql': 'SELECT 2;',
      '001-first.sql': 'SELECT 1;',
      '010-tenth.sql': 'SELECT 10;',
      'not-a-migration.txt': 'ignore me',
    });
  });
  afterEach(() => rmSync(dir, { recursive: true, force: true }));

  it('returns SQL files in lexical order', () => {
    const files = discoverMigrations(dir);
    expect(files.map((f) => f.filename)).toEqual([
      '001-first.sql',
      '002-second.sql',
      '010-tenth.sql',
    ]);
  });

  it('computes sha256 of contents', () => {
    const files = discoverMigrations(dir);
    expect(files[0].sha256).toMatch(/^[a-f0-9]{64}$/);
    expect(files[0].sha256).not.toBe(files[1].sha256);
  });

  it('skips non-SQL files', () => {
    const files = discoverMigrations(dir);
    expect(files.find((f) => f.filename.endsWith('.txt'))).toBeUndefined();
  });
});

describe('detectDrift', () => {
  it('returns empty when shas match', () => {
    const onDisk = [{ filename: '001.sql', sha256: 'abc', contents: '' }];
    const applied = [{ filename: '001.sql', sha256: 'abc' }];
    expect(detectDrift(onDisk, applied)).toEqual([]);
  });

  it('flags recorded sha differing from on-disk sha', () => {
    const onDisk = [{ filename: '001.sql', sha256: 'newhash', contents: '' }];
    const applied = [{ filename: '001.sql', sha256: 'oldhash' }];
    const drift = detectDrift(onDisk, applied);
    expect(drift).toHaveLength(1);
    expect(drift[0]).toMatchObject({
      filename: '001.sql',
      recordedSha: 'oldhash',
      onDiskSha: 'newhash',
    });
  });

  it('flags applied migration missing from disk', () => {
    const onDisk: Array<{ filename: string; sha256: string; contents: string }> = [];
    const applied = [{ filename: '001.sql', sha256: 'abc' }];
    const drift = detectDrift(onDisk, applied);
    expect(drift).toHaveLength(1);
    expect(drift[0].onDiskSha).toBe('<missing on disk>');
  });
});

describe('selectPending', () => {
  it('returns only files not yet recorded', () => {
    const onDisk = [
      { filename: '001.sql', sha256: 'a', contents: '' },
      { filename: '002.sql', sha256: 'b', contents: '' },
      { filename: '003.sql', sha256: 'c', contents: '' },
    ];
    const applied = [{ filename: '001.sql', sha256: 'a' }];
    const pending = selectPending(onDisk, applied);
    expect(pending.map((m) => m.filename)).toEqual(['002.sql', '003.sql']);
  });
});

describe('applyMigration', () => {
  it('wraps the file contents in BEGIN/COMMIT and inserts into _migrations', async () => {
    const runner = makeRunner();
    await applyMigration(runner, {
      filename: '001.sql',
      sha256: 'abc123',
      contents: 'CREATE TABLE foo (id INT);',
    });

    const texts = runner.calls.map((c) => c.text.trim());
    expect(texts).toEqual([
      'BEGIN',
      'CREATE TABLE foo (id INT);',
      'INSERT INTO _migrations (filename, sha256) VALUES ($1, $2)',
      'COMMIT',
    ]);
    const insert = runner.calls.find((c) => c.text.trim().startsWith('INSERT'))!;
    expect(insert.params).toEqual(['001.sql', 'abc123']);
  });

  it('rolls back when the migration body throws', async () => {
    const runner = makeRunner();
    runner.state.failNextApply = true;
    await expect(
      applyMigration(runner, { filename: 'bad.sql', sha256: 'x', contents: 'INVALID;' })
    ).rejects.toThrow('simulated SQL failure');
    const texts = runner.calls.map((c) => c.text.trim());
    expect(texts).toContain('ROLLBACK');
    expect(texts).not.toContain('COMMIT');
  });
});

describe('runMigrations', () => {
  let dir: string;
  beforeEach(() => {
    dir = makeFixtureDir({
      '001-first.sql': '-- first',
      '002-second.sql': '-- second',
    });
  });
  afterEach(() => rmSync(dir, { recursive: true, force: true }));

  it('applies pending migrations in order on a fresh database', async () => {
    const runner = makeRunner();
    const result = await runMigrations(runner, { dir });
    expect(result.applied).toEqual(['001-first.sql', '002-second.sql']);
    expect(result.driftDetected).toEqual([]);
  });

  it('reports nothing pending when all migrations are recorded', async () => {
    const runner = makeRunner();
    await runMigrations(runner, { dir });
    runner.calls.length = 0;

    const second = await runMigrations(runner, { dir });
    expect(second.applied).toEqual([]);
    expect(second.skipped).toEqual(['001-first.sql', '002-second.sql']);
  });

  it('refuses to apply when an applied file has been edited on disk', async () => {
    const runner = makeRunner();
    await runMigrations(runner, { dir });

    writeFileSync(join(dir, '001-first.sql'), '-- mutated');

    await expect(runMigrations(runner, { dir })).rejects.toThrow(/Migration drift detected/);
  });

  it('--dry-run lists pending without applying', async () => {
    const runner = makeRunner();
    const result = await runMigrations(runner, { dir, dryRun: true });
    expect(result.applied).toEqual([]);
    const texts = runner.calls.map((c) => c.text.trim());
    expect(texts.some((text) => text.startsWith('CREATE TABLE'))).toBe(false);
    expect(texts.some((text) => text.startsWith('INSERT INTO _migrations'))).toBe(false);
    expect(texts).not.toContain('BEGIN');
    expect(texts).not.toContain('COMMIT');
    expect(texts).not.toContain('ROLLBACK');
    expect(texts).not.toContain('SELECT pg_advisory_lock($1, $2)');
    expect(texts.some((text) => text.startsWith("SELECT to_regclass('public._migrations')"))).toBe(
      true
    );
  });

  it('acquires and releases the advisory lock around the run', async () => {
    const runner = makeRunner();
    await runMigrations(runner, { dir });
    const texts = runner.calls.map((c) => c.text.trim());
    const lockIdx = texts.indexOf('SELECT pg_advisory_lock($1, $2)');
    const unlockIdx = texts.indexOf('SELECT pg_advisory_unlock($1, $2)');
    expect(lockIdx).toBeGreaterThanOrEqual(0);
    expect(unlockIdx).toBeGreaterThan(lockIdx);
    expect(lockIdx).toBeLessThan(texts.indexOf('BEGIN'));
  });

  it('releases the advisory lock even when drift detection throws', async () => {
    const runner = makeRunner();
    await runMigrations(runner, { dir });

    writeFileSync(join(dir, '001-first.sql'), '-- mutated');

    await expect(runMigrations(runner, { dir })).rejects.toThrow(/Migration drift detected/);
    const texts = runner.calls.map((c) => c.text.trim());
    expect(texts.filter((t) => t === 'SELECT pg_advisory_unlock($1, $2)').length).toBeGreaterThan(
      0
    );
  });
});
