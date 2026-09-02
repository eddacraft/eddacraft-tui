import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';

const SCRIPT = resolve(import.meta.dirname, 'pr-required-status.mjs');

function run(required, checks) {
  const dir = mkdtempSync(join(tmpdir(), 'pr-req-'));
  const requiredPath = join(dir, 'required.json');
  const checksPath = join(dir, 'checks.json');
  writeFileSync(requiredPath, JSON.stringify(required));
  writeFileSync(checksPath, JSON.stringify(checks));
  try {
    return spawnSync(
      process.execPath,
      [SCRIPT, '--required-json', requiredPath, '--checks-json', checksPath],
      { encoding: 'utf8' }
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

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
