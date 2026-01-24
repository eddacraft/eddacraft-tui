/**
 * Plan Unlock Command
 * Unlocks (cancels) a locked task
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { resolve } from 'node:path';
import { TaskLocker, type UnlockResult } from '@eddacraft/anvil-aps';

export interface UnlockOptions {
  plan?: string;
  json?: boolean;
}

/**
 * Format unlock result as JSON
 */
function formatAsJson(result: UnlockResult): string {
  return JSON.stringify(result, null, 2);
}

export function createUnlockSubcommand(): Command {
  return new Command('unlock')
    .description('Unlock (cancel) a locked task')
    .argument('<task>', 'Task ID to unlock (e.g., AUTH-001)')
    .option('--plan <path>', 'Path to planning document', 'docs/planning/APS.md')
    .option('--json', 'Output as JSON')
    .action(async (taskId: string, options: UnlockOptions) => {
      const planPath = resolve(options.plan || 'docs/planning/APS.md');
      const projectRoot = process.cwd();

      if (options.json) {
        // JSON mode
        try {
          const locker = new TaskLocker({
            projectRoot,
            planPath,
            skipValidation: true, // Don't need to validate for unlock
          });

          const result = await locker.unlock(taskId);
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
        const spinner = ora(`Unlocking task ${taskId}...`).start();

        try {
          const locker = new TaskLocker({
            projectRoot,
            planPath,
            skipValidation: true,
          });

          const result = await locker.unlock(taskId);

          if (result.success) {
            spinner.succeed(chalk.green(`Task ${taskId} unlocked (cancelled)`));
            console.log(chalk.gray('  Previous status:'), chalk.yellow(result.previousStatus));
          } else {
            spinner.fail(chalk.red(`Failed to unlock task ${taskId}`));
            console.error(chalk.red('  Error:'), result.error);
            if (result.previousStatus) {
              console.log(chalk.gray('  Current status:'), chalk.yellow(result.previousStatus));
            }
            process.exit(1);
          }
        } catch (error) {
          spinner.fail(chalk.red(`Failed to unlock task ${taskId}`));
          console.error(
            chalk.red('Error:'),
            error instanceof Error ? error.message : String(error)
          );
          process.exit(1);
        }
      }
    });
}
