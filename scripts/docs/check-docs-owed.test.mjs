// CIB-383: the change-scoped docs-owed gate must diff to the commit under test,
// not to whatever `HEAD` happens to be. On a `pull_request` event
// `actions/checkout` checks out `refs/pull/N/merge`, so bare `HEAD` is this PR
// merged into current main and carries main's newer commits. PR #4247 was
// failed over `crates/anvil-cli/ARCHITECTURE.md`, a file it never touched,
// after main gained 94 commits behind its back.
import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const CHECKER = new URL('./check-docs-owed.mjs', import.meta.url).pathname;

async function gitIn(root, args) {
  return (await execFileAsync('git', args, { cwd: root })).stdout.trim();
}

async function commit(root, message) {
  await gitIn(root, ['add', '-A']);
  await gitIn(root, [
    '-c',
    'user.name=DOCRB test',
    '-c',
    'user.email=docrb@example.invalid',
    'commit',
    '--quiet',
    '-m',
    message,
  ]);
}

/**
 * A repository shaped like the failure: a governed document whose declared
 * upstream only `main` moves, and a PR head that is behind `main` and touches
 * nothing the document depends on.
 */
async function buildMergeRefRepo() {
  const root = await mkdtemp(join(tmpdir(), 'docs-owed-head-'));
  await mkdir(join(root, 'crates', 'example', 'src'), { recursive: true });
  await mkdir(join(root, 'docs', 'architecture'), { recursive: true });
  await writeFile(join(root, 'crates', 'example', 'src', 'lib.rs'), 'export const value = 1;\n');
  await writeFile(
    join(root, 'docs', 'architecture', 'owner.md'),
    `# Head scoping owner

| Type  | Authority     | Owner | Status | Freshness                                                     |
| ----- | ------------- | ----- | ------ | ------------------------------------------------------------- |
| Guide | Authoritative | DOCRB | Live   | Last reviewed 2020-01-01 against \`crates/example/src/lib.rs\` |

| Upstream                      | Downstream |
| ----------------------------- | ---------- |
| \`crates/example/src/lib.rs\` | none       |

Body.
`
  );
  await gitIn(root, ['init', '--quiet', '-b', 'main']);
  await commit(root, 'base');
  const base = await gitIn(root, ['rev-parse', 'HEAD']);

  await gitIn(root, ['checkout', '--quiet', '-b', 'pr']);
  await writeFile(join(root, 'unrelated.txt'), 'not a declared upstream\n');
  await commit(root, 'pr: unrelated change');
  const prHead = await gitIn(root, ['rev-parse', 'HEAD']);

  await gitIn(root, ['checkout', '--quiet', 'main']);
  await writeFile(join(root, 'crates', 'example', 'src', 'lib.rs'), 'export const value = 2;\n');
  await commit(root, 'main: someone else moves the upstream');

  // GitHub's refs/pull/N/merge is the PR head merged INTO the base branch.
  await gitIn(root, ['checkout', '--quiet', '-b', 'merge-ref']);
  await gitIn(root, [
    '-c',
    'user.name=DOCRB test',
    '-c',
    'user.email=docrb@example.invalid',
    'merge',
    '--no-ff',
    '--no-edit',
    '--quiet',
    prHead,
  ]);
  return { root, base, prHead };
}

async function runChecker(root, args) {
  try {
    const { stdout } = await execFileAsync('node', [CHECKER, '--root', root, ...args], {
      cwd: root,
    });
    return { code: 0, out: stdout };
  } catch (err) {
    return { code: err.code ?? 1, out: `${err.stdout ?? ''}${err.stderr ?? ''}` };
  }
}

test('diffing to the merge ref blames the PR for an upstream main moved', async (t) => {
  const { root, base } = await buildMergeRefRepo();
  t.after(() => rm(root, { recursive: true, force: true }));

  const { out } = await runChecker(root, ['--since', base]);
  assert.match(
    out,
    /docs\/architecture\/owner\.md/,
    `without --head the gate reports a document this PR did not affect:\n${out}`
  );
});

test('--head scopes the diff to the PR head, so main-only movement is not owed', async (t) => {
  const { root, base, prHead } = await buildMergeRefRepo();
  t.after(() => rm(root, { recursive: true, force: true }));

  const { code, out } = await runChecker(root, [
    '--since',
    base,
    '--head',
    prHead,
    '--fail-on-owed',
  ]);
  assert.equal(code, 0, `PR head range must not fail the gate:\n${out}`);
  assert.doesNotMatch(out, /owner\.md/, 'the document is owed by main, not by this PR');
});

test('the summary names the range actually used, so scope is auditable', async (t) => {
  const { root, base, prHead } = await buildMergeRefRepo();
  t.after(() => rm(root, { recursive: true, force: true }));

  const { out } = await runChecker(root, ['--since', base, '--head', prHead]);
  assert.match(out, new RegExp(`diff \\(${base}\\.\\.\\.${prHead}\\)`));
});

test('blank --head is treated as HEAD, so the summary matches the range used', async (t) => {
  const { root, base } = await buildMergeRefRepo();
  t.after(() => rm(root, { recursive: true, force: true }));

  const { out } = await runChecker(root, ['--since', base, '--head', '   ']);
  assert.match(
    out,
    new RegExp(`diff \\(${base}\\.\\.\\.HEAD\\)`),
    `summary must name HEAD, not the blank flag:\n${out}`
  );
  assert.match(
    out,
    /docs\/architecture\/owner\.md/,
    'blank --head must not silently empty the diff range'
  );
});
