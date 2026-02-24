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
  constructor(
    message: string,
    public readonly exitCode: number = 1
  ) {
    super(message);
    this.name = 'CliError';
  }
}
