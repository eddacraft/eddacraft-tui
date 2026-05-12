#!/usr/bin/env node
// Surface delegate: APS/index consistency.
//
// Wraps the existing `pnpm aps:drift` checker so the `pnpm docs:check`
// orchestrator can include APS/index drift in its labelled summary without
// duplicating logic. Forwards the underlying exit code.

import { spawnSync } from 'node:child_process';

const SURFACE = 'aps';
const result = spawnSync('pnpm', ['-s', 'aps:drift'], {
  stdio: ['inherit', 'pipe', 'pipe'],
  encoding: 'utf8',
});

const stdout = (result.stdout ?? '').trimEnd();
const stderr = (result.stderr ?? '').trimEnd();
for (const line of stdout.split('\n')) if (line) console.log(`[${SURFACE}] ${line}`);
for (const line of stderr.split('\n')) if (line) console.error(`[${SURFACE}] ${line}`);

if (result.error) {
  console.error(`[${SURFACE}] failed to invoke pnpm aps:drift: ${result.error.message}`);
  process.exit(2);
}
process.exit(result.status ?? 1);
