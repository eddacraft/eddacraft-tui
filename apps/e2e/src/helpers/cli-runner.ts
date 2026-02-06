/**
 * CLI Runner
 *
 * Spawns the real `anvil` CLI binary as a child process and captures
 * stdout, stderr, and exit code. Designed for non-interactive E2E tests.
 *
 * For interactive TUI tests, use the tuistory-based tests in
 * apps/anvil-cli/src/__tests__/e2e/ instead.
 */

import { execFile } from 'node:child_process';
import { resolve } from 'node:path';
import { existsSync } from 'node:fs';

/** Path to the built CLI entry point */
const CLI_ENTRY = resolve(__dirname, '../../../../anvil-cli/dist/index.js');

export interface CliResult {
  /** Process exit code (0 = success) */
  exitCode: number;
  /** Combined stdout output */
  stdout: string;
  /** Combined stderr output */
  stderr: string;
  /** stdout + stderr for convenience */
  output: string;
}

export interface CliRunOptions {
  /** Working directory for the CLI process */
  cwd?: string;
  /** Extra environment variables (merged with process.env) */
  env?: Record<string, string | undefined>;
  /** Timeout in ms (default: 15 000) */
  timeout?: number;
}

/**
 * Ensure the CLI has been built before tests run.
 * Call this in a beforeAll() block.
 */
export function assertCliBuild(): void {
  if (!existsSync(CLI_ENTRY)) {
    throw new Error(
      `CLI not built — run \`pnpm build\` before E2E tests.\nExpected: ${CLI_ENTRY}`
    );
  }
}

/**
 * Run an `anvil` CLI command and return the result.
 *
 * @example
 * ```ts
 * const result = await runCli(['--version']);
 * expect(result.exitCode).toBe(0);
 * expect(result.stdout).toMatch(/\d+\.\d+\.\d+/);
 * ```
 */
export function runCli(args: string[], options: CliRunOptions = {}): Promise<CliResult> {
  const { cwd = process.cwd(), env = {}, timeout = 15_000 } = options;

  return new Promise((resolve) => {
    const child = execFile(
      process.execPath,
      [CLI_ENTRY, ...args],
      {
        cwd,
        timeout,
        env: {
          ...process.env,
          ...env,
          // Suppress colour codes and TUI for clean assertions
          NO_COLOR: '1',
          FORCE_COLOR: '0',
          CI: 'true',
          NO_TUI: '1',
        },
        maxBuffer: 10 * 1024 * 1024, // 10 MB
      },
      (error, stdout, stderr) => {
        const exitCode = error && 'code' in error ? (error.code as number) ?? 1 : 0;
        resolve({
          exitCode,
          stdout: stdout?.toString() ?? '',
          stderr: stderr?.toString() ?? '',
          output: `${stdout ?? ''}${stderr ?? ''}`,
        });
      }
    );

    // Safety net — kill if the process hangs
    child.on('error', () => {
      /* handled by callback */
    });
  });
}

/**
 * Run CLI and assert it succeeds (exit code 0).
 * Throws with full output on failure.
 */
export async function runCliExpectSuccess(args: string[], options: CliRunOptions = {}): Promise<CliResult> {
  const result = await runCli(args, options);
  if (result.exitCode !== 0) {
    throw new Error(
      `CLI exited with code ${result.exitCode}\n` +
        `Command: anvil ${args.join(' ')}\n` +
        `stdout: ${result.stdout}\n` +
        `stderr: ${result.stderr}`
    );
  }
  return result;
}

/**
 * Run CLI and assert it fails (non-zero exit code).
 */
export async function runCliExpectFailure(args: string[], options: CliRunOptions = {}): Promise<CliResult> {
  const result = await runCli(args, options);
  if (result.exitCode === 0) {
    throw new Error(
      `Expected CLI to fail but it succeeded\n` +
        `Command: anvil ${args.join(' ')}\n` +
        `stdout: ${result.stdout}`
    );
  }
  return result;
}
