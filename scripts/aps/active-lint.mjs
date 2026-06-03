#!/usr/bin/env node
import { existsSync, readdirSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import process from 'node:process';

// Files under plans/modules/ that are intentionally NOT canonical work-item
// modules and are therefore excluded from canonical `aps lint`. Keep this list
// tiny and justified — each entry is a different document *type*, not a module
// that merely needs migrating.
//
// Currently empty: the v0.7.0-beta clawpatch release-findings tracker that
// previously lived here was archived to plans/archive/modules/ (CIB-039), which
// removes it from the active walk scope, so its carve-out is no longer needed.
const NON_CANONICAL_MODULES = new Set();

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
  if (json) console.log(JSON.stringify({ files: [], status: 0, results: [] }, null, 2));
  else console.log('[aps-active-lint] no active APS files found');
  process.exit(0);
}

// CIB-037: canonical `aps lint` only honours its final path argument, so passing
// the whole active set in a single invocation silently validated one file (the
// last). Lint each file in its own invocation and aggregate the results so every
// active APS surface is actually checked.
const results = [];
let spawnError;
for (const file of files) {
  // maxBuffer: per-file output is small, but a pathologically large lint report
  // must not overflow the default 1 MiB buffer and be misreported as a spawn
  // failure (which would surface as exit 2 below).
  const r = spawnSync(apsBin, ['lint', file], {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  });
  if (r.error) {
    spawnError = r.error;
    results.push({ file, status: 2, stdout: '', stderr: '', error: r.error.message });
    break;
  }
  results.push({ file, status: r.status ?? 1, stdout: r.stdout ?? '', stderr: r.stderr ?? '' });
}

const exitStatus = aggregateStatus(results, spawnError);

if (json) {
  console.log(
    JSON.stringify(
      {
        files,
        status: exitStatus,
        results,
        stdout: results.map((r) => r.stdout).join(''),
        stderr: results.map((r) => r.stderr).join(''),
        error: spawnError?.message,
      },
      null,
      2
    )
  );
  process.exit(exitStatus);
}

let withFindings = 0;
for (const r of results) {
  const failed = r.status !== 0 || r.stderr.trim() !== '';
  if (!failed) continue;
  withFindings += 1;
  const out = `${r.stdout}${r.stderr}`.trim();
  if (out) console.log(out);
}

if (spawnError) {
  console.error(`[aps-active-lint] failed to invoke ${apsBin}: ${spawnError.message}`);
  process.exit(exitStatus);
}

if (exitStatus === 0) {
  console.log(`[aps-active-lint] ${results.length} files checked, all clean`);
} else {
  console.log(`[aps-active-lint] ${results.length} files checked, ${withFindings} with findings`);
}
process.exit(exitStatus);

// Aggregate per-file exit codes into a single status: 2 if any invocation failed
// to spawn; 0 if every file linted clean; the shared code when all failures
// returned the same non-zero status (preserves a single file's exact code); 1
// otherwise.
function aggregateStatus(perFile, hadSpawnError) {
  if (hadSpawnError) return 2;
  const failedCodes = perFile.filter((r) => r.status !== 0).map((r) => r.status);
  if (failedCodes.length === 0) return 0;
  const distinct = new Set(failedCodes);
  return distinct.size === 1 ? [...distinct][0] : 1;
}

function activeApsFiles(projectRoot) {
  const plans = join(projectRoot, 'plans');
  const candidates = [join(plans, 'index.aps.md'), join(plans, 'issues.md')];
  candidates.push(...walk(join(plans, 'modules'), (name) => name.endsWith('.aps.md')));
  candidates.push(...walk(join(plans, 'execution'), (name) => name.endsWith('.actions.md')));
  return candidates
    .filter((path) => existsSync(path))
    .map((path) => relative(projectRoot, path).replaceAll('\\', '/'))
    .filter((rel) => !NON_CANONICAL_MODULES.has(rel))
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
