#!/usr/bin/env node
// Surface: RELEASE-PLAN.md must stay forward-looking.
//
// RELEASE-PLAN.md scopes exactly one *active* release window (theme, phase
// plans, cut criteria). Closed releases are pruned on closeout — their durable
// record lives in `plans/releases/<tag>.md`. This surface fails the build if the
// document accretes history again:
//   1. anything other than exactly one `## Active window` section;
//   2. a legacy `## Next Release Window` header, or a window header that says
//      `Shipped` (a closed release left in the plan);
//   3. an active-window version that is already a git tag (shipped — should have
//      been pruned).
//
// Wired as a `pnpm docs:check` surface and runnable standalone via
// `pnpm release-plan:check`. See RELEASE-PLAN.md ("How this document works")
// and docs/policies/release-cadence.md (Operator Checklist).

import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import process from 'node:process';

const SURFACE = 'release-plan';

const argv = process.argv.slice(2);
const rootIdx = argv.indexOf('--root');
const root = rootIdx >= 0 && argv[rootIdx + 1] ? argv[rootIdx + 1] : process.cwd();
const file = resolve(root, 'RELEASE-PLAN.md');

let text;
try {
  text = readFileSync(file, 'utf8');
} catch (err) {
  console.error(`[${SURFACE}] cannot read ${file}: ${err.message}`);
  process.exit(2);
}

const errors = [];

// (1) Exactly one active window.
const activeHeadings = [...text.matchAll(/^##\s+Active window\b.*$/gm)].map((m) => m[0].trim());
if (activeHeadings.length === 0) {
  errors.push(
    'no "## Active window" section — RELEASE-PLAN.md must scope exactly one active release window.'
  );
} else if (activeHeadings.length > 1) {
  errors.push(
    `${activeHeadings.length} "## Active window" sections — keep exactly one (closed releases belong in plans/releases/<tag>.md).`
  );
}

// (2) No legacy / shipped window headers.
for (const m of text.matchAll(/^##\s+Next Release Window\b.*$/gm)) {
  errors.push(
    `legacy "## Next Release Window" header — use a single "## Active window" (history belongs in plans/releases/): ${m[0].trim()}`
  );
}
for (const m of text.matchAll(/^#{2,3}\s+.*\bShipped\b.*$/gm)) {
  errors.push(
    `shipped-release header in RELEASE-PLAN.md — prune it; the record lives in plans/releases/: ${m[0].trim()}`
  );
}

// (3) The active-window version must not already be tagged (shipped).
if (activeHeadings.length === 1) {
  const verMatch = activeHeadings[0].match(/v\d+\.\d+\.\d+[0-9A-Za-z.-]*/);
  if (verMatch) {
    const version = verMatch[0];
    // Best-effort: if git/tags are unavailable (shallow checkout), skip rather
    // than false-fail. A real shipped tag in the plan is what we want to catch.
    const res = spawnSync('git', ['-C', root, 'tag', '--list', version], { encoding: 'utf8' });
    if (res.status === 0 && res.stdout.trim().split('\n').includes(version)) {
      errors.push(
        `active window names \`${version}\`, which is already a git tag (shipped) — prune it and scope the next window.`
      );
    }
  } else {
    errors.push(`active window heading carries no vX.Y.Z version: ${activeHeadings[0]}`);
  }
}

if (errors.length > 0) {
  for (const e of errors) console.error(`[${SURFACE}] ERROR: ${e}`);
  console.error(
    `[${SURFACE}] ${errors.length} error(s). RELEASE-PLAN.md must stay forward-looking — see its "How this document works" and docs/policies/release-cadence.md.`
  );
  process.exit(1);
}

const ver = activeHeadings[0].match(/v\d+\.\d+\.\d+[0-9A-Za-z.-]*/);
console.log(
  `[${SURFACE}] ok: RELEASE-PLAN.md is forward-looking — one active window${ver ? ` (\`${ver[0]}\`, not yet tagged)` : ''}.`
);
process.exit(0);
