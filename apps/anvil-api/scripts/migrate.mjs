#!/usr/bin/env node
// CLI entry for the database migration runner. Wraps the testable
// implementation in src/db/migrate.ts (compiled to dist/db/migrate.js)
// behind a Pool from @neondatabase/serverless so we can ship arbitrary
// multi-statement SQL files (each as a single Postgres query).
//
// Usage:
//   node scripts/migrate.mjs                # apply pending migrations
//   node scripts/migrate.mjs --dry-run      # report pending without applying
//
// Exit codes:
//   0  success — pending applied or nothing to do
//   1  drift detected, missing DATABASE_URL, or SQL error
//
// CI wiring lives in .github/workflows/release.yml between the `host`
// job (artefacts published) and the existing Pulumi Up step. Manual
// invocation is documented in docs/runbooks/db-migrations.md.

import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { Pool } from '@neondatabase/serverless';
import { runMigrations } from '../dist/db/migrate.js';

const here = dirname(fileURLToPath(import.meta.url));
const migrationsDir = resolve(here, '..', 'src', 'db', 'migrations');

const dryRun = process.argv.includes('--dry-run');

const databaseUrl = process.env.DATABASE_URL;
if (!databaseUrl) {
  console.error('error: DATABASE_URL environment variable is required');
  process.exit(1);
}

const pool = new Pool({ connectionString: databaseUrl });

const runner = {
  query: async (text, params) => {
    const result = await pool.query(text, params);
    return { rows: result.rows };
  },
};

try {
  const result = await runMigrations(runner, {
    dir: migrationsDir,
    dryRun,
    log: (msg) => console.log(msg),
  });

  if (result.applied.length > 0) {
    console.log(`\napplied ${result.applied.length} migration(s):`);
    for (const f of result.applied) console.log(`  ✓ ${f}`);
  } else if (!dryRun) {
    console.log('no pending migrations.');
  }
  process.exit(0);
} catch (err) {
  console.error(`\nmigration failed: ${err instanceof Error ? err.message : String(err)}`);
  process.exit(1);
} finally {
  await pool.end();
}
