// Shared helpers for continuous-improvement log durability (CIB-191).
//
// Pending notes live under the git common directory so every Worktrunk
// worktree of a clone shares one queue. That survives worktree removal and
// never dirties a feature PR as an "unrelated" tracked file.
//
// Tracked log: plans/reviews/continuous-improvement-log.md (merge=union).

import { execFileSync } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join, resolve } from 'node:path';

export const TRACKED_LOG_REL = 'plans/reviews/continuous-improvement-log.md';
export const PENDING_DIR_NAME = 'anvil/ci-log-pending';
export const WATERMARK_RE = /^>\s*\*\*Last triaged:\*\*\s*(\d{4}-\d{2}-\d{2}|never)\s*$/m;

export function runGit(args, { cwd, allowFail = false } = {}) {
  try {
    return execFileSync('git', args, {
      cwd,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    }).trim();
  } catch (error) {
    if (allowFail) return '';
    const stderr = error.stderr?.toString?.() ?? error.message;
    throw new Error(`git ${args.join(' ')} failed: ${stderr}`);
  }
}

export function resolveRepoRoot(cwd = process.cwd()) {
  const root = runGit(['rev-parse', '--show-toplevel'], { cwd });
  return root || resolve(cwd);
}

export function resolveGitCommonDir(cwd = process.cwd()) {
  const common = runGit(['rev-parse', '--git-common-dir'], { cwd });
  return resolve(cwd, common);
}

export function pendingDir(cwd = process.cwd()) {
  return join(resolveGitCommonDir(cwd), PENDING_DIR_NAME);
}

export function trackedLogPath(cwd = process.cwd()) {
  return join(resolveRepoRoot(cwd), TRACKED_LOG_REL);
}

export function ensurePendingDir(cwd = process.cwd()) {
  const dir = pendingDir(cwd);
  mkdirSync(dir, { recursive: true });
  return dir;
}

export function readText(path) {
  return readFileSync(path, 'utf8');
}

export function writeText(path, text) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text, 'utf8');
}

export function listPendingFiles(cwd = process.cwd()) {
  const dir = pendingDir(cwd);
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((name) => name.endsWith('.md'))
    .sort()
    .map((name) => join(dir, name));
}

export function utcStamp(date = new Date()) {
  const iso = date.toISOString(); // 2026-07-12T01:02:03.456Z
  return iso.replace(/[-:]/g, '').replace(/\.\d{3}Z$/, 'Z');
}

export function todayUtc(date = new Date()) {
  return date.toISOString().slice(0, 10);
}

/** Normalise an entry body to start with ### heading and end with one blank line. */
export function normaliseEntry(raw) {
  let text = String(raw).replace(/\r\n/g, '\n').trim();
  if (!text) {
    throw new Error('CI-log entry is empty');
  }
  if (!/^###\s+\d{4}-\d{2}-\d{2}\b/m.test(text)) {
    throw new Error('CI-log entry must start with a heading like `### YYYY-MM-DD — agent`');
  }
  // Drop leading blank lines so merge=union neighbours stay separated cleanly.
  text = text.replace(/^\n+/, '');
  if (!text.endsWith('\n')) text += '\n';
  // Exactly one trailing blank line after the entry body.
  text = text.replace(/\n+$/, '\n') + '\n';
  return text;
}

export function buildEntryFromFields({
  date = todayUtc(),
  agent = 'other',
  task,
  outcome = '',
  worked = '',
  failed = '',
  friction = '',
  improvement = '',
  followUp = 'none',
  title = '',
} = {}) {
  if (!task || !String(task).trim()) {
    throw new Error('--task is required when building an entry from fields');
  }
  const heading = title ? `### ${date} — ${agent} — ${title}` : `### ${date} — ${agent}`;
  const lines = [
    heading,
    '',
    `- **Task:** ${String(task).trim()}`,
    `- **Outcome:** ${String(outcome || '—').trim()}`,
    `- **Worked:** ${String(worked || '—').trim()}`,
    `- **Failed:** ${String(failed || 'none').trim()}`,
    `- **Friction:** ${String(friction || 'none').trim()}`,
    `- **Improvement:** ${String(improvement || 'none').trim()}`,
    `- **Follow-up:** ${String(followUp || 'none').trim()}`,
  ];
  return normaliseEntry(lines.join('\n'));
}

export function slugFromEntry(entry) {
  const heading = entry.match(/^###\s+(.+)$/m)?.[1] ?? 'entry';
  return heading
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 60);
}

export function writePendingEntry(entry, { cwd = process.cwd(), stamp } = {}) {
  const body = normaliseEntry(entry);
  const dir = ensurePendingDir(cwd);
  const ts = stamp ?? utcStamp();
  const slug = slugFromEntry(body) || 'entry';
  let path = join(dir, `${ts}-${slug}.md`);
  let n = 0;
  while (existsSync(path)) {
    n += 1;
    path = join(dir, `${ts}-${slug}-${n}.md`);
  }
  writeText(path, body);
  return path;
}

export function appendTrackedEntry(entry, { cwd = process.cwd() } = {}) {
  const body = normaliseEntry(entry);
  const path = trackedLogPath(cwd);
  if (!existsSync(path)) {
    throw new Error(`Tracked CI-log missing at ${TRACKED_LOG_REL}`);
  }
  let existing = readText(path);
  if (!existing.endsWith('\n')) existing += '\n';
  // Ensure a blank line between previous content and new entry when needed.
  if (!existing.endsWith('\n\n')) existing += '\n';
  writeText(path, existing + body);
  return path;
}

export function readWatermark(cwd = process.cwd()) {
  const path = trackedLogPath(cwd);
  if (!existsSync(path)) return null;
  const text = readText(path);
  const match = text.match(WATERMARK_RE);
  return match ? match[1] : null;
}

export function setWatermark(date, { cwd = process.cwd() } = {}) {
  const path = trackedLogPath(cwd);
  const text = readText(path);
  const line = `> **Last triaged:** ${date}`;
  let next;
  if (WATERMARK_RE.test(text)) {
    next = text.replace(WATERMARK_RE, line);
  } else {
    // Insert after the concurrent-writes blockquote, or after the title.
    const concurrentEnd = text.indexOf('\n## Template');
    if (concurrentEnd !== -1) {
      next = `${text.slice(0, concurrentEnd)}\n${line}\n${text.slice(concurrentEnd)}`;
    } else {
      next = text.replace(/^(# [^\n]+\n)/, `$1\n${line}\n`);
    }
  }
  writeText(path, next);
  return date;
}

/**
 * Split tracked log into entries. Returns array of { heading, body, start, end }.
 * body includes the heading line and content up to (not including) next heading.
 */
export function parseEntries(text) {
  const lines = text.replace(/\r\n/g, '\n').split('\n');
  const entries = [];
  let current = null;
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (/^###\s+\d{4}-\d{2}-\d{2}\b/.test(line)) {
      if (current) {
        current.endLine = i;
        current.body = lines.slice(current.startLine, i).join('\n').replace(/\n+$/, '') + '\n';
        entries.push(current);
      }
      current = {
        heading: line.replace(/^###\s+/, ''),
        startLine: i,
        date: line.match(/^###\s+(\d{4}-\d{2}-\d{2})/)?.[1] ?? null,
      };
    }
  }
  if (current) {
    current.endLine = lines.length;
    current.body = lines.slice(current.startLine).join('\n').replace(/\n+$/, '') + '\n';
    entries.push(current);
  }
  return entries;
}

export function entriesSince(date, { cwd = process.cwd() } = {}) {
  const text = readText(trackedLogPath(cwd));
  const entries = parseEntries(text);
  if (!date || date === 'never') return entries;
  return entries.filter((entry) => entry.date && entry.date >= date);
}

export function harvestPending({ cwd = process.cwd(), dryRun = false } = {}) {
  const files = listPendingFiles(cwd);
  if (files.length === 0) {
    return { harvested: 0, paths: [], dryRun };
  }
  const entries = files.map((file) => ({
    file,
    body: normaliseEntry(readText(file)),
  }));
  if (dryRun) {
    return { harvested: entries.length, paths: files, dryRun: true };
  }
  const logPath = trackedLogPath(cwd);
  let existing = readText(logPath);
  if (!existing.endsWith('\n')) existing += '\n';
  if (!existing.endsWith('\n\n')) existing += '\n';
  const block = entries.map((e) => e.body.replace(/\n+$/, '\n') + '\n').join('');
  // Atomic-ish write: write temp beside target then rename.
  const tmp = `${logPath}.harvest-tmp`;
  writeText(tmp, existing + block);
  renameSync(tmp, logPath);
  for (const { file } of entries) {
    rmSync(file, { force: true });
  }
  return { harvested: entries.length, paths: files, dryRun: false, logPath };
}

export function pendingSummary(cwd = process.cwd()) {
  const files = listPendingFiles(cwd);
  return {
    dir: pendingDir(cwd),
    count: files.length,
    files: files.map((f) => f.split(/[\\/]/).pop()),
  };
}
