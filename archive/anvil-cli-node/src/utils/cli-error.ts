/**
 * Typed error for CLI command failures.
 *
 * Thrown instead of calling process.exit() so that:
 * - finally blocks and cleanup run
 * - Commander.js post-action hooks execute
 * - the CLI is testable without mocking process.exit
 *
 * The top-level catch handler in index.ts reads `exitCode` and
 * calls process.exit() once, at the boundary.
 */
export class CliError extends Error {
  /** When true, the command already printed user-facing output before throwing. */
  public readonly reported: boolean;

  constructor(
    message: string,
    public readonly exitCode: number = 1,
    options?: { reported?: boolean }
  ) {
    super(message);
    this.name = 'CliError';
    this.reported = options?.reported ?? false;
  }
}

/**
 * Signals clean early termination (exit code 0).
 *
 * Thrown instead of calling process.exit(0) — same benefits as CliError
 * but semantically distinct: the command succeeded, it just wants to
 * stop execution early (e.g. after printing results).
 */
export class CliExit extends Error {
  readonly exitCode = 0;

  constructor(message = 'Clean exit') {
    super(message);
    this.name = 'CliExit';
  }
}
