import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

// BACT-011: same posture as plan-activity-migration.test.ts (BACT-008) — no
// live Postgres in this suite; structural/content assertions on the
// migration file and its schema.sql mirror.

const here = dirname(fileURLToPath(import.meta.url));
const migrationPath = join(here, '..', 'db', 'migrations', '022-account-activity-rollup-daily.sql');
const schemaPath = join(here, '..', 'db', 'schema.sql');

const migrationSql = readFileSync(migrationPath, 'utf-8');
const schemaSql = readFileSync(schemaPath, 'utf-8');

describe('022-account-activity-rollup-daily.sql (BACT-011)', () => {
  it('creates activity_rollup_daily with day/plan grain, idempotent guard', () => {
    expect(migrationSql).toMatch(/CREATE TABLE IF NOT EXISTS activity_rollup_daily/);
    expect(migrationSql).toMatch(/day\s+date NOT NULL/);
    expect(migrationSql).toMatch(/plan\s+text NOT NULL/);
    expect(migrationSql).toMatch(/active_accounts\s+int\s+NOT NULL CHECK \(active_accounts >= 0\)/);
    expect(migrationSql).toMatch(/computed_at\s+timestamptz NOT NULL DEFAULT now\(\)/);
    expect(migrationSql).toMatch(/PRIMARY KEY \(day, plan\)/);
  });

  it('creates a supporting index for recent-history reads', () => {
    expect(migrationSql).toMatch(
      /CREATE INDEX IF NOT EXISTS idx_activity_rollup_daily_plan_day\s+ON activity_rollup_daily \(plan, day DESC\);/
    );
  });

  it('documents the retention choice (kept indefinitely, small volume)', () => {
    expect(migrationSql.toLowerCase()).toMatch(/retention/);
    expect(migrationSql.toLowerCase()).toMatch(/indefinitely/);
  });

  it('documents the late-rollup undercount caveat', () => {
    expect(migrationSql.toLowerCase()).toMatch(/undercount/);
  });

  it('documents best-observation GREATEST semantics (BACT-011 F2)', () => {
    expect(migrationSql).toMatch(/GREATEST/);
    expect(migrationSql.toLowerCase()).toMatch(/never decrease/);
  });
});

describe('schema.sql mirrors migration 022 (fresh-install parity)', () => {
  it('declares activity_rollup_daily with the same shape', () => {
    expect(schemaSql).toMatch(/CREATE TABLE activity_rollup_daily/);
    expect(schemaSql).toMatch(/day\s+date NOT NULL/);
    expect(schemaSql).toMatch(/plan\s+text NOT NULL/);
    expect(schemaSql).toMatch(/active_accounts\s+int\s+NOT NULL CHECK \(active_accounts >= 0\)/);
    expect(schemaSql).toMatch(/PRIMARY KEY \(day, plan\)/);
  });

  it('declares the plan/day index', () => {
    expect(schemaSql).toMatch(
      /CREATE INDEX idx_activity_rollup_daily_plan_day\s+ON activity_rollup_daily \(plan, day DESC\);/
    );
  });
});
