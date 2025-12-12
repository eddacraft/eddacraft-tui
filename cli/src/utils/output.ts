import chalk from 'chalk';
import type { GateRunResult, GateRunResultWithCache } from '@anvil/core';

export function success(message: string): void {
  console.log(chalk.green('✓'), message);
}

export function error(message: string): void {
  console.error(chalk.red('✗'), message);
}

export function warning(message: string): void {
  console.warn(chalk.yellow('⚠'), message);
}

export function info(message: string): void {
  console.log(chalk.blue('ℹ'), message);
}

export function formatValidationErrors(errors: Array<{ message: string; path?: string }>): void {
  if (errors.length === 0) return;

  console.log(chalk.red('\nValidation Errors:'));
  errors.forEach((error, index) => {
    console.log(chalk.red(`  ${index + 1}. ${error.message}`));
    if (error.path) {
      console.log(chalk.gray(`     at ${error.path}`));
    }
  });
}

export function formatGateResults(results: GateRunResult): void {
  console.log(chalk.bold('\nGate Results:'));
  console.log(
    chalk.bold(`Overall: ${results.overall ? chalk.green('PASSED') : chalk.red('FAILED')}`)
  );
  console.log(chalk.bold(`Score: ${results.score.toFixed(1)}%`));

  console.log(chalk.bold('\nCheck Results:'));
  results.checks.forEach((check) => {
    const status = check.passed ? chalk.green('PASS') : chalk.red('FAIL');
    const score = check.score ? ` (${check.score.toFixed(1)}%)` : '';
    console.log(`  ${status} ${check.check}${score}: ${check.message}`);

    if (check.error) {
      console.log(chalk.gray(`    Error: ${check.error}`));
    }
  });

  console.log(chalk.bold('\nSummary:'));
  console.log(`  Total: ${results.summary.total}`);
  console.log(`  Passed: ${chalk.green(results.summary.passed)}`);
  console.log(`  Failed: ${chalk.red(results.summary.failed)}`);
  console.log(`  Skipped: ${chalk.yellow(results.summary.skipped)}`);
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

  console.log(JSON.stringify(output, null, 2));
}
