#!/usr/bin/env node
// Print tracked CI-log entries since a date or the Last-triaged watermark.

import { parseArgs } from 'node:util';
import { entriesSince, parseEntries, readText, readWatermark, trackedLogPath } from './lib.mjs';

if (process.argv[2] === '--') process.argv.splice(2, 1);

const { values, positionals } = parseArgs({
  options: {
    since: { type: 'string' },
    watermark: { type: 'boolean', default: false },
    headings: { type: 'boolean', default: false },
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
      '  pnpm ci-log:since --watermark',
      '  pnpm ci-log:since --since YYYY-MM-DD',
      '  pnpm ci-log:since YYYY-MM-DD',
      '',
      'Prints tracked log entries on/after the date. --watermark uses Last triaged',
      'from the log header (or all entries if unset/never).',
      '',
    ].join('\n')
  );
  process.exit(0);
}

try {
  let since = values.since ?? positionals[0] ?? null;
  if (values.watermark || !since) {
    since = readWatermark() ?? 'never';
  }
  const entries = entriesSince(since);
  if (values.json) {
    process.stdout.write(
      `${JSON.stringify(
        {
          since,
          count: entries.length,
          entries: entries.map((e) => ({
            date: e.date,
            heading: e.heading,
            body: e.body,
          })),
        },
        null,
        2
      )}\n`
    );
    process.exit(0);
  }
  process.stdout.write(
    `ci-log:since ${since} → ${entries.length} entr${entries.length === 1 ? 'y' : 'ies'}\n\n`
  );
  if (entries.length === 0) {
    const all = parseEntries(readText(trackedLogPath()));
    if (all.length) {
      process.stdout.write('(no entries after watermark; last tracked heading:)\n');
      process.stdout.write(`### ${all[all.length - 1].heading}\n`);
    }
    process.exit(0);
  }
  for (const entry of entries) {
    if (values.headings) {
      process.stdout.write(`### ${entry.heading}\n`);
    } else {
      process.stdout.write(entry.body);
      if (!entry.body.endsWith('\n\n')) process.stdout.write('\n');
    }
  }
} catch (error) {
  process.stderr.write(`ci-log:since: ${error.message}\n`);
  process.exit(1);
}
