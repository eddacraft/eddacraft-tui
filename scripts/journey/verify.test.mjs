import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const SCRIPT = resolve(dirname(fileURLToPath(import.meta.url)), 'verify.mjs');

function run(args, { env = {}, inputFiles = {} } = {}) {
  const dir = mkdtempSync(join(tmpdir(), 'journey-verify-'));
  try {
    for (const [relative, contents] of Object.entries(inputFiles)) {
      const path = join(dir, relative);
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, contents);
      if (relative.includes('fake-anvil')) {
        chmodSync(path, 0o755);
      }
    }
    const result = spawnSync(process.execPath, [SCRIPT, '--root', dir, '--no-build', ...args], {
      encoding: 'utf8',
      env: { ...process.env, ...env, ANVIL_BIN: env.ANVIL_BIN ?? '' },
    });
    return { dir, status: result.status, stdout: result.stdout, stderr: result.stderr };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

function catalog(scenarios) {
  return JSON.stringify({ scenarios }, null, 2);
}

test('missing binary fails closed and is not reported as a skip', () => {
  const r = run(['--catalog', 'catalog.json', '--bin', 'does-not-exist'], {
    inputFiles: {
      'catalog.json': catalog([
        { id: 'probe-only', kind: 'command', argv: [process.execPath, '-e', 'console.log("ok")'] },
      ]),
    },
  });
  assert.notEqual(r.status, 0, r.stdout + r.stderr);
  assert.match(`${r.stdout}${r.stderr}`, /binary not found|anvil CLI binary not found/i);
  assert.doesNotMatch(`${r.stdout}${r.stderr}`, /skipped as pass/i);
});

test('missing required scenario file fails closed', () => {
  const r = run(['--catalog', 'catalog.json', '--bin', 'bin/fake-anvil'], {
    inputFiles: {
      'bin/fake-anvil': '#!/bin/sh\necho fake\n',
      'catalog.json': catalog([
        {
          id: 'daemon-identity',
          kind: 'cargo-test',
          package: 'eddacraft-anvil',
          test: 'daemon_identity_missing',
        },
      ]),
    },
  });
  assert.notEqual(r.status, 0, r.stdout + r.stderr);
  assert.match(`${r.stdout}${r.stderr}`, /required scenario .* missing|scenario file not found/i);
});

test('required command that reports a skip fails closed', () => {
  const r = run(['--catalog', 'catalog.json', '--bin', 'bin/fake-anvil'], {
    inputFiles: {
      'bin/fake-anvil': '#!/bin/sh\necho fake\n',
      'catalog.json': catalog([
        {
          id: 'skipped-leg',
          kind: 'command',
          argv: [process.execPath, '-e', 'console.log("JOURNEY_SKIP required-leg")'],
        },
      ]),
    },
  });
  assert.notEqual(r.status, 0, r.stdout + r.stderr);
  assert.match(`${r.stdout}${r.stderr}`, /skipped required|JOURNEY_SKIP/i);
});

test('successful required command records identity and pass', () => {
  const dir = mkdtempSync(join(tmpdir(), 'journey-verify-id-'));
  try {
    const bin = join(dir, 'bin', 'fake-anvil');
    mkdirSync(dirname(bin), { recursive: true });
    writeFileSync(bin, '#!/bin/sh\necho anvil 0.9.8-test\n');
    chmodSync(bin, 0o755);
    const catalogPath = join(dir, 'catalog.json');
    writeFileSync(
      catalogPath,
      catalog([
        {
          id: 'probe-only',
          kind: 'command',
          argv: [process.execPath, '-e', 'console.log("probe ok")'],
        },
      ])
    );
    const identityPath = join(dir, 'identity.json');
    const result = spawnSync(
      process.execPath,
      [
        SCRIPT,
        '--root',
        dir,
        '--no-build',
        '--catalog',
        catalogPath,
        '--bin',
        bin,
        '--identity-out',
        identityPath,
      ],
      { encoding: 'utf8', env: { ...process.env, ANVIL_BIN: bin } }
    );
    assert.equal(result.status, 0, result.stdout + result.stderr);
    const identity = JSON.parse(readFileSync(identityPath, 'utf8'));
    assert.equal(identity.binary.path, bin);
    assert.equal(typeof identity.binary.sha256, 'string');
    assert.match(identity.binary.sha256, /^[a-f0-9]{64}$/);
    assert.equal(identity.scenarios[0].id, 'probe-only');
    assert.equal(identity.scenarios[0].result, 'pass');
    assert.equal(identity.previousPublicBinary, null);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('upgrade scenario is an explicit gap unless a previous public binary is supplied', () => {
  const r = run(
    ['--catalog', 'catalog.json', '--bin', 'bin/fake-anvil', '--identity-out', 'id.json'],
    {
      inputFiles: {
        'bin/fake-anvil': '#!/bin/sh\necho fake\n',
        'catalog.json': catalog([
          { id: 'upgrade-previous-public', kind: 'upgrade' },
          {
            id: 'probe-only',
            kind: 'command',
            argv: [process.execPath, '-e', 'console.log("ok")'],
          },
        ]),
      },
    }
  );
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(`${r.stdout}${r.stderr}`, /not-supplied|not supplied/i);
});

test('--require-upgrade fails when no previous public binary is supplied', () => {
  const r = run(['--catalog', 'catalog.json', '--bin', 'bin/fake-anvil', '--require-upgrade'], {
    inputFiles: {
      'bin/fake-anvil': '#!/bin/sh\necho fake\n',
      'catalog.json': catalog([{ id: 'upgrade-previous-public', kind: 'upgrade' }]),
    },
  });
  assert.notEqual(r.status, 0, r.stdout + r.stderr);
  assert.match(`${r.stdout}${r.stderr}`, /previous public binary/i);
});

test('default catalog cargo-test files exist in this repository', () => {
  const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
  const catalogPath = resolve(dirname(fileURLToPath(import.meta.url)), 'scenarios.json');
  const catalog = JSON.parse(readFileSync(catalogPath, 'utf8'));
  const missing = [];
  for (const scenario of catalog.scenarios) {
    if (scenario.kind !== 'cargo-test') continue;
    const candidates = [
      join(repoRoot, 'crates/anvil-cli/tests', `${scenario.test}.rs`),
      join(repoRoot, 'crates/anvil-intercept/tests', `${scenario.test}.rs`),
    ];
    if (
      !candidates.some((path) => {
        try {
          return statSync(path).isFile();
        } catch {
          return false;
        }
      })
    ) {
      missing.push(scenario.test);
    }
  }
  assert.deepEqual(missing, []);
});

test('--list does not require a binary', () => {
  const r = run(['--catalog', 'catalog.json', '--list'], {
    inputFiles: {
      'catalog.json': catalog([
        { id: 'probe-only', kind: 'command', argv: ['true'] },
        { id: 'upgrade-previous-public', kind: 'upgrade' },
      ]),
    },
  });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(r.stdout, /probe-only/);
  assert.match(r.stdout, /upgrade-previous-public/);
});
