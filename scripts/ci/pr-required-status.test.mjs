import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';

const SCRIPT = resolve(import.meta.dirname, 'pr-required-status.mjs');

function run(required, checks, threads, merge) {
  const dir = mkdtempSync(join(tmpdir(), 'pr-req-'));
  const requiredPath = join(dir, 'required.json');
  const checksPath = join(dir, 'checks.json');
  writeFileSync(requiredPath, JSON.stringify(required));
  writeFileSync(checksPath, JSON.stringify(checks));
  const argv = [SCRIPT, '--required-json', requiredPath, '--checks-json', checksPath];
  if (threads !== undefined) {
    const threadsPath = join(dir, 'threads.json');
    writeFileSync(threadsPath, JSON.stringify(threads));
    argv.push('--threads-json', threadsPath);
  }
  if (merge !== undefined) {
    const mergePath = join(dir, 'merge.json');
    writeFileSync(mergePath, JSON.stringify(merge));
    argv.push('--merge-json', mergePath);
  }
  try {
    return spawnSync(process.execPath, argv, { encoding: 'utf8' });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const GREEN = [
  { name: 'Docs Lint', status: 'COMPLETED', conclusion: 'SUCCESS' },
  { name: 'Lint & Format', status: 'COMPLETED', conclusion: 'SUCCESS' },
];
const REQUIRED = ['Docs Lint', 'Lint & Format'];

test('pending required context is not finished, not a pass', () => {
  const r = run(
    ['Docs Lint', 'Lint & Format'],
    [{ name: 'Lint & Format', status: 'COMPLETED', conclusion: 'SUCCESS' }]
  );
  assert.equal(r.status, 2, r.stdout + r.stderr);
  assert.match(r.stdout, /not finished/i);
  assert.match(r.stdout, /Docs Lint/);
  assert.doesNotMatch(r.stdout, /all required contexts reported and passed/i);
});

test('fully reported green required set is success', () => {
  const r = run(
    ['Docs Lint', 'Lint & Format'],
    [
      { name: 'Docs Lint', status: 'COMPLETED', conclusion: 'SUCCESS' },
      { name: 'Lint & Format', status: 'COMPLETED', conclusion: 'SUCCESS' },
    ]
  );
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(r.stdout, /all required contexts reported and passed/);
});

test('reported failure is a failure, not a pending sample', () => {
  const r = run(['Docs Lint'], [{ name: 'Docs Lint', status: 'COMPLETED', conclusion: 'FAILURE' }]);
  assert.equal(r.status, 1, r.stdout + r.stderr);
  assert.match(r.stdout, /failed: Docs Lint/);
  assert.doesNotMatch(r.stdout, /not finished/i);
});

test('in-progress required context is not finished', () => {
  const r = run(['Docs Lint'], [{ name: 'Docs Lint', status: 'IN_PROGRESS', conclusion: null }]);
  assert.equal(r.status, 2, r.stdout + r.stderr);
  assert.match(r.stdout, /not finished/i);
  assert.match(r.stdout, /Docs Lint/);
});

// A branch whose ruleset requires thread resolution is not mergeable on green
// checks alone. Observed on PR #4341: every required context reported and
// passed while GitHub held the PR at `BLOCKED` on one unresolved thread, so a
// checks-only reading called a still-blocked PR a pass.
test('unresolved review thread blocks even when every required check is green', () => {
  const r = run(REQUIRED, GREEN, [{ isResolved: false }, { isResolved: true }]);
  assert.equal(r.status, 3, r.stdout + r.stderr);
  assert.match(r.stdout, /blocked: 1 unresolved review thread\b/);
  assert.doesNotMatch(r.stdout, /all required contexts reported and passed/);
});

test('resolved threads with green checks is a pass, and says so', () => {
  const r = run(REQUIRED, GREEN, [{ isResolved: true }, { isResolved: true }]);
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(r.stdout, /all required contexts reported and passed/);
  assert.match(r.stdout, /review threads resolved/);
});

test('thread state absent means the gate does not apply, not that it passed', () => {
  const r = run(REQUIRED, GREEN);
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(r.stdout, /all required contexts reported and passed/);
  assert.doesNotMatch(r.stdout, /review threads resolved/);
});

// Pending checks outrank an open thread: the honest report is "not finished",
// because the checks have not reached a verdict at all yet.
test('pending checks still report not-finished even with an open thread', () => {
  const r = run(
    REQUIRED,
    [{ name: 'Lint & Format', status: 'COMPLETED', conclusion: 'SUCCESS' }],
    [{ isResolved: false }]
  );
  assert.equal(r.status, 2, r.stdout + r.stderr);
  assert.match(r.stdout, /not finished/i);
});

// --- CIB-404: a PR with no merge candidate ------------------------------------
// When a PR conflicts with its base, GitHub builds no merge commit, so no
// workflow runs and no required context can ever report. Reporting that as
// "not finished" tells a polling caller to wait when the truth is "repair the
// base" — observed on PR #4372, where the poller burned its full 52-minute
// budget on a branch that needed a thirty-second rebase.
const NOTHING_REPORTED = [];

test('a conflicting PR is its own exit code, not "not finished"', () => {
  const r = run(REQUIRED, NOTHING_REPORTED, undefined, {
    mergeable: 'CONFLICTING',
    mergeStateStatus: 'DIRTY',
  });
  assert.equal(r.status, 4, r.stdout + r.stderr);
  assert.match(r.stdout, /conflict/i);
  assert.doesNotMatch(r.stdout, /not finished/i);
  assert.doesNotMatch(r.stdout, /all required contexts reported and passed/i);
});

test('the conflict report names the base repair, not the missing contexts', () => {
  const r = run(REQUIRED, NOTHING_REPORTED, undefined, {
    mergeable: 'CONFLICTING',
    mergeStateStatus: 'DIRTY',
  });
  assert.equal(r.status, 4, r.stdout + r.stderr);
  assert.match(r.stdout, /mergeable: CONFLICTING/);
  assert.match(r.stdout, /mergeStateStatus: DIRTY/);
  assert.match(r.stdout, /rebase|repair|update the branch/i);
});

test('mergeStateStatus DIRTY alone is a conflict even if mergeable lags', () => {
  const r = run(REQUIRED, NOTHING_REPORTED, undefined, {
    mergeable: 'UNKNOWN',
    mergeStateStatus: 'DIRTY',
  });
  assert.equal(r.status, 4, r.stdout + r.stderr);
  assert.match(r.stdout, /conflict/i);
});

// The other arm: the tool must not start calling healthy, merely-early PRs
// conflicted. A mergeable PR whose checks genuinely have not started is still
// exit 2 and still names what is missing.
test('a healthy PR whose checks have not started is still not finished', () => {
  const r = run(REQUIRED, NOTHING_REPORTED, undefined, {
    mergeable: 'MERGEABLE',
    mergeStateStatus: 'CLEAN',
  });
  assert.equal(r.status, 2, r.stdout + r.stderr);
  assert.match(r.stdout, /not finished/i);
  assert.match(r.stdout, /Docs Lint/);
  assert.match(r.stdout, /Lint & Format/);
  assert.doesNotMatch(r.stdout, /conflict/i);
});

test('a mergeable PR with green checks still passes', () => {
  const r = run(REQUIRED, GREEN, undefined, {
    mergeable: 'MERGEABLE',
    mergeStateStatus: 'BLOCKED',
  });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(r.stdout, /all required contexts reported and passed/);
});

// GitHub computes mergeability asynchronously, so UNKNOWN is itself a
// not-yet-a-verdict: it is neither a conflict nor evidence that the checks are
// merely early. It gets its own message so a caller polling on exit 2 can tell
// the two apart, and is never reported as a conflict.
test('mergeable UNKNOWN is its own case, not a conflict', () => {
  const r = run(REQUIRED, NOTHING_REPORTED, undefined, {
    mergeable: 'UNKNOWN',
    mergeStateStatus: 'UNKNOWN',
  });
  assert.equal(r.status, 2, r.stdout + r.stderr);
  assert.match(r.stdout, /mergeability (is )?(still )?(not|un)/i);
  assert.match(r.stdout, /UNKNOWN/);
  assert.doesNotMatch(r.stdout, /conflict/i);
  assert.doesNotMatch(r.stdout, /all required contexts reported and passed/i);
});

test('mergeable UNKNOWN is distinguishable from plain not-started checks', () => {
  const unknown = run(REQUIRED, NOTHING_REPORTED, undefined, {
    mergeable: 'UNKNOWN',
    mergeStateStatus: 'UNKNOWN',
  });
  const early = run(REQUIRED, NOTHING_REPORTED, undefined, {
    mergeable: 'MERGEABLE',
    mergeStateStatus: 'CLEAN',
  });
  assert.equal(unknown.status, 2, unknown.stdout + unknown.stderr);
  assert.equal(early.status, 2, early.stdout + early.stderr);
  assert.notEqual(unknown.stdout, early.stdout);
});

// Absent mergeability data must not invent a verdict: every pre-CIB-404 caller
// passes no merge state at all and must keep its old behaviour.
test('absent merge state leaves the existing classification untouched', () => {
  const r = run(REQUIRED, NOTHING_REPORTED);
  assert.equal(r.status, 2, r.stdout + r.stderr);
  assert.match(r.stdout, /not finished/i);
  assert.doesNotMatch(r.stdout, /conflict/i);
  assert.doesNotMatch(r.stdout, /mergeab/i);
});
