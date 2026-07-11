#!/usr/bin/env node
// Set or clear the Last-triaged watermark on the tracked CI-log.

import { parseArgs } from 'node:util';
import { setWatermark, todayUtc } from './lib.mjs';

if (process.argv[2] === '--') process.argv.splice(2, 1);

const { values, positionals } = parseArgs({
  options: {
    date: { type: 'string' },
    today: { type: 'boolean', default: false },
    never: { type: 'boolean', default: false },
    json: { type: 'boolean', default: false },
    help: { type: 'boolean', default: false, short: 'h' },
  },
  allowPositionals: true,
  strict: true,
});

if (values.help) {
  process.stdout.write(
    [
      'Usage:',
      '  pnpm ci-log:set-watermark --today',
      '  pnpm ci-log:set-watermark --date YYYY-MM-DD',
      '  pnpm ci-log:set-watermark --never',
      '',
    ].join('\n')
  );
  process.exit(0);
}

try {
  let date;
  if (values.never) date = 'never';
  else if (values.today) date = todayUtc();
  else date = values.date ?? positionals[0];
  if (!date) {
    throw new Error('provide --today, --never, --date YYYY-MM-DD, or a positional date');
  }
  if (date !== 'never' && !/^\d{4}-\d{2}-\d{2}$/.test(date)) {
    throw new Error(`invalid date: ${date}`);
  }
  const written = setWatermark(date);
  if (values.json) {
    process.stdout.write(`${JSON.stringify({ ok: true, lastTriaged: written })}\n`);
  } else {
    process.stdout.write(`ci-log:set-watermark: Last triaged → ${written}\n`);
  }
} catch (error) {
  process.stderr.write(`ci-log:set-watermark: ${error.message}\n`);
  process.exit(1);
}
