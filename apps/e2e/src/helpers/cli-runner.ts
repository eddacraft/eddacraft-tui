/**
 * CLI Runner
 *
 * Spawns the real `anvil` CLI binary as a child process and captures
 * stdout, stderr, and exit code. Designed for non-interactive E2E tests.
 *
 * The CLI is now a Rust binary (ADR-011/011a). When a Rust build is present
 * the runner spawns it; when absent, `cliBinaryAvailable()` returns false so
 * test suites can skip gracefully rather than fail hard. A pure TypeScript
 * test run therefore does not require `cargo build`.
 */

import { execFile } from 'node:child_process';
import { resolve } from 'node:path';
import { accessSync, constants as fsConstants, statSync } from 'node:fs';

const EXE = process.platform === 'win32' ? 'anvil.exe' : 'anvil';
const REPO_ROOT = resolve(__dirname, '../../../..');
const CONFIGURED_TARGET_DIR = process.env.CARGO_TARGET_DIR;
const RUST_CANDIDATES = [
  ...(CONFIGURED_TARGET_DIR
    ? [resolve(CONFIGURED_TARGET_DIR, 'debug', EXE), resolve(CONFIGURED_TARGET_DIR, 'release', EXE)]
    : []),
  resolve(REPO_ROOT, 'target/debug', EXE),
  resolve(REPO_ROOT, 'target/release', EXE),
];

function isUsableBinary(p: string): boolean {
  // Existence alone is not enough: a failed `cargo build`, a cached artefact,
  // or a wrong-arch cross-build can leave behind a zero-byte or non-executable
  // file. Treat those as "binary not built" so CLI suites skip cleanly rather
  // than run against a broken binary and report opaque exit 127s.
  try {
    const stat = statSync(p);
    if (!stat.isFile() || stat.size === 0) return false;
    if (process.platform !== 'win32') {
      accessSync(p, fsConstants.X_OK);
    }
    return true;
  } catch {
    return false;
  }
}

export function resolveCliBinary(): string | undefined {
  return RUST_CANDIDATES.find(isUsableBinary);
}

export interface CliResult {
  exitCode: number;
  stdout: string;
  stderr: string;
  output: string;
}

export interface CliRunOptions {
  cwd?: string;
  env?: Record<string, string | undefined>;
  timeout?: number;
  /** Keep the shared CI/no-TUI guard on by default; disable only in tests that
   * explicitly exercise stdio-based interactivity detection. */
  forceNonInteractive?: boolean;
}

/** Returns true when a Rust `anvil` binary exists to run against. */
export function cliBinaryAvailable(): boolean {
  return resolveCliBinary() !== undefined;
}

/**
 * Throw if the CLI binary cannot be found. Most suites should prefer
 * `describe.skipIf(!cliBinaryAvailable())` so a missing Rust build does
 * not turn into a hard failure on TypeScript-only runs.
 */
export function assertCliBuild(): void {
  if (!cliBinaryAvailable()) {
    throw new Error(
      `anvil CLI binary not found — build with \`cargo build\` first.\n` +
        `Searched:\n  - ${RUST_CANDIDATES.join('\n  - ')}`
    );
  }
}

export function runCli(args: string[], options: CliRunOptions = {}): Promise<CliResult> {
  const { cwd = process.cwd(), env = {}, timeout = 15_000, forceNonInteractive = true } = options;
  const binary = resolveCliBinary();
  if (!binary) {
    return Promise.resolve({
      exitCode: 127,
      stdout: '',
      stderr: 'anvil binary not built',
      output: 'anvil binary not built',
    });
  }

  const childEnv: Record<string, string> = {};
  for (const [key, value] of Object.entries({
    ...process.env,
    ...env,
    NO_COLOR: '1',
    FORCE_COLOR: '0',
    ...(forceNonInteractive ? { CI: 'true', NO_TUI: '1' } : {}),
  })) {
    if (value !== undefined) childEnv[key] = value;
  }

  return new Promise((resolve) => {
    execFile(
      binary,
      args,
      {
        cwd,
        timeout,
        env: childEnv,
        maxBuffer: 10 * 1024 * 1024,
      },
      (error, stdout, stderr) => {
        // `execFile` errors carry `code` as a numeric process exit code on
        // a normal non-zero exit, but as a string sentinel (e.g. `'ENOENT'`,
        // `'ETIMEDOUT'`) when the OS rejects the spawn or the process is
        // killed before it could exit. Casting a string to `number` yields
        // `NaN`, which then leaks into `CliResult.exitCode` and breaks
        // assertions. Branch on the runtime type instead so:
        //   - numeric `code` → use it
        //   - string `code`  → 127 for ENOENT-class spawn failures, 1 otherwise
        //   - no `code`      → 0 (the success path)
        let exitCode = 0;
        if (error && 'code' in error) {
          const code = (error as { code?: unknown }).code;
          if (typeof code === 'number') {
            exitCode = code;
          } else if (typeof code === 'string') {
            exitCode = code === 'ENOENT' ? 127 : 1;
          } else {
            exitCode = 1;
          }
        }
        resolve({
          exitCode,
          stdout: stdout?.toString() ?? '',
          stderr: stderr?.toString() ?? '',
          output: `${stdout ?? ''}${stderr ?? ''}`,
        });
      }
    );
  });
}

export async function runCliExpectSuccess(
  args: string[],
  options: CliRunOptions = {}
): Promise<CliResult> {
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

export async function runCliExpectFailure(
  args: string[],
  options: CliRunOptions = {}
): Promise<CliResult> {
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
