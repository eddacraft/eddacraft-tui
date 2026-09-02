import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';

const SCRIPT = resolve(import.meta.dirname, 'run-markdownlint.mjs');

function makeFixture(files) {
  const root = mkdtempSync(join(tmpdir(), 'mdl-honest-'));
  for (const [rel, body] of Object.entries(files)) {
    const abs = join(root, rel);
    mkdirSync(resolve(abs, '..'), { recursive: true });
    writeFileSync(abs, body, 'utf8');
  }
  return root;
}

function run(root, args) {
  return spawnSync(process.execPath, [SCRIPT, '--cwd', root, ...args], {
    encoding: 'utf8',
  });
}

const IGNORE = ['plans/**', 'ACKNOWLEDGEMENTS.md', ''].join('\n');
const CLEAN = '# Title\n\nSome prose.\n';
const DIRTY = '# Title\n\nTrailing space. \n';

test('named ignored file fails instead of exiting 0', () => {
  const root = makeFixture({
    '.markdownlintignore': IGNORE,
    'plans/index.aps.md': CLEAN,
    'README.md': CLEAN,
  });
  try {
    const r = run(root, ['plans/index.aps.md']);
    assert.notEqual(r.status, 0, r.stdout + r.stderr);
    assert.match(r.stderr, /named input excluded/i);
    assert.match(`${r.stdout}\n${r.stderr}`, /0 files examined/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('named linted file still passes when clean', () => {
  const root = makeFixture({
    '.markdownlintignore': IGNORE,
    'README.md': CLEAN,
  });
  try {
    const r = run(root, ['README.md']);
    assert.equal(r.status, 0, r.stdout + r.stderr);
    assert.match(r.stdout, /1 files checked/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('named linted file still fails on a real violation', () => {
  const root = makeFixture({
    '.markdownlintignore': IGNORE,
    'README.md': DIRTY,
  });
  try {
    const r = run(root, ['README.md']);
    assert.equal(r.status, 1, r.stdout + r.stderr);
    assert.match(r.stdout, /1 files checked/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('directory sweep reports a file count and ignores excluded trees', () => {
  const root = makeFixture({
    '.markdownlintignore': IGNORE,
    'plans/index.aps.md': DIRTY,
    'README.md': CLEAN,
  });
  try {
    const r = run(root, ['.']);
    assert.equal(r.status, 0, r.stdout + r.stderr);
    assert.match(r.stdout, /1 files checked/);
    assert.doesNotMatch(r.stderr, /named input excluded/i);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('omit-ignored drops named ignored files instead of treating them as a pass', () => {
  const root = makeFixture({
    '.markdownlintignore': IGNORE,
    'plans/index.aps.md': DIRTY,
    'README.md': CLEAN,
  });
  try {
    const r = run(root, ['--omit-ignored', 'plans/index.aps.md', 'README.md']);
    assert.equal(r.status, 0, r.stdout + r.stderr);
    assert.match(r.stdout, /1 files checked/);
    assert.doesNotMatch(r.stderr, /named input excluded/i);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('directory sweep does not lint hidden directories, matching markdownlint-cli', () => {
  const root = makeFixture({
    '.markdownlintignore': IGNORE,
    'README.md': CLEAN,
    '.clawpatch/report.md': DIRTY,
  });
  try {
    const r = run(root, ['.']);
    assert.equal(r.status, 0, r.stdout + r.stderr);
    assert.match(r.stdout, /1 files checked/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('directory sweep that matches nothing is an explicit zero, not a pass-by-silence', () => {
  const root = makeFixture({
    '.markdownlintignore': '**/*.md\n',
    'README.md': CLEAN,
  });
  try {
    const r = run(root, ['.']);
    assert.equal(r.status, 0, r.stdout + r.stderr);
    assert.match(r.stdout, /0 files checked/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
