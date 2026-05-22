#!/usr/bin/env node
// Surface validator: generated-index freshness.

import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const scriptPath = fileURLToPath(new URL('./docs-index.mjs', import.meta.url));
const result = spawnSync('node', [scriptPath, '--check', ...process.argv.slice(2)], {
  stdio: 'inherit',
});

process.exit(result.status ?? 1);
