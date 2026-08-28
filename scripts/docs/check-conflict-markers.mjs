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
// `pnpm conflict-markers:check`. Explicit paths may be passed as positional
// arguments to scan just those files — the pre-commit hook uses that to check
// exactly the staged Markdown, including `plans/**`, which `.markdownlintignore`
// excludes and so no other staged-file task covers.

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { join, relative, resolve, sep } from 'node:path';
import { parseArgs } from 'node:util';
import process from 'node:process';

const SURFACE = 'conflict-markers';

/** Per-file opt-out for documents that demonstrate conflict resolution. */
const OPT_OUT = 'docs-check: allow-conflict-markers';

/**
 * The opt-out counts only as a standalone HTML comment on its own line.
 *
 * Two tightenings, both found by pointing the check at itself. Matching the
 * bare phrase anywhere let any file that merely *mentions* the opt-out exempt
 * itself; requiring the comment form was not enough either, because this
 * script's own header documents the syntax and so still skipped itself. A
 * line-anchored comment is how a real document writes it, and an incidental
 * mention inside prose, backticks, or a string literal no longer counts.
 */
const OPT_OUT_COMMENT = /^[ \t]*<!--[ \t]*docs-check:[ \t]*allow-conflict-markers[ \t]*-->[ \t]*$/m;

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

const { values, positionals } = parseArgs({
  options: {
    root: { type: 'string' },
    'all-text': { type: 'boolean', default: false },
  },
  // Positional paths select an explicit file list, which is how the pre-commit
  // hook hands over exactly the staged files. Without them the whole corpus is
  // scanned.
  allowPositionals: true,
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
 * Walk the tree without shelling out.
 *
 * A Node walk rather than `find`, for two reasons that would both have bitten
 * on the Windows leg: `find` is not available there, and building results from
 * absolute paths would emit `\` separators while the Git branch below emits
 * `/` — so the same corpus would produce different-looking findings per
 * platform. Relative paths are assembled with `/` directly, so the two sources
 * agree everywhere.
 *
 * Symlinks are not followed: `Dirent.isDirectory()` is false for a symlink, so
 * a link pointing at an ancestor cannot loop the walk.
 */
function walkFiles(dir, prefix, out) {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    // Unreadable directory — an honest under-report, never a fabricated pass
    // for content we did not actually read.
    return out;
  }
  for (const entry of entries) {
    if (entry.name === '.git') continue;
    const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isDirectory()) {
      walkFiles(join(dir, entry.name), rel, out);
    } else if (entry.isFile() && (values['all-text'] || rel.endsWith('.md'))) {
      out.push(rel);
    }
  }
  return out;
}

/**
 * Prefer the Git index so untracked scratch files and build output are out of
 * scope. Fall back to the filesystem walk when the root is not a repository —
 * the unit tests run against plain temporary directories.
 */
function listFiles() {
  if (positionals.length > 0) {
    // Explicit list wins: report paths relative to the root and with forward
    // slashes, so hook output and corpus output read identically.
    return positionals.map((p) => relative(root, resolve(root, p)).split(sep).join('/'));
  }
  const args = ['-C', root, 'ls-files', '-z'];
  if (!values['all-text']) args.push('--', '*.md');
  const res = spawnSync('git', args, { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  // Git already reports repo-relative, forward-slash paths.
  if (res.status === 0) return res.stdout.split('\0').filter(Boolean);
  return walkFiles(root, '', []);
}

const findings = [];
const optedOutFiles = [];
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
  if (OPT_OUT_COMMENT.test(text)) {
    optedOutFiles.push(rel);
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
    (optedOutFiles.length > 0
      ? `; ${optedOutFiles.length} file(s) opted out (${optedOutFiles.join(', ')})`
      : '') +
    (values['all-text'] ? ' [all tracked text]' : ' [tracked Markdown]') +
    '.'
);
process.exit(0);
