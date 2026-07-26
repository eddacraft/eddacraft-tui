#!/usr/bin/env node
// CIB-196: promote the `## [Unreleased]` draft into a real release section.
//
// The previous inline implementation appended `## <version>` plus a
// "Release preparation metadata generated." line to the BOTTOM of both
// changelogs and never touched the Unreleased draft. Every cut therefore
// began with a structural rewrite by hand: move the draft up, delete the
// stub, fix the heading format. At the v0.9.0-beta cut that curation took two
// follow-up commits on the promotion PR and introduced a duplicated bullet
// that Copilot had to catch.
//
// Usage:
//   node promote-changelog.mjs --version <v> --date <YYYY-MM-DD> [--root <dir>]
//                              [--allow-empty] [--check]
//
// `--check` validates that a promotable draft exists and writes nothing, so
// `prepare` can refuse a cut *before* it starts bumping versions. Without it a
// late failure leaves the tree half-bumped and the retry blocked on a dirty
// working tree.
//
// Exit codes:
//   0  promoted (or already promoted — this is idempotent)
//   2  bad invocation
//   3  CHANGELOG.md has no `## [Unreleased]` section to promote
//   4  the Unreleased draft is empty (pass --allow-empty for a genuinely
//      internal-only release)

import fs from 'node:fs';
import path from 'node:path';

const MONTHS = [
  'January',
  'February',
  'March',
  'April',
  'May',
  'June',
  'July',
  'August',
  'September',
  'October',
  'November',
  'December',
];

const PUBLIC_CHANGELOG = 'docs/public/anvil/releases/changelog.md';

function fail(code, message) {
  process.stderr.write(`promote-changelog: ${message}\n`);
  process.exit(code);
}

function parseArgs(argv) {
  const args = { root: process.cwd(), allowEmpty: false, check: false };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--allow-empty') {
      args.allowEmpty = true;
    } else if (arg === '--check') {
      args.check = true;
    } else if (arg === '--version' || arg === '--date' || arg === '--root') {
      const value = argv[i + 1];
      if (!value) fail(2, `${arg} needs a value`);
      args[arg.slice(2)] = value;
      i += 1;
    } else {
      fail(2, `unknown argument ${arg}`);
    }
  }
  if (!args.version) fail(2, '--version is required');
  if (!args.date) fail(2, '--date is required');
  if (!/^\d{4}-\d{2}-\d{2}$/.test(args.date)) {
    fail(2, `--date must be YYYY-MM-DD, got ${args.date}`);
  }
  // Tags carry a leading `v`; changelog headings do not.
  args.version = args.version.replace(/^v/, '');
  return args;
}

/** `2026-07-12` -> `12 July 2026`, matching the public changelog's style. */
function humanDate(iso) {
  const [year, month, day] = iso.split('-');
  return `${Number(day)} ${MONTHS[Number(month) - 1]} ${year}`;
}

/**
 * Split a changelog into everything before the first `## ` heading and the
 * sections that follow, each retaining its own heading.
 */
function splitSections(text) {
  const lines = text.split('\n');
  const start = lines.findIndex((line) => line.startsWith('## '));
  if (start === -1) return { preamble: text, sections: [] };

  const preamble = lines.slice(0, start).join('\n');
  const sections = [];
  let current = null;
  for (const line of lines.slice(start)) {
    if (line.startsWith('## ')) {
      if (current) sections.push(current);
      current = { heading: line, body: [] };
    } else {
      current.body.push(line);
    }
  }
  if (current) sections.push(current);
  return { preamble, sections };
}

/**
 * Separate the Unreleased section's standing `> **Draft.**` note from the
 * actual entries. The note describes the section and must survive promotion;
 * the entries are what moves.
 */
function splitDraftNote(bodyLines) {
  const note = [];
  let index = 0;
  // Skip blank lines, then take a leading blockquote block if present.
  while (index < bodyLines.length && bodyLines[index].trim() === '') index += 1;
  if (index < bodyLines.length && bodyLines[index].startsWith('>')) {
    while (index < bodyLines.length && bodyLines[index].trim() !== '') {
      note.push(bodyLines[index]);
      index += 1;
    }
  }
  return { note, entries: bodyLines.slice(index) };
}

function trimBlank(lines) {
  const copy = [...lines];
  while (copy.length && copy[0].trim() === '') copy.shift();
  while (copy.length && copy[copy.length - 1].trim() === '') copy.pop();
  return copy;
}

function promoteMain(root, version, date, allowEmpty, check) {
  const file = path.join(root, 'CHANGELOG.md');
  const text = fs.readFileSync(file, 'utf8');
  const heading = `## [${version}] — ${date}`;

  const { preamble, sections } = splitSections(text);

  // Already promoted. Return the entries from the existing release section
  // rather than a bare "done" signal: a run that wrote CHANGELOG.md and then
  // died before the public changelog has to be recoverable by rerunning, and
  // the public promotion needs those entries as its source.
  const already = sections.find((section) => section.heading.startsWith(`## [${version}]`));
  if (already) return trimBlank(already.body);

  const unreleased = sections.find((section) => /^## \[Unreleased\]/i.test(section.heading));
  if (!unreleased) {
    fail(3, `${file} has no '## [Unreleased]' section to promote.`);
  }

  const { note, entries } = splitDraftNote(unreleased.body);
  const promoted = trimBlank(entries);
  if (promoted.length === 0 && !allowEmpty) {
    fail(
      4,
      `${file} '## [Unreleased]' has no entries to promote. Write the ` +
        'customer-facing changes before preparing the release, or pass ' +
        '--allow-empty for a genuinely internal-only release.'
    );
  }

  if (check) return promoted;

  const out = [];
  if (preamble.trim() !== '') out.push(preamble.replace(/\s*$/, ''), '');
  // Unreleased survives, emptied back down to its standing note, so the next
  // cycle has somewhere to accumulate.
  out.push(unreleased.heading, '');
  if (note.length) out.push(...note, '');
  if (promoted.length) {
    out.push(heading, '', ...promoted, '');
  } else {
    out.push(heading, '', '- Internal maintenance only; no customer-facing changes.', '');
  }
  for (const section of sections) {
    if (section === unreleased) continue;
    out.push(section.heading, '', ...trimBlank(section.body), '');
  }

  fs.writeFileSync(file, `${out.join('\n').replace(/\s*$/, '')}\n`);
  return promoted;
}

/**
 * The public changelog is a curated summary with no draft of its own, and it
 * leads with the current release. Seed it with the same promoted entries, at
 * the top where the current release belongs, so the curator trims prose in
 * place instead of authoring a section from nothing at the bottom of the file.
 */
function promotePublic(root, version, date, promoted) {
  const file = path.join(root, PUBLIC_CHANGELOG);
  // Read first and treat a missing file as "nothing to do", rather than
  // testing for existence and then reading it. The check-then-use form is a
  // time-of-check/time-of-use race (`js/file-system-race`): the file can
  // vanish between the two calls, and the read is the operation that has to
  // cope with that anyway.
  let text;
  try {
    text = fs.readFileSync(file, 'utf8');
  } catch (error) {
    if (error.code === 'ENOENT') return; // no public changelog in this checkout
    throw error;
  }
  if (text.includes(`## ${version} —`) || text.includes(`## ${version}\n`)) return;

  const { preamble, sections } = splitSections(text);
  const out = [];
  if (preamble.trim() !== '') out.push(preamble.replace(/\s*$/, ''), '');
  out.push(`## ${version} — ${humanDate(date)}`, '');
  const entries = promoted && promoted.length ? promoted : ['- Internal maintenance only.'];
  out.push(...entries, '');
  for (const section of sections) {
    out.push(section.heading, '', ...trimBlank(section.body), '');
  }
  fs.writeFileSync(file, `${out.join('\n').replace(/\s*$/, '')}\n`);
}

const args = parseArgs(process.argv.slice(2));
const promoted = promoteMain(args.root, args.version, args.date, args.allowEmpty, args.check);
// Always attempt the public promotion outside --check, even when the main
// changelog was already done: both writes are idempotent, so a rerun is how a
// half-finished promotion is repaired.
if (!args.check) {
  promotePublic(args.root, args.version, args.date, promoted);
}
