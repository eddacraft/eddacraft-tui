import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { promisify } from 'node:util';

import {
  collectGitDiffPaths,
  parseGitNameStatusZ,
} from './git-name-status-paths.mjs';

const execFileAsync = promisify(execFile);

test('parses add, modify, delete, copy, and rename endpoints from NUL status records', () => {
  assert.deepEqual(
    parseGitNameStatusZ(
      ['A', 'docs/new.md', 'M', 'crates/example/src/lib.rs', 'D', 'scripts/old.sh', ''].join('\0')
    ),
    ['docs/new.md', 'crates/example/src/lib.rs', 'scripts/old.sh']
  );
  assert.deepEqual(parseGitNameStatusZ(['R100', 'old/exact.ts', 'new/exact.ts', ''].join('\0')), [
    'old/exact.ts',
    'new/exact.ts',
  ]);
  assert.deepEqual(parseGitNameStatusZ(['C80', 'src/kept.ts', 'src/copy.ts', ''].join('\0')), [
    'src/kept.ts',
    'src/copy.ts',
  ]);
});

test('fails closed on unknown or truncated name-status records', () => {
  assert.throws(() => parseGitNameStatusZ(['U', 'unmerged.ts', ''].join('\0')), /unrecognised/u);
  assert.throws(() => parseGitNameStatusZ(['R100', 'only-source.ts', ''].join('\0')), /incomplete/u);
  assert.throws(() => parseGitNameStatusZ(['A', ''].join('\0')), /incomplete/u);
  assert.throws(() => parseGitNameStatusZ(['X99', 'mystery.ts', ''].join('\0')), /unrecognised/u);
});

test('the real git collector retains both rename endpoints and ordinary ACDM paths', async (t) => {
  const root = await mkdtemp(join(tmpdir(), 'git-name-status-'));
  t.after(() => rm(root, { recursive: true, force: true }));

  const commit = async (message) => {
    await execFileAsync('git', ['add', '-A'], { cwd: root });
    await execFileAsync(
      'git',
      [
        '-c',
        'user.name=DOCRB test',
        '-c',
        'user.email=docrb@example.invalid',
        'commit',
        '--quiet',
        '-m',
        message,
      ],
      { cwd: root }
    );
  };

  await mkdir(join(root, 'crates', 'example', 'src'), { recursive: true });
  await writeFile(join(root, 'crates', 'example', 'src', 'lib.rs'), 'export const value = 1;\n');
  await writeFile(join(root, 'kept.ts'), 'export const kept = true;\n');
  await execFileAsync('git', ['init', '--quiet', '-b', 'main'], { cwd: root });
  await commit('base');
  const base = (await execFileAsync('git', ['rev-parse', 'HEAD'], { cwd: root })).stdout.trim();

  await execFileAsync(
    'git',
    ['mv', 'crates/example/src/lib.rs', 'crates/example/src/renamed.rs'],
    { cwd: root }
  );
  await writeFile(join(root, 'added.ts'), 'export const added = true;\n');
  await writeFile(join(root, 'kept.ts'), 'export const kept = false;\n');
  await commit('rename plus add and modify');

  const nameOnly = (
    await execFileAsync('git', ['diff', '--name-only', '--diff-filter=ACDMR', `${base}...HEAD`], {
      cwd: root,
    })
  ).stdout
    .split(/\r?\n/u)
    .filter(Boolean);
  assert.ok(
    nameOnly.includes('crates/example/src/renamed.rs'),
    `name-only should keep the destination: ${nameOnly.join(',')}`
  );
  assert.equal(
    nameOnly.includes('crates/example/src/lib.rs'),
    false,
    'name-only drops the rename source — that is the fail-open this collector closes'
  );

  const paths = await collectGitDiffPaths({
    cwd: root,
    extraArgs: [`${base}...HEAD`],
  });
  assert.deepEqual(
    [...paths].sort(),
    ['added.ts', 'crates/example/src/lib.rs', 'crates/example/src/renamed.rs', 'kept.ts'].sort()
  );
});
