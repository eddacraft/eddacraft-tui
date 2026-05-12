#!/usr/bin/env node
/**
 * ANVFMT — `.anvil` → compiled pattern registry CLI.
 *
 * Usage:
 *   tsx scripts/compile-patterns.ts [--input <dir>] [--output <file>] [--check]
 *
 * Defaults assume invocation from the repository root:
 *   --input   patterns
 *   --output  patterns/compiled/registry.json
 *
 * Flags:
 *   --check   Do not write; exit non-zero if any errors or warnings surface
 *             or if the existing registry on disk does not match a fresh
 *             compile. Intended for CI drift detection.
 */

import { existsSync, promises as fs } from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  compilePatterns,
  type AnvilCompileIssue,
  type AnvilCompileResult,
} from '../src/anvil-format/compile.js';

interface CliArgs {
  input: string;
  output: string;
  check: boolean;
  workspaceRoot: string;
}

/**
 * Resolve the monorepo root by walking up from the script's directory looking
 * for `pnpm-workspace.yaml`. This lets `pnpm --filter ... run patterns:compile`
 * find `patterns/` at the repo root without the caller passing `--input`.
 *
 * If the walker finds `pnpm-workspace.yaml` but the sibling `patterns/`
 * directory does not exist, we assume the walker escaped into an ancestor
 * monorepo (e.g. this repo vendored inside another workspace) and fall back
 * to `startDir` rather than silently resolving inputs outside the project.
 */
function findWorkspaceRoot(startDir: string): string {
  let dir = startDir;
  for (;;) {
    if (existsSync(path.join(dir, 'pnpm-workspace.yaml'))) {
      if (existsSync(path.join(dir, 'patterns'))) return dir;
      // Keep walking — this workspace manifest has no `patterns/`, so it's
      // the wrong root. If nothing else matches we'll fall back to startDir.
    }
    const parent = path.dirname(dir);
    if (parent === dir) return startDir;
    dir = parent;
  }
}

function parseArgs(argv: string[], workspaceRoot: string): CliArgs {
  const args: CliArgs = {
    input: path.join(workspaceRoot, 'patterns'),
    output: path.join(workspaceRoot, 'patterns', 'compiled', 'registry.json'),
    check: false,
    workspaceRoot,
  };

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    switch (arg) {
      case '--input':
      case '-i': {
        const value = argv[++i];
        if (!value) throw new Error('--input requires a value');
        args.input = value;
        break;
      }
      case '--output':
      case '-o': {
        const value = argv[++i];
        if (!value) throw new Error('--output requires a value');
        args.output = value;
        break;
      }
      case '--check':
        args.check = true;
        break;
      case '--help':
      case '-h': {
        process.stdout.write(usage());
        process.exit(0);
      }
      // eslint-disable-next-line no-fallthrough -- process.exit never returns
      default:
        throw new Error(`unknown argument: ${arg}`);
    }
  }

  return args;
}

function usage(): string {
  return `Usage: compile-patterns [--input <dir>] [--output <file>] [--check]\n`;
}

function printIssues(label: string, issues: AnvilCompileIssue[]): void {
  if (issues.length === 0) return;
  process.stderr.write(`${label}:\n`);
  for (const issue of issues) {
    process.stderr.write(`  [${issue.path}] ${issue.detail}\n`);
  }
}

async function readExisting(outputPath: string): Promise<string | null> {
  try {
    return await fs.readFile(outputPath, 'utf8');
  } catch (err) {
    const errno = err as NodeJS.ErrnoException;
    if (errno.code === 'ENOENT') return null;
    throw err;
  }
}

/**
 * Compare two serialised registries for equality, ignoring the non-
 * deterministic `compiled_at` timestamp. Operates on parsed JSON rather
 * than raw strings so whitespace, key order, or future additional
 * non-deterministic fields can be added without a regex rewrite.
 */
function registriesMatch(existing: string, fresh: string): boolean {
  let a: Record<string, unknown>;
  let b: Record<string, unknown>;
  try {
    a = JSON.parse(existing) as Record<string, unknown>;
    b = JSON.parse(fresh) as Record<string, unknown>;
  } catch {
    return false;
  }
  delete a.compiled_at;
  delete b.compiled_at;
  return JSON.stringify(a) === JSON.stringify(b);
}

async function main(): Promise<void> {
  const scriptDir = path.dirname(fileURLToPath(import.meta.url));
  const workspaceRoot = findWorkspaceRoot(scriptDir);
  const args = parseArgs(process.argv.slice(2), workspaceRoot);
  const inputDir = path.resolve(args.input);
  const outputFile = path.resolve(args.output);

  if (!existsSync(inputDir)) {
    process.stderr.write(`compile-patterns: input directory does not exist: ${inputDir}\n`);
    process.exit(1);
  }

  const result: AnvilCompileResult = await compilePatterns({
    patternsDir: inputDir,
    referenceRoot: args.workspaceRoot,
  });

  printIssues('errors', result.errors);
  printIssues('warnings', result.warnings);

  if (!result.registry) {
    process.stderr.write(`\ncompile failed: ${result.errors.length} error(s)\n`);
    process.exit(1);
  }

  const serialised = JSON.stringify(result.registry, null, 2) + '\n';

  if (args.check) {
    if (result.warnings.length > 0) {
      process.stderr.write(
        `\n--check: compile produced ${result.warnings.length} warning(s); treating as drift.\n`
      );
      process.exit(1);
    }
    const existing = await readExisting(outputFile);
    if (existing === null) {
      process.stderr.write(
        `\n--check: no compiled registry at ${args.output}; run without --check to generate.\n`
      );
      process.exit(1);
    }
    if (!registriesMatch(existing, serialised)) {
      process.stderr.write(
        `\n--check: compiled registry at ${args.output} is stale; rerun the compiler.\n`
      );
      process.exit(1);
    }
    process.stdout.write(
      `compile-patterns: ${result.registry.patterns.length} patterns in ${result.registry.families.length} families — in sync\n`
    );
    return;
  }

  await fs.mkdir(path.dirname(outputFile), { recursive: true });
  await fs.writeFile(outputFile, serialised, 'utf8');
  process.stdout.write(
    `compile-patterns: wrote ${result.registry.patterns.length} patterns (${result.registry.families.length} families) → ${path.relative(process.cwd(), outputFile)}\n`
  );
}

const invokedDirectly =
  typeof process.argv[1] === 'string' &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);

if (invokedDirectly) {
  main().catch((err: unknown) => {
    const detail = err instanceof Error ? (err.stack ?? err.message) : String(err);
    process.stderr.write(`compile-patterns: ${detail}\n`);
    process.exit(1);
  });
}
