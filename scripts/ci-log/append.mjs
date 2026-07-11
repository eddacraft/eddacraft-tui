#!/usr/bin/env node
// Append a continuous-improvement note to the shared pending queue (default)
// or the tracked log (--tracked). Prefer pending so feature PRs do not have to
// carry an "unrelated" log file; harvest later with `pnpm ci-log:harvest`.

import { parseArgs } from 'node:util';
import { readFileSync } from 'node:fs';
import {
  appendTrackedEntry,
  buildEntryFromFields,
  normaliseEntry,
  todayUtc,
  writePendingEntry,
} from './lib.mjs';

// pnpm users often pass a bare "--" before flags; drop it so parseArgs is happy.
if (process.argv[2] === '--') process.argv.splice(2, 1);

function usage(code = 0) {
  const text = [
    'Usage:',
    '  pnpm ci-log:append --task "..." [options]',
    '  pnpm ci-log:append --body-file entry.md',
    '  pnpm ci-log:append --stdin < entry.md',
    '  (a bare -- before flags is also accepted)',
    '',
    'Default destination is the shared pending queue under the git common dir',
    '(survives worktree removal; not part of feature PRs).',
    '',
    'Options:',
    '  --task <text>         Required unless --body-file/--stdin/--body',
    '  --outcome <text>',
    '  --worked <text>',
    '  --failed <text>',
    '  --friction <text>',
    '  --improvement <text>',
    '  --follow-up <text>    Prefer: none | session:... | promote: CIB | theme:... | owned: ID',
    '  --agent <name>        opencode | claude | codex | other (default: other)',
    '  --date YYYY-MM-DD     default: today UTC',
    '  --title <text>        optional heading suffix after the agent',
    '  --body <text>         full entry markdown (overrides fields)',
    '  --body-file <path>    read full entry markdown from file',
    '  --stdin               read full entry markdown from stdin',
    '  --tracked             write to tracked log instead of pending (bookkeeping PRs)',
    '  --json                machine-readable result',
    '  -h, --help',
    '',
  ].join('\n');
  process.stdout.write(text);
  process.exit(code);
}

function readStdin() {
  return readFileSync(0, 'utf8');
}

const { values } = parseArgs({
  options: {
    task: { type: 'string' },
    outcome: { type: 'string' },
    worked: { type: 'string' },
    failed: { type: 'string' },
    friction: { type: 'string' },
    improvement: { type: 'string' },
    'follow-up': { type: 'string' },
    agent: { type: 'string', default: 'other' },
    date: { type: 'string' },
    title: { type: 'string' },
    body: { type: 'string' },
    'body-file': { type: 'string' },
    stdin: { type: 'boolean', default: false },
    tracked: { type: 'boolean', default: false },
    json: { type: 'boolean', default: false },
    help: { type: 'boolean', default: false, short: 'h' },
  },
  allowPositionals: false,
  strict: true,
});

if (values.help) usage(0);

let entry;
try {
  if (values.stdin) {
    entry = normaliseEntry(readStdin());
  } else if (values['body-file']) {
    entry = normaliseEntry(readFileSync(values['body-file'], 'utf8'));
  } else if (values.body) {
    entry = normaliseEntry(values.body);
  } else {
    entry = buildEntryFromFields({
      date: values.date ?? todayUtc(),
      agent: values.agent,
      task: values.task,
      outcome: values.outcome,
      worked: values.worked,
      failed: values.failed,
      friction: values.friction,
      improvement: values.improvement,
      followUp: values['follow-up'],
      title: values.title,
    });
  }

  const dest = values.tracked ? appendTrackedEntry(entry) : writePendingEntry(entry);

  const result = {
    ok: true,
    destination: values.tracked ? 'tracked' : 'pending',
    path: dest,
  };
  if (values.json) {
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  } else {
    process.stdout.write(`ci-log: wrote ${result.destination} entry → ${result.path}\n`);
  }
} catch (error) {
  process.stderr.write(`ci-log:append: ${error.message}\n`);
  process.exit(1);
}
