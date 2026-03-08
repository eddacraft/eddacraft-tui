/**
 * Agent Status Command
 *
 * Shows the current agent's status and registration info.
 */

import { Command } from 'commander';
import chalk from 'chalk';
import { createDebugger } from '@eddacraft/anvil-core';
import { createAgentManager, createAgentInfo } from '@eddacraft/anvil-runtime';
import { getWorkspaceRoot } from '../../utils/file-io.js';
import { CliError, CliExit } from '../../utils/cli-error.js';
import { blank, data, print } from '../../utils/output.js';

const log = createDebugger('cli');

interface StatusOptions {
  json?: boolean;
}

export function createAgentStatusCommand(): Command {
  const command = new Command('status');

  command
    .description('Show current agent status and identification')
    .option('--json', 'Output as JSON')
    .action(async (options: StatusOptions) => {
      log(`agent status: json=${options.json}`);
      try {
        const workspaceRoot = getWorkspaceRoot();
        const agent = createAgentInfo();
        log(`agent status: id=${agent.id} type=${agent.type}`);
        const manager = createAgentManager({ workspaceRoot });

        // Get registration status
        const allAgents = await manager.getAllAgents();
        const registration = allAgents.find((a) => a.agent.id === agent.id);

        const status = {
          agent: {
            id: agent.id,
            type: agent.type,
            name: agent.name,
            pid: agent.pid,
            sessionId: maskSensitive(agent.sessionId),
          },
          registration: registration
            ? {
                registered: true,
                state: registration.state,
                registeredAt: registration.registeredAt,
                lastHeartbeat: registration.lastHeartbeat,
                heartbeatCount: registration.heartbeatCount,
                currentOperation: registration.currentOperation,
              }
            : {
                registered: false,
              },
          environment: {
            ANVIL_AGENT_ID: process.env['ANVIL_AGENT_ID'] != null,
            ANVIL_AGENT_TYPE: agent.type,
            CLAUDE_SESSION_ID: process.env['CLAUDE_SESSION_ID'] != null,
            CI: process.env['CI'] != null,
          },
        };

        if (options.json) {
          data(JSON.stringify(status, null, 2));
          return;
        }

        // Pretty print
        print(chalk.bold('\nAgent Information'));
        print(chalk.gray('─'.repeat(40)));
        print(`  ${chalk.cyan('ID:')}           ${agent.id}`);
        print(`  ${chalk.cyan('Type:')}         ${getAgentTypeDisplay(agent.type)}`);
        print(`  ${chalk.cyan('Name:')}         ${agent.name}`);
        print(`  ${chalk.cyan('PID:')}          ${agent.pid}`);
        if (agent.sessionId) {
          print(`  ${chalk.cyan('Session:')}      ${maskSensitive(agent.sessionId)}`);
        }

        print(chalk.bold('\nRegistration'));
        print(chalk.gray('─'.repeat(40)));
        if (registration) {
          print(`  ${chalk.cyan('Status:')}       ${getStateDisplay(registration.state)}`);
          print(`  ${chalk.cyan('Registered:')}   ${registration.registeredAt}`);
          print(`  ${chalk.cyan('Last Beat:')}    ${registration.lastHeartbeat}`);
          print(`  ${chalk.cyan('Beat Count:')}   ${registration.heartbeatCount}`);
          if (registration.currentOperation) {
            print(`  ${chalk.cyan('Operation:')}    ${registration.currentOperation}`);
          }
        } else {
          print(`  ${chalk.yellow('Not registered')}`);
        }

        blank();
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        print(chalk.red(`Error: ${err instanceof Error ? err.message : 'Unknown error'}`));
        throw new CliError(err instanceof Error ? err.message : 'Unknown error');
      }
    });

  return command;
}

function maskSensitive(value: string | undefined): string | null {
  if (!value) return null;
  if (value.length <= 8) return '***';
  return `${value.slice(0, 4)}..${value.slice(-4)}`;
}

function getAgentTypeDisplay(type: string): string {
  const icons: Record<string, string> = {
    claude: '🤖 Claude',
    cursor: '🖱️ Cursor',
    copilot: '🐙 Copilot',
    aider: '🔧 Aider',
    continue: '➡️ Continue',
    codeium: '💡 Codeium',
    human: '👤 Human',
    ci: '⚙️ CI',
    unknown: '❓ Unknown',
  };
  return icons[type] || type;
}

function getStateDisplay(state: string): string {
  switch (state) {
    case 'active':
      return chalk.green('● Active');
    case 'idle':
      return chalk.yellow('○ Idle');
    case 'stale':
      return chalk.red('✕ Stale');
    case 'terminated':
      return chalk.gray('◌ Terminated');
    default:
      return state;
  }
}
