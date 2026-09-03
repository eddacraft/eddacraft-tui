#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, statSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import process from 'node:process';

const MD_GLOB = '**/*.{md,markdown}';
const TOOLING_EXIT = 2;

function couldNotRun(reason) {
  process.stderr.write(`[markdownlint] could not run: ${reason}\n`);
  process.exit(TOOLING_EXIT);
}

function usage() {
  process.stderr.write(
    'Usage: scripts/docs/run-markdownlint.mjs [--cwd PATH] [--fix] [--omit-ignored] [--list-files] [--ignore-path PATH] [paths...]\n'
  );
}

function loadIgnoreLines(ignoreFile) {
  if (!existsSync(ignoreFile)) return [];
  return readFileSync(ignoreFile, 'utf8')
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith('#'));
}

function toPosix(rel) {
  return rel.split(sep).join('/');
}

function loadGlobby() {
  try {
    const require = createRequire(import.meta.url);
    const globby = require('globby');
    if (typeof globby?.sync !== 'function') {
      couldNotRun('globby.sync is not available');
    }
    return globby;
  } catch (err) {
    couldNotRun(`cannot load globby (${err.code || err.message})`);
  }
}

function markdownlintBin() {
  try {
    const require = createRequire(import.meta.url);
    const pkg = require.resolve('markdownlint-cli/package.json');
    const bin = join(dirname(pkg), 'markdownlint.js');
    if (!existsSync(bin)) {
      couldNotRun(`markdownlint-cli entry is missing (${bin})`);
    }
    return bin;
  } catch (err) {
    couldNotRun(`cannot resolve markdownlint-cli (${err.code || err.message})`);
  }
}

function parseArgs(argv) {
  let cwd = process.cwd();
  let fix = false;
  let listFiles = false;
  let omitIgnored = false;
  let ignorePath = '.markdownlintignore';
  const paths = [];
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--help' || arg === '-h') {
      usage();
      process.exit(0);
    }
    if (arg === '--fix') {
      fix = true;
      continue;
    }
    if (arg === '--omit-ignored') {
      omitIgnored = true;
      continue;
    }
    if (arg === '--list-files') {
      listFiles = true;
      continue;
    }
    if (arg === '--cwd' || arg === '--ignore-path') {
      const value = argv[i + 1];
      if (value === undefined || value.startsWith('--')) {
        process.stderr.write(`run-markdownlint.mjs: ${arg} requires a value\n`);
        process.exit(TOOLING_EXIT);
      }
      if (arg === '--cwd') cwd = value;
      else ignorePath = value;
      i += 1;
      continue;
    }
    if (arg.startsWith('-')) {
      process.stderr.write(`run-markdownlint.mjs: unknown argument: ${arg}\n`);
      process.exit(TOOLING_EXIT);
    }
    paths.push(arg);
  }
  return {
    cwd: resolve(cwd),
    fix,
    listFiles,
    omitIgnored,
    ignorePath,
    paths: paths.length > 0 ? paths : ['.'],
  };
}

function classify(cwd, inputPath) {
  const abs = isAbsolute(inputPath) ? inputPath : resolve(cwd, inputPath);
  const rel = toPosix(relative(cwd, abs)) || '.';
  if (!existsSync(abs)) {
    return { kind: 'missing', abs, rel, inputPath };
  }
  const stat = statSync(abs);
  if (stat.isDirectory()) {
    return { kind: 'directory', abs, rel, inputPath };
  }
  return { kind: 'file', abs, rel, inputPath };
}

function globFiles(globby, cwd, patterns, ignoreLines) {
  return globby
    .sync(patterns, {
      cwd,
      ignore: ignoreLines,
      onlyFiles: true,
      unique: true,
      // markdownlint-cli defaults `dot` to false unless `--dot` is passed.
      // Keep the repo sweep identical (CIB-390: `pnpm lint:md` unchanged).
      dot: false,
      absolute: false,
    })
    .map(toPosix);
}

function run() {
  const args = parseArgs(process.argv.slice(2));
  const globby = loadGlobby();
  const ignoreFile = isAbsolute(args.ignorePath)
    ? args.ignorePath
    : resolve(args.cwd, args.ignorePath);
  const ignoreLines = loadIgnoreLines(ignoreFile);
  const namedExcluded = [];
  const missing = [];
  const toLint = new Set();

  for (const inputPath of args.paths) {
    const classified = classify(args.cwd, inputPath);
    if (classified.kind === 'missing') {
      missing.push(classified.inputPath);
      continue;
    }
    if (classified.kind === 'directory') {
      const pattern =
        classified.rel === '.' ? MD_GLOB : `${classified.rel.replace(/\/+$/u, '')}/${MD_GLOB}`;
      for (const file of globFiles(globby, args.cwd, [pattern], ignoreLines)) {
        toLint.add(file);
      }
      continue;
    }
    const kept = globFiles(globby, args.cwd, [classified.rel], ignoreLines);
    if (kept.length === 0) {
      if (!args.omitIgnored) namedExcluded.push(classified.rel);
      continue;
    }
    for (const file of kept) toLint.add(file);
  }

  if (missing.length > 0) {
    process.stderr.write(`[markdownlint] named input not found: ${missing.join(', ')}\n`);
    process.exit(TOOLING_EXIT);
  }

  if (namedExcluded.length > 0) {
    process.stderr.write(
      `[markdownlint] named input excluded by ignore file: ${namedExcluded.join(', ')}\n[markdownlint] 0 files examined\n`
    );
    process.exit(TOOLING_EXIT);
  }

  const files = [...toLint].sort();
  if (args.listFiles) {
    process.stdout.write(files.join('\n') + (files.length > 0 ? '\n' : ''));
    process.exit(0);
  }

  const bin = files.length > 0 ? markdownlintBin() : null;

  process.stdout.write(`[markdownlint] ${files.length} files checked\n`);
  if (files.length === 0) {
    process.exit(0);
  }

  const result = spawnSync(
    process.execPath,
    [bin, '--ignore-path', ignoreFile, ...(args.fix ? ['--fix'] : []), ...files],
    { cwd: args.cwd, encoding: 'utf8' }
  );
  if (result.error) {
    couldNotRun(result.error.message);
  }
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  process.exit(result.status === null ? TOOLING_EXIT : result.status);
}

function main() {
  try {
    run();
  } catch (err) {
    couldNotRun(err && err.message ? err.message : String(err));
  }
}

main();
