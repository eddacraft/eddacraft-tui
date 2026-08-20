#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, join, relative, resolve, sep } from 'node:path';

import { annotateSvg, loadContract } from './lib/public-diagrams.mjs';

const args = process.argv.slice(2);
const binaryIndex = args.indexOf('--drawio-bin');
const rootIndex = args.indexOf('--root');
const drawioBinary = binaryIndex >= 0 && args[binaryIndex + 1] ? args[binaryIndex + 1] : 'drawio';
const positional = args.filter(
  (_, index) =>
    index !== binaryIndex &&
    index !== binaryIndex + 1 &&
    index !== rootIndex &&
    index !== rootIndex + 1
);
if (positional.length !== 1) {
  fail('usage: node scripts/docs/export-public-diagram.mjs <path.drawio> [--drawio-bin <path>]');
}

const repoRoot =
  rootIndex >= 0 && args[rootIndex + 1]
    ? resolve(args[rootIndex + 1])
    : resolve(import.meta.dirname, '../..');
const contract = await loadContract(repoRoot);
const sourcePath = resolve(repoRoot, positional[0]);
const relativeSource = relative(repoRoot, sourcePath).split(sep).join('/');
const family = contract.families.find(({ root }) => relativeSource.startsWith(`${root}/`));
if (!family) {
  fail(`${relativeSource} is outside the governed mounted public family roots`);
}
if (!/\/[a-z0-9]+(?:-[a-z0-9]+)*\.drawio$/.test(`/${relativeSource}`)) {
  fail('Draw.io source must use a lower-kebab-case .drawio basename');
}

const versionResult = spawnSync(drawioBinary, ['--version'], {
  encoding: 'utf8',
  stdio: ['ignore', 'pipe', 'pipe'],
});
if (versionResult.status !== 0) {
  fail(
    `could not run Draw.io Desktop version check: ${versionResult.stderr || versionResult.error?.message || 'unknown error'}`
  );
}
const versionOutput = `${versionResult.stdout}\n${versionResult.stderr}`;
if (!versionOutput.includes(contract.drawioDesktopVersion)) {
  fail(
    `Draw.io Desktop ${contract.drawioDesktopVersion} is required; got ${versionOutput.trim() || 'unknown'}`
  );
}

const temporary = await mkdtemp(join(tmpdir(), 'anvil-drawio-export-'));
try {
  const rawOutput = join(temporary, `${basename(sourcePath, '.drawio')}.svg`);
  const exportResult = spawnSync(
    drawioBinary,
    [...contract.exportArgs, '--output', rawOutput, sourcePath],
    { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }
  );
  if (exportResult.status !== 0) {
    fail(
      `Draw.io Desktop export failed: ${exportResult.stderr || exportResult.error?.message || 'unknown error'}`
    );
  }

  const [source, rawSvg] = await Promise.all([
    readFile(sourcePath, 'utf8'),
    readFile(rawOutput, 'utf8'),
  ]);
  const outputPath = join(dirname(sourcePath), `${basename(sourcePath, '.drawio')}.svg`);
  const annotated = annotateSvg({
    svg: rawSvg,
    source,
    sourceName: basename(sourcePath),
    contract,
  });
  await writeFile(outputPath, annotated, 'utf8');
  process.stdout.write(
    `[public-diagrams] exported ${relativeSource} -> ${relative(repoRoot, outputPath).split(sep).join('/')} with Draw.io Desktop ${contract.drawioDesktopVersion}\n`
  );
} finally {
  await rm(temporary, { recursive: true, force: true });
}

function fail(message) {
  process.stderr.write(`[public-diagrams] ERROR: ${message}\n`);
  process.exit(1);
}
