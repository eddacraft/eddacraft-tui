/**
 * Shared git subprocess wrappers — public API for `@eddacraft/anvil-core/utils`.
 *
 * Consolidates direct `execFile`/`execFileSync` git calls behind a typed API.
 * Every call uses the array-arg form of `execFile` (no shell) with a default
 * 30 s timeout.
 */

import { execFile, execFileSync } from 'node:child_process';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

export interface GitExecOptions {
  cwd?: string;
  /** @default 30_000 */
  timeout?: number;
  /** Maximum bytes allowed on stdout. Only applies to async `gitExec`. */
  maxBuffer?: number;
}

export interface GitExecResult {
  stdout: string;
  stderr: string;
}

export class GitOperationError extends Error {
  readonly command: string;
  readonly args: readonly string[];
  readonly exitCode: number | null;
  readonly stderr: string;

  constructor(
    command: string,
    args: readonly string[],
    exitCode: number | null,
    stderr: string,
    cause?: unknown
  ) {
    super(`git ${command} failed (exit ${exitCode ?? '?'}): ${stderr.slice(0, 500)}`, { cause });
    this.name = 'GitOperationError';
    this.command = command;
    this.args = args;
    this.exitCode = exitCode;
    this.stderr = stderr;
  }
}

const DEFAULT_TIMEOUT = 30_000;

/**
 * Execute a git command asynchronously.
 *
 * @throws {GitOperationError} When the git process exits with a non-zero code.
 */
export async function gitExec(
  args: readonly string[],
  options: GitExecOptions = {}
): Promise<GitExecResult> {
  const { cwd, timeout = DEFAULT_TIMEOUT, maxBuffer } = options;

  try {
    const { stdout, stderr } = await execFileAsync('git', [...args], {
      cwd,
      encoding: 'utf8',
      timeout,
      ...(maxBuffer !== undefined && { maxBuffer }),
    });

    return { stdout: stdout.trimEnd(), stderr: stderr.trim() };
  } catch (error: unknown) {
    const exitCode =
      error !== null && typeof error === 'object' && 'code' in error
        ? (error as { code: number | null }).code
        : null;
    const stderr =
      error !== null && typeof error === 'object' && 'stderr' in error
        ? String((error as { stderr: unknown }).stderr).trim()
        : '';

    throw new GitOperationError(args[0] ?? 'git', args, exitCode, stderr, error);
  }
}

/**
 * Execute a git command synchronously.
 *
 * @throws {GitOperationError} When the git process exits with a non-zero code.
 */
export function gitExecSync(args: readonly string[], options: GitExecOptions = {}): string {
  const { cwd, timeout = DEFAULT_TIMEOUT } = options;

  try {
    return execFileSync('git', [...args], {
      cwd,
      encoding: 'utf8',
      timeout,
      stdio: ['pipe', 'pipe', 'pipe'],
    }).trimEnd();
  } catch (error: unknown) {
    const exitCode =
      error !== null && typeof error === 'object' && 'status' in error
        ? (error as { status: number | null }).status
        : null;
    const stderr =
      error !== null && typeof error === 'object' && 'stderr' in error
        ? String((error as { stderr: unknown }).stderr).trim()
        : '';

    throw new GitOperationError(args[0] ?? 'git', args, exitCode, stderr, error);
  }
}

export async function gitRevParse(
  cwd: string,
  { short = false }: { short?: boolean } = {}
): Promise<string> {
  const args = short ? ['rev-parse', '--short', 'HEAD'] : ['rev-parse', 'HEAD'];
  const { stdout } = await gitExec(args, { cwd });
  return stdout;
}

export async function gitCurrentBranch(cwd: string): Promise<string> {
  const { stdout } = await gitExec(['rev-parse', '--abbrev-ref', 'HEAD'], { cwd });
  return stdout;
}

export async function gitRemoteUrl(cwd: string, remote = 'origin'): Promise<string | undefined> {
  try {
    const { stdout } = await gitExec(['remote', 'get-url', remote], { cwd });
    return stdout || undefined;
  } catch {
    return undefined;
  }
}

export async function gitStatusPorcelain(cwd: string): Promise<string> {
  const { stdout } = await gitExec(['status', '--porcelain'], { cwd });
  return stdout;
}

export async function gitStagedFiles(cwd: string): Promise<string[]> {
  const { stdout } = await gitExec(['diff', '--name-only', '--cached'], { cwd });
  return stdout.split('\n').filter(Boolean);
}

export async function gitLastCommitMessage(cwd: string): Promise<string> {
  const { stdout } = await gitExec(['log', '-1', '--format=%s'], { cwd });
  return stdout;
}

export async function gitLastCommitAuthor(cwd: string): Promise<string> {
  const { stdout } = await gitExec(['log', '-1', '--format=%an <%ae>'], { cwd });
  return stdout;
}

export function gitRevParseSync(cwd: string, { short = false }: { short?: boolean } = {}): string {
  const args = short ? ['rev-parse', '--short', 'HEAD'] : ['rev-parse', 'HEAD'];
  return gitExecSync(args, { cwd });
}

export function gitCurrentBranchSync(cwd: string): string {
  return gitExecSync(['rev-parse', '--abbrev-ref', 'HEAD'], { cwd });
}

export function gitStatusPorcelainSync(cwd: string): string {
  return gitExecSync(['status', '--porcelain'], { cwd });
}

export function gitLogSync(args: readonly string[], options: GitExecOptions = {}): string {
  return gitExecSync(['log', ...args], options);
}
