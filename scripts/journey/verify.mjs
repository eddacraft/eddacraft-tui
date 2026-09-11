#!/usr/bin/env node
/**
 * JREL-012: fail-closed product-journey verification.
 *
 * A green run means the real anvil binary was present and every required
 * scenario actually executed. Missing binaries, missing scenario files, and
 * skipped required legs are failures — never silent passes.
 *
 * Conductor rehearsal (JOURNEY-014/-015):
 *   pnpm journey:verify
 *
 * Optional previous-public-build upgrade:
 *   ANVIL_PREVIOUS_PUBLIC_BIN=/path/to/anvil pnpm journey:verify --require-upgrade
 *
 * The upgrade leg invokes the previous public binary, checks that it identifies
 * as anvil, and requires a cross-version hand-off: previous
 * `mcp-config --json --write` materialises a project MCP config, then the
 * current binary must `mcp-config --verify` it. --require-upgrade fails if the
 * previous executable is not invoked, the hand-off is absent, or compatibility
 * fails.
 */

import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  accessSync,
  constants as fsConstants,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const DEFAULT_CATALOG = join(HERE, 'scenarios.json');
const SKIP_TOKEN = 'JOURNEY_SKIP';
const EXE = process.platform === 'win32' ? 'anvil.exe' : 'anvil';

function parseArgs(argv) {
  const options = {
    root: process.cwd(),
    catalog: DEFAULT_CATALOG,
    bin: process.env.ANVIL_BIN || '',
    identityOut: '',
    list: false,
    noBuild: false,
    requireUpgrade: false,
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    const next = (flag) => {
      i += 1;
      if (i >= argv.length || argv[i] === undefined || String(argv[i]).startsWith('--')) {
        fail(`${flag} requires a value`);
      }
      return argv[i];
    };
    switch (arg) {
      case '--root':
        options.root = resolve(next('--root'));
        break;
      case '--catalog':
        options.catalog = next('--catalog');
        break;
      case '--bin':
        options.bin = next('--bin');
        break;
      case '--identity-out':
        options.identityOut = next('--identity-out');
        break;
      case '--list':
        options.list = true;
        break;
      case '--no-build':
        options.noBuild = true;
        break;
      case '--require-upgrade':
        options.requireUpgrade = true;
        break;
      default:
        fail(`unknown argument: ${arg}`);
    }
  }
  return options;
}

function fail(message, extra = '') {
  const err = new Error(message);
  err.code = 'JOURNEY_VERIFY_FAIL';
  err.extra = extra;
  throw err;
}

function loadCatalog(options) {
  const path = isAbsolute(options.catalog)
    ? options.catalog
    : resolve(options.root, options.catalog);
  if (!existsSync(path)) {
    fail(`catalog not found: ${path}`);
  }
  let parsed;
  try {
    parsed = JSON.parse(readFileSync(path, 'utf8'));
  } catch (err) {
    fail(`catalog is not valid JSON: ${path}`, String(err));
  }
  if (!Array.isArray(parsed.scenarios) || parsed.scenarios.length === 0) {
    fail(`catalog has no scenarios: ${path}`);
  }
  return { path, scenarios: parsed.scenarios };
}

function candidateBinaries(root) {
  const targetDir = process.env.CARGO_TARGET_DIR;
  return [
    process.env.ANVIL_BIN,
    ...(targetDir ? [join(targetDir, 'debug', EXE), join(targetDir, 'release', EXE)] : []),
    join(root, 'target', 'debug', EXE),
    join(root, 'target', 'release', EXE),
  ].filter(Boolean);
}

function isUsableBinary(path) {
  try {
    const stat = statSync(path);
    if (!stat.isFile() || stat.size === 0) return false;
    if (process.platform !== 'win32') {
      accessSync(path, fsConstants.X_OK);
    }
    return true;
  } catch {
    return false;
  }
}

function resolveBinary(options) {
  if (options.bin) {
    const path = isAbsolute(options.bin) ? options.bin : resolve(options.root, options.bin);
    return isUsableBinary(path) ? path : null;
  }
  return candidateBinaries(options.root).find(isUsableBinary) ?? null;
}

function tryBuild(root) {
  const result = spawnSync('cargo', ['build', '-p', 'eddacraft-anvil'], {
    cwd: root,
    encoding: 'utf8',
    env: process.env,
  });
  return result.status === 0;
}

function sha256File(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function gitSha(root) {
  const result = spawnSync('git', ['rev-parse', 'HEAD'], {
    cwd: root,
    encoding: 'utf8',
  });
  return result.status === 0 ? result.stdout.trim() : 'unknown';
}

function binaryVersion(bin, { requireSuccess = false } = {}) {
  const result = spawnSync(bin, ['--version'], { encoding: 'utf8' });
  const combined = `${result.stdout ?? ''}${result.stderr ?? ''}`.trim();
  const line = combined.split('\n')[0] || 'unknown';
  if (requireSuccess && result.status !== 0) {
    fail(`binary --version failed (status ${result.status})`, combined);
  }
  return line;
}

function resolveMaybeRelative(raw, root) {
  if (!raw) return '';
  return isAbsolute(raw) ? raw : resolve(root, raw);
}

function isAnvilIdentity(versionText) {
  return /^anvil\s+\S+/i.test(String(versionText || '').trim());
}

function sameBinary(left, right) {
  try {
    return realpathSync(left) === realpathSync(right);
  } catch {
    return resolve(left) === resolve(right);
  }
}

function resolveExistingAncestor(target) {
  let cur = resolve(target);
  const missing = [];
  while (!existsSync(cur)) {
    const parent = dirname(cur);
    if (parent === cur) break;
    missing.unshift(basename(cur));
    cur = parent;
  }
  const realBase = realpathSync(cur);
  return missing.length === 0 ? realBase : join(realBase, ...missing);
}

function isInsideRoot(root, target) {
  let realRoot;
  try {
    realRoot = realpathSync(root);
  } catch {
    realRoot = resolve(root);
  }
  let realTarget;
  try {
    realTarget = existsSync(target) ? realpathSync(target) : resolveExistingAncestor(target);
  } catch {
    realTarget = resolve(target);
  }
  const rel = relative(realRoot, realTarget);
  return rel !== '' && !rel.startsWith('..') && !isAbsolute(rel);
}

function resolvePreviousBinary(options, scenarios = []) {
  const fromEnv = process.env.ANVIL_PREVIOUS_PUBLIC_BIN || '';
  if (fromEnv) {
    return resolveMaybeRelative(fromEnv, options.root);
  }
  const upgrade = scenarios.find(
    (scenario) => scenario && scenario.kind === 'upgrade' && scenario.bin
  );
  return upgrade ? resolveMaybeRelative(upgrade.bin, options.root) : '';
}

function parseJsonDocument(text) {
  const trimmed = String(text || '').trim();
  if (!trimmed) return null;
  try {
    return JSON.parse(trimmed);
  } catch {
    const start = trimmed.indexOf('{');
    const end = trimmed.lastIndexOf('}');
    if (start < 0 || end <= start) return null;
    try {
      return JSON.parse(trimmed.slice(start, end + 1));
    } catch {
      return null;
    }
  }
}

function currentPlatform() {
  if (process.platform === 'win32') return 'windows';
  if (process.platform === 'darwin') return 'macos';
  return 'linux';
}

function platformAllowed(scenario) {
  if (!Array.isArray(scenario.platforms) || scenario.platforms.length === 0) {
    return true;
  }
  return scenario.platforms.includes(currentPlatform());
}

function cargoTestPath(root, scenario) {
  const name = `${scenario.test}.rs`;
  const searchRoots = [
    join(root, 'crates', 'anvil-cli', 'tests', name),
    join(root, 'crates', 'anvil-intercept', 'tests', name),
  ];
  return searchRoots.find((candidate) => existsSync(candidate)) ?? null;
}

function runCommand(argv, cwd, extraEnv = {}) {
  const [command, ...args] = argv;
  return spawnSync(command, args, {
    cwd,
    encoding: 'utf8',
    env: { ...process.env, ...extraEnv },
  });
}

function record(id, result, detail = '') {
  return { id, result, detail };
}

function executeUpgrade(scenario, options, bin) {
  const previousRaw = process.env.ANVIL_PREVIOUS_PUBLIC_BIN || scenario.bin || '';
  const previous = resolveMaybeRelative(previousRaw, options.root);
  if (!previous || !isUsableBinary(previous)) {
    if (options.requireUpgrade) {
      fail('previous public binary is required for the upgrade scenario');
    }
    return record(scenario.id, 'not-supplied', 'ANVIL_PREVIOUS_PUBLIC_BIN unset');
  }

  if (sameBinary(previous, bin)) {
    fail('previous public binary must be distinct from the current binary');
  }

  const previousVersion = binaryVersion(previous, { requireSuccess: true });
  if (!isAnvilIdentity(previousVersion)) {
    fail('previous public binary did not identify as anvil', previousVersion);
  }

  const upgradeRoot = mkdtempSync(join(tmpdir(), 'anvil-journey-upgrade-'));
  try {
    // Keep host HOME / ANVIL_HOME so a licensed previous public binary can
    // run. The MCP-config write is confined with --workspace / --scope project.
    const produce = runCommand(
      [
        previous,
        'mcp-config',
        '--json',
        '--target',
        'grok',
        '--scope',
        'project',
        '--workspace',
        upgradeRoot,
        '--write',
        '--yes',
      ],
      upgradeRoot
    );
    if (produce.status !== 0) {
      fail(
        'previous public binary was not invoked for the upgrade hand-off',
        `${produce.stdout ?? ''}${produce.stderr ?? ''}`
      );
    }

    const payload = parseJsonDocument(produce.stdout);
    const handoffPath = payload?.path;
    if (!payload || typeof handoffPath !== 'string' || !handoffPath || payload.wrote !== true) {
      fail(
        'upgrade hand-off is absent: previous binary did not produce MCP config',
        `${produce.stdout ?? ''}${produce.stderr ?? ''}`
      );
    }
    if (!isInsideRoot(upgradeRoot, handoffPath)) {
      fail('upgrade hand-off path escapes the isolated workspace', handoffPath);
    }
    if (!existsSync(handoffPath)) {
      fail('upgrade hand-off is absent');
    }
    // Re-check after existence: a lexical in-root path may still be a symlink
    // whose real target sits outside the isolated workspace.
    if (!isInsideRoot(upgradeRoot, handoffPath)) {
      fail('upgrade hand-off path escapes the isolated workspace', handoffPath);
    }
    let handoffBody;
    try {
      handoffBody = readFileSync(handoffPath, 'utf8');
    } catch (err) {
      fail('upgrade hand-off is absent', String(err));
    }
    if (!handoffBody.trim()) {
      fail('upgrade hand-off is absent');
    }

    const consume = runCommand(
      [
        bin,
        'mcp-config',
        '--json',
        '--target',
        'grok',
        '--scope',
        'project',
        '--workspace',
        upgradeRoot,
        '--verify',
        '--yes',
      ],
      upgradeRoot
    );
    const consumePayload = parseJsonDocument(consume.stdout);
    if (consume.status !== 0 || consumePayload?.ok !== true) {
      fail(
        'current binary did not consume the previous-public hand-off',
        `${consume.stdout ?? ''}${consume.stderr ?? ''}`
      );
    }

    return record(scenario.id, 'pass', `${previous} (${previousVersion}) → ${handoffPath}`);
  } finally {
    rmSync(upgradeRoot, { recursive: true, force: true });
  }
}

function executeScenario(scenario, options, bin) {
  if (!platformAllowed(scenario)) {
    return record(scenario.id, 'not-on-platform', currentPlatform());
  }

  switch (scenario.kind) {
    case 'upgrade': {
      return executeUpgrade(scenario, options, bin);
    }
    case 'cargo-test': {
      const path = cargoTestPath(options.root, scenario);
      if (!path) {
        fail(`required scenario file not found: ${scenario.test}`);
      }
      const result = runCommand(
        [
          'cargo',
          'test',
          '-p',
          scenario.package || 'eddacraft-anvil',
          '--test',
          scenario.test,
          '--no-fail-fast',
        ],
        options.root
      );
      const output = `${result.stdout ?? ''}${result.stderr ?? ''}`;
      if (result.status !== 0) {
        fail(`cargo test ${scenario.test} failed`, output);
      }
      if (/\b0 passed\b/.test(output) && /\b0 failed\b/.test(output)) {
        fail(`required scenario ran 0 tests: ${scenario.id}`, output);
      }
      if (output.includes(SKIP_TOKEN)) {
        fail(`skipped required scenario: ${scenario.id}`, output);
      }
      return record(scenario.id, 'pass', path);
    }
    case 'pnpm-e2e': {
      const result = runCommand(
        ['pnpm', '--filter', '@eddacraft/anvil-e2e', scenario.script],
        options.root,
        {
          ANVIL_BIN: bin,
          ANVIL_E2E_REQUIRE_BIN: '1',
          // MCP probes resolve configured anvil via PATH
          [process.platform === 'win32' ? 'Path' : 'PATH']:
            dirname(bin) +
            (process.platform === 'win32' ? ';' : ':') +
            (process.env[process.platform === 'win32' ? 'Path' : 'PATH'] || ''),
        }
      );
      const output = `${result.stdout ?? ''}${result.stderr ?? ''}`;
      if (result.status !== 0) {
        fail(`e2e ${scenario.script} failed`, output);
      }
      if (output.includes(SKIP_TOKEN) || /\b[0-9]+ skipped\b/i.test(output)) {
        fail(`skipped required scenario: ${scenario.id}`, output);
      }
      return record(scenario.id, 'pass', scenario.script);
    }
    case 'command': {
      if (!Array.isArray(scenario.argv) || scenario.argv.length === 0) {
        fail(`command scenario ${scenario.id} has no argv`);
      }
      const result = runCommand(scenario.argv, options.root, { ANVIL_BIN: bin });
      const output = `${result.stdout ?? ''}${result.stderr ?? ''}`;
      if (output.includes(SKIP_TOKEN)) {
        fail(`skipped required scenario: ${scenario.id}`, output);
      }
      if (result.status !== 0) {
        fail(`command scenario ${scenario.id} failed`, output);
      }
      return record(scenario.id, 'pass');
    }
    default:
      fail(`unknown scenario kind: ${scenario.kind}`);
  }
}

function run() {
  const options = parseArgs(process.argv.slice(2));
  const catalog = loadCatalog(options);

  if (options.list) {
    for (const scenario of catalog.scenarios) {
      process.stdout.write(`${scenario.id}\t${scenario.kind}\n`);
    }
    return;
  }

  let bin = resolveBinary(options);
  if (!bin && !options.noBuild) {
    process.stderr.write('journey:verify: building eddacraft-anvil\n');
    tryBuild(options.root);
    bin = resolveBinary(options);
  }
  if (!bin) {
    fail(
      'anvil CLI binary not found — journey verification forbids skip',
      `searched:\n  - ${candidateBinaries(options.root).join('\n  - ')}`
    );
  }

  const scenarioResults = [];
  for (const scenario of catalog.scenarios) {
    scenarioResults.push(executeScenario(scenario, options, bin));
  }

  if (options.requireUpgrade) {
    const upgradePass = scenarioResults.some(
      (scenario) => scenario.id === 'upgrade-previous-public' && scenario.result === 'pass'
    );
    if (!upgradePass) {
      fail('--require-upgrade requires a passing upgrade-previous-public scenario');
    }
  }

  const previousPath = resolvePreviousBinary(options, catalog.scenarios);
  const previousUsable = previousPath && isUsableBinary(previousPath);
  const identity = {
    sourceSha: gitSha(options.root),
    binary: {
      path: bin,
      sha256: sha256File(bin),
      version: binaryVersion(bin, { requireSuccess: true }),
    },
    platform: {
      os: process.platform,
      arch: process.arch,
      name: currentPlatform(),
    },
    executedAt: new Date().toISOString(),
    catalog: catalog.path,
    scenarios: scenarioResults,
    previousPublicBinary: previousUsable
      ? previousPath
      : process.env.ANVIL_PREVIOUS_PUBLIC_BIN || null,
    previousPublic: previousUsable
      ? {
          path: previousPath,
          sha256: sha256File(previousPath),
          version: binaryVersion(previousPath, { requireSuccess: true }),
          invoked: scenarioResults.some(
            (scenario) => scenario.id === 'upgrade-previous-public' && scenario.result === 'pass'
          ),
        }
      : null,
  };

  const identityOut = options.identityOut
    ? isAbsolute(options.identityOut)
      ? options.identityOut
      : resolve(options.root, options.identityOut)
    : join(options.root, 'coverage', 'journey-identity.json');
  mkdirSync(dirname(identityOut), { recursive: true });
  writeFileSync(identityOut, `${JSON.stringify(identity, null, 2)}\n`);

  process.stdout.write(`journey:verify: pass (${scenarioResults.length} scenarios)\n`);
  process.stdout.write(`identity: ${identityOut}\n`);
  for (const scenario of scenarioResults) {
    process.stdout.write(
      `  ${scenario.id}: ${scenario.result}${scenario.detail ? ` (${scenario.detail})` : ''}\n`
    );
  }
}

function main() {
  try {
    run();
  } catch (err) {
    if (err && err.code === 'JOURNEY_VERIFY_FAIL') {
      process.stderr.write(`journey:verify: ${err.message}\n`);
      if (err.extra) process.stderr.write(`${err.extra}\n`);
      process.exit(1);
    }
    throw err;
  }
}

main();
