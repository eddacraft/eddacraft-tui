#!/usr/bin/env node
// Surface delegate: ADR integrity.
//
// Invokes scripts/docs/adr-integrity.sh directly so the `pnpm docs:check`
// orchestrator can include ADR integrity in its labelled summary without
// duplicating logic. It deliberately does NOT re-enter the package manager
// (`pnpm adr:check`): that round-trip made this surface report a docs content
// `FAIL` whenever the package manager itself was broken, on a corpus that was
// clean (CIB-278). Exit codes follow lib/surface-delegate.mjs.

import { resolve } from 'node:path';
import { REPO_ROOT, runSurfaceDelegate } from './lib/surface-delegate.mjs';

runSurfaceDelegate({
  surface: 'adr',
  command: 'bash',
  args: [resolve(REPO_ROOT, 'scripts/docs/adr-integrity.sh')],
  isolate: 'bash scripts/docs/adr-integrity.sh',
});
