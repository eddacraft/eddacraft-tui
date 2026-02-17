/**
 * Agent Cleanup Command
 *
 * Cleans up stale agents, expired locks, and timed-out queue entries.
 */

import { Command } from 'commander';
import chalk from 'chalk';
import { createDebugger } from '@eddacraft/anvil-core';
import {
  createAgentManager,
  createLockManager,
  createQueueManager,
} from '@eddacraft/anvil-runtime';
import { getWorkspaceRoot } from '../../utils/file-io.js';

const log = createDebugger('cli');

interface CleanupOptions {
  json?: boolean;
  dryRun?: boolean;
}

export function createAgentCleanupCommand(): Command {
  const command = new Command('cleanup');

  command
    .description('Clean up stale agents, expired locks, and queue entries')
    .option('--json', 'Output as JSON')
    .option('--dry-run', 'Show what would be cleaned without actually cleaning')
    .action(async (options: CleanupOptions) => {
      log('agent cleanup: dryRun=%s json=%s', options.dryRun, options.json);
      try {
        const workspaceRoot = getWorkspaceRoot();

        const agentManager = createAgentManager({ workspaceRoot });
        const lockManager = createLockManager({ workspaceRoot });
        const queueManager = createQueueManager({ workspaceRoot });

        const results = {
          staleAgents: [] as string[],
          expiredLocks: 0,
          timedOutQueueEntries: 0,
        };

        if (!options.dryRun) {
          // Clean up stale agents
          results.staleAgents = await agentManager.cleanupStaleAgents();

          // Clean up expired locks
          results.expiredLocks = await lockManager.cleanupExpiredLocks();

          // Clean up timed-out queue entries
          results.timedOutQueueEntries = await queueManager.cleanupAllTimedOut();
        } else {
          // Dry run - just count what would be cleaned
          const allAgents = await agentManager.getAllAgents();
          const now = Date.now();
          const staleThreshold = 30000; // 30 seconds default

          for (const agent of allAgents) {
            const lastHeartbeat = new Date(agent.lastHeartbeat).getTime();
            if (now - lastHeartbeat > staleThreshold && agent.state === 'active') {
              results.staleAgents.push(agent.agent.id);
            }
          }
        }

        log(
          'agent cleanup result: staleAgents=%d expiredLocks=%d timedOutQueue=%d',
          results.staleAgents.length,
          results.expiredLocks,
          results.timedOutQueueEntries
        );
        const totalCleaned =
          results.staleAgents.length + results.expiredLocks + results.timedOutQueueEntries;

        if (options.json) {
          console.log(
            JSON.stringify(
              {
                dryRun: options.dryRun ?? false,
                ...results,
                total: totalCleaned,
              },
              null,
              2
            )
          );
          return;
        }

        // Pretty print
        const prefix = options.dryRun ? chalk.yellow('[DRY RUN] ') : '';

        console.log(chalk.bold(`\n${prefix}Cleanup Results`));
        console.log(chalk.gray('─'.repeat(40)));

        if (results.staleAgents.length > 0) {
          console.log(
            `  ${chalk.cyan('Stale Agents:')}    ${chalk.green(results.staleAgents.length)} ${
              options.dryRun ? 'would be' : ''
            } marked stale`
          );
          for (const agentId of results.staleAgents) {
            console.log(chalk.gray(`    • ${agentId}`));
          }
        } else {
          console.log(`  ${chalk.cyan('Stale Agents:')}    ${chalk.gray('none')}`);
        }

        console.log(
          `  ${chalk.cyan('Expired Locks:')}   ${
            results.expiredLocks > 0
              ? chalk.green(results.expiredLocks) + ` ${options.dryRun ? 'would be' : ''} removed`
              : chalk.gray('none')
          }`
        );

        console.log(
          `  ${chalk.cyan('Queue Entries:')}   ${
            results.timedOutQueueEntries > 0
              ? chalk.green(results.timedOutQueueEntries) +
                ` ${options.dryRun ? 'would be' : ''} removed`
              : chalk.gray('none')
          }`
        );

        console.log(chalk.gray('─'.repeat(40)));

        if (totalCleaned === 0) {
          console.log(chalk.gray('  Nothing to clean up.'));
        } else {
          const action = options.dryRun ? 'would be cleaned' : 'cleaned';
          console.log(`  ${chalk.green(`Total: ${totalCleaned} items ${action}`)}`);
        }

        console.log('');
      } catch (err) {
        console.error(chalk.red(`Error: ${err instanceof Error ? err.message : 'Unknown error'}`));
        process.exit(1);
      }
    });

  return command;
}
