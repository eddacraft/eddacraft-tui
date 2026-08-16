#!/usr/bin/env node
// DOCFRESH-007 (ADR-119 D7): ANVIL_DOCS_VERSION must track the newest
// *published* public-changelog heading — the binary command-probe can actually
// download. During a release cut, prepare may promote the next version into the
// public changelog before the tag/assets exist; the pin stays on the previous
// published release until closeout bumps it.
//
// Exit codes follow the docs-check taxonomy (CIB-278): 0 clean, 1 content
// defect, 2 the check could not run.

import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { parseArgs } from 'node:util';
import process from 'node:process';

const SURFACE = 'docs-version-pin';

const { values } = parseArgs({
  options: {
    root: { type: 'string' },
    workflow: { type: 'string' },
    changelog: { type: 'string' },
  },
  allowPositionals: false,
});

const root = resolve(values.root ?? process.cwd());
const workflowPath = resolve(root, values.workflow ?? '.github/workflows/ci.yml');
const changelogPath = resolve(root, values.changelog ?? 'docs/public/anvil/releases/changelog.md');

function readOrExit(path) {
  try {
    return readFileSync(path, 'utf8');
  } catch (err) {
    const unreadable = err.code !== 'ENOENT';
    console.error(`[${SURFACE}] ERROR: ${rel(path)} — cannot read (${err.code ?? 'error'})`);
    process.exit(unreadable ? 2 : 1);
  }
}

function publishedVersions(repoRoot) {
  try {
    const out = execFileSync('git', ['-C', repoRoot, 'tag', '--list', 'v*'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    return new Set(
      out
        .split(/\r?\n/)
        .filter(Boolean)
        .map((tag) => tag.replace(/^v/, ''))
    );
  } catch {
    // Fixtures and non-git roots fall back to newest-heading matching.
    return null;
  }
}

const workflow = readOrExit(workflowPath);
const changelog = readOrExit(changelogPath);

const pinMatch = workflow.match(/^\s*ANVIL_DOCS_VERSION:\s*(\S+)\s*$/m);
if (!pinMatch) {
  console.error(
    `[${SURFACE}] ERROR: ${rel(workflowPath)} — no ANVIL_DOCS_VERSION assignment found`
  );
  process.exit(1);
}
const pin = pinMatch[1];

const headingRe = /^##\s+(\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?)\b/gm;
const headings = [...changelog.matchAll(headingRe)].map((m) => m[1]);
if (headings.length === 0) {
  console.error(`[${SURFACE}] ERROR: ${rel(changelogPath)} — no version headings found`);
  process.exit(1);
}

const tags = publishedVersions(root);
const newest = headings[0];
const expected =
  tags && tags.size > 0 ? (headings.find((version) => tags.has(version)) ?? newest) : newest;
const expectedLabel =
  expected === newest
    ? 'newest changelog heading'
    : `newest published changelog heading (top heading ${newest} is not tagged yet)`;

if (pin !== expected) {
  console.error(
    `[${SURFACE}] ERROR: ${rel(workflowPath)} — ANVIL_DOCS_VERSION is ${pin}, ${expectedLabel} is ${expected}`
  );
  process.exit(1);
}

console.log(`[${SURFACE}] ok: ANVIL_DOCS_VERSION ${pin} matches ${expectedLabel}`);
process.exit(0);

function rel(path) {
  return path.startsWith(`${root}/`) ? path.slice(root.length + 1) : path;
}
