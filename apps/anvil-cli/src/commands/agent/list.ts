/**
 * Agent List Command
 *
 * Lists all registered agents in the workspace.
 */

import { Command } from 'commander';
import chalk from 'chalk';
import { createAgentManager } from '@eddacraft/anvil-runtime';
import { getWorkspaceRoot } from '../../utils/file-io.js';

interface ListOptions {
  json?: boolean;
  all?: boolean;
}

export function createAgentListCommand(): Command {
  const command = new Command('list');

  command
    .alias('ls')
    .description('List all registered agents in the workspace')
    .option('--json', 'Output as JSON')
    .option('--all', 'Include stale and terminated agents')
    .action(async (options: ListOptions) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const manager = createAgentManager({ workspaceRoot });

        const allAgents = await manager.getAllAgents();

        // Filter based on options
        let agents = allAgents;
        if (!options.all) {
          agents = allAgents.filter((a) => a.state === 'active' || a.state === 'idle');
        }

        if (options.json) {
          console.log(JSON.stringify(agents, null, 2));
          return;
        }

        if (agents.length === 0) {
          console.log(chalk.yellow('\nNo agents registered in this workspace.'));
          if (!options.all && allAgents.length > 0) {
            console.log(
              chalk.gray(`(${allAgents.length} stale/terminated agents hidden. Use --all to show)`)
            );
          }
          console.log('');
          return;
        }

        // Count by state
        const active = agents.filter((a) => a.state === 'active').length;
        const idle = agents.filter((a) => a.state === 'idle').length;
        const stale = agents.filter((a) => a.state === 'stale').length;
        const terminated = agents.filter((a) => a.state === 'terminated').length;

        console.log(chalk.bold('\nRegistered Agents'));
        console.log(chalk.gray('─'.repeat(80)));
        console.log(
          chalk.gray(
            `  Active: ${chalk.green(active)}  |  Idle: ${chalk.yellow(idle)}  |  ` +
              `Stale: ${chalk.red(stale)}  |  Terminated: ${chalk.gray(terminated)}`
          )
        );
        console.log(chalk.gray('─'.repeat(80)));

        // Table header
        console.log(
          chalk.cyan(
            `  ${'ID'.padEnd(30)} ${'Type'.padEnd(12)} ${'State'.padEnd(12)} ${'Last Heartbeat'.padEnd(24)}`
          )
        );
        console.log(chalk.gray('  ' + '─'.repeat(78)));

        for (const reg of agents) {
          const stateDisplay = getStateIcon(reg.state);
          const timeAgo = getTimeAgo(reg.lastHeartbeat);
          const id = truncate(reg.agent.id, 28);
          const type = reg.agent.type.padEnd(12);

          console.log(`  ${id.padEnd(30)} ${type} ${stateDisplay.padEnd(12)} ${timeAgo}`);

          if (reg.currentOperation) {
            console.log(chalk.gray(`    └─ ${reg.currentOperation}`));
          }
        }

        console.log('');

        // Show command hints
        console.log(chalk.gray('  Commands:'));
        console.log(chalk.gray('    anvil agent cleanup    Clean up stale agents'));
        console.log(chalk.gray('    anvil agent status     Show current agent info'));
        console.log('');
      } catch (err) {
        console.error(chalk.red(`Error: ${err instanceof Error ? err.message : 'Unknown error'}`));
        process.exit(1);
      }
    });

  return command;
}

function getStateIcon(state: string): string {
  switch (state) {
    case 'active':
      return chalk.green('● active');
    case 'idle':
      return chalk.yellow('○ idle');
    case 'stale':
      return chalk.red('✕ stale');
    case 'terminated':
      return chalk.gray('◌ done');
    default:
      return state;
  }
}

function truncate(str: string, length: number): string {
  if (str.length <= length) return str;
  return str.slice(0, length - 2) + '..';
}

function getTimeAgo(isoDate: string): string {
  const date = new Date(isoDate);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSecs = Math.floor(diffMs / 1000);
  const diffMins = Math.floor(diffSecs / 60);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffSecs < 60) {
    return chalk.green(`${diffSecs}s ago`);
  } else if (diffMins < 60) {
    const color = diffMins < 5 ? chalk.green : chalk.yellow;
    return color(`${diffMins}m ago`);
  } else if (diffHours < 24) {
    return chalk.yellow(`${diffHours}h ago`);
  } else {
    return chalk.red(`${diffDays}d ago`);
  }
}
