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
      if (
        relative.includes('fake-anvil') ||
        relative.includes('fake-previous') ||
        relative.includes('fake-current') ||
        contents.startsWith('#!')
      ) {
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

function upgradeCatalog() {
  return catalog([{ id: 'upgrade-previous-public', kind: 'upgrade' }]);
}

/**
 * Distinguishable fake CLI: previous --write must materialise a marker the
 * current binary --verify consumes. Current writing its own marker must fail.
 */
function fakeAnvilSource({ version, role }) {
  const versionLiteral = JSON.stringify(version);
  const roleLiteral = JSON.stringify(role);
  return `#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const args = process.argv.slice(2);
if (args[0] === '--version' || args.includes('--version')) {
  process.stdout.write(${versionLiteral} + '\\n');
  process.exit(0);
}
const json = args.includes('--json');
const verify = args.includes('--verify');
const write = args.includes('--write');
const wsIdx = args.indexOf('--workspace');
const workspace = wsIdx >= 0 && args[wsIdx + 1] ? args[wsIdx + 1] : process.cwd();
const handoffPath = path.join(workspace, '.grok', 'config.toml');
const previousMarker = 'PREVIOUS_HANDOFF_MARKER';
const role = ${roleLiteral};
if (verify) {
  try {
    const text = fs.readFileSync(handoffPath, 'utf8');
    if (!text.includes(previousMarker)) process.exit(1);
    if (json) {
      process.stdout.write(JSON.stringify({
        target: 'grok',
        path: handoffPath,
        wrote: false,
        ok: true,
      }) + '\\n');
    } else {
      process.stdout.write('Status   : ok\\n');
    }
    process.exit(0);
  } catch {
    process.exit(1);
  }
}
if (args.includes('mcp-config') && (json || write)) {
  const marker = role === 'previous' ? previousMarker : 'CURRENT_HANDOFF_MARKER';
  const config =
    '[mcp_servers.anvil]\\ncommand = ' + JSON.stringify(role) + '\\n# ' + marker + '\\n';
  if (write) {
    fs.mkdirSync(path.dirname(handoffPath), { recursive: true });
    fs.writeFileSync(handoffPath, config);
    process.stdout.write(JSON.stringify({
      target: 'grok',
      path: handoffPath,
      wrote: true,
      ok: true,
      entry: { command: role },
    }) + '\\n');
    process.exit(0);
  }
  process.stdout.write(
    JSON.stringify({ target: 'grok', path: handoffPath, format: 'toml', config }) + '\\n'
  );
  process.exit(0);
}
process.stderr.write('unexpected argv: ' + args.join(' ') + '\\n');
process.exit(1);
`;
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

test('upgrade requires the previous binary to produce a hand-off the current binary consumes', () => {
  const r = run(
    [
      '--catalog',
      'catalog.json',
      '--bin',
      'bin/fake-current',
      '--require-upgrade',
      '--identity-out',
      'id.json',
    ],
    {
      env: { ANVIL_PREVIOUS_PUBLIC_BIN: 'bin/fake-previous' },
      inputFiles: {
        'bin/fake-previous': fakeAnvilSource({
          version: 'anvil 0.9.7-previous',
          role: 'previous',
        }),
        'bin/fake-current': fakeAnvilSource({
          version: 'anvil 0.9.8-current',
          role: 'current',
        }),
        'catalog.json': upgradeCatalog(),
      },
    }
  );
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(`${r.stdout}${r.stderr}`, /upgrade-previous-public: pass/i);
  assert.match(`${r.stdout}${r.stderr}`, /0\.9\.7-previous/);
});

test('--require-upgrade fails when the previous binary does not identify as anvil', () => {
  const r = run(['--catalog', 'catalog.json', '--bin', 'bin/fake-current', '--require-upgrade'], {
    env: { ANVIL_PREVIOUS_PUBLIC_BIN: 'bin/fake-previous' },
    inputFiles: {
      'bin/fake-previous': '#!/bin/sh\necho not-anvil\n',
      'bin/fake-current': fakeAnvilSource({
        version: 'anvil 0.9.8-current',
        role: 'current',
      }),
      'catalog.json': upgradeCatalog(),
    },
  });
  assert.notEqual(r.status, 0, r.stdout + r.stderr);
  assert.match(`${r.stdout}${r.stderr}`, /did not identify as anvil|identity/i);
});

test('--require-upgrade fails when the previous binary is not invoked for the hand-off', () => {
  const r = run(['--catalog', 'catalog.json', '--bin', 'bin/fake-current', '--require-upgrade'], {
    env: { ANVIL_PREVIOUS_PUBLIC_BIN: 'bin/fake-previous' },
    inputFiles: {
      'bin/fake-previous': `#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "anvil 0.9.7-previous"
  exit 0
fi
echo "previous ignored the upgrade hand-off"
exit 0
`,
      'bin/fake-current': fakeAnvilSource({
        version: 'anvil 0.9.8-current',
        role: 'current',
      }),
      'catalog.json': upgradeCatalog(),
    },
  });
  assert.notEqual(r.status, 0, r.stdout + r.stderr);
  assert.match(`${r.stdout}${r.stderr}`, /hand-off is absent|not invoked/i);
});

test('--require-upgrade fails when the current binary cannot consume the hand-off', () => {
  const r = run(['--catalog', 'catalog.json', '--bin', 'bin/fake-current', '--require-upgrade'], {
    env: { ANVIL_PREVIOUS_PUBLIC_BIN: 'bin/fake-previous' },
    inputFiles: {
      'bin/fake-previous': fakeAnvilSource({
        version: 'anvil 0.9.7-previous',
        role: 'previous',
      }),
      'bin/fake-current': `#!/usr/bin/env node
const args = process.argv.slice(2);
if (args[0] === '--version' || args.includes('--version')) {
  process.stdout.write('anvil 0.9.8-current\\n');
  process.exit(0);
}
process.stderr.write('current refused the upgrade hand-off\\n');
process.exit(1);
`,
      'catalog.json': upgradeCatalog(),
    },
  });
  assert.notEqual(r.status, 0, r.stdout + r.stderr);
  assert.match(`${r.stdout}${r.stderr}`, /did not consume|compatibility/i);
});

test('--require-upgrade fails when previous and current are the same binary', () => {
  const r = run(['--catalog', 'catalog.json', '--bin', 'bin/fake-current', '--require-upgrade'], {
    env: { ANVIL_PREVIOUS_PUBLIC_BIN: 'bin/fake-current' },
    inputFiles: {
      'bin/fake-current': fakeAnvilSource({
        version: 'anvil 0.9.8-current',
        role: 'current',
      }),
      'catalog.json': upgradeCatalog(),
    },
  });
  assert.notEqual(r.status, 0, r.stdout + r.stderr);
  assert.match(`${r.stdout}${r.stderr}`, /distinct from the current binary/i);
});
