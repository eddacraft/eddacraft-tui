/**
 * Agent Status Command
 *
 * Shows the current agent's status and registration info.
 */

import { Command } from 'commander';
import chalk from 'chalk';
import { createAgentManager, createAgentInfo } from '@eddacraft/anvil-runtime';
import { getWorkspaceRoot } from '../../utils/file-io.js';

interface StatusOptions {
  json?: boolean;
}

export function createAgentStatusCommand(): Command {
  const command = new Command('status');

  command
    .description('Show current agent status and identification')
    .option('--json', 'Output as JSON')
    .action(async (options: StatusOptions) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const agent = createAgentInfo();
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
            sessionId: agent.sessionId,
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
            ANVIL_AGENT_ID: process.env['ANVIL_AGENT_ID'] || null,
            ANVIL_AGENT_TYPE: process.env['ANVIL_AGENT_TYPE'] || null,
            CLAUDE_SESSION_ID: process.env['CLAUDE_SESSION_ID'] || null,
            CI: process.env['CI'] || null,
          },
        };

        if (options.json) {
          console.log(JSON.stringify(status, null, 2));
          return;
        }

        // Pretty print
        console.log(chalk.bold('\nAgent Information'));
        console.log(chalk.gray('─'.repeat(40)));
        console.log(`  ${chalk.cyan('ID:')}           ${agent.id}`);
        console.log(`  ${chalk.cyan('Type:')}         ${getAgentTypeDisplay(agent.type)}`);
        console.log(`  ${chalk.cyan('Name:')}         ${agent.name}`);
        console.log(`  ${chalk.cyan('PID:')}          ${agent.pid}`);
        if (agent.sessionId) {
          console.log(`  ${chalk.cyan('Session:')}      ${agent.sessionId}`);
        }

        console.log(chalk.bold('\nRegistration'));
        console.log(chalk.gray('─'.repeat(40)));
        if (registration) {
          console.log(`  ${chalk.cyan('Status:')}       ${getStateDisplay(registration.state)}`);
          console.log(`  ${chalk.cyan('Registered:')}   ${registration.registeredAt}`);
          console.log(`  ${chalk.cyan('Last Beat:')}    ${registration.lastHeartbeat}`);
          console.log(`  ${chalk.cyan('Beat Count:')}   ${registration.heartbeatCount}`);
          if (registration.currentOperation) {
            console.log(`  ${chalk.cyan('Operation:')}    ${registration.currentOperation}`);
          }
        } else {
          console.log(`  ${chalk.yellow('Not registered')}`);
        }

        console.log('');
      } catch (err) {
        console.error(chalk.red(`Error: ${err instanceof Error ? err.message : 'Unknown error'}`));
        process.exit(1);
      }
    });

  return command;
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
