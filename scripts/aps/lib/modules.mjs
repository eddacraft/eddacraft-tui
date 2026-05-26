// Shared APS module parsing for `plans/modules/*.aps.md`.
//
// Extracted from `scripts/aps/drift-check.mjs` so the advisory drift checker
// and the enforcing index-count generator (`scripts/aps/index-counts.mjs`,
// CIB-022) compute `done/total` from the SAME source. If they diverged, the
// generator could "fix" the index to a value drift-check then flags as drift —
// a self-contradicting loop. Keep this the single definition.

import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { basename, join } from 'node:path';

export function readText(path) {
  return readFileSync(path, 'utf8');
}

export function normalisePath(path) {
  return path.replaceAll('\\', '/').replace(/^\.\//, '');
}

export function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

export function listModulePaths(root) {
  const modulesDir = join(root, 'plans/modules');
  if (!existsSync(modulesDir)) return [];
  return readdirSync(modulesDir)
    .filter((name) => name.endsWith('.aps.md'))
    .sort() // deterministic order regardless of filesystem readdir order
    .map((name) => join(modulesDir, name));
}

// #1769: progress is "done" when a task reaches any of the canonical or
// lifecycle-terminal states. The accepted terminal forms — `Done`, `Complete`,
// `Merged`, `Released/Shipped` — are documented in `plans/aps-rules.md` and
// `plans/project-context.md#project-status-extensions`. Real module text
// appends free-form prose after the token (`Merged 2026-05-17 — ...`,
// `Complete (commit \`06d764d4\`)`), so match a leading prefix with a word
// boundary rather than an exact Set lookup.
export const DONE_PATTERNS = [/^Done\b/, /^Complete\b/, /^Merged\b/, /^Released\/Shipped\b/];

export function isDoneStatus(status) {
  const trimmed = status.trim();
  return DONE_PATTERNS.some((pattern) => pattern.test(trimmed));
}

export function extractModule(path) {
  const text = readText(path);
  const id = text.match(/^\|\s*([A-Z][A-Z0-9-]*)\s*\|.*?\|\s*(\d+)\/(\d+)\s*\|\s*$/m);
  const slug = basename(path, '.aps.md');
  const items = [];
  // CICD-011 council follow-up: `[a-z]?` after `\d{3}` admits suffixed
  // work-item IDs (e.g. `RCLI3-016b`) that real modules already declare.
  // Without it, b-suffix items are silently dropped from extractModule's
  // count, which under-reports progress.
  const headingPattern = /^###\s+([A-Z][A-Z0-9]*-\d{3}[a-z]?)(?::|\s+[—-])\s+(.+)$/gm;
  const headings = [...text.matchAll(headingPattern)];

  headings.forEach((heading, index) => {
    const start = heading.index ?? 0;
    const end =
      index + 1 < headings.length ? (headings[index + 1].index ?? text.length) : text.length;
    const block = text.slice(start, end);
    const status = block.match(/^- \*\*Status:\*\*\s+(.+)$/m)?.[1]?.trim() ?? 'Unknown';
    const filesBlock = block.match(/^- \*\*Files:\*\*\s+([^\n]+(?:\n\s{2,}[^\n]+)*)/m)?.[1] ?? '';
    const files = filesBlock
      .split(/,|\n/)
      .map((entry) =>
        entry
          .replace(/`/g, '')
          .replace(/\s+when implemented$/, '')
          .trim()
      )
      .filter(Boolean)
      .map(normalisePath);
    items.push({
      id: heading[1],
      title: heading[2].trim(),
      status,
      block,
      files,
      moduleSlug: slug,
    });
  });

  return {
    id: id?.[1] ?? slug.toUpperCase(),
    progressDone: id ? Number(id[2]) : null,
    progressTotal: id ? Number(id[3]) : null,
    headerMatch: id,
    path,
    items,
  };
}
