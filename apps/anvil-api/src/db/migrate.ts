import { createHash } from 'node:crypto';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

export interface MigrationFile {
  filename: string;
  sha256: string;
  contents: string;
}

export interface AppliedMigrationRow {
  filename: string;
  sha256: string;
}

export interface QueryRunner {
  query: (text: string, params?: unknown[]) => Promise<{ rows: unknown[] }>;
}

export interface ApplyResult {
  applied: string[];
  skipped: string[];
  driftDetected: Array<{ filename: string; recordedSha: string; onDiskSha: string }>;
}

const TRACKING_TABLE_DDL = `
  CREATE TABLE IF NOT EXISTS _migrations (
    filename    TEXT PRIMARY KEY,
    sha256      TEXT NOT NULL,
    applied_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
  );
`;

// Advisory lock keys derived from a stable name so two concurrent
// runners (e.g. CI + an operator running by hand, or a retried deploy)
// serialise instead of racing on the same DDL. Postgres advisory locks
// take two int4 keys; we hash the name and split.
const LOCK_NAME = 'apps/anvil-api:db:migrations';
const LOCK_KEYS = (() => {
  const digest = createHash('sha256').update(LOCK_NAME).digest();
  return [digest.readInt32BE(0), digest.readInt32BE(4)] as const;
})();

async function acquireLock(runner: QueryRunner): Promise<void> {
  await runner.query('SELECT pg_advisory_lock($1, $2)', [LOCK_KEYS[0], LOCK_KEYS[1]]);
}

async function releaseLock(runner: QueryRunner): Promise<void> {
  await runner.query('SELECT pg_advisory_unlock($1, $2)', [LOCK_KEYS[0], LOCK_KEYS[1]]);
}

export function discoverMigrations(dir: string): MigrationFile[] {
  const entries = readdirSync(dir)
    .filter((name) => name.endsWith('.sql'))
    .sort();

  return entries.map((filename) => {
    const contents = readFileSync(join(dir, filename), 'utf8');
    const sha256 = createHash('sha256').update(contents).digest('hex');
    return { filename, sha256, contents };
  });
}

export async function ensureTrackingTable(runner: QueryRunner): Promise<void> {
  await runner.query(TRACKING_TABLE_DDL);
}

export async function fetchAppliedMigrations(runner: QueryRunner): Promise<AppliedMigrationRow[]> {
  const result = await runner.query('SELECT filename, sha256 FROM _migrations ORDER BY filename');
  return result.rows as AppliedMigrationRow[];
}

async function trackingTableExists(runner: QueryRunner): Promise<boolean> {
  const result = await runner.query("SELECT to_regclass('public._migrations') AS relation");
  const row = result.rows[0] as { relation?: unknown } | undefined;
  return typeof row?.relation === 'string';
}

export function detectDrift(
  onDisk: MigrationFile[],
  applied: AppliedMigrationRow[]
): Array<{ filename: string; recordedSha: string; onDiskSha: string }> {
  const onDiskByName = new Map(onDisk.map((m) => [m.filename, m]));
  const drift: Array<{ filename: string; recordedSha: string; onDiskSha: string }> = [];

  for (const row of applied) {
    const file = onDiskByName.get(row.filename);
    if (!file) {
      drift.push({
        filename: row.filename,
        recordedSha: row.sha256,
        onDiskSha: '<missing on disk>',
      });
      continue;
    }
    if (file.sha256 !== row.sha256) {
      drift.push({
        filename: row.filename,
        recordedSha: row.sha256,
        onDiskSha: file.sha256,
      });
    }
  }

  return drift;
}

export function selectPending(
  onDisk: MigrationFile[],
  applied: AppliedMigrationRow[]
): MigrationFile[] {
  const appliedSet = new Set(applied.map((r) => r.filename));
  return onDisk.filter((m) => !appliedSet.has(m.filename));
}

export async function applyMigration(runner: QueryRunner, migration: MigrationFile): Promise<void> {
  await runner.query('BEGIN');
  try {
    await runner.query(migration.contents);
    await runner.query('INSERT INTO _migrations (filename, sha256) VALUES ($1, $2)', [
      migration.filename,
      migration.sha256,
    ]);
    await runner.query('COMMIT');
  } catch (err) {
    await runner.query('ROLLBACK');
    throw err;
  }
}

export interface RunOptions {
  dir: string;
  dryRun?: boolean;
  log?: (message: string) => void;
}

export async function runMigrations(
  runner: QueryRunner,
  options: RunOptions
): Promise<ApplyResult> {
  const log = options.log ?? (() => {});

  const lockRequired = !options.dryRun;
  if (lockRequired) {
    await acquireLock(runner);
  }
  try {
    const onDisk = discoverMigrations(options.dir);
    let applied: AppliedMigrationRow[];
    if (options.dryRun) {
      applied = (await trackingTableExists(runner)) ? await fetchAppliedMigrations(runner) : [];
    } else {
      await ensureTrackingTable(runner);
      applied = await fetchAppliedMigrations(runner);
    }
    const drift = detectDrift(onDisk, applied);

    if (drift.length > 0) {
      const lines = drift.map(
        (d) =>
          `  ${d.filename}: recorded sha=${d.recordedSha.slice(0, 12)} on-disk sha=${
            typeof d.onDiskSha === 'string' && d.onDiskSha.startsWith('<')
              ? d.onDiskSha
              : d.onDiskSha.slice(0, 12)
          }`
      );
      throw new Error(
        `Migration drift detected — applied migrations have changed on disk:\n${lines.join(
          '\n'
        )}\nRefusing to apply. Investigate the diff or revert the file before re-running.`
      );
    }

    const pending = selectPending(onDisk, applied);

    log(
      `Discovered ${onDisk.length} migration files. ${applied.length} applied, ${pending.length} pending.`
    );

    if (pending.length === 0) {
      return { applied: [], skipped: applied.map((r) => r.filename), driftDetected: [] };
    }

    if (options.dryRun) {
      log('--dry-run: would apply:');
      for (const m of pending) {
        log(`  + ${m.filename}`);
      }
      return { applied: [], skipped: applied.map((r) => r.filename), driftDetected: [] };
    }

    const appliedThisRun: string[] = [];
    for (const migration of pending) {
      log(`applying ${migration.filename} (sha256=${migration.sha256.slice(0, 12)})`);
      await applyMigration(runner, migration);
      appliedThisRun.push(migration.filename);
    }

    return {
      applied: appliedThisRun,
      skipped: applied.map((r) => r.filename),
      driftDetected: [],
    };
  } finally {
    if (lockRequired) {
      await releaseLock(runner);
    }
  }
}
