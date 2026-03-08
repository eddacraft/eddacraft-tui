/**
 * Plan Status Command
 * Shows task states across the plan
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { resolve } from 'node:path';
import { TaskLocker, formatTaskStatus, type TaskStatusInfo } from '@eddacraft/anvil-aps';
import { blank, data, print } from '../../utils/output.js';
import { CliError, CliExit } from '../../utils/cli-error.js';

export interface StatusOptions {
  plan?: string;
  json?: boolean;
  summary?: boolean;
}

/**
 * Format status as JSON
 */
function formatAsJson(statuses: TaskStatusInfo[], summary: Record<string, number>): string {
  return JSON.stringify(
    {
      tasks: statuses,
      summary,
    },
    null,
    2
  );
}

/**
 * Format summary for human display
 */
function formatSummaryDisplay(summary: Record<string, number>): void {
  blank();
  print(chalk.bold.underline('Summary'));
  print(`  ${chalk.green('●')} Open:      ${summary.open}`);
  print(`  ${chalk.yellow('●')} Locked:    ${summary.locked}`);
  print(`  ${chalk.blue('●')} Completed: ${summary.completed}`);
  print(`  ${chalk.gray('●')} Cancelled: ${summary.cancelled}`);
  blank();
  print(
    chalk.bold('Total:'),
    summary.open + summary.locked + summary.completed + summary.cancelled
  );
}

/**
 * Format all statuses for human display
 */
function formatStatusDisplay(statuses: TaskStatusInfo[]): void {
  blank();
  print(chalk.bold.underline('Task Status'));
  blank();

  if (statuses.length === 0) {
    print(chalk.gray('  (no tasks found)'));
    return;
  }

  // Group by status
  const byStatus = {
    locked: statuses.filter((s) => s.status === 'locked'),
    open: statuses.filter((s) => s.status === 'open'),
    completed: statuses.filter((s) => s.status === 'completed'),
    cancelled: statuses.filter((s) => s.status === 'cancelled'),
  };

  // Show locked tasks first (in progress)
  if (byStatus.locked.length > 0) {
    print(chalk.yellow.bold('Locked (In Progress)'));
    for (const task of byStatus.locked) {
      print(`  ${chalk.yellow('●')} ${chalk.cyan(task.taskId)}`);
      if (task.lockedBy) {
        print(chalk.gray(`      Locked by ${task.lockedBy} at ${task.lockedAt}`));
      }
      if (task.source) {
        const loc = task.source.line ? `${task.source.file}:${task.source.line}` : task.source.file;
        print(chalk.gray(`      Source: ${loc}`));
      }
    }
    blank();
  }

  // Show open tasks
  if (byStatus.open.length > 0) {
    print(chalk.green.bold('Open'));
    for (const task of byStatus.open) {
      print(`  ${chalk.green('●')} ${chalk.cyan(task.taskId)}`);
      if (task.source) {
        const loc = task.source.line ? `${task.source.file}:${task.source.line}` : task.source.file;
        print(chalk.gray(`      Source: ${loc}`));
      }
    }
    blank();
  }

  // Show completed tasks
  if (byStatus.completed.length > 0) {
    print(chalk.blue.bold('Completed'));
    for (const task of byStatus.completed) {
      print(`  ${chalk.blue('●')} ${chalk.cyan(task.taskId)}`);
      if (task.completedAt) {
        print(chalk.gray(`      Completed: ${task.completedAt}`));
      }
    }
    blank();
  }

  // Show cancelled tasks
  if (byStatus.cancelled.length > 0) {
    print(chalk.gray.bold('Cancelled'));
    for (const task of byStatus.cancelled) {
      print(`  ${chalk.gray('●')} ${chalk.gray(task.taskId)}`);
      if (task.cancelledAt) {
        print(chalk.gray(`      Cancelled: ${task.cancelledAt}`));
      }
    }
    blank();
  }
}

export function createStatusSubcommand(): Command {
  return new Command('status')
    .description('Show task states across the plan')
    .argument('[task]', 'Optional task ID to check specific task')
    .option('--plan <path>', 'Path to planning document', 'docs/plans/APS.md')
    .option('--json', 'Output as JSON')
    .option('--summary', 'Show summary only')
    .action(async (taskId: string | undefined, options: StatusOptions) => {
      const planPath = resolve(options.plan || 'docs/plans/APS.md');
      const projectRoot = process.cwd();

      if (options.json) {
        // JSON mode
        try {
          const locker = new TaskLocker({
            projectRoot,
            planPath,
            skipValidation: true,
          });

          if (taskId) {
            // Single task status
            const status = await locker.getStatus(taskId);
            if (status) {
              data(JSON.stringify(status, null, 2));
            } else {
              data(JSON.stringify({ error: `Task ${taskId} not found` }, null, 2));
              throw new CliError(`Task ${taskId} not found`);
            }
          } else {
            // All tasks
            const statuses = await locker.getAllStatus();
            const summary = await locker.getStatusSummary();
            data(formatAsJson(statuses, summary));
          }
        } catch (error) {
          if (error instanceof CliError || error instanceof CliExit) throw error;
          data(
            JSON.stringify(
              {
                error: error instanceof Error ? error.message : String(error),
              },
              null,
              2
            )
          );
          throw new CliError(error instanceof Error ? error.message : 'Failed to load status');
        }
      } else {
        // Human-readable mode
        const spinner = ora('Loading task status...').start();

        try {
          const locker = new TaskLocker({
            projectRoot,
            planPath,
            skipValidation: true,
          });

          if (taskId) {
            // Single task status
            const status = await locker.getStatus(taskId);
            spinner.stop();

            if (status) {
              blank();
              print(formatTaskStatus(status));
            } else {
              print(chalk.red(`Task ${taskId} not found`));
              throw new CliError(`Task ${taskId} not found`);
            }
          } else {
            // All tasks
            const statuses = await locker.getAllStatus();
            const summary = await locker.getStatusSummary();

            spinner.succeed(chalk.green('Status loaded'));

            if (options.summary) {
              formatSummaryDisplay(summary);
            } else {
              formatStatusDisplay(statuses);
              formatSummaryDisplay(summary);
            }
          }
        } catch (error) {
          if (error instanceof CliError || error instanceof CliExit) throw error;
          spinner.fail(chalk.red('Failed to load status'));
          print(chalk.red('Error:'), error instanceof Error ? error.message : String(error));
          throw new CliError(error instanceof Error ? error.message : 'Failed to load status');
        }
      }
    });
}
