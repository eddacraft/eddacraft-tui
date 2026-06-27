#!/usr/bin/env node
// Derive APS progress counts from work-item statuses (CIB-022).
//
// The source of truth is each module's work items (`### ID-NNN:` headings and
// their `- **Status:**` line). This script computes `done/total` from those and
// keeps two derived surfaces in sync:
//   1. the module header row in `plans/modules/<slug>.aps.md`
//   2. the module's row in `plans/index.aps.md`
//
// Counts are advisory-derived (ADR-053): feature PRs flip only per-item Status
// lines; a single-writer reconcile (`pnpm aps:index`) refreshes stored N/M.
// `--check` (CI) recomputes and reports freshness drift but exits 0 so
// concurrent same-module PRs do not collide on the aggregate count cell.
//
// Scope: only active modules under `plans/modules/` that declare a
// `| ID | … | N/M |` progress header. Index rows pointing at
// `plans/archive/modules/` are frozen (no live source) and never touched. Only
// the `N/M` token of the row's dedicated count cell is rewritten — annotation
// prose, other columns, and table padding are preserved.

import { existsSync, writeFileSync } from 'node:fs';
import { basename, join, relative } from 'node:path';
import { parseArgs } from 'node:util';
import { extractModule, isDoneStatus, listModulePaths, readText } from './lib/modules.mjs';

const { values } = parseArgs({
  options: {
    root: { type: 'string' },
    check: { type: 'boolean', default: false },
    json: { type: 'boolean', default: false },
  },
  allowPositionals: false,
});

const root = values.root ?? process.cwd();

/** Rewrite the trailing `N/M` cell of a module header line, preserving padding. */
function rewriteHeaderLine(headerLine, done, total) {
  return headerLine.replace(/(\d+)\/(\d+)(\s*\|\s*)$/, `${done}/${total}$3`);
}

/**
 * Locate a module's index row and the cell that holds its count.
 *
 * A row belongs to the module only if the module link is in the row's NAME
 * cell (the first `| … |` cell). Two traps this avoids — both live in the real
 * index — are:
 *   - another row's PROSE linking to this module (e.g. "split into
 *     [multilayer-protection-v2](…)") would hijack a match made anywhere on the
 *     line; and
 *   - a row whose count lives inside prose (e.g. "…infrastructure (6/38
 *     complete)") rather than a dedicated cell — rewriting the first `N/M` on
 *     the line would corrupt that prose.
 * So we match the name-cell link, then take the count from the first cell whose
 * trimmed text *starts with* `N/M`. Rows with no such cell return `cell: -1`
 * (left untouched, surfaced as a note); archived rows never match.
 */
function findIndexRow(lines, slug) {
  const suffix = `modules/${slug}.aps.md`;
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (!line.startsWith('|')) continue;
    const cells = line.split('|');
    const link = (cells[1] ?? '').match(/\]\(([^)]+)\)/);
    if (!link || !link[1].endsWith(suffix) || link[1].includes('/archive/')) continue;
    for (let c = 2; c < cells.length; c += 1) {
      const m = cells[c].match(/^\s*(\d+)\/(\d+)\b/);
      if (m) return { index: i, cell: c, actual: `${m[1]}/${m[2]}` };
    }
    return { index: i, cell: -1, actual: null };
  }
  return { index: -1, cell: -1, actual: null };
}

// Managed scope = modules that declare a `| ID | … | N/M |` progress header AND
// have at least one work item. Headerless modules (e.g. `| ID | Owner |
// Status |`, or item-listing tables) carry hand-curated *planned* totals in the
// index that are not item-derivable, so — like drift-check — we leave them
// untouched.
const modules = listModulePaths(root)
  .map(extractModule)
  .filter((module) => module.headerMatch && module.items.length > 0);

// Per-file pending content (path → { before, after }) and a flat drift list.
const pending = new Map();
const drifts = [];
const notes = [];

function stage(path, before, after) {
  pending.set(path, { before, after });
}

// 1. Module headers — replace by position so a non-unique header line can't be
//    mis-targeted by a plain string replace.
for (const module of modules) {
  const done = module.items.filter((item) => isDoneStatus(item.status)).length;
  const total = module.items.length;
  module.derived = { done, total };
  const text = readText(module.path);
  const oldLine = module.headerMatch[0];
  const newLine = rewriteHeaderLine(oldLine, done, total);
  if (newLine !== oldLine) {
    drifts.push({
      file: relative(root, module.path),
      module: module.id,
      surface: 'module header',
      actual: `${module.progressDone}/${module.progressTotal}`,
      expected: `${done}/${total}`,
    });
    const at = module.headerMatch.index;
    stage(module.path, text, text.slice(0, at) + newLine + text.slice(at + oldLine.length));
  }
}

// 2. Index rows — single file, all module replacements applied in one pass.
const indexPath = join(root, 'plans/index.aps.md');
if (existsSync(indexPath)) {
  const before = readText(indexPath);
  const lines = before.split('\n');
  for (const module of modules) {
    const { done, total } = module.derived;
    const expected = `${done}/${total}`;
    const { index, cell, actual } = findIndexRow(lines, basename(module.path, '.aps.md'));
    if (index === -1) {
      notes.push(`${module.id}: active module has no row in plans/index.aps.md.`);
      continue;
    }
    if (cell === -1) {
      notes.push(`${module.id}: index row has no dedicated N/M count cell; count not managed.`);
      continue;
    }
    if (actual !== expected) {
      drifts.push({
        file: 'plans/index.aps.md',
        module: module.id,
        surface: 'index',
        actual,
        expected,
      });
      const cells = lines[index].split('|');
      cells[cell] = cells[cell].replace(/^(\s*)\d+\/\d+\b/, `$1${expected}`);
      lines[index] = cells.join('|');
    }
  }
  const after = lines.join('\n');
  if (after !== before) stage(indexPath, before, after);
}

// Apply or report. Freshness drift is advisory in --check mode (ADR-053).
let exitCode = 0;
if (!values.check) {
  for (const [path, { after }] of pending) {
    writeFileSync(path, after, 'utf8');
  }
}

const summary = {
  modules: modules.length,
  drifts: drifts.length,
  mode: values.check ? 'check' : 'write',
};

if (values.json) {
  process.stdout.write(
    `${JSON.stringify({ surface: 'aps-index-counts', drifts, notes, summary }, null, 2)}\n`
  );
} else {
  if (drifts.length === 0) {
    process.stdout.write(
      values.check
        ? '[aps-index-counts] ok: all module headers and index counts match work-item statuses\n'
        : '[aps-index-counts] ok: nothing to update\n'
    );
  } else {
    for (const d of drifts) {
      const verb = values.check ? 'is' : 'updated';
      process.stdout.write(
        `[aps-index-counts] ${d.module} ${d.surface} ${verb} ${d.actual} — work items count ${d.expected}\n`
      );
    }
    if (values.check) {
      process.stdout.write(
        '[aps-index-counts] advisory: run `pnpm aps:index` to reconcile stored counts.\n'
      );
    }
  }
  // Notes are informational (managed module not in the index, or no count
  // cell). They never affect the exit code, so keep them off stdout.
  for (const note of notes) process.stderr.write(`[aps-index-counts] note: ${note}\n`);
}

process.exit(exitCode);
