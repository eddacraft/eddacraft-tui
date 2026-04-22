import { execFileSync } from 'node:child_process';
import { isAbsolute, relative, resolve } from 'node:path';
import chalk from 'chalk';
import type { CargoDependency, CargoMetadata, CargoPackage } from '../models/cargo-metadata';
import { runProcess } from './run-process';

/**
 * Spawn `cargo <args>` with inherited stdio and always-on colour. Returns the
 * success flag. Logs the command in dim text so failures are easy to
 * reproduce.
 *
 * Cargo rejects any flag before `+toolchain`, so if the first arg is a
 * toolchain selector we emit it ahead of `--color always`.
 */
export async function cargoCommand(...args: string[]): Promise<{ success: boolean }> {
  const [head, ...rest] = args;
  const ordered =
    head && head.startsWith('+')
      ? [head, '--color', 'always', ...rest]
      : ['--color', 'always', ...args];

   
  console.log(chalk.dim(`> cargo ${redactArgs(ordered).join(' ')}`));
  return runProcess('cargo', ...ordered);
}

/**
 * Redact secret-bearing flag values in-place for log output. Cargo surfaces
 * tokens via `--token <value>`; that value must never appear in terminal
 * history, `/proc/<pid>/cmdline` readers, or CI log scrapers.
 */
function redactArgs(argv: readonly string[]): string[] {
  const SECRET_FLAGS = new Set<string>(['--token']);
  const out: string[] = [];
  for (let i = 0; i < argv.length; i++) {
    const token = argv[i];
    out.push(token);
    if (SECRET_FLAGS.has(token) && i + 1 < argv.length) {
      out.push('***');
      i++;
    }
  }
  return out;
}

/**
 * Run `cargo metadata --format-version=1` and parse the JSON output. Returns
 * `null` on failure — graph resolution has to be resilient to transient cargo
 * errors (e.g. during `cargo clean`).
 *
 * `cargo metadata` is the supported stable contract for consuming a Cargo
 * workspace; parsing Cargo.toml by hand loses resolved versions, path-dep
 * resolution, and external dependency source info.
 *
 * Uses `execFileSync` (no shell) so cargo arg injection is not possible.
 */
export function cargoMetadata(cwd?: string): CargoMetadata | null {
  try {
    const output = execFileSync('cargo', ['metadata', '--format-version=1'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      maxBuffer: 1024 * 1024 * 64,
      cwd,
      windowsHide: true,
    });
    return JSON.parse(output) as CargoMetadata;
  } catch {
    return null;
  }
}

/**
 * True if the package/dep resolves to a registry, git, or out-of-workspace
 * path. Used to decide whether a dep becomes an internal Nx edge or an
 * external `cargo:<name>` node.
 */
export function isExternal(
  packageOrDep: CargoPackage | CargoDependency,
  workspaceRoot: string
): boolean {
  const source = packageOrDep.source ?? '';
  if (source.startsWith('registry+')) return true;
  if (source.startsWith('git+')) return true;

  // cargo metadata emits absolute manifest/path values, so the workspace root
  // must also be absolute for `relative()` to produce correct answers.
  const absRoot = isAbsolute(workspaceRoot) ? workspaceRoot : resolve(workspaceRoot);

  const candidate =
    ('manifest_path' in packageOrDep && packageOrDep.manifest_path) ||
    ('path' in packageOrDep && packageOrDep.path) ||
    null;

  // No source and no path → almost certainly a workspace-inherited registry
  // dep whose `source` is elided in the metadata. Treat as external; a missing
  // path cannot describe a local path dep.
  if (!candidate) return true;

  const absCandidate = isAbsolute(candidate) ? candidate : resolve(absRoot, candidate);
  const rel = relative(absRoot, absCandidate);
  return rel.startsWith('..') || isAbsolute(rel);
}
