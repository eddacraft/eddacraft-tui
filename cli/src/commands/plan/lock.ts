/**
 * Plan Lock Command
 * Locks a task for execution
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { resolve } from 'path';
import { TaskLocker, type LockResult } from '@anvil/aps';

export interface LockOptions {
  plan?: string;
  user?: string;
  skipValidation?: boolean;
  json?: boolean;
}

/**
 * Format lock result as JSON
 */
function formatAsJson(result: LockResult): string {
  return JSON.stringify(result, null, 2);
}

export function createLockSubcommand(): Command {
  return new Command('lock')
    .description('Lock a task for execution')
    .argument('<task>', 'Task ID to lock (e.g., AUTH-001)')
    .option('--plan <path>', 'Path to planning document', 'docs/planning/APS.md')
    .option('--user <name>', 'User name for provenance')
    .option('--skip-validation', 'Skip planning document validation')
    .option('--json', 'Output as JSON')
    .action(async (taskId: string, options: LockOptions) => {
      const planPath = resolve(options.plan || 'docs/planning/APS.md');
      const projectRoot = process.cwd();

      if (options.json) {
        // JSON mode
        try {
          const locker = new TaskLocker({
            projectRoot,
            planPath,
            user: options.user,
            skipValidation: options.skipValidation,
          });

          const result = await locker.lock(taskId);
          console.log(formatAsJson(result));
          process.exit(result.success ? 0 : 1);
        } catch (error) {
          console.log(
            JSON.stringify(
              {
                success: false,
                taskId,
                error: error instanceof Error ? error.message : String(error),
              },
              null,
              2
            )
          );
          process.exit(1);
        }
      } else {
        // Human-readable mode
        const spinner = ora(`Locking task ${taskId}...`).start();

        try {
          const locker = new TaskLocker({
            projectRoot,
            planPath,
            user: options.user,
            skipValidation: options.skipValidation,
          });

          const result = await locker.lock(taskId);

          if (result.success) {
            spinner.succeed(chalk.green(`Task ${taskId} locked successfully`));
            console.log(chalk.gray('  Execution plan:'), chalk.cyan(result.executionPlanPath));
          } else {
            spinner.fail(chalk.red(`Failed to lock task ${taskId}`));
            console.error(chalk.red('  Error:'), result.error);
            process.exit(1);
          }
        } catch (error) {
          spinner.fail(chalk.red(`Failed to lock task ${taskId}`));
          console.error(
            chalk.red('Error:'),
            error instanceof Error ? error.message : String(error)
          );
          process.exit(1);
        }
      }
    });
}
