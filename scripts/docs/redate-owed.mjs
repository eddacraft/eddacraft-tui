/**
 * Re-date the documents a change makes owed, round by round, to a fixpoint.
 *
 * `check-docs-owed.mjs` and `check-diagram-impact.mjs` each report only what is
 * owed *right now*. Re-dating a document commits it, which makes it an upstream
 * that moved, so its own downstreams come due on the next run: re-dating
 * `docs/guides/documentation-governance.md` costs two further rounds and a
 * dozen documents. Discovering that by hand — run a gate, read the errors, find
 * each Freshness cell, edit, commit, run again — is the toil this removes.
 *
 * The owed set is taken from the gates themselves rather than recomputed here.
 * Their rules are subtle (ADR-119 D2 binds only on file-level upstreams; a
 * document committed after its upstream moved is absolved; the diagram gate
 * accepts *being touched* as the review; baselined edges are already absorbed)
 * and a second implementation that drifted would either miss documents or stamp
 * review dates onto documents nobody had to review.
 *
 * This tool never invents a justification. `--note` is written verbatim into
 * every cell it touches; deciding that a rule or diagram is unaffected stays a
 * human claim.
 *
 * The note is placed between the date and the demoted `Prior review …` text, so
 * phrase it to sit between them: `for the X change; Y is unaffected.` reads as
 * "Last reviewed 2026-08-31 for the X change; Y is unaffected. Prior review
 * 2026-08-30 …". A note opening with its own capitalised verb collides with the
 * date, and one without closing punctuation collides with `Prior review`. It
 * also lands in a markdown table cell, so use inline code for commands and
 * paths as the surrounding cells do.
 */

import { readFile, writeFile } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { resolve } from 'node:path';
import process from 'node:process';
import { pathToFileURL } from 'node:url';
import { parseArgs, promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const SURFACE = 'redate-owed';
const DEFAULT_MAX_ROUNDS = 6;

/**
 * Today's date in the **local** calendar, as `YYYY-MM-DD`.
 *
 * Not `toISOString()`: that is UTC, and this repository dates in local time
 * (UTC+8). West of midnight UTC the two disagree, so the UTC form silently
 * stamps yesterday onto a review done today — and a document dated a day behind
 * its upstream's commit is owed again the moment the gate runs.
 */
export function localDate(now = new Date()) {
  const pad = (value) => String(value).padStart(2, '0');
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

/**
 * A `|` that markdown would read as a column break — one not already escaped.
 */
export function hasUnescapedPipe(value) {
  return /(^|[^\\])\|/u.test(value);
}

export function tableCells(line) {
  return line
    .split('|')
    .slice(1, -1)
    .map((cell) => cell.trim());
}

/**
 * Locate the DOCGOV-002 metadata table's Freshness cell.
 *
 * Returns the row index and its cells, or null when the document carries no
 * such table — in which case it is left alone and reported.
 */
export function findFreshnessCell(content) {
  const rows = content.split(/\r?\n/u);
  const header = rows.findIndex((row) =>
    /^\|\s*Type\s*\|\s*Authority\s*\|\s*Owner\s*\|\s*Status\s*\|\s*Freshness\s*\|$/u.test(row)
  );
  if (header === -1) return null;
  const valueRow = header + 2;
  const cells = tableCells(rows[valueRow] ?? '');
  if (cells.length < 5) return null;
  return { row: valueRow, cells, freshness: cells[4] };
}

/**
 * Lead with today's review and demote the existing one, keeping the whole prior
 * cell so no recorded provenance is dropped.
 *
 * Two shapes appear in the corpus: most documents open `Last reviewed <date>`,
 * while `plans/specs/**` opens `<date> — `.
 */
export function rewriteFreshness(existing, { date, note }) {
  const trimmed = existing.trim();
  const squash = (value) => value.replace(/\s+/gu, ' ').trim();

  const lastReviewed = /^Last reviewed\s+(\d{4}-\d{2}-\d{2})\s*/u.exec(trimmed);
  if (lastReviewed) {
    const rest = trimmed.slice(lastReviewed[0].length);
    return squash(`Last reviewed ${date} ${note} Prior review ${lastReviewed[1]} ${rest}`);
  }

  const bareDate = /^(\d{4}-\d{2}-\d{2})\s*—\s*/u.exec(trimmed);
  if (bareDate) {
    const rest = trimmed.slice(bareDate[0].length);
    return squash(`${date} — ${note} Prior ${bareDate[1]} — ${rest}`);
  }

  return squash(`Last reviewed ${date} ${note} Prior review: ${trimmed}`);
}

/**
 * Run one gate and take its JSON report.
 *
 * No `--head` is threaded through (CIB-383 added one to both gates): that flag
 * exists for CI, where `actions/checkout` leaves HEAD on `refs/pull/N/merge`.
 * This tool re-dates and commits, so it only ever runs on a real branch and the
 * gates' `HEAD` default is the correct tip by construction.
 */
async function runGate(root, script, args) {
  try {
    const { stdout } = await execFileAsync(
      process.execPath,
      [resolve(root, 'scripts/docs', script), ...args, '--json'],
      { cwd: root, maxBuffer: 64 * 1024 * 1024 }
    );
    return JSON.parse(stdout);
  } catch (error) {
    // Both gates exit non-zero *with* a full JSON report when they find
    // something; only an empty or unparseable stdout is a real failure.
    if (error.stdout) {
      try {
        return JSON.parse(error.stdout);
      } catch {
        /* fall through */
      }
    }
    throw new Error(`${script} did not produce JSON: ${error.stderr || error.message}`);
  }
}

/**
 * Documents owed right now, from the gates that actually fail a build.
 *
 * `docs-owed`: gating, unbaselined findings only — an advisory finding is real
 * staleness the gate has decided it cannot act on, and re-dating it would
 * quietly clear a backlog nobody chose to clear.
 * `diagram-impact`: the `diagram-review-owed` findings; its other findings are
 * mermaid render failures, which a date cannot fix.
 */
export async function owedNow({ root, since }) {
  const owed = new Map();

  const docs = await runGate(root, 'check-docs-owed.mjs', ['--since', since, '--fail-on-owed']);
  for (const finding of docs.findings ?? []) {
    if (finding.posture !== 'gating' || finding.baselined) continue;
    owed.set(finding.file, { path: finding.file, reason: 'docs-owed', detail: finding.message });
  }

  const diagrams = await runGate(root, 'check-diagram-impact.mjs', ['--since', since]);
  for (const finding of diagrams.findings ?? []) {
    if (finding.code !== 'diagram-review-owed') continue;
    const existing = owed.get(finding.path);
    owed.set(finding.path, {
      path: finding.path,
      reason: existing ? 'docs-owed + diagram' : 'diagram',
      detail: `declared upstream changed: ${finding.upstream}`,
    });
  }

  return [...owed.values()].sort((a, b) => a.path.localeCompare(b.path));
}

async function applyRedate({ root, docPath, date, note }) {
  const abs = resolve(root, docPath);
  const content = await readFile(abs, 'utf8');
  const cell = findFreshnessCell(content);
  if (!cell) return false;
  const cells = [...cell.cells];
  cells[4] = rewriteFreshness(cell.freshness, { date, note });
  const rows = content.split(/\r?\n/u);
  rows[cell.row] = `| ${cells.join(' | ')} |`;
  await writeFile(abs, rows.join('\n'), 'utf8');
  return true;
}

/**
 * Format and commit one round.
 *
 * Committing is not bookkeeping here — it is what makes the next round
 * measurable. `check-docs-owed.mjs` reads the document's `Last reviewed` date
 * from the working tree but its upstream's date from git, so an uncommitted
 * re-date makes the gate report green while the cascade it triggers is still
 * invisible. A round that cannot commit must therefore stop the run, not
 * continue into a false fixpoint.
 */
async function formatAndCommit({ root, paths, message }) {
  // `plans/**` is excluded from the formatters; oxfmt reflows the rest, and a
  // markdown table always needs it after a cell grows.
  const formattable = paths.filter((path) => !path.startsWith('plans/'));
  if (formattable.length > 0) {
    // CI=true: without a TTY pnpm otherwise aborts on a modules-dir check
    // rather than running the formatter.
    await execFileAsync('pnpm', ['exec', 'oxfmt', '--write', ...formattable], {
      cwd: root,
      env: { ...process.env, CI: 'true' },
    });
  }
  await execFileAsync('git', ['add', '--', ...paths], { cwd: root });
  await execFileAsync('git', ['commit', '--no-verify', '-m', message], { cwd: root });
}

export async function runRedateCli(argv = process.argv.slice(2)) {
  const { values } = parseArgs({
    args: argv,
    options: {
      root: { type: 'string' },
      since: { type: 'string' },
      note: { type: 'string' },
      date: { type: 'string' },
      write: { type: 'boolean', default: false },
      'max-rounds': { type: 'string' },
    },
  });

  const root = resolve(values.root ?? process.cwd());
  const since = values.since;
  if (!since) {
    process.stderr.write(`[${SURFACE}] --since <base> is required\n`);
    return 2;
  }
  const date = values.date ?? localDate();
  if (!/^\d{4}-\d{2}-\d{2}$/u.test(date)) {
    process.stderr.write(`[${SURFACE}] --date must be YYYY-MM-DD, got '${date}'\n`);
    return 2;
  }
  // The note is the review claim. Stamping a date with an empty one would
  // assert a review that never happened.
  if (values.write && !values.note?.trim()) {
    process.stderr.write(`[${SURFACE}] --write requires a non-empty --note\n`);
    return 2;
  }
  // The note lands in a markdown table cell, so a bare `|` would add columns and
  // corrupt the DOCGOV-002 metadata table — across every document in the
  // cascade. Refuse before writing anything rather than escaping silently: the
  // note is written verbatim, so altering it is not this tool's call.
  if (values.note && hasUnescapedPipe(values.note)) {
    process.stderr.write(
      `[${SURFACE}] --note may not contain an unescaped '|' — it would break the ` +
        `metadata table. Write it as '\\|' for a literal pipe.\n`
    );
    return 2;
  }
  const maxRounds = Number.parseInt(values['max-rounds'] ?? `${DEFAULT_MAX_ROUNDS}`, 10);
  if (!Number.isInteger(maxRounds) || maxRounds < 1) {
    process.stderr.write(`[${SURFACE}] --max-rounds must be a positive integer\n`);
    return 2;
  }

  const note = values.note?.trim();
  let total = 0;

  for (let round = 1; round <= maxRounds; round += 1) {
    const owed = await owedNow({ root, since });
    if (owed.length === 0) {
      process.stdout.write(
        total === 0
          ? `[${SURFACE}] nothing owed against ${since}\n`
          : `[${SURFACE}] settled: ${total} document(s) re-dated to ${date} over ${round - 1} round(s)\n`
      );
      return 0;
    }

    for (const doc of owed) {
      process.stdout.write(`[${SURFACE}] round ${round}: ${doc.path} (${doc.reason})\n`);
    }

    if (!values.write) {
      process.stdout.write(
        `[${SURFACE}] round ${round} shows ${owed.length} owed. This is one round only — ` +
          `re-dating these reveals the next. Re-run with --write --note "<review note>" ` +
          `to apply and follow the cascade to a fixpoint.\n`
      );
      return 0;
    }

    const applied = [];
    for (const doc of owed) {
      if (await applyRedate({ root, docPath: doc.path, date, note })) {
        applied.push(doc.path);
      } else {
        process.stderr.write(`[${SURFACE}] WARN: no metadata table in ${doc.path}; left alone\n`);
      }
    }
    if (applied.length === 0) {
      process.stderr.write(`[${SURFACE}] round ${round}: nothing could be re-dated; stopping\n`);
      return 1;
    }

    try {
      await formatAndCommit({
        root,
        paths: applied,
        message: `docs(governance): review triage round ${round}\n\n${note}`,
      });
    } catch (error) {
      process.stderr.write(
        `[${SURFACE}] round ${round}: could not format and commit — ` +
          `${(error.stdout || error.stderr || error.message).trim().split('\n')[0]}\n` +
          `[${SURFACE}] ${applied.length} document(s) are re-dated but UNCOMMITTED. The gates read a\n` +
          `[${SURFACE}] document's date from the working tree and its upstream's from git, so they\n` +
          `[${SURFACE}] will now report green while the rest of the cascade is still hidden. Commit\n` +
          `[${SURFACE}] these, then re-run to continue.\n`
      );
      return 1;
    }
    total += applied.length;
  }

  process.stderr.write(
    `[${SURFACE}] still owed after ${maxRounds} rounds — stopping. Raise --max-rounds only if ` +
      `the set is still shrinking; a set that repeats is a bug, not a deeper cascade.\n`
  );
  return 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    process.exit(await runRedateCli());
  } catch (error) {
    // A stack trace here is never the useful part: the failure is always either
    // a gate that could not run or a git/format command that refused.
    process.stderr.write(`[${SURFACE}] ${error.message.trim().split('\n')[0]}\n`);
    process.exit(2);
  }
}
