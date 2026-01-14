/**
 * Spinner utilities
 *
 * Shared utilities for consistent spinner handling in CLI commands.
 */

import ora, { type Ora } from 'ora';

/**
 * Options for withSpinner function
 */
export interface SpinnerOptions {
  /** Text to show while spinner is running */
  text: string;
  /** Text to show on success (optional, defaults to text) */
  successText?: string;
  /** Text to show on failure (optional, defaults to error message) */
  failText?: string;
}

/**
 * Execute an async function with spinner UI.
 *
 * Automatically handles spinner lifecycle:
 * - Starts spinner with initial text
 * - Shows success message on completion
 * - Shows error message on failure
 * - Always stops the spinner
 *
 * @param options - Spinner configuration
 * @param fn - Async function to execute
 * @returns The result from the function
 * @throws Re-throws any error from the function
 *
 * @example
 * ```typescript
 * const result = await withSpinner(
 *   { text: 'Loading...', successText: 'Loaded!' },
 *   async () => {
 *     return await loadData();
 *   }
 * );
 * ```
 */
export async function withSpinner<T>(options: SpinnerOptions, fn: () => Promise<T>): Promise<T> {
  const spinner = ora(options.text).start();

  try {
    const result = await fn();
    spinner.succeed(options.successText || options.text);
    return result;
  } catch (error) {
    const errorMessage =
      options.failText || (error instanceof Error ? error.message : 'Operation failed');
    spinner.fail(errorMessage);
    throw error;
  }
}

/**
 * Create a managed spinner that can be controlled manually.
 *
 * Useful for operations that need to update the spinner text during execution
 * or have multiple stages.
 *
 * @param text - Initial spinner text
 * @returns Managed spinner instance
 *
 * @example
 * ```typescript
 * const spinner = createSpinner('Starting...');
 * try {
 *   spinner.text = 'Processing...';
 *   await doWork();
 *   spinner.succeed('Done!');
 * } catch (error) {
 *   spinner.fail('Failed!');
 * }
 * ```
 */
export function createSpinner(text: string): Ora {
  return ora(text).start();
}
