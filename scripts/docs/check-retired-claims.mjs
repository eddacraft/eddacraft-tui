#!/usr/bin/env node
// Surface: retired product claims must not survive or reappear.
//
// When an honesty item retires a user-facing claim (see
// `retired-claims.mjs`), this surface holds the whole tracked tree to that
// decision: every occurrence of a retired phrase outside the documented
// historical corpora is a content failure unless it is baselined to the open
// CIB item that owns it, or marked as deliberate quotation on the line
// (`retired-claim-ok: CIB-NNN`).
//
// Failure modes it exists to close (both observed in this repository):
//   - survival: the claim is fixed on one surface and ships on in another
//     ("daily save-time protection": welcome fixed in CIB-260, install.sh
//     went on selling it until CIB-288);
//   - reintroduction: a later edit restates the retired claim verbatim on
//     any surface, which no per-file guard test can see.
//
// Scan universe: `git ls-files` (tracked files only), so results are
// deterministic and build output can never be flagged. Fixtures and golden
// output are deliberately in scope — recorded CLI output is where a
// reintroduced claim hides longest.
//
// Exit codes follow the docs-check taxonomy (CIB-278): 0 clean, 1 content
// defect, 2 the check could not run (says nothing about the corpus).
//
// Wired as a `pnpm docs:check` surface; standalone via
// `node scripts/docs/check-retired-claims.mjs`.

import { closeSync, fstatSync, openSync, readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { basename, extname, resolve } from 'node:path';
import { parseArgs } from 'node:util';
import process from 'node:process';

import {
  RETIRED_CLAIMS,
  EXCLUDED_PREFIXES,
  EXCLUDED_FILES,
  EXCLUDED_EXACT_BASENAMES,
  EXCLUDED_EXTENSIONS,
  LINE_MARKER,
} from './retired-claims.mjs';

const SURFACE = 'retired-claims';

const { values } = parseArgs({
  options: {
    root: { type: 'string' },
  },
  allowPositionals: false,
});

const root = resolve(values.root ?? process.cwd());

// ---------------------------------------------------------------------------
// File list: tracked files only, NUL-delimited so exotic names cannot split.
// A failure here is an environment problem (not a git repo, git missing), so
// it exits 2 — the corpus carries no signal either way (CIB-278).
// ---------------------------------------------------------------------------
const ls = spawnSync('git', ['ls-files', '-z'], {
  cwd: root,
  encoding: 'utf8',
  maxBuffer: 64 * 1024 * 1024,
});
if (ls.error || ls.status !== 0) {
  const reason = ls.error?.message ?? (ls.stderr || `git exited ${ls.status}`);
  console.error(`[${SURFACE}] cannot list tracked files under ${root}: ${reason.trim()}`);
  process.exit(2);
}

const files = ls.stdout.split('\0').filter(Boolean).filter(isScanned);

function isScanned(path) {
  if (EXCLUDED_FILES.includes(path)) return false;
  if (EXCLUDED_PREFIXES.some((p) => path.startsWith(p))) return false;
  if (EXCLUDED_EXACT_BASENAMES.includes(basename(path))) return false;
  if (EXCLUDED_EXTENSIONS.includes(extname(path).toLowerCase())) return false;
  return true;
}

// ---------------------------------------------------------------------------
// Scan. Case-insensitive literal substring per line; the marker exempts a
// line; binary files (NUL byte) are skipped as unscannable prose.
// ---------------------------------------------------------------------------
const needles = RETIRED_CLAIMS.map((claim) => ({
  claim,
  lower: claim.phrase.toLowerCase(),
}));

/** @type {Map<string, Map<string, {line: number, text: string, fingerprint: string}[]>>} phrase → path → hits */
const hits = new Map(needles.map(({ claim }) => [claim.phrase, new Map()]));
let scanned = 0;
const unreadable = [];

for (const path of files) {
  let fd;
  let text;
  try {
    const absolutePath = resolve(root, path);
    // Open once, then inspect and read the same descriptor. This prevents a
    // checked path from being swapped before the read while preserving the
    // existing contract that tracked symlink targets remain in the corpus.
    fd = openSync(absolutePath, 'r');
    const opened = fstatSync(fd);
    if (opened.isDirectory()) continue;
    if (!opened.isFile()) throw new Error('tracked path is not a regular file');
    text = readFileSync(fd, 'utf8');
  } catch (err) {
    // A tracked file that could not be read makes the scan inconclusive.
    // Continue to collect every affected path, then report one tooling failure
    // instead of laundering the reduced corpus as clean.
    unreadable.push({ path, message: err instanceof Error ? err.message : String(err) });
    continue;
  } finally {
    if (fd !== undefined) {
      try {
        closeSync(fd);
      } catch (err) {
        unreadable.push({
          path,
          message: `could not close tracked file: ${err instanceof Error ? err.message : String(err)}`,
        });
      }
    }
  }
  if (text.includes('\u0000')) continue; // binary
  scanned += 1;

  // Cheap whole-file pre-check before the per-line pass.
  const lowerText = text.toLowerCase();
  const present = needles.filter(({ lower }) => lowerText.includes(lower));
  if (present.length === 0) continue;

  const lines = text.split('\n');
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (line.includes(LINE_MARKER)) continue;
    const lowerLine = line.toLowerCase();
    for (const { claim, lower } of present) {
      if (!lowerLine.includes(lower)) continue;
      const byPath = hits.get(claim.phrase);
      if (!byPath.has(path)) byPath.set(path, []);
      byPath.get(path).push({
        line: i + 1,
        text: line.trim(),
        fingerprint: contextFingerprint(path, lines, i),
      });
    }
  }
}

if (unreadable.length > 0) {
  for (const failure of unreadable) {
    console.error(`[${SURFACE}] cannot read tracked file ${failure.path}: ${failure.message}`);
  }
  console.error(
    `[${SURFACE}] tooling failure: ${unreadable.length} tracked file(s) were unreadable; ` +
      'the retired-claim corpus was not fully checked.'
  );
  process.exit(2);
}

function contextFingerprint(path, lines, index) {
  const material = [
    path,
    lines[index - 1]?.trim() ?? '',
    lines[index].trim(),
    lines[index + 1]?.trim() ?? '',
  ].join('\0');
  return createHash('sha256').update(material).digest('hex');
}

// ---------------------------------------------------------------------------
// Judge against baselines.
// ---------------------------------------------------------------------------
const errors = [];

for (const { claim } of needles) {
  const byPath = hits.get(claim.phrase);
  const baselined = new Map(claim.baseline.map((b) => [b.path, b]));

  for (const [path, found] of byPath) {
    const entry = baselined.get(path);
    if (!entry) {
      for (const hit of found) {
        errors.push(
          `${path}:${hit.line}: retired claim "${claim.phrase}" (retired by ${claim.retiredBy})\n` +
            `    ${hit.text}\n` +
            `    Do not reintroduce this claim — describe the observed state instead. If this line\n` +
            `    deliberately quotes the phrase (e.g. a guard test), mark it '${LINE_MARKER} <CIB id>'.`
        );
      }
      continue;
    }
    const expected = new Set(entry.fingerprints ?? []);
    const actual = new Set(found.map((hit) => hit.fingerprint));
    const missing = [...expected].filter((fingerprint) => !actual.has(fingerprint));
    const unexpected = [...actual].filter((fingerprint) => !expected.has(fingerprint));
    if (missing.length > 0 || unexpected.length > 0 || found.length !== expected.size) {
      errors.push(
        `${path}: retired claim "${claim.phrase}" — survivor fingerprint mismatch for ` +
          `${entry.owner}; expected ${expected.size}, found ${found.length} ` +
          `(lines ${found.map((h) => h.line).join(', ')}).\n` +
          `    A moved or changed survivor is new spread; fix it rather than replacing an allowed\n` +
          `    occurrence in place. If ${entry.owner} landed, delete the stale baseline entry.`
      );
    }
  }

  // A baselined path with zero matches means the owning item landed (or the
  // file went away). Either way the entry is stale and must be deleted — that
  // deletion is the moment the claim becomes fully banned, so it must not
  // rot silently.
  for (const [path, entry] of baselined) {
    if (!byPath.has(path)) {
      errors.push(
        `${path}: stale baseline for retired claim "${claim.phrase}" — expected ` +
          `${entry.fingerprints?.length ?? 0} fingerprinted occurrence(s) owned by ${entry.owner}, found none.\n` +
          `    ${entry.owner} appears to have landed; delete this baseline entry in retired-claims.mjs.`
      );
    }
  }
}

// ---------------------------------------------------------------------------
// Report.
// ---------------------------------------------------------------------------
if (errors.length > 0) {
  for (const error of errors) console.error(`[${SURFACE}] ${error}`);
  console.error(
    `[${SURFACE}] summary: ${errors.length} error(s); ` +
      `${RETIRED_CLAIMS.length} retired claim(s) checked over ${scanned} tracked files.`
  );
  process.exit(1);
}

console.log(
  `[${SURFACE}] ok: ${RETIRED_CLAIMS.length} retired claim(s) checked over ` +
    `${scanned} tracked files; baselined survivors match their fingerprints.`
);
process.exit(0);
