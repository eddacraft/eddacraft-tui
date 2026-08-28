#!/usr/bin/env node
// Surface: no Git conflict markers may survive into the tracked corpus.
//
// Why this exists. `plans/index.aps.md` reached `main` carrying a literal
// diff3 base marker and a duplicate, stale module row beside the live one
// (fixed in PR #4187). Nothing caught it, and the reason is specific: in a
// Markdown table a run of pipes reads as ordinary table syntax, so neither the
// linters nor a human reviewer skimming a rendered diff saw anything wrong.
// The conspicuous `ours` and `theirs` markers had already been cleaned up by
// hand; only the base marker — the one that looks like content — was left.
//
// Markdown is the dangerous case precisely because it degrades silently. A
// conflict marker in Rust or TypeScript fails to compile and announces itself;
// a conflict marker in a governed document just renders, and downstream
// readers pick up whichever of the two duplicated stanzas they hit first.
// The default scope is therefore tracked Markdown. `--all-text` widens it to
// every tracked file for callers that want the belt-and-braces sweep.
//
// What is NOT flagged, deliberately:
//
//   * A bare run of `=` on its own line. That is a valid setext H1 underline
//     in Markdown, and Git never writes a conflict separator without also
//     writing one of the other three markers — so flagging it would buy no
//     detection and cost false positives on ordinary headings.
//   * Longer runs (8+ characters) and markers that appear mid-line. Git emits
//     exactly seven characters at the start of a line.
//
// Escape hatch: a document that legitimately teaches conflict resolution can
// opt out with `<!-- docs-check: allow-conflict-markers -->` anywhere in the
// file. The opt-out is per-file and is reported in the summary, so it stays
// visible rather than becoming a silent hole.
//
// Wired as a `pnpm docs:check` surface and runnable standalone via
// `pnpm conflict-markers:check`.

import { readFileSync, statSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { relative, resolve } from 'node:path';
import { parseArgs } from 'node:util';
import process from 'node:process';

const SURFACE = 'conflict-markers';

/** Per-file opt-out for documents that demonstrate conflict resolution. */
const OPT_OUT = 'docs-check: allow-conflict-markers';

/**
 * The three markers Git writes at the start of a line, each exactly seven
 * characters. Built by repetition rather than written literally so this file
 * does not trip its own check under `--all-text`.
 */
const MARKERS = ['<'.repeat(7), '|'.repeat(7), '>'.repeat(7)];

/**
 * A line is a marker when it starts with exactly seven of one marker character
 * and the next character is whitespace or end-of-line. The trailing check is
 * what rejects an eight-character decorative run.
 */
function markerAt(line) {
  for (const marker of MARKERS) {
    if (!line.startsWith(marker)) continue;
    const next = line[marker.length];
    if (next === undefined || next === ' ' || next === '\t' || next === '\r') return marker;
  }
  return null;
}

const { values } = parseArgs({
  options: {
    root: { type: 'string' },
    'all-text': { type: 'boolean', default: false },
  },
  allowPositionals: false,
});

const root = resolve(values.root ?? process.cwd());

try {
  if (!statSync(root).isDirectory()) throw new Error('not a directory');
} catch (err) {
  // The check could not run — say nothing about the corpus (CIB-278).
  console.error(`[${SURFACE}] cannot read ${root}: ${err.message}`);
  process.exit(2);
}

/**
 * Prefer the Git index so untracked scratch files and build output are out of
 * scope. Fall back to a filesystem walk when the root is not a repository —
 * the unit tests run against plain temporary directories.
 */
function listFiles() {
  const args = ['-C', root, 'ls-files', '-z'];
  if (!values['all-text']) args.push('--', '*.md');
  const res = spawnSync('git', args, { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  if (res.status === 0 && res.stdout.length > 0) {
    return res.stdout.split('\0').filter(Boolean);
  }
  if (res.status === 0) return [];
  const walk = spawnSync(
    'find',
    values['all-text']
      ? [root, '-type', 'f', '-not', '-path', '*/.git/*']
      : [root, '-type', 'f', '-name', '*.md', '-not', '-path', '*/.git/*'],
    { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }
  );
  if (walk.status !== 0) {
    console.error(`[${SURFACE}] cannot enumerate files under ${root}`);
    process.exit(2);
  }
  return walk.stdout
    .split('\n')
    .filter(Boolean)
    .map((abs) => relative(root, abs));
}

const findings = [];
let optedOut = 0;
let scanned = 0;

for (const rel of listFiles()) {
  let text;
  try {
    text = readFileSync(resolve(root, rel), 'utf8');
  } catch {
    // Unreadable or binary-ish entries (a deleted-but-indexed path, a symlink
    // to nowhere) carry no signal. Skipping is an honest under-report, never a
    // fabricated pass for a file we did read.
    continue;
  }
  scanned += 1;
  if (text.includes(OPT_OUT)) {
    optedOut += 1;
    continue;
  }
  const lines = text.split('\n');
  for (const [i, line] of lines.entries()) {
    const marker = markerAt(line);
    if (marker) findings.push({ rel, line: i + 1, text: line.trimEnd().slice(0, 120) });
  }
}

if (findings.length > 0) {
  for (const f of findings) {
    console.error(
      `[${SURFACE}] ERROR: ${f.rel}:${f.line} — conflict marker in tracked content: ${f.text}`
    );
  }
  console.error(
    `[${SURFACE}] ${findings.length} conflict marker(s) across ${new Set(findings.map((f) => f.rel)).size} file(s). ` +
      `Resolve the conflict and remove the markers. A document that deliberately demonstrates ` +
      `conflict resolution may opt out with an HTML comment containing "${OPT_OUT}".`
  );
  process.exit(1);
}

console.log(
  `[${SURFACE}] ok: no conflict markers in ${scanned} scanned file(s)` +
    (optedOut > 0 ? `; ${optedOut} file(s) opted out` : '') +
    (values['all-text'] ? ' [all tracked text]' : ' [tracked Markdown]') +
    '.'
);
process.exit(0);
