#!/usr/bin/env node
// Advance Merged APS work items to Released/Shipped at release time (#1715).
//
// Given a published release record (`plans/releases/<tag>.md`) plus the tag,
// commit sha, and date, this walks every item in the record's `aps.items[]`
// and rewrites its module work-item `- **Status:** Merged …` line to
// `- **Status:** Released/Shipped via <tag> (<8-char-sha> · <date>)`.
//
// It replaces the manual `awk | jq | perl` walk in the release runbook §13.
// Two ways it is more robust than that walk:
//   1. Items are located by **heading search across all module files**
//      (IDs are unique) rather than guessing `plans/modules/<module>.aps.md`
//      from the record's `module` field — that field is the ID *prefix*
//      (e.g. "INTL"), not the file slug ("intent-ledger-governance"), so the
//      runbook's filename guess MISSed every such item.
//   2. It matches both `###` and `####` headings, so the nested-group modules
//      (e.g. multilayer-protection-v2 / MLP2-*) are advanced too — the perl
//      walk hard-coded `####` and silently skipped normal `###` modules.
//
// Exit code: 0 when every item was advanced or idempotently skipped (already
// Released/Shipped or Complete); 1 when any item MISSes (heading not found in
// any module, no Status line, or a status that is not yet Merged — the record
// claims it shipped but it never reached Merged).
//
// Merged and Released/Shipped are both "done" (DONE_PATTERNS in
// lib/modules.mjs), so the module done/total counters do not change; this
// script does not touch them. Run `aps:index --check` afterwards to confirm.

import { existsSync, readdirSync, writeFileSync } from 'node:fs';
import { isAbsolute, join } from 'node:path';
import { parseArgs } from 'node:util';
import { listModulePaths, readText } from './lib/modules.mjs';

const { values } = parseArgs({
  options: {
    'release-record': { type: 'string' },
    tag: { type: 'string' },
    sha: { type: 'string' },
    date: { type: 'string' },
    root: { type: 'string' },
    'dry-run': { type: 'boolean', default: false },
    json: { type: 'boolean', default: false },
  },
  allowPositionals: false,
});

const root = values.root ?? process.cwd();
const dryRun = values['dry-run'];

function die(message) {
  console.error(`[advance-released] error: ${message}`);
  process.exit(2);
}

for (const required of ['release-record', 'tag', 'sha', 'date']) {
  if (!values[required]) {
    die(`--${required} is required`);
  }
}

// These three are interpolated verbatim into the rewritten Status line, so
// validate their shape — a newline or stray char in `--tag`/`--sha`/`--date`
// (e.g. from an unvalidated CI variable) would otherwise split or corrupt the
// line in every module file written.
const tag = values.tag;
if (!/^[A-Za-z0-9][A-Za-z0-9._/+-]*$/.test(tag)) {
  die(`--tag has an unexpected shape: ${JSON.stringify(tag)}`);
}
if (!/^[0-9a-fA-F]{7,40}$/.test(values.sha)) {
  die(`--sha must be a hex commit sha (7-40 chars): ${JSON.stringify(values.sha)}`);
}
if (!/^\d{4}-\d{2}-\d{2}$/.test(values.date)) {
  die(`--date must be YYYY-MM-DD: ${JSON.stringify(values.date)}`);
}
const sha8 = values.sha.slice(0, 8);
const date = values.date;
const newStatus = `- **Status:** Released/Shipped via ${tag} (${sha8} · ${date})`;

// ── Parse the release record's aps.items[] ──────────────────────────────────
// The record is markdown with a fenced ```json block; a future sibling `.json`
// record would parse directly. Take the first fenced JSON block that carries
// `aps.items`.
function extractReleaseItems(recordPath) {
  if (!existsSync(recordPath)) {
    die(`release record not found: ${recordPath}`);
  }
  const text = readText(recordPath);
  const blocks = [...text.matchAll(/```json\s*\n([\s\S]*?)\n```/g)].map((m) => m[1]);
  // Fall back to treating the whole file as JSON (sibling `.json` record).
  const candidates = blocks.length > 0 ? blocks : [text];
  for (const raw of candidates) {
    let parsed;
    try {
      parsed = JSON.parse(raw);
    } catch {
      continue;
    }
    if (Array.isArray(parsed?.aps?.items)) {
      return { items: parsed.aps.items, lifecycleState: parsed.lifecycleState };
    }
  }
  die(`no JSON \`aps.items[]\` found in ${recordPath}`);
  return { items: [], lifecycleState: undefined };
}

// ── Index every work-item heading across module files ───────────────────────
// Matches both `### ID:` and `#### ID:` (and `ID — title` / `ID - title`).
// Maps id -> { path, lines, headingLine } so a later edit can rewrite in place.
const headingRe = /^(#{3,4})\s+([A-Z][A-Z0-9]*-\d{3}[a-z]?)(?::|\s+[—-])/;
// Block boundary = the next section/group/item heading (`##`–`####`). Depth-5/6
// sub-headings inside an item body do NOT terminate the block, so a Status line
// after such a sub-section is still found (the Status bullet normally leads the
// block, but this keeps the scan robust to richer item bodies). (Council AR-002)
const anyHeadingRe = /^#{2,4}\s/;

// Archived modules are scanned too: a release record's items may belong to a
// module that has since been archived (Complete → archived). Such items are
// already done, so they resolve to SKIP rather than a false MISS — and a
// re-run against a historical record stays clean. Archive files are frozen,
// so a Merged item found there is treated as an anomaly, never rewritten.
function listArchivePaths() {
  const dir = join(root, 'plans/archive/modules');
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((name) => name.endsWith('.aps.md'))
    .sort()
    .map((name) => join(dir, name));
}

function buildHeadingIndex() {
  /** @type {Map<string, {path: string, headingLine: number, archived: boolean}>} */
  const index = new Map();
  /** @type {Map<string, string[]>} cached, mutable line arrays per file */
  const fileLines = new Map();
  const scan = (paths, archived) => {
    for (const path of paths) {
      const lines = readText(path).split('\n');
      fileLines.set(path, lines);
      lines.forEach((line, i) => {
        const m = line.match(headingRe);
        if (m && !index.has(m[2])) {
          index.set(m[2], { path, headingLine: i, archived });
        }
      });
    }
  };
  scan(listModulePaths(root), false);
  scan(listArchivePaths(), true);
  return { index, fileLines };
}

// Resolve a relative --release-record against `root` (not the caller's CWD),
// so it stays consistent with module discovery when --root/--repo points at a
// different tree than the working directory.
const recordArg = values['release-record'];
const recordPath = isAbsolute(recordArg) ? recordArg : join(root, recordArg);
const { items, lifecycleState } = extractReleaseItems(recordPath);
const { index, fileLines } = buildHeadingIndex();

const results = []; // { id, action: advanced|skipped|miss, reason }
const dirtyFiles = new Set();

for (const item of items) {
  const id = item.id;
  if (typeof id !== 'string' || id === '') {
    results.push({ id: '<missing>', action: 'miss', reason: 'release item has no string `id`' });
    continue;
  }
  const loc = index.get(id);
  if (!loc) {
    results.push({ id, action: 'miss', reason: 'heading not found in any module file' });
    continue;
  }
  const lines = fileLines.get(loc.path);
  // Find the first Status line in this item's block (heading → next heading).
  let statusLine = -1;
  for (let i = loc.headingLine + 1; i < lines.length; i += 1) {
    if (anyHeadingRe.test(lines[i])) break;
    if (/^- \*\*Status:\*\*\s+/.test(lines[i])) {
      statusLine = i;
      break;
    }
  }
  if (statusLine === -1) {
    results.push({ id, action: 'miss', reason: `no Status line under heading in ${loc.path}` });
    continue;
  }
  const status = lines[statusLine].replace(/^- \*\*Status:\*\*\s+/, '').trim();
  if (/^Merged\b/.test(status)) {
    if (loc.archived) {
      results.push({
        id,
        action: 'miss',
        reason: `Merged item in archived module ${loc.path} — refusing to rewrite a frozen archive file`,
      });
    } else {
      lines[statusLine] = newStatus;
      dirtyFiles.add(loc.path);
      results.push({ id, action: 'advanced', reason: loc.path });
    }
  } else if (/^Released\/Shipped\b/.test(status) || /^Complete\b/.test(status)) {
    results.push({ id, action: 'skipped', reason: `already ${status.split(/\s/)[0]}` });
  } else {
    results.push({
      id,
      action: 'miss',
      reason: `status is "${status.split(/\s/)[0]}", not Merged — item never reached Merged`,
    });
  }
}

const advanced = results.filter((r) => r.action === 'advanced');
const skipped = results.filter((r) => r.action === 'skipped');
const misses = results.filter((r) => r.action === 'miss');

// ── Apply edits — all-or-nothing ────────────────────────────────────────────
// Write only when every item resolved cleanly (advanced or idempotently
// skipped). If anything MISSed, write nothing: the operator fixes the record
// or modules and re-runs, never inheriting a half-advanced tree from a run that
// exited non-zero. (Council AR-001.) `--dry-run` also writes nothing.
const wrote = !dryRun && misses.length === 0;
if (wrote) {
  for (const path of dirtyFiles) {
    writeFileSync(path, fileLines.get(path).join('\n'));
  }
}

// ── Report ──────────────────────────────────────────────────────────────────
if (values.json) {
  console.log(
    JSON.stringify({ tag, sha: sha8, date, dryRun, wrote, lifecycleState, results }, null, 2)
  );
} else {
  const prefix = dryRun ? '[advance-released] (dry-run) ' : '[advance-released] ';
  if (lifecycleState && lifecycleState !== 'published') {
    console.error(
      `${prefix}note: release record lifecycleState is "${lifecycleState}", not "published"`
    );
  }
  const verb = wrote ? 'ADVANCED' : 'WOULD ADVANCE';
  for (const r of advanced) console.log(`${prefix}${verb} ${r.id} (${r.reason})`);
  for (const r of skipped) console.log(`${prefix}SKIP ${r.id} — ${r.reason}`);
  for (const r of misses) console.error(`${prefix}MISS: ${r.id} — ${r.reason}`);
  if (misses.length > 0 && !dryRun) {
    console.error(
      `${prefix}NOT WRITTEN — ${misses.length} MISS item(s) block this all-or-nothing run; ` +
        `fix them and re-run (${advanced.length} would advance, ${skipped.length} skipped).`
    );
  } else {
    console.log(
      `${prefix}${advanced.length} advanced${wrote ? ' (written)' : ''}, ${skipped.length} skipped, ${misses.length} missed (${items.length} items)`
    );
  }
}

process.exit(misses.length > 0 ? 1 : 0);
