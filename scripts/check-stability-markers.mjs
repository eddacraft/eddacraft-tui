#!/usr/bin/env node
// TUIN-006 — eddacraft-tui public-API stability-marker check.
//
// Per D-TUIN-005 (accepted 2026-06-22), every public item in `eddacraft-tui`
// should carry a `# Stability` rustdoc section graded `stable` / `unstable` /
// `experimental`; new public items default to `unstable` until graded.
//
// Enforcement is **warn-only** and follows Anvil's "baseline existing state,
// warn on new edges" posture: the legacy unmarked surface is recorded in a
// baseline file, and this check warns (exit 0) only on public items that lack a
// `# Stability` section AND are not in the baseline — i.e. newly added items.
//
//   node scripts/check-stability-markers.mjs            # check (warn-only)
//   node scripts/check-stability-markers.mjs --update   # rewrite the baseline
//   node scripts/check-stability-markers.mjs --json
//
// The check never fails the build (exit 0); a `--strict` flag is reserved for a
// future graduation once the baseline is burned down.

import { readFile, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { parseArgs } from 'node:util';
import globby from 'globby';

const ROOT = resolve(process.cwd());
const SRC_GLOB = 'crates/eddacraft-tui/src/**/*.rs';
const BASELINE = 'crates/eddacraft-tui/stability-baseline.txt';
// Fully-public items only — `pub fn`/`pub struct`/… but NOT `pub(crate)`/
// `pub(super)`/`pub(in …)`, which are not part of the public API surface. The
// leading qualifier group absorbs `default`/`const`/`async`/`unsafe`/`extern "C"`
// so `pub const fn` reads as `fn` (not `const`) and `pub unsafe fn` matches.
const PUB_ITEM =
  /^\s*pub\s+(?:(?:default|const|async|unsafe)\s+)*(?:extern\s+(?:"[^"]+"\s+)?)?(fn|struct|enum|trait|const|type|union)\s+([A-Za-z_][A-Za-z0-9_]*)/;

const { values } = parseArgs({
  options: {
    update: { type: 'boolean', default: false },
    json: { type: 'boolean', default: false },
    root: { type: 'string' },
  },
});
const root = values.root ? resolve(values.root) : ROOT;

/** Walk the contiguous doc-comment / attribute block immediately above `idx`. */
function hasStabilitySection(lines, idx) {
  for (let i = idx - 1; i >= 0; i--) {
    const t = lines[i].trim();
    // Only outer doc comments (`///`) attach to the item below. `//!` is
    // module/inner doc and must NOT leak its `# Stability` onto an item.
    if (t.startsWith('///')) {
      if (/#\s*Stability\b/i.test(t)) return true;
      continue;
    }
    if (t.startsWith('#[') || t === '') continue;
    break; // first non-`///` / non-attr / non-blank line ends the block
  }
  return false;
}

const files = (await globby([SRC_GLOB], { cwd: root })).sort();
const found = [];
for (const rel of files) {
  const content = await readFile(resolve(root, rel), 'utf8');
  const lines = content.split('\n');
  for (let i = 0; i < lines.length; i++) {
    const m = PUB_ITEM.exec(lines[i]);
    if (!m) continue;
    if (hasStabilitySection(lines, i)) continue;
    found.push(`${rel}::${m[1]} ${m[2]}`);
  }
}
// Keys are file+kind+name (line-independent, so the baseline is stable across
// edits); dedup the rare same-name collisions within a file.
const unmarked = [...new Set(found)].sort();

const baselinePath = resolve(root, BASELINE);

if (values.update) {
  const header =
    '# eddacraft-tui public items without a `# Stability` rustdoc section (TUIN-006).\n' +
    '# Baselined legacy surface — burn down by adding `# Stability` to each item.\n' +
    '# Regenerate: node scripts/check-stability-markers.mjs --update\n';
  await writeFile(baselinePath, header + unmarked.join('\n') + '\n');
  process.stdout.write(`[stability] baseline updated: ${unmarked.length} unmarked items\n`);
  process.exit(0);
}

const baseline = new Set(
  existsSync(baselinePath)
    ? (await readFile(baselinePath, 'utf8'))
        .split('\n')
        .map((l) => l.trim())
        .filter((l) => l && !l.startsWith('#'))
    : []
);

const newUnmarked = unmarked.filter((k) => !baseline.has(k));
const graded = baseline.size === 0 ? 0 : [...baseline].filter((k) => !unmarked.includes(k)).length;

if (values.json) {
  process.stdout.write(
    JSON.stringify(
      {
        totalUnmarked: unmarked.length,
        baselined: baseline.size,
        newUnmarked,
        gradedSinceBaseline: graded,
      },
      null,
      2
    ) + '\n'
  );
  process.exit(0);
}

if (newUnmarked.length > 0) {
  process.stdout.write(
    `[stability] WARN: ${newUnmarked.length} new public item(s) lack a \`# Stability\` section ` +
      `(add one — default grade \`unstable\` — or run --update to baseline):\n`
  );
  for (const k of newUnmarked) process.stdout.write(`  - ${k}\n`);
} else {
  process.stdout.write('[stability] ok: no new public items missing a `# Stability` section.\n');
}
if (graded > 0)
  process.stdout.write(
    `[stability] note: ${graded} baselined item(s) now carry a marker — prune them via --update.\n`
  );
process.stdout.write(
  `[stability] summary: ${unmarked.length} unmarked total, ${baseline.size} baselined, ${newUnmarked.length} new. (warn-only)\n`
);
process.exit(0); // warn-only — never blocks
