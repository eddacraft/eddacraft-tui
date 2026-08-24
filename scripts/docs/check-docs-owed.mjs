#!/usr/bin/env node
// Surface validator: documentation owed by a source change.
//
// Every other docs surface answers "is this document internally well-formed?"
// — metadata present, tags known, links resolve, cited paths exist. None of
// them answers the question that actually makes documentation wrong over time:
// *has the thing this document describes changed since anyone last checked the
// document against it?*
//
// The answer is already declared, in every governed document, and nothing reads
// it. The DOCGOV metadata convention makes each document name its Upstream
// sources ("canonical source(s) this doc must not contradict") and a freshness
// line carrying `Last reviewed YYYY-MM-DD`. Put those two together with git
// history and the staleness question becomes decidable: if an Upstream path has
// a commit newer than the review date, the assertion the document makes about
// itself has expired.
//
// `asbuilt-paths` already walks the same references, but only asks whether they
// *resolve* — a path can exist, be rewritten from top to bottom, and still pass.
//
// WHY THIS CAN GATE AT ALL, AND WHAT IT IS STILL WAITING ON
//
// ADR-117: a merge gate may only fail for something a commit did. A calendar
// rule ("reviewed over 90 days ago") violates that outright — it goes red while
// everyone sleeps. Upstream-moved does not: the commit that edits
// `crates/anvil-cli/src/commands/hooks.rs` is precisely what invalidates
// `docs/guides/git-hook-compatibility.md`, so the failure is caused by, and
// fixable in, the pull request that causes it. That is why this check is
// written against the diff (`--since <ref>`) as well as the whole corpus: the
// gate shape is the diff mode, and the corpus mode is the backlog report.
//
// Registered in DEFAULT_SURFACES since DOCFRESH-001, so `pnpm docs:check` runs
// it. It still exits 0 on unbaselined errors: DOCFRESH-003 owns moving the CI
// trigger off `markdownlint-required` and flipping this to the usual
// `errors > 0 ? 1 : 0` contract at the same time, so the gate turns on together
// with the trigger that makes it meaningful. `--fail-on-owed` exercises the
// gate before then.
//
// CONFIDENCE, AND WHY FINDINGS ARE SPLIT INTO TWO CLASSES
//
// A stale review date is evidence, not proof. A document can be kept correct in
// the same commit that changes its upstream while nobody remembers to bump the
// date — common, and reporting it identically to a document nobody has opened
// in three months would bury the signal. So each finding is classified by
// whether the document itself was touched after its upstream moved:
//
//   owed      — upstream moved, and the document's last commit is not the same
//               as or a descendant of that upstream commit. Nobody has looked.
//   review    — the document's last commit is the upstream commit, or comes
//               after it in history. Somebody may have already reconciled it
//               and left the date behind. Rebase often stamps several commits
//               with the same %ct, so this uses ancestry, not timestamps.
//
// Only `owed` is gate-eligible. `review` is a date-hygiene backlog.
//
// GRANULARITY IS THE SECOND AXIS (ADR-119 D2, DOCFRESH-002)
//
// Confidence says whether anyone has looked; granularity says whether the
// declaration is precise enough to act on. Both must hold for a finding to
// bind, so severity is the AND of them:
//
//   ERROR — `owed` AND at least one moved upstream resolves to a file.
//   WARN  — everything else, in two distinguishable flavours recorded on
//           `posture`: `advisory-confidence` (the document was committed after
//           its upstream moved) and `advisory-granularity` (real staleness,
//           declared only as a directory or glob, so nobody can act on it).
//
// A mixed document gates on its file-level components and reports the rest.
// Advisory findings are never hidden — they are counted in the summary in both
// modes — because an unreported dependency reads as "clean", which is the exact
// failure this surface exists to stop.
//
// KNOWN LIMITS (do not let these be discovered as surprises later)
//
//   * Date granularity, and its exact scope. `Last reviewed` is a date, commits
//     carry a timestamp, so a same-day upstream commit is treated as reviewed
//     and this UNDER-reports. Chosen deliberately: a false "you must review
//     this" is far more corrosive to a docs gate's credibility than a missed
//     one. That rule applies ONLY to the human-written review date. Ordering
//     one commit against another uses git ancestry — see lastCommit() and
//     documentCommitCoversUpstream() — because `git rebase` can give several
//     commits the same committer timestamp, and `ts > ts` then misfiles a
//     later restamp as untouched.
//   * Declared, not derived. A document that names no Upstream path is not
//     checkable — it is invisible here, not clean. `uncheckable` in the summary
//     is the real coverage denominator, and shrinking it is a corpus problem
//     (backfill), not a tooling one.
//   * Renames. `git log -- <path>` without `--follow` stops at a rename, so a
//     renamed upstream reads as "no recent commits". `--follow` cannot take
//     multiple paths, and the failure mode is under-reporting, so it is left
//     out; `asbuilt-paths` independently catches a path that stopped existing.

import { readFile } from 'node:fs/promises';
import { existsSync, readFileSync, statSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { resolve, isAbsolute, relative as relPath } from 'node:path';
import { parseArgs } from 'node:util';
import process from 'node:process';
import globby from 'globby';
import { parseDocGovernance, ParseError } from '@eddacraft/anvil-docs-meta';

const SURFACE = 'docs-owed';

// Archive and template documents describe frozen or hypothetical states, so an
// upstream that moved past them is expected rather than a defect.
const SKIPPED_STATUSES = new Set(['Archived']);

const { values } = parseArgs({
  options: {
    root: { type: 'string' },
    baseline: { type: 'string' },
    'no-baseline': { type: 'boolean', default: false },
    json: { type: 'boolean', default: false },
    // Diff mode — the future gate shape. Only upstream paths modified in
    // `<ref>..HEAD` are considered, so the report answers "what does THIS change
    // owe?" rather than "what does the repository owe?".
    since: { type: 'string' },
    // Off by default: this is a report. The flag exists so the gate experiment
    // does not need a code change.
    'fail-on-owed': { type: 'boolean', default: false },
    limit: { type: 'string' },
  },
  allowPositionals: false,
});

const root = resolve(values.root ?? process.cwd());
const limit = values.limit ? Number.parseInt(values.limit, 10) : Infinity;
const baselinePath = resolve(root, values.baseline ?? 'docs/governance/docs-check.baseline.json');

let baseline = {};
if (!values['no-baseline'] && existsSync(baselinePath)) {
  try {
    baseline = JSON.parse(readFileSync(baselinePath, 'utf8'))[SURFACE] ?? {};
  } catch (err) {
    // Matches the sibling surfaces: an unreadable baseline degrades to "absorb
    // nothing" rather than failing the run, so a corrupt ratchet file surfaces
    // as louder findings instead of a dead check.
    process.stderr.write(`[${SURFACE}] failed to read baseline ${baselinePath}: ${err.message}\n`);
  }
}

function git(args) {
  return execFileSync('git', args, {
    cwd: root,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    maxBuffer: 64 * 1024 * 1024,
  });
}

// A surface that cannot run must say so with exit 2 rather than implying the
// corpus is clean (see lib/surface-delegate.mjs).
try {
  git(['rev-parse', '--git-dir']);
} catch (err) {
  process.stderr.write(`[${SURFACE}] cannot run: not a git repository (${err.message})\n`);
  process.exit(2);
}

/**
 * Is this declared upstream implicated by the diff?
 *
 * `git diff --name-only` yields files, so a set-membership test silently drops
 * every directory upstream — `crates/anvil-cli` is never itself a changed path.
 * That would make directory-declared dependencies invisible in diff mode while
 * they remain visible in corpus mode, contradicting the posture that they stay
 * reported (advisory, but never hidden) and breaking the coverage counts.
 * Directories therefore match on path prefix.
 */
/**
 * The baseline fingerprint for one moved upstream.
 *
 * Single source of truth for the string the baseline matches on, so the
 * reported `message` and the absorption candidates can never drift apart.
 * Deliberately carries no date and no count: the baseline matches by exact
 * string, so anything volatile in here silently breaks the ratchet.
 */
function fingerprintFor(upstreamPath) {
  return `upstream moved since review: ${upstreamPath}`;
}

function upstreamTouchedByDiff(upstream, changed) {
  if (changed.has(upstream.path)) return true;
  if (upstream.kind === 'directory') {
    const prefix = `${upstream.path}/`;
    for (const path of changed) if (path.startsWith(prefix)) return true;
    return false;
  }
  if (upstream.kind === 'glob') {
    const pattern = globPathspec(upstream.path);
    for (const path of changed) if (pattern.test(path)) return true;
    return false;
  }
  return false;
}

const globCache = new Map();
/**
 * Mirror git's DEFAULT pathspec globbing — what corpus mode gets from
 * `git log -- <glob>`. Both modes have to agree on what a glob covers, so this
 * follows git rather than intuition, and git's default is not the segment-aware
 * globbing most people expect:
 *
 *   `*` matches any characters INCLUDING `/`. Verified against this repository:
 *   `git ls-files -- 'scripts/release/*'` returns 17 paths under
 *   `scripts/release/_test/`, so a segment-bounded `[^/]*` under-reports.
 *
 *   `**` is therefore just two such wildcards and carries no special
 *   zero-directory meaning: `packages/**` + `/package.json` does NOT match
 *   `packages/package.json`, because the literal `/` before the basename must
 *   still be present. Only `:(glob)` magic — which this surface does not use —
 *   gives `**` the zero-segment behaviour. Confirmed with `git ls-files`
 *   against a fixture containing all three depths.
 */
function globPathspec(glob) {
  let re = globCache.get(glob);
  if (!re) {
    re = new RegExp(`^${glob.replace(/[.+^${}()|[\]\\]/g, '\\$&').replace(/\*+/g, '.*')}$`);
    globCache.set(glob, re);
  }
  return re;
}

/** Paths touched in the diff range, when running in diff mode. */
let changedPaths = null;
if (values.since) {
  try {
    changedPaths = new Set(
      git(['diff', '--name-only', `${values.since}...HEAD`])
        .split('\n')
        .filter(Boolean)
    );
  } catch (err) {
    process.stderr.write(`[${SURFACE}] cannot run: bad --since ref "${values.since}"\n`);
    process.stderr.write(`[${SURFACE}] ${err.message}\n`);
    process.exit(2);
  }
}

const files = await globby(
  [
    'docs/**/*.md',
    '!docs/**/archive/**',
    '!docs/archive/**',
    '!docs/**/*template*.md',
    '!docs/indexes/**',
  ],
  { cwd: root, gitignore: true }
);
files.sort();

const lastCommitCache = new Map();
/**
 * Newest commit touching `path` as `{ date, ts, sha }`, or null if git knows none.
 *
 * `date` (`%cs`, YYYY-MM-DD) is the only thing comparable to a hand-written
 * `Last reviewed` date; comparing at day granularity there is the deliberate
 * under-reporting rule. `ts` (`%ct`) still ranks *which* upstream moved most
 * recently when reporting. Whether the document has been looked at since that
 * upstream commit is ancestry (`sha`), not `ts`: rebase can stamp several
 * commits with the same committer second.
 */
function lastCommit(path) {
  if (lastCommitCache.has(path)) return lastCommitCache.get(path);
  let value = null;
  try {
    const raw = git(['log', '-1', '--format=%cs %ct %H', '--', path]).trim();
    if (raw) {
      const [date, ts, sha] = raw.split(' ');
      value = { date, ts: Number.parseInt(ts, 10), sha };
    }
  } catch {
    // Unreadable or untracked path: asbuilt-paths owns "does it exist", so a
    // silent skip here avoids duplicate reporting of the same defect.
  }
  lastCommitCache.set(path, value);
  return value;
}

/**
 * True when the document's last commit is the upstream commit, or a descendant
 * of it. Equal timestamps are not enough: `git rebase` commonly assigns the
 * same `%ct` to every rewritten commit, and `ts > ts` then treats a later
 * restamp as unread.
 */
function documentCommitCoversUpstream(docCommit, upstreamCommit) {
  if (!docCommit?.sha || !upstreamCommit?.sha) return false;
  if (docCommit.sha === upstreamCommit.sha) return true;
  try {
    git(['merge-base', '--is-ancestor', upstreamCommit.sha, docCommit.sha]);
    return true;
  } catch {
    return false;
  }
}

/**
 * Reduce a metadata reference to a classified repository upstream, or null if it
 * is not one.
 *
 * Upstream cells legitimately hold non-paths — document titles, skill names,
 * APS module IDs — and placeholder examples in angle brackets. Those are not
 * defects, they are simply not checkable here.
 *
 * The `kind` is what ADR-119 D2 binds on, so it is decided once, here:
 *
 *   file      — resolves to a blob. Precise enough to act on, so gate-eligible.
 *   directory — resolves to a tree. Fires on any commit anywhere beneath it.
 *   glob      — contains `*`. Same objection as a directory: the declaration
 *               does not say which file mattered.
 *
 * Globs are classified rather than discarded. Dropping them (the previous
 * behaviour) silently removed real declared dependencies from the corpus —
 * `docs/runbooks/release-runbook.md` declares `scripts/release/*`, and that
 * runbook was invisible to this surface as a result. Advisory and visible beats
 * absent: an unreported dependency reads as "clean", which is the failure this
 * whole surface exists to stop.
 */
function toUpstream(ref) {
  if (!ref) return null;
  if (ref.includes('<') || ref.includes('>') || ref.includes('{')) return null;
  if (/^https?:/.test(ref)) return null;
  const normalised = ref
    .trim()
    .replace(/#.*$/, '')
    .replace(/:\d+(?:-\d+)?$/, '')
    .replace(/\/$/, '');
  if (!normalised) return null;
  if (!/[/.]/.test(normalised)) return null;

  // Keep the traversal guard ahead of any filesystem or git call.
  const abs = resolve(root, normalised.replace(/\*/g, 'x'));
  const rel = relPath(root, abs);
  if (rel.startsWith('..') || isAbsolute(rel)) return null;

  if (normalised.includes('*')) return { path: normalised, kind: 'glob' };
  if (!existsSync(resolve(root, normalised))) return null;
  return {
    path: normalised,
    kind: statSync(resolve(root, normalised)).isDirectory() ? 'directory' : 'file',
  };
}

const findings = [];
let checked = 0;
let uncheckable = 0;
let skipped = 0;
let unparsed = 0;

for (const relFile of files) {
  let parsed;
  try {
    parsed = parseDocGovernance(await readFile(resolve(root, relFile), 'utf8'), relFile);
  } catch (err) {
    // A document with missing or malformed metadata is the metadata surface's
    // finding, not ours; counting it keeps the coverage denominator honest.
    // Anything else — an I/O failure, an unexpected parser crash — means this
    // surface did not actually read the corpus it is about to report on.
    // Folding that into `unparsed` would let a broken scan render as a clean
    // one, which is exactly the misattribution the exit-2 taxonomy exists to
    // prevent (lib/surface-delegate.mjs).
    if (err instanceof ParseError) {
      unparsed += 1;
      continue;
    }
    process.stderr.write(`[${SURFACE}] cannot run: failed to read ${relFile}: ${err.message}\n`);
    process.stderr.write(
      `[${SURFACE}] this is not a docs content defect; the corpus was not read.\n`
    );
    process.exit(2);
  }

  if (SKIPPED_STATUSES.has(parsed.metadata.status)) {
    skipped += 1;
    continue;
  }

  const reviewedOn = parsed.freshness.reviewedOn;
  // The upstream cell is the declared contract. Freshness anchors are included
  // because the convention explicitly invites "reviewed against <path>" there,
  // and a document that names its check anchor is making the same assertion.
  const upstreams = [];
  const seenPaths = new Set();
  for (const ref of parsed.sourceReferences) {
    if (ref.context !== 'upstream' && ref.context !== 'freshness') continue;
    const upstream = toUpstream(ref.path);
    if (!upstream || seenPaths.has(upstream.path)) continue;
    seenPaths.add(upstream.path);
    upstreams.push(upstream);
  }

  if (!reviewedOn || upstreams.length === 0) {
    uncheckable += 1;
    continue;
  }

  const considered = changedPaths
    ? upstreams.filter((u) => upstreamTouchedByDiff(u, changedPaths))
    : upstreams;
  if (considered.length === 0) {
    if (!changedPaths) uncheckable += 1;
    continue;
  }
  checked += 1;

  const moved = considered
    .map((u) => ({ path: u.path, kind: u.kind, commit: lastCommit(u.path) }))
    .filter((u) => u.commit && u.commit.date > reviewedOn)
    .sort((a, b) => b.commit.ts - a.commit.ts);

  if (moved.length === 0) continue;

  const docCommit = lastCommit(relFile);
  const newest = moved[0];
  // Commit-versus-commit uses ancestry. The day-granularity rule is specific
  // to `reviewedOn` above, where no clock time exists.
  const documentTouchedSince = documentCommitCoversUpstream(docCommit, newest.commit);

  // ADR-119 D2. A file-level upstream is a claim precise enough to act on, so
  // it can bind; a directory or glob fires on any commit anywhere beneath it and
  // cannot be acted on, so it may only ever advise. A mixed document therefore
  // gates on its file-level components and merely reports the rest — which is
  // why this asks whether ANY moved upstream is a file, not whether the newest
  // one is.
  const movedFiles = moved.filter((u) => u.kind === 'file');
  const gateEligible = !documentTouchedSince && movedFiles.length > 0;
  // Name an actionable path when the finding can bind: pointing a would-be gate
  // failure at `crates/anvil-cli` tells nobody what to review.
  const primary = gateEligible ? movedFiles[0] : newest;

  findings.push({
    severity: gateEligible ? 'ERROR' : 'WARN',
    class: documentTouchedSince ? 'review' : 'owed',
    // Why this finding cannot bind, when it cannot. `advisory-granularity` is
    // the D2 case: real staleness, but declared too coarsely to act on.
    posture: gateEligible
      ? 'gating'
      : documentTouchedSince
        ? 'advisory-confidence'
        : 'advisory-granularity',
    upstreamKinds: [...new Set(moved.map((u) => u.kind))].sort(),
    file: relFile,
    line: parsed.sourceLineNumber ?? 1,
    type: parsed.metadata.type,
    owner: parsed.metadata.owner,
    reviewedOn,
    docLastCommit: docCommit?.date ?? null,
    daysBehind: Math.round((Date.parse(newest.commit.date) - Date.parse(reviewedOn)) / 86_400_000),
    movedUpstream: moved.map((m) => `${m.path}@${m.commit.date}`),
    // Every upstream that could legitimately name this finding, newest first.
    // Baseline absorption matches on ANY of them, not just the current newest
    // — see the absorption loop for why.
    fingerprints: moved.map((m) => fingerprintFor(m.path)),
    // STABLE fingerprint — deliberately carries no date and no count.
    //
    // The baseline matches findings by exact message string, so anything
    // volatile in here destroys the ratchet: embedding "committed 2026-08-10"
    // would un-baseline a known finding the moment its upstream took one more
    // commit, and the absorbed backlog would reappear as fresh errors on an
    // unrelated pull request. Naming the implicated upstream is the part worth
    // pinning; the dates are reporting detail and are rendered from the
    // structured fields below instead.
    message: fingerprintFor(primary.path),
    detail:
      `${primary.path} (${primary.kind}) committed ${primary.commit.date}, ` +
      `document last reviewed ${reviewedOn}` +
      (moved.length > 1 ? ` (+${moved.length - 1} more upstream path(s))` : '') +
      (documentTouchedSince ? `; document itself committed ${docCommit.date}` : '') +
      (documentTouchedSince || gateEligible
        ? ''
        : `; advisory — only ${[...new Set(moved.map((u) => u.kind))].sort().join('/')} upstreams moved`),
  });
}

// Absorbed findings are downgraded rather than dropped: the ratchet is meant to
// stop the known backlog failing the build, not to make it invisible. Same
// shape as every other baselineable surface (see check-asbuilt-paths.mjs).
//
// Absorption matches ANY of the document's moved upstreams, not only the one
// currently reported. `message` names the newest-moved path, and which path is
// newest changes as different upstreams take commits — so a document declaring
// several upstreams would drift out of its own baseline entry the first time
// the ordering changed, and the absorbed backlog would resurface as fresh
// errors on an unrelated pull request. The unit being ratcheted is the
// *document's* known staleness, not one particular edge of it.
//
// A genuinely new upstream starting to move therefore does not un-absorb the
// finding. That is deliberate: the document is already in the backlog, and one
// more stale edge does not make it newly stale. The finding clears when someone
// re-reviews the document and bumps its date, which removes it entirely.
for (const finding of findings) {
  const absorbed = baseline[finding.file];
  if (Array.isArray(absorbed) && finding.fingerprints.some((f) => absorbed.includes(f))) {
    finding.severity = 'WARN';
    finding.baselined = true;
    finding.message = `[baselined] ${finding.message}`;
  }
}

findings.sort((a, b) => b.daysBehind - a.daysBehind || a.file.localeCompare(b.file));

const owed = findings.filter((f) => f.class === 'owed');
const review = findings.filter((f) => f.class === 'review');
const errors = findings.filter((f) => f.severity === 'ERROR').length;
const warnings = findings.filter((f) => f.severity === 'WARN').length;
const summary = {
  // `errors` / `warnings` / `filesChecked` are the keys docs-check.mjs prints
  // when regenerating the baseline; the rest is this surface's own reporting.
  errors,
  warnings,
  filesChecked: checked,
  owed: owed.length,
  review: review.length,
  // D2 posture, reported so a shrinking gate-eligible count cannot be mistaken
  // for a shrinking backlog — advisory findings are real staleness that simply
  // cannot bind until the document declares a file-level upstream.
  gating: findings.filter((f) => f.posture === 'gating').length,
  advisoryGranularity: findings.filter((f) => f.posture === 'advisory-granularity').length,
  baselined: findings.filter((f) => f.baselined).length,
  checked,
  uncheckable,
  skippedArchived: skipped,
  withoutGovernanceMetadata: unparsed,
  corpus: files.length,
  mode: changedPaths ? `diff (${values.since}...HEAD)` : 'corpus',
};

if (values.json) {
  process.stdout.write(`${JSON.stringify({ surface: SURFACE, findings, summary }, null, 2)}\n`);
} else {
  for (const f of findings.slice(0, limit)) {
    process.stdout.write(
      `[${SURFACE}] ${f.severity}: ${f.file}:${f.line} — ${f.message} — ${f.detail} ` +
        `[${f.daysBehind}d, ${f.type}, owner ${f.owner}]\n`
    );
  }
  if (findings.length > limit) {
    process.stdout.write(`[${SURFACE}] … ${findings.length - limit} more (raise --limit)\n`);
  }
  process.stdout.write(
    `[${SURFACE}] summary [${summary.mode}]: ${summary.owed} owed ` +
      `(${summary.gating} gating, ${summary.advisoryGranularity} advisory by granularity), ` +
      `${summary.review} review, ` +
      `${summary.baselined} baselined, ${summary.checked} checked, ` +
      `${summary.uncheckable} uncheckable ` +
      `(no review date or no resolvable upstream path), ` +
      `${summary.withoutGovernanceMetadata} without governance metadata, ` +
      `${summary.skippedArchived} archived, of ${summary.corpus} documents\n`
  );
}

// Registered but deliberately non-gating under `docs:check`: this exits 0 even
// with unbaselined ERROR findings. DOCFRESH-003 moves the CI trigger off
// `markdownlint-required` and flips this to the usual
// `errors > 0 ? 1 : 0` contract at the same time, so the gate turns on together
// with the trigger that makes it meaningful.
//
// What CAN fail, under `--fail-on-owed`, is an unbaselined finding that is
// *gate-eligible* — ERROR severity, meaning `owed` AND backed by a file-level
// upstream. Keying this on the `owed` class instead would fail on
// directory-only findings, which ADR-119 D2 says must never turn a check red;
// the D2 fixture in docs-check.test.sh pins exactly that.
const gatingFailure =
  values['fail-on-owed'] && findings.some((f) => f.severity === 'ERROR' && !f.baselined);
process.exit(gatingFailure ? 1 : 0);
