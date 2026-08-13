#!/usr/bin/env node
// DOCFRESH-007 (ADR-119 D7): ANVIL_DOCS_VERSION must track the
// newest heading in the public changelog. Command-probe authority is only
// as good as the binary it runs; a stale pin silently validates against
// an old release.
//
// Exit codes follow the docs-check taxonomy (CIB-278): 0 clean, 1 content
// defect, 2 the check could not run.

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
    console.error(`[${SURFACE}] cannot read ${path}: ${err.message}`);
    process.exit(unreadable ? 2 : 1);
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
const newest = headings[0];

if (pin !== newest) {
  console.error(
    `[${SURFACE}] ERROR: ${rel(workflowPath)} — ANVIL_DOCS_VERSION is ${pin}, newest changelog heading is ${newest}`
  );
  process.exit(1);
}

console.log(`[${SURFACE}] ok: ANVIL_DOCS_VERSION ${pin} matches newest changelog heading`);
process.exit(0);

function rel(path) {
  return path.startsWith(`${root}/`) ? path.slice(root.length + 1) : path;
}
