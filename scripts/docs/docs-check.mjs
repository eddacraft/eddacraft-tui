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
//   public-docs       — scripts/docs/check-public-docs.mjs (real)
//   aps               — scripts/docs/check-aps.mjs (delegates to scripts/aps/drift-check.mjs)
//   adr               — scripts/docs/check-adr.mjs (delegates to scripts/docs/adr-integrity.sh)
//   index-freshness   — scripts/docs/check-index-freshness.mjs (real)
//   asbuilt-paths     — scripts/docs/check-asbuilt-paths.mjs (stub, DOCGOV-006)
//   retired-claims    — scripts/docs/check-retired-claims.mjs (real)
//
// Verdicts (CIB-278): a surface that could not RUN is reported as
// `ERROR (tooling)`, never as `FAIL`. Collapsing both into `FAIL` told
// contributors their docs change had broken a surface when the real cause was
// a broken toolchain and the corpus was clean. See lib/surface-delegate.mjs for
// the shared taxonomy. The process still exits non-zero either way — a tooling
// failure is misattributed, not silenced.
//
// Baseline: each real-surface script reads the same
// docs/governance/docs-check.baseline.json file. `--update-baseline` regenerates
// the baseline from the current corpus by running every surface in --json mode,
// collecting their findings, and writing the combined JSON. Existing baseline
// entries for surfaces that did not run are preserved unchanged.

import { spawnSync } from 'node:child_process';
import { writeFileSync, mkdirSync, readFileSync, existsSync, renameSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import process from 'node:process';
import {
  EXIT_CONTENT_FAILURE,
  EXIT_PASS,
  EXIT_TOOLING_FAILURE,
  classify,
} from './lib/surface-delegate.mjs';

const DEFAULT_SURFACES = [
  { name: 'metadata', script: 'scripts/docs/check-metadata.mjs', baselineable: true },
  { name: 'tags', script: 'scripts/docs/check-tags.mjs', baselineable: true },
  { name: 'links', script: 'scripts/docs/check-links.mjs', baselineable: true },
  {
    name: 'public-docs',
    script: 'scripts/docs/check-public-docs.mjs',
    baselineable: false,
  },
  { name: 'aps', script: 'scripts/docs/check-aps.mjs', baselineable: false },
  { name: 'adr', script: 'scripts/docs/check-adr.mjs', baselineable: false },
  {
    name: 'index-freshness',
    script: 'scripts/docs/check-index-freshness.mjs',
    baselineable: false,
  },
  { name: 'asbuilt-paths', script: 'scripts/docs/check-asbuilt-paths.mjs', baselineable: true },
  { name: 'release-plan', script: 'scripts/docs/check-release-plan.mjs', baselineable: false },
  {
    // Tombstone lint: retired product claims must not survive on other
    // surfaces or be reintroduced (see scripts/docs/retired-claims.mjs).
    // Carries its own inline baseline (survivors owned by open CIB items),
    // so it does not participate in docs-check.baseline.json.
    name: 'retired-claims',
    script: 'scripts/docs/check-retired-claims.mjs',
    baselineable: false,
  },
];

// Declared above the top-level runAll()/regenerateBaseline() call below, which
// would otherwise hit these in the temporal dead zone.
const VERDICT_LABEL = {
  [EXIT_PASS]: 'pass',
  [EXIT_CONTENT_FAILURE]: 'FAIL',
  [EXIT_TOOLING_FAILURE]: 'ERROR (tooling)',
};
const LABEL_WIDTH = Math.max(...Object.values(VERDICT_LABEL).map((l) => l.length));

const argv = process.argv.slice(2);
const args = new Set(argv);

// Test seam: --root and --surfaces let the fixture-level contract tests drive
// regenerateBaseline() against stub surface scripts without touching the live
// corpus or the tracked baseline. Neither flag is used by the CI invocation.
function flagValue(name) {
  const idx = argv.indexOf(name);
  return idx !== -1 && idx + 1 < argv.length ? argv[idx + 1] : undefined;
}

const rootOverride = flagValue('--root');
const REPO_ROOT = rootOverride
  ? resolve(rootOverride)
  : fileURLToPath(new URL('../..', import.meta.url));

const surfacesOverride = flagValue('--surfaces');
const SURFACES = surfacesOverride
  ? JSON.parse(readFileSync(resolve(surfacesOverride), 'utf8'))
  : DEFAULT_SURFACES;

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
  // A real content defect outranks a broken tool: if any surface actually ran
  // and failed, exit 1 so a tooling problem elsewhere can never mask it.
  if (results.some((r) => r.verdict === EXIT_CONTENT_FAILURE)) process.exit(EXIT_CONTENT_FAILURE);
  if (results.some((r) => r.verdict === EXIT_TOOLING_FAILURE)) process.exit(EXIT_TOOLING_FAILURE);
  process.exit(EXIT_PASS);
}

function runSurface(surface, { json, forceNoBaseline }) {
  const scriptPath = resolve(REPO_ROOT, surface.script);
  const childArgv = [scriptPath];
  if (json) childArgv.push('--json');
  // The --no-baseline flag is only meaningful for surfaces that actually read
  // the baseline (surface.baselineable). Non-baselineable surfaces such as
  // index-freshness forward their argv straight to a generator (docs-index.mjs)
  // whose strict parseArgs rejects the unknown flag and crashes — so we must NOT
  // append baseline flags to them.
  //
  // When regenerating the baseline we must NOT apply the existing one — otherwise
  // previously-baselined errors are downgraded to WARN and dropped by
  // collapseFindings(), so the baseline can only shrink. forceNoBaseline lets
  // regenerateBaseline() bypass the user's --no-baseline flag (which is itself
  // only meaningful for a normal run).
  if (surface.baselineable) {
    if (json && forceNoBaseline) childArgv.push('--no-baseline');
    else if (noBaseline) childArgv.push('--no-baseline');
  }

  const result = spawnSync('node', childArgv, {
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
    verdict: classify(result),
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
  const unrunnable = [];
  for (const r of results) {
    if (r.verdict === EXIT_PASS) passed += 1;
    else if (r.verdict === EXIT_TOOLING_FAILURE) unrunnable.push(r.surface);
    else failed += 1;
    process.stdout.write(`  ${VERDICT_LABEL[r.verdict].padEnd(LABEL_WIDTH)} ${r.surface}\n`);
  }
  // Name the unrunnable count in the tally itself, so `8/9 passed; 0 failed`
  // does not read as arithmetic that does not add up.
  const unrunnableTally = unrunnable.length > 0 ? `; ${unrunnable.length} could not run` : '';
  process.stdout.write(
    `[docs-check] ${passed}/${results.length} surfaces passed; ${failed} failed${unrunnableTally}.\n`
  );
  if (unrunnable.length > 0) {
    // Say plainly that these carry no content signal. The failure mode this
    // guards against is a contributor reading `FAIL aps` and going looking for
    // a defect in their own docs change that was never there (CIB-278).
    process.stdout.write(
      `[docs-check] ${unrunnable.length} surface(s) could not run: ${unrunnable.join(', ')} — ` +
        `this is a tooling failure, not a docs content defect. ` +
        `See the labelled output above for the underlying error and remedy.\n`
    );
  }
}

async function regenerateBaseline() {
  process.stdout.write('[docs-check] regenerating baseline from current corpus...\n');
  // Start from an empty baseline for the surfaces we are about to regenerate.
  // Preserve entries for non-baselineable surfaces if they somehow exist (none
  // do today). We never read the existing baseline for baselineable surfaces —
  // that would be self-referential and would let absorbed errors silently drop
  // out of scope on regeneration.
  const next = {};
  const failures = [];

  for (const surface of SURFACES) {
    if (!surface.baselineable) continue;
    process.stdout.write(`[docs-check] regenerating ${surface.name}...\n`);
    const result = runSurface(surface, { json: true, forceNoBaseline: true });
    if (result.verdict === EXIT_TOOLING_FAILURE) {
      process.stderr.write(
        `[docs-check] ${surface.name}: ERROR (tooling) — JSON output rejected\n`
      );
      failures.push(surface.name);
      continue;
    }
    if (!result.stdout.trim()) {
      process.stderr.write(`[docs-check] ${surface.name}: no JSON output\n`);
      failures.push(surface.name);
      continue;
    }
    let parsed;
    try {
      parsed = JSON.parse(result.stdout);
    } catch (err) {
      process.stderr.write(`[docs-check] ${surface.name}: JSON parse failed (${err.message})\n`);
      failures.push(surface.name);
      continue;
    }
    next[surface.name] = collapseFindings(parsed.findings ?? []);
    process.stdout.write(
      `[docs-check] ${surface.name}: ${parsed.summary?.errors ?? '?'} errors, ` +
        `${parsed.summary?.warnings ?? '?'} warnings, ` +
        `${parsed.summary?.filesChecked ?? '?'} files\n`
    );
  }

  // Data-loss guard (DOCGOV-012): a partial run must NEVER overwrite the tracked
  // baseline. If any baselineable surface failed to produce valid JSON we leave
  // the existing baseline untouched and exit non-zero, so the failure is loud
  // and the operator's known-good entries are preserved.
  if (failures.length > 0) {
    process.stderr.write(
      `[docs-check] baseline NOT written — ${failures.length} surface(s) failed: ` +
        `${failures.join(', ')}. Existing baseline left unchanged.\n`
    );
    process.exit(1);
  }

  // Carry forward existing entries for non-baselineable surfaces (the
  // regeneration loop only repopulates baselineable ones) so an unrelated key
  // is not silently dropped.
  if (existsSync(baselinePath)) {
    try {
      const existing = JSON.parse(readFileSync(baselinePath, 'utf8'));
      for (const [key, value] of Object.entries(existing)) {
        if (!(key in next)) next[key] = value;
      }
    } catch (err) {
      process.stderr.write(
        `[docs-check] could not read existing baseline for carry-forward (${err.message})\n`
      );
    }
  }

  // mkdirSync with recursive:true is idempotent and atomic — no need for the
  // race-prone existsSync-then-mkdir guard CodeQL flagged.
  mkdirSync(dirname(baselinePath), { recursive: true });
  // Write to a sibling temp file then rename, so a crash mid-serialize cannot
  // leave a truncated baseline behind.
  const tmpPath = `${baselinePath}.tmp-${process.pid}`;
  writeFileSync(tmpPath, JSON.stringify(next, null, 2) + '\n');
  renameSync(tmpPath, baselinePath);
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
