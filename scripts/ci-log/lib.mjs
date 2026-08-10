// Shared helpers for continuous-improvement log durability (CIB-191).
//
// Pending notes live under the git common directory so every Worktrunk
// worktree of a clone shares one queue. That survives worktree removal and
// never dirties a feature PR as an "unrelated" tracked file.
//
// Tracked log: plans/reviews/continuous-improvement-log.md (merge=union).

import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import {
  closeSync,
  constants,
  existsSync,
  fstatSync,
  fsyncSync,
  mkdirSync,
  openSync,
  readdirSync,
  readFileSync,
  readSync,
  renameSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join, resolve } from 'node:path';

export const TRACKED_LOG_REL = 'plans/reviews/continuous-improvement-log.md';
export const PENDING_DIR_NAME = 'anvil/ci-log-pending';
export const WATERMARK_RE = /^>\s*\*\*Last triaged:\*\*\s*(\d{4}-\d{2}-\d{2}|never)\s*$/m;

const TRACKED_LOG_LOCK_NAME = 'anvil/ci-log-tracked.lock';
const DEFAULT_LOCK_ACQUIRE_TIMEOUT_MS = 10_000;
const LOCK_RETRY_MS = 25;
const LOCK_SLEEP_BUFFER = new Int32Array(new SharedArrayBuffer(4));
const LOCK_OWNER_RE = /^\d+:[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const MAX_LOCK_OWNER_BYTES = 128;

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
    throw new Error(`git ${args.join(' ')} failed: ${stderr}`, { cause: error });
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

export function trackedLogLockPath(cwd = process.cwd()) {
  return join(resolveGitCommonDir(cwd), TRACKED_LOG_LOCK_NAME);
}

function sleepSync(milliseconds) {
  Atomics.wait(LOCK_SLEEP_BUFFER, 0, 0, milliseconds);
}

function lockAcquireTimeoutMs() {
  const configured = process.env.ANVIL_CI_LOG_TEST_LOCK_TIMEOUT_MS;
  if (configured === undefined) return DEFAULT_LOCK_ACQUIRE_TIMEOUT_MS;
  if (!/^[1-9]\d{0,4}$/.test(configured)) {
    throw new Error('ANVIL_CI_LOG_TEST_LOCK_TIMEOUT_MS must be an integer from 1 to 99999');
  }
  return Number(configured);
}

function lockOwnerForDiagnostic(path) {
  let fd;
  try {
    const noFollow = constants.O_NOFOLLOW ?? 0;
    fd = openSync(path, constants.O_RDONLY | noFollow);
    if (!fstatSync(fd).isFile()) return '<invalid>';
    const buffer = Buffer.alloc(MAX_LOCK_OWNER_BYTES);
    const bytesRead = readSync(fd, buffer, 0, buffer.length, 0);
    if (bytesRead === buffer.length) return '<invalid>';
    const owner = buffer.toString('utf8', 0, bytesRead).trim();
    return LOCK_OWNER_RE.test(owner) ? owner : '<invalid>';
  } catch {
    // The holder may release the lock between EEXIST and this diagnostic. On
    // platforms with O_NOFOLLOW, a substituted symlink is rejected here too.
    return '<unavailable>';
  } finally {
    if (fd !== undefined) {
      try {
        closeSync(fd);
      } catch {
        // Owner reporting is diagnostic-only; preserve the timeout error.
      }
    }
  }
}

function acquireTrackedLogLock(cwd = process.cwd()) {
  const path = trackedLogLockPath(cwd);
  const token = `${process.pid}:${randomUUID()}`;
  const deadline = Date.now() + lockAcquireTimeoutMs();
  mkdirSync(dirname(path), { recursive: true });

  for (;;) {
    let fd;
    try {
      fd = openSync(path, 'wx', 0o600);
      writeFileSync(fd, `${token}\n`, 'utf8');
      closeSync(fd);
      return { path, token };
    } catch (error) {
      if (fd !== undefined) {
        try {
          closeSync(fd);
        } catch {
          // The descriptor may already have been closed after a successful write.
        }
        rmSync(path, { force: true });
      }
      if (!error || error.code !== 'EEXIST') throw error;
      if (Date.now() >= deadline) {
        const owner = lockOwnerForDiagnostic(path);
        throw new Error(`Timed out waiting for tracked CI-log lock at ${path} (owner: ${owner})`);
      }
      sleepSync(Math.min(LOCK_RETRY_MS, Math.max(1, deadline - Date.now())));
    }
  }
}

function releaseTrackedLogLock(lock) {
  let owner;
  try {
    owner = readText(lock.path).trim();
  } catch (error) {
    throw new Error(`Tracked CI-log lock disappeared before release: ${lock.path}`, {
      cause: error,
    });
  }
  if (owner !== lock.token) {
    throw new Error(`Tracked CI-log lock ownership changed before release: ${lock.path}`);
  }
  rmSync(lock.path);
}

function withTrackedLogLock(cwd, mutation) {
  const lock = acquireTrackedLogLock(cwd);
  let result;
  let mutationError;
  try {
    result = mutation();
  } catch (error) {
    mutationError = error;
  }
  try {
    releaseTrackedLogLock(lock);
  } catch (releaseError) {
    if (mutationError) {
      throw new AggregateError(
        [mutationError, releaseError],
        'Tracked CI-log mutation failed and its lock could not be released'
      );
    }
    throw releaseError;
  }
  if (mutationError) throw mutationError;
  return result;
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

function assertRealCalendarDate(date, { allowNever = false } = {}) {
  if (allowNever && date === 'never') return date;
  if (typeof date !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(date)) {
    throw new Error(`invalid date: ${date}`);
  }
  const parsed = new Date(`${date}T00:00:00.000Z`);
  if (Number.isNaN(parsed.getTime()) || parsed.toISOString().slice(0, 10) !== date) {
    throw new Error(`invalid date: ${date}`);
  }
  return date;
}

/** Normalise an entry body to start with ### heading and end with one blank line. */
export function normaliseEntry(raw) {
  let text = String(raw).replace(/\r\n/g, '\n').trim();
  if (!text) {
    throw new Error('CI-log entry is empty');
  }
  // Heading must be the first non-empty line (no multiline search — a later
  // ### heading must not validate a malformed prefix).
  const headingDate = text.match(/^###\s+(\d{4}-\d{2}-\d{2})\b/)?.[1];
  if (!headingDate) {
    throw new Error('CI-log entry must start with a heading like `### YYYY-MM-DD — agent`');
  }
  assertRealCalendarDate(headingDate);
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
  const temp = join(dir, `.ci-log-pending-tmp-${process.pid}-${randomUUID()}`);
  let fd;
  let published = false;
  try {
    fd = openSync(temp, 'wx', 0o600);
    writeFileSync(fd, body, 'utf8');
    fsyncSync(fd);
    closeSync(fd);
    fd = undefined;

    // The tracked-log lock also serialises final-name selection against every
    // harvest. Only the complete inode becomes visible when rename succeeds.
    return withTrackedLogLock(cwd, () => {
      let n = 0;
      for (;;) {
        const path = join(dir, n === 0 ? `${ts}-${slug}.md` : `${ts}-${slug}-${n}.md`);
        if (existsSync(path)) {
          n += 1;
          continue;
        }
        renameSync(temp, path);
        published = true;
        return path;
      }
    });
  } catch (error) {
    if (fd !== undefined) {
      try {
        closeSync(fd);
      } catch {
        // Preserve the publication failure; cleanup below owns this temp.
      }
    }
    if (!published) rmSync(temp, { force: true });
    throw error;
  }
}

export function appendTrackedEntry(entry, { cwd = process.cwd() } = {}) {
  const body = normaliseEntry(entry);
  return withTrackedLogLock(cwd, () => {
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
  });
}

export function readWatermark(cwd = process.cwd()) {
  const path = trackedLogPath(cwd);
  if (!existsSync(path)) return null;
  const text = readText(path);
  const match = text.match(WATERMARK_RE);
  return match ? match[1] : null;
}

export function setWatermark(date, { cwd = process.cwd() } = {}) {
  assertRealCalendarDate(date, { allowNever: true });
  return withTrackedLogLock(cwd, () => {
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
  });
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
  if (date && date !== 'never') assertRealCalendarDate(date);
  const text = readText(trackedLogPath(cwd));
  const entries = parseEntries(text);
  if (!date || date === 'never') return entries;
  return entries.filter((entry) => entry.date && entry.date >= date);
}

export function harvestPending({ cwd = process.cwd(), dryRun = false } = {}) {
  if (dryRun) {
    const files = listPendingFiles(cwd);
    const entries = files.map((file) => ({
      file,
      body: normaliseEntry(readText(file)),
    }));
    return { harvested: entries.length, paths: files, dryRun: true };
  }

  return withTrackedLogLock(cwd, () => {
    const files = listPendingFiles(cwd);
    if (files.length === 0) {
      return { harvested: 0, paths: [], dryRun: false };
    }
    const entries = files.map((file) => ({
      file,
      body: normaliseEntry(readText(file)),
    }));
    const logPath = trackedLogPath(cwd);
    let existing = readText(logPath);
    if (!existing.endsWith('\n')) existing += '\n';
    if (!existing.endsWith('\n\n')) existing += '\n';
    const block = entries.map((e) => e.body.replace(/\n+$/, '\n') + '\n').join('');
    const tmp = `${logPath}.harvest-${process.pid}-${randomUUID()}.tmp`;
    try {
      writeText(tmp, existing + block);
      renameSync(tmp, logPath);
      for (const { file } of entries) {
        rmSync(file, { force: true });
      }
    } finally {
      rmSync(tmp, { force: true });
    }
    return { harvested: entries.length, paths: files, dryRun: false, logPath };
  });
}

export function pendingSummary(cwd = process.cwd()) {
  const files = listPendingFiles(cwd);
  return {
    dir: pendingDir(cwd),
    count: files.length,
    files: files.map((f) => f.split(/[\\/]/).pop()),
  };
}
