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

import { closeSync, constants, fstatSync, openSync, readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { basename, dirname, extname, resolve } from 'node:path';
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
const TRACKED_DIRECTORY_SYMLINKS = new Set([
  // This alias points at docs/public/aps, whose tracked files are scanned
  // independently. No other tracked path may resolve to a directory.
  'apps/docs-public-astro/src/content/docs/aps',
]);
const READ_NONBLOCK = constants.O_RDONLY | (constants.O_NONBLOCK ?? 0);

const { values } = parseArgs({
  options: {
    root: { type: 'string' },
  },
  allowPositionals: false,
});

const root = resolve(values.root ?? process.cwd());

// ---------------------------------------------------------------------------
// File list: tracked files and their Git modes, NUL-delimited so exotic names
// cannot split. A failure here is an environment problem (not a git repo, git
// missing), so it exits 2 — the corpus carries no signal either way (CIB-278).
// ---------------------------------------------------------------------------
const ls = spawnSync('git', ['ls-files', '-s', '-z'], {
  cwd: root,
  encoding: 'utf8',
  maxBuffer: 64 * 1024 * 1024,
});
if (ls.error || ls.status !== 0) {
  const reason = ls.error?.message ?? (ls.stderr || `git exited ${ls.status}`);
  console.error(`[${SURFACE}] cannot list tracked files under ${root}: ${reason.trim()}`);
  process.exit(2);
}

const files = [];
for (const entry of ls.stdout.split('\0').filter(Boolean)) {
  const tab = entry.indexOf('\t');
  if (tab < 0) {
    console.error(`[${SURFACE}] cannot parse tracked-file entry from git ls-files`);
    process.exit(2);
  }
  const metadata = entry.slice(0, tab).split(' ');
  const path = entry.slice(tab + 1);
  if (
    metadata.length < 3 ||
    !/^[0-7]{6}$/.test(metadata[0]) ||
    !/^[0-9a-f]{40,64}$/i.test(metadata[1]) ||
    metadata[2] !== '0'
  ) {
    console.error(`[${SURFACE}] cannot parse tracked-file metadata for ${path}`);
    process.exit(2);
  }
  if (isScanned(path)) files.push({ mode: metadata[0], oid: metadata[1], path });
}

// Scan immutable index blobs, not mutable worktree paths. Refuse any scanned
// path whose worktree representation differs from the index; this preserves
// local unstaged-change coverage without relying on racy pathname observations.
const diff = spawnSync('git', ['diff', '--no-ext-diff', '--name-only', '-z', '--'], {
  cwd: root,
  encoding: 'utf8',
  maxBuffer: 64 * 1024 * 1024,
});
if (diff.error || diff.status !== 0) {
  const reason = diff.error?.message ?? (diff.stderr || `git exited ${diff.status}`);
  console.error(`[${SURFACE}] cannot compare tracked files under ${root}: ${reason.trim()}`);
  process.exit(2);
}
const dirtyPaths = new Set(diff.stdout.split('\0').filter(Boolean));
const blobs = readTrackedBlobs(files);

function readTrackedBlobs(entries) {
  const batch = spawnSync('git', ['cat-file', '--batch'], {
    cwd: root,
    input: `${entries.map(({ oid }) => oid).join('\n')}\n`,
    maxBuffer: 256 * 1024 * 1024,
  });
  if (batch.error || batch.status !== 0) {
    const stderr = batch.stderr?.toString('utf8') || `git exited ${batch.status}`;
    console.error(`[${SURFACE}] cannot read tracked blobs under ${root}: ${stderr.trim()}`);
    process.exit(2);
  }

  const result = [];
  let offset = 0;
  for (const { oid, path } of entries) {
    const headerEnd = batch.stdout.indexOf(0x0a, offset);
    if (headerEnd < 0) {
      console.error(`[${SURFACE}] cannot parse tracked blob header for ${path}`);
      process.exit(2);
    }
    const header = batch.stdout.subarray(offset, headerEnd).toString('utf8');
    const match = /^([0-9a-f]{40,64}) blob ([0-9]+)$/i.exec(header);
    if (!match || match[1] !== oid) {
      console.error(`[${SURFACE}] cannot parse tracked blob metadata for ${path}`);
      process.exit(2);
    }
    const size = Number(match[2]);
    const start = headerEnd + 1;
    const end = start + size;
    if (!Number.isSafeInteger(size) || end >= batch.stdout.length || batch.stdout[end] !== 0x0a) {
      console.error(`[${SURFACE}] cannot parse tracked blob content for ${path}`);
      process.exit(2);
    }
    result.push(batch.stdout.subarray(start, end));
    offset = end + 1;
  }
  return result;
}

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

for (let fileIndex = 0; fileIndex < files.length; fileIndex += 1) {
  const { mode, path } = files[fileIndex];
  if (dirtyPaths.has(path)) {
    unreadable.push({ path, message: 'worktree content differs from the indexed blob' });
    continue;
  }

  let fd;
  let text;
  try {
    if (mode === '120000') {
      // Resolve from the immutable indexed link text. This works for native
      // symlinks and core.symlinks=false regular-file materialisations, and a
      // worktree-path ABA cannot redirect the descriptor.
      const target = blobs[fileIndex].toString('utf8');
      if (target.length === 0 || target.includes('\u0000')) {
        throw new Error('indexed symlink target is invalid');
      }
      const targetPath = resolve(dirname(resolve(root, path)), target);
      fd = openSync(targetPath, READ_NONBLOCK);
      const opened = fstatSync(fd);
      if (opened.isDirectory()) {
        if (TRACKED_DIRECTORY_SYMLINKS.has(path)) continue;
        throw new Error('tracked symlink resolved to an unexpected directory');
      }
      if (!opened.isFile()) throw new Error('tracked symlink target is not a regular file');
      text = readFileSync(fd, 'utf8');
    } else if (mode === '100644' || mode === '100755') {
      text = blobs[fileIndex].toString('utf8');
    } else {
      throw new Error(`unsupported tracked mode ${mode}`);
    }
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
