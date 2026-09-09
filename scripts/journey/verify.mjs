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
 */

import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  accessSync,
  constants as fsConstants,
  existsSync,
  mkdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { dirname, isAbsolute, join, resolve } from 'node:path';
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
    const next = () => {
      i += 1;
      return argv[i];
    };
    switch (arg) {
      case '--root':
        options.root = resolve(next());
        break;
      case '--catalog':
        options.catalog = next();
        break;
      case '--bin':
        options.bin = next();
        break;
      case '--identity-out':
        options.identityOut = next();
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
  process.stderr.write(`journey:verify: ${message}\n`);
  if (extra) process.stderr.write(`${extra}\n`);
  process.exit(1);
}

function loadCatalog(options) {
  const path = isAbsolute(options.catalog)
    ? options.catalog
    : resolve(options.root, options.catalog);
  if (!existsSync(path)) {
    fail(`catalog not found: ${path}`);
  }
  const parsed = JSON.parse(readFileSync(path, 'utf8'));
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

function binaryVersion(bin) {
  const result = spawnSync(bin, ['--version'], { encoding: 'utf8' });
  const text = `${result.stdout ?? ''}${result.stderr ?? ''}`.trim();
  return text.split('\n')[0] || 'unknown';
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

function executeScenario(scenario, options, bin) {
  if (!platformAllowed(scenario)) {
    return record(scenario.id, 'not-on-platform', currentPlatform());
  }

  switch (scenario.kind) {
    case 'upgrade': {
      const previous = process.env.ANVIL_PREVIOUS_PUBLIC_BIN || scenario.bin || '';
      if (!previous || !isUsableBinary(previous)) {
        if (options.requireUpgrade) {
          fail('previous public binary is required for the upgrade scenario');
        }
        return record(scenario.id, 'not-supplied', 'ANVIL_PREVIOUS_PUBLIC_BIN unset');
      }
      const result = runCommand([bin, 'mcp', 'serve', '--help'], options.root, {
        ANVIL_PREVIOUS_PUBLIC_BIN: previous,
      });
      if (result.status !== 0) {
        fail(`upgrade scenario failed`, `${result.stdout}${result.stderr}`);
      }
      return record(scenario.id, 'pass', previous);
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

function main() {
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

  const identity = {
    sourceSha: gitSha(options.root),
    binary: {
      path: bin,
      sha256: sha256File(bin),
      version: binaryVersion(bin),
    },
    platform: {
      os: process.platform,
      arch: process.arch,
      name: currentPlatform(),
    },
    executedAt: new Date().toISOString(),
    catalog: catalog.path,
    scenarios: scenarioResults,
    previousPublicBinary: process.env.ANVIL_PREVIOUS_PUBLIC_BIN || null,
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

main();
