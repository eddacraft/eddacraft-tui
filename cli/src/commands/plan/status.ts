/**
 * Plan Status Command
 * Shows task states across the plan
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { resolve } from 'path';
import { TaskLocker, formatTaskStatus, type TaskStatusInfo } from '@anvil/aps';

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
  console.log('');
  console.log(chalk.bold.underline('Summary'));
  console.log(`  ${chalk.green('●')} Open:      ${summary.open}`);
  console.log(`  ${chalk.yellow('●')} Locked:    ${summary.locked}`);
  console.log(`  ${chalk.blue('●')} Completed: ${summary.completed}`);
  console.log(`  ${chalk.gray('●')} Cancelled: ${summary.cancelled}`);
  console.log('');
  console.log(
    chalk.bold('Total:'),
    summary.open + summary.locked + summary.completed + summary.cancelled
  );
}

/**
 * Format all statuses for human display
 */
function formatStatusDisplay(statuses: TaskStatusInfo[]): void {
  console.log('');
  console.log(chalk.bold.underline('Task Status'));
  console.log('');

  if (statuses.length === 0) {
    console.log(chalk.gray('  (no tasks found)'));
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
    console.log(chalk.yellow.bold('Locked (In Progress)'));
    for (const task of byStatus.locked) {
      console.log(`  ${chalk.yellow('●')} ${chalk.cyan(task.taskId)}`);
      if (task.lockedBy) {
        console.log(chalk.gray(`      Locked by ${task.lockedBy} at ${task.lockedAt}`));
      }
      if (task.source) {
        const loc = task.source.line ? `${task.source.file}:${task.source.line}` : task.source.file;
        console.log(chalk.gray(`      Source: ${loc}`));
      }
    }
    console.log('');
  }

  // Show open tasks
  if (byStatus.open.length > 0) {
    console.log(chalk.green.bold('Open'));
    for (const task of byStatus.open) {
      console.log(`  ${chalk.green('●')} ${chalk.cyan(task.taskId)}`);
      if (task.source) {
        const loc = task.source.line ? `${task.source.file}:${task.source.line}` : task.source.file;
        console.log(chalk.gray(`      Source: ${loc}`));
      }
    }
    console.log('');
  }

  // Show completed tasks
  if (byStatus.completed.length > 0) {
    console.log(chalk.blue.bold('Completed'));
    for (const task of byStatus.completed) {
      console.log(`  ${chalk.blue('●')} ${chalk.cyan(task.taskId)}`);
      if (task.completedAt) {
        console.log(chalk.gray(`      Completed: ${task.completedAt}`));
      }
    }
    console.log('');
  }

  // Show cancelled tasks
  if (byStatus.cancelled.length > 0) {
    console.log(chalk.gray.bold('Cancelled'));
    for (const task of byStatus.cancelled) {
      console.log(`  ${chalk.gray('●')} ${chalk.gray(task.taskId)}`);
      if (task.cancelledAt) {
        console.log(chalk.gray(`      Cancelled: ${task.cancelledAt}`));
      }
    }
    console.log('');
  }
}

export function createStatusSubcommand(): Command {
  return new Command('status')
    .description('Show task states across the plan')
    .argument('[task]', 'Optional task ID to check specific task')
    .option('--plan <path>', 'Path to planning document', 'docs/planning/APS.md')
    .option('--json', 'Output as JSON')
    .option('--summary', 'Show summary only')
    .action(async (taskId: string | undefined, options: StatusOptions) => {
      const planPath = resolve(options.plan || 'docs/planning/APS.md');
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
              console.log(JSON.stringify(status, null, 2));
            } else {
              console.log(JSON.stringify({ error: `Task ${taskId} not found` }, null, 2));
              process.exit(1);
            }
          } else {
            // All tasks
            const statuses = await locker.getAllStatus();
            const summary = await locker.getStatusSummary();
            console.log(formatAsJson(statuses, summary));
          }
        } catch (error) {
          console.log(
            JSON.stringify(
              {
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
              console.log('');
              console.log(formatTaskStatus(status));
            } else {
              console.log(chalk.red(`Task ${taskId} not found`));
              process.exit(1);
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
          spinner.fail(chalk.red('Failed to load status'));
          console.error(
            chalk.red('Error:'),
            error instanceof Error ? error.message : String(error)
          );
          process.exit(1);
        }
      }
    });
}
