#!/usr/bin/env node
// Summarise CI-log pending queue + tracked log watermark / last entry.

import { parseArgs } from 'node:util';
import { existsSync } from 'node:fs';
import {
  parseEntries,
  pendingSummary,
  readText,
  readWatermark,
  trackedLogPath,
  TRACKED_LOG_REL,
} from './lib.mjs';

if (process.argv[2] === '--') process.argv.splice(2, 1);

const { values } = parseArgs({
  options: {
    json: { type: 'boolean', default: false },
    help: { type: 'boolean', default: false, short: 'h' },
  },
  allowPositionals: false,
  strict: true,
});

if (values.help) {
  process.stdout.write('Usage: pnpm ci-log:status [--json]\n');
  process.exit(0);
}

try {
  const pending = pendingSummary();
  const logPath = trackedLogPath();
  let lastEntry = null;
  let entryCount = 0;
  if (existsSync(logPath)) {
    const entries = parseEntries(readText(logPath));
    entryCount = entries.length;
    lastEntry = entries.length ? entries[entries.length - 1].heading : null;
  }
  const watermark = readWatermark();
  const payload = {
    trackedLog: TRACKED_LOG_REL,
    entryCount,
    lastEntry,
    lastTriaged: watermark,
    pendingDir: pending.dir,
    pendingCount: pending.count,
    pendingFiles: pending.files,
  };
  if (values.json) {
    process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  } else {
    process.stdout.write('Continuous improvement log\n');
    process.stdout.write(`  Tracked:       ${payload.trackedLog}\n`);
    process.stdout.write(`  Entries:       ${payload.entryCount}\n`);
    process.stdout.write(`  Last entry:    ${payload.lastEntry ?? '(none)'}\n`);
    process.stdout.write(`  Last triaged:  ${payload.lastTriaged ?? '(no watermark)'}\n`);
    process.stdout.write(`  Pending dir:   ${payload.pendingDir}\n`);
    process.stdout.write(`  Pending:       ${payload.pendingCount}\n`);
    for (const name of payload.pendingFiles) {
      process.stdout.write(`    - ${name}\n`);
    }
    if (payload.pendingCount > 0) {
      process.stdout.write('\nHarvest with: pnpm ci-log:harvest  (then commit the tracked log)\n');
    }
  }
} catch (error) {
  process.stderr.write(`ci-log:status: ${error.message}\n`);
  process.exit(1);
}
