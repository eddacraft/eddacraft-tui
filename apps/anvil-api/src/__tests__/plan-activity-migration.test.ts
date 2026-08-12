import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

// BACT-008: no live Postgres is available in this test suite (see
// migrate.test.ts, which exercises the runner mechanism against a fake query
// log rather than real SQL execution — the same posture BACT-002/004 shipped
// under). These are structural/content assertions on the actual migration
// file and its schema.sql mirror, catching drift between the two and
// confirming the idempotent-guard shape the runner expects.

const here = dirname(fileURLToPath(import.meta.url));
const migrationPath = join(here, '..', 'db', 'migrations', '021-beta-users-plan-activity.sql');
const schemaPath = join(here, '..', 'db', 'schema.sql');

const migrationSql = readFileSync(migrationPath, 'utf-8');
const schemaSql = readFileSync(schemaPath, 'utf-8');

describe('021-beta-users-plan-activity.sql (BACT-008)', () => {
  it('adds plan as NOT NULL DEFAULT beta with a closed CHECK', () => {
    expect(migrationSql).toMatch(
      /ALTER TABLE beta_users ADD COLUMN plan text NOT NULL DEFAULT 'beta'/
    );
    expect(migrationSql).toMatch(/CHECK \(plan IN \('beta'\)\)/);
  });

  it('adds nullable last_activity_at', () => {
    expect(migrationSql).toMatch(/ADD COLUMN last_activity_at timestamptz;/);
  });

  it('adds last_activity_kind constrained to login|refresh|feature', () => {
    expect(migrationSql).toMatch(/ADD COLUMN last_activity_kind text/);
    expect(migrationSql).toContain("last_activity_kind IN ('login', 'refresh', 'feature')");
    expect(migrationSql).toContain('last_activity_kind IS NULL');
  });

  it('guards every ADD COLUMN with an information_schema existence check (idempotent re-apply)', () => {
    const addColumnCount = (migrationSql.match(/ALTER TABLE beta_users ADD COLUMN/g) ?? []).length;
    const guardCount = (migrationSql.match(/SELECT 1 FROM information_schema\.columns/g) ?? [])
      .length;
    expect(addColumnCount).toBe(3); // plan, last_activity_at, last_activity_kind
    expect(guardCount).toBe(addColumnCount);
  });

  it('creates the last_activity_at index with IF NOT EXISTS', () => {
    expect(migrationSql).toMatch(
      /CREATE INDEX IF NOT EXISTS idx_beta_users_last_activity_at\s+ON beta_users \(last_activity_at DESC NULLS LAST\);/
    );
  });

  it('sets a lock_timeout guard, matching prior beta_users migrations', () => {
    expect(migrationSql).toContain("SET LOCAL lock_timeout = '30s';");
  });
});

describe('schema.sql mirrors migration 021 (fresh-install parity)', () => {
  it('declares plan, last_activity_at, and last_activity_kind on beta_users', () => {
    expect(schemaSql).toMatch(/plan\s+text NOT NULL DEFAULT 'beta'/);
    expect(schemaSql).toMatch(/CHECK \(plan IN \('beta'\)\)/);
    expect(schemaSql).toMatch(/last_activity_at\s+timestamptz,/);
    expect(schemaSql).toContain("last_activity_kind IN ('login', 'refresh', 'feature')");
  });

  it('declares the last_activity_at index', () => {
    expect(schemaSql).toMatch(
      /CREATE INDEX idx_beta_users_last_activity_at\s+ON beta_users \(last_activity_at DESC NULLS LAST\);/
    );
  });
});
