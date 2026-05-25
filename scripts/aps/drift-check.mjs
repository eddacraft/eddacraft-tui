#!/usr/bin/env node
import { appendFileSync, existsSync, readdirSync, readFileSync } from 'node:fs';
import { basename, join, relative } from 'node:path';

const args = process.argv.slice(2);

function usage() {
  console.log(`Usage: scripts/aps/drift-check.mjs [--root PATH] [--changed-files PATH] [--release-record PATH] [--pr-title TEXT] [--pr-body-file PATH] [--json]

Warning-mode APS/repo/release drift checks. Findings are advisory and exit 0.`);
}

let root = process.cwd();
let changedFilesPath = '';
let releaseRecordPath = '';
let prTitle = '';
let prBodyPath = '';
let json = false;

for (let i = 0; i < args.length; i += 1) {
  const arg = args[i];
  if (arg === '--help' || arg === '-h') {
    usage();
    process.exit(0);
  }
  if (arg === '--json') {
    json = true;
    continue;
  }
  if (
    arg === '--root' ||
    arg === '--changed-files' ||
    arg === '--release-record' ||
    arg === '--pr-title' ||
    arg === '--pr-body-file'
  ) {
    const value = args[i + 1];
    if (value === undefined || value.startsWith('--')) {
      console.error(`drift-check.mjs: ${arg} requires a value`);
      process.exit(2);
    }
    if (arg === '--root') root = value;
    if (arg === '--changed-files') changedFilesPath = value;
    if (arg === '--release-record') releaseRecordPath = value;
    if (arg === '--pr-title') prTitle = value;
    if (arg === '--pr-body-file') prBodyPath = value;
    i += 1;
    continue;
  }
  console.error(`drift-check.mjs: unknown argument: ${arg}`);
  process.exit(2);
}

const findings = [];

function addFinding(code, message, details = {}) {
  findings.push({ severity: 'warning', code, message, ...details });
}

function readText(path) {
  return readFileSync(path, 'utf8');
}

function readJson(path) {
  return JSON.parse(readText(path));
}

function listModulePaths() {
  const modulesDir = join(root, 'plans/modules');
  if (!existsSync(modulesDir)) return [];
  return readdirSync(modulesDir)
    .filter((name) => name.endsWith('.aps.md'))
    .map((name) => join(modulesDir, name));
}

function normalisePath(path) {
  return path.replaceAll('\\', '/').replace(/^\.\//, '');
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function extractModule(path) {
  const text = readText(path);
  const id = text.match(/^\|\s*([A-Z][A-Z0-9-]*)\s*\|.*?\|\s*(\d+)\/(\d+)\s*\|\s*$/m);
  const slug = basename(path, '.aps.md');
  const items = [];
  // CICD-011 council follow-up: `[a-z]?` after `\d{3}` admits suffixed
  // work-item IDs (e.g. `RCLI3-016b`) that real modules already declare.
  // Without it, b-suffix items are silently dropped from extractModule's
  // count, which under-reports progress and false-positives
  // pr-missing-aps-reference for any PR mentioning a b-suffix ID.
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
    path,
    items,
  };
}

const modules = listModulePaths().map(extractModule);
const items = modules.flatMap((module) => module.items);

// #1769: progress is "done" when a task reaches any of the canonical
// or lifecycle-terminal states. The canonical APS vocabulary lives in
// `plans/aps-rules.md`; Anvil's local status extensions (which map back
// to canonical `Done` for portability) are documented in
// `plans/project-context.md#project-status-extensions`. The accepted
// terminal forms:
//   - `Done`             — canonical schema value (aps-rules Status Rule 3)
//   - `Complete`         — legacy alias the parser normalises to `Done`
//   - `Merged`           — Anvil extension: integration target reached
//   - `Released/Shipped` — Anvil extension: release-record evidence
// External APS tooling that does not know about the Anvil extensions
// will treat them as opaque; this drift-check folds them into the done
// count so progress counters stay accurate inside Anvil.
//
// The earlier equality-on-`Complete` check was misaligned with the
// documented convention and emitted spurious `aps-progress-mismatch`
// warnings across 15 modules. The narrower
// `aps-complete-without-validation-evidence` check below intentionally
// stays keyed on literal `Complete` per Status Rule 4 (the narrative
// validation gate).
//
// Real module text appends free-form prose after the terminal-state
// token. Examples found in-tree (`plans/modules/`):
//   - `Complete — merged 2026-04-29 via PR #1186`
//   - `Complete (commit \`06d764d4\`; file present at ...)`
//   - `Merged 2026-05-17 — all four umbrella-required sub-tasks ...`
//   - `Done — landed via PR #100`
// PR #1771 review-round 1 caught the `Merged 2026-05-17 — ...` form: an
// earlier split-on-separator approach yielded `Merged 2026-05-17` as
// the head token, which then failed an exact Set lookup. Match the
// done-state as a leading prefix with a word boundary instead, so any
// trailing prose (date, paren, dash, period, slash for compound
// `Released/Shipped`) is accepted.
const DONE_PATTERNS = [/^Done\b/, /^Complete\b/, /^Merged\b/, /^Released\/Shipped\b/];

function isDoneStatus(status) {
  const trimmed = status.trim();
  return DONE_PATTERNS.some((pattern) => pattern.test(trimmed));
}

for (const module of modules) {
  if (module.progressDone === null || module.progressTotal === null) continue;
  if (module.items.length === 0) continue;
  const doneCount = module.items.filter((item) => isDoneStatus(item.status)).length;
  if (doneCount !== module.progressDone || module.items.length !== module.progressTotal) {
    addFinding(
      'aps-progress-mismatch',
      `${relative(root, module.path)} progress is ${module.progressDone}/${module.progressTotal}, but tasks count as ${doneCount}/${module.items.length}.`,
      {
        module: module.id,
      }
    );
  }
}

const indexPath = join(root, 'plans/index.aps.md');
if (existsSync(indexPath)) {
  const indexText = readText(indexPath);
  for (const module of modules) {
    const indexProgress = indexText.match(
      new RegExp(`\\|\\s*${escapeRegExp(module.id)}\\s*\\|[^\\n]*?(\\d+)\\/(\\d+)`)
    );
    if (!indexProgress || module.progressDone === null || module.progressTotal === null) continue;
    const done = Number(indexProgress[1]);
    const total = Number(indexProgress[2]);
    if (done !== module.progressDone || total !== module.progressTotal) {
      addFinding(
        'aps-index-progress-mismatch',
        `plans/index.aps.md lists ${module.id} ${done}/${total}, but the module header is ${module.progressDone}/${module.progressTotal}.`,
        {
          module: module.id,
        }
      );
    }
  }
}

for (const item of items.filter((entry) => entry.status === 'Complete')) {
  if (!/^- \*\*Validation:\*\*/m.test(item.block) || !/validation\s+passed/i.test(item.block)) {
    addFinding(
      'aps-complete-without-validation-evidence',
      `${item.id} is Complete without explicit validation evidence.`,
      {
        apsItem: item.id,
      }
    );
  }
}

if (changedFilesPath) {
  const changedFiles = readText(changedFilesPath)
    .split(/\r?\n/)
    .map((entry) => normalisePath(entry.trim()))
    .filter(Boolean);
  const trackedPatterns = items.flatMap((item) =>
    item.files.map((file) => ({ file, item: item.id }))
  );
  for (const file of changedFiles) {
    if (file.startsWith('plans/') || file.startsWith('.changeset/')) continue;
    const referenced = trackedPatterns.some(({ file: pattern }) => {
      if (pattern.endsWith('/**')) return file.startsWith(pattern.slice(0, -3));
      if (pattern.endsWith('/')) return file.startsWith(pattern);
      if (pattern.includes('<date>')) return file.startsWith(pattern.split('<date>')[0]);
      return file === pattern || file.startsWith(`${pattern}/`);
    });
    if (!referenced) {
      addFinding(
        'changed-file-without-aps-reference',
        `${file} is changed but no active APS Files field references it.`,
        {
          path: file,
        }
      );
    }
  }
}

let releaseRecord = null;
if (releaseRecordPath) {
  releaseRecord = readJson(releaseRecordPath);
  if (
    releaseRecord.version &&
    releaseRecord.source?.tag &&
    releaseRecord.version !== releaseRecord.source.tag
  ) {
    addFinding(
      'release-version-tag-mismatch',
      `Release record version ${releaseRecord.version} does not match source tag ${releaseRecord.source.tag}.`
    );
  }
  if (releaseRecord.source?.tag) {
    const packageJsonPath = join(root, 'package.json');
    if (existsSync(packageJsonPath)) {
      const packageVersion = readJson(packageJsonPath).version;
      if (packageVersion && `v${packageVersion}` !== releaseRecord.source.tag) {
        addFinding(
          'package-version-tag-mismatch',
          `package.json version v${packageVersion} does not match release tag ${releaseRecord.source.tag}.`
        );
      }
    }
  }
  if (releaseRecord.lifecycleState === 'published') {
    const artifacts = Array.isArray(releaseRecord.artifacts) ? releaseRecord.artifacts : [];
    if (artifacts.length === 0) {
      addFinding('release-missing-artifacts', 'Published release record has no artifacts.');
    }
    artifacts.forEach((artifact) => {
      if (!artifact.name || !artifact.url || (!artifact.sha256 && !artifact.integrityRef)) {
        addFinding(
          'release-artifact-missing-integrity',
          `Published artifact ${artifact.name ?? '<unnamed>'} is missing required location or integrity metadata.`
        );
      }
    });
  }
  if (releaseRecord.lifecycleState === 'candidate') {
    const recordItems = new Set((releaseRecord.aps?.items ?? []).map((item) => item.id));
    // Prefix match: real APS text writes `Status: Merged YYYY-MM-DD via PR #N`,
    // so strict `=== 'Merged'` silently misses every populated closeout.
    // Mirrors `DONE_PATTERNS`; intentionally distinct from the literal
    // `Complete` match below (Status Rule 4 narrative validation gate).
    for (const item of items.filter((entry) => /^Merged\b/.test(entry.status))) {
      if (!recordItems.has(item.id)) {
        addFinding(
          'candidate-missing-merged-aps-item',
          `Candidate release record does not include merged APS item ${item.id}.`,
          {
            apsItem: item.id,
          }
        );
      }
    }
  }
}

const publishedItems = new Set(
  releaseRecord?.lifecycleState === 'published'
    ? (releaseRecord.aps?.items ?? []).map((item) => item.id)
    : []
);
// Prefix match: real APS text writes `Status: Released/Shipped via vX.Y.Z`,
// so strict `=== 'Released/Shipped'` silently misses every populated closeout.
for (const item of items.filter((entry) => /^Released\/Shipped\b/.test(entry.status))) {
  if (!publishedItems.has(item.id)) {
    addFinding(
      'shipped-aps-without-release-record',
      `${item.id} is Released/Shipped without a matching published release record item.`,
      {
        apsItem: item.id,
      }
    );
  }
}

// CICD-011: PR-metadata drift. A PR should either reference at least one APS
// work item ID (e.g. `CICD-005`, `OPMODEL-012`) anywhere in its title or body,
// or explicitly opt out via an `Unplanned-work:` line in the body. Falls
// through silently when no PR metadata was supplied (e.g. push events).
//
// Pattern notes (#1438 follow-up):
//   - `[a-z]?` after `\d{3}` admits suffixed IDs (`RCLI3-016b`).
//   - `(?<![\w-])` negative lookbehind prevents `pre-FIX-001` from matching
//     `FIX-001`: `-` is a non-word char, so `\b` alone would accept the
//     hyphen-preceded form. The lookbehind also covers `\w`, which is what
//     a normal `\b` already gives us.
//   - The character class `[A-Z][A-Z0-9]{1,15}` deliberately admits
//     digit-tail prefixes like `RCLI3` because that's a real module prefix.
//     False positives from `HTTP-404` / `EC2-123` / `TLS13-001` are handled
//     downstream by checking against known APS prefixes — if the matched
//     token's prefix isn't a prefix any module declares, treat it as
//     non-APS noise rather than a malformed reference.
const apsWorkItemPattern = /(?<![\w-])[A-Z][A-Z0-9]{1,15}-\d{3}[a-z]?\b/g;
const knownApsItems = new Set(items.map((entry) => entry.id));
const knownApsPrefixes = new Set([...knownApsItems].map((id) => id.split('-')[0]));
if (prTitle || prBodyPath) {
  const prBody = prBodyPath && existsSync(prBodyPath) ? readText(prBodyPath) : '';
  // PR #1439 council follow-up: scan ALL APS-shaped tokens in title +
  // body, not just the first. The earlier `.match(pattern)` returned
  // the first match only, so a PR like `addresses HTTP-404 in
  // CICD-005 path` would extract `HTTP-404` first and fire a false-
  // positive `pr-aps-reference-unknown` even though CICD-005 is a
  // legitimate reference. The spec rule is "reference at least one
  // APS work item anywhere".
  //
  // #1438 follow-up: route tokens by prefix awareness.
  //   - if ANY match is in knownApsItems → silent (resolved)
  //   - else if a match has a KNOWN prefix but unknown ID
  //     (e.g. `CICD-999` against a `CICD` module) → `pr-aps-reference-
  //     unknown` names that token — the operator typed an APS-shaped
  //     reference that doesn't resolve, worth investigating
  //   - else (matches exist but no prefix matches a real module —
  //     `HTTP-404`, `EC2-123`, etc.) → treat as non-APS noise and
  //     fall through to the missing/opt-out check
  //   - else (no matches at all) → missing/opt-out check
  const titleMatches = [...prTitle.matchAll(apsWorkItemPattern)].map((m) => m[0]);
  const bodyMatches = [...prBody.matchAll(apsWorkItemPattern)].map((m) => m[0]);
  const allMatches = [...titleMatches, ...bodyMatches];
  const knownMatch = allMatches.find((id) => knownApsItems.has(id));
  const knownPrefixUnknownId = allMatches.find(
    (id) => !knownApsItems.has(id) && knownApsPrefixes.has(id.split('-')[0])
  );
  // #1438 follow-up: `Unplanned-work:` opt-out is now case-sensitive
  // and requires a value of at least 4 non-whitespace characters. The
  // old `/i` flag silently absorbed prose lines like `This is
  // unplanned-work: see thread`; the length requirement stops trivial
  // `Unplanned-work: x` from satisfying it.
  const unplannedOptOut = /^\s*Unplanned-work:\s*\S{4,}/m.test(prBody);
  // CICD-011 council follow-up: surface a `pr-aps-check-degraded`
  // advisory when no known items are loaded. The earlier
  // `knownApsItems.size > 0` guard on `pr-aps-reference-unknown` was
  // present to avoid false-positives in a brand-new repo, but the
  // active-block guard (`prTitle || prBodyPath`) already covers that
  // case; an empty set during a real PR run means module extraction
  // is degraded (broken headingPattern, refactored layout, sparse
  // checkout) and the unknown-reference check is silently disabled.
  // Emit the advisory so the degraded state is observable rather than
  // invisible.
  //
  // CICD-011 cycle-2 council: in degraded mode, short-circuit the
  // remaining PR-metadata checks entirely. The `pr-missing-aps-
  // reference` finding is non-authoritative when the index is empty
  // (we cannot know whether the PR's reference would have resolved
  // against a healthy index), so firing it alongside the degraded
  // advisory gives the operator contradictory signal. Let the
  // degraded advisory stand alone.
  if (knownApsItems.size === 0) {
    addFinding(
      'pr-aps-check-degraded',
      'No APS work items extracted from plans/modules/ — PR-reference checks are disabled for this run.'
    );
  } else if (knownMatch) {
    // At least one match resolves to a real APS item — silent. No
    // finding.
  } else if (knownPrefixUnknownId) {
    addFinding(
      'pr-aps-reference-unknown',
      `PR references APS work item ${knownPrefixUnknownId}, but no module under plans/modules/ declares that ID.`,
      {
        apsItem: knownPrefixUnknownId,
      }
    );
  } else if (!unplannedOptOut) {
    // No match resolved to a known APS ID *and* no match's prefix is
    // a known module prefix (so any matches were `HTTP-404`-style
    // false positives, not malformed APS refs). Treat as missing.
    addFinding(
      'pr-missing-aps-reference',
      'PR title and body do not reference an APS work item (e.g. `CICD-005`) and do not declare `Unplanned-work:` in the body.'
    );
  }
}

const result = { advisory: true, enforcement: 'none', findingCount: findings.length, findings };

if (json) {
  console.log(JSON.stringify(result));
} else {
  console.log('APS drift check');
  console.log('Advisory: true (enforcement: none)');
  if (findings.length === 0) {
    console.log('No drift warnings detected.');
  } else {
    for (const finding of findings) {
      console.log(`- [${finding.code}] ${finding.message}`);
    }
  }

  // #1438 follow-up: drift advisories live in raw job logs by default,
  // which `continue-on-error: true` renders as an ignorable yellow ✓.
  // Surface them via GitHub Actions workflow commands when we detect a
  // runner context:
  //   - `::warning::` emits a Files-Changed-tab annotation so the
  //     finding appears in the PR review surface as well as the job
  //     log.
  //   - `$GITHUB_STEP_SUMMARY` writes a Markdown block visible in the
  //     job summary panel, giving operators a single discoverable view
  //     without log-diving.
  // We DO NOT mutate exit code — these remain advisory.
  if (process.env.GITHUB_ACTIONS === 'true' && findings.length > 0) {
    // Workflow-command escaping per GitHub Actions docs:
    // <https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands#example-passing-a-property-to-a-command>.
    // The runner URL-decodes `%XX` sequences after parsing, so any
    // literal `%`, `\r`, or `\n` in the payload must be escaped FIRST.
    // For property values (after `name=`), commas and colons are also
    // separator-significant and need the same treatment.
    //   - data (after `::`): escape `%` `\r` `\n`
    //   - prop value (in `title=foo`): also escape `,` `:`
    // `%` must be encoded before the others so we don't double-escape.
    const escapeCmdData = (s) =>
      String(s).replace(/%/g, '%25').replace(/\r/g, '%0D').replace(/\n/g, '%0A');
    const escapeCmdProp = (s) => escapeCmdData(s).replace(/,/g, '%2C').replace(/:/g, '%3A');
    for (const finding of findings) {
      const title = escapeCmdProp(`APS drift / ${finding.code}`);
      const message = escapeCmdData(finding.message);
      console.log(`::warning title=${title}::${message}`);
    }
    const summaryPath = process.env.GITHUB_STEP_SUMMARY;
    if (summaryPath) {
      try {
        const lines = ['## APS drift findings', ''];
        lines.push(
          `Advisory-only — ${findings.length} finding${findings.length === 1 ? '' : 's'}:`
        );
        lines.push('');
        for (const finding of findings) {
          lines.push(`- \`${finding.code}\` — ${finding.message}`);
        }
        lines.push('');
        appendFileSync(summaryPath, `${lines.join('\n')}\n`);
      } catch {
        // Best-effort observability surface — never fail the run on a
        // step-summary write error.
      }
    }
  }
}
