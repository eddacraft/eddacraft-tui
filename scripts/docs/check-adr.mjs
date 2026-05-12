#!/usr/bin/env node
// Surface delegate: ADR integrity.
//
// Wraps the existing `pnpm adr:check` (which runs `scripts/docs/adr-integrity.sh`)
// so the `pnpm docs:check` orchestrator can include ADR integrity in its
// labelled summary without duplicating logic. Forwards the underlying exit
// code.

import { spawnSync } from 'node:child_process';

const SURFACE = 'adr';
const result = spawnSync('pnpm', ['-s', 'adr:check'], {
  stdio: ['inherit', 'pipe', 'pipe'],
  encoding: 'utf8',
});

const stdout = (result.stdout ?? '').trimEnd();
const stderr = (result.stderr ?? '').trimEnd();
for (const line of stdout.split('\n')) if (line) console.log(`[${SURFACE}] ${line}`);
for (const line of stderr.split('\n')) if (line) console.error(`[${SURFACE}] ${line}`);

if (result.error) {
  console.error(`[${SURFACE}] failed to invoke pnpm adr:check: ${result.error.message}`);
  process.exit(2);
}
process.exit(result.status ?? 1);
