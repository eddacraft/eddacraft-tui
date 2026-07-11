#!/usr/bin/env node
// Move shared pending CI-log notes into the tracked merge=union log.
// Run from a bookkeeping branch/PR (or any branch that will commit the log).

import { parseArgs } from 'node:util';
import { harvestPending, pendingSummary } from './lib.mjs';

if (process.argv[2] === '--') process.argv.splice(2, 1);

const { values } = parseArgs({
  options: {
    'dry-run': { type: 'boolean', default: false },
    json: { type: 'boolean', default: false },
    help: { type: 'boolean', default: false, short: 'h' },
  },
  allowPositionals: false,
  strict: true,
});

if (values.help) {
  process.stdout.write(
    [
      'Usage: pnpm ci-log:harvest [--dry-run] [--json]',
      '',
      'Appends every pending note under the git-common-dir queue into',
      'plans/reviews/continuous-improvement-log.md and deletes the pending files.',
      'Commit the tracked log in a bookkeeping PR (merge=union is safe under concurrent',
      'appends).',
      '',
    ].join('\n')
  );
  process.exit(0);
}

try {
  const before = pendingSummary();
  const result = harvestPending({ dryRun: values['dry-run'] });
  const payload = {
    ok: true,
    ...result,
    pendingDir: before.dir,
    remaining: values['dry-run'] ? before.count : 0,
  };
  if (values.json) {
    process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  } else if (result.harvested === 0) {
    process.stdout.write('ci-log:harvest: nothing pending\n');
  } else if (values['dry-run']) {
    process.stdout.write(
      `ci-log:harvest: would harvest ${result.harvested} note(s) from ${before.dir}\n`
    );
    for (const name of before.files) process.stdout.write(`  - ${name}\n`);
  } else {
    process.stdout.write(
      `ci-log:harvest: harvested ${result.harvested} note(s) into ${result.logPath}\n`
    );
    process.stdout.write(
      'Commit plans/reviews/continuous-improvement-log.md on a bookkeeping branch.\n'
    );
  }
} catch (error) {
  process.stderr.write(`ci-log:harvest: ${error.message}\n`);
  process.exit(1);
}
