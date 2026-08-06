#!/usr/bin/env node
// Surface delegate: APS/index consistency.
//
// Invokes scripts/aps/drift-check.mjs directly so the `pnpm docs:check`
// orchestrator can include APS/index drift in its labelled summary without
// duplicating logic. It deliberately does NOT re-enter the package manager
// (`pnpm aps:drift`): that round-trip made this surface report a docs content
// `FAIL` whenever the package manager itself was broken, on a corpus that was
// clean (CIB-278). Exit codes follow lib/surface-delegate.mjs.

import { resolve } from 'node:path';
import { REPO_ROOT, runSurfaceDelegate } from './lib/surface-delegate.mjs';
import process from 'node:process';

runSurfaceDelegate({
  surface: 'aps',
  command: process.execPath,
  args: [resolve(REPO_ROOT, 'scripts/aps/drift-check.mjs')],
  isolate: 'node scripts/aps/drift-check.mjs',
});
