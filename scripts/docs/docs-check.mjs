#!/usr/bin/env node
// docs-check orchestrator (DOCGOV-005).
//
// Runs every documentation validation surface as a child process, prefixes
// their output with the surface label, prints a unified summary, and exits
// non-zero if any surface failed. ADR-042 names this command as a closeout-
// enforcement check, which is why it exits non-zero by design despite
// ADR-002's "warnings exit 0" default.
//
// Surfaces:
//   metadata          — scripts/docs/check-metadata.mjs (real)
//   tags              — scripts/docs/check-tags.mjs (real)
//   links             — scripts/docs/check-links.mjs (real)
//   aps               — scripts/docs/check-aps.mjs (delegates to pnpm aps:drift)
//   adr               — scripts/docs/check-adr.mjs (delegates to pnpm adr:check)
//   index-freshness   — scripts/docs/check-index-freshness.mjs (stub, DOCGOV-007)
//   asbuilt-paths     — scripts/docs/check-asbuilt-paths.mjs (stub, DOCGOV-006)
//
// Baseline: each real-surface script reads the same
// docs/governance/docs-check.baseline.json file. `--update-baseline` regenerates
// the baseline from the current corpus by running every surface in --json mode,
// collecting their findings, and writing the combined JSON. Existing baseline
// entries for surfaces that did not run are preserved unchanged.

import { spawnSync } from 'node:child_process';
import { writeFileSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import process from 'node:process';

const REPO_ROOT = fileURLToPath(new URL('../..', import.meta.url));
const SURFACES = [
  { name: 'metadata', script: 'scripts/docs/check-metadata.mjs', baselineable: true },
  { name: 'tags', script: 'scripts/docs/check-tags.mjs', baselineable: true },
  { name: 'links', script: 'scripts/docs/check-links.mjs', baselineable: true },
  { name: 'aps', script: 'scripts/docs/check-aps.mjs', baselineable: false },
  { name: 'adr', script: 'scripts/docs/check-adr.mjs', baselineable: false },
  {
    name: 'index-freshness',
    script: 'scripts/docs/check-index-freshness.mjs',
    baselineable: false,
  },
  { name: 'asbuilt-paths', script: 'scripts/docs/check-asbuilt-paths.mjs', baselineable: true },
];

const args = new Set(process.argv.slice(2));
const updateBaseline = args.has('--update-baseline');
const noBaseline = args.has('--no-baseline');
const baselinePath = resolve(REPO_ROOT, 'docs/governance/docs-check.baseline.json');

if (updateBaseline) {
  await regenerateBaseline();
} else {
  await runAll();
}

async function runAll() {
  const results = [];
  for (const surface of SURFACES) {
    const result = runSurface(surface, { json: false, forceNoBaseline: false });
    results.push({ surface: surface.name, ...result });
  }
  printSummary(results);
  const anyError = results.some((r) => r.status !== 0);
  process.exit(anyError ? 1 : 0);
}

function runSurface(surface, { json, forceNoBaseline }) {
  const scriptPath = resolve(REPO_ROOT, surface.script);
  const argv = [scriptPath];
  if (json) argv.push('--json');
  // When regenerating the baseline we must NOT apply the existing one — otherwise
  // previously-baselined errors are downgraded to WARN and dropped by
  // collapseFindings(), so the baseline can only shrink. forceNoBaseline lets
  // regenerateBaseline() bypass the user's --no-baseline flag (which is itself
  // only meaningful for a normal run).
  if (json && forceNoBaseline) argv.push('--no-baseline');
  else if (noBaseline) argv.push('--no-baseline');

  const result = spawnSync('node', argv, {
    cwd: REPO_ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    maxBuffer: 64 * 1024 * 1024,
  });

  if (!json) {
    const stdout = (result.stdout ?? '').trimEnd();
    if (stdout) process.stdout.write(stdout + '\n');
    const stderr = (result.stderr ?? '').trimEnd();
    if (stderr) process.stderr.write(stderr + '\n');
  }
  return {
    status: result.status ?? 1,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
    error: result.error?.message,
  };
}

function printSummary(results) {
  process.stdout.write('\n');
  process.stdout.write('[docs-check] summary:\n');
  let passed = 0;
  let failed = 0;
  for (const r of results) {
    const verdict = r.status === 0 ? 'pass' : 'FAIL';
    if (r.status === 0) passed += 1;
    else failed += 1;
    process.stdout.write(`  ${verdict.padEnd(4)} ${r.surface}\n`);
  }
  process.stdout.write(
    `[docs-check] ${passed}/${results.length} surfaces passed; ${failed} failed.\n`
  );
}

async function regenerateBaseline() {
  process.stdout.write('[docs-check] regenerating baseline from current corpus...\n');
  // Start from an empty baseline for the surfaces we are about to regenerate.
  // Preserve entries for non-baselineable surfaces if they somehow exist (none
  // do today). We never read the existing baseline for baselineable surfaces —
  // that would be self-referential and would let absorbed errors silently drop
  // out of scope on regeneration.
  const next = {};

  for (const surface of SURFACES) {
    if (!surface.baselineable) continue;
    process.stdout.write(`[docs-check] regenerating ${surface.name}...\n`);
    const result = runSurface(surface, { json: true, forceNoBaseline: true });
    if (!result.stdout.trim()) {
      process.stderr.write(`[docs-check] ${surface.name}: no JSON output\n`);
      continue;
    }
    let parsed;
    try {
      parsed = JSON.parse(result.stdout);
    } catch (err) {
      process.stderr.write(`[docs-check] ${surface.name}: JSON parse failed (${err.message})\n`);
      continue;
    }
    next[surface.name] = collapseFindings(parsed.findings ?? []);
    process.stdout.write(
      `[docs-check] ${surface.name}: ${parsed.summary?.errors ?? '?'} errors, ` +
        `${parsed.summary?.warnings ?? '?'} warnings, ` +
        `${parsed.summary?.filesChecked ?? '?'} files\n`
    );
  }

  // mkdirSync with recursive:true is idempotent and atomic — no need for the
  // race-prone existsSync-then-mkdir guard CodeQL flagged.
  mkdirSync(dirname(baselinePath), { recursive: true });
  writeFileSync(baselinePath, JSON.stringify(next, null, 2) + '\n');
  process.stdout.write(`[docs-check] baseline written to ${baselinePath}\n`);
}

function collapseFindings(findings) {
  const byFile = {};
  for (const f of findings) {
    if (f.severity !== 'ERROR') continue;
    if (!byFile[f.file]) byFile[f.file] = [];
    if (!byFile[f.file].includes(f.message)) byFile[f.file].push(f.message);
  }
  for (const k of Object.keys(byFile)) byFile[k].sort();
  return byFile;
}
