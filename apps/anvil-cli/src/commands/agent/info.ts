/**
 * Agent Info Command
 *
 * Shows detailed information about the multi-agent coordination system.
 */

import { Command } from 'commander';
import chalk from 'chalk';
import {
  createAgentManager,
  createLockManager,
  createQueueManager,
  getDefaultConcurrencyConfig,
} from '@eddacraft/anvil-runtime';
import { getWorkspaceRoot } from '../../utils/file-io.js';

interface InfoOptions {
  json?: boolean;
}

export function createAgentInfoCommand(): Command {
  const command = new Command('info');

  command
    .description('Show multi-agent coordination system information')
    .option('--json', 'Output as JSON')
    .action(async (options: InfoOptions) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const config = getDefaultConcurrencyConfig();

        const agentManager = createAgentManager({ workspaceRoot });
        const lockManager = createLockManager({ workspaceRoot });
        const queueManager = createQueueManager({ workspaceRoot });

        // Gather information
        const agents = await agentManager.getAllAgents();
        const activeAgents = await agentManager.getActiveAgents();
        const heldLocks = lockManager.getHeldLocks();
        const queues = await queueManager.getAllQueues();

        // Check for watch lock
        const watchLockInfo = await lockManager.getLockInfo('watch', 'workspace');

        const info = {
          workspace: workspaceRoot,
          config: {
            lockTimeoutMs: config.lockTimeoutMs,
            heartbeatIntervalMs: config.heartbeatIntervalMs,
            staleThresholdMs: config.staleThresholdMs,
            queueTimeoutMs: config.queueTimeoutMs,
            maxQueueSize: config.maxQueueSize,
          },
          agents: {
            total: agents.length,
            active: activeAgents.length,
            stale: agents.filter((a) => a.state === 'stale').length,
            terminated: agents.filter((a) => a.state === 'terminated').length,
          },
          locks: {
            heldByThisProcess: heldLocks.length,
            watchLock: watchLockInfo
              ? {
                  heldBy: watchLockInfo.agentId,
                  acquiredAt: watchLockInfo.acquiredAt,
                  expiresAt: watchLockInfo.expiresAt,
                }
              : null,
          },
          queues: queues.map((q) => ({
            type: q.type,
            resource: q.resource,
            entries: q.entries,
          })),
          paths: {
            registry: config.registryPath,
            locks: config.lockDir,
            queue: config.queueDir,
          },
        };

        if (options.json) {
          console.log(JSON.stringify(info, null, 2));
          return;
        }

        // Pretty print
        console.log(chalk.bold('\nMulti-Agent Coordination System'));
        console.log(chalk.gray('═'.repeat(50)));

        console.log(chalk.bold('\nConfiguration'));
        console.log(chalk.gray('─'.repeat(40)));
        console.log(
          `  ${chalk.cyan('Lock Timeout:')}      ${config.lockTimeoutMs}ms (${Math.round(config.lockTimeoutMs / 60000)}min)`
        );
        console.log(`  ${chalk.cyan('Heartbeat:')}         ${config.heartbeatIntervalMs}ms`);
        console.log(`  ${chalk.cyan('Stale Threshold:')}   ${config.staleThresholdMs}ms`);
        console.log(`  ${chalk.cyan('Queue Timeout:')}     ${config.queueTimeoutMs}ms`);
        console.log(`  ${chalk.cyan('Max Queue Size:')}    ${config.maxQueueSize}`);

        console.log(chalk.bold('\nAgents'));
        console.log(chalk.gray('─'.repeat(40)));
        console.log(`  ${chalk.cyan('Total:')}      ${agents.length}`);
        console.log(`  ${chalk.cyan('Active:')}     ${chalk.green(activeAgents.length)}`);
        console.log(`  ${chalk.cyan('Stale:')}      ${chalk.yellow(info.agents.stale)}`);
        console.log(`  ${chalk.cyan('Terminated:')} ${chalk.gray(info.agents.terminated)}`);

        console.log(chalk.bold('\nWatch Lock'));
        console.log(chalk.gray('─'.repeat(40)));
        if (watchLockInfo) {
          console.log(`  ${chalk.cyan('Status:')}     ${chalk.green('● Held')}`);
          console.log(`  ${chalk.cyan('Holder:')}     ${watchLockInfo.agentId}`);
          console.log(`  ${chalk.cyan('Since:')}      ${watchLockInfo.acquiredAt}`);
          console.log(`  ${chalk.cyan('Expires:')}    ${watchLockInfo.expiresAt}`);
        } else {
          console.log(`  ${chalk.cyan('Status:')}     ${chalk.gray('○ Available')}`);
        }

        if (queues.length > 0) {
          console.log(chalk.bold('\nActive Queues'));
          console.log(chalk.gray('─'.repeat(40)));
          for (const q of queues) {
            console.log(`  ${chalk.cyan(`${q.type}:${q.resource}`)}: ${q.entries} waiting`);
          }
        }

        console.log(chalk.bold('\nStorage Paths'));
        console.log(chalk.gray('─'.repeat(40)));
        console.log(`  ${chalk.cyan('Registry:')}   ${config.registryPath}`);
        console.log(`  ${chalk.cyan('Locks:')}      ${config.lockDir}`);
        console.log(`  ${chalk.cyan('Queues:')}     ${config.queueDir}`);

        console.log('');

        // Environment variable hints
        console.log(chalk.bold('Environment Variables'));
        console.log(chalk.gray('─'.repeat(40)));
        console.log(chalk.gray('  Set these to customize agent identification:'));
        console.log(chalk.gray('    ANVIL_AGENT_ID      Custom agent identifier'));
        console.log(chalk.gray('    ANVIL_AGENT_TYPE    Agent type (claude, cursor, etc.)'));
        console.log(chalk.gray('    ANVIL_AGENT_NAME    Human-readable agent name'));
        console.log(chalk.gray('    ANVIL_SESSION_ID    Session identifier'));
        console.log('');
      } catch (err) {
        console.error(chalk.red(`Error: ${err instanceof Error ? err.message : 'Unknown error'}`));
        process.exit(1);
      }
    });

  return command;
}
