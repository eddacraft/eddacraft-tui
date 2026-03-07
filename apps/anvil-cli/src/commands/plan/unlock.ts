/**
 * Plan Unlock Command
 * Unlocks (cancels) a locked task
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { resolve } from 'node:path';
import { TaskLocker, type UnlockResult } from '@eddacraft/anvil-aps';
import { data, print } from '../../utils/output.js';
import { CliError, CliExit } from '../../utils/cli-error.js';

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
    .option('--plan <path>', 'Path to planning document', 'docs/plans/APS.md')
    .option('--json', 'Output as JSON')
    .action(async (taskId: string, options: UnlockOptions) => {
      const planPath = resolve(options.plan || 'docs/plans/APS.md');
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
          data(formatAsJson(result));
          if (result.success) throw new CliExit();
          throw new CliError('Unlock failed');
        } catch (error) {
          if (error instanceof CliError || error instanceof CliExit) throw error;
          data(
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
          throw new CliError(error instanceof Error ? error.message : 'Unlock failed');
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
            print(chalk.gray('  Previous status:'), chalk.yellow(result.previousStatus));
          } else {
            spinner.fail(chalk.red(`Failed to unlock task ${taskId}`));
            print(chalk.red('  Error:'), result.error);
            if (result.previousStatus) {
              print(chalk.gray('  Current status:'), chalk.yellow(result.previousStatus));
            }
            throw new CliError(`Failed to unlock task ${taskId}`);
          }
        } catch (error) {
          if (error instanceof CliError || error instanceof CliExit) throw error;
          spinner.fail(chalk.red(`Failed to unlock task ${taskId}`));
          print(chalk.red('Error:'), error instanceof Error ? error.message : String(error));
          throw new CliError(
            error instanceof Error ? error.message : `Failed to unlock task ${taskId}`
          );
        }
      }
    });
}
