import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';

const SCRIPT = resolve(import.meta.dirname, 'pr-required-status.mjs');

function run(required, checks, threads) {
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
