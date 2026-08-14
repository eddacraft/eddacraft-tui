#!/usr/bin/env node
/**
 * ANVFMT — `.anvil` → compiled pattern registry CLI.
 *
 * Usage:
 *   tsx scripts/compile-patterns.ts [--input <dir>] [--output <file>]
 *                                   [--check] [--strict]
 *
 * Defaults assume invocation from the repository root:
 *   --input   patterns
 *   --output  patterns/compiled/registry.json
 *
 * Flags:
 *   --check   Do not write. Exit non-zero on compile ERRORS or when the
 *             registry on disk does not match a fresh compile. Compiler
 *             warnings are reported but are advisory — see `--strict`.
 *             Intended for CI drift detection.
 *   --strict  Escalate compiler warnings to failures. Off by default: the
 *             `patterns/` tree emits nine by-design legacy `AP` prefix
 *             collision warnings on every run, and conflating those with
 *             registry drift made `--check` unable to pass (CIB-335).
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
  strict: boolean;
  help: boolean;
  workspaceRoot: string;
}

/** Output sinks, injected so the CLI is testable without spawning a process. */
export interface CompilePatternsIo {
  stdout: (chunk: string) => void;
  stderr: (chunk: string) => void;
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
    strict: false,
    help: false,
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
      case '--strict':
        args.strict = true;
        break;
      case '--help':
      case '-h':
        args.help = true;
        break;
      default:
        throw new Error(`unknown argument: ${arg}`);
    }
  }

  return args;
}

function usage(): string {
  return `Usage: compile-patterns [--input <dir>] [--output <file>] [--check] [--strict]\n`;
}

function printIssues(io: CompilePatternsIo, label: string, issues: AnvilCompileIssue[]): void {
  if (issues.length === 0) return;
  io.stderr(`${label}:\n`);
  for (const issue of issues) {
    io.stderr(`  [${issue.path}] ${issue.detail}\n`);
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

interface RegistryRecord {
  id?: unknown;
  [key: string]: unknown;
}

interface RegistryShape {
  patterns?: unknown;
  families?: unknown;
  prefixes?: unknown;
  [key: string]: unknown;
}

/**
 * Drift report. `total` is the real number of differences found; `lines` may be
 * truncated for readability, so callers must not infer a count from its length
 * (the truncation notice is itself a line, which would overstate by one while
 * understating the remainder).
 */
export interface RegistryDrift {
  total: number;
  lines: string[];
}

/** Cap on reported drift lines so a wholesale regeneration stays readable. */
const MAX_DRIFT_LINES = 25;

/**
 * Parse a registry payload, rejecting valid JSON that is not a JSON object.
 * `null`, arrays, and scalars all parse cleanly but are not registries, and
 * the field-level diff below indexes them as objects — so reject here with a
 * message rather than throwing from `delete` or a property read.
 */
function parseRegistryObject(text: string): RegistryShape {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new Error('is not valid JSON');
  }
  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
    const kind = parsed === null ? 'null' : Array.isArray(parsed) ? 'an array' : typeof parsed;
    throw new Error(`is valid JSON but ${kind}, not a registry object`);
  }
  return parsed as RegistryShape;
}

/**
 * Recursively sort object keys so two structurally identical values serialise
 * identically. Array order is preserved — the compiler sorts patterns and
 * families deliberately for stable diffs, so a reordered array IS drift.
 *
 * Flagged twice by CLAWPATCH audits (2026-05-20, 2026-05-31): comparing raw
 * `JSON.stringify` output made key order significant, so a re-serialised
 * registry with identical values reported drift.
 */
function sortKeysDeep(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortKeysDeep);
  if (value !== null && typeof value === 'object') {
    const source = value as Record<string, unknown>;
    const sorted: Record<string, unknown> = {};
    for (const key of Object.keys(source).sort()) sorted[key] = sortKeysDeep(source[key]);
    return sorted;
  }
  return value;
}

/** Order-insensitive serialisation used for every drift comparison. */
function canonical(value: unknown): string {
  return JSON.stringify(sortKeysDeep(value));
}

function asRecords(value: unknown): RegistryRecord[] {
  return Array.isArray(value) ? (value as RegistryRecord[]) : [];
}

function byId(value: unknown): Map<string, RegistryRecord> {
  const map = new Map<string, RegistryRecord>();
  for (const entry of asRecords(value)) {
    if (entry && typeof entry.id === 'string') map.set(entry.id, entry);
  }
  return map;
}

/** Name the first field on which two registry records disagree. */
function firstDifferingField(a: RegistryRecord, b: RegistryRecord): string {
  const keys = [...new Set([...Object.keys(a), ...Object.keys(b)])].sort();
  for (const key of keys) {
    if (canonical(a[key]) !== canonical(b[key])) return key;
  }
  return '<unknown>';
}

function diffCollection(label: string, existing: unknown, fresh: unknown): string[] {
  const lines: string[] = [];
  const before = byId(existing);
  const after = byId(fresh);

  for (const id of after.keys()) {
    if (!before.has(id)) lines.push(`${label} "${id}" is missing from the committed registry`);
  }
  for (const id of before.keys()) {
    if (!after.has(id)) {
      lines.push(`${label} "${id}" is in the committed registry but no longer compiles`);
    }
  }
  for (const [id, afterEntry] of after) {
    const beforeEntry = before.get(id);
    if (!beforeEntry) continue;
    if (canonical(beforeEntry) !== canonical(afterEntry)) {
      lines.push(
        `${label} "${id}" differs (first change: ${firstDifferingField(beforeEntry, afterEntry)})`
      );
    }
  }
  return lines;
}

/**
 * Describe how the committed registry differs from a fresh compile.
 * Returns an empty array when they agree.
 *
 * Comparison is on PARSED JSON, not text: the compiler emits expanded
 * `JSON.stringify(..., 2)` while the committed `registry.json` is
 * oxfmt-normalised (short arrays collapsed onto one line), so a text
 * comparison reports drift on an identical registry. The non-deterministic
 * `compiled_at` timestamp is excluded for the same reason.
 */
export function describeRegistryDrift(existingText: string, freshText: string): RegistryDrift {
  let before: RegistryShape;
  let after: RegistryShape;
  try {
    before = parseRegistryObject(existingText);
  } catch (error) {
    return { total: 1, lines: [`the committed registry ${(error as Error).message}`] };
  }
  try {
    after = parseRegistryObject(freshText);
  } catch (error) {
    return { total: 1, lines: [`the freshly compiled registry ${(error as Error).message}`] };
  }

  delete before.compiled_at;
  delete after.compiled_at;
  if (canonical(before) === canonical(after)) return { total: 0, lines: [] };

  const lines: string[] = [];

  const structural = new Set(['patterns', 'families', 'prefixes']);
  const scalarKeys = [...new Set([...Object.keys(before), ...Object.keys(after)])]
    .filter((key) => !structural.has(key))
    .sort();
  for (const key of scalarKeys) {
    if (canonical(before[key]) !== canonical(after[key])) {
      lines.push(`${key}: ${JSON.stringify(before[key])} → ${JSON.stringify(after[key])}`);
    }
  }

  lines.push(...diffCollection('pattern', before.patterns, after.patterns));
  lines.push(...diffCollection('family', before.families, after.families));

  if (canonical(before.prefixes) !== canonical(after.prefixes)) {
    lines.push(`prefixes: ${JSON.stringify(before.prefixes)} → ${JSON.stringify(after.prefixes)}`);
  }

  if (lines.length === 0) {
    // Same values, different array element order — still drift (the compiler
    // sorts deliberately), but nothing field-level to point at.
    lines.push('registry contents match but their element order differs');
  }

  const total = lines.length;
  if (total > MAX_DRIFT_LINES) {
    const overflow = total - MAX_DRIFT_LINES;
    return {
      total,
      lines: [...lines.slice(0, MAX_DRIFT_LINES), `…and ${overflow} further difference(s)`],
    };
  }
  return { total, lines };
}

/**
 * Run the CLI and return the process exit code. Kept separate from `main()`
 * so tests can drive real CLI semantics without spawning a process.
 */
export async function runCompilePatternsCli(
  argv: string[],
  io: CompilePatternsIo,
  workspaceRootOverride?: string
): Promise<number> {
  const scriptDir = path.dirname(fileURLToPath(import.meta.url));
  const workspaceRoot = workspaceRootOverride ?? findWorkspaceRoot(scriptDir);

  let args: CliArgs;
  try {
    args = parseArgs(argv, workspaceRoot);
  } catch (err) {
    io.stderr(`compile-patterns: ${err instanceof Error ? err.message : String(err)}\n`);
    io.stderr(usage());
    return 1;
  }

  if (args.help) {
    io.stdout(usage());
    return 0;
  }

  const inputDir = path.resolve(args.input);
  const outputFile = path.resolve(args.output);

  if (!existsSync(inputDir)) {
    io.stderr(`compile-patterns: input directory does not exist: ${inputDir}\n`);
    return 1;
  }

  const result: AnvilCompileResult = await compilePatterns({
    patternsDir: inputDir,
    referenceRoot: args.workspaceRoot,
  });

  printIssues(io, 'errors', result.errors);
  printIssues(io, 'warnings', result.warnings);

  if (!result.registry || result.errors.length > 0) {
    io.stderr(`\ncompile failed: ${result.errors.length} error(s)\n`);
    return 1;
  }

  // Warnings are advisory by default. The `patterns/` tree emits nine legacy
  // `AP` prefix-collision warnings by design; treating them as failures made
  // the parity gate permanently red regardless of registry state (CIB-335).
  if (result.warnings.length > 0) {
    if (args.strict) {
      io.stderr(
        `\n--strict: compile produced ${result.warnings.length} warning(s); failing as requested.\n`
      );
      return 1;
    }
    io.stderr(
      `\nnote: ${result.warnings.length} warning(s) above are advisory and do not fail this run; pass --strict to escalate them.\n`
    );
  }

  const serialised = JSON.stringify(result.registry, null, 2) + '\n';

  if (args.check) {
    const existing = await readExisting(outputFile);
    if (existing === null) {
      io.stderr(
        `\n--check: no compiled registry at ${args.output}; run without --check to generate.\n`
      );
      return 1;
    }
    const drift = describeRegistryDrift(existing, serialised);
    if (drift.total > 0) {
      const shown =
        drift.total > drift.lines.length || drift.lines.length > MAX_DRIFT_LINES
          ? ` (showing first ${MAX_DRIFT_LINES})`
          : '';
      io.stderr(
        `\n--check: compiled registry at ${args.output} is stale — ${drift.total} difference(s)${shown} vs a fresh compile:\n`
      );
      for (const line of drift.lines) io.stderr(`  ${line}\n`);
      io.stderr(`\nRegenerate with: pnpm --filter @eddacraft/anvil-core patterns:compile\n`);
      return 1;
    }
    io.stdout(
      `compile-patterns: ${result.registry.patterns.length} patterns in ${result.registry.families.length} families — in sync\n`
    );
    return 0;
  }

  await fs.mkdir(path.dirname(outputFile), { recursive: true });
  await fs.writeFile(outputFile, serialised, 'utf8');
  io.stdout(
    `compile-patterns: wrote ${result.registry.patterns.length} patterns (${result.registry.families.length} families) → ${path.relative(process.cwd(), outputFile)}\n`
  );
  return 0;
}

async function main(): Promise<void> {
  const code = await runCompilePatternsCli(process.argv.slice(2), {
    stdout: (chunk) => process.stdout.write(chunk),
    stderr: (chunk) => process.stderr.write(chunk),
  });
  if (code !== 0) process.exit(code);
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
