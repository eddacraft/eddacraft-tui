/**
 * Console Mocking Utilities
 *
 * Helpers for capturing and mocking console output in tests
 */

export interface ConsoleMock {
  log: string[];
  error: string[];
  warn: string[];
  restore: () => void;
}

/**
 * Mock console methods to capture output
 */
export function mockConsole(): ConsoleMock {
  const originalLog = console.log;
  const originalError = console.error;
  const originalWarn = console.warn;

  const logs: string[] = [];
  const errors: string[] = [];
  const warns: string[] = [];

  console.log = (...args: unknown[]) => {
    logs.push(args.map((arg) => String(arg)).join(' '));
  };

  console.error = (...args: unknown[]) => {
    errors.push(args.map((arg) => String(arg)).join(' '));
  };

  console.warn = (...args: unknown[]) => {
    warns.push(args.map((arg) => String(arg)).join(' '));
  };

  return {
    log: logs,
    error: errors,
    warn: warns,
    restore: () => {
      console.log = originalLog;
      console.error = originalError;
      console.warn = originalWarn;
    },
  };
}

/**
 * @deprecated CLI commands now throw CliError/CliExit instead of calling
 * process.exit(). Assert with `toThrow(CliError)` or `toThrow(CliExit)`.
 * See: src/utils/cli-error.ts
 */
export function mockProcessExit(): {
  exitCode: number | null;
  restore: () => void;
} {
  const originalExit = process.exit;
  let exitCode: number | null = null;

  // @ts-expect-error - Mocking process.exit
  process.exit = (code?: number) => {
    exitCode = code ?? 0;
    throw new Error(`process.exit(${exitCode})`);
  };

  return {
    get exitCode() {
      return exitCode;
    },
    restore: () => {
      process.exit = originalExit;
    },
  };
}
