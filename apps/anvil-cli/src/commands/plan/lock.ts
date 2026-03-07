/**
 * Plan Lock Command
 * Locks a task for execution
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { resolve } from 'node:path';
import { TaskLocker, type LockResult } from '@eddacraft/anvil-aps';
import { data, print } from '../../utils/output.js';
import { CliError, CliExit } from '../../utils/cli-error.js';

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
    .option('--plan <path>', 'Path to planning document', 'docs/plans/APS.md')
    .option('--user <name>', 'User name for provenance')
    .option('--skip-validation', 'Skip planning document validation')
    .option('--json', 'Output as JSON')
    .action(async (taskId: string, options: LockOptions) => {
      const planPath = resolve(options.plan || 'docs/plans/APS.md');
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
          data(formatAsJson(result));
          if (result.success) throw new CliExit();
          throw new CliError('Lock failed');
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
          throw new CliError(error instanceof Error ? error.message : 'Lock failed');
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
            print(chalk.gray('  Execution plan:'), chalk.cyan(result.executionPlanPath));
          } else {
            spinner.fail(chalk.red(`Failed to lock task ${taskId}`));
            print(chalk.red('  Error:'), result.error);
            throw new CliError(`Failed to lock task ${taskId}`);
          }
        } catch (error) {
          if (error instanceof CliError || error instanceof CliExit) throw error;
          spinner.fail(chalk.red(`Failed to lock task ${taskId}`));
          print(chalk.red('Error:'), error instanceof Error ? error.message : String(error));
          throw new CliError(
            error instanceof Error ? error.message : `Failed to lock task ${taskId}`
          );
        }
      }
    });
}
