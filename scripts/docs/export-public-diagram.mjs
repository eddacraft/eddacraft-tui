#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { lstat, mkdtemp, open, readFile, realpath, rename, rm } from 'node:fs/promises';
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
  fail(
    'usage: node scripts/docs/export-public-diagram.mjs <path.drawio> [--root <path>] [--drawio-bin <path>]'
  );
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
const diagramDirectory = contract.diagramDirectories.find((directory) =>
  relativeSource.startsWith(`${directory}/`)
);
if (!diagramDirectory) {
  fail(`${relativeSource} is outside the explicit governed diagram directories`);
}
if (!/\/[a-z0-9]+(?:-[a-z0-9]+)*\.drawio$/.test(`/${relativeSource}`)) {
  fail('Draw.io source must use a lower-kebab-case .drawio basename');
}
const familyRoot = resolve(repoRoot, family.root);
const diagramRoot = resolve(repoRoot, diagramDirectory);
const outputPath = join(dirname(sourcePath), `${basename(sourcePath, '.drawio')}.svg`);
try {
  await assertNoSymlinkPath(repoRoot, sourcePath);
  await assertNoSymlinkPath(repoRoot, outputPath, { allowMissingLeaf: true });
  const [canonicalFamily, canonicalDiagramRoot, canonicalSource] = await Promise.all([
    realpath(familyRoot),
    realpath(diagramRoot),
    realpath(sourcePath),
  ]);
  if (!isWithin(canonicalFamily, canonicalSource)) {
    throw new Error('Draw.io source resolves outside its governed family root');
  }
  if (!isWithin(canonicalDiagramRoot, canonicalSource)) {
    throw new Error('Draw.io source resolves outside its governed diagram directory');
  }
  if (!(await lstat(sourcePath)).isFile()) {
    throw new Error('Draw.io source must be a regular file');
  }
} catch (error) {
  fail(error.message);
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
const versionOutput = versionResult.stdout.trim();
const versionError = versionResult.stderr.trim();
if (
  versionError ||
  versionOutput !== contract.drawioDesktopVersionOutput ||
  !/^(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)$/.test(versionOutput) ||
  versionOutput !== contract.drawioDesktopVersion
) {
  fail(
    `Draw.io Desktop exact version output "${contract.drawioDesktopVersionOutput}" is required; got "${[versionOutput, versionError].filter(Boolean).join(' | ') || 'unknown'}"`
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
  const annotated = annotateSvg({
    svg: rawSvg,
    source,
    sourceName: basename(sourcePath),
    contract,
    actualVersionOutput: versionOutput,
  });
  const atomicOutput = join(
    dirname(outputPath),
    `.${basename(outputPath)}.${process.pid}.${randomUUID()}.tmp`
  );
  let outputHandle;
  try {
    outputHandle = await open(atomicOutput, 'wx', 0o644);
    await outputHandle.writeFile(annotated, 'utf8');
    await outputHandle.sync();
    await outputHandle.close();
    outputHandle = undefined;
    await assertNoSymlinkPath(repoRoot, outputPath, { allowMissingLeaf: true });
    await rename(atomicOutput, outputPath);
  } finally {
    await outputHandle?.close();
    await rm(atomicOutput, { force: true });
  }
  process.stdout.write(
    `[public-diagrams] exported ${relativeSource} -> ${relative(repoRoot, outputPath).split(sep).join('/')} with Draw.io Desktop output ${versionOutput}\n`
  );
} finally {
  await rm(temporary, { recursive: true, force: true });
}

function fail(message) {
  process.stderr.write(`[public-diagrams] ERROR: ${message}\n`);
  process.exit(1);
}

function isWithin(parent, child) {
  const path = relative(parent, child);
  return path === '' || (!path.startsWith(`..${sep}`) && path !== '..' && !path.startsWith(sep));
}

async function assertNoSymlinkPath(root, target, { allowMissingLeaf = false } = {}) {
  const path = relative(root, target);
  if (path === '..' || path.startsWith(`..${sep}`) || resolve(root, path) !== target) {
    throw new Error('diagram path escapes the repository root');
  }
  const parts = path ? path.split(sep) : [];
  let current = root;
  const candidates = [root, ...parts.map((part) => (current = join(current, part)))];
  for (const [index, candidate] of candidates.entries()) {
    try {
      const info = await lstat(candidate);
      if (info.isSymbolicLink()) {
        throw new Error(`diagram path contains a symlink: ${relative(root, candidate) || '.'}`);
      }
    } catch (error) {
      if (error.code === 'ENOENT' && allowMissingLeaf && index === candidates.length - 1) return;
      throw error;
    }
  }
}
