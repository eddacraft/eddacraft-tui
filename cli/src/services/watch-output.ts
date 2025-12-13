/**
 * Watch Output Service
 *
 * Real-time output formatting for watch mode.
 */

import chalk from 'chalk';
import type { WatchStatusEvent, WatchActionResult } from '@anvil/core';

/**
 * Format timestamp for display
 */
function formatTime(): string {
  const now = new Date();
  return chalk.gray(
    `[${now.getHours().toString().padStart(2, '0')}:${now.getMinutes().toString().padStart(2, '0')}:${now.getSeconds().toString().padStart(2, '0')}]`
  );
}

/**
 * Watch output handler
 *
 * Formats and displays watch events in real-time.
 */
export class WatchOutput {
  private passCount = 0;
  private failCount = 0;
  private verbose: boolean;

  constructor(options?: { verbose?: boolean }) {
    this.verbose = options?.verbose ?? false;
  }

  /**
   * Display watch mode header
   */
  showHeader(options: {
    patterns: string[];
    action: string;
    gitFilter: boolean;
    profile?: string;
  }): void {
    console.log('');
    console.log(chalk.bold('  Anvil Watch Mode'));
    console.log(chalk.gray('  ─────────────────'));
    console.log(chalk.gray('  Watching: ') + chalk.cyan(options.patterns.slice(0, 3).join(', ')));
    if (options.patterns.length > 3) {
      console.log(chalk.gray(`           + ${options.patterns.length - 3} more patterns`));
    }

    const actionLabel = options.profile
      ? `${options.action} (profile: ${options.profile})`
      : options.action;
    console.log(chalk.gray('  Action: ') + chalk.yellow(actionLabel));

    const filterLabel = options.gitFilter ? 'unstaged changes only' : 'all file changes';
    console.log(chalk.gray('  Filter: ') + filterLabel);

    console.log('');
    console.log(chalk.gray('  Press Ctrl+C to stop'));
    console.log('');
  }

  /**
   * Handle watch status event
   */
  handleEvent(event: WatchStatusEvent): void {
    switch (event.type) {
      case 'ready':
        this.onReady(event);
        break;
      case 'change':
        this.onChange(event);
        break;
      case 'action:start':
        this.onActionStart(event);
        break;
      case 'action:complete':
        this.onActionComplete(event);
        break;
      case 'action:error':
        this.onActionError(event);
        break;
      case 'stopped':
        this.onStopped();
        break;
    }
  }

  /**
   * Show watching status line
   */
  showWatching(): void {
    const stats =
      this.passCount > 0 || this.failCount > 0
        ? ` (${chalk.green(this.passCount + ' pass')}, ${chalk.red(this.failCount + ' fail')})`
        : '';
    console.log(chalk.gray(`\n  Watching for changes...${stats}\n`));
  }

  private onReady(event: Extract<WatchStatusEvent, { type: 'ready' }>): void {
    if (this.verbose) {
      console.log(
        chalk.gray(
          `  Ready. Watching ${event.patterns.length} pattern(s), git filter: ${event.gitFilter}`
        )
      );
    }
  }

  private onChange(event: Extract<WatchStatusEvent, { type: 'change' }>): void {
    if (this.verbose) {
      const filtered = event.files.length - event.filtered.length;
      if (filtered > 0) {
        console.log(chalk.gray(`  ${filtered} file(s) filtered out (staged or excluded)`));
      }
    }
  }

  private onActionStart(event: Extract<WatchStatusEvent, { type: 'action:start' }>): void {
    const files = event.files.map((f: string) => f.split('/').pop()).join(', ');
    console.log(`${formatTime()} ${chalk.cyan(files)} changed`);
    console.log(chalk.gray(`  Running ${event.action}...`));
  }

  private onActionComplete(event: Extract<WatchStatusEvent, { type: 'action:complete' }>): void {
    const { result } = event;

    if (result.success) {
      this.passCount++;
      this.showSuccessResult(result);
    } else {
      this.failCount++;
      this.showFailureResult(result);
    }

    this.showWatching();
  }

  private onActionError(event: Extract<WatchStatusEvent, { type: 'action:error' }>): void {
    this.failCount++;
    console.log(chalk.red(`  Error: ${event.error.message}`));
    this.showWatching();
  }

  private onStopped(): void {
    console.log('');
    console.log(chalk.gray('  Watch mode stopped'));
    console.log(chalk.gray(`  Total: ${this.passCount} passed, ${this.failCount} failed`));
  }

  private showSuccessResult(result: WatchActionResult): void {
    if (result.action === 'validate') {
      console.log(
        chalk.green(`  ✓ Validation passed`) + chalk.gray(` (${result.executionTimeMs}ms)`)
      );
    } else {
      // Gate result
      const details = result.details as
        | {
            score?: number;
            checks?: Array<{ check: string; passed: boolean; score?: number }>;
          }
        | undefined;

      if (details?.checks && this.verbose) {
        for (const check of details.checks) {
          const icon = check.passed ? chalk.green('✓') : chalk.red('✗');
          const scoreStr = check.score !== undefined ? ` (${check.score}%)` : '';
          console.log(`    ${icon} ${check.check}${scoreStr}`);
        }
      }

      const scoreStr = details?.score !== undefined ? ` (score: ${details.score.toFixed(1)}%)` : '';
      console.log(
        chalk.green(`  ✓ Gate passed${scoreStr}`) + chalk.gray(` (${result.executionTimeMs}ms)`)
      );
    }
  }

  private showFailureResult(result: WatchActionResult): void {
    if (result.action === 'validate') {
      console.log(chalk.red(`  ✗ Validation failed`));
      if (result.error) {
        console.log(chalk.red(`    ${result.error}`));
      }
    } else {
      // Gate result
      const details = result.details as
        | {
            score?: number;
            checks?: Array<{ check: string; passed: boolean; message?: string }>;
          }
        | undefined;

      if (details?.checks) {
        for (const check of details.checks) {
          if (!check.passed) {
            console.log(chalk.red(`    ✗ ${check.check}: ${check.message || 'failed'}`));
          }
        }
      }

      const scoreStr = details?.score !== undefined ? ` (score: ${details.score.toFixed(1)}%)` : '';
      console.log(chalk.red(`  ✗ Gate failed${scoreStr}`));
    }
  }
}

/**
 * Create watch output handler
 */
export function createWatchOutput(options?: { verbose?: boolean }): WatchOutput {
  return new WatchOutput(options);
}
