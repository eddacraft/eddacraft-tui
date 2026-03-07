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
import { CliError, CliExit } from '../../utils/cli-error.js';
import { blank, data, print } from '../../utils/output.js';

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
          data(JSON.stringify(info, null, 2));
          return;
        }

        // Pretty print
        print(chalk.bold('\nMulti-Agent Coordination System'));
        print(chalk.gray('═'.repeat(50)));

        print(chalk.bold('\nConfiguration'));
        print(chalk.gray('─'.repeat(40)));
        print(
          `  ${chalk.cyan('Lock Timeout:')}      ${config.lockTimeoutMs}ms (${Math.round(config.lockTimeoutMs / 60000)}min)`
        );
        print(`  ${chalk.cyan('Heartbeat:')}         ${config.heartbeatIntervalMs}ms`);
        print(`  ${chalk.cyan('Stale Threshold:')}   ${config.staleThresholdMs}ms`);
        print(`  ${chalk.cyan('Queue Timeout:')}     ${config.queueTimeoutMs}ms`);
        print(`  ${chalk.cyan('Max Queue Size:')}    ${config.maxQueueSize}`);

        print(chalk.bold('\nAgents'));
        print(chalk.gray('─'.repeat(40)));
        print(`  ${chalk.cyan('Total:')}      ${agents.length}`);
        print(`  ${chalk.cyan('Active:')}     ${chalk.green(activeAgents.length)}`);
        print(`  ${chalk.cyan('Stale:')}      ${chalk.yellow(info.agents.stale)}`);
        print(`  ${chalk.cyan('Terminated:')} ${chalk.gray(info.agents.terminated)}`);

        print(chalk.bold('\nWatch Lock'));
        print(chalk.gray('─'.repeat(40)));
        if (watchLockInfo) {
          print(`  ${chalk.cyan('Status:')}     ${chalk.green('● Held')}`);
          print(`  ${chalk.cyan('Holder:')}     ${watchLockInfo.agentId}`);
          print(`  ${chalk.cyan('Since:')}      ${watchLockInfo.acquiredAt}`);
          print(`  ${chalk.cyan('Expires:')}    ${watchLockInfo.expiresAt}`);
        } else {
          print(`  ${chalk.cyan('Status:')}     ${chalk.gray('○ Available')}`);
        }

        if (queues.length > 0) {
          print(chalk.bold('\nActive Queues'));
          print(chalk.gray('─'.repeat(40)));
          for (const q of queues) {
            print(`  ${chalk.cyan(`${q.type}:${q.resource}`)}: ${q.entries} waiting`);
          }
        }

        print(chalk.bold('\nStorage Paths'));
        print(chalk.gray('─'.repeat(40)));
        print(`  ${chalk.cyan('Registry:')}   ${config.registryPath}`);
        print(`  ${chalk.cyan('Locks:')}      ${config.lockDir}`);
        print(`  ${chalk.cyan('Queues:')}     ${config.queueDir}`);

        blank();

        // Environment variable hints
        print(chalk.bold('Environment Variables'));
        print(chalk.gray('─'.repeat(40)));
        print(chalk.gray('  Set these to customize agent identification:'));
        print(chalk.gray('    ANVIL_AGENT_ID      Custom agent identifier'));
        print(chalk.gray('    ANVIL_AGENT_TYPE    Agent type (claude, cursor, etc.)'));
        print(chalk.gray('    ANVIL_AGENT_NAME    Human-readable agent name'));
        print(chalk.gray('    ANVIL_SESSION_ID    Session identifier'));
        blank();
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        print(chalk.red(`Error: ${err instanceof Error ? err.message : 'Unknown error'}`));
        throw new CliError(err instanceof Error ? err.message : 'Unknown error');
      }
    });

  return command;
}
