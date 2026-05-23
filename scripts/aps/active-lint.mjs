#!/usr/bin/env node
import { existsSync, readdirSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import process from 'node:process';

const args = process.argv.slice(2);

function usage() {
  console.log(`Usage: scripts/aps/active-lint.mjs [--root PATH] [--aps-bin PATH] [--list-files] [--json]

Runs canonical APS lint against active APS surfaces only. Archive and legacy
planning history is intentionally excluded from the default scope.`);
}

let root = process.cwd();
let apsBin = process.env.APS_CANONICAL_BIN || 'aps';
let listFiles = false;
let json = false;

for (let i = 0; i < args.length; i += 1) {
  const arg = args[i];
  if (arg === '--help' || arg === '-h') {
    usage();
    process.exit(0);
  }
  if (arg === '--list-files') {
    listFiles = true;
    continue;
  }
  if (arg === '--json') {
    json = true;
    continue;
  }
  if (arg === '--root' || arg === '--aps-bin') {
    const value = args[i + 1];
    if (value === undefined || value.startsWith('--')) {
      console.error(`active-lint.mjs: ${arg} requires a value`);
      process.exit(2);
    }
    if (arg === '--root') root = value;
    if (arg === '--aps-bin') apsBin = value;
    i += 1;
    continue;
  }
  console.error(`active-lint.mjs: unknown argument: ${arg}`);
  process.exit(2);
}

root = resolve(root);
const files = activeApsFiles(root);

if (listFiles) {
  if (json) {
    console.log(JSON.stringify({ files }, null, 2));
  } else {
    console.log(files.join('\n'));
  }
  process.exit(0);
}

if (files.length === 0) {
  if (json) console.log(JSON.stringify({ files: [], status: 0 }, null, 2));
  else console.log('[aps-active-lint] no active APS files found');
  process.exit(0);
}

const result = spawnSync(apsBin, ['lint', ...files], {
  cwd: root,
  stdio: json ? ['ignore', 'pipe', 'pipe'] : 'inherit',
  encoding: 'utf8',
});
const exitStatus = result.error ? 2 : (result.status ?? 1);

if (json) {
  console.log(
    JSON.stringify(
      {
        files,
        status: exitStatus,
        stdout: result.stdout ?? '',
        stderr: result.stderr ?? '',
        error: result.error?.message,
      },
      null,
      2
    )
  );
}

if (result.error) {
  if (!json) console.error(`[aps-active-lint] failed to invoke ${apsBin}: ${result.error.message}`);
  process.exit(exitStatus);
}

process.exit(exitStatus);

function activeApsFiles(projectRoot) {
  const plans = join(projectRoot, 'plans');
  const candidates = [join(plans, 'index.aps.md'), join(plans, 'issues.md')];
  candidates.push(...walk(join(plans, 'modules'), (name) => name.endsWith('.aps.md')));
  candidates.push(...walk(join(plans, 'execution'), (name) => name.endsWith('.actions.md')));
  return candidates
    .filter((path) => existsSync(path))
    .map((path) => relative(projectRoot, path).replaceAll('\\', '/'))
    .sort();
}

function walk(dir, includeFile) {
  if (!existsSync(dir)) return [];
  const files = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...walk(path, includeFile));
    } else if (entry.isFile() && includeFile(entry.name)) {
      files.push(path);
    }
  }
  return files;
}
