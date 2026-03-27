import chalk from 'chalk';
import type { GateRunResult, GateRunResultWithCache } from '@eddacraft/anvil-runtime';

/**
 * CLI Output Conventions
 * ─────────────────────
 * - **stderr** for all human-readable / diagnostic output (status, progress, errors)
 * - **stdout** for structured data only (JSON, piped content)
 *
 * Functions:
 *   success(msg)    → stderr  ✓ message
 *   error(msg)      → stderr  ✗ message
 *   warning(msg)    → stderr  ⚠ message
 *   info(msg)       → stderr  ℹ message
 *   print(msg)      → stderr  raw text (chalk-formatted UI output)
 *   blank()         → stderr  empty line (visual spacing)
 *   data(content)   → stdout  raw content + newline (structured/piped data)
 *   json(obj)       → stdout  JSON.stringify (pretty-printed)
 *   debug(msg)      → stderr  [debug] message (when ANVIL_DEBUG=1)
 */

let debugEnabled = false;

export function enableDebug(): void {
  debugEnabled = true;
}

/** @internal Reset debug state — for test isolation only. */
export function resetDebug(): void {
  debugEnabled = false;
}

function isAnvilDebug(): boolean {
  const value = process.env['ANVIL_DEBUG'];
  return value === '1' || value?.toLowerCase() === 'true';
}

export function isDebugEnabled(): boolean {
  return debugEnabled || isAnvilDebug();
}

/**
 * Debug message to stderr, visible when enableDebug() was called or ANVIL_DEBUG is set to "1" or "true" (case-insensitive).
 * Intended for silent-fallback paths and catch blocks that return defaults.
 */
export function debug(message: string): void {
  if (!isDebugEnabled()) return;
  console.error(chalk.dim('[debug]'), chalk.dim(message));
}

export function success(message: string): void {
  console.error(chalk.green('✓'), message);
}

export function error(message: string): void {
  console.error(chalk.red('✗'), message);
}

export function warning(message: string): void {
  console.warn(chalk.yellow('⚠'), message);
}

export function info(message: string): void {
  console.error(chalk.blue('ℹ'), message);
}

/**
 * Write human-readable output to stderr.
 *
 * Use for chalk-formatted UI text, tables, progress messages —
 * anything a human reads but a pipe consumer should not see.
 */
export function print(...args: unknown[]): void {
  console.error(...args);
}

/**
 * Write an empty line to stderr (visual spacing).
 */
export function blank(): void {
  console.error('');
}

/**
 * Write raw structured data to stdout (pipe-safe).
 */
export function data(content: string): void {
  process.stdout.write(content + '\n');
}

/**
 * Write a JSON-serialised object to stdout (pipe-safe).
 */
export function json(obj: unknown, pretty = true): void {
  process.stdout.write(JSON.stringify(obj, null, pretty ? 2 : 0) + '\n');
}

export function formatValidationErrors(errors: Array<{ message: string; path?: string }>): void {
  if (errors.length === 0) return;

  console.error(chalk.red('\nValidation Errors:'));
  errors.forEach((error, index) => {
    console.error(chalk.red(`  ${index + 1}. ${error.message}`));
    if (error.path) {
      console.error(chalk.gray(`     at ${error.path}`));
    }
  });
}

export function formatGateResults(results: GateRunResult): void {
  console.error(chalk.bold('\nGate Results:'));
  console.error(
    chalk.bold(`Overall: ${results.overall ? chalk.green('PASSED') : chalk.red('FAILED')}`)
  );
  console.error(chalk.bold(`Score: ${results.score.toFixed(1)}%`));

  console.error(chalk.bold('\nCheck Results:'));
  results.checks.forEach((check) => {
    const status = check.passed ? chalk.green('PASS') : chalk.red('FAIL');
    const score = check.score ? ` (${check.score.toFixed(1)}%)` : '';
    console.error(`  ${status} ${check.check}${score}: ${check.message}`);

    if (check.error) {
      console.error(chalk.gray(`    Error: ${check.error}`));
    }
  });

  console.error(chalk.bold('\nSummary:'));
  console.error(`  Total: ${results.summary.total}`);
  console.error(`  Passed: ${chalk.green(results.summary.passed)}`);
  console.error(`  Failed: ${chalk.red(results.summary.failed)}`);
  console.error(`  Skipped: ${chalk.yellow(results.summary.skipped)}`);
}

/**
 * JSON output structure for gate results
 */
export interface JSONGateOutput {
  version: '1.0.0';
  timestamp: string;
  overall: boolean;
  score: number;
  checks: Array<{
    name: string;
    passed: boolean;
    score?: number;
    message: string;
    error?: string;
    skipped?: boolean;
    cached?: boolean;
  }>;
  summary: {
    total: number;
    passed: number;
    failed: number;
    skipped: number;
  };
  cache?: {
    hits: number;
    misses: number;
    timeSavedMs: number;
  };
  timing?: {
    totalMs: number;
    checks: Record<string, number>;
  };
}

/**
 * Format gate results as JSON (for CI/CD integration)
 */
export function formatGateResultsJSON(results: GateRunResultWithCache): void {
  const output: JSONGateOutput = {
    version: '1.0.0',
    timestamp: new Date().toISOString(),
    overall: results.overall,
    score: results.score,
    checks: results.checks.map((check) => ({
      name: check.check,
      passed: check.passed,
      score: check.score,
      message: check.message,
      error: check.error,
      skipped: check.skipped,
      cached: check.details?.cached as boolean | undefined,
    })),
    summary: results.summary,
    cache: results.cacheStats,
    timing: results.timing,
  };

  json(output);
}
