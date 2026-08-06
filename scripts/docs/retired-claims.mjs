// Retired product claims — the tombstone list.
//
// When a user-facing claim is judged dishonest and removed (a CIB honesty
// item), fixing only the surface in that item's scope lets the same words
// survive elsewhere and reappear later. This repository has produced two
// independent multi-item chains of exactly that shape:
//
//   - "daily save-time protection": removed from welcome (CIB-260, #3618),
//     then survived in install.sh until CIB-288;
//   - the Windows `\\?\` path prefix: CIB-237 → 279 → 282 → 285 → 287,
//     five items, one defect, one surface at a time.
//
// An entry here retires the *claim*, not one occurrence of it. The
// `check-retired-claims.mjs` surface fails when a retired phrase appears
// anywhere in the tracked tree outside the documented historical corpora,
// so the whole class must be fixed (or explicitly baselined to an owning
// CIB item) in the same change — and a reworded surface cannot quietly
// reintroduce the exact phrase later.
//
// Adding an entry:
//   - `phrase`    — matched case-insensitively as a literal substring.
//                   Retire the *specific false claim*, not the vocabulary:
//                   "daily save-time protection" (an outcome promise), never
//                   "save-time" (an honest feature name).
//   - `retiredBy` — the CIB item (and PR) that judged the claim false. Only
//                   add entries for claims an operator-authorised item has
//                   actually retired; a Draft finding is not a retirement.
//   - `baseline`  — known survivors, each owned by an open CIB item. The
//                   check requires exactly `occurrences` matches in `path`:
//                   more is new spread (fail), fewer means the owning item
//                   landed and the entry must be deleted (fail, on purpose —
//                   that is the moment the phrase becomes fully banned).
//
// Deliberate quotation (for example a guard test asserting a phrase stays
// absent) is exempted per line by including the marker
// `retired-claim-ok: CIB-NNN` in a comment on the same line.
//
// Not scanned (documented in EXCLUDED_PREFIXES / EXCLUDED_FILES below):
// planning corpus and changelogs, which quote retired claims as history.

/** @type {{phrase: string, retiredBy: string, baseline: {path: string, occurrences: number, owner: string}[]}[]} */
export const RETIRED_CLAIMS = [
  {
    // CIB-260: a bare `anvil start` does not attach save-time coverage to
    // the worktree (no daemon at all in non-interactive contexts; a freshly
    // started daemon has not yet admitted the worktree; the watch fallback
    // spawns only under `--watch`). The next-step copy now describes the
    // activation start actually performs.
    phrase: 'daily save-time protection',
    retiredBy: 'CIB-260 (#3618)',
    // No survivors: CIB-288 cleared the last one (the post-install banner in
    // `install.sh`), so this phrase is now banned outright across the tracked
    // tree — any reappearance is new spread, not a known one.
    baseline: [],
  },
];

// Historical corpora: these quote retired claims as a record of what was
// said and when, which is exactly what they are for. Everything else in the
// tracked tree — source, docs, scripts, test fixtures, golden output — is
// scanned, because fixtures and goldens are where reintroductions hide.
export const EXCLUDED_PREFIXES = [
  'plans/', // APS corpus: items quote the claims they retire
  'docs/public/anvil/releases/', // published changelog / upgrade notes
];

export const EXCLUDED_FILES = [
  'CHANGELOG.md', // repository changelog: historical record
  'scripts/docs/retired-claims.mjs', // this list names every phrase it bans
  'scripts/docs/check-retired-claims.mjs', // checker self-reference in output
];

// Content that cannot carry a product claim and is expensive to scan.
export const EXCLUDED_EXACT_BASENAMES = ['pnpm-lock.yaml', 'Cargo.lock'];
export const EXCLUDED_EXTENSIONS = [
  '.png',
  '.jpg',
  '.jpeg',
  '.gif',
  '.ico',
  '.svg',
  '.woff',
  '.woff2',
  '.ttf',
  '.otf',
  '.wasm',
  '.zip',
  '.gz',
];

/** Per-line escape hatch for deliberate quotation (guard tests, lint docs). */
export const LINE_MARKER = 'retired-claim-ok:';
