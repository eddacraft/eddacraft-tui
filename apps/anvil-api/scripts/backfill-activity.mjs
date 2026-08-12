#!/usr/bin/env node
// CLI entry for the BACT-012 one-shot refresh-token activity backfill
// (ADR-121, OQ-B). Wraps the testable implementation in
// src/db/activity-backfill.ts (compiled to dist/db/activity-backfill.js)
// behind a Pool from @neondatabase/serverless.
//
// Usage:
//   node scripts/backfill-activity.mjs           # dry-run (default) — report only
//   node scripts/backfill-activity.mjs --apply   # write last_activity_at for null rows
//
// Dry-run by default; --apply is required to write. Idempotent — a second
// --apply run always reports 0 rows affected (only-null guard). NEVER sets
// first_login_at / last_login_at / last_login_method — see
// docs/runbooks/account-activity-backfill.md.
//
// Exit codes:
//   0  success (dry-run report, or apply — including 0 rows affected)
//   1  missing DATABASE_URL or SQL error

import { Pool } from '@neondatabase/serverless';
import { runActivityBackfill } from '../dist/db/activity-backfill.js';

const apply = process.argv.includes('--apply');

const databaseUrl = process.env.DATABASE_URL;
if (!databaseUrl) {
  console.error('error: DATABASE_URL environment variable is required');
  process.exit(1);
}

const pool = new Pool({ connectionString: databaseUrl });
let client;

try {
  client = await pool.connect();
  const runner = {
    query: async (text, params) => {
      const result = await client.query(text, params);
      return { rows: result.rows };
    },
  };

  const result = await runActivityBackfill(runner, {
    apply,
    log: (msg) => console.log(msg),
  });

  if (result.dryRun) {
    console.log(`\n${result.affected} account(s) would be backfilled. Re-run with --apply to write.`);
  } else {
    console.log(`\nbackfilled ${result.affected} account(s).`);
  }
  process.exitCode = 0;
} catch (err) {
  console.error(`\nbackfill failed: ${err instanceof Error ? err.message : String(err)}`);
  process.exitCode = 1;
} finally {
  // process.exit() inside the try/catch would skip this; using
  // process.exitCode lets the finally run so the pool closes cleanly
  // and Node exits with the right code once all handles are released.
  client?.release();
  await pool.end();
}
