import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { chmod, cp, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const REPO_ROOT = resolve(fileURLToPath(new URL('../..', import.meta.url)));
const PUBLIC_GENERATOR = resolve(REPO_ROOT, 'scripts/docs/generate-anvil-public-reference.mjs');
const CATALOGUE_GENERATOR = resolve(REPO_ROOT, 'scripts/docs/generate-product-catalogue.mjs');
const ATOMIC_HELPER = resolve(REPO_ROOT, 'scripts/docs/lib/atomic-output-batch.mjs');

const PUBLIC_INPUTS = [
  'patterns/compiled/registry.json',
  'crates/anvil-cli/src/main.rs',
  'crates/anvil-cli/src/commands/start.rs',
  'crates/anvil-cli/src/commands/check.rs',
  'crates/anvil-cli/src/commands/gate.rs',
  'crates/anvil-cli/src/commands/config.rs',
  'crates/anvil-cli/src/commands/watch.rs',
  'crates/anvil-cli/src/commands/doctor.rs',
  'crates/anvil-cli/src/commands/init.rs',
  'crates/anvil-cli/src/commands/policy/mod.rs',
  'crates/anvil-cli/src/commands/check_catalog.rs',
  'crates/anvil-cli/src/activation/agent_registry.rs',
  'crates/anvil-kernel/src/parser/languages.rs',
  'flags/manifest.json',
  'dist-workspace.toml',
];
const PUBLIC_PAGE_NAMES = ['cli.md', 'rules.md', 'checks.md', 'support.md'];
const HELP_COMMANDS = ['check', 'gate', 'policy', 'config', 'start', 'watch', 'doctor', 'init'];

async function copyPath(root, relativePath) {
  const destination = resolve(root, relativePath);
  await mkdir(dirname(destination), { recursive: true });
  await cp(resolve(REPO_ROOT, relativePath), destination, { recursive: true });
}

async function createPublicFixture() {
  const root = await mkdtemp(join(tmpdir(), 'public-reference-atomic-'));
  for (const path of PUBLIC_INPUTS) await copyPath(root, path);
  const changelog = resolve(root, 'docs/public/anvil/releases/changelog.md');
  await mkdir(dirname(changelog), { recursive: true });
  await writeFile(changelog, '# Release notes\n\n## 0.0.0-test\n');
  await execFileAsync('git', ['-C', root, 'init', '-q']);
  return root;
}

async function createCatalogueFixture() {
  const root = await mkdtemp(join(tmpdir(), 'product-catalogue-atomic-'));
  await copyPath(root, 'flags/surfaces.json');
  await copyPath(root, 'flags/manifest.json');
  const generator = resolve(root, 'scripts/docs/generate-product-catalogue.mjs');
  const helper = resolve(root, 'scripts/docs/lib/atomic-output-batch.mjs');
  await mkdir(dirname(generator), { recursive: true });
  await mkdir(dirname(helper), { recursive: true });
  await cp(CATALOGUE_GENERATOR, generator);
  await cp(ATOMIC_HELPER, helper);
  return { root, generator };
}

async function writeFaultPreload(root) {
  const path = resolve(root, 'atomic-output-fault-preload.mjs');
  await writeFile(
    path,
    `import fs from 'node:fs';
import { basename } from 'node:path';
import { syncBuiltinESMExports } from 'node:module';

const originalOpenSync = fs.openSync;
const originalRenameSync = fs.renameSync;
fs.openSync = function (path, flags, mode) {
  if (
    process.env.ANVIL_ATOMIC_TEST_FAILURE === 'stage-open' &&
    String(path).includes('.anvil-stage-') &&
    String(path).includes('.' + process.env.ANVIL_ATOMIC_TEST_TARGET + '.anvil-stage-')
  ) {
    throw new Error('injected atomic staging failure');
  }
  return originalOpenSync.call(this, path, flags, mode);
};
fs.renameSync = function (from, to) {
  if (
    process.env.ANVIL_ATOMIC_TEST_FAILURE === 'publish-rename' &&
    String(from).includes('.anvil-stage-') &&
    basename(String(to)) === process.env.ANVIL_ATOMIC_TEST_TARGET
  ) {
    throw new Error('injected atomic publish failure');
  }
  return originalRenameSync.call(this, from, to);
};
syncBuiltinESMExports();
`
  );
  return path;
}

async function writeFakeAnvil(root) {
  const path = resolve(root, 'fake-anvil');
  await writeFile(
    path,
    `#!/usr/bin/env node
const command = process.argv[2];
if (command === process.env.ANVIL_FAKE_HELP_FAILURE) {
  process.stderr.write('injected invalid help for ' + command + '\\n');
  process.exit(1);
}
process.stdout.write('Usage: anvil ' + command + ' [OPTIONS]\\n\\nOptions:\\n      --example\\n');
`
  );
  await chmod(path, 0o755);
  return path;
}

async function runNode(script, args, { cwd, preload, env = {} } = {}) {
  const nodeArgs = preload ? ['--import', preload, script, ...args] : [script, ...args];
  try {
    const result = await execFileAsync(process.execPath, nodeArgs, {
      cwd,
      encoding: 'utf8',
      env: { ...process.env, ...env },
      maxBuffer: 16 * 1024 * 1024,
    });
    return { status: 0, stdout: result.stdout, stderr: result.stderr };
  } catch (error) {
    return {
      status: typeof error.code === 'number' ? error.code : 1,
      stdout: error.stdout ?? '',
      stderr: error.stderr ?? error.message,
    };
  }
}

async function writeSentinels(directory, names) {
  const old = new Map();
  await mkdir(directory, { recursive: true });
  for (const name of names) {
    const content = `old ${name}\n`;
    await writeFile(resolve(directory, name), content);
    old.set(name, content);
  }
  return old;
}

async function assertSentinels(directory, old) {
  for (const [name, content] of old) {
    assert.equal(await readFile(resolve(directory, name), 'utf8'), content);
  }
  assert.ok(
    (await readdir(directory)).every(
      (name) =>
        !name.includes('.anvil-stage-') &&
        !name.includes('.anvil-recovery-') &&
        !name.includes('.anvil-restore-')
    )
  );
}

test('public reference generation rolls all four pages back after a late publish failure', async () => {
  const root = await createPublicFixture();
  const outputDirectory = resolve(root, 'docs/public/anvil/reference');
  const old = await writeSentinels(outputDirectory, PUBLIC_PAGE_NAMES);

  try {
    const preload = await writeFaultPreload(root);
    const result = await runNode(PUBLIC_GENERATOR, ['--root', root], {
      cwd: root,
      preload,
      env: {
        ANVIL_ATOMIC_TEST_FAILURE: 'publish-rename',
        ANVIL_ATOMIC_TEST_TARGET: 'rules.md',
      },
    });

    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /injected atomic publish failure/);
    await assertSentinels(outputDirectory, old);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('public reference captures every valid help snapshot before staging the twelve-file batch', async () => {
  const root = await createPublicFixture();
  const outputDirectory = resolve(root, 'docs/public/anvil/reference');
  const snapshotDirectory = resolve(root, 'scripts/docs/fixtures/anvil-cli-help');
  const oldPages = await writeSentinels(outputDirectory, PUBLIC_PAGE_NAMES);
  const oldSnapshots = await writeSentinels(
    snapshotDirectory,
    HELP_COMMANDS.map((command) => `${command}.txt`)
  );

  try {
    const fakeAnvil = await writeFakeAnvil(root);
    const result = await runNode(
      PUBLIC_GENERATOR,
      ['--root', root, '--update-help-snapshots', '--anvil-bin', fakeAnvil],
      {
        cwd: root,
        env: { ANVIL_FAKE_HELP_FAILURE: 'init' },
      }
    );

    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /could not capture .* init --help/);
    await assertSentinels(outputDirectory, oldPages);
    await assertSentinels(snapshotDirectory, oldSnapshots);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('public reference rolls pages and help snapshots back as one twelve-file batch', async () => {
  const root = await createPublicFixture();
  const outputDirectory = resolve(root, 'docs/public/anvil/reference');
  const snapshotDirectory = resolve(root, 'scripts/docs/fixtures/anvil-cli-help');
  const oldPages = await writeSentinels(outputDirectory, PUBLIC_PAGE_NAMES);
  const oldSnapshots = await writeSentinels(
    snapshotDirectory,
    HELP_COMMANDS.map((command) => `${command}.txt`)
  );

  try {
    const fakeAnvil = await writeFakeAnvil(root);
    const preload = await writeFaultPreload(root);
    const result = await runNode(
      PUBLIC_GENERATOR,
      ['--root', root, '--update-help-snapshots', '--anvil-bin', fakeAnvil],
      {
        cwd: root,
        preload,
        env: {
          ANVIL_ATOMIC_TEST_FAILURE: 'publish-rename',
          ANVIL_ATOMIC_TEST_TARGET: 'config.txt',
        },
      }
    );

    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /injected atomic publish failure/);
    await assertSentinels(outputDirectory, oldPages);
    await assertSentinels(snapshotDirectory, oldSnapshots);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('public reference rejects check plus help-snapshot update before reading product inputs', async () => {
  const root = await mkdtemp(join(tmpdir(), 'public-reference-contradiction-'));
  try {
    const result = await runNode(
      PUBLIC_GENERATOR,
      ['--root', root, '--check', '--update-help-snapshots'],
      { cwd: root }
    );

    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /--check cannot be combined with --update-help-snapshots/);
    assert.deepEqual(await readdir(root), []);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('product catalogue preserves its old output on injected staging and rename failures', async (t) => {
  for (const failure of ['stage-open', 'publish-rename']) {
    await t.test(failure, async () => {
      const { root, generator } = await createCatalogueFixture();
      const outputDirectory = resolve(root, 'docs/guides');
      const old = await writeSentinels(outputDirectory, ['product-feature-catalogue.md']);

      try {
        const preload = await writeFaultPreload(root);
        const result = await runNode(generator, ['--write'], {
          cwd: root,
          preload,
          env: {
            ANVIL_ATOMIC_TEST_FAILURE: failure,
            ANVIL_ATOMIC_TEST_TARGET: 'product-feature-catalogue.md',
          },
        });

        assert.notEqual(result.status, 0);
        assert.match(result.stderr, /injected atomic (staging|publish) failure/);
        await assertSentinels(outputDirectory, old);
      } finally {
        await rm(root, { recursive: true, force: true });
      }
    });
  }
});
